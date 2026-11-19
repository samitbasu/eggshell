//! The kaolin gallery's sidebar rows hold stock buttons the library sizes:
//! no row overlaps the next, and each button is drawn inside its row.

use std::time::Duration;

use eggshell_ui::testing::{After, Animation, Driver, Reports};
use egui::accesskit::Role;
use egui::{ImageSource, Rect, Theme, vec2};
use egui_kittest::kittest::Queryable as _;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/kaolin/main.rs"]
mod kaolin;

#[derive(Clone, Copy, Debug)]
enum Styling {
    Preset,
    EguiDefaults,
}

/// The rects of the sidebar's rows, and of the buttons drawn in them.
fn sidebar(styling: Styling, theme: Theme) -> (Vec<Rect>, Vec<Rect>) {
    let step = Duration::from_millis(250);
    let mut d = Driver::whole_frame(
        kaolin::Gallery { selected: 1 },
        move |ui, gallery: &mut kaolin::Gallery, layout| {
            if matches!(styling, Styling::Preset) {
                common::use_preset(ui.ctx());
            }
            let area = ui.max_rect();
            let selected = gallery.selected;
            layout
                .build(ui.ctx(), |b| {
                    kaolin::window(b, selected, ImageSource::Uri("bytes://logo".into()));
                })
                .layout(area)
                .draw(gallery, ui);
        },
    )
    .size(vec2(900.0, 700.0))
    .theme(theme)
    .base_text(common::base_text(theme))
    .step(step)
    .settle_within(step * 4)
    .motion(Animation::Reduced)
    .after(After::Frames(0))
    .reports(Reports::Unchecked)
    .build();
    d.frames(3);

    let rows = d
        .frame()
        .nodes()
        .filter(|node| node.path().starts_with("/(\"entry\", ") && !node.path().ends_with("<text>"))
        .map(|node| node.rect())
        .collect();
    let buttons = kaolin::ENTRIES
        .iter()
        .map(|(name, _)| d.harness().get_by_role_and_label(Role::Button, name).rect())
        .collect();
    (rows, buttons)
}

#[test]
fn sidebar_rows_do_not_overlap_and_hold_their_buttons() {
    for styling in [Styling::Preset, Styling::EguiDefaults] {
        for theme in [Theme::Light, Theme::Dark] {
            let (rows, buttons) = sidebar(styling, theme);
            let case = format!("{styling:?}, {theme:?}");
            assert_eq!(
                rows.len(),
                kaolin::ENTRIES.len(),
                "{case}: one row per entry"
            );
            for pair in rows.windows(2) {
                assert!(
                    !pair[0].intersects(pair[1].shrink(0.01)),
                    "{case}: rows {:?} and {:?} overlap",
                    pair[0],
                    pair[1]
                );
            }
            for (index, (row, button)) in rows.iter().zip(&buttons).enumerate() {
                assert!(
                    row.expand(0.5).contains_rect(*button),
                    "{case}: button {index} {button:?} spills out of its row {row:?}"
                );
            }
        }
    }
}
