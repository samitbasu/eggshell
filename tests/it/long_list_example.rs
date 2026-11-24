//! The `long_list` example: 10,000 rows of mixed height, of which only those
//! near the viewport are built, and buttons that scroll to a row.

use eggshell_ui::TextStyle;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use egui::{Theme, vec2};

use crate::support;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/long_list.rs"]
mod example;

use example::{ROWS, Row, State};

fn harness() -> Driver<State> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(480.0, 640.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        State::default(),
        example::layout,
    )
    .build();
    support::set_companion(&d, support::Companion::Alone);
    d.frames(4);
    d
}

/// The rows built in the last frame, by the readable path's last segment.
fn built(d: &Driver<State>) -> Vec<String> {
    support::nodes(d)
        .iter()
        .filter_map(|(path, _)| {
            let last = path.rsplit('/').next()?;
            (last.starts_with("Day(") || last.starts_with("Note(")).then(|| last.to_owned())
        })
        .collect()
}

/// The last row's readable key.
fn last_row(d: &Driver<State>) -> String {
    match *d.app().rows.last().expect("rows") {
        Row::Day(day) => format!("Day({day})"),
        Row::Note(note) => format!("Note({note})"),
    }
}

fn rect_of_row(d: &Driver<State>, row: &str) -> Option<egui::Rect> {
    let suffix = format!("/{row}");
    support::nodes(d)
        .iter()
        .find(|(path, _)| path.ends_with(&suffix))
        .map(|(_, rect)| *rect)
}

/// Whether the last row ends at the viewport's bottom.
fn at_the_end(d: &Driver<State>) -> bool {
    let view = list_viewport(d).rect;
    rect_of_row(d, &last_row(d)).is_some_and(|row| (row.bottom() - view.bottom()).abs() < 0.5)
}

fn list_viewport(d: &Driver<State>) -> eggshell_ui::scroll::Viewport {
    let list = support::id_of(d, "/List");
    d.shell().viewport(list).expect("the list was laid out")
}

#[test]
fn only_a_screenful_of_the_ten_thousand_rows_is_built() {
    let d = harness();
    assert_eq!(d.app().rows.len(), ROWS);
    let built = built(&d);
    assert_eq!(built.first().map(String::as_str), Some("Day(0)"));
    assert!(built.len() < 80, "{} rows built of {ROWS}", built.len());
}

#[test]
fn the_buttons_scroll_to_the_last_row_and_back_to_the_first() {
    let mut d = harness();
    d.click(Target::suffix("/Last")).frames(4);
    let view = list_viewport(&d);
    let last = rect_of_row(&d, &last_row(&d)).expect("the last row is built");
    assert!(
        view.rect.contains_rect(last),
        "the last row {last:?} is in the viewport {:?}",
        view.rect
    );
    d.click(Target::suffix("/First")).frames(4);
    assert_eq!(list_viewport(&d).offset().y, 0.0);
    assert_eq!(built(&d).first().map(String::as_str), Some("Day(0)"));
}

#[test]
fn one_scroll_past_the_end_lands_on_the_last_row_without_further_input() {
    let mut d = harness();
    d.resize(vec2(1400.0, 1000.0)).frames(3);
    let list = support::id_of(&d, "/List");
    assert!(!at_the_end(&d), "precondition: the list starts at the top");
    d.shell().scroll_by(list, vec2(0.0, 1_000_000.0));
    let mut frames = 0;
    while !at_the_end(&d) && frames < 10 {
        d.frames(1);
        frames += 1;
    }
    assert!(
        frames <= 3,
        "the last row {:?} reached the viewport's bottom {:?} after {frames} frames",
        rect_of_row(&d, &last_row(&d)),
        list_viewport(&d).rect
    );
    let mut more = 0;
    while d.ctx().has_requested_repaint() && more < 10 {
        d.frames(1);
        more += 1;
    }
    assert!(
        !d.ctx().has_requested_repaint(),
        "the frames stop once it is settled"
    );
    assert!(at_the_end(&d), "and it stays at the end");
}
