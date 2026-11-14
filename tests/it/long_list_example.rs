//! The `long_list` example: 10,000 rows of mixed height, of which only those
//! near the viewport are built, and buttons that scroll to a row.

use eggshell_ui::TextStyle;
use eggshell_ui_shadcn::Tokens;
use egui::{Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/long_list.rs"]
mod example;

use example::{ROWS, Row, State};

fn harness() -> Harness<'static, Rig<State>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(480.0, 640.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        State::default(),
        example::layout,
    );
    harness.state_mut().companion = support::Companion::Alone;
    harness.run_steps(4);
    harness
}

/// The rows built in the last frame, by the readable path's last segment.
fn built(harness: &Harness<'_, Rig<State>>) -> Vec<String> {
    harness
        .state()
        .nodes
        .iter()
        .filter_map(|(path, _)| {
            let last = path.rsplit('/').next()?;
            (last.starts_with("Day(") || last.starts_with("Note(")).then(|| last.to_owned())
        })
        .collect()
}

fn list_viewport(harness: &Harness<'_, Rig<State>>) -> eggshell_ui::scroll::Viewport {
    let list = support::id_of(harness, "/List");
    eggshell_ui::scroll::viewport(&harness.ctx, list).expect("the list was laid out")
}

#[test]
fn only_a_screenful_of_the_ten_thousand_rows_is_built() {
    let harness = harness();
    assert_eq!(harness.state().app.rows.len(), ROWS);
    let built = built(&harness);
    assert_eq!(built.first().map(String::as_str), Some("Day(0)"));
    assert!(built.len() < 80, "{} rows built of {ROWS}", built.len());
}

#[test]
fn the_buttons_scroll_to_the_last_row_and_back_to_the_first() {
    let mut harness = harness();
    let last = *harness.state().app.rows.last().expect("rows");
    let last = match last {
        Row::Day(day) => format!("Day({day})"),
        Row::Note(note) => format!("Note({note})"),
    };
    click_node(&mut harness, "/Last");
    harness.run_steps(4);
    assert!(built(&harness).contains(&last), "the last row is built");
    let view = list_viewport(&harness);
    assert!(
        view.offset().y > 100_000.0,
        "scrolled far down: {:?}",
        view.offset()
    );
    click_node(&mut harness, "/First");
    harness.run_steps(4);
    assert_eq!(list_viewport(&harness).offset().y, 0.0);
    assert_eq!(built(&harness).first().map(String::as_str), Some("Day(0)"));
}
