//! The owned slider, select and stepper, through kittest's real pointer and
//! key events and the accessibility check the support harness switches on.

use bullseye::owned::{Enablement, Select, Slider, Step, Stepper};
use bullseye::{Builder, Style, TextStyle, tw};
use bullseye_shadcn::{Shadcn, Tokens};
use egui::epaint::Shape;
use egui::{Key, Modifiers, Rect, Role, Theme, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, box_at, click_node, drag, focused_path, rect_of, ring_at, shapes};

const REGIONS: [(usize, &str); 3] = [(0, "Europe"), (1, "North America"), (2, "Asia Pacific")];

struct App {
    volume: f64,
    locked: f64,
    region: usize,
    count: f64,
    changes: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            volume: 40.0,
            locked: 40.0,
            region: 0,
            count: 3.0,
            changes: 0,
        }
    }
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4 w-[300px]">
///   <input type="range" min="0" max="100" step="5" value="40" aria-label="Volume">
///   <input type="range" disabled aria-label="Locked">
///   <button role="combobox" class="w-[200px]">Europe</button>
///   <button role="combobox" class="w-[200px]" disabled>Europe</button>
///   <input type="number" min="0" max="10" aria-label="Count">
///   <input type="number" disabled aria-label="Fixed">
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_stretch()
            .gap(4.0)
            .p(4.0)
            .w_px(300.0),
        |b| {
            b.slider(
                "volume",
                Slider::new(app.volume, 0.0..=100.0)
                    .step(Step::new(5.0))
                    .named("Volume"),
                |app: &mut App, value| {
                    app.volume = value;
                    app.changes += 1;
                },
            );
            b.slider(
                "locked",
                Slider::new(app.locked, 0.0..=100.0)
                    .named("Locked")
                    .enablement(Enablement::Disabled),
                |app: &mut App, value| app.locked = value,
            );
            b.select(
                "region",
                Select::new(app.region, &REGIONS)
                    .named("Region")
                    .classes(tw().w_px(200.0)),
                |app: &mut App, _, region| app.region = region,
            );
            b.select(
                "frozen",
                Select::new(0, &REGIONS)
                    .named("Frozen")
                    .classes(tw().w_px(200.0))
                    .enablement(Enablement::Disabled),
                |_: &mut App, _, _| {},
            );
            b.stepper(
                "count",
                Stepper::new(app.count, 0.0..=10.0).named("Count"),
                |app: &mut App, value| app.count = value,
            );
            b.stepper(
                "fixed",
                Stepper::new(5.0, 0.0..=10.0)
                    .named("Fixed")
                    .enablement(Enablement::Disabled),
                |_: &mut App, _| {},
            );
        },
    );
}

fn harness(theme: Theme) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(theme);
    let mut harness = support::harness(
        vec2(400.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    );
    harness.ctx.set_theme(theme);
    harness.run_steps(12);
    harness
}

fn style(theme: Theme) -> Style {
    Shadcn::for_theme(theme).style()
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..16 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(harness));
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(3);
}

const TRACK: &str = "/\"volume\"/Track";
const THUMB: &str = "/\"volume\"/Track/Thumb";

/// The `n`th node of `path`: anonymous nodes share their parent's path.
fn nth(harness: &Harness<'_, Rig<App>>, path: &str, n: usize) -> Rect {
    harness
        .state()
        .nodes
        .iter()
        .filter(|(candidate, _)| candidate == path)
        .nth(n)
        .unwrap_or_else(|| panic!("no node {n} of {path}"))
        .1
}

fn bar(harness: &Harness<'_, Rig<App>>, slider: &str) -> Rect {
    nth(harness, &format!("/\"{slider}\"/Track/<div>"), 0)
}

fn range(harness: &Harness<'_, Rig<App>>, slider: &str) -> Rect {
    nth(harness, &format!("/\"{slider}\"/Track/<div>"), 1)
}

#[test]
fn slider_track_range_and_thumb_are_the_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        let harness = harness(theme);
        let slider = style(theme).slider;
        let track = rect_of(&harness, TRACK);
        assert_eq!(track.height(), slider.thumb_size);
        let bar = bar(&harness, "volume");
        assert_eq!(bar.height(), slider.track_height, "h-1.5");
        assert_eq!(bar.width(), track.width());
        let bar_shape = box_at(&harness, bar);
        assert_eq!(bar_shape.fill, slider.track.rest.fill, "{theme:?}");
        assert_eq!(bar_shape.corner_radius, egui::CornerRadius::same(u8::MAX));

        let range = range(&harness, "volume");
        assert!((range.width() - 0.4 * bar.width()).abs() < 0.6, "{range:?}");
        assert_eq!(box_at(&harness, range).fill, slider.range.rest.fill);

        let thumb = rect_of(&harness, THUMB);
        assert_eq!(thumb.size(), vec2(16.0, 16.0), "size-4");
        let centre = track.left() + 8.0 + 0.4 * (track.width() - 16.0);
        assert!((thumb.center().x - centre).abs() < 0.6, "{thumb:?}");
        assert!((thumb.center().y - track.center().y).abs() < 0.6);
        let shape = box_at(&harness, thumb);
        let look = slider.thumb;
        assert_eq!(shape.fill, look.paint.states.rest.fill);
        assert_eq!(shape.stroke.color, look.paint.states.rest.border);
        assert_eq!(shape.stroke.width, 1.0);
        assert_eq!(shape.corner_radius, egui::CornerRadius::same(u8::MAX));
        assert_ne!(slider.range.rest.fill, slider.track.rest.fill);
    }
}

#[test]
fn the_range_is_round_where_the_track_is_round_at_every_value() {
    for value in [2.0, 40.0, 100.0] {
        let mut harness = harness(Theme::Light);
        harness.state_mut().app.volume = value;
        harness.run_steps(4);
        let bar_rect = bar(&harness, "volume");
        let range_rect = range(&harness, "volume");
        assert!(range_rect.width() > 0.0, "precondition: a range is drawn");
        assert!(
            range_rect.height() == bar_rect.height(),
            "precondition: the range is as tall as the track"
        );
        assert_eq!(
            box_at(&harness, range_rect).corner_radius,
            box_at(&harness, bar_rect).corner_radius,
            "value {value}"
        );
    }
}

#[test]
fn a_disabled_thumb_hides_the_track_beneath_it() {
    for theme in [Theme::Light, Theme::Dark] {
        let harness = harness(theme);
        let bar = bar(&harness, "locked");
        let thumb = rect_of(&harness, "/\"locked\"/Track/Thumb");
        assert!(
            thumb.intersects(bar) && thumb.height() >= bar.height(),
            "precondition: the thumb lies over the track's full height"
        );
        let shape = box_at(&harness, thumb);
        assert_eq!(shape.fill.a(), 255, "{theme:?} thumb fill");
        assert_eq!(shape.stroke.color.a(), 255, "{theme:?} thumb border");
        assert_eq!(shape.corner_radius, egui::CornerRadius::same(u8::MAX));
    }
}

#[test]
fn pressing_the_track_jumps_the_thumb_and_dragging_follows_the_pointer() {
    let mut harness = harness(Theme::Light);
    let track = rect_of(&harness, TRACK);
    let at = |fraction: f32| {
        pos2(
            track.left() + 8.0 + fraction * (track.width() - 16.0),
            track.center().y,
        )
    };
    support::click_node(&mut harness, TRACK);
    assert_eq!(harness.state().app.volume, 50.0, "the middle of the track");

    support::press_and_release(&mut harness, at(0.75));
    harness.run_steps(3);
    assert_eq!(harness.state().app.volume, 75.0, "a click on the track");

    let thumb = rect_of(&harness, THUMB).center();
    drag(&mut harness, thumb, at(0.2));
    assert_eq!(harness.state().app.volume, 20.0, "dragged by the thumb");

    drag(&mut harness, at(0.2), at(1.0) + vec2(80.0, 40.0));
    assert_eq!(harness.state().app.volume, 100.0, "clamped at the end");
    drag(&mut harness, at(1.0), at(0.0) - vec2(80.0, 0.0));
    assert_eq!(harness.state().app.volume, 0.0, "clamped at the start");
    let thumb = rect_of(&harness, THUMB);
    assert!((thumb.left() - track.left()).abs() < 0.6, "{thumb:?}");

    drag(&mut harness, at(0.0), at(0.5) + vec2(0.0, 0.0));
    let value = harness.state().app.volume;
    assert_eq!(value % 5.0, 0.0, "values snap to the step");
}

#[test]
fn slider_keys_step_page_and_jump_to_the_ends() {
    let mut harness = harness(Theme::Light);
    tab_to(&mut harness, THUMB);
    let volume = |harness: &Harness<'_, Rig<App>>| harness.state().app.volume;
    press(&mut harness, Key::ArrowRight);
    assert_eq!(volume(&harness), 45.0);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(volume(&harness), 50.0);
    press(&mut harness, Key::ArrowLeft);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(volume(&harness), 40.0);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(THUMB),
        "arrows do not move the focus"
    );
    press(&mut harness, Key::PageUp);
    assert_eq!(volume(&harness), 90.0, "ten steps");
    press(&mut harness, Key::PageUp);
    assert_eq!(volume(&harness), 100.0, "clamped");
    press(&mut harness, Key::Home);
    assert_eq!(volume(&harness), 0.0);
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(volume(&harness), 0.0, "clamped at the start");
    press(&mut harness, Key::End);
    assert_eq!(volume(&harness), 100.0);
    press(&mut harness, Key::PageDown);
    assert_eq!(volume(&harness), 50.0);
}

#[test]
fn a_focused_thumb_shows_the_ring_and_the_slider_is_one_tab_stop() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        let style = style(theme);
        tab_to(&mut harness, THUMB);
        harness.run_steps(12);
        let ring = ring_at(&harness, rect_of(&harness, THUMB)).expect("a ring on the thumb");
        assert_eq!(ring.stroke.width, style.focus.width);
        assert_eq!(ring.stroke.color, style.focus.color);
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert_ne!(
            focused_path(&harness).as_deref(),
            Some(TRACK),
            "the track is not a stop"
        );
    }
}

#[test]
fn a_disabled_slider_ignores_the_pointer_and_the_keyboard() {
    let mut harness = harness(Theme::Light);
    let track = rect_of(&harness, "/\"locked\"/Track");
    support::press_and_release(&mut harness, track.right_center() - vec2(10.0, 0.0));
    drag(&mut harness, track.center(), track.left_center());
    assert_eq!(harness.state().app.locked, 40.0);
    for _ in 0..24 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        let focus = focused_path(&harness);
        assert!(
            !focus.is_some_and(|path| path.contains("locked")),
            "not a Tab stop"
        );
    }
    let slider = style(Theme::Light).slider;
    let bar = bar(&harness, "locked");
    assert_eq!(box_at(&harness, bar).fill, slider.track.disabled.fill);
    let thumb = rect_of(&harness, "/\"locked\"/Track/Thumb");
    assert_eq!(
        box_at(&harness, thumb).fill,
        slider.thumb.paint.states.disabled.fill
    );
    assert!(
        harness
            .get_by_role_and_label(Role::Slider, "Locked")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_slider_is_a_slider_with_a_value_for_assistive_technology() {
    let mut harness = harness(Theme::Light);
    let node = harness.get_by_role_and_label(Role::Slider, "Volume");
    let access = node.accesskit_node();
    assert_eq!(access.numeric_value(), Some(40.0));
    assert_eq!(access.min_numeric_value(), Some(0.0));
    assert_eq!(access.max_numeric_value(), Some(100.0));
    tab_to(&mut harness, THUMB);
    press(&mut harness, Key::ArrowRight);
    let node = harness.get_by_role_and_label(Role::Slider, "Volume");
    assert_eq!(node.accesskit_node().numeric_value(), Some(45.0));
}

const REGION: &str = "/\"region\"";

fn is_open(harness: &Harness<'_, Rig<App>>) -> bool {
    harness
        .query_by_role_and_label(Role::ListBoxOption, "Asia Pacific")
        .is_some()
}

fn chevron_in(harness: &Harness<'_, Rig<App>>, rect: Rect) -> Option<egui::epaint::PathShape> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Path(path) if path.points.iter().all(|p| rect.contains(*p)) => {
                Some(path.clone())
            }
            _ => None,
        })
        .next_back()
}

#[test]
fn select_trigger_is_an_input_like_box_with_a_value_and_a_chevron() {
    for theme in [Theme::Light, Theme::Dark] {
        let harness = harness(theme);
        let select = style(theme).select;
        let trigger = rect_of(&harness, REGION);
        assert_eq!(trigger.size(), vec2(200.0, 36.0), "h-9");
        let shape = box_at(&harness, trigger);
        let look = select.trigger;
        assert_eq!(shape.fill, look.paint.states.rest.fill, "{theme:?}");
        assert_eq!(shape.stroke.color, look.paint.states.rest.border);
        assert_eq!(shape.stroke.width, 1.0);
        assert_eq!(
            shape.corner_radius,
            egui::CornerRadius::same(select.metrics.radius)
        );
        let text = rect_of(&harness, "/\"region\"/<text>");
        assert_eq!(text.left() - trigger.left(), 1.0 + select.metrics.padding_x);
        assert_eq!(
            support::text_color(&harness, trigger, "Europe"),
            look.paint.states.rest.text
        );
        let glyph = rect_of(&harness, "/\"region\"/<paint>");
        assert_eq!(glyph.size(), vec2(16.0, 16.0), "size-4");
        assert_eq!(
            trigger.right() - glyph.right(),
            1.0 + select.metrics.padding_x
        );
        let chevron = chevron_in(&harness, glyph).expect("a chevron");
        assert_eq!(
            chevron.stroke.color,
            egui::epaint::ColorMode::Solid(look.paint.states.rest.text.gamma_multiply(0.5)),
            "opacity-50"
        );
        let frozen = rect_of(&harness, "/\"frozen\"");
        assert_eq!(
            box_at(&harness, frozen).fill,
            look.paint.states.disabled.fill
        );
    }
}

#[test]
fn clicking_the_trigger_opens_the_list_and_clicking_an_option_chooses_it() {
    let mut harness = harness(Theme::Light);
    assert!(!is_open(&harness));
    click_node(&mut harness, REGION);
    harness.run_steps(4);
    assert!(is_open(&harness), "a click opens");

    let list = style(Theme::Light).select;
    let colors = list.popup.paint.states.rest;
    assert!(
        shapes(&harness).iter().any(|shape| matches!(
            shape,
            Shape::Rect(r) if r.fill == colors.fill && r.stroke.color == colors.border
                && r.corner_radius == egui::CornerRadius::same(list.popup_radius)
        )),
        "the list is drawn in the popup colours"
    );
    assert!(
        shapes(&harness).iter().any(|shape| matches!(
            shape, Shape::Rect(r) if r.fill == list.item.paint.states.hover.fill
        )),
        "the highlighted item is drawn in the item colours"
    );

    let option = harness
        .get_by_role_and_label(Role::ListBoxOption, "North America")
        .rect();
    support::hover_over(&mut harness, option.center());
    support::press_and_release(&mut harness, option.center());
    harness.run_steps(4);
    assert_eq!(harness.state().app.region, 1);
    assert!(!is_open(&harness), "choosing closes the list");
    assert!(
        harness
            .query_by_role_and_label(Role::ComboBox, "Region")
            .is_some()
    );

    click_node(&mut harness, REGION);
    harness.run_steps(4);
    assert!(is_open(&harness));
    support::press_and_release(&mut harness, pos2(390.0, 490.0));
    harness.run_steps(4);
    assert!(!is_open(&harness), "a click outside closes it");
    assert_eq!(harness.state().app.region, 1);
}

#[test]
fn select_keyboard_opens_moves_chooses_and_closes() {
    let mut harness = harness(Theme::Light);
    tab_to(&mut harness, REGION);
    for key in [Key::Space, Key::Enter, Key::ArrowDown] {
        press(&mut harness, key);
        assert!(is_open(&harness), "{key:?} opens");
        press(&mut harness, Key::Escape);
        assert!(!is_open(&harness), "Escape closes");
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(REGION),
            "and the trigger keeps the focus"
        );
        assert_eq!(harness.state().app.region, 0);
    }

    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowUp);
    assert!(is_open(&harness));
    assert_eq!(harness.state().app.region, 0, "moving only highlights");
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.region, 1, "Enter chooses");
    assert!(!is_open(&harness));

    press(&mut harness, Key::Space);
    press(&mut harness, Key::End);
    press(&mut harness, Key::Space);
    assert_eq!(harness.state().app.region, 2, "Space chooses too");
    assert!(!is_open(&harness));
    assert_eq!(focused_path(&harness).as_deref(), Some(REGION));
}

#[test]
fn a_select_trigger_does_not_shift_when_pressed_and_is_a_combobox() {
    let mut harness = harness(Theme::Light);
    let trigger = rect_of(&harness, REGION);
    let rest = support::text_color(&harness, trigger, "Europe");
    let top = |harness: &Harness<'_, Rig<App>>| {
        shapes(harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == "Europe" => Some(t.pos.y),
                _ => None,
            })
            .expect("the value")
    };
    let before = top(&harness);
    support::press(&mut harness, REGION);
    assert_eq!(top(&harness), before, "opens_popup: no 1px press shift");
    support::release(&mut harness, REGION);
    let _ = rest;

    let node = harness.get_by_role_and_label(Role::ComboBox, "Region");
    assert_eq!(node.accesskit_node().value().as_deref(), Some("Europe"));
    let frozen = harness.get_by_role_and_label(Role::ComboBox, "Frozen");
    assert!(frozen.accesskit_node().is_disabled());
}

#[test]
fn a_disabled_select_cannot_be_opened_or_focused() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, "/\"frozen\"");
    harness.run_steps(3);
    assert!(!is_open(&harness));
    for _ in 0..24 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert_ne!(focused_path(&harness).as_deref(), Some("/\"frozen\""));
    }
}

const FIELD: &str = "/\"count\"/Box/Field";
const UP: &str = "/\"count\"/Box/Buttons/Up";
const DOWN: &str = "/\"count\"/Box/Buttons/Down";

fn shows(harness: &Harness<'_, Rig<App>>, text: &str) -> bool {
    shapes(harness)
        .into_iter()
        .any(|shape| matches!(shape, Shape::Text(t) if t.galley.text() == text))
}

#[test]
fn stepper_is_one_bordered_group_with_a_column_of_buttons() {
    for theme in [Theme::Light, Theme::Dark] {
        let harness = harness(theme);
        let stepper = style(theme).stepper;
        let group = rect_of(&harness, "/\"count\"/Box");
        assert_eq!(group.height(), stepper.height);
        let shape = box_at(&harness, group);
        assert_eq!(
            shape.fill, stepper.group.paint.states.rest.fill,
            "{theme:?}"
        );
        assert_eq!(shape.stroke.color, stepper.group.paint.states.rest.border);
        assert_eq!(
            shape.corner_radius,
            egui::CornerRadius::same(stepper.radius)
        );
        let (up, down) = (rect_of(&harness, UP), rect_of(&harness, DOWN));
        assert_eq!(up.width(), stepper.button_width);
        assert_eq!(down.width(), stepper.button_width);
        assert_eq!(down.top() - up.bottom(), 1.0, "a divider between");
        let border = stepper.group.border_width;
        assert_eq!(up.top() - group.top(), border, "inside the border");
        assert_eq!(group.bottom() - down.bottom(), border);
        assert_eq!(group.right() - up.right(), border);
        assert!(group.contains_rect(rect_of(&harness, FIELD)));
        assert!(rect_of(&harness, FIELD).right() <= up.left() - 1.0);
        assert_eq!(
            box_at(&harness, up).fill,
            stepper.button.paint.states.rest.fill
        );
        let inner = f32::from(stepper.radius) - border;
        let (up_corners, down_corners) = (
            box_at(&harness, up).corner_radius,
            box_at(&harness, down).corner_radius,
        );
        assert_eq!((up_corners.ne, up_corners.se), (inner as u8, 0));
        assert_eq!((down_corners.ne, down_corners.se), (0, inner as u8));
        assert!(chevron_in(&harness, up).is_some());
        assert!(shows(&harness, "3"));
    }
}

#[test]
fn a_focused_stepper_rings_the_group_and_never_crosses_a_button() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        let stepper = style(theme).stepper;
        let group = rect_of(&harness, "/\"count\"/Box");
        assert!(ring_at(&harness, group).is_none(), "no ring at rest");
        click_node(&mut harness, FIELD);
        harness.run_steps(12);
        let ring = ring_at(&harness, group).expect("a ring around the group");
        assert_eq!(ring.stroke.width, 3.0);
        assert_eq!(
            box_at(&harness, group).stroke.color,
            stepper.group.paint.states.focused.border,
            "the group's border takes the ring colour"
        );
        for path in [UP, DOWN] {
            let button = rect_of(&harness, path);
            assert!(ring.rect.contains_rect(button), "{path} is enclosed");
        }
        for shape in shapes(&harness) {
            let Shape::Rect(r) = shape else { continue };
            if r.stroke_kind != egui::StrokeKind::Outside || r.stroke.width == 0.0 {
                continue;
            }
            for path in [UP, DOWN] {
                let button = rect_of(&harness, path);
                let band = r.rect.expand(r.stroke.width);
                let crosses = band.intersects(button) && !r.rect.contains_rect(button);
                assert!(!crosses, "a ring at {:?} crosses {path}", r.rect);
            }
        }
    }
}

#[test]
fn a_disabled_stepper_group_is_painted_in_opaque_disabled_colours() {
    let harness = harness(Theme::Light);
    let stepper = style(Theme::Light).stepper;
    let group = box_at(&harness, rect_of(&harness, "/\"fixed\"/Box"));
    assert_eq!(group.fill, stepper.group.paint.states.disabled.fill);
    assert_eq!(
        group.stroke.color,
        stepper.group.paint.states.disabled.border
    );
    assert_eq!(group.fill.a(), 255);
    assert_eq!(group.stroke.color.a(), 255);
    let up = harness.get_by_role_and_label(Role::Button, "Increase Fixed");
    assert!(up.accesskit_node().is_disabled());
}

#[test]
fn the_buttons_step_the_value_and_disable_at_the_limits() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, UP);
    harness.run_steps(3);
    assert_eq!(harness.state().app.count, 4.0);
    assert!(shows(&harness, "4"));
    click_node(&mut harness, DOWN);
    click_node(&mut harness, DOWN);
    assert_eq!(harness.state().app.count, 2.0);

    for _ in 0..8 {
        click_node(&mut harness, DOWN);
    }
    assert_eq!(harness.state().app.count, 0.0);
    let down = harness.get_by_role_and_label(Role::Button, "Decrease Count");
    assert!(down.accesskit_node().is_disabled(), "at the lower limit");
    assert!(
        !harness
            .get_by_role_and_label(Role::Button, "Increase Count")
            .accesskit_node()
            .is_disabled()
    );
    let stepper = style(Theme::Light).stepper;
    assert_eq!(
        box_at(&harness, rect_of(&harness, DOWN)).fill,
        stepper.button.paint.states.disabled.fill
    );
    click_node(&mut harness, DOWN);
    assert_eq!(harness.state().app.count, 0.0);

    for _ in 0..12 {
        click_node(&mut harness, UP);
    }
    assert_eq!(harness.state().app.count, 10.0, "clamped to the range");
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Increase Count")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn arrow_keys_in_the_field_step_the_value() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, FIELD);
    harness.run_steps(3);
    assert_eq!(focused_path(&harness).as_deref(), Some("<widget>"));
    press(&mut harness, Key::ArrowUp);
    assert_eq!(harness.state().app.count, 4.0);
    press(&mut harness, Key::ArrowUp);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(harness.state().app.count, 2.0);
    assert!(shows(&harness, "2"));
    for _ in 0..4 {
        press(&mut harness, Key::ArrowDown);
    }
    assert_eq!(harness.state().app.count, 0.0, "clamped");
    for _ in 0..12 {
        press(&mut harness, Key::ArrowUp);
    }
    assert_eq!(harness.state().app.count, 10.0, "clamped");
}

#[test]
fn typing_sets_the_value_clamps_it_and_snaps_when_the_field_is_left() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, FIELD);
    harness.run_steps(3);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run_steps(2);
    harness.event(egui::Event::Text("7".into()));
    harness.run_steps(3);
    assert_eq!(harness.state().app.count, 7.0);
    harness.event(egui::Event::Text(".4".into()));
    harness.run_steps(3);
    assert_eq!(harness.state().app.count, 7.4, "applied as it parses");
    assert!(shows(&harness, "7.4"), "and the text stays as typed");

    harness.key_press(Key::Tab);
    harness.run_steps(4);
    assert_eq!(
        harness.state().app.count,
        7.0,
        "snapped to the step on leaving"
    );
    assert!(shows(&harness, "7"));

    click_node(&mut harness, FIELD);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.event(egui::Event::Text("99".into()));
    harness.run_steps(3);
    assert_eq!(harness.state().app.count, 10.0, "clamped");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.event(egui::Event::Text("-".into()));
    harness.run_steps(3);
    assert_eq!(
        harness.state().app.count,
        10.0,
        "text that is not a number is ignored"
    );
}

#[test]
fn a_stepper_is_a_spin_button_and_its_buttons_are_not_tab_stops() {
    let mut harness = harness(Theme::Light);
    let node = harness.get_by_role_and_label(Role::SpinButton, "Count");
    assert_eq!(node.accesskit_node().numeric_value(), Some(3.0));
    assert_eq!(node.accesskit_node().max_numeric_value(), Some(10.0));
    let mut visited = Vec::new();
    for _ in 0..24 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        visited.extend(focused_path(&harness));
    }
    assert!(visited.iter().any(|path| path == "<widget>"));
    assert!(
        visited
            .iter()
            .all(|path| !path.contains("Up") && !path.contains("Down") && !path.contains("fixed")),
        "{visited:?}"
    );
    let fixed = harness.get_by_role_and_label(Role::SpinButton, "Fixed");
    assert!(fixed.accesskit_node().is_disabled());
}

#[test]
fn installing_a_modified_style_changes_the_new_widgets_with_no_code_change() {
    let mut style = style(Theme::Light);
    let red = egui::Color32::from_rgb(200, 30, 30);
    style.slider.range.rest.fill = red;
    style.slider.track_height = 10.0;
    style.slider.thumb_size = 24.0;
    style.select.trigger.paint.states.rest.fill = red;
    style.select.metrics.height = 44.0;
    style.stepper.group.paint.states.rest.fill = red;
    style.stepper.button_width = 40.0;
    let mut harness = harness(Theme::Light);
    style.install(&harness.ctx);
    harness.run_steps(6);

    let track = rect_of(&harness, TRACK);
    assert_eq!(track.height(), 24.0);
    assert_eq!(bar(&harness, "volume").height(), 10.0);
    assert_eq!(box_at(&harness, range(&harness, "volume")).fill, red);
    assert!(
        (rect_of(&harness, THUMB).size() - vec2(24.0, 24.0))
            .abs()
            .max_elem()
            < 0.01
    );

    let trigger = rect_of(&harness, REGION);
    assert_eq!(trigger.height(), 44.0);
    assert_eq!(box_at(&harness, trigger).fill, red);

    assert_eq!(
        box_at(&harness, rect_of(&harness, "/\"count\"/Box")).fill,
        red
    );
    assert_eq!(rect_of(&harness, UP).width(), 40.0);
}

#[test]
fn steps_snap_to_multiples_from_the_start_and_count_their_decimals() {
    let range = 0.0..=1.0;
    let step = Step::new(0.25);
    assert_eq!(step.decimals(), 2);
    assert_eq!(step.snap(0.3, &range), 0.25);
    assert_eq!(step.snap(0.9, &range), 1.0);
    assert_eq!(step.snap(7.0, &range), 1.0);
    assert_eq!(step.moved(0.5, 1, &range), 0.75);
    assert_eq!(step.moved(0.0, -1, &range), 0.0);
    assert_eq!(Step::new(0.1).snap(0.30000000000000004, &range), 0.3);
    assert_eq!(Step::new(-3.0), Step::ONE);
    assert_eq!(Step::new(f64::NAN), Step::ONE);
    assert_eq!(Step::ONE.decimals(), 0);
}
