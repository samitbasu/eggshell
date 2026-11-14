//! The `responsive` example: a grid whose columns follow its region's width,
//! and title bars that pick the form that fits.

use std::collections::BTreeSet;

use eggshell_ui::testing::Headless;
use eggshell_ui::{NodeKind, Placed, TextSpec, TextStyle};
use eggshell_ui_shadcn::Shadcn;
use egui::{Color32, FontFamily, vec2};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/responsive.rs"]
mod example;

use example::Form;

const PANES: usize = 8;

fn place(width: f32) -> Placed<()> {
    Headless::new(vec2(width, 800.0))
        .prepare(Shadcn::apply_both)
        .base_text(TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 18.0,
                family: FontFamily::Proportional,
            },
            Color32::BLACK,
        ))
        .layout(&(), |b, _| example::layout(b, &()))
}

fn pane(placed: &Placed<()>, index: usize) -> eggshell_ui::testing::Query<'_> {
    placed.at("grid").at("panes").at(index)
}

fn columns(placed: &Placed<()>) -> usize {
    (0..PANES)
        .map(|index| (pane(placed, index).rect().min.x * 10.0).round() as i64)
        .collect::<BTreeSet<_>>()
        .len()
}

fn form(placed: &Placed<()>, index: usize) -> Form {
    let title = pane(placed, index).at("title");
    let prefix = format!("{}/", title.path());
    let titled = placed
        .nodes()
        .any(|node| node.path().starts_with(&prefix) && node.kind() == NodeKind::Text);
    let add = title.at("add").rect();
    if add.width() > add.height() + 1.0 {
        Form::Full
    } else if titled {
        Form::Compact
    } else {
        Form::Icons
    }
}

fn richness(form: Form) -> u8 {
    match form {
        Form::Icons => 0,
        Form::Compact => 1,
        Form::Full => 2,
    }
}

#[test]
fn the_grid_has_as_many_columns_as_fit() {
    let widths = [320.0, 480.0, 700.0, 1000.0, 1400.0];
    let counts: Vec<usize> = widths.iter().map(|&width| columns(&place(width))).collect();
    assert_eq!(counts[0], 1);
    assert!(
        counts.windows(2).all(|pair| pair[0] <= pair[1]),
        "wider windows do not lose columns: {counts:?}"
    );
    assert!(counts[4] >= 4, "{counts:?}");
    for (&width, &count) in widths.iter().zip(&counts) {
        let placed = place(width);
        let first = pane(&placed, 0).rect();
        if count > 1 {
            assert!(
                first.width() >= 200.0,
                "no pane is narrower than the least it is given at {width}: {first:?}"
            );
        }
    }
}

#[test]
fn title_bars_fall_back_from_full_to_compact_to_icons_as_the_pane_narrows() {
    let mut seen = BTreeSet::new();
    let mut by_width: Vec<(i64, u8)> = Vec::new();
    for width in (160..=1600).step_by(20) {
        let placed = place(width as f32);
        for index in 0..PANES {
            let pane_rect = pane(&placed, index).rect();
            let kind = form(&placed, index);
            seen.insert(richness(kind));
            if index == 0 {
                by_width.push((pane_rect.width().round() as i64, richness(kind)));
            }
            let bar = pane(&placed, index).at("title").rect();
            let close = pane(&placed, index).at("title").at("close").rect();
            assert!(
                bar.contains_rect(close),
                "the chosen form fits: {kind:?} at {width} for pane {index}"
            );
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from([0, 1, 2]),
        "precondition: the sweep shows every form"
    );
    by_width.sort_unstable();
    assert!(
        by_width.windows(2).all(|pair| pair[0].1 <= pair[1].1),
        "a wider pane never says less: {by_width:?}"
    );
}
