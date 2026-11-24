//! The segmented control's look as public parts: a track and cells built
//! from `Builder::part` with `TabsPart::List` and `TabsPart::Trigger` and
//! their keys draw exactly as `Builder::segmented` does, at the size and enablement a
//! subtree gives them (`Builder::control_size`, `Builder::enablement`).

use eggshell_ui::contract::{Disabled, Size, Style};
use eggshell_ui::owned::{Cell, SegmentedRows, TabsPart};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Icon, Opacity, TextStyle, tw};
use egui::{Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, box_at, rect_of, text_color};

const SM: Size = Size("sm");
const LABELS: [&str; 3] = ["Day", "Week", "Month"];

fn icon() -> Icon {
    Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
        egui::TextureId::default(),
        vec2(4.0, 4.0),
    )))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ambient {
    Plain,
    Small,
    Inert,
    /// Inert, under a style that draws disabled controls faded.
    InertFaded,
}

struct App {
    ambient: Ambient,
    rows: SegmentedRows,
}

fn edit(style: &mut Style, ambient: Ambient) {
    // The disabled token reaches the widgets' own look, not the preset's.
    if matches!(ambient, Ambient::Inert | Ambient::InertFaded) {
        *style = Style::default();
    }
    if ambient == Ambient::InertFaded {
        style.tokens.disabled = Disabled::Faded(Opacity::new(0.4));
    }
    style.sheet.as_app();
}

/// The library's control, then the same one built from the public classes.
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let around = match app.ambient {
        Ambient::Plain => tw(),
        Ambient::Small => tw().control_size(SM),
        Ambient::Inert | Ambient::InertFaded => tw().inert(),
    };
    let rows = app.rows;
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.div(around.clone(), |b| {
            let options = LABELS
                .into_iter()
                .enumerate()
                .map(|(index, label)| (index, icon(), label));
            let chain = b.segmented("library", 1, options);
            let chain = if rows == SegmentedRows::Wrapping {
                chain.wrapping()
            } else {
                chain
            };
            chain.on_change(|_: &mut App, _| {});
        });
        b.div(around, |b| {
            let size = TabsPart::size_key(b, b.control_size());
            let enablement = b.enablement();
            let track = b.part(
                TabsPart::List,
                &TabsPart::list_keys(size, rows, enablement),
                tw(),
            );
            b.div_id("public", track, |b| {
                for (index, label) in LABELS.into_iter().enumerate() {
                    let cell = if index == 1 {
                        Cell::Pressed
                    } else {
                        Cell::Released
                    };
                    let keys = TabsPart::trigger_keys(size, rows, cell, enablement);
                    let classes = b.part(TabsPart::Trigger, &keys, tw().sense_click());
                    b.div_id(index, classes, |b| {
                        b.glyph(tw(), icon());
                        b.text(tw(), label);
                    });
                }
            });
        });
    });
}

fn harness(ambient: Ambient, rows: SegmentedRows) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(500.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App { ambient, rows },
        view,
    );
    harness
        .state_mut()
        .layout
        .set_style(support::shadcn_with(|style| edit(style, ambient)));
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(12);
    harness
}

fn same_look(ambient: Ambient, rows: SegmentedRows) {
    let harness = harness(ambient, rows);
    let (library, public) = ("/\"library\"", "/\"public\"");
    let nodes = &harness.state().nodes;
    let origin = |path: &str| rect_of(&harness, path).min;
    let ours: Vec<_> = nodes
        .iter()
        .filter_map(|(path, rect)| Some((path.strip_prefix(library)?, *rect)))
        .collect();
    let theirs: Vec<_> = nodes
        .iter()
        .filter_map(|(path, rect)| Some((path.strip_prefix(public)?, *rect)))
        .collect();
    assert!(
        ours.len() > 3 * LABELS.len(),
        "precondition: the track, its cells, their icons and labels: {ours:?}"
    );
    assert_eq!(ours.len(), theirs.len(), "{ours:?}\n{theirs:?}");
    for ((path, mine), (other, built)) in ours.iter().zip(&theirs) {
        assert_eq!(path, other);
        assert_eq!(mine.size(), built.size(), "{path:?}");
        assert_eq!(
            mine.min - origin(library),
            built.min - origin(public),
            "{path:?} sits where it does in the library's track"
        );
    }
    let fill = |path: String| box_at(&harness, rect_of(&harness, &path)).fill;
    assert_eq!(
        fill(library.into()),
        fill(public.into()),
        "the track's fill"
    );
    for (index, label) in LABELS.into_iter().enumerate() {
        let (mine, built) = (format!("{library}/{index}"), format!("{public}/{index}"));
        assert_eq!(
            fill(mine.clone()),
            fill(built.clone()),
            "cell {index}'s fill"
        );
        assert_eq!(
            text_color(&harness, rect_of(&harness, &mine), label),
            text_color(&harness, rect_of(&harness, &built), label),
            "cell {index}'s text"
        );
    }
}

#[test]
fn public_classes_draw_the_library_s_control() {
    same_look(Ambient::Plain, SegmentedRows::One);
}

#[test]
fn they_follow_the_size_a_subtree_gives() {
    let small = harness(Ambient::Small, SegmentedRows::One);
    let plain = harness(Ambient::Plain, SegmentedRows::One);
    assert_ne!(
        rect_of(&small, "/\"public\"").height(),
        rect_of(&plain, "/\"public\"").height(),
        "precondition: the small size is another height"
    );
    same_look(Ambient::Small, SegmentedRows::One);
}

#[test]
fn they_follow_an_inert_subtree() {
    same_look(Ambient::Inert, SegmentedRows::One);
}

#[test]
fn a_wrapping_track_matches_too() {
    same_look(Ambient::Plain, SegmentedRows::Wrapping);
}
