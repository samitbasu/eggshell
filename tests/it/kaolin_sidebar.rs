//! The kaolin gallery's sidebar rows hold stock buttons the library sizes:
//! no row overlaps the next, and each button is drawn inside its row.

use bullseye::LayoutState;
use egui::accesskit::Role;
use egui::{ImageSource, Rect, Theme, vec2};
use egui_kittest::Harness;
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

struct Rig {
    gallery: kaolin::Gallery,
    rows: Vec<Rect>,
}

/// The rects of the sidebar's rows, and of the buttons drawn in them.
fn sidebar(styling: Styling, theme: Theme) -> (Vec<Rect>, Vec<Rect>) {
    let mut layout = LayoutState::new().with_base_text(common::base_text(theme));
    let mut harness = Harness::builder()
        .with_size(vec2(900.0, 700.0))
        .with_theme(theme)
        .build_ui_state(
            move |ui, rig: &mut Rig| {
                if matches!(styling, Styling::Preset) {
                    common::use_preset(ui.ctx());
                }
                let area = ui.max_rect();
                let selected = rig.gallery.selected;
                let placed = layout
                    .build(ui.ctx(), |b| {
                        kaolin::window(b, selected, ImageSource::Uri("bytes://logo".into()));
                    })
                    .layout(area);
                rig.rows = placed
                    .nodes()
                    .filter(|node| node.path().starts_with("/(\"entry\", "))
                    .map(|node| node.rect())
                    .collect();
                placed.draw(&mut rig.gallery, ui);
            },
            Rig {
                gallery: kaolin::Gallery { selected: 1 },
                rows: Vec::new(),
            },
        );
    harness.run_steps(3);

    let buttons = kaolin::ENTRIES
        .iter()
        .map(|(name, _)| harness.get_by_role_and_label(Role::Button, name).rect())
        .collect();
    (harness.state().rows.clone(), buttons)
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
