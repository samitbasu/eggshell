//! `n` items of `w-1/n` fill a wrapping row exactly and never wrap, at every
//! container width, however `1/n` rounds in `f32` (as in a browser).

use bullseye::testing::Headless;
use bullseye::{Builder, tw};
use egui::vec2;

/// The tops of `parts` laid out in one wrapping row `width` points wide.
fn row(headless: &mut Headless, width: f32, parts: &'static [(f32, f32)]) -> Vec<f32> {
    let placed = headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
        b.div_id("row", tw().flex().flex_wrap().w_px(width), |b| {
            for (index, (numerator, denominator)) in parts.iter().enumerate() {
                b.div_id(
                    index,
                    tw().w_frac(*numerator, *denominator).h_px(10.0),
                    |_| {},
                );
            }
        });
    });
    (0..parts.len())
        .map(|index| placed.at("row").at(index).rect().min.y)
        .collect()
}

#[test]
fn fractions_that_add_up_to_one_never_wrap() {
    const THIRDS: [(f32, f32); 3] = [(1.0, 3.0); 3];
    const SIXTHS: [(f32, f32); 6] = [(1.0, 6.0); 6];
    const SEVENTHS: [(f32, f32); 7] = [(1.0, 7.0); 7];
    const TWO_AND_ONE: [(f32, f32); 2] = [(2.0, 3.0), (1.0, 3.0)];
    let cases: [(&str, &'static [(f32, f32)]); 4] = [
        ("1/3 ×3", &THIRDS),
        ("1/6 ×6", &SIXTHS),
        ("1/7 ×7", &SEVENTHS),
        ("2/3 + 1/3", &TWO_AND_ONE),
    ];
    let mut headless = Headless::new(vec2(1400.0, 200.0));
    for (name, parts) in cases {
        let wrapped: Vec<u32> = (100..1300)
            .filter(|&width| {
                let tops = row(&mut headless, width as f32, parts);
                tops.iter().any(|&top| top != tops[0])
            })
            .collect();
        assert!(wrapped.is_empty(), "{name} wraps at widths {wrapped:?}");
    }
}
