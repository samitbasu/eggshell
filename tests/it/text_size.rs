//! `Tw::text_size` through the real path: the text is painted at its size,
//! and its node is one line of its line height tall.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, tw};
use egui::epaint::Shape;
use egui::{Theme, Vec2};

use crate::support;

#[path = "../../examples/common/mod.rs"]
mod common;

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().flex().flex_col().items_start().p(4.0), |b| {
        b.div_id("small", tw(), |b| {
            b.text(tw().text_size(12.0, 16.0), "small")
        });
        b.div_id("large", tw(), |b| {
            b.text(tw().text_size(20.0, 28.0), "large")
        });
    });
}

fn driver() -> Driver<()> {
    let mut d = support::driver(
        Vec2::new(300.0, 200.0),
        common::base_text(Theme::Light),
        (),
        view,
    )
    .build();
    d.frames(3);
    d
}

/// The font size `text` was painted at.
fn painted_size(d: &Driver<()>, text: &str) -> f32 {
    support::shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.galley.job.sections[0].format.font_id.size)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text:?} was painted"))
}

#[test]
fn text_is_painted_at_its_size_in_a_line_of_its_height() {
    let d = driver();
    let base = common::base_text(Theme::Light).spec;
    assert_ne!(
        (base.size, base.line_height),
        (12.0, 16.0),
        "precondition: the base text is another size"
    );
    for (key, size, line) in [("small", 12.0, 16.0), ("large", 20.0, 28.0)] {
        assert_eq!(painted_size(&d, key), size, "{key}");
        let rect = d.rect(Target::suffix(&format!("{key:?}")));
        assert!((rect.height() - line).abs() < 0.01, "{key}: {rect:?}");
    }
}
