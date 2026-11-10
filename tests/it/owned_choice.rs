//! The owned radio group, switch, segmented control and the label that names
//! a control, through kittest's real pointer and key events and the
//! accessibility check the support harness switches on.

use bullseye::contract::Size;
use bullseye::owned::Enablement;
use bullseye::{Builder, Style, TextStyle, tw};
use bullseye_shadcn::{Shadcn, Tokens};
use egui::epaint::{CircleShape, Shape};
use egui::{Key, Rect, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, box_at, click_node, focused_path, rect_of, ring_at, shapes};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Unit {
    #[default]
    Metric,
    Imperial,
    Nautical,
}

const UNITS: [(Unit, &str); 3] = [
    (Unit::Metric, "Metric"),
    (Unit::Imperial, "Imperial"),
    (Unit::Nautical, "Nautical"),
];

const SEGMENTS: [(Unit, &str); 3] = [
    (Unit::Metric, "Km"),
    (Unit::Imperial, "Miles"),
    (Unit::Nautical, "Knots"),
];

#[derive(Default)]
struct App {
    unit: Unit,
    locked: Unit,
    on: bool,
    segment: Unit,
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <div role="radiogroup">Metric Imperial Nautical</div>
///   <div role="radiogroup" aria-disabled>…</div>
///   <label><button role="switch"></button> Airplane mode</label>
///   <label><button role="switch" disabled></button> Locked</label>
///   <button role="switch" aria-label="Bare switch"></button>
///   <div class="segmented">Metric Imperial Nautical</div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.radio_group("units", app.unit, &UNITS)
            .on_change(|app: &mut App, unit| app.unit = unit);
        b.radio_group("locked", app.locked, &UNITS)
            .enablement(Enablement::Disabled)
            .on_change(|app: &mut App, unit| app.locked = unit);
        b.switch("air", "Airplane mode", app.on)
            .bind(|app: &mut App| &mut app.on);
        b.switch("frozen", "Locked", true)
            .enablement(Enablement::Disabled)
            .bind(|app: &mut App| &mut app.on);
        b.switch_unlabelled("bare", false)
            .named("Bare switch")
            .on_toggle(|_: &mut App| {});
        b.segmented("seg", app.segment, &SEGMENTS)
            .on_change(|app: &mut App, unit| app.segment = unit);
    });
}

fn harness(theme: Theme) -> Harness<'static, Rig<App>> {
    harness_stepping(theme, 0.05)
}

fn harness_stepping(theme: Theme, step_dt: f32) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(theme);
    let mut harness = support::harness_stepped(
        vec2(500.0, 600.0),
        step_dt,
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

fn circle_in(harness: &Harness<'_, Rig<App>>, rect: Rect) -> Option<CircleShape> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Circle(circle) if rect.expand(0.5).contains(circle.center) => Some(*circle),
            _ => None,
        })
        .next_back()
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

fn circle_path(unit: Unit) -> String {
    format!("/\"units\"/Circle({unit:?})")
}

#[test]
fn radio_circles_and_dots_are_the_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        let harness = harness(theme);
        let radio = style(theme).radio;
        for (unit, _) in UNITS {
            let rect = rect_of(&harness, &circle_path(unit));
            assert_eq!(rect.size(), vec2(16.0, 16.0), "size-4");
            let shape = box_at(&harness, rect);
            let chosen = unit == Unit::Metric;
            let look = if chosen { radio.on } else { radio.off };
            assert_eq!(shape.fill, look.states.rest.fill, "{theme:?} {unit:?}");
            assert_eq!(shape.stroke.color, look.states.rest.border);
            assert_eq!(shape.stroke.width, 1.0);
            assert_eq!(shape.corner_radius, egui::CornerRadius::same(u8::MAX));
            match (chosen, circle_in(&harness, rect)) {
                (true, Some(dot)) => {
                    assert_eq!(dot.radius, 4.0, "size-2 dot");
                    assert_eq!(dot.fill, look.states.rest.text);
                    assert_eq!(dot.center, rect.center());
                }
                (false, None) => {}
                (chosen, dot) => panic!("{unit:?} chosen {chosen}: dot {dot:?}"),
            }
        }
    }
}

#[test]
fn clicking_a_circle_or_its_label_chooses_it() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, &circle_path(Unit::Imperial));
    assert_eq!(harness.state().app.unit, Unit::Imperial);
    click_node(&mut harness, "/\"units\"/Label(Nautical)");
    harness.run_steps(3);
    assert_eq!(
        harness.state().app.unit,
        Unit::Nautical,
        "the label forwards"
    );
    let rect = rect_of(&harness, &circle_path(Unit::Nautical));
    assert!(circle_in(&harness, rect).is_some(), "and the dot follows");
}

#[test]
fn arrow_keys_move_the_choice_and_the_focus_within_the_group() {
    let mut harness = harness(Theme::Light);
    tab_to(&mut harness, &circle_path(Unit::Metric));
    let step = |harness: &mut Harness<'_, Rig<App>>, key, expected: Unit| {
        harness.key_press(key);
        harness.run_steps(4);
        assert_eq!(harness.state().app.unit, expected, "{key:?}");
        assert_eq!(
            focused_path(harness).as_deref(),
            Some(circle_path(expected).as_str()),
            "{key:?} moves the focus with the choice"
        );
    };
    step(&mut harness, Key::ArrowDown, Unit::Imperial);
    step(&mut harness, Key::ArrowRight, Unit::Nautical);
    step(&mut harness, Key::ArrowDown, Unit::Metric);
    step(&mut harness, Key::ArrowUp, Unit::Nautical);
    step(&mut harness, Key::ArrowLeft, Unit::Imperial);

    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert_eq!(harness.state().app.unit, Unit::Imperial, "Space chooses");
}

#[test]
fn a_disabled_group_cannot_be_focused_or_changed() {
    let mut harness = harness(Theme::Light);
    for unit in [Unit::Imperial, Unit::Nautical] {
        click_node(&mut harness, &format!("/\"locked\"/Circle({unit:?})"));
        click_node(&mut harness, &format!("/\"locked\"/Label({unit:?})"));
    }
    assert_eq!(harness.state().app.locked, Unit::Metric);
    let mut visited = Vec::new();
    for _ in 0..16 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        visited.extend(focused_path(&harness));
    }
    assert!(
        visited.iter().all(|path| !path.contains("locked")),
        "{visited:?}"
    );
    let disabled: Vec<_> = harness
        .get_all_by_role(Role::RadioButton)
        .filter(|node| node.accesskit_node().is_disabled())
        .collect();
    assert_eq!(disabled.len(), 3);
}

#[test]
fn labels_name_their_controls_for_assistive_technology() {
    let harness = harness(Theme::Light);
    let radio = harness
        .get_all_by_role_and_label(Role::RadioButton, "Metric")
        .next()
        .expect("the first group's Metric");
    assert_eq!(
        radio.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    let labels: Vec<_> = radio.accesskit_node().labelled_by().collect();
    assert_eq!(labels.len(), 1, "named by exactly its label");
    assert_eq!(labels[0].role(), Role::Label);
    assert_eq!(labels[0].value().as_deref(), Some("Metric"));
    assert_eq!(
        harness
            .get_all_by_role_and_label(Role::RadioButton, "Imperial")
            .next()
            .expect("the first group's Imperial")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );

    let switch = harness.get_by_role_and_label(Role::Switch, "Airplane mode");
    assert_eq!(switch.accesskit_node().labelled_by().count(), 1);
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Switch, "Bare switch")
            .accesskit_node()
            .labelled_by()
            .count(),
        0,
        "named() names a switch without a label"
    );
    let frozen = harness.get_by_role_and_label(Role::Switch, "Locked");
    assert!(frozen.accesskit_node().is_disabled());
    assert_eq!(
        frozen.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

fn thumb(harness: &Harness<'_, Rig<App>>, path: &str) -> (Rect, CircleShape) {
    let rect = rect_of(harness, path);
    let thumb = circle_in(harness, rect).unwrap_or_else(|| panic!("no thumb in {rect:?}"));
    (rect, thumb)
}

#[test]
fn switch_track_and_thumb_are_the_styles_off_and_on() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        let switch = style(theme).switch;
        let metrics = *switch.sizes.get(Size::DEFAULT);
        for on in [false, true] {
            if on {
                click_node(&mut harness, "/\"air\"/Track");
                harness.run_steps(12);
            }
            let look = if on { switch.on } else { switch.off };
            let (rect, thumb) = thumb(&harness, "/\"air\"/Track");
            assert!((rect.size() - metrics.track).abs().max_elem() < 0.01);
            let track = box_at(&harness, rect);
            assert_eq!(track.fill, look.states.rest.fill, "{theme:?} on {on}");
            assert_eq!(track.corner_radius, egui::CornerRadius::same(u8::MAX));
            assert_eq!(thumb.fill, look.states.rest.text);
            assert_eq!(
                thumb.radius,
                if on {
                    metrics.thumb_on
                } else {
                    metrics.thumb_off
                } / 2.0
            );
            let inset = rect.height() / 2.0;
            let x = if on {
                rect.right() - inset
            } else {
                rect.left() + inset
            };
            assert!(
                (thumb.center.x - x).abs() < 0.6,
                "{theme:?} on {on}: {thumb:?}"
            );
            assert!((thumb.center.y - rect.center().y).abs() < 0.6);
        }
        assert_ne!(switch.on.states.rest.fill, switch.off.states.rest.fill);
    }
}

#[test]
fn the_thumb_slides_between_the_ends() {
    let mut harness = harness_stepping(Theme::Light, 0.016);
    let (rect, rest) = thumb(&harness, "/\"air\"/Track");
    click_node(&mut harness, "/\"air\"/Track");
    harness.run_steps(2);
    let (_, mid) = thumb(&harness, "/\"air\"/Track");
    harness.run_steps(12);
    let (_, end) = thumb(&harness, "/\"air\"/Track");
    assert!(
        rest.center.x < mid.center.x && mid.center.x < end.center.x,
        "{} < {} < {} in {rect:?}",
        rest.center.x,
        mid.center.x,
        end.center.x
    );
}

#[test]
fn a_switch_flips_from_the_track_the_label_and_the_keyboard() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, "/\"air\"/Track");
    assert!(harness.state().app.on);
    click_node(&mut harness, "/\"air\"/Label");
    assert!(!harness.state().app.on, "the label flips it too");

    tab_to(&mut harness, "/\"air\"/Track");
    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert!(harness.state().app.on, "Space");
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert!(!harness.state().app.on, "Enter");

    click_node(&mut harness, "/\"frozen\"/Track");
    click_node(&mut harness, "/\"frozen\"/Label");
    assert!(!harness.state().app.on, "a disabled switch is inert");
}

#[test]
fn a_focused_switch_shows_the_styles_ring() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        let style = style(theme);
        tab_to(&mut harness, "/\"air\"/Track");
        harness.run_steps(12);
        let rect = rect_of(&harness, "/\"air\"/Track");
        let ring = ring_at(&harness, rect).expect("a focus ring");
        assert_eq!(ring.stroke.width, style.focus.width);
        assert_eq!(ring.stroke.color, style.focus.color);
    }
}

fn cell_path(unit: Unit) -> String {
    format!("/\"seg\"/{unit:?}")
}

#[test]
fn segmented_track_and_cells_are_the_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        let harness = harness(theme);
        let seg = style(theme).segmented;
        let track = rect_of(&harness, "/\"seg\"");
        assert_eq!(track.height(), seg.height);
        assert_eq!(box_at(&harness, track).fill, seg.track_fill);
        let metric = rect_of(&harness, &cell_path(Unit::Metric));
        assert_eq!(metric.top() - track.top(), seg.track_padding);
        let selected = box_at(&harness, metric);
        assert_eq!(selected.fill, seg.selected.states.rest.fill, "{theme:?}");
        assert_eq!(
            selected.corner_radius,
            egui::CornerRadius::same(seg.cell_radius)
        );
        assert_eq!(selected.stroke.width, seg.cell_border_width);
        let imperial = rect_of(&harness, &cell_path(Unit::Imperial));
        assert!(
            !shapes(&harness).iter().any(|shape| matches!(
                shape,
                Shape::Rect(r) if r.rect.is_positive() && support::same(r.rect, imperial) && r.fill != egui::Color32::TRANSPARENT
            )),
            "an unselected cell has no fill"
        );
    }
}

#[test]
fn segmented_has_a_tab_stop_per_cell_and_enter_chooses() {
    let mut harness = harness(Theme::Light);
    let mut stops = Vec::new();
    for _ in 0..16 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        stops.extend(focused_path(&harness).filter(|path| path.starts_with("/\"seg\"/")));
    }
    assert_eq!(
        &stops[..3],
        [Unit::Metric, Unit::Imperial, Unit::Nautical].map(cell_path)
    );

    tab_to(&mut harness, &cell_path(Unit::Imperial));
    harness.run_steps(12);
    let seg = style(Theme::Light).segmented;
    let ring = ring_at(&harness, rect_of(&harness, &cell_path(Unit::Imperial)))
        .expect("the focused cell has a ring");
    assert_eq!(ring.stroke.width, 3.0);
    let _ = seg;
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(harness.state().app.segment, Unit::Imperial);

    click_node(&mut harness, &cell_path(Unit::Nautical));
    harness.run_steps(3);
    assert_eq!(harness.state().app.segment, Unit::Nautical);
    let selected: Vec<_> = harness
        .get_all_by_role(Role::RadioButton)
        .filter(|node| node.accesskit_node().label().as_deref() == Some("Knots"))
        .map(|node| node.accesskit_node().toggled())
        .collect();
    assert_eq!(selected, [Some(egui::accesskit::Toggled::True)]);
}

#[test]
fn installing_a_modified_style_changes_every_owned_widget() {
    let mut style = style(Theme::Light);
    let red = egui::Color32::from_rgb(200, 30, 30);
    style.switch.on.states.rest.fill = red;
    style.switch.sizes.get_mut(Size::DEFAULT).unwrap().track = vec2(60.0, 30.0);
    style.radio.size = 24.0;
    style.segmented.track_fill = red;
    let mut harness = harness(Theme::Light);
    style.install(&harness.ctx);
    harness.run_steps(4);
    click_node(&mut harness, "/\"air\"/Track");
    harness.run_steps(12);
    assert!(harness.state().app.on, "precondition: the switch is on");
    harness.hover_at(egui::pos2(490.0, 590.0));
    harness.run_steps(12);

    let track = rect_of(&harness, "/\"air\"/Track");
    assert_eq!(track.size(), vec2(60.0, 30.0));
    assert_eq!(box_at(&harness, track).fill, red);
    assert_eq!(
        rect_of(&harness, &circle_path(Unit::Metric)).size(),
        vec2(24.0, 24.0)
    );
    assert_eq!(box_at(&harness, rect_of(&harness, "/\"seg\"")).fill, red);
}

#[test]
fn a_radio_group_is_one_tab_stop_on_the_chosen_option() {
    let mut harness = harness(Theme::Light);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(circle_path(Unit::Metric).as_str()),
        "Tab enters the group on the chosen option"
    );
    let mut visited = Vec::new();
    for _ in 0..16 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        visited.extend(focused_path(&harness));
    }
    assert!(
        visited
            .iter()
            .filter(|path| path.contains("Circle"))
            .all(|path| *path == circle_path(Unit::Metric)),
        "a full cycle of Tab stops visits no other circle: {visited:?}"
    );

    tab_to(&mut harness, &circle_path(Unit::Metric));
    harness.key_press(Key::ArrowDown);
    harness.run_steps(4);
    assert_eq!(harness.state().app.unit, Unit::Imperial);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(
        !focused_path(&harness).is_some_and(|path| path.contains("Circle")),
        "Tab leaves from the moved choice"
    );
    harness.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(circle_path(Unit::Imperial).as_str()),
        "Shift+Tab comes back to the chosen option"
    );
}

#[test]
fn tab_enters_a_group_on_its_chosen_option_wherever_it_is() {
    let mut harness = harness(Theme::Light);
    harness.state_mut().app.unit = Unit::Nautical;
    harness.run_steps(3);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(circle_path(Unit::Nautical).as_str())
    );
}

#[test]
fn the_switch_track_eases_to_its_new_colour() {
    let mut harness = harness_stepping(Theme::Light, 0.016);
    let switch = style(Theme::Light).switch;
    let (off, on) = (switch.off.states.rest.fill, switch.on.states.rest.fill);
    assert_ne!(off, on);
    let track = rect_of(&harness, "/\"air\"/Track");
    assert_eq!(box_at(&harness, track).fill, off);
    click_node(&mut harness, "/\"air\"/Track");
    harness.run_steps(2);
    let mid = box_at(&harness, track).fill;
    assert!(
        mid != off && mid != on,
        "{mid:?} between {off:?} and {on:?}"
    );
    harness.run_steps(12);
    assert_eq!(box_at(&harness, track).fill, on);
    click_node(&mut harness, "/\"air\"/Track");
    harness.run_steps(2);
    let back = box_at(&harness, track).fill;
    assert!(back != off && back != on, "and back again");
    harness.run_steps(12);
    assert_eq!(box_at(&harness, track).fill, off);
}

#[test]
fn the_switch_thumb_keeps_its_rest_colour_when_hovered_or_pressed() {
    let red = egui::Color32::from_rgb(200, 30, 30);
    let mut style = style(Theme::Light);
    style.switch.off.states.hover.text = red;
    style.switch.off.states.pressed.text = red;
    let mut harness = harness(Theme::Light);
    style.clone().install(&harness.ctx);
    harness.run_steps(4);
    let rest = style.switch.off.states.rest.text;
    let (_, at_rest) = thumb(&harness, "/\"air\"/Track");
    assert_eq!(at_rest.fill, rest);
    support::hover_node(&mut harness, "/\"air\"/Track");
    harness.run_steps(12);
    assert_eq!(thumb(&harness, "/\"air\"/Track").1.fill, rest, "hovered");
    support::press(&mut harness, "/\"air\"/Track");
    assert_eq!(thumb(&harness, "/\"air\"/Track").1.fill, rest, "pressed");
    support::release(&mut harness, "/\"air\"/Track");
}
