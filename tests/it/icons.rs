//! Vector icons: drawn in the node's text colour through the real draw path,
//! on a glyph, a button, a segmented cell and a menu item (#128).

use eggshell_ui::{Angle, Builder, Icon, Mirror, TextSpec, TextStyle, Weight, icons, tw};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Pos2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, shapes};

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
            s.glyph(tw().size_px(24.0), Icon::vector(&icons::CHEVRON_DOWN));
        });
        s.div_id("turned", tw().text(RED), |s| {
            s.glyph(
                tw().size_px(24.0),
                Icon::vector(&icons::CHEVRON_DOWN).rotated(Angle::radians(std::f32::consts::PI)),
            );
        });
        s.div_id("mirrored", tw().text(RED), |s| {
            s.glyph(
                tw().size_px(24.0),
                Icon::vector(&icons::CHEVRON_DOWN).mirrored(Mirror::Vertical),
            );
        });
    });
}

fn harness() -> Harness<'static, Rig<()>> {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), (), view);
    harness.run_steps(4);
    harness
}

/// The vertices of the first line shape painted in `color`.
fn line_in(harness: &Harness<'_, Rig<()>>, color: Color32, nth: usize) -> Vec<Pos2> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { .. } => None,
            Shape::Path(path) if path.stroke.color == egui::epaint::ColorMode::Solid(color) => {
                Some(path.points.clone())
            }
            _ => None,
        })
        .nth(nth)
        .unwrap_or_else(|| panic!("a line in {color:?}"))
}

#[test]
fn a_vector_icon_is_stroked_in_the_nodes_text_colour() {
    let harness = harness();
    let points = line_in(&harness, RED, 0);
    assert_eq!(points.len(), 3, "the chevron's three vertices");
}

#[test]
fn rotation_and_mirroring_move_the_strokes() {
    let harness = harness();
    let (plain, turned, mirrored) = (
        line_in(&harness, RED, 0),
        line_in(&harness, RED, 1),
        line_in(&harness, RED, 2),
    );
    assert!(plain[1].y > plain[0].y, "a down chevron's point is lowest");
    assert!(
        turned[1].y < turned[0].y,
        "turned half a turn, the point is highest"
    );
    assert!(
        mirrored[1].y < mirrored[0].y,
        "mirrored top to bottom, the point is highest"
    );
}

fn controls(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().gap(4.0).p(4.0), |s| {
        s.button("button", "Close")
            .icon(Icon::vector(&icons::CLOSE))
            .on_click(|_| {});
        s.segmented(
            "segmented",
            0,
            [
                (0, Icon::vector(&icons::PLAY), "Play"),
                (1, Icon::vector(&icons::PAUSE), "Pause"),
            ],
        )
        .on_change(|_, _| {});
    });
}

#[test]
fn a_vector_icon_draws_in_a_button_and_a_segmented_cell() {
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), (), controls);
    harness.run_steps(4);
    for path in [r#"/"Page"/"button""#, r#"/"Page"/"segmented""#] {
        let rect = support::rect_of(&harness, path);
        let strokes = shapes(&harness)
            .into_iter()
            .filter(|shape| {
                matches!(shape, Shape::Path(p) if p.points.iter().all(|point| rect.contains(*point)))
            })
            .count();
        assert!(strokes > 0, "{path} draws a stroked icon");
    }
}
