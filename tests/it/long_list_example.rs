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

/// The last row's readable key.
fn last_row(harness: &Harness<'_, Rig<State>>) -> String {
    match *harness.state().app.rows.last().expect("rows") {
        Row::Day(day) => format!("Day({day})"),
        Row::Note(note) => format!("Note({note})"),
    }
}

fn rect_of_row(harness: &Harness<'_, Rig<State>>, row: &str) -> Option<egui::Rect> {
    let suffix = format!("/{row}");
    harness
        .state()
        .nodes
        .iter()
        .find(|(path, _)| path.ends_with(&suffix))
        .map(|(_, rect)| *rect)
}

/// Whether the last row ends at the viewport's bottom.
fn at_the_end(harness: &Harness<'_, Rig<State>>) -> bool {
    let view = list_viewport(harness).rect;
    rect_of_row(harness, &last_row(harness))
        .is_some_and(|row| (row.bottom() - view.bottom()).abs() < 0.5)
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
    click_node(&mut harness, "/Last");
    harness.run_steps(4);
    let view = list_viewport(&harness);
    let last = rect_of_row(&harness, &last_row(&harness)).expect("the last row is built");
    assert!(
        view.rect.contains_rect(last),
        "the last row {last:?} is in the viewport {:?}",
        view.rect
    );
    click_node(&mut harness, "/First");
    harness.run_steps(4);
    assert_eq!(list_viewport(&harness).offset().y, 0.0);
    assert_eq!(built(&harness).first().map(String::as_str), Some("Day(0)"));
}

#[test]
fn one_scroll_past_the_end_lands_on_the_last_row_without_further_input() {
    let mut harness = harness();
    harness.set_size(vec2(1400.0, 1000.0));
    harness.run_steps(4);
    let list = support::id_of(&harness, "/List");
    assert!(
        !at_the_end(&harness),
        "precondition: the list starts at the top"
    );
    eggshell_ui::scroll::scroll_by(&harness.ctx, list, vec2(0.0, 1_000_000.0));
    let mut frames = 0;
    while !at_the_end(&harness) && frames < 10 {
        harness.step();
        frames += 1;
    }
    assert!(
        frames <= 3,
        "the last row {:?} reached the viewport's bottom {:?} after {frames} frames",
        rect_of_row(&harness, &last_row(&harness)),
        list_viewport(&harness).rect
    );
    let mut more = 0;
    while harness.ctx.has_requested_repaint() && more < 10 {
        harness.step();
        more += 1;
    }
    assert!(
        !harness.ctx.has_requested_repaint(),
        "the frames stop once it is settled"
    );
    assert!(at_the_end(&harness), "and it stays at the end");
}
