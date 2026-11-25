//! The separator: shadcn's `separator`, a line in the `border` token.

use eggshell_ui::owned::separator::Orientation;
use eggshell_ui::testing::Headless;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, NodeKind, tw};
use egui::{Rect, vec2};

/// The rect and fill of the one line `view` lays out.
fn line(view: fn(&mut Builder<'_, '_, ()>, &())) -> (Rect, Option<egui::Color32>) {
    let placed = Headless::new(vec2(300.0, 200.0))
        .theme(egui::Theme::Light)
        .layout(&(), view);
    let lines: Vec<_> = placed
        .nodes()
        .filter(|node| node.kind() == NodeKind::Div && node.paint().background.is_some())
        .map(|node| (node.rect(), node.paint().background))
        .collect();
    assert_eq!(lines.len(), 1, "precondition: one filled node, the line");
    lines[0]
}

#[test]
fn a_horizontal_separator_is_a_full_width_point_in_the_border_token() {
    let (rect, fill) = line(|b, _| {
        b.div(tw().w_px(200.0).flex().flex_col(), |b| {
            b.separator(Orientation::Horizontal);
        });
    });
    assert_eq!(rect.size(), vec2(200.0, 1.0), "h-px w-full");
    assert_eq!(fill, Some(Tokens::light().border), "bg-border");
}

#[test]
fn a_vertical_separator_is_a_point_wide_and_stretches_down_its_row() {
    let (rect, _) = line(|b, _| {
        b.div(tw().h_px(40.0).flex().flex_row(), |b| {
            b.separator(Orientation::Vertical);
        });
    });
    assert_eq!(rect.size(), vec2(1.0, 40.0), "w-px self-stretch");
}
