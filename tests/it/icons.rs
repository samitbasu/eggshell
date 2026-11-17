//! Icons: lucide's SVGs, rasterised at the size and scale they are drawn
//! and tinted with the node's text colour, through the real draw path, on a
//! glyph, a button, a segmented cell and a menu item (#128, #253).

use eggshell_ui::{Angle, Builder, Icon, Mirror, TextSpec, TextStyle, Weight, icons, tw};
use egui::epaint::{Mesh, Shape};
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, shapes, textured, turn};

const RED: Color32 = Color32::from_rgb(210, 30, 30);

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().gap(4.0).p(4.0), |s| {
        s.div_id("plain", tw().text(RED), |s| {
            s.glyph(tw().size_px(24.0), Icon::new(icons::CHEVRON_DOWN));
        });
        s.div_id("turned", tw().text(RED), |s| {
            s.glyph(
                tw().size_px(24.0),
                Icon::new(icons::CHEVRON_DOWN).rotated(Angle::degrees(90.0)),
            );
        });
        s.div_id("mirrored", tw().text(RED), |s| {
            s.glyph(
                tw().size_px(24.0),
                Icon::new(icons::CHEVRON_DOWN).mirrored(Mirror::Vertical),
            );
        });
    });
}

fn harness() -> Harness<'static, Rig<()>> {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), (), view);
    harness.run_steps(4);
    harness
}

/// The textured mesh painted inside the node at `path`.
fn icon_in<'a>(harness: &'a Harness<'_, Rig<()>>, path: &str) -> &'a Mesh {
    let rect = support::rect_of(harness, path);
    let inside: Vec<_> = textured(harness)
        .into_iter()
        .filter(|mesh| rect.expand(0.5).contains_rect(mesh.calc_bounds()))
        .collect();
    assert_eq!(inside.len(), 1, "one icon in {path}");
    inside[0]
}

/// The size, in pixels, of the texture `mesh` is painted with.
fn texture_size(harness: &Harness<'_, Rig<()>>, mesh: &Mesh) -> [usize; 2] {
    harness
        .ctx
        .tex_manager()
        .read()
        .meta(mesh.texture_id)
        .expect("a live texture")
        .size
}

#[test]
fn an_icon_is_tinted_with_the_nodes_text_colour() {
    let harness = harness();
    let mesh = icon_in(&harness, r#"/"Page"/"plain""#);
    assert!(mesh.vertices.iter().all(|vertex| vertex.color == RED));
}

#[test]
fn rotation_turns_the_quad_and_mirroring_flips_its_texture() {
    let harness = harness();
    let (plain, turned, mirrored) = (
        icon_in(&harness, r#"/"Page"/"plain""#),
        icon_in(&harness, r#"/"Page"/"turned""#),
        icon_in(&harness, r#"/"Page"/"mirrored""#),
    );
    assert_eq!(turn(plain).round(), 0.0);
    assert_eq!(turn(turned).round(), 90.0);
    let top_v = |mesh: &Mesh| mesh.vertices[0].uv.y;
    assert_eq!((top_v(plain), top_v(mirrored)), (0.0, 1.0));
}

fn sizes(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().gap(4.0).p(4.0), |s| {
        for side in [12, 24] {
            s.div_id(side, tw(), |s| {
                s.glyph(tw().size_px(side as f32), Icon::new(icons::GEAR));
            });
        }
        s.div_id("image", tw(), |s| {
            s.image(tw().size_px(40.0), icons::GEAR);
        });
    });
}

#[test]
fn an_icon_is_rasterised_at_its_drawn_size_times_the_scale() {
    for scale in [1.0, 2.0] {
        let mut harness =
            support::harness_scaled(vec2(300.0, 100.0), scale, base_text(), (), sizes, |_, _| {});
        harness.run_steps(4);
        for side in [12, 24] {
            let mesh = icon_in(&harness, &format!(r#"/"Page"/{side}"#));
            let pixels = (side as f32 * scale) as usize;
            assert_eq!(
                texture_size(&harness, mesh),
                [pixels, pixels],
                "{side} pt at {scale}x"
            );
        }
    }
}

#[test]
fn an_svg_image_draws_at_its_size() {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), (), sizes);
    harness.run_steps(4);
    let image = support::rect_of(&harness, r#"/"Page"/"image""#);
    let texture = shapes(&harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if support::same(rect.rect, image) => {
                rect.brush.as_ref().map(|brush| brush.fill_texture_id)
            }
            _ => None,
        })
        .expect("the image's rect is filled with a texture");
    let size = harness
        .ctx
        .tex_manager()
        .read()
        .meta(texture)
        .map(|meta| meta.size);
    assert_eq!(size, Some([40, 40]));
}

fn controls(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().gap(4.0).p(4.0), |s| {
        s.button("button", "Close")
            .icon(Icon::new(icons::CLOSE))
            .on_click(|_| {});
        s.segmented(
            "segmented",
            0,
            [
                (0, Icon::new(icons::PLAY), "Play"),
                (1, Icon::new(icons::PAUSE), "Pause"),
            ],
        )
        .on_change(|_, _| {});
    });
}

#[test]
fn an_icon_draws_in_a_button_and_a_segmented_cell() {
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), (), controls);
    harness.run_steps(4);
    for (path, count) in [(r#"/"Page"/"button""#, 1), (r#"/"Page"/"segmented""#, 2)] {
        let rect = support::rect_of(&harness, path);
        let icons = textured(&harness)
            .into_iter()
            .filter(|mesh| rect.contains_rect(mesh.calc_bounds()))
            .count();
        assert_eq!(icons, count, "{path} draws an icon in each of its cells");
    }
}

fn menu(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().p(4.0), |s| {
        s.dropdown_menu("file", "File").items(|m| {
            m.item("open", "Open")
                .icon(Icon::new(icons::SEARCH))
                .on_click(|_| {});
            m.item("save", "Save").on_click(|_| {});
            m.item("help", "Help").inset().on_click(|_| {});
        });
    });
}

#[test]
fn a_menu_item_with_an_icon_draws_it_before_its_label() {
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), (), menu);
    harness.run_steps(4);
    support::click_node(&mut harness, r#"/"Page"/"file""#);
    harness.run_steps(8);
    let item = |name: &str| {
        harness
            .state()
            .nodes
            .iter()
            .find(|(path, _)| path.ends_with(&format!(r#"/"{name}""#)))
            .unwrap_or_else(|| panic!("no item {name}: {:?}", harness.state().nodes))
            .1
    };
    let (open, save) = (item("open"), item("save"));
    let inside = |rect: egui::Rect| {
        textured(&harness)
            .into_iter()
            .filter(|mesh| rect.contains_rect(mesh.calc_bounds()))
            .count()
    };
    assert_eq!(inside(open), 1, "the icon is drawn in its item");
    assert_eq!(inside(save), 0, "an item without one draws none");
    let label_x = |rect: egui::Rect, text: &str| {
        shapes(&harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == text && rect.expand(1.0).contains(t.pos) => {
                    Some(t.pos.x - rect.left())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no label {text}"))
    };
    assert!(label_x(open, "Open") > label_x(save, "Save") + 8.0);
}

#[test]
fn an_inset_item_lines_its_label_up_with_an_item_that_has_an_icon() {
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), (), menu);
    harness.run_steps(4);
    support::click_node(&mut harness, r#"/"Page"/"file""#);
    harness.run_steps(8);
    let label_x = |name: &str, text: &str| {
        let rect = harness
            .state()
            .nodes
            .iter()
            .find(|(path, _)| path.ends_with(&format!(r#"/"{name}""#)))
            .unwrap_or_else(|| panic!("no item {name}"))
            .1;
        shapes(&harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == text && rect.expand(1.0).contains(t.pos) => {
                    Some(t.pos.x)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no label {text}"))
    };
    let (open, help, save) = (
        label_x("open", "Open"),
        label_x("help", "Help"),
        label_x("save", "Save"),
    );
    assert!(open > save + 8.0, "precondition: an icon pushes the label");
    assert!((open - help).abs() < 0.6, "{open} vs {help}");
}

#[path = "../../examples/icons.rs"]
#[allow(dead_code, reason = "the example's own `main` is unused here")]
mod demo;

/// Every icon painted, as its quad's bounds: a brushed rect for an untinted
/// image, a mesh for a tinted one.
fn painted_icons(harness: &Harness<'_, Rig<()>>) -> Vec<egui::Rect> {
    textured(harness)
        .into_iter()
        .map(Mesh::calc_bounds)
        .collect()
}

#[test]
fn icons_keep_their_aspect_in_a_narrow_portrait_window() {
    let mut harness = support::harness(vec2(390.0, 844.0), base_text(), (), demo::layout);
    harness.run_steps(4);
    let row = support::rect_of(&harness, r#"/"Page"/24"#);
    assert!(
        row.width() < 16.0 * 24.0 + 15.0 * 12.0,
        "precondition: the 24 pt row is narrower than its icons want"
    );
    let icons = painted_icons(&harness);
    assert!(
        icons.len() >= 64,
        "precondition: the rows' icons are painted"
    );
    for icon in icons {
        assert!(
            (icon.width() - icon.height()).abs() < 0.5,
            "{icon:?} is square"
        );
    }
}

fn squashed(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().p(4.0), |s| {
        s.div_id("wide", tw(), |s| {
            s.glyph(tw().w_px(40.0).h_px(20.0), Icon::new(icons::GEAR));
        });
    });
}

#[test]
fn an_icon_in_a_box_of_another_shape_meets_it_centred() {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), (), squashed);
    harness.run_steps(4);
    let node = support::rect_of(&harness, r#"/"Page"/"wide""#);
    let icon = icon_in(&harness, r#"/"Page"/"wide""#).calc_bounds();
    assert_eq!(
        node.size(),
        vec2(40.0, 20.0),
        "precondition: the box is wide"
    );
    assert!((icon.size() - vec2(20.0, 20.0)).length() < 0.5, "{icon:?}");
    assert!(
        (icon.center() - node.center()).length() < 0.5,
        "{icon:?} in {node:?}"
    );
}
