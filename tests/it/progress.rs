//! `Builder::progress`: the style's progress track and range, announced as a
//! progress indicator.

use eggshell_ui::{Builder, Progress, Style, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::epaint::Shape;
use egui::{Rect, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, box_at, rect_of, shapes};

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

fn harness(progress: Progress) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 100.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App { progress },
        view,
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(6);
    harness
}

fn style() -> Style {
    Shadcn::for_theme(Theme::Light).style()
}

#[test]
fn the_bar_is_the_styles_track_and_range_filled_to_the_fraction() {
    let harness = harness(Progress::new(0.4));
    let progress = style().progress;
    let bar = rect_of(&harness, "/\"upload\"");
    assert_eq!(progress.height, 8.0, "shadcn h-2");
    assert_ne!(progress.track.fill.a(), u8::MAX, "the track is translucent");
    assert_eq!(bar.width(), 200.0);
    assert_eq!(bar.height(), progress.height);
    assert_eq!(box_at(&harness, bar).fill, progress.track.fill);
    let fill = shapes(&harness)
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
        let harness = harness(Progress::new(progress));
        let node = harness.get_by_role_and_label(Role::ProgressIndicator, "Upload");
        assert_eq!(node.accesskit_node().numeric_value(), Some(percent));
    }
}
