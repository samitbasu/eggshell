//! A slider's label and readout above its track, and the readout edited as
//! text.

use eggshell_ui::contract::Size;
use eggshell_ui::owned::Step;
use eggshell_ui::{Builder, Style, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens, size};
use egui::{Event, Key, Theme, vec2};
use egui_kittest::Harness;
use std::time::Duration;

use crate::support::{self, Rig, box_at, click_node, focused_node, rect_of, ring_at, text_color};

struct App {
    volume: f64,
    changes: Vec<f64>,
    current: f64,
    current_changes: Vec<f64>,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_stretch()
            .gap(4.0)
            .p(4.0)
            .w_px(300.0),
        |b| {
            b.slider("volume", app.volume, 0.0..=100.0)
                .step(Step::new(5.0))
                .label("Volume")
                .readout(|v| format!("{v:.0}%"))
                .editable(|text| text.trim().trim_end_matches('%').parse().ok())
                .on_change(|app: &mut App, value| {
                    app.volume = value;
                    app.changes.push(value);
                });
            b.div_id("sm", tw().control_size(size::SM), |b| {
                b.slider("small", 30.0, 0.0..=100.0)
                    .label("Small")
                    .readout(|v| format!("{v:.1} dB"))
                    .editable(|text| text.trim().trim_end_matches("dB").trim().parse().ok())
                    .on_change(|_: &mut App, _| {});
            });
            b.div_id("xs", tw().control_size(size::XS), |b| {
                b.slider("tiny", 30.0, 0.0..=100.0)
                    .label("Tiny")
                    .readout(|v| format!("{v:.0}"))
                    .editable(|text| text.trim().parse().ok())
                    .on_change(|_: &mut App, _| {});
            });
            b.slider("current", app.current, 0.0..=10.0)
                .step(Step::new(0.01))
                .label("Current")
                .readout(|v| format!("{v:.1} A"))
                .editable(|text| text.trim().trim_end_matches('A').trim().parse().ok())
                .on_change(|app: &mut App, value| {
                    app.current = value;
                    app.current_changes.push(value);
                });
            b.slider("plain", 10.0, 0.0..=100.0)
                .label("Plain")
                .on_change(|_: &mut App, _| {});
            b.slider("bare", 10.0, 0.0..=100.0)
                .named("Bare")
                .size(Size::DEFAULT)
                .on_change(|_: &mut App, _| {});
            b.button("elsewhere", "Elsewhere").on_click(|_| {});
        },
    );
}

fn harness() -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App {
            volume: 40.0,
            changes: vec![],
            current: 6.04,
            current_changes: vec![],
        },
        view,
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(12);
    harness
}

const BOX: &str = "/\"volume\"/Box";
const TRACK: &str = "/\"volume\"/Box/Track";
const LABEL: &str = "/\"volume\"/Box/Header/Label";
const READOUT: &str = "/\"volume\"/Box/Header/Readout";
const SMALL: &str = "/\"sm\"/\"small\"/Box/Header/Readout";
const TINY: &str = "/\"xs\"/\"tiny\"/Box/Header/Readout";

fn type_text(harness: &mut Harness<'_, Rig<App>>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run_steps(4);
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(6);
}

fn open(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, READOUT);
    harness.run_steps(6);
}

#[test]
fn the_label_and_the_readout_share_a_row_above_the_track() {
    let harness = harness();
    let (whole, track) = (rect_of(&harness, BOX), rect_of(&harness, TRACK));
    let (label, readout) = (rect_of(&harness, LABEL), rect_of(&harness, READOUT));
    assert!(label.max.y <= track.min.y, "{label:?} above {track:?}");
    assert!(readout.max.y <= track.min.y);
    assert_eq!(label.left(), whole.left());
    assert_eq!(readout.right(), whole.right());
    assert!((label.center().y - readout.center().y).abs() < 0.6);
    let gap = Style::of(&harness.ctx)
        .slider
        .sizes
        .get(Size::DEFAULT)
        .header_gap;
    let row = rect_of(&harness, "/\"volume\"/Box/Header");
    assert!((track.min.y - row.max.y - gap).abs() < 0.6, "{gap}");
}

#[test]
fn the_readout_shows_the_value_as_the_format_writes_it_in_the_styles_colours() {
    let harness = harness();
    let slider = Shadcn::for_theme(Theme::Light).style().slider;
    let readout = rect_of(&harness, READOUT);
    assert_eq!(
        text_color(&harness, readout.expand(1.0), "40%"),
        slider.readout.rest.text
    );
    let label = rect_of(&harness, LABEL);
    assert_eq!(
        text_color(&harness, label.expand(1.0), "Volume"),
        slider.label.rest.text
    );
    assert_ne!(slider.label.rest.text, slider.readout.rest.text);
}

#[test]
fn a_slider_with_no_header_keeps_its_track_where_it_was() {
    let harness = harness();
    rect_of(&harness, "/\"bare\"/Track");
}

#[test]
fn a_label_alone_gives_the_row_no_readout() {
    let harness = harness();
    rect_of(&harness, "/\"plain\"/Box/Header/Label");
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path == "/\"plain\"/Box/Header/Readout")
    );
}

#[test]
fn a_click_on_the_editable_readout_opens_a_field_that_has_focus() {
    let mut harness = harness();
    open(&mut harness);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some("/\"volume\"/Box/Header/Readout/Field"),
        "the field has focus"
    );
}

#[test]
fn the_readout_being_edited_is_drawn_in_the_inputs_box_with_its_ring() {
    let mut harness = harness();
    let before = rect_of(&harness, READOUT);
    assert!(ring_at(&harness, before).is_none());
    open(&mut harness);
    let rect = rect_of(&harness, READOUT);
    let input = Style::of(&harness.ctx).input.clone();
    let height = input.sizes.get(Size::DEFAULT).height;
    assert!((rect.height() - height).abs() < 0.6, "{rect:?} {height}");
    assert_eq!(
        box_at(&harness, rect).fill,
        input.look.paint.states.rest.fill
    );
    assert!(
        ring_at(&harness, rect).is_some(),
        "ringed while it has focus"
    );
}

/// What the field of the readout at `readout` shows once `typed` is typed
/// into it.
fn typed_into(harness: &mut Harness<'_, Rig<App>>, readout: &str, typed: &str) -> String {
    assert_eq!(
        focused_node(harness),
        Some(format!("{readout}/Field")),
        "precondition: the field has focus"
    );
    type_text(harness, typed);
    let field = rect_of(harness, readout);
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if field.contains(text.pos) => {
                Some(text.galley.text().to_owned())
            }
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn a_click_puts_the_caret_where_it_lands_and_selects_nothing() {
    let mut harness = harness();
    let zero = support::part_rect(&harness, "40%", "0");
    support::press_and_release(&mut harness, egui::pos2(zero.min.x + 1.0, zero.center().y));
    harness.run_steps(6);
    assert_eq!(typed_into(&mut harness, READOUT, "5"), "450%");
}

#[test]
fn a_double_click_selects_the_whole_value_not_the_word_under_it() {
    let mut harness = harness();
    let number = support::part_rect(&harness, "30.0 dB", "30");
    support::click_twice(&mut harness, number.center(), Duration::from_millis(200));
    assert_eq!(typed_into(&mut harness, SMALL, "7"), "7");
}

/// Where a press on the last digit of the tiny readout's "30" lands: in the
/// readout, and in the padding of the box that replaces it.
fn on_the_last_digit(harness: &Harness<'_, Rig<App>>) -> egui::Pos2 {
    let digit = support::part_rect(harness, "30", "0");
    let at = egui::pos2(digit.right() - 1.0, digit.center().y);
    let readout = rect_of(harness, TINY);
    let padding = Style::of(&harness.ctx).input.sizes.get(size::XS).padding.x;
    assert!(
        readout.contains(at),
        "precondition: {at:?} is on the readout {readout:?}"
    );
    assert!(
        readout.right() - at.x < padding,
        "precondition: {at:?} is within the box's padding of its right edge {readout:?}"
    );
    at
}

#[test]
fn a_double_click_on_a_short_readouts_last_digit_selects_the_whole_value() {
    let mut harness = harness();
    let at = on_the_last_digit(&harness);
    support::click_twice(&mut harness, at, Duration::from_millis(200));
    assert_eq!(typed_into(&mut harness, TINY, "7"), "7");
}

#[test]
fn the_field_spans_its_box_and_its_text_keeps_the_inputs_padding() {
    let mut harness = harness();
    open(&mut harness);
    let boxed = rect_of(&harness, READOUT);
    let field = rect_of(&harness, &format!("{READOUT}/Field"));
    let input = Style::of(&harness.ctx).input.clone();
    let (border, padding) = (
        input.look.border_width,
        input.sizes.get(Size::DEFAULT).padding.x,
    );
    assert!(
        (boxed.right() - field.right() - border).abs() < 0.6,
        "the field reaches the box's border: {field:?} in {boxed:?}"
    );
    let text = support::shapes(&harness)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if field.contains(text.pos) => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            _ => None,
        })
        .expect("precondition: the field shows its text");
    assert!(
        (field.right() - text.right() - padding).abs() < 0.6,
        "the text sits the input's padding in: {text:?} in {field:?}"
    );
}

#[test]
fn a_click_in_the_fields_padding_keeps_it_open() {
    let mut harness = harness();
    let at = on_the_last_digit(&harness);
    support::click_twice(&mut harness, at, Duration::from_millis(1000));
    assert_eq!(
        focused_node(&harness),
        Some(format!("{TINY}/Field")),
        "the second click lands in the field"
    );
}

#[test]
fn a_relaxed_double_click_still_selects_the_whole_value() {
    let mut harness = harness();
    let number = support::part_rect(&harness, "30.0 dB", "30");
    support::click_twice(&mut harness, number.center(), Duration::from_millis(450));
    assert_eq!(typed_into(&mut harness, SMALL, "7"), "7");
}

#[test]
fn two_clicks_further_apart_than_a_double_click_are_two_clicks() {
    let mut harness = harness();
    let number = support::part_rect(&harness, "30.0 dB", "30");
    support::click_twice(&mut harness, number.center(), Duration::from_millis(600));
    let typed = typed_into(&mut harness, SMALL, "7");
    assert_eq!(
        typed.len(),
        "30.0 dB".len() + 1,
        "nothing replaced: {typed:?}"
    );
}

#[test]
fn eggshell_widens_eguis_double_click_window_to_its_own() {
    let harness = harness();
    let window = harness
        .ctx
        .options(|options| options.input_options.max_double_click_delay);
    assert_eq!(Duration::from_secs_f64(window), eggshell_ui::DOUBLE_CLICK);
}

#[test]
fn an_app_that_sets_its_own_double_click_window_keeps_it() {
    let mut harness = harness();
    harness
        .ctx
        .options_mut(|options| options.input_options.max_double_click_delay = 0.25);
    harness.run_steps(2);
    let window = harness
        .ctx
        .options(|options| options.input_options.max_double_click_delay);
    assert_eq!(window, 0.25);
}

#[test]
fn opening_the_readout_from_the_keyboard_selects_its_value() {
    let mut harness = harness();
    support::past_bar(&mut harness);
    press(&mut harness, Key::Tab);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some(READOUT),
        "precondition: Tab reaches the readout"
    );
    press(&mut harness, Key::Enter);
    assert_eq!(typed_into(&mut harness, READOUT, "45"), "45");
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.volume, 45.0);
}

#[test]
fn return_commits_what_was_typed_through_parse_range_and_step() {
    for (typed, committed) in [("55", 55.0), ("57%", 55.0), ("500", 100.0), ("-9", 0.0)] {
        let mut harness = harness();
        open(&mut harness);
        press(&mut harness, Key::A);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
        type_text(&mut harness, typed);
        assert!(
            harness.state().app.changes.is_empty(),
            "nothing is applied while typing"
        );
        press(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.volume, committed, "{typed:?}");
        assert_eq!(harness.state().app.changes, [committed]);
        assert!(focused_node(&harness).is_none(), "the field closes");
    }
}

#[test]
fn text_that_does_not_parse_changes_nothing() {
    let mut harness = harness();
    open(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    type_text(&mut harness, "loud");
    press(&mut harness, Key::Enter);
    assert!(harness.state().app.changes.is_empty());
    let readout = rect_of(&harness, READOUT);
    text_color(&harness, readout.expand(1.0), "40%");
}

#[test]
fn escape_reverts_what_was_typed() {
    let mut harness = harness();
    open(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    type_text(&mut harness, "10");
    press(&mut harness, Key::Escape);
    assert!(harness.state().app.changes.is_empty());
    text_color(&harness, rect_of(&harness, READOUT).expand(1.0), "40%");
}

#[test]
fn a_click_away_reverts_what_was_typed() {
    let mut harness = harness();
    open(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    type_text(&mut harness, "10");
    click_node(&mut harness, "/\"elsewhere\"");
    harness.run_steps(6);
    assert!(harness.state().app.changes.is_empty());
    text_color(&harness, rect_of(&harness, READOUT).expand(1.0), "40%");
}

#[test]
fn a_readout_that_is_not_editable_takes_no_focus() {
    let mut harness = harness();
    click_node(&mut harness, "/\"plain\"/Box/Header/Label");
    harness.run_steps(4);
    assert!(focused_node(&harness).is_none());
}

#[test]
fn dragging_the_thumb_of_a_slider_with_a_header_moves_the_value() {
    let mut harness = harness();
    let track = rect_of(&harness, TRACK);
    let thumb = Style::of(&harness.ctx)
        .slider
        .sizes
        .get(Size::DEFAULT)
        .thumb_size;
    let start = egui::pos2(
        track.left() + thumb / 2.0 + 0.4 * (track.width() - thumb),
        track.center().y,
    );
    let end = egui::pos2(track.right() - thumb / 2.0 - 10.0, track.center().y);
    support::drag(&mut harness, start, end);
    let value = harness.state().app.volume;
    assert!(value > 80.0, "{value}");
}

#[test]
fn the_readout_opens_for_editing_at_every_size() {
    for (path, size) in [
        (READOUT, Size::DEFAULT),
        ("/\"sm\"/\"small\"/Box/Header/Readout", size::SM),
        ("/\"xs\"/\"tiny\"/Box/Header/Readout", size::XS),
    ] {
        let mut harness = harness();
        click_node(&mut harness, path);
        harness.run_steps(6);
        assert_eq!(
            focused_node(&harness).as_deref(),
            Some(format!("{path}/Field").as_str()),
            "{size:?}"
        );
    }
}

#[test]
fn starting_and_ending_an_edit_moves_nothing() {
    for track in [
        TRACK,
        "/\"sm\"/\"small\"/Box/Track",
        "/\"xs\"/\"tiny\"/Box/Track",
    ] {
        let mut harness = harness();
        let readout = track.replace("Track", "Header/Readout");
        let before = rect_of(&harness, track);
        click_node(&mut harness, &readout);
        harness.run_steps(6);
        assert_eq!(rect_of(&harness, track), before, "during {track}");
        press(&mut harness, Key::Escape);
        assert_eq!(rect_of(&harness, track), before, "after {track}");
    }
}

#[test]
fn the_readouts_digits_are_as_wide_as_each_other() {
    let mut harness = harness();
    let mut widths = vec![];
    for value in [80.0, 10.0, 45.0, 11.0] {
        harness.state_mut().app.volume = value;
        harness.run_steps(4);
        widths.push(rect_of(&harness, READOUT).width());
    }
    assert!(
        widths.windows(2).all(|w| (w[0] - w[1]).abs() < 0.25),
        "{widths:?}"
    );
}

/// Where the first glyph of the galley painted with `text` lands on screen:
/// its left edge and its baseline, after the tessellator snaps the galley to
/// whole pixels.
fn first_glyph(harness: &Harness<'_, Rig<App>>, text: &str) -> egui::Pos2 {
    use egui::emath::GuiRounding as _;
    let (galley, at) = support::galley_of(harness, text);
    assert!(
        harness
            .ctx
            .tessellation_options(|options| options.round_text_to_pixels),
        "precondition: text is drawn on whole pixels"
    );
    let row = galley
        .rows
        .first()
        .expect("precondition: the text has a row");
    let glyph = row
        .glyphs
        .first()
        .expect("precondition: the row has a glyph");
    at.round_to_pixels(harness.ctx.pixels_per_point()) + row.pos.to_vec2() + glyph.pos.to_vec2()
}

#[test]
fn the_open_field_draws_the_value_where_the_readout_drew_it_at_every_size() {
    let readouts = [
        (READOUT, Size::DEFAULT, "40%"),
        (SMALL, size::SM, "30.0 dB"),
        (TINY, size::XS, "30"),
    ];
    let sizes: Vec<Size> = Style::of(&harness().ctx).slider.sizes.keys().collect();
    assert!(
        sizes
            .iter()
            .all(|size| readouts.iter().any(|(_, covered, _)| covered == size)),
        "precondition: every slider size is covered: {sizes:?}"
    );
    for (readout, size, text) in readouts {
        let mut harness = harness();
        let closed = first_glyph(&harness, text);
        click_node(&mut harness, readout);
        harness.run_steps(6);
        assert_eq!(
            focused_node(&harness),
            Some(format!("{readout}/Field")),
            "precondition: the field is open at {size:?}"
        );
        let open = first_glyph(&harness, text);
        assert!(
            (open - closed).length() < 0.01,
            "{size:?}: closed at {closed:?}, open at {open:?}"
        );
    }
}

const CURRENT: &str = "/\"current\"/Box/Header/Readout";

/// The current slider at 6.04, its readout "6.0 A" opened into a field.
fn current_open() -> Harness<'static, Rig<App>> {
    let mut harness = harness();
    support::galley_of(&harness, "6.0 A");
    click_node(&mut harness, CURRENT);
    harness.run_steps(6);
    assert_eq!(
        focused_node(&harness),
        Some(format!("{CURRENT}/Field")),
        "precondition: the field is open"
    );
    harness
}

#[test]
fn return_on_the_text_as_it_opened_writes_nothing() {
    let mut harness = current_open();
    press(&mut harness, Key::Enter);
    assert!(focused_node(&harness).is_none(), "the field closes");
    let app = &harness.state().app;
    assert_eq!(app.current, 6.04);
    assert!(app.current_changes.is_empty(), "{:?}", app.current_changes);
}

#[test]
fn return_after_typing_still_commits() {
    let mut harness = current_open();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    type_text(&mut harness, "6.5");
    press(&mut harness, Key::Enter);
    let app = &harness.state().app;
    assert_eq!(app.current, 6.5);
    assert_eq!(app.current_changes, [6.5]);
}
