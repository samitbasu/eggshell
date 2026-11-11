//! The `ch` width unit: widths in the advance of "0" in the node's resolved
//! font, so that live values of bounded length never move the layout.

use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, Debug, Placed, Tw, tw};
use egui::{Color32, FontFamily, FontId, vec2};
use proptest::prelude::*;

#[path = "../../examples/common/mod.rs"]
mod common;

const DIGITS: u32 = 7;

#[derive(Clone, Copy, Debug)]
enum Cells {
    Ch,
    Content,
}

fn cell(cells: Cells) -> Tw {
    let value = tw()
        .font(FontFamily::Monospace)
        .text_right()
        .whitespace_nowrap();
    match cells {
        Cells::Ch => value.w_ch(DIGITS as f32),
        Cells::Content => value,
    }
}

/// ```html
/// <div class="flex items-center gap-2 text-sm">
///   <span>Label</span><span class="w-[7ch] font-mono text-right whitespace-nowrap">-12.5</span> ...
/// </div>
/// ```
fn readout(b: &mut Builder<'_, '_, ()>, values: &[String], cells: Cells) {
    b.div_id(
        "row",
        tw().flex()
            .items_center()
            .gap(2.0)
            .p(2.0)
            .role(common::sized(14.0)),
        |b| {
            for (i, value) in values.iter().enumerate() {
                b.span(
                    ("label", i),
                    tw().whitespace_nowrap(),
                    format!("Channel {i}"),
                );
                b.span(("value", i), cell(cells), value.clone());
                b.div_id(("chip", i), tw().w_px(10.0).h_px(10.0), |_| {});
            }
        },
    );
}

fn lay_out(headless: &mut Headless, values: &[String], cells: Cells) -> Placed<()> {
    headless.layout(&(), |b, ()| readout(b, values, cells))
}

fn headless() -> Headless {
    Headless::new(vec2(1200.0, 100.0))
        .base_text(common::base_text(egui::Theme::Light))
        .prepare(|ctx| Debug::set(ctx, Debug::Outlines))
}

/// Up to `DIGITS` characters: an optional sign, then digits.
fn value() -> impl Strategy<Value = String> {
    (any::<bool>(), 0..10u32.pow(DIGITS - 1)).prop_map(|(negative, n)| {
        if negative && n != 0 {
            format!("-{n}")
        } else {
            n.to_string()
        }
    })
}

fn frames() -> impl Strategy<Value = Vec<Vec<String>>> {
    prop::collection::vec(prop::collection::vec(value(), 4), 2..24)
}

proptest! {
    #[test]
    fn live_values_in_ch_cells_never_move_the_layout(frames in frames()) {
        let mut headless = headless();
        let first = lay_out(&mut headless, &frames[0], Cells::Ch);
        prop_assert!(first.overflowing().is_empty(), "{:?}", first.overflowing());
        for values in &frames[1..] {
            let next = lay_out(&mut headless, values, Cells::Ch);
            let moved: Vec<_> = first.diff(&next).iter().map(|c| c.path().to_owned()).collect();
            prop_assert!(moved.is_empty(), "{values:?} moved {moved:?}");
            prop_assert!(next.overflowing().is_empty(), "{values:?}: {:?}", next.overflowing());
        }
    }
}

#[test]
fn without_ch_the_same_values_move_the_layout() {
    let mut headless = headless();
    let short = ["1", "2", "3", "4"].map(String::from);
    let long = ["-123456", "2", "3", "4"].map(String::from);
    let before = lay_out(&mut headless, &short, Cells::Content);
    let after = lay_out(&mut headless, &long, Cells::Content);
    let changes = before.diff(&after);
    assert!(changes.iter().any(|c| c.moved()), "{changes:?}");
}

#[test]
fn a_value_wider_than_its_cell_is_flagged_as_overflowing() {
    let mut headless = headless();
    let too_long = ["-12345678", "2", "3", "4"].map(String::from);
    let placed = lay_out(&mut headless, &too_long, Cells::Ch);
    let overflowing = placed.overflowing();
    assert!(
        overflowing.iter().any(|path| path.contains("value")),
        "{overflowing:?}"
    );
}

fn zero_advance(headless: &Headless, size: f32) -> f32 {
    let font = FontId::new(size, FontFamily::Monospace);
    headless.ctx().fonts_mut(|fonts| {
        fonts
            .layout_no_wrap("0".to_owned(), font, Color32::WHITE)
            .size()
            .x
    })
}

fn width_of(placed: &Placed<()>, key: &'static str) -> f32 {
    placed.at(key).rect().width()
}

#[test]
fn one_ch_is_the_advance_of_zero_in_the_resolved_font() {
    let mut headless = headless();
    let placed = headless.layout(&(), |b, ()| {
        b.div(tw().role(common::mono(14.0)), |b| {
            b.div_id("inherited", tw().w_ch(1.0).h_px(4.0), |_| {});
            b.div_id("six", tw().w_ch(6.0).h_px(4.0), |_| {});
        });
        b.div_id(
            "own",
            tw().role(common::mono(22.0)).w_ch(1.0).h_px(4.0),
            |_| {},
        );
        b.div_id(
            "own_family",
            tw().font(FontFamily::Monospace).w_ch(1.0).h_px(4.0),
            |_| {},
        );
    });
    let small = zero_advance(&headless, 14.0);
    let large = zero_advance(&headless, 22.0);
    assert!(
        large > small * 1.4,
        "precondition: sizes differ ({small}, {large})"
    );
    assert_eq!(width_of(&placed, "inherited"), small);
    assert_eq!(width_of(&placed, "six"), 6.0 * small);
    assert_eq!(width_of(&placed, "own"), large);
    let body = common::base_text(egui::Theme::Light).spec.size;
    assert_eq!(
        width_of(&placed, "own_family"),
        zero_advance(&headless, body)
    );
}

#[test]
fn min_and_max_widths_resolve_in_ch() {
    let mut headless = headless();
    let placed = headless.layout(&(), |b, ()| {
        b.div(tw().role(common::mono(14.0)).flex(), |b| {
            b.div_id("floor", tw().min_w_ch(5.0).h_px(4.0), |_| {});
            b.div_id("ceiling", tw().w_px(500.0).max_w_ch(3.0).h_px(4.0), |_| {});
        });
    });
    let ch = zero_advance(&headless, 14.0);
    assert_eq!(width_of(&placed, "floor"), 5.0 * ch);
    assert_eq!(width_of(&placed, "ceiling"), 3.0 * ch);
}

#[test]
fn a_later_width_class_wins_in_either_direction() {
    let mut headless = headless();
    let placed = headless.layout(&(), |b, ()| {
        b.div(tw().role(common::mono(14.0)), |b| {
            b.div_id("px_over_ch", tw().w_ch(9.0).w_px(30.0).h_px(4.0), |_| {});
            b.div_id("ch_over_px", tw().w_px(30.0).w_ch(2.0).h_px(4.0), |_| {});
            b.div_id(
                "merged_px",
                tw().w_ch(9.0).merge(tw().w_px(40.0)).h_px(4.0),
                |_| {},
            );
            b.div_id(
                "merged_ch",
                tw().w_px(40.0).merge(tw().w_ch(2.0)).h_px(4.0),
                |_| {},
            );
            b.div_id(
                "merge_leaves_width",
                tw().w_ch(2.0).merge(tw().h_px(4.0)),
                |_| {},
            );
        });
    });
    let ch = zero_advance(&headless, 14.0);
    assert_eq!(width_of(&placed, "px_over_ch"), 30.0);
    assert_eq!(width_of(&placed, "ch_over_px"), 2.0 * ch);
    assert_eq!(width_of(&placed, "merged_px"), 40.0);
    assert_eq!(width_of(&placed, "merged_ch"), 2.0 * ch);
    assert_eq!(width_of(&placed, "merge_leaves_width"), 2.0 * ch);
}
