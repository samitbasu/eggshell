//! The owned slider, select and stepper, through kittest's real pointer and
//! key events and the accessibility check the support harness switches on.

use eggshell_ui::contract::Size;
use eggshell_ui::owned::{Enablement, Step};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Style, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, ShadcnStyle, Tokens};
use egui::epaint::Shape;
use egui::{Key, KeyboardShortcut, Modifiers, Rect, Role, Theme, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, box_at, focused_path, press_and_release, ring_at, shapes, text_color};

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
            b.slider("volume", app.volume, 0.0..=100.0)
                .step(Step::new(5.0))
                .named("Volume")
                .on_change(|app: &mut App, value| {
                    app.volume = value;
                    app.changes += 1;
                });
            b.slider("locked", app.locked, 0.0..=100.0)
                .named("Locked")
                .enablement(Enablement::Disabled)
                .on_change(|app: &mut App, value| app.locked = value);
            b.select("region", app.region, REGIONS)
                .named("Region")
                .classes(tw().w_px(200.0))
                .on_change(|app: &mut App, region| app.region = region);
            b.select("frozen", 0, REGIONS)
                .named("Frozen")
                .classes(tw().w_px(200.0))
                .enablement(Enablement::Disabled)
                .on_change(|_: &mut App, _| {});
            b.stepper("count", app.count, 0.0..=10.0)
                .named("Count")
                .on_change(|app: &mut App, value| app.count = value);
            b.stepper("fixed", 5.0, 0.0..=10.0)
                .named("Fixed")
                .enablement(Enablement::Disabled)
                .on_change(|_: &mut App, _| {});
        },
    );
}

fn harness(theme: Theme) -> Driver<App> {
    let tokens = Tokens::for_theme(theme);
    let mut d = support::driver(
        vec2(400.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.ctx().set_theme(theme);
    d.frames(12);
    d
}

fn style(theme: Theme) -> Style {
    Shadcn::for_theme(theme).style()
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

const TRACK: &str = "/\"volume\"/Track";
const THUMB: &str = "/\"volume\"/Track/Thumb";

/// The `n`th node of `path`: anonymous nodes share their parent's path.
fn nth(d: &Driver<App>, path: &str, n: usize) -> Rect {
    support::nodes(d)
        .iter()
        .filter(|(candidate, _)| candidate == path)
        .nth(n)
        .unwrap_or_else(|| panic!("no node {n} of {path}"))
        .1
}

fn bar(d: &Driver<App>, slider: &str) -> Rect {
    nth(d, &format!("/\"{slider}\"/Track/<div>"), 0)
}

fn range(d: &Driver<App>, slider: &str) -> Rect {
    nth(d, &format!("/\"{slider}\"/Track/<div>"), 1)
}

#[test]
fn slider_track_range_and_thumb_are_the_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let slider = style(theme).slider;
        let track = d.rect(Target::suffix(TRACK));
        assert_eq!(track.height(), slider.sizes.get(Size::DEFAULT).thumb_size);
        let bar = bar(&d, "volume");
        assert_eq!(
            bar.height(),
            slider.sizes.get(Size::DEFAULT).track_height,
            "h-1.5"
        );
        assert_eq!(bar.width(), track.width());
        let bar_shape = box_at(&d, bar);
        assert_eq!(bar_shape.fill, slider.track.rest.fill, "{theme:?}");
        assert_eq!(bar_shape.corner_radius, egui::CornerRadius::same(u8::MAX));

        let range = range(&d, "volume");
        assert!((range.width() - 0.4 * bar.width()).abs() < 0.6, "{range:?}");
        assert_eq!(box_at(&d, range).fill, slider.range.rest.fill);

        let thumb = d.rect(Target::suffix(THUMB));
        assert_eq!(thumb.size(), vec2(16.0, 16.0), "size-4");
        let centre = track.left() + 8.0 + 0.4 * (track.width() - 16.0);
        assert!((thumb.center().x - centre).abs() < 0.6, "{thumb:?}");
        assert!((thumb.center().y - track.center().y).abs() < 0.6);
        let shape = box_at(&d, thumb);
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
        let mut d = harness(Theme::Light);
        d.app_mut().volume = value;
        d.frames(4);
        let bar_rect = bar(&d, "volume");
        let range_rect = range(&d, "volume");
        assert!(range_rect.width() > 0.0, "precondition: a range is drawn");
        assert!(
            range_rect.height() == bar_rect.height(),
            "precondition: the range is as tall as the track"
        );
        assert_eq!(
            box_at(&d, range_rect).corner_radius,
            box_at(&d, bar_rect).corner_radius,
            "value {value}"
        );
    }
}

#[test]
fn a_disabled_thumb_hides_the_track_beneath_it() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let bar = bar(&d, "locked");
        let thumb = d.rect(Target::suffix("/\"locked\"/Track/Thumb"));
        assert!(
            thumb.intersects(bar) && thumb.height() >= bar.height(),
            "precondition: the thumb lies over the track's full height"
        );
        let shape = box_at(&d, thumb);
        assert_eq!(shape.fill.a(), 255, "{theme:?} thumb fill");
        assert_eq!(shape.stroke.color.a(), 255, "{theme:?} thumb border");
        assert_eq!(shape.corner_radius, egui::CornerRadius::same(u8::MAX));
    }
}

#[test]
fn pressing_the_track_jumps_the_thumb_and_dragging_follows_the_pointer() {
    let mut d = harness(Theme::Light);
    let track = d.rect(Target::suffix(TRACK));
    let at = |fraction: f32| {
        pos2(
            track.left() + 8.0 + fraction * (track.width() - 16.0),
            track.center().y,
        )
    };
    d.click(Target::suffix(TRACK));
    assert_eq!(d.app().volume, 50.0, "the middle of the track");

    press_and_release(&mut d, at(0.75));
    d.frames(3);
    assert_eq!(d.app().volume, 75.0, "a click on the track");

    let thumb = d.rect(Target::suffix(THUMB)).center();
    d.drag(thumb, at(0.2));
    assert_eq!(d.app().volume, 20.0, "dragged by the thumb");

    d.drag(at(0.2), at(1.0) + vec2(80.0, 40.0));
    assert_eq!(d.app().volume, 100.0, "clamped at the end");
    d.drag(at(1.0), at(0.0) - vec2(80.0, 0.0));
    assert_eq!(d.app().volume, 0.0, "clamped at the start");
    let thumb = d.rect(Target::suffix(THUMB));
    assert!((thumb.left() - track.left()).abs() < 0.6, "{thumb:?}");

    d.drag(at(0.0), at(0.5) + vec2(0.0, 0.0));
    let value = d.app().volume;
    assert_eq!(value % 5.0, 0.0, "values snap to the step");
}

#[test]
fn slider_keys_step_page_and_jump_to_the_ends() {
    let mut d = harness(Theme::Light);
    tab_to(&mut d, THUMB);
    let volume = |d: &Driver<App>| d.app().volume;
    d.press(Key::ArrowRight).frames(7);
    assert_eq!(volume(&d), 45.0);
    d.press(Key::ArrowUp).frames(7);
    assert_eq!(volume(&d), 50.0);
    d.press(Key::ArrowLeft).frames(7);
    d.press(Key::ArrowDown).frames(7);
    assert_eq!(volume(&d), 40.0);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(THUMB),
        "arrows do not move the focus"
    );
    d.press(Key::PageUp).frames(7);
    assert_eq!(volume(&d), 90.0, "ten steps");
    d.press(Key::PageUp).frames(7);
    assert_eq!(volume(&d), 100.0, "clamped");
    d.press(Key::Home).frames(7);
    assert_eq!(volume(&d), 0.0);
    d.press(Key::ArrowLeft).frames(7);
    assert_eq!(volume(&d), 0.0, "clamped at the start");
    d.press(Key::End).frames(7);
    assert_eq!(volume(&d), 100.0);
    d.press(Key::PageDown).frames(7);
    assert_eq!(volume(&d), 50.0);
}

#[test]
fn a_focused_thumb_shows_the_ring_and_the_slider_is_one_tab_stop() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let style = style(theme);
        tab_to(&mut d, THUMB);
        d.frames(12);
        let ring = ring_at(&d, d.rect(Target::suffix(THUMB))).expect("a ring on the thumb");
        let halo = style
            .slider
            .thumb
            .paint
            .focus
            .expect("the thumb's own ring");
        assert_eq!(ring.stroke.width, halo.width);
        assert_eq!(ring.stroke.color, halo.color);
        d.press(Key::Tab).frames(1);
        assert_ne!(
            focused_path(&d).as_deref(),
            Some(TRACK),
            "the track is not a stop"
        );
    }
}

#[test]
fn the_thumb_shows_its_halo_under_the_pointer_and_not_at_rest() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let halo = style(theme)
            .slider
            .thumb
            .paint
            .hover_ring
            .expect("the thumb's halo");
        let thumb = d.rect(Target::suffix(THUMB));
        assert!(ring_at(&d, thumb).is_none(), "{theme:?}: no ring at rest");
        d.carry_to(thumb.center()).frames(11);
        let ring = ring_at(&d, thumb).expect("a halo under the pointer");
        assert_eq!(ring.stroke.width, halo.width, "{theme:?}");
        assert_eq!(ring.stroke.color, halo.color, "{theme:?}");
        d.carry_to(pos2(390.0, 490.0)).frames(11);
        assert!(
            ring_at(&d, thumb).is_none(),
            "{theme:?}: the halo goes with the pointer"
        );
    }
}

#[test]
fn the_halo_is_shadcns_per_style_in_the_ring_colour_at_half() {
    for (style, width) in [(ShadcnStyle::Vega, 4.0), (ShadcnStyle::Lyra, 1.0)] {
        let preset = Shadcn::for_theme(Theme::Light).with_style(style);
        let halo = preset
            .style()
            .slider
            .thumb
            .paint
            .hover_ring
            .expect("a halo");
        assert_eq!(halo.width, width, "{style:?}: hover:ring-{width}");
        assert_eq!(halo.color, preset.tokens.ring.gamma_multiply(0.5));
    }
}

#[test]
fn a_disabled_slider_ignores_the_pointer_and_the_keyboard() {
    let mut d = harness(Theme::Light);
    let track = d.rect(Target::suffix("/\"locked\"/Track"));
    press_and_release(&mut d, track.right_center() - vec2(10.0, 0.0));
    d.drag(track.center(), track.left_center());
    assert_eq!(d.app().locked, 40.0);
    for _ in 0..24 {
        d.press(Key::Tab).frames(1);
        let focus = focused_path(&d);
        assert!(
            !focus.is_some_and(|path| path.contains("locked")),
            "not a Tab stop"
        );
    }
    let slider = style(Theme::Light).slider;
    let bar = bar(&d, "locked");
    assert_eq!(box_at(&d, bar).fill, slider.track.disabled.fill);
    let thumb = d.rect(Target::suffix("/\"locked\"/Track/Thumb"));
    assert_eq!(
        box_at(&d, thumb).fill,
        slider.thumb.paint.states.disabled.fill
    );
    assert!(
        d.harness()
            .get_by_role_and_label(Role::Slider, "Locked")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_slider_is_a_slider_with_a_value_for_assistive_technology() {
    let mut d = harness(Theme::Light);
    let node = d.harness().get_by_role_and_label(Role::Slider, "Volume");
    let access = node.accesskit_node();
    assert_eq!(access.numeric_value(), Some(40.0));
    assert_eq!(access.min_numeric_value(), Some(0.0));
    assert_eq!(access.max_numeric_value(), Some(100.0));
    tab_to(&mut d, THUMB);
    d.press(Key::ArrowRight).frames(7);
    let node = d.harness().get_by_role_and_label(Role::Slider, "Volume");
    assert_eq!(node.accesskit_node().numeric_value(), Some(45.0));
}

const REGION: &str = "/\"region\"";

fn is_open(d: &Driver<App>) -> bool {
    d.harness()
        .query_by_role_and_label(Role::ListBoxOption, "Asia Pacific")
        .is_some()
}

/// The tint of the icon drawn inside `rect`, if one is.
fn chevron_in(d: &Driver<App>, rect: Rect) -> Option<egui::Color32> {
    support::textured(d)
        .into_iter()
        .filter(|mesh| rect.expand(0.5).contains_rect(mesh.calc_bounds()))
        .map(|mesh| mesh.vertices[0].color)
        .next_back()
}

#[test]
fn select_trigger_is_an_input_like_box_with_a_value_and_a_chevron() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let select = style(theme).select;
        let trigger = d.rect(Target::suffix(REGION));
        assert_eq!(trigger.size(), vec2(200.0, 36.0), "h-9");
        let shape = box_at(&d, trigger);
        let look = select.trigger;
        assert_eq!(shape.fill, look.paint.states.rest.fill, "{theme:?}");
        assert_eq!(shape.stroke.color, look.paint.states.rest.border);
        assert_eq!(shape.stroke.width, 1.0);
        assert_eq!(
            shape.corner_radius,
            egui::CornerRadius::same(select.sizes.get(Size::DEFAULT).radius)
        );
        let text = d.rect(Target::suffix("/\"region\"/<text>"));
        assert_eq!(
            text.left() - trigger.left(),
            1.0 + select.sizes.get(Size::DEFAULT).padding_x
        );
        assert_eq!(
            text_color(&d, trigger, "Europe"),
            look.paint.states.rest.text
        );
        let glyph = d.rect(Target::suffix("/\"region\"/<glyph>"));
        assert_eq!(glyph.size(), vec2(16.0, 16.0), "size-4");
        assert_eq!(
            trigger.right() - glyph.right(),
            1.0 + select.sizes.get(Size::DEFAULT).padding_x
        );
        let chevron = chevron_in(&d, glyph).expect("a chevron");
        assert_eq!(
            chevron,
            look.paint.states.rest.text.gamma_multiply(0.5),
            "opacity-50"
        );
        let frozen = d.rect(Target::suffix("/\"frozen\""));
        assert_eq!(box_at(&d, frozen).fill, look.paint.states.disabled.fill);
    }
}

#[test]
fn clicking_the_trigger_opens_the_list_and_clicking_an_option_chooses_it() {
    let mut d = harness(Theme::Light);
    assert!(!is_open(&d));
    d.click(Target::suffix(REGION)).frames(4);
    assert!(is_open(&d), "a click opens");

    let list = style(Theme::Light).menu;
    let colors = list.popup.paint.states.rest;
    assert!(
        shapes(&d).iter().any(|shape| matches!(
            shape,
            Shape::Rect(r) if r.fill == colors.fill && r.stroke.color == colors.border
                && r.corner_radius == egui::CornerRadius::same(list.radius)
        )),
        "the list is drawn in the popup colours"
    );
    assert!(
        shapes(&d).iter().any(|shape| matches!(
            shape, Shape::Rect(r) if r.fill == list.item.paint.states.hover.fill
        )),
        "the highlighted item is drawn in the item colours"
    );

    let option = d
        .harness()
        .get_by_role_and_label(Role::ListBoxOption, "North America")
        .rect();
    d.hover(option.center());
    press_and_release(&mut d, option.center());
    d.frames(4);
    assert_eq!(d.app().region, 1);
    assert!(!is_open(&d), "choosing closes the list");
    assert!(
        d.harness()
            .query_by_role_and_label(Role::ComboBox, "Region")
            .is_some()
    );

    d.click(Target::suffix(REGION)).frames(4);
    assert!(is_open(&d));
    press_and_release(&mut d, pos2(390.0, 490.0));
    d.frames(4);
    assert!(!is_open(&d), "a click outside closes it");
    assert_eq!(d.app().region, 1);
}

#[test]
fn select_keyboard_opens_moves_chooses_and_closes() {
    let mut d = harness(Theme::Light);
    tab_to(&mut d, REGION);
    for key in [Key::Space, Key::Enter, Key::ArrowDown] {
        d.press(key).frames(7);
        assert!(is_open(&d), "{key:?} opens");
        d.press(Key::Escape).frames(7);
        assert!(!is_open(&d), "Escape closes");
        assert_eq!(
            focused_path(&d).as_deref(),
            Some(REGION),
            "and the trigger keeps the focus"
        );
        assert_eq!(d.app().region, 0);
    }

    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowUp).frames(7);
    assert!(is_open(&d));
    assert_eq!(d.app().region, 0, "moving only highlights");
    d.press(Key::Enter).frames(7);
    assert_eq!(d.app().region, 1, "Enter chooses");
    assert!(!is_open(&d));

    d.press(Key::Space).frames(7);
    d.press(Key::End).frames(7);
    d.press(Key::Space).frames(7);
    assert_eq!(d.app().region, 2, "Space chooses too");
    assert!(!is_open(&d));
    assert_eq!(focused_path(&d).as_deref(), Some(REGION));
}

#[test]
fn a_select_trigger_does_not_shift_when_pressed_and_is_a_combobox() {
    let mut d = harness(Theme::Light);
    let trigger = d.rect(Target::suffix(REGION));
    let rest = text_color(&d, trigger, "Europe");
    let top = |d: &Driver<App>| {
        shapes(d)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == "Europe" => Some(t.pos.y),
                _ => None,
            })
            .expect("the value")
    };
    let before = top(&d);
    d.pointer_down(Target::suffix(REGION)).frames(11);
    assert_eq!(top(&d), before, "opens_popup: no 1px press shift");
    d.pointer_up(Target::suffix(REGION)).frames(11);
    let _ = rest;

    let node = d.harness().get_by_role_and_label(Role::ComboBox, "Region");
    assert_eq!(node.accesskit_node().value().as_deref(), Some("Europe"));
    let frozen = d.harness().get_by_role_and_label(Role::ComboBox, "Frozen");
    assert!(frozen.accesskit_node().is_disabled());
}

#[test]
fn a_disabled_select_cannot_be_opened_or_focused() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix("/\"frozen\"")).frames(3);
    assert!(!is_open(&d));
    for _ in 0..24 {
        d.press(Key::Tab).frames(1);
        assert_ne!(focused_path(&d).as_deref(), Some("/\"frozen\""));
    }
}

const FIELD: &str = "/\"count\"/Box/Field";
const UP: &str = "/\"count\"/Box/Buttons/Up";
const DOWN: &str = "/\"count\"/Box/Buttons/Down";

#[test]
fn stepper_is_one_bordered_group_with_a_column_of_buttons() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let stepper = style(theme).stepper;
        let group = d.rect(Target::suffix("/\"count\"/Box"));
        assert_eq!(group.height(), stepper.sizes.get(Size::DEFAULT).height);
        let shape = box_at(&d, group);
        assert_eq!(
            shape.fill, stepper.group.paint.states.rest.fill,
            "{theme:?}"
        );
        assert_eq!(shape.stroke.color, stepper.group.paint.states.rest.border);
        assert_eq!(
            shape.corner_radius,
            egui::CornerRadius::same(stepper.sizes.get(Size::DEFAULT).radius)
        );
        let (up, down) = (d.rect(Target::suffix(UP)), d.rect(Target::suffix(DOWN)));
        assert_eq!(up.width(), stepper.sizes.get(Size::DEFAULT).button_width);
        assert_eq!(down.width(), stepper.sizes.get(Size::DEFAULT).button_width);
        assert_eq!(down.top() - up.bottom(), 1.0, "a divider between");
        let border = stepper.group.border_width;
        assert_eq!(up.top() - group.top(), border, "inside the border");
        assert_eq!(group.bottom() - down.bottom(), border);
        assert_eq!(group.right() - up.right(), border);
        assert!(group.contains_rect(d.rect(Target::suffix(FIELD))));
        assert!(d.rect(Target::suffix(FIELD)).right() <= up.left() - 1.0);
        assert_eq!(box_at(&d, up).fill, stepper.button.paint.states.rest.fill);
        let inner = f32::from(stepper.sizes.get(Size::DEFAULT).radius) - border;
        let (up_corners, down_corners) =
            (box_at(&d, up).corner_radius, box_at(&d, down).corner_radius);
        assert_eq!((up_corners.ne, up_corners.se), (inner as u8, 0));
        assert_eq!((down_corners.ne, down_corners.se), (0, inner as u8));
        assert!(chevron_in(&d, up).is_some());
        assert!(d.painted().shows("3"));
    }
}

#[test]
fn a_focused_stepper_rings_the_group_and_never_crosses_a_button() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let stepper = style(theme).stepper;
        let group = d.rect(Target::suffix("/\"count\"/Box"));
        assert!(ring_at(&d, group).is_none(), "no ring at rest");
        d.click(Target::suffix(FIELD)).frames(12);
        let ring = ring_at(&d, group).expect("a ring around the group");
        assert_eq!(ring.stroke.width, 3.0);
        assert_eq!(
            box_at(&d, group).stroke.color,
            stepper.group.paint.states.focused.border,
            "the group's border takes the ring colour"
        );
        for path in [UP, DOWN] {
            let button = d.rect(Target::suffix(path));
            assert!(ring.rect.contains_rect(button), "{path} is enclosed");
        }
        for shape in shapes(&d) {
            let Shape::Rect(r) = shape else { continue };
            if r.stroke_kind != egui::StrokeKind::Outside || r.stroke.width == 0.0 {
                continue;
            }
            for path in [UP, DOWN] {
                let button = d.rect(Target::suffix(path));
                let band = r.rect.expand(r.stroke.width);
                let crosses = band.intersects(button) && !r.rect.contains_rect(button);
                assert!(!crosses, "a ring at {:?} crosses {path}", r.rect);
            }
        }
    }
}

#[test]
fn a_disabled_stepper_group_is_painted_in_opaque_disabled_colours() {
    let d = harness(Theme::Light);
    let stepper = style(Theme::Light).stepper;
    let group = box_at(&d, d.rect(Target::suffix("/\"fixed\"/Box")));
    assert_eq!(group.fill, stepper.group.paint.states.disabled.fill);
    assert_eq!(
        group.stroke.color,
        stepper.group.paint.states.disabled.border
    );
    assert_eq!(group.fill.a(), 255);
    assert_eq!(group.stroke.color.a(), 255);
    let up = d
        .harness()
        .get_by_role_and_label(Role::Button, "Increase Fixed");
    assert!(up.accesskit_node().is_disabled());
}

#[test]
fn the_buttons_step_the_value_and_disable_at_the_limits() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(UP)).frames(3);
    assert_eq!(d.app().count, 4.0);
    assert!(d.painted().shows("4"));
    d.click(Target::suffix(DOWN));
    d.click(Target::suffix(DOWN));
    assert_eq!(d.app().count, 2.0);

    for _ in 0..8 {
        d.click(Target::suffix(DOWN));
    }
    assert_eq!(d.app().count, 0.0);
    let down = d
        .harness()
        .get_by_role_and_label(Role::Button, "Decrease Count");
    assert!(down.accesskit_node().is_disabled(), "at the lower limit");
    assert!(
        !d.harness()
            .get_by_role_and_label(Role::Button, "Increase Count")
            .accesskit_node()
            .is_disabled()
    );
    let stepper = style(Theme::Light).stepper;
    assert_eq!(
        box_at(&d, d.rect(Target::suffix(DOWN))).fill,
        stepper.button.paint.states.disabled.fill
    );
    d.click(Target::suffix(DOWN));
    assert_eq!(d.app().count, 0.0);

    for _ in 0..12 {
        d.click(Target::suffix(UP));
    }
    assert_eq!(d.app().count, 10.0, "clamped to the range");
    assert!(
        d.harness()
            .get_by_role_and_label(Role::Button, "Increase Count")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn arrow_keys_in_the_field_step_the_value() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(FIELD)).frames(3);
    assert_eq!(focused_path(&d).as_deref(), Some("<widget>"));
    d.press(Key::ArrowUp).frames(7);
    assert_eq!(d.app().count, 4.0);
    d.press(Key::ArrowUp).frames(7);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    assert_eq!(d.app().count, 2.0);
    assert!(d.painted().shows("2"));
    for _ in 0..4 {
        d.press(Key::ArrowDown).frames(7);
    }
    assert_eq!(d.app().count, 0.0, "clamped");
    for _ in 0..12 {
        d.press(Key::ArrowUp).frames(7);
    }
    assert_eq!(d.app().count, 10.0, "clamped");
}

fn type_seven(d: &mut Driver<App>) {
    d.type_text("7").frames(2);
}

#[test]
fn a_click_into_the_stepper_field_places_the_caret_and_selects_nothing() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(FIELD)).frames(3);
    type_seven(&mut d);
    assert_eq!(
        d.app().count,
        10.0,
        "the 7 joins the 3, and 37 or 73 clamps to 10"
    );
}

#[test]
fn tab_into_the_stepper_field_selects_its_value_so_typing_replaces_it() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(FIELD)).frames(3);
    let field = d.ctx().memory(|memory| memory.focused());
    assert!(field.is_some(), "precondition: the click focused the field");
    d.press(Key::Tab).frames(2);
    let next = d.ctx().memory(|memory| memory.focused());
    assert_ne!(next, field, "precondition: Tab left the field");
    d.press(KeyboardShortcut::new(Modifiers::SHIFT, Key::Tab))
        .frames(2);
    assert_eq!(
        d.ctx().memory(|memory| memory.focused()),
        field,
        "precondition: Shift+Tab came back"
    );
    type_seven(&mut d);
    assert_eq!(d.app().count, 7.0);
}

#[test]
fn typing_sets_the_value_clamps_it_and_snaps_when_the_field_is_left() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(FIELD)).frames(3);
    d.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::A))
        .frames(1);
    d.type_text("7").frames(2);
    assert_eq!(d.app().count, 7.0);
    d.type_text(".4").frames(2);
    assert_eq!(d.app().count, 7.4, "applied as it parses");
    assert!(d.painted().shows("7.4"), "and the text stays as typed");

    d.press(Key::Tab).frames(3);
    assert_eq!(d.app().count, 7.0, "snapped to the step on leaving");
    assert!(d.painted().shows("7"));

    d.click(Target::suffix(FIELD));
    d.harness().key_press_modifiers(Modifiers::COMMAND, Key::A);
    d.type_text("99").frames(2);
    assert_eq!(d.app().count, 10.0, "clamped");
    d.harness().key_press_modifiers(Modifiers::COMMAND, Key::A);
    d.type_text("-").frames(2);
    assert_eq!(d.app().count, 10.0, "text that is not a number is ignored");
}

#[test]
fn a_stepper_is_a_spin_button_and_its_buttons_are_not_tab_stops() {
    let mut d = harness(Theme::Light);
    let node = d.harness().get_by_role_and_label(Role::SpinButton, "Count");
    assert_eq!(node.accesskit_node().numeric_value(), Some(3.0));
    assert_eq!(node.accesskit_node().max_numeric_value(), Some(10.0));
    let mut visited = Vec::new();
    for _ in 0..24 {
        d.press(Key::Tab).frames(1);
        visited.extend(focused_path(&d));
    }
    assert!(visited.iter().any(|path| path == "<widget>"));
    assert!(
        visited
            .iter()
            .all(|path| !path.contains("Up") && !path.contains("Down") && !path.contains("fixed")),
        "{visited:?}"
    );
    let fixed = d.harness().get_by_role_and_label(Role::SpinButton, "Fixed");
    assert!(fixed.accesskit_node().is_disabled());
}

#[test]
fn installing_a_modified_style_changes_the_new_widgets_with_no_code_change() {
    let mut style = style(Theme::Light);
    let red = egui::Color32::from_rgb(200, 30, 30);
    style.slider.range.rest.fill = red;
    style.slider.sizes.default_entry_mut().track_height = 10.0;
    style.slider.sizes.default_entry_mut().thumb_size = 24.0;
    style.select.trigger.paint.states.rest.fill = red;
    style.select.sizes.default_entry_mut().height = 44.0;
    style.stepper.group.paint.states.rest.fill = red;
    style.stepper.sizes.default_entry_mut().button_width = 40.0;
    let mut d = harness(Theme::Light);
    style.install(d.ctx());
    d.frames(6);

    let track = d.rect(Target::suffix(TRACK));
    assert_eq!(track.height(), 24.0);
    assert_eq!(bar(&d, "volume").height(), 10.0);
    assert_eq!(box_at(&d, range(&d, "volume")).fill, red);
    assert!(
        (d.rect(Target::suffix(THUMB)).size() - vec2(24.0, 24.0))
            .abs()
            .max_elem()
            < 0.01
    );

    let trigger = d.rect(Target::suffix(REGION));
    assert_eq!(trigger.height(), 44.0);
    assert_eq!(box_at(&d, trigger).fill, red);

    assert_eq!(
        box_at(&d, d.rect(Target::suffix("/\"count\"/Box"))).fill,
        red
    );
    assert_eq!(d.rect(Target::suffix(UP)).width(), 40.0);
}

#[test]
fn snapping_reaches_a_range_end_that_is_off_the_step_lattice() {
    use std::f64::consts::TAU;
    let range = 0.0..=TAU;
    let step = Step::new(0.01);
    assert_eq!(step.snap(TAU, &range), TAU);
    assert_eq!(step.snap(100.0, &range), TAU);
    assert_eq!(step.snap(628.1 / 100.0, &range), 628.0 / 100.0);
    assert_eq!(step.snap(0.0, &range), 0.0);
    assert_eq!(step.snap(-1.0, &range), 0.0);
    assert_eq!(step.moved(TAU, -1, &range), 627.0 / 100.0);
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

fn narrow_select(b: &mut Builder<'_, '_, App>, app: &App) {
    b.select("region", app.region, REGIONS)
        .named("Region")
        .classes(tw().w_px(120.0))
        .on_change(|app: &mut App, region| app.region = region);
}

fn option_width(window: f32, option: &str) -> f32 {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(window, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        narrow_select,
    )
    .build();
    d.frames(4);
    let trigger = d.harness().get_by_label("Region").rect().width();
    assert_eq!(trigger, 120.0, "the trigger is narrower than the window");
    d.click(Target::suffix(REGION)).frames(4);
    d.harness()
        .get_by_role_and_label(Role::ListBoxOption, option)
        .rect()
        .width()
}

#[test]
fn the_select_list_is_as_wide_as_its_options_not_the_window() {
    for window in [400.0, 800.0, 1200.0] {
        let width = option_width(window, "North America");
        assert!(width < 200.0, "{width} wide in a {window} pt window");
        assert!(
            width >= 120.0 - 8.0,
            "{width} is not narrower than the trigger"
        );
    }
    assert_eq!(
        option_width(400.0, "Europe"),
        option_width(1200.0, "Europe")
    );
}
