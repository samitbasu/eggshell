//! `Builder::progress`: the slider's track and range, announced as a progress
//! indicator.

use eggshell_ui::{Builder, Progress, Style, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::{Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, box_at, rect_of};

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
fn the_bar_is_the_sliders_track_and_range_filled_to_the_fraction() {
    let harness = harness(Progress::new(0.4));
    let slider = style().slider;
    let bar = rect_of(&harness, "/\"upload\"");
    assert_eq!(bar.width(), 200.0);
    assert_eq!(bar.height(), slider.track_height);
    assert_eq!(box_at(&harness, bar).fill, slider.track.rest.fill);
    let range = rect_of(&harness, "/\"upload\"/<div>");
    assert_eq!(range.width(), 80.0);
    assert_eq!(range.height(), slider.track_height);
    assert_eq!(box_at(&harness, range).fill, slider.range.rest.fill);
}

#[test]
fn it_is_a_progress_indicator_with_the_percentage_as_its_value() {
    for (progress, percent) in [(0.0, 0.0), (0.4, 40.0), (1.0, 100.0), (7.0, 100.0)] {
        let harness = harness(Progress::new(progress));
        let node = harness.get_by_role_and_label(Role::ProgressIndicator, "Upload");
        assert_eq!(node.accesskit_node().numeric_value(), Some(percent));
    }
}
