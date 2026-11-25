//! `Builder::progress`: the style's progress track and range, announced as a
//! progress indicator.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, Tokens};
use eggshell_ui::{Builder, Progress, Style, TextStyle, tw};
use egui::epaint::Shape;
use egui::{Rect, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, box_at, shapes};

struct App {
    progress: Progress,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().p(4.0).w_px(300.0), |b| {
        b.progress("upload", app.progress)
            .named("Upload")
            .classes(tw().w_px(200.0))
            .add();
    });
}

fn harness(progress: Progress) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 100.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App { progress },
        view,
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(6);
    d
}

fn style() -> Style {
    Shadcn::for_theme(Theme::Light).style()
}

#[test]
fn the_bar_is_the_styles_track_and_range_filled_to_the_fraction() {
    let d = harness(Progress::new(0.4));
    let progress = style().progress;
    let bar = d.rect(Target::suffix("/\"upload\""));
    let tokens = eggshell_ui::theme::Tokens::for_theme(Theme::Light);
    assert_eq!(progress.range.fill, tokens.primary, "bg-primary");
    assert_eq!(bar.width(), 200.0);
    assert_eq!(bar.height(), 6.0, "vega h-1.5");
    // Recorded exception (#132): the tracks are the input token, not `bg-muted`.
    assert_eq!(box_at(&d, bar).fill, tokens.input);
    let fill = shapes(&d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Path(path) if path.fill == progress.range.fill => {
                Some(Rect::from_points(&path.points))
            }
            _ => None,
        })
        .expect("the fill was painted");
    assert_eq!(fill.left(), bar.left());
    assert_eq!(fill.width(), 0.4 * bar.width(), "ends at the value");
    assert_eq!((fill.top(), fill.bottom()), (bar.top(), bar.bottom()));
}

#[test]
fn it_is_a_progress_indicator_with_the_percentage_as_its_value() {
    for (progress, percent) in [(0.0, 0.0), (0.4, 40.0), (1.0, 100.0), (7.0, 100.0)] {
        let d = harness(Progress::new(progress));
        let node = d
            .harness()
            .get_by_role_and_label(Role::ProgressIndicator, "Upload");
        assert_eq!(node.accesskit_node().numeric_value(), Some(percent));
    }
}

/// The bar draws from the style's sheet: an App rule restyles its track.
#[test]
fn an_app_rule_restyles_the_track() {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 100.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App {
            progress: Progress::new(0.4),
        },
        view,
    )
    .style(support::shadcn_with(|style| {
        style
            .sheet
            .as_app()
            .rule(eggshell_ui::owned::progress::Part::Root, |t, _| {
                t.bg(egui::Color32::RED)
            });
    }))
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(6);
    let bar = d.rect(Target::suffix("/\"upload\""));
    assert_eq!(box_at(&d, bar).fill, egui::Color32::RED);
}
