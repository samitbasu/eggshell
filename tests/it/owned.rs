//! The owned button and checkbox (issue #19 spike) through the real path:
//! kittest pointer and key events, the draw walk and the accessibility check
//! the support harness switches on.

use bullseye::Style;
use bullseye::contract::{Size, Variant};
use bullseye::owned::Mark;
use bullseye::owned::{Button, Checkbox, Enablement};
use bullseye::{Builder, TextStyle, tw};
use bullseye_shadcn::{Shadcn, Tokens, variant};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, FontId, Key, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{
    self, Rig, box_at, click_node, focused_path, hover_node, press, rect_of, release, ring_at,
    shapes, text_color,
};

const VARIANTS: [Variant; 5] = [
    variant::PRIMARY,
    variant::SECONDARY,
    variant::DESTRUCTIVE,
    variant::OUTLINE,
    variant::GHOST,
];

#[derive(Default)]
struct App {
    clicks: Vec<String>,
    toggled: bool,
    checked: bool,
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <button class="…">Save</button>   <!-- one per variant, then the same disabled -->
///   <button class="…">Toggle</button> <!-- toggle -->
///   <label><input type="checkbox"> Accept terms</label>  <!-- enabled, then disabled -->
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        for enablement in [Enablement::Enabled, Enablement::Disabled] {
            for name in VARIANTS {
                b.button(
                    (name, enablement),
                    Button::new("Save").variant(name).enablement(enablement),
                    move |app: &mut App| {
                        app.clicks.push(name.0.to_owned());
                    },
                );
            }
        }
        b.button(
            "toggle",
            Button::toggle("Toggle", Mark::from(app.toggled)),
            |app: &mut App| {
                app.toggled = !app.toggled;
            },
        );
        for enablement in [Enablement::Enabled, Enablement::Disabled] {
            b.checkbox(
                ("check", enablement),
                Checkbox::new("Accept terms", Mark::from(app.checked)).enablement(enablement),
                |app: &mut App| {
                    app.checked = !app.checked;
                },
            );
        }
        b.checkbox(
            "bare",
            Checkbox::unlabelled(Mark::Off).named("Bare"),
            |_: &mut App| {},
        );
        b.button(
            "menu",
            Button::new("Menu").opens_popup(),
            |app: &mut App| app.clicks.push("menu".to_owned()),
        );
    });
}

fn path(name: Variant, enablement: Enablement) -> String {
    format!("/({name:?}, {enablement:?})")
}

fn harness(theme: Theme) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(theme);
    let mut harness = support::harness(
        vec2(500.0, 760.0),
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

fn move_away(harness: &mut Harness<'_, Rig<App>>) {
    harness.hover_at(egui::pos2(490.0, 690.0));
    harness.run_steps(12);
}

#[test]
fn every_variant_and_state_draws_the_styles_colours() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = style(theme);
        let mut harness = harness(theme);
        for name in VARIANTS {
            let look = style.button.variants.get(name);
            let node = path(name, Enablement::Enabled);
            let rect = rect_of(&harness, &node);
            let radius = f32::from(style.button.sizes.get(Size::DEFAULT).radius);
            let rest = look.paint.states.rest;
            let shape = box_at(&harness, rect);
            assert_eq!(shape.fill, rest.fill, "{theme:?} {name} rest fill");
            assert_eq!(
                shape.corner_radius,
                egui::CornerRadius::same(style.button.sizes.get(Size::DEFAULT).radius),
                "{theme:?} {name} radius {radius}"
            );
            assert_eq!(text_color(&harness, rect, "Save"), rest.text);
            if look.border_width > 0.0 {
                assert_eq!(shape.stroke.color, rest.border, "{theme:?} {name} border");
                assert_eq!(shape.stroke.width, look.border_width);
            } else {
                assert_eq!(shape.stroke.width, 0.0, "{theme:?} {name} has no border");
            }

            hover_node(&mut harness, &node);
            harness.run_steps(12);
            let shape = box_at(&harness, rect);
            assert_eq!(
                shape.fill, look.paint.states.hover.fill,
                "{theme:?} {name} hover fill"
            );
            assert_eq!(
                text_color(&harness, rect, "Save"),
                look.paint.states.hover.text
            );

            press(&mut harness, &node);
            let shape = box_at(&harness, rect.translate(vec2(0.0, 1.0)));
            assert_eq!(
                shape.fill, look.paint.states.pressed.fill,
                "{theme:?} {name} pressed"
            );
            release(&mut harness, &node);
            move_away(&mut harness);

            let off = path(name, Enablement::Disabled);
            let off_rect = rect_of(&harness, &off);
            let shape = box_at(&harness, off_rect);
            assert_eq!(
                shape.fill, look.paint.states.disabled.fill,
                "{theme:?} {name} disabled"
            );
            assert_eq!(
                text_color(&harness, off_rect, "Save"),
                look.paint.states.disabled.text
            );
            assert_ne!(
                look.paint.states.disabled, look.paint.states.rest,
                "precondition: the disabled colours differ from rest"
            );
        }
    }
}

#[test]
fn vega_values_are_shadcns() {
    let light = style(Theme::Light);
    let primary = light.button.variants.get(variant::PRIMARY);
    assert_eq!(
        primary.paint.states.rest.fill,
        Color32::from_rgb(0x17, 0x17, 0x17)
    );
    assert_eq!(
        primary.paint.states.rest.text,
        Color32::from_rgb(0xfa, 0xfa, 0xfa)
    );
    assert_eq!(
        primary.paint.states.hover.fill,
        Color32::from_rgb(0x17, 0x17, 0x17).gamma_multiply(0.9)
    );
    assert_eq!(
        light.button.sizes.get(Size::DEFAULT).radius,
        8,
        "rounded-md is radius - 2px"
    );
    assert_eq!(light.checkbox.radius, 4);
    let dark = style(Theme::Dark);
    let outline = dark.button.variants.get(variant::OUTLINE);
    assert_eq!(
        outline.paint.states.rest.fill,
        Tokens::dark().input.gamma_multiply(0.3)
    );
    assert_eq!(
        outline.paint.states.hover.fill,
        Tokens::dark().input.gamma_multiply(0.5)
    );
}

#[test]
fn disabled_is_explicit_opaque_colours_not_alpha() {
    let harness = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Disabled);
    let rect = rect_of(&harness, &node);
    let shape = box_at(&harness, rect);
    let rest = style(Theme::Light)
        .button
        .variants
        .get(variant::PRIMARY)
        .paint
        .states
        .rest;
    let background = Tokens::light().background;
    assert_eq!(
        shape.fill,
        rest.faded_over(bullseye::Opacity::new(0.5), background)
            .fill
    );
    assert_eq!(shape.fill.a(), 255, "opaque, so nothing shows through");
    assert!(
        ring_at(&harness, rect).is_none(),
        "a disabled button never shows a focus ring"
    );
}

#[test]
fn the_focus_ring_is_shadcns_three_pixels_at_half_ring() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        let tokens = Tokens::for_theme(theme);
        harness.key_press(Key::Tab);
        harness.run_steps(12);
        let node = path(variant::PRIMARY, Enablement::Enabled);
        assert_eq!(focused_path(&harness).as_deref(), Some(node.as_str()));
        let rect = rect_of(&harness, &node);
        let ring = ring_at(&harness, rect).expect("a focus ring");
        assert_eq!(ring.stroke.width, 3.0, "ring-[3px]");
        assert_eq!(
            ring.stroke.color,
            tokens.ring.gamma_multiply(0.5),
            "ring-ring/50"
        );
        assert_eq!(ring.rect, box_at(&harness, rect).rect, "no ring offset");
        assert_eq!(
            box_at(&harness, rect).stroke.width,
            0.0,
            "primary has no border to recolour"
        );

        let outline = path(variant::OUTLINE, Enablement::Enabled);
        while focused_path(&harness).as_deref() != Some(outline.as_str()) {
            harness.key_press(Key::Tab);
            harness.run_steps(12);
        }
        let rect = rect_of(&harness, &outline);
        let border = box_at(&harness, rect).stroke;
        assert_eq!(
            (border.color, border.width),
            (tokens.ring, 1.0),
            "border-ring"
        );
        assert_eq!(ring_at(&harness, rect).expect("a ring").stroke.width, 3.0);
    }
}

#[test]
fn destructive_has_its_own_focus_ring() {
    let mut harness = harness(Theme::Light);
    let node = path(variant::DESTRUCTIVE, Enablement::Enabled);
    click_node(&mut harness, &node);
    for _ in 0..3 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        if focused_path(&harness).as_deref() == Some(node.as_str()) {
            break;
        }
    }
    harness.run_steps(12);
    let ring = ring_at(&harness, rect_of(&harness, &node)).expect("a focus ring");
    assert_eq!(
        ring.stroke.color,
        Tokens::light().destructive.gamma_multiply(0.2)
    );
}

#[test]
fn sizes_are_what_the_classes_imply() {
    let mut harness = harness(Theme::Light);
    let style = style(Theme::Light);
    let font = FontId::new(style.text.size, FontFamily::Proportional);
    let text_width = |harness: &Harness<'_, Rig<App>>, text: &str| {
        harness.ctx.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    let save = text_width(&harness, "Save");
    for name in VARIANTS {
        let rect = rect_of(&harness, &path(name, Enablement::Enabled));
        let border = if name == variant::OUTLINE { 2.0 } else { 0.0 };
        assert_eq!(rect.height(), 36.0, "{name}: h-9");
        assert!(
            (rect.width() - (save + 32.0 + border)).abs() < 0.01,
            "{name}: px-4 around the text{}: {} vs {}",
            if border > 0.0 {
                " and a 1px border"
            } else {
                ""
            },
            rect.width(),
            save + 32.0 + border
        );
    }
    let toggle = rect_of(&harness, "/\"toggle\"");
    assert_eq!(toggle.height(), 36.0);
    assert!(
        (toggle.width() - (text_width(&harness, "Toggle") + 16.0)).abs() < 0.01,
        "px-2"
    );

    let bare = rect_of(&harness, "/\"bare\"/Box");
    assert_eq!(bare.size(), vec2(16.0, 16.0), "size-4");
    let boxed = rect_of(&harness, "/(\"check\", Enabled)/Box");
    let label = rect_of(&harness, "/(\"check\", Enabled)/Label");
    assert_eq!(boxed.size(), vec2(16.0, 16.0));
    assert!((label.min.x - boxed.max.x - 8.0).abs() < 0.01, "gap-2");
    let again = harness.state().nodes.len();
    assert!(again > 0);
    harness.run_steps(1);
}

#[test]
fn checkbox_box_and_glyph_are_the_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        let style = style(theme);
        let node = "/(\"check\", Enabled)/Box";
        let rect = rect_of(&harness, node);
        let shape = box_at(&harness, rect);
        let unchecked = style.checkbox.off.states.rest;
        assert_eq!(shape.fill, unchecked.fill, "{theme:?} unchecked");
        assert_eq!(shape.stroke.color, unchecked.border);
        assert_eq!(shape.corner_radius, egui::CornerRadius::same(4));
        assert!(
            !shapes(&harness)
                .iter()
                .any(|s| matches!(s, Shape::LineSegment { .. } | Shape::Path(_))
                    && s.visual_bounding_rect().intersects(rect)),
            "no check glyph while unchecked"
        );
        click_node(&mut harness, node);
        harness.run_steps(12);
        assert!(harness.state().app.checked);
        let checked = style.checkbox.on.states.rest;
        let shape = box_at(&harness, rect);
        assert_eq!(shape.fill, checked.fill, "{theme:?} checked");
        assert_eq!(shape.stroke.color, checked.border);
        let glyph = shapes(&harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Path(path)
                    if rect.contains_rect(path.visual_bounding_rect().shrink(0.5)) =>
                {
                    Some(path.clone())
                }
                _ => None,
            })
            .expect("a check glyph inside the box");
        assert_eq!(
            glyph.stroke.color,
            egui::epaint::ColorMode::Solid(checked.text)
        );
        assert!(
            (glyph.stroke.width - 2.0 * 14.0 / 24.0).abs() < 1e-4,
            "stroke-2 on a 14 point icon"
        );
    }
}

#[test]
fn clicking_and_keys_activate() {
    let mut harness = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    click_node(&mut harness, &node);
    assert_eq!(harness.state().app.clicks, ["primary"]);

    harness.hover_at(egui::pos2(490.0, 690.0));
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(path(variant::PRIMARY, Enablement::Enabled).as_str())
    );
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert_eq!(
        harness.state().app.clicks,
        ["primary", "primary", "primary"]
    );
}

#[test]
fn tab_visits_enabled_controls_only() {
    let mut harness = harness(Theme::Light);
    let mut visited = Vec::new();
    for _ in 0..12 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        visited.extend(focused_path(&harness));
    }
    let expected: Vec<String> = VARIANTS
        .into_iter()
        .map(|name| path(name, Enablement::Enabled))
        .chain([
            "/\"toggle\"".to_owned(),
            "/(\"check\", Enabled)/Box".to_owned(),
            "/\"bare\"/Box".to_owned(),
        ])
        .collect();
    assert_eq!(&visited[..expected.len()], &expected[..]);
    assert!(
        visited
            .iter()
            .all(|p| !p.contains("Disabled") && !p.contains("Label")),
        "{visited:?}"
    );
}

#[test]
fn disabled_controls_block_input() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, &path(variant::PRIMARY, Enablement::Disabled));
    click_node(&mut harness, "/(\"check\", Disabled)/Box");
    click_node(&mut harness, "/(\"check\", Disabled)/Label");
    assert!(harness.state().app.clicks.is_empty());
    assert!(!harness.state().app.checked);
}

#[test]
fn toggle_and_checkbox_toggle() {
    let mut harness = harness(Theme::Light);
    click_node(&mut harness, "/\"toggle\"");
    assert!(harness.state().app.toggled);
    click_node(&mut harness, "/\"toggle\"");
    assert!(!harness.state().app.toggled);

    click_node(&mut harness, "/(\"check\", Enabled)/Label");
    assert!(harness.state().app.checked, "the label toggles the box");
    click_node(&mut harness, "/(\"check\", Enabled)/Box");
    assert!(!harness.state().app.checked);

    let before = harness.state().app.checked;
    let node = "/(\"check\", Enabled)/Box";
    for _ in 0..12 {
        if focused_path(&harness).as_deref() == Some(node) {
            break;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    assert_eq!(focused_path(&harness).as_deref(), Some(node));
    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert_eq!(
        harness.state().app.checked,
        !before,
        "Space on the focused box toggles it"
    );
}

#[test]
fn accessibility_roles_labels_and_state() {
    let mut harness = harness(Theme::Light);
    let buttons: Vec<_> = harness
        .get_all_by_role_and_label(Role::Button, "Save")
        .map(|node| node.accesskit_node().is_disabled())
        .collect();
    assert_eq!(buttons.len(), 10, "one button per variant, enabled and not");
    assert_eq!(buttons.iter().filter(|disabled| **disabled).count(), 5);

    let checkboxes: Vec<_> = harness
        .get_all_by_role_and_label(Role::CheckBox, "Accept terms")
        .map(|node| {
            (
                node.accesskit_node().is_disabled(),
                node.accesskit_node().toggled(),
            )
        })
        .collect();
    assert_eq!(
        checkboxes,
        [
            (false, Some(egui::accesskit::Toggled::False)),
            (true, Some(egui::accesskit::Toggled::False))
        ]
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::CheckBox, "Bare")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False),
        "named() labels a checkbox without text"
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Toggle")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );

    click_node(&mut harness, "/(\"check\", Enabled)/Box");
    click_node(&mut harness, "/\"toggle\"");
    harness.run_steps(2);
    assert_eq!(
        harness
            .get_all_by_role_and_label(Role::CheckBox, "Accept terms")
            .next()
            .unwrap()
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Toggle")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn a_changed_style_changes_the_drawing() {
    let mut style = style(Theme::Light);
    let red = Color32::from_rgb(200, 30, 30);
    let look = style.button.variants.get_mut(variant::PRIMARY).unwrap();
    look.paint.states.rest.fill = red;
    style.button.sizes.get_mut(Size::DEFAULT).unwrap().radius = 0;
    style.button.sizes.get_mut(Size::DEFAULT).unwrap().height = 40.0;
    let mut harness = harness(Theme::Light);
    style.install(&harness.ctx);
    harness.run_steps(12);
    let rect = rect_of(&harness, &path(variant::PRIMARY, Enablement::Enabled));
    let shape = box_at(&harness, rect);
    assert_eq!(shape.fill, red);
    assert_eq!(shape.corner_radius, egui::CornerRadius::ZERO);
    assert_eq!(rect.height(), 40.0);
}

fn press_at(harness: &mut Harness<'_, Rig<App>>, at: egui::Pos2) {
    harness.hover_at(at);
    harness.run_steps(2);
    harness.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
}

fn label_top(harness: &Harness<'_, Rig<App>>, inside: egui::Rect) -> f32 {
    shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if inside.contains(t.pos) => Some(t.pos.y),
            _ => None,
        })
        .expect("text in the button")
}

#[test]
fn a_pressed_button_is_painted_one_point_lower_and_nothing_else_moves() {
    let mut harness = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    let rect = rect_of(&harness, &node);
    let rest_text = label_top(&harness, rect);
    let nodes_before = harness.state().nodes.clone();
    assert_eq!(
        style(Theme::Light).button.press_shift,
        1.0,
        "precondition: vega shifts"
    );

    press_at(&mut harness, rect.center());
    harness.run_steps(12);
    let moved = rect.translate(vec2(0.0, 1.0));
    assert_eq!(
        box_at(&harness, moved).fill,
        style(Theme::Light)
            .button
            .variants
            .get(variant::PRIMARY)
            .paint
            .states
            .pressed
            .fill
    );
    assert_eq!(
        label_top(&harness, moved) - rest_text,
        1.0,
        "the subtree moves with the box"
    );
    assert_eq!(harness.state().nodes, nodes_before, "layout rects stay");
}

#[test]
fn the_hit_rect_stays_where_layout_put_it() {
    let mut harness = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    let rect = rect_of(&harness, &node);
    let top_edge = egui::pos2(rect.center().x, rect.top() + 0.25);
    press_at(&mut harness, top_edge);
    harness.run_steps(12);
    assert!(
        box_at(&harness, rect.translate(vec2(0.0, 1.0))).rect.top() > top_edge.y,
        "precondition: the point is above the moved box"
    );
    harness.event(egui::Event::PointerButton {
        pos: top_edge,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(2);
    assert_eq!(harness.state().app.clicks, ["primary"]);
}

#[test]
fn the_press_eases_in_and_out_with_the_transition() {
    let mut harness = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    let rect = rect_of(&harness, &node);
    let offset = |harness: &Harness<'_, Rig<App>>| {
        let top = shapes(harness)
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::Rect(r)
                    if (r.rect.size() - rect.size()).length() < 0.6
                        && (r.rect.min.x - rect.min.x).abs() < 0.6
                        && (r.rect.min.y - rect.min.y).abs() < 1.6
                        && r.stroke_kind == egui::StrokeKind::Inside =>
                {
                    Some(r.rect.min.y)
                }
                _ => None,
            })
            .next_back()
            .expect("the box");
        top - rect.top()
    };
    assert_eq!(offset(&harness), 0.0);
    press_at(&mut harness, rect.center());
    let mut seen = Vec::new();
    for _ in 0..6 {
        harness.step();
        seen.push(offset(&harness));
    }
    assert!(
        seen.iter().any(|y| *y > 0.0 && *y < 1.0),
        "it passes through in-between offsets: {seen:?}"
    );
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{seen:?}");
    assert_eq!(*seen.last().unwrap(), 1.0);

    harness.event(egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut back = Vec::new();
    for _ in 0..6 {
        harness.step();
        back.push(offset(&harness));
    }
    assert!(back.iter().any(|y| *y > 0.0 && *y < 1.0), "{back:?}");
    assert_eq!(*back.last().unwrap(), 0.0);
}

#[test]
fn a_button_that_opens_a_popup_stays_put_when_pressed() {
    let mut harness = harness(Theme::Light);
    let rect = rect_of(&harness, "/\"menu\"");
    press_at(&mut harness, rect.center());
    harness.run_steps(12);
    assert_eq!(
        box_at(&harness, rect).fill,
        style(Theme::Light)
            .button
            .variants
            .get(variant::SECONDARY)
            .paint
            .states
            .pressed
            .fill,
        "the box is pressed and at its layout rect"
    );
}

#[test]
fn a_style_without_press_shift_does_not_move_buttons() {
    let mut style = style(Theme::Light);
    style.button.press_shift = 0.0;
    let mut harness = harness(Theme::Light);
    style.install(&harness.ctx);
    harness.run_steps(4);
    let rect = rect_of(&harness, &path(variant::PRIMARY, Enablement::Enabled));
    press_at(&mut harness, rect.center());
    harness.run_steps(12);
    box_at(&harness, rect);
}
