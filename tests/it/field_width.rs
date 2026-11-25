//! A text field's width in a flex row: as shadcn's `Input` (`min-w-0`), it
//! shrinks below egui's text-edit width to stay inside a narrow container.

use eggshell_ui::owned::text_field::Field;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, vec2};

use crate::support;

const SURFACE: f32 = 288.0;
const PADDING: f32 = 16.0;

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

/// A popover's body: `w-72 p-4`, a row of a `w-16` label and a `flex-1`
/// field.
fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "Surface",
        tw().flex().flex_col().w_px(SURFACE).p_px(PADDING),
        |b| {
            b.div_id("Row", tw().flex().items_center().gap(4.0), |b| {
                b.text(tw().w_px(64.0), "Width");
                b.text_field_with("Width", Field::singleline("100%"))
                    .named("Width")
                    .classes(tw().flex_1())
                    .on_change(|_: &mut (), _| {});
            });
        },
    );
}

fn driver() -> Driver<()> {
    let mut d = support::driver(vec2(500.0, 200.0), base_text(), (), view).build();
    d.frames(4);
    d
}

#[test]
fn a_flex_1_field_stays_inside_a_surface_narrower_than_egui_s_text_edit_width() {
    let d = driver();
    let surface = d.rect(Target::suffix("/\"Surface\""));
    let field = d.rect(Target::suffix("/\"Width\""));
    let room = surface.width() - 2.0 * PADDING - 64.0 - 16.0;
    let text_edit_width = d.ctx().global_style().spacing.text_edit_width;
    assert!(
        room < text_edit_width,
        "precondition: the row has less room ({room}) than egui's text-edit width ({text_edit_width})"
    );
    assert_eq!(
        field.right(),
        surface.right() - PADDING,
        "inside, and filling the row"
    );
    assert_eq!(field.width(), room);
}

#[test]
fn a_field_with_room_keeps_egui_s_text_edit_width() {
    fn wide(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.div(tw().flex(), |b| {
            b.text_field_with("Wide", Field::singleline(""))
                .named("Wide")
                .on_change(|_: &mut (), _| {});
        });
    }
    let mut d = support::driver(vec2(500.0, 200.0), base_text(), (), wide).build();
    d.frames(4);
    let field = d.rect(Target::suffix("/\"Wide\""));
    assert_eq!(
        field.width(),
        d.ctx().global_style().spacing.text_edit_width
    );
}
