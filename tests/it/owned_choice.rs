//! The owned radio group, switch, segmented control and the label that names
//! a control, through kittest's real pointer and key events and the
//! accessibility check the support harness switches on.

use core::time::Duration;
use std::sync::Arc;

use eggshell_ui::contract::{Focus, Size, Themed};
use eggshell_ui::owned::{Cell, Enablement, Mark, SegmentedRows, SwitchPart, TabsPart};
use eggshell_ui::{Builder, Style, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::epaint::{CircleShape, Shape};
use egui::{Key, KeyboardShortcut, Rect, Role, Theme, vec2};

use eggshell_ui::testing::{Driver, Target};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, box_at, focused_path, press_and_release, ring_at, shapes};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Unit {
    #[default]
    Metric,
    Imperial,
    Nautical,
}

impl Unit {
    /// The position of the option, which keys its node.
    fn at(self) -> usize {
        self as usize
    }
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
        b.radio_group("units", app.unit, UNITS)
            .on_change(|app: &mut App, unit| app.unit = unit);
        b.radio_group("locked", app.locked, UNITS)
            .enablement(Enablement::Disabled)
            .on_change(|app: &mut App, unit| app.locked = unit);
        b.switch("air", "Airplane mode", app.on)
            .on_change(|app: &mut App, value| app.on = value);
        b.switch("frozen", "Locked", true)
            .enablement(Enablement::Disabled)
            .on_change(|app: &mut App, value| app.on = value);
        b.switch_unlabelled("bare", false)
            .named("Bare switch")
            .on_change(|_: &mut App, _| {});
        b.segmented("seg", app.segment, SEGMENTS)
            .on_change(|app: &mut App, unit| app.segment = unit);
    });
}

fn harness(theme: Theme) -> Driver<App> {
    harness_stepping(theme, 0.05)
}

fn harness_stepping(theme: Theme, step_dt: f32) -> Driver<App> {
    let tokens = Tokens::for_theme(theme);
    let mut d = support::driver(
        vec2(500.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .step(Duration::from_secs_f32(step_dt))
    .settle_within(Duration::from_secs_f32(step_dt) * 4)
    .build();
    d.ctx().set_theme(theme);
    d.frames(12);
    d
}

fn style(theme: Theme) -> Style {
    Shadcn::for_theme(theme).style()
}

/// The fills `style` gives an enabled switch's track and thumb, `on` or not.
fn switch_look(style: &Style, on: bool) -> (egui::Color32, egui::Color32) {
    let mark = if on { Mark::On } else { Mark::Off };
    let keys = SwitchPart::keys(Size::DEFAULT, mark, Enablement::Enabled);
    let fill = |part| {
        support::part_at(style, part, &keys)
            .paint_classes()
            .background
            .expect("filled")
    };
    (fill(SwitchPart::Root), fill(SwitchPart::Thumb))
}

fn circle_in(d: &Driver<App>, rect: Rect) -> Option<CircleShape> {
    shapes(d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Circle(circle) if rect.expand(0.5).contains(circle.center) => Some(*circle),
            _ => None,
        })
        .next_back()
}

/// The radio's dot drawn inside `rect`: a filled, fully rounded box smaller
/// than it.
fn dot_in(d: &Driver<App>, rect: Rect) -> Option<egui::epaint::RectShape> {
    shapes(d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r)
                if rect.expand(0.5).contains_rect(r.rect)
                    && r.rect.width() < rect.width() - 2.0
                    && r.fill.a() > 0
                    && r.corner_radius == egui::CornerRadius::same(u8::MAX) =>
            {
                Some(r.clone())
            }
            _ => None,
        })
        .next_back()
}

fn tab_to(d: &mut Driver<App>, path: &str) {
    for _ in 0..16 {
        if focused_path(d).as_deref() == Some(path) {
            return;
        }
        d.press(Key::Tab).frames(1);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(d));
}

fn circle_path(unit: Unit) -> String {
    format!("/\"units\"/Circle({})", unit.at())
}

#[test]
fn radio_circles_and_dots_are_shadcns() {
    // style-vega.css `.cn-radio-group-item`, `.cn-radio-group-indicator-icon`.
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let t = Tokens::for_theme(theme);
        for (unit, _) in UNITS {
            let rect = d.rect(Target::suffix(&circle_path(unit)));
            assert_eq!(rect.size(), vec2(16.0, 16.0), "size-4");
            let shape = box_at(&d, rect);
            let chosen = unit == Unit::Metric;
            let (fill, border) = match (chosen, theme) {
                (true, _) => (t.primary, t.primary),
                (false, Theme::Light) => (egui::Color32::TRANSPARENT, t.input),
                (false, Theme::Dark) => (t.input.gamma_multiply(0.3), t.input),
            };
            assert_eq!(shape.fill, fill, "{theme:?} {unit:?}");
            assert_eq!(shape.stroke.color, border, "{theme:?} {unit:?}");
            assert_eq!(shape.stroke.width, 1.0);
            assert_eq!(shape.corner_radius, egui::CornerRadius::same(u8::MAX));
            match (chosen, dot_in(&d, rect)) {
                (true, Some(dot)) => {
                    assert_eq!(dot.rect.size(), vec2(8.0, 8.0), "size-2");
                    assert_eq!(dot.fill, t.primary_foreground, "bg-primary-foreground");
                    // The dot snaps to the pixel grid; the circle need not.
                    let off = dot.rect.center() - rect.center();
                    assert!(off.x.abs() <= 0.5 && off.y.abs() <= 0.5, "centred: {off:?}");
                }
                (false, None) => {}
                (chosen, dot) => panic!("{unit:?} chosen {chosen}: dot {dot:?}"),
            }
        }
    }
}

#[test]
fn clicking_a_circle_or_its_label_chooses_it() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(&circle_path(Unit::Imperial)));
    assert_eq!(d.app().unit, Unit::Imperial);
    d.click(Target::suffix("/\"units\"/Label(2)")).frames(3);
    assert_eq!(d.app().unit, Unit::Nautical, "the label forwards");
    let rect = d.rect(Target::suffix(&circle_path(Unit::Nautical)));
    assert!(dot_in(&d, rect).is_some(), "and the dot follows");
}

#[test]
fn arrow_keys_move_the_choice_and_the_focus_within_the_group() {
    let mut d = harness(Theme::Light);
    tab_to(&mut d, &circle_path(Unit::Metric));
    let step = |d: &mut Driver<App>, key, expected: Unit| {
        d.press(key).frames(3);
        assert_eq!(d.app().unit, expected, "{key:?}");
        assert_eq!(
            focused_path(d).as_deref(),
            Some(circle_path(expected).as_str()),
            "{key:?} moves the focus with the choice"
        );
    };
    step(&mut d, Key::ArrowDown, Unit::Imperial);
    step(&mut d, Key::ArrowRight, Unit::Nautical);
    step(&mut d, Key::ArrowDown, Unit::Metric);
    step(&mut d, Key::ArrowUp, Unit::Nautical);
    step(&mut d, Key::ArrowLeft, Unit::Imperial);

    d.press(Key::Space).frames(1);
    assert_eq!(d.app().unit, Unit::Imperial, "Space chooses");
}

#[test]
fn a_disabled_group_cannot_be_focused_or_changed() {
    let mut d = harness(Theme::Light);
    for unit in [Unit::Imperial, Unit::Nautical] {
        d.click(Target::suffix(&format!(
            "/\"locked\"/Circle({})",
            unit.at()
        )));
        d.click(Target::suffix(&format!("/\"locked\"/Label({})", unit.at())));
    }
    assert_eq!(d.app().locked, Unit::Metric);
    let mut visited = Vec::new();
    for _ in 0..16 {
        d.press(Key::Tab).frames(1);
        visited.extend(focused_path(&d));
    }
    assert!(
        visited.iter().all(|path| !path.contains("locked")),
        "{visited:?}"
    );
    let disabled: Vec<_> = d
        .harness()
        .get_all_by_role(Role::RadioButton)
        .filter(|node| node.accesskit_node().is_disabled())
        .collect();
    assert_eq!(disabled.len(), 3);
}

#[test]
fn labels_name_their_controls_for_assistive_technology() {
    let d = harness(Theme::Light);
    let radio = d
        .harness()
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
        d.harness()
            .get_all_by_role_and_label(Role::RadioButton, "Imperial")
            .next()
            .expect("the first group's Imperial")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );

    let switch = d
        .harness()
        .get_by_role_and_label(Role::Switch, "Airplane mode");
    assert_eq!(switch.accesskit_node().labelled_by().count(), 1);
    assert_eq!(
        d.harness()
            .get_by_role_and_label(Role::Switch, "Bare switch")
            .accesskit_node()
            .labelled_by()
            .count(),
        0,
        "named() names a switch without a label"
    );
    let frozen = d.harness().get_by_role_and_label(Role::Switch, "Locked");
    assert!(frozen.accesskit_node().is_disabled());
    assert_eq!(
        frozen.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

fn thumb(d: &Driver<App>, path: &str) -> (Rect, CircleShape) {
    let rect = d.rect(Target::suffix(path));
    let thumb = circle_in(d, rect).unwrap_or_else(|| panic!("no thumb in {rect:?}"));
    (rect, thumb)
}

#[test]
fn switch_track_and_thumb_are_the_styles_off_and_on() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let switch = style(theme).switch().clone();
        let metrics = *switch.sizes.get(Size::DEFAULT);
        for on in [false, true] {
            if on {
                d.click(Target::suffix("/\"air\"/Track")).frames(12);
            }
            let (track_look, thumb_look) = switch_look(&style(theme), on);
            let (rect, thumb) = thumb(&d, "/\"air\"/Track");
            assert!((rect.size() - metrics.track).abs().max_elem() < 0.01);
            let track = box_at(&d, rect);
            assert_eq!(track.fill, track_look, "{theme:?} on {on}");
            assert_eq!(track.corner_radius, egui::CornerRadius::same(u8::MAX));
            assert_eq!(thumb.fill, thumb_look);
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
        assert_ne!(
            switch_look(&style(theme), true).0,
            switch_look(&style(theme), false).0
        );
    }
}

#[test]
fn the_thumb_slides_between_the_ends() {
    let mut d = harness_stepping(Theme::Light, 0.016);
    let (rect, rest) = thumb(&d, "/\"air\"/Track");
    d.click(Target::suffix("/\"air\"/Track")).frames(2);
    let (_, mid) = thumb(&d, "/\"air\"/Track");
    d.frames(12);
    let (_, end) = thumb(&d, "/\"air\"/Track");
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
    let mut d = harness(Theme::Light);
    d.click(Target::suffix("/\"air\"/Track"));
    assert!(d.app().on);
    d.click(Target::suffix("/\"air\"/Label"));
    assert!(!d.app().on, "the label flips it too");

    tab_to(&mut d, "/\"air\"/Track");
    d.press(Key::Space).frames(1);
    assert!(d.app().on, "Space");
    d.press(Key::Enter).frames(1);
    assert!(!d.app().on, "Enter");

    d.click(Target::suffix("/\"frozen\"/Track"));
    d.click(Target::suffix("/\"frozen\"/Label"));
    assert!(!d.app().on, "a disabled switch is inert");
}

#[test]
fn a_focused_switch_shows_the_styles_ring() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let style = style(theme);
        tab_to(&mut d, "/\"air\"/Track");
        d.frames(12);
        let rect = d.rect(Target::suffix("/\"air\"/Track"));
        let ring = ring_at(&d, rect).expect("a focus ring");
        assert_eq!(ring.stroke.width, style.tokens.focus.width);
        assert_eq!(ring.stroke.color, style.tokens.focus.color);
    }
}

fn cell_path(unit: Unit) -> String {
    format!("/\"seg\"/{}", unit.at())
}

#[test]
fn segmented_track_and_cells_are_the_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let seg = style(theme).segmented().clone();
        let track = d.rect(Target::suffix("/\"seg\""));
        assert_eq!(track.height(), seg.sizes.get(Size::DEFAULT).height);
        assert_eq!(box_at(&d, track).fill, seg.track_fill);
        let metric = d.rect(Target::suffix(&cell_path(Unit::Metric)));
        let metrics = seg.sizes.get(Size::DEFAULT);
        assert_eq!(
            metric.top() - track.top(),
            metrics.track_padding + metrics.cell_trim / 2.0
        );
        let selected = box_at(&d, metric);
        let trigger = support::part_at(
            &style(theme),
            TabsPart::Trigger,
            &TabsPart::trigger_keys(
                Size::DEFAULT,
                SegmentedRows::One,
                Cell::Pressed,
                Enablement::Enabled,
            ),
        );
        assert_eq!(
            Some(selected.fill),
            trigger.paint_classes().background,
            "{theme:?}"
        );
        assert_eq!(
            selected.corner_radius,
            egui::CornerRadius::same(seg.sizes.get(Size::DEFAULT).cell_radius)
        );
        assert_eq!(selected.stroke.width, 1.0, "shadcn's `border`");
        let imperial = d.rect(Target::suffix(&cell_path(Unit::Imperial)));
        assert!(
            !shapes(&d).iter().any(|shape| matches!(
                shape,
                Shape::Rect(r) if r.rect.is_positive() && support::same(r.rect, imperial) && r.fill != egui::Color32::TRANSPARENT
            )),
            "an unselected cell has no fill"
        );
    }
}

#[test]
fn segmented_has_a_tab_stop_per_cell_and_enter_chooses() {
    let mut d = harness(Theme::Light);
    let mut stops = Vec::new();
    for _ in 0..16 {
        d.press(Key::Tab).frames(1);
        stops.extend(focused_path(&d).filter(|path| path.starts_with("/\"seg\"/")));
    }
    assert_eq!(
        &stops[..3],
        [Unit::Metric, Unit::Imperial, Unit::Nautical].map(cell_path)
    );

    tab_to(&mut d, &cell_path(Unit::Imperial));
    d.frames(12);
    let seg = style(Theme::Light).segmented().clone();
    let ring = ring_at(&d, d.rect(Target::suffix(&cell_path(Unit::Imperial))))
        .expect("the focused cell has a ring");
    assert_eq!(ring.stroke.width, 3.0);
    let _ = seg;
    d.press(Key::Enter).frames(2);
    assert_eq!(d.app().segment, Unit::Imperial);

    d.click(Target::suffix(&cell_path(Unit::Nautical)))
        .frames(3);
    assert_eq!(d.app().segment, Unit::Nautical);
    let selected: Vec<_> = d
        .harness()
        .get_all_by_role(Role::RadioButton)
        .filter(|node| node.accesskit_node().label().as_deref() == Some("Knots"))
        .map(|node| node.accesskit_node().toggled())
        .collect();
    assert_eq!(selected, [Some(egui::accesskit::Toggled::True)]);
}

#[test]
fn a_modified_style_changes_every_owned_widget() {
    // The sections draw the widgets where no preset's look is over them.
    let mut style = support::sections_of(style(Theme::Light));
    let red = egui::Color32::from_rgb(200, 30, 30);
    let mut switch = style.switch().clone();
    switch.sizes.get_mut(Size::DEFAULT).unwrap().track = vec2(60.0, 30.0);
    style.set_switch(switch);
    style.sheet.rule_when(
        SwitchPart::Root,
        eggshell_ui::sheet::When::always().aria(eggshell_ui::Aria::Checked),
        move |t, _| t.bg(red),
    );
    let mut radio = style.radio().clone();
    radio.sizes.default_entry_mut().size = 24.0;
    style.set_radio(radio);
    let mut segmented = style.segmented().clone();
    segmented.track_fill = red;
    style.set_segmented(segmented);
    let mut d = harness(Theme::Light);
    d.set_style(Themed::both(Arc::new(style)));
    d.frames(4);
    d.click(Target::suffix("/\"air\"/Track")).frames(12);
    assert!(d.app().on, "precondition: the switch is on");
    d.carry_to(egui::pos2(490.0, 590.0)).frames(11);

    let track = d.rect(Target::suffix("/\"air\"/Track"));
    assert_eq!(track.size(), vec2(60.0, 30.0));
    assert_eq!(box_at(&d, track).fill, red);
    assert_eq!(
        d.rect(Target::suffix(&circle_path(Unit::Metric))).size(),
        vec2(24.0, 24.0)
    );
    assert_eq!(box_at(&d, d.rect(Target::suffix("/\"seg\""))).fill, red);
}

#[test]
fn a_radio_group_is_one_tab_stop_on_the_chosen_option() {
    let mut d = harness(Theme::Light);
    support::drive_past_bar(&mut d);
    d.press(Key::Tab).frames(1);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(circle_path(Unit::Metric).as_str()),
        "Tab enters the group on the chosen option"
    );
    let mut visited = Vec::new();
    for _ in 0..16 {
        d.press(Key::Tab).frames(1);
        visited.extend(focused_path(&d));
    }
    assert!(
        visited
            .iter()
            .filter(|path| path.contains("Circle"))
            .all(|path| *path == circle_path(Unit::Metric)),
        "a full cycle of Tab stops visits no other circle: {visited:?}"
    );

    tab_to(&mut d, &circle_path(Unit::Metric));
    d.press(Key::ArrowDown).frames(3);
    assert_eq!(d.app().unit, Unit::Imperial);
    d.press(Key::Tab).frames(1);
    assert!(
        !focused_path(&d).is_some_and(|path| path.contains("Circle")),
        "Tab leaves from the moved choice"
    );
    d.press(KeyboardShortcut::new(egui::Modifiers::SHIFT, Key::Tab))
        .frames(1);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(circle_path(Unit::Imperial).as_str()),
        "Shift+Tab comes back to the chosen option"
    );
}

#[test]
fn tab_enters_a_group_on_its_chosen_option_wherever_it_is() {
    let mut d = harness(Theme::Light);
    d.app_mut().unit = Unit::Nautical;
    d.frames(3);
    support::drive_past_bar(&mut d);
    d.press(Key::Tab).frames(1);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(circle_path(Unit::Nautical).as_str())
    );
}

#[test]
fn the_switch_track_eases_to_its_new_colour() {
    let mut d = harness_stepping(Theme::Light, 0.016);
    let style = style(Theme::Light);
    let (off, on) = (switch_look(&style, false).0, switch_look(&style, true).0);
    assert_ne!(off, on);
    let track = d.rect(Target::suffix("/\"air\"/Track"));
    assert_eq!(box_at(&d, track).fill, off);
    d.click(Target::suffix("/\"air\"/Track")).frames(2);
    let mid = box_at(&d, track).fill;
    assert!(
        mid != off && mid != on,
        "{mid:?} between {off:?} and {on:?}"
    );
    d.frames(12);
    assert_eq!(box_at(&d, track).fill, on);
    d.click(Target::suffix("/\"air\"/Track")).frames(2);
    let back = box_at(&d, track).fill;
    assert!(back != off && back != on, "and back again");
    d.frames(12);
    assert_eq!(box_at(&d, track).fill, off);
}

#[test]
fn the_switch_thumb_keeps_its_rest_colour_when_hovered_or_pressed() {
    let mut d = harness(Theme::Light);
    let rest = switch_look(&style(Theme::Light), false).1;
    let (_, at_rest) = thumb(&d, "/\"air\"/Track");
    assert_eq!(at_rest.fill, rest);
    d.hover(Target::suffix("/\"air\"/Track")).frames(12);
    assert_eq!(thumb(&d, "/\"air\"/Track").1.fill, rest, "hovered");
    d.pointer_down(Target::suffix("/\"air\"/Track")).frames(11);
    assert_eq!(thumb(&d, "/\"air\"/Track").1.fill, rest, "pressed");
    d.pointer_up(Target::suffix("/\"air\"/Track")).frames(11);
}

#[test]
fn the_tokens_ring_is_the_mechanisms_on_a_switch_and_a_cell() {
    let ring = Focus {
        width: 5.0,
        color: egui::Color32::from_rgb(200, 30, 30),
    };
    // The preset's switch and cells take shadcn's
    // `focus-visible:ring-[3px]`; the tokens' ring is the mechanism's, which
    // the style without a preset draws them with.
    let mut style = Style::from_egui(&egui::Context::default().style_of(Theme::Light));
    assert_ne!(
        style.tokens.focus, ring,
        "precondition: the preset's ring is another"
    );
    style.tokens.focus = ring;
    for path in ["/\"air\"/Track".to_owned(), cell_path(Unit::Metric)] {
        let mut d = harness(Theme::Light);
        d.set_style(Themed::both(Arc::new(style.clone())));
        d.frames(4);
        tab_to(&mut d, &path);
        d.frames(12);
        let drawn = ring_at(&d, d.rect(Target::suffix(&path))).expect("a focus ring");
        assert_eq!(drawn.stroke.width, ring.width, "{path}");
        assert_eq!(drawn.stroke.color, ring.color, "{path}");
    }
}

#[test]
fn the_neutral_style_draws_the_chosen_cell_in_egui_s_active_fill() {
    let egui_style = egui::Context::default().style_of(Theme::Light);
    let mut d = harness(Theme::Light);
    d.set_style(Themed::both(Arc::new(Style::from_egui(&egui_style))));
    d.frames(4);
    let chosen = box_at(&d, d.rect(Target::suffix(&cell_path(Unit::Metric))));
    assert_eq!(chosen.fill, egui_style.visuals.widgets.active.weak_bg_fill);
    let other = d.rect(Target::suffix(&cell_path(Unit::Imperial)));
    assert!(
        !shapes(&d).iter().any(|shape| matches!(
            shape,
            Shape::Rect(r) if support::same(r.rect, other) && r.fill != egui::Color32::TRANSPARENT
        )),
        "another cell has no fill"
    );
}

#[test]
fn a_click_on_a_segmented_cell_does_not_show_the_focus_ring() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(&cell_path(Unit::Nautical)))
        .frames(12);
    assert_eq!(d.app().segment, Unit::Nautical, "precondition");
    let cell = d.rect(Target::suffix(&cell_path(Unit::Nautical)));
    assert!(ring_at(&d, cell).is_none(), "focus-visible: keyboard only");
}

/// The cells of a segmented control tile its track for the pointer: the
/// track's padding and the gaps between cells each belong to the nearest
/// cell, as a native segmented control's do.
mod tiling {
    use egui::{Pos2, pos2};

    use super::*;

    const CELLS: [&str; 3] = ["/\"seg\"/0", "/\"seg\"/1", "/\"seg\"/2"];

    fn cells(d: &Driver<App>) -> [Rect; 3] {
        CELLS.map(|path| d.rect(Target::suffix(path)))
    }

    /// Asserts `at` is on the track and in no cell's box.
    fn between_cells(d: &Driver<App>, at: Pos2) {
        assert!(
            d.rect(Target::suffix("/\"seg\"")).contains(at),
            "precondition: {at:?} is on the track"
        );
        assert!(
            cells(d).iter().all(|cell| !cell.contains(at)),
            "precondition: {at:?} is in no cell"
        );
    }

    fn click(d: &mut Driver<App>, at: Pos2) {
        d.hover(at);
        press_and_release(d, at);
        d.frames(4);
    }

    #[test]
    fn a_click_in_the_tracks_padding_beside_the_last_cell_chooses_it() {
        let mut d = harness(Theme::Light);
        let track = d.rect(Target::suffix("/\"seg\""));
        let [_, _, last] = cells(&d);
        let at = pos2((last.max.x + track.max.x) / 2.0, last.center().y);
        between_cells(&d, at);
        click(&mut d, at);
        assert_eq!(d.app().segment, Unit::Nautical);
    }

    #[test]
    fn a_click_in_the_tracks_padding_above_a_cell_chooses_it() {
        let mut d = harness(Theme::Light);
        let track = d.rect(Target::suffix("/\"seg\""));
        let [_, middle, _] = cells(&d);
        let at = pos2(middle.center().x, (track.min.y + middle.min.y) / 2.0);
        between_cells(&d, at);
        click(&mut d, at);
        assert_eq!(d.app().segment, Unit::Imperial);
    }

    fn gapped(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().p(4.0), |b| {
            b.segmented("seg", app.segment, SEGMENTS)
                .track(tw().gap_x(2.0))
                .on_change(|app: &mut App, unit| app.segment = unit);
        });
    }

    #[test]
    fn a_click_in_the_gap_between_cells_chooses_the_nearer() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(500.0, 200.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            gapped,
        )
        .build();
        d.frames(4);
        let [first, second, _] = cells(&d);
        let gap = second.min.x - first.max.x;
        assert!(gap >= 4.0, "precondition: a gap of {gap} between the cells");
        let y = first.center().y;
        let near_second = pos2(second.min.x - gap / 4.0, y);
        between_cells(&d, near_second);
        click(&mut d, near_second);
        assert_eq!(d.app().segment, Unit::Imperial);
        let near_first = pos2(first.max.x + gap / 4.0, y);
        between_cells(&d, near_first);
        click(&mut d, near_first);
        assert_eq!(d.app().segment, Unit::Metric);
    }
}

/// An app's rules on the choice controls' parts reach what they draw.
mod app_rules {
    use eggshell_ui::Aria;
    use eggshell_ui::owned::{CheckboxPart, FieldPart, RadioGroupPart, SwitchPart};
    use eggshell_ui::sheet::When;
    use egui::Color32;

    use super::*;
    use crate::support::text_color;

    const RED: Color32 = Color32::from_rgb(200, 30, 30);
    const BLUE: Color32 = Color32::from_rgb(30, 30, 200);
    const GREEN: Color32 = Color32::from_rgb(30, 160, 30);

    /// ```html
    /// <div class="flex flex-col items-start gap-4 p-4">
    ///   <label><button role="checkbox" aria-checked="true"></button> Tick</label>
    ///   <div role="radiogroup">Metric Imperial Nautical</div>
    ///   <label><button role="switch" aria-checked="true"></button> Lit</label>
    /// </div>
    /// ```
    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
            b.checkbox("tick", "Tick", true)
                .on_change(|_: &mut App, _| {});
            b.radio_group("units", app.unit, UNITS)
                .on_change(|app: &mut App, unit| app.unit = unit);
            b.switch("lit", "Lit", true).on_change(|_: &mut App, _| {});
        });
    }

    fn restyled() -> Style {
        let mut style = style(Theme::Light);
        let checked = When::always().aria(Aria::Checked);
        style
            .sheet
            .as_app()
            .rule_when(CheckboxPart::Root, checked, |t, _| t.bg(RED))
            .rule_when(RadioGroupPart::Item, checked, |t, _| t.bg(RED))
            .rule(RadioGroupPart::IndicatorIcon, |t, _| t.size_px(12.0))
            .rule_when(SwitchPart::Root, checked, |t, _| t.bg(RED))
            .rule_when(SwitchPart::Thumb, checked, |t, _| t.bg(BLUE))
            .rule(FieldPart::Label, |t, _| t.text(GREEN));
        style
    }

    #[test]
    fn an_app_rule_on_a_choice_part_restyles_it() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(500.0, 400.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            view,
        )
        .build();
        d.ctx().set_theme(Theme::Light);
        d.set_style(Themed::both(Arc::new(restyled())));
        d.frames(12);

        let tick = d.rect(Target::suffix("/\"tick\"/Box"));
        assert_eq!(box_at(&d, tick).fill, RED, "the ticked box");
        let chosen = d.rect(Target::suffix(&circle_path(Unit::Metric)));
        assert_eq!(box_at(&d, chosen).fill, RED, "the chosen circle");
        let dot = dot_in(&d, chosen).expect("a dot");
        assert_eq!(dot.rect.width(), 12.0, "the dot");
        let unchosen = d.rect(Target::suffix(&circle_path(Unit::Imperial)));
        assert_ne!(
            box_at(&d, unchosen).fill,
            RED,
            "the preset's other rules stand"
        );
        let (track, thumb) = thumb(&d, "/\"lit\"/Track");
        assert_eq!(box_at(&d, track).fill, RED, "the track");
        assert_eq!(thumb.fill, BLUE, "the thumb");
        let label = d.rect(Target::suffix("/\"tick\"/Label"));
        assert_eq!(text_color(&d, label, "Tick"), GREEN, "the label");
    }
}

/// style-vega.css `.cn-switch`, `.cn-switch-thumb`: `h-[18.4px] w-[32px]`,
/// `data-checked:bg-primary data-unchecked:bg-input`, and a `size-4` thumb
/// in `bg-background`, `dark:data-unchecked:bg-foreground`,
/// `dark:data-checked:bg-primary-foreground`.
#[test]
fn a_switch_is_shadcns() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let t = Tokens::for_theme(theme);
        let (track, knob) = thumb(&d, "/\"air\"/Track");
        assert!(
            (track.size() - vec2(32.0, 18.4)).length() < 0.01,
            "{theme:?}: {:?}",
            track.size()
        );
        let off = match theme {
            Theme::Light => t.input,
            Theme::Dark => t.input.gamma_multiply(0.8),
        };
        assert_eq!(box_at(&d, track).fill, off, "{theme:?} bg-input");
        assert_eq!(knob.radius, 8.0, "size-4");
        let knob_off = match theme {
            Theme::Light => t.background,
            Theme::Dark => t.foreground,
        };
        assert_eq!(knob.fill, knob_off, "{theme:?} unchecked thumb");
        d.click(Target::suffix("/\"air\"/Track")).frames(30);
        assert!(d.app().on, "precondition: it turned on");
        let (track, knob) = thumb(&d, "/\"air\"/Track");
        assert_eq!(box_at(&d, track).fill, t.primary, "{theme:?} bg-primary");
        let knob_on = match theme {
            Theme::Light => t.background,
            Theme::Dark => t.primary_foreground,
        };
        assert_eq!(knob.fill, knob_on, "{theme:?} checked thumb");
    }
}
