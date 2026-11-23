//! A slider's label and readout above its track, and the readout edited as
//! text.

use eggshell_ui::contract::Size;
use eggshell_ui::owned::Step;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens, size};
use egui::{Key, Theme, vec2};
use std::time::Duration;

use crate::support::{
    self, box_at, click_twice, focused_node, press_and_release, ring_at, text_color,
};

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

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App {
            volume: 40.0,
            changes: vec![],
            current: 6.04,
            current_changes: vec![],
        },
        view,
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(12);
    d
}

const BOX: &str = "/\"volume\"/Box";
const TRACK: &str = "/\"volume\"/Box/Track";
const LABEL: &str = "/\"volume\"/Box/Header/Label";
const READOUT: &str = "/\"volume\"/Box/Header/Readout";
const SMALL: &str = "/\"sm\"/\"small\"/Box/Header/Readout";
const TINY: &str = "/\"xs\"/\"tiny\"/Box/Header/Readout";

fn open(d: &mut Driver<App>) {
    d.click(Target::suffix(READOUT)).frames(6);
}

#[test]
fn the_label_and_the_readout_share_a_row_above_the_track() {
    let d = harness();
    let (whole, track) = (d.rect(Target::suffix(BOX)), d.rect(Target::suffix(TRACK)));
    let (label, readout) = (
        d.rect(Target::suffix(LABEL)),
        d.rect(Target::suffix(READOUT)),
    );
    assert!(label.max.y <= track.min.y, "{label:?} above {track:?}");
    assert!(readout.max.y <= track.min.y);
    assert_eq!(label.left(), whole.left());
    assert_eq!(readout.right(), whole.right());
    assert!((label.center().y - readout.center().y).abs() < 0.6);
    let gap = d.shell().style().slider.sizes.get(Size::DEFAULT).header_gap;
    let row = d.rect(Target::suffix("/\"volume\"/Box/Header"));
    assert!((track.min.y - row.max.y - gap).abs() < 0.6, "{gap}");
}

#[test]
fn the_readout_shows_the_value_as_the_format_writes_it_in_the_styles_colours() {
    let d = harness();
    let slider = Shadcn::for_theme(Theme::Light).style().slider;
    let readout = d.rect(Target::suffix(READOUT));
    assert_eq!(
        text_color(&d, readout.expand(1.0), "40%"),
        slider.readout.rest.text
    );
    let label = d.rect(Target::suffix(LABEL));
    assert_eq!(
        text_color(&d, label.expand(1.0), "Volume"),
        slider.label.rest.text
    );
    assert_ne!(slider.label.rest.text, slider.readout.rest.text);
}

#[test]
fn a_slider_with_no_header_keeps_its_track_where_it_was() {
    let d = harness();
    d.rect(Target::suffix("/\"bare\"/Track"));
}

#[test]
fn a_label_alone_gives_the_row_no_readout() {
    let d = harness();
    d.rect(Target::suffix("/\"plain\"/Box/Header/Label"));
    assert!(
        !support::nodes(&d)
            .iter()
            .any(|(path, _)| path == "/\"plain\"/Box/Header/Readout")
    );
}

#[test]
fn a_click_on_the_editable_readout_opens_a_field_that_has_focus() {
    let mut d = harness();
    open(&mut d);
    assert_eq!(
        focused_node(&d).as_deref(),
        Some("/\"volume\"/Box/Header/Readout/Field"),
        "the field has focus"
    );
}

#[test]
fn the_readout_being_edited_is_drawn_in_the_inputs_box_with_its_ring() {
    let mut d = harness();
    let before = d.rect(Target::suffix(READOUT));
    assert!(ring_at(&d, before).is_none());
    open(&mut d);
    let rect = d.rect(Target::suffix(READOUT));
    let input = d.shell().style().input().clone();
    let height = input.sizes.get(Size::DEFAULT).height;
    assert!((rect.height() - height).abs() < 0.6, "{rect:?} {height}");
    assert_eq!(
        box_at(&d, rect).fill,
        egui::Color32::TRANSPARENT,
        "cn-input-group: no fill in light"
    );
    assert!(ring_at(&d, rect).is_some(), "ringed while it has focus");
}

/// What the field of the readout at `readout` shows once `typed` is typed
/// into it.
fn typed_into(d: &mut Driver<App>, readout: &str, typed: &str) -> String {
    assert_eq!(
        focused_node(d),
        Some(format!("{readout}/Field")),
        "precondition: the field has focus"
    );
    d.type_text(typed).frames(3);
    let field = d.rect(Target::suffix(readout));
    support::shapes(d)
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
    let mut d = harness();
    let zero = d
        .painted()
        .expect("40%")
        .part_rect("0")
        .expect("precondition: the part is on one row");
    press_and_release(&mut d, egui::pos2(zero.min.x + 1.0, zero.center().y));
    d.frames(6);
    assert_eq!(typed_into(&mut d, READOUT, "5"), "450%");
}

#[test]
fn a_double_click_selects_the_whole_value_not_the_word_under_it() {
    let mut d = harness();
    let number = d
        .painted()
        .expect("30.0 dB")
        .part_rect("30")
        .expect("precondition: the part is on one row");
    click_twice(&mut d, number.center(), Duration::from_millis(200));
    assert_eq!(typed_into(&mut d, SMALL, "7"), "7");
}

/// Where a press on the last digit of the tiny readout's "30" lands: in the
/// readout, and in the padding of the box that replaces it.
fn on_the_last_digit(d: &Driver<App>) -> egui::Pos2 {
    let digit = d
        .painted()
        .expect("30")
        .part_rect("0")
        .expect("precondition: the part is on one row");
    let at = egui::pos2(digit.right() - 1.0, digit.center().y);
    let readout = d.rect(Target::suffix(TINY));
    let padding = d.shell().style().input().sizes.get(size::XS).padding.x;
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
    let mut d = harness();
    let at = on_the_last_digit(&d);
    click_twice(&mut d, at, Duration::from_millis(200));
    assert_eq!(typed_into(&mut d, TINY, "7"), "7");
}

#[test]
fn the_field_spans_its_box_and_its_text_keeps_the_inputs_padding() {
    let mut d = harness();
    open(&mut d);
    let boxed = d.rect(Target::suffix(READOUT));
    let field = d.rect(Target::suffix(&format!("{READOUT}/Field")));
    let style = d.shell().style();
    let (border, padding) = (
        support::input_border(&style).0,
        style.input().sizes.get(Size::DEFAULT).padding.x,
    );
    assert!(
        (boxed.right() - field.right() - border).abs() < 0.6,
        "the field reaches the box's border: {field:?} in {boxed:?}"
    );
    let text = support::shapes(&d)
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
    let mut d = harness();
    let at = on_the_last_digit(&d);
    click_twice(&mut d, at, Duration::from_millis(1000));
    assert_eq!(
        focused_node(&d),
        Some(format!("{TINY}/Field")),
        "the second click lands in the field"
    );
}

#[test]
fn a_relaxed_double_click_still_selects_the_whole_value() {
    let mut d = harness();
    let number = d
        .painted()
        .expect("30.0 dB")
        .part_rect("30")
        .expect("precondition: the part is on one row");
    click_twice(&mut d, number.center(), Duration::from_millis(450));
    assert_eq!(typed_into(&mut d, SMALL, "7"), "7");
}

#[test]
fn two_clicks_further_apart_than_a_double_click_are_two_clicks() {
    let mut d = harness();
    let number = d
        .painted()
        .expect("30.0 dB")
        .part_rect("30")
        .expect("precondition: the part is on one row");
    click_twice(&mut d, number.center(), Duration::from_millis(600));
    let typed = typed_into(&mut d, SMALL, "7");
    assert_eq!(
        typed.len(),
        "30.0 dB".len() + 1,
        "nothing replaced: {typed:?}"
    );
}

#[test]
fn eggshell_widens_eguis_double_click_window_to_its_own() {
    let d = harness();
    let window = d
        .ctx()
        .options(|options| options.input_options.max_double_click_delay);
    assert_eq!(Duration::from_secs_f64(window), eggshell_ui::DOUBLE_CLICK);
}

#[test]
fn an_app_that_sets_its_own_double_click_window_keeps_it() {
    let mut d = harness();
    d.ctx()
        .options_mut(|options| options.input_options.max_double_click_delay = 0.25);
    d.frames(2);
    let window = d
        .ctx()
        .options(|options| options.input_options.max_double_click_delay);
    assert_eq!(window, 0.25);
}

#[test]
fn opening_the_readout_from_the_keyboard_selects_its_value() {
    let mut d = harness();
    support::drive_past_bar(&mut d);
    d.press(Key::Tab).frames(5);
    assert_eq!(
        focused_node(&d).as_deref(),
        Some(READOUT),
        "precondition: Tab reaches the readout"
    );
    d.press(Key::Enter).frames(5);
    assert_eq!(typed_into(&mut d, READOUT, "45"), "45");
    d.press(Key::Enter).frames(5);
    assert_eq!(d.app().volume, 45.0);
}

#[test]
fn return_commits_what_was_typed_through_parse_range_and_step() {
    for (typed, committed) in [("55", 55.0), ("57%", 55.0), ("500", 100.0), ("-9", 0.0)] {
        let mut d = harness();
        open(&mut d);
        d.press(Key::A).frames(5);
        d.harness()
            .key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
        d.type_text(typed).frames(3);
        assert!(
            d.app().changes.is_empty(),
            "nothing is applied while typing"
        );
        d.press(Key::Enter).frames(5);
        assert_eq!(d.app().volume, committed, "{typed:?}");
        assert_eq!(d.app().changes, [committed]);
        assert!(focused_node(&d).is_none(), "the field closes");
    }
}

#[test]
fn text_that_does_not_parse_changes_nothing() {
    let mut d = harness();
    open(&mut d);
    d.harness()
        .key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    d.type_text("loud").frames(3);
    d.press(Key::Enter).frames(5);
    assert!(d.app().changes.is_empty());
    let readout = d.rect(Target::suffix(READOUT));
    text_color(&d, readout.expand(1.0), "40%");
}

#[test]
fn escape_reverts_what_was_typed() {
    let mut d = harness();
    open(&mut d);
    d.harness()
        .key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    d.type_text("10").frames(3);
    d.press(Key::Escape).frames(5);
    assert!(d.app().changes.is_empty());
    text_color(&d, d.rect(Target::suffix(READOUT)).expand(1.0), "40%");
}

#[test]
fn a_click_away_commits_what_was_typed() {
    let mut d = harness();
    open(&mut d);
    d.harness()
        .key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    d.type_text("10").frames(3);
    assert!(d.app().changes.is_empty(), "nothing while typing");
    d.click(Target::suffix("/\"elsewhere\"")).frames(6);
    assert_eq!(d.app().changes, [10.0]);
    text_color(&d, d.rect(Target::suffix(READOUT)).expand(1.0), "10%");
}

#[test]
fn a_readout_that_is_not_editable_takes_no_focus() {
    let mut d = harness();
    d.click(Target::suffix("/\"plain\"/Box/Header/Label"))
        .frames(4);
    assert!(focused_node(&d).is_none());
}

#[test]
fn dragging_the_thumb_of_a_slider_with_a_header_moves_the_value() {
    let mut d = harness();
    let track = d.rect(Target::suffix(TRACK));
    let thumb = d.shell().style().slider.sizes.get(Size::DEFAULT).thumb_size;
    let start = egui::pos2(
        track.left() + thumb / 2.0 + 0.4 * (track.width() - thumb),
        track.center().y,
    );
    let end = egui::pos2(track.right() - thumb / 2.0 - 10.0, track.center().y);
    d.drag(start, end);
    let value = d.app().volume;
    assert!(value > 80.0, "{value}");
}

#[test]
fn the_readout_opens_for_editing_at_every_size() {
    for (path, size) in [
        (READOUT, Size::DEFAULT),
        ("/\"sm\"/\"small\"/Box/Header/Readout", size::SM),
        ("/\"xs\"/\"tiny\"/Box/Header/Readout", size::XS),
    ] {
        let mut d = harness();
        d.click(Target::suffix(path)).frames(6);
        assert_eq!(
            focused_node(&d).as_deref(),
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
        let mut d = harness();
        let readout = track.replace("Track", "Header/Readout");
        let before = d.rect(Target::suffix(track));
        d.click(Target::suffix(&readout)).frames(6);
        assert_eq!(d.rect(Target::suffix(track)), before, "during {track}");
        d.press(Key::Escape).frames(5);
        assert_eq!(d.rect(Target::suffix(track)), before, "after {track}");
    }
}

#[test]
fn the_readouts_digits_are_as_wide_as_each_other() {
    let mut d = harness();
    let mut widths = vec![];
    for value in [80.0, 10.0, 45.0, 11.0] {
        d.app_mut().volume = value;
        d.frames(4);
        widths.push(d.rect(Target::suffix(READOUT)).width());
    }
    assert!(
        widths.windows(2).all(|w| (w[0] - w[1]).abs() < 0.25),
        "{widths:?}"
    );
}

/// Where the first glyph of the galley painted with `text` lands on screen:
/// its left edge and its baseline, after the tessellator snaps the galley to
/// whole pixels.
fn first_glyph(d: &Driver<App>, text: &str) -> egui::Pos2 {
    use egui::emath::GuiRounding as _;
    let painted = d.painted().expect(text);
    let (galley, at) = (painted.galley(), painted.pos());
    assert!(
        d.ctx()
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
    at.round_to_pixels(d.ctx().pixels_per_point()) + row.pos.to_vec2() + glyph.pos.to_vec2()
}

#[test]
fn the_open_field_draws_the_value_where_the_readout_drew_it_at_every_size() {
    let readouts = [
        (READOUT, Size::DEFAULT, "40%"),
        (SMALL, size::SM, "30.0 dB"),
        (TINY, size::XS, "30"),
    ];
    let sizes: Vec<Size> = harness().shell().style().slider.sizes.keys().collect();
    assert!(
        sizes
            .iter()
            .all(|size| readouts.iter().any(|(_, covered, _)| covered == size)),
        "precondition: every slider size is covered: {sizes:?}"
    );
    for (readout, size, text) in readouts {
        let mut d = harness();
        let closed = first_glyph(&d, text);
        d.click(Target::suffix(readout)).frames(6);
        assert_eq!(
            focused_node(&d),
            Some(format!("{readout}/Field")),
            "precondition: the field is open at {size:?}"
        );
        let open = first_glyph(&d, text);
        assert!(
            (open - closed).length() < 0.01,
            "{size:?}: closed at {closed:?}, open at {open:?}"
        );
    }
}

const CURRENT: &str = "/\"current\"/Box/Header/Readout";

/// The current slider at 6.04, its readout "6.0 A" opened into a field.
fn current_open() -> Driver<App> {
    let mut d = harness();
    d.painted().expect("6.0 A");
    d.click(Target::suffix(CURRENT)).frames(6);
    assert_eq!(
        focused_node(&d),
        Some(format!("{CURRENT}/Field")),
        "precondition: the field is open"
    );
    d
}

#[test]
fn return_on_the_text_as_it_opened_writes_nothing() {
    let mut d = current_open();
    d.press(Key::Enter).frames(5);
    assert!(focused_node(&d).is_none(), "the field closes");
    let app = &d.app();
    assert_eq!(app.current, 6.04);
    assert!(app.current_changes.is_empty(), "{:?}", app.current_changes);
}

#[test]
fn return_after_typing_still_commits() {
    let mut d = current_open();
    d.harness()
        .key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    d.type_text("6.5").frames(3);
    d.press(Key::Enter).frames(5);
    let app = &d.app();
    assert_eq!(app.current, 6.5);
    assert_eq!(app.current_changes, [6.5]);
}
