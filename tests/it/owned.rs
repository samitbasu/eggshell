//! The owned button and checkbox (issue #19 spike) through the real path:
//! kittest pointer and key events, the draw walk and the accessibility check
//! the support harness switches on.

use eggshell_ui::Style;
use std::sync::Arc;

use eggshell_ui::contract::{Size, Themed, Variant};
use eggshell_ui::owned::Enablement;
use eggshell_ui::owned::button::ButtonPart;
use eggshell_ui::sheet::{Key as Keyed, When};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, Tokens, variant};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::epaint::Shape;
use egui::{Color32, Key, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::button::look;
use crate::support::{self, box_at, focused_path, ring_at, shapes, text_color};

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
                b.button((name, enablement), "Save")
                    .variant(name)
                    .enablement(enablement)
                    .on_click(move |app: &mut App| app.clicks.push(name.0.to_owned()));
            }
        }
        b.toggle("toggle", "Toggle", app.toggled)
            .on_change(|app: &mut App, toggled| app.toggled = toggled);
        for enablement in [Enablement::Enabled, Enablement::Disabled] {
            b.checkbox(("check", enablement), "Accept terms", app.checked)
                .enablement(enablement)
                .on_change(|app: &mut App, value| app.checked = value);
        }
        b.checkbox_unlabelled("bare", false)
            .named("Bare")
            .on_change(|_: &mut App, _| {});
        b.button("menu", "Menu")
            .opens_popup()
            .on_click(|app: &mut App| app.clicks.push("menu".to_owned()));
    });
}

fn path(name: Variant, enablement: Enablement) -> String {
    format!("/({name:?}, {enablement:?})")
}

fn harness(theme: Theme) -> Driver<App> {
    let tokens = Tokens::for_theme(theme);
    let mut d = support::driver(
        vec2(500.0, 760.0),
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

fn move_away(d: &mut Driver<App>) {
    d.carry_to(egui::pos2(490.0, 690.0)).frames(11);
}

#[test]
fn every_variant_and_state_draws_shadcns_classes() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        for name in VARIANTS {
            let look = look(theme, name);
            let node = path(name, Enablement::Enabled);
            let rect = d.rect(Target::suffix(&node));
            let shape = box_at(&d, rect);
            assert_eq!(shape.fill, look.fill, "{theme:?} {name} rest fill");
            assert_eq!(
                shape.corner_radius,
                egui::CornerRadius::same(8),
                "{theme:?} {name}: rounded-md"
            );
            assert_eq!(text_color(&d, rect, "Save"), look.text);
            assert_eq!(
                (shape.stroke.color, shape.stroke.width),
                (look.border, 1.0),
                "{theme:?} {name}: border, transparent unless the variant colours it"
            );

            d.hover(Target::suffix(&node)).frames(12);
            let shape = box_at(&d, rect);
            assert_eq!(shape.fill, look.hover_fill, "{theme:?} {name} hover fill");
            assert_eq!(text_color(&d, rect, "Save"), look.hover_text);

            d.pointer_down(Target::suffix(&node)).frames(11);
            let shape = box_at(&d, rect.translate(vec2(0.0, 1.0)));
            assert_eq!(
                shape.fill, look.hover_fill,
                "{theme:?} {name}: pressed is hovered, moved down"
            );
            d.pointer_up(Target::suffix(&node)).frames(11);
            move_away(&mut d);

            let off = path(name, Enablement::Disabled);
            let off_rect = d.rect(Target::suffix(&off));
            assert_eq!(
                box_at(&d, off_rect).fill,
                look.fill.gamma_multiply(0.5),
                "{theme:?} {name}: disabled:opacity-50"
            );
            assert_eq!(
                text_color(&d, off_rect, "Save"),
                look.text.gamma_multiply(0.5)
            );
        }
    }
}

#[test]
fn the_sheet_holds_shadcns_classes_and_where_they_came_from() {
    let primary = [Keyed::Variant(variant::PRIMARY), Keyed::Size(Size::DEFAULT)];
    let light_style = style(Theme::Light);
    let light = light_style
        .sheet
        .resolve(ButtonPart::Root, &primary, &light_style.tokens);
    assert_eq!(
        light.paint_classes().background,
        Some(Color32::from_rgb(0x17, 0x17, 0x17))
    );
    assert_eq!(
        light.paint_classes().radius,
        egui::CornerRadius::same(8),
        "rounded-md is radius - 2px"
    );
    #[cfg(debug_assertions)]
    for (property, line, class) in [
        (eggshell_ui::Property::Background, 150, "bg-primary"),
        (eggshell_ui::Property::height, 182, "h-9"),
        (eggshell_ui::Property::RadiusNw, 146, "rounded-md"),
    ] {
        assert_eq!(
            light.origin_of(property),
            Some(eggshell_ui::Origin {
                layer: eggshell_ui::sheet::Layer::Base,
                source: eggshell_ui::Source::Shadcn {
                    file: "registry/styles/style-vega.css",
                    line,
                    class,
                },
            }),
            "{property:?}"
        );
    }
    let outline = [Keyed::Variant(variant::OUTLINE), Keyed::Size(Size::DEFAULT)];
    let dark_style = style(Theme::Dark);
    let dark = dark_style
        .sheet
        .resolve(ButtonPart::Root, &outline, &dark_style.tokens);
    assert_eq!(
        dark.paint_classes().background,
        Some(Tokens::dark().input.gamma_multiply(0.3)),
        "dark:bg-input/30"
    );
    assert_eq!(style(Theme::Light).tokens.checkbox_radius, 4);
}

#[test]
fn disabled_is_shadcns_half_opacity_with_no_focus_ring() {
    let d = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Disabled);
    let rect = d.rect(Target::suffix(&node));
    let shape = box_at(&d, rect);
    let rest = look(Theme::Light, variant::PRIMARY);
    assert_eq!(
        shape.fill,
        rest.fill.gamma_multiply(0.5),
        "disabled:opacity-50"
    );
    assert!(shape.fill.a() < 255, "the page shows through");
    assert!(
        ring_at(&d, rect).is_none(),
        "a disabled button never shows a focus ring"
    );
}

#[test]
fn the_focus_ring_is_shadcns_three_pixels_at_half_ring() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let tokens = Tokens::for_theme(theme);
        support::drive_past_bar(&mut d);
        d.press(Key::Tab).frames(11);
        let node = path(variant::PRIMARY, Enablement::Enabled);
        assert_eq!(focused_path(&d).as_deref(), Some(node.as_str()));
        let rect = d.rect(Target::suffix(&node));
        let ring = ring_at(&d, rect).expect("a focus ring");
        assert_eq!(ring.stroke.width, 3.0, "ring-[3px]");
        assert_eq!(
            ring.stroke.color,
            tokens.ring.gamma_multiply(0.5),
            "ring-ring/50"
        );
        assert_eq!(ring.rect, box_at(&d, rect).rect, "no ring offset");
        let border = box_at(&d, rect).stroke;
        assert_eq!(
            (border.color, border.width),
            (tokens.ring, 1.0),
            "focus-visible:border-ring on the transparent border"
        );

        let outline = path(variant::OUTLINE, Enablement::Enabled);
        while focused_path(&d).as_deref() != Some(outline.as_str()) {
            d.press(Key::Tab).frames(11);
        }
        let rect = d.rect(Target::suffix(&outline));
        let border = box_at(&d, rect).stroke;
        assert_eq!(
            (border.color, border.width),
            (tokens.ring, 1.0),
            "border-ring"
        );
        assert_eq!(ring_at(&d, rect).expect("a ring").stroke.width, 3.0);
    }
}

#[test]
fn destructive_has_its_own_focus_ring() {
    let mut d = harness(Theme::Light);
    let node = path(variant::DESTRUCTIVE, Enablement::Enabled);
    d.click(Target::suffix(&node));
    support::drive_past_bar(&mut d);
    for _ in 0..3 {
        d.press(Key::Tab).frames(1);
        if focused_path(&d).as_deref() == Some(node.as_str()) {
            break;
        }
    }
    d.frames(12);
    let ring = ring_at(&d, d.rect(Target::suffix(&node))).expect("a focus ring");
    assert_eq!(
        ring.stroke.color,
        Tokens::light().destructive.gamma_multiply(0.2)
    );
}

#[test]
fn sizes_are_what_the_classes_imply() {
    let mut d = harness(Theme::Light);
    let tokens = Tokens::light();
    let font = eggshell_ui::TextSpec {
        size: 14.0,
        line_height: 20.0,
        weight: eggshell_ui::Weight::Medium,
        ..tokens.text_role()
    }
    .font_id(d.ctx());
    let text_width = |d: &Driver<App>, text: &str| {
        d.ctx().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    let save = text_width(&d, "Save");
    for name in VARIANTS {
        let rect = d.rect(Target::suffix(&path(name, Enablement::Enabled)));
        assert_eq!(rect.height(), 36.0, "{name}: h-9");
        assert!(
            (rect.width() - (save + 20.0 + 2.0)).abs() < 0.01,
            "{name}: px-2.5 around the text in a 1px border: {} vs {}",
            rect.width(),
            save + 22.0
        );
    }
    let toggle = d.rect(Target::suffix("/\"toggle\""));
    assert_eq!(toggle.height(), 36.0);
    assert!(
        (toggle.width() - (text_width(&d, "Toggle") + 16.0)).abs() < 0.01,
        "px-2"
    );

    let bare = d.rect(Target::suffix("/\"bare\"/Box"));
    assert_eq!(bare.size(), vec2(16.0, 16.0), "size-4");
    let boxed = d.rect(Target::suffix("/(\"check\", Enabled)/Box"));
    let label = d.rect(Target::suffix("/(\"check\", Enabled)/Label"));
    assert_eq!(boxed.size(), vec2(16.0, 16.0));
    assert!((label.min.x - boxed.max.x - 8.0).abs() < 0.01, "gap-2");
    let again = support::nodes(&d).len();
    assert!(again > 0);
    d.frames(1);
}

#[test]
fn checkbox_box_and_glyph_are_shadcns() {
    // style-vega.css `.cn-checkbox`, `.cn-checkbox-indicator`.
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness(theme);
        let t = Tokens::for_theme(theme);
        let node = "/(\"check\", Enabled)/Box";
        let rect = d.rect(Target::suffix(node));
        let shape = box_at(&d, rect);
        let fill = match theme {
            Theme::Light => Color32::TRANSPARENT,
            Theme::Dark => t.input.gamma_multiply(0.3),
        };
        assert_eq!(shape.fill, fill, "{theme:?} unchecked: dark:bg-input/30");
        assert_eq!(shape.stroke.color, t.input, "border-input");
        assert_eq!(
            shape.corner_radius,
            egui::CornerRadius::same(4),
            "rounded-[4px]"
        );
        assert!(
            !shapes(&d)
                .iter()
                .any(|s| matches!(s, Shape::LineSegment { .. } | Shape::Path(_))
                    && s.visual_bounding_rect().intersects(rect)),
            "no check glyph while unchecked"
        );
        d.click(Target::suffix(node)).frames(12);
        assert!(d.app().checked);
        let shape = box_at(&d, rect);
        assert_eq!(shape.fill, t.primary, "{theme:?} checked: bg-primary");
        assert_eq!(shape.stroke.color, t.primary, "border-primary");
        let glyphs: Vec<_> = support::textured(&d)
            .into_iter()
            .filter(|mesh| rect.contains_rect(mesh.calc_bounds()))
            .collect();
        assert_eq!(glyphs.len(), 1, "a check glyph inside the box");
        let glyph = glyphs[0];
        assert!(
            glyph
                .vertices
                .iter()
                .all(|vertex| vertex.color == t.primary_foreground),
            "text-primary-foreground"
        );
        assert_eq!(glyph.calc_bounds().size(), vec2(14.0, 14.0), "size-3.5");
        // The glyph snaps to the pixel grid; the box need not sit on it.
        let off = glyph.calc_bounds().center() - rect.center();
        assert!(off.x.abs() <= 0.5 && off.y.abs() <= 0.5, "centred: {off:?}");
    }
}

#[test]
fn clicking_and_keys_activate() {
    let mut d = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    d.click(Target::suffix(&node));
    assert_eq!(d.app().clicks, ["primary"]);

    d.harness().hover_at(egui::pos2(490.0, 690.0));
    support::drive_past_bar(&mut d);
    d.press(Key::Tab).frames(1);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(path(variant::PRIMARY, Enablement::Enabled).as_str())
    );
    d.press(Key::Enter).frames(1);
    d.press(Key::Space).frames(1);
    assert_eq!(d.app().clicks, ["primary", "primary", "primary"]);
}

#[test]
fn tab_visits_enabled_controls_only() {
    let mut d = harness(Theme::Light);
    let mut visited = Vec::new();
    support::drive_past_bar(&mut d);
    for _ in 0..12 {
        d.press(Key::Tab).frames(1);
        visited.extend(focused_path(&d));
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
    let mut d = harness(Theme::Light);
    d.click(Target::suffix(&path(
        variant::PRIMARY,
        Enablement::Disabled,
    )));
    d.click(Target::suffix("/(\"check\", Disabled)/Box"));
    d.click(Target::suffix("/(\"check\", Disabled)/Label"));
    assert!(d.app().clicks.is_empty());
    assert!(!d.app().checked);
}

#[test]
fn toggle_and_checkbox_toggle() {
    let mut d = harness(Theme::Light);
    d.click(Target::suffix("/\"toggle\""));
    assert!(d.app().toggled);
    d.click(Target::suffix("/\"toggle\""));
    assert!(!d.app().toggled);

    d.click(Target::suffix("/(\"check\", Enabled)/Label"));
    assert!(d.app().checked, "the label toggles the box");
    d.click(Target::suffix("/(\"check\", Enabled)/Box"));
    assert!(!d.app().checked);

    let before = d.app().checked;
    let node = "/(\"check\", Enabled)/Box";
    for _ in 0..12 {
        if focused_path(&d).as_deref() == Some(node) {
            break;
        }
        d.press(Key::Tab).frames(1);
    }
    assert_eq!(focused_path(&d).as_deref(), Some(node));
    d.press(Key::Space).frames(1);
    assert_eq!(
        d.app().checked,
        !before,
        "Space on the focused box toggles it"
    );
}

#[test]
fn accessibility_roles_labels_and_state() {
    let mut d = harness(Theme::Light);
    let buttons: Vec<_> = d
        .harness()
        .get_all_by_role_and_label(Role::Button, "Save")
        .map(|node| node.accesskit_node().is_disabled())
        .collect();
    assert_eq!(buttons.len(), 10, "one button per variant, enabled and not");
    assert_eq!(buttons.iter().filter(|disabled| **disabled).count(), 5);

    let checkboxes: Vec<_> = d
        .harness()
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
        d.harness()
            .get_by_role_and_label(Role::CheckBox, "Bare")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False),
        "named() labels a checkbox without text"
    );
    assert_eq!(
        d.harness()
            .get_by_role_and_label(Role::Button, "Toggle")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );

    d.click(Target::suffix("/(\"check\", Enabled)/Box"));
    d.click(Target::suffix("/\"toggle\"")).frames(2);
    assert_eq!(
        d.harness()
            .get_all_by_role_and_label(Role::CheckBox, "Accept terms")
            .next()
            .unwrap()
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert_eq!(
        d.harness()
            .get_by_role_and_label(Role::Button, "Toggle")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn an_app_rule_changes_the_drawing() {
    let mut style = style(Theme::Light);
    let red = Color32::from_rgb(200, 30, 30);
    style.sheet.as_app().rule_when(
        ButtonPart::Root,
        When::always().variant(variant::PRIMARY),
        move |t, _| t.bg(red).rounded_px(0.0).h_px(40.0),
    );
    let mut d = harness(Theme::Light);
    d.set_style(Themed::both(Arc::new(style)));
    d.frames(12);
    let rect = d.rect(Target::suffix(&path(variant::PRIMARY, Enablement::Enabled)));
    let shape = box_at(&d, rect);
    assert_eq!(shape.fill, red);
    assert_eq!(shape.corner_radius, egui::CornerRadius::ZERO);
    assert_eq!(rect.height(), 40.0);
}

fn press_at(d: &mut Driver<App>, at: egui::Pos2) {
    d.carry_to(at).frames(1);
    d.harness().event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
}

fn label_top(d: &Driver<App>, inside: egui::Rect) -> f32 {
    shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if inside.contains(t.pos) => Some(t.pos.y),
            _ => None,
        })
        .expect("text in the button")
}

#[test]
fn a_pressed_button_is_painted_one_point_lower_and_nothing_else_moves() {
    let mut d = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    let rect = d.rect(Target::suffix(&node));
    let rest_text = label_top(&d, rect);
    let nodes_before = support::nodes(&d).clone();

    press_at(&mut d, rect.center());
    d.frames(12);
    let moved = rect.translate(vec2(0.0, 1.0));
    assert_eq!(
        box_at(&d, moved).fill,
        look(Theme::Light, variant::PRIMARY).hover_fill,
        "active:translate-y-px: the hovered look, a point lower"
    );
    assert_eq!(
        label_top(&d, moved) - rest_text,
        1.0,
        "the subtree moves with the box"
    );
    assert_eq!(support::nodes(&d), nodes_before, "layout rects stay");
}

#[test]
fn the_hit_rect_stays_where_layout_put_it() {
    let mut d = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    let rect = d.rect(Target::suffix(&node));
    let top_edge = egui::pos2(rect.center().x, rect.top() + 0.25);
    press_at(&mut d, top_edge);
    d.frames(12);
    assert!(
        box_at(&d, rect.translate(vec2(0.0, 1.0))).rect.top() > top_edge.y,
        "precondition: the point is above the moved box"
    );
    d.event(egui::Event::PointerButton {
        pos: top_edge,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    })
    .frames(1);
    assert_eq!(d.app().clicks, ["primary"]);
}

#[test]
fn the_press_eases_in_and_out_with_the_transition() {
    let mut d = harness(Theme::Light);
    let node = path(variant::PRIMARY, Enablement::Enabled);
    let rect = d.rect(Target::suffix(&node));
    let offset = |d: &Driver<App>| {
        let top = shapes(d)
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
    assert_eq!(offset(&d), 0.0);
    press_at(&mut d, rect.center());
    let mut seen = Vec::new();
    for _ in 0..6 {
        d.frames(1);
        seen.push(offset(&d));
    }
    assert!(
        seen.iter().any(|y| *y > 0.0 && *y < 1.0),
        "it passes through in-between offsets: {seen:?}"
    );
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{seen:?}");
    assert_eq!(*seen.last().unwrap(), 1.0);

    d.harness().event(egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut back = Vec::new();
    for _ in 0..6 {
        d.frames(1);
        back.push(offset(&d));
    }
    assert!(back.iter().any(|y| *y > 0.0 && *y < 1.0), "{back:?}");
    assert_eq!(*back.last().unwrap(), 0.0);
}

#[test]
fn a_button_that_opens_a_popup_stays_put_when_pressed() {
    let mut d = harness(Theme::Light);
    let rect = d.rect(Target::suffix("/\"menu\""));
    press_at(&mut d, rect.center());
    d.frames(12);
    assert_eq!(
        box_at(&d, rect).fill,
        look(Theme::Light, variant::SECONDARY).hover_fill,
        "the box is pressed and at its layout rect: not-aria-[haspopup]"
    );
}

#[test]
fn an_app_rule_can_take_the_press_shift_away() {
    let mut style = style(Theme::Light);
    style
        .sheet
        .as_app()
        .rule_when(ButtonPart::Root, When::always(), |t, _| {
            t.active(|p| p.translate_y(0.0))
        });
    let mut d = harness(Theme::Light);
    d.set_style(Themed::both(Arc::new(style)));
    d.frames(4);
    let rect = d.rect(Target::suffix(&path(variant::PRIMARY, Enablement::Enabled)));
    press_at(&mut d, rect.center());
    d.frames(12);
    assert_eq!(
        box_at(&d, rect).rect.top(),
        rect.top(),
        "the pressed box stays put"
    );
}

mod toggle_chain {
    use super::*;

    #[derive(Default)]
    struct App {
        on: bool,
        off_on: bool,
        hovered: bool,
        clicks_seen: u32,
    }

    /// ```html
    /// <div class="flex flex-col items-start gap-4 p-4">
    ///   <button aria-pressed="false">Live</button>
    ///   <button aria-pressed="false" disabled>Off</button>
    /// </div>
    /// ```
    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
            b.toggle("live", "Live", app.on)
                .on_response(|app: &mut App, response| {
                    app.hovered = response.hovered();
                    app.clicks_seen += u32::from(response.clicked());
                })
                .on_change(|app: &mut App, on| app.on = on);
            b.toggle("off", "Off", app.off_on)
                .enablement(Enablement::Disabled)
                .on_change(|app: &mut App, on| app.off_on = on);
        });
    }

    fn harness() -> Driver<App> {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(300.0, 200.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            view,
        )
        .build();
        d.frames(4);
        d
    }

    fn disabled(d: &Driver<App>, label: &str) -> bool {
        d.harness()
            .get_by_role_and_label(Role::Button, label)
            .accesskit_node()
            .is_disabled()
    }

    #[test]
    fn a_disabled_toggle_ignores_clicks_and_keys_and_is_announced_disabled() {
        let mut d = harness();
        assert!(!disabled(&d, "Live"), "precondition: Live is enabled");
        d.click(Target::suffix("/\"live\""));
        assert!(d.app().on, "precondition: a click toggles Live");

        assert!(disabled(&d, "Off"));
        d.click(Target::suffix("/\"off\"")).frames(2);
        assert!(!d.app().off_on, "the click did not toggle it");

        let mut live_focused = false;
        for _ in 0..4 {
            d.press(Key::Tab).frames(1);
            let focused = focused_path(&d);
            live_focused |= focused.as_deref() == Some("/\"live\"");
            assert_ne!(
                focused.as_deref(),
                Some("/\"off\""),
                "a disabled toggle takes no focus"
            );
        }
        assert!(live_focused, "precondition: Tab moves focus");
    }

    #[test]
    fn on_response_sees_the_toggles_hover_and_click() {
        let mut d = harness();
        assert!(!d.app().hovered, "precondition: not hovered");
        d.hover(Target::suffix("/\"live\"")).frames(2);
        assert!(d.app().hovered);
        assert_eq!(d.app().clicks_seen, 0);

        d.click(Target::suffix("/\"live\"")).frames(2);
        assert_eq!(d.app().clicks_seen, 1, "the click is seen once");
        assert!(d.app().on, "and still toggles");

        d.hover(egui::pos2(290.0, 190.0)).frames(2);
        assert!(!d.app().hovered);
    }
}
