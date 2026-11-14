//! The examples ported onto `virtual_list`: egui_taffy's virtual grid and
//! hello_egui's virtual list example, its stable-id test, the chat and the
//! stargazers. Each builds only the rows in and near its viewport, and the
//! rows it shows are the ones its data puts there.

use eggshell_ui::{TextStyle, scroll, vlist};
use eggshell_ui_shadcn::Tokens;
use egui::{Align, Rect, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, View};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/virtual_grid.rs"]
mod virtual_grid;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/virtual_list_example.rs"]
mod virtual_list_example;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/virtual_list_stable_id.rs"]
mod virtual_list_stable_id;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/stargazers.rs"]
mod stargazers;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/chat.rs"]
mod chat;

fn harness<A>(app: A, view: View<A>) -> Harness<'static, Rig<A>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(640.0, 480.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.state_mut().companion = support::Companion::Alone;
    harness.run_steps(4);
    harness
}

/// The rows of the list `list` laid out in the last frame, by the readable
/// path's last segment, with their rects, in order.
fn rows<A>(harness: &Harness<'_, Rig<A>>, list: &str) -> Vec<(String, Rect)> {
    let prefix = format!("{list}/<rows>/");
    harness
        .state()
        .nodes
        .iter()
        .filter_map(|(path, rect)| {
            let row = path.strip_prefix(&prefix)?.rsplit('/').next()?;
            let keyed = path
                .strip_prefix(&prefix)?
                .split('/')
                .filter(|part| !part.starts_with('<'))
                .count()
                == 1;
            (keyed && !row.starts_with('<')).then(|| (row.to_owned(), *rect))
        })
        .collect()
}

fn viewport<A>(harness: &Harness<'_, Rig<A>>, list: &str) -> scroll::Viewport {
    scroll::viewport(&harness.ctx, support::id_of(harness, list)).expect("the list was laid out")
}

/// The rows of `list` that can be seen: inside its viewport, give or take a
/// pixel of rounding.
fn seen<A>(harness: &Harness<'_, Rig<A>>, list: &str) -> Vec<String> {
    let view = viewport(harness, list).rect;
    rows(harness, list)
        .into_iter()
        .filter(|(_, rect)| rect.intersects(view.shrink(1.0)))
        .map(|(row, _)| row)
        .collect()
}

mod grid {
    use super::*;
    use virtual_grid::{ROWS, Row, State, layout};

    #[test]
    fn it_shows_its_first_rows_and_builds_only_a_screenful() {
        let harness = harness(State::default(), layout);
        let seen = seen(&harness, "/Grid");
        assert_eq!(seen.first().map(String::as_str), Some("Row(0)"));
        assert!(seen.len() > 5, "a screenful: {seen:?}");
        let built = rows(&harness, "/Grid").len();
        assert!(built < 60, "{built} of {ROWS} rows built");
    }

    #[test]
    fn odd_rows_have_two_cells_and_even_ones_one_that_spans_both() {
        let harness = harness(State::default(), layout);
        let cells = |row: u32| {
            harness
                .state()
                .nodes
                .iter()
                .filter(|(path, _)| {
                    path.starts_with(&format!("/Grid/<rows>/Row({row})/Cell("))
                        && path.matches('/').count() == 4
                })
                .count()
        };
        assert_eq!((cells(0), cells(1), cells(2), cells(3)), (1, 2, 1, 2));
    }

    #[test]
    fn a_row_far_down_is_shown_where_its_index_puts_it() {
        let mut harness = harness(State::default(), layout);
        let list = support::id_of(&harness, "/Grid");
        vlist::scroll_to(&harness.ctx, list, Row(60_001), Some(Align::Min));
        harness.run_steps(3);
        let seen = seen(&harness, "/Grid");
        assert_eq!(seen.first().map(String::as_str), Some("Row(60001)"));
    }
}

mod items {
    use super::*;
    use virtual_list_example::{ITEMS, Item, State, extra, layout};

    #[test]
    fn each_item_is_as_tall_as_its_index_makes_it() {
        let harness = harness(State::default(), layout);
        let rows = rows(&harness, "/List");
        assert!(rows.len() > 3, "precondition: items built");
        let base = rows[0].1.height() - extra(Item(0));
        for (row, rect) in &rows {
            let index: u32 = row
                .strip_prefix("Item(")
                .and_then(|row| row.strip_suffix(')'))
                .and_then(|row| row.parse().ok())
                .expect("an item row");
            assert!(
                (rect.height() - extra(Item(index)) - base).abs() < 0.5,
                "{row} is {} tall",
                rect.height()
            );
        }
    }

    #[test]
    fn the_last_of_a_hundred_thousand_items_can_be_scrolled_to() {
        let mut harness = harness(State::default(), layout);
        let list = support::id_of(&harness, "/List");
        let last = Item(ITEMS - 1);
        vlist::scroll_to(&harness.ctx, list, last, Some(Align::Max));
        harness.run_steps(4);
        let seen = seen(&harness, "/List");
        assert!(
            seen.contains(&format!("{last:?}")),
            "{last:?} is seen: {seen:?}"
        );
        assert!(rows(&harness, "/List").len() < 40, "a screenful built");
    }
}

mod stable_ids {
    use super::*;
    use virtual_list_stable_id::{Row, State, layout};

    /// The id the button of row `row` registered, and the label beside it.
    fn hello(harness: &Harness<'_, Rig<State>>, row: u32) -> (egui::Id, String) {
        let id = support::id_of(harness, &format!("/List/<rows>/Row({row})/Hello"));
        let label = support::shapes(harness)
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text)
                    if text.galley.text().ends_with(&format!("index: {row}")) =>
                {
                    Some(text.galley.text().to_owned())
                }
                _ => None,
            })
            .expect("the row's label is painted");
        (id, label)
    }

    #[test]
    fn a_row_keeps_its_ids_when_scrolled_away_and_back() {
        let mut harness = harness(State::default(), layout);
        let before = hello(&harness, 3);
        assert!(
            before.1.contains(&format!("{:016X}", before.0.value())),
            "the label names the button's id: {before:?}"
        );
        let list = support::id_of(&harness, "/List");
        vlist::scroll_to(&harness.ctx, list, Row(99), Some(Align::Max));
        harness.run_steps(4);
        assert!(
            !rows(&harness, "/List")
                .iter()
                .any(|(row, _)| row == "Row(3)"),
            "precondition: row 3 is no longer built"
        );
        vlist::scroll_to(&harness.ctx, list, Row(0), Some(Align::Min));
        harness.run_steps(4);
        assert!(seen(&harness, "/List").contains(&"Row(3)".to_owned()));
        assert_eq!(hello(&harness, 3), before);
    }
}

mod people {
    use super::*;
    use stargazers::{PEOPLE, State, layout};

    #[test]
    fn it_shows_the_first_people_and_builds_only_a_screenful() {
        let harness = harness(State::default(), layout);
        let seen = seen(&harness, "/List");
        assert_eq!(seen.first().map(String::as_str), Some("Person(0)"));
        let built = rows(&harness, "/List").len();
        assert!(built < 40, "{built} of {PEOPLE} built");
    }
}

mod messages {
    use super::*;
    use chat::{ARRIVAL, HISTORY, State, layout};

    fn last_seen(harness: &Harness<'_, Rig<State>>) -> Option<String> {
        seen(harness, "/List").last().cloned()
    }

    #[test]
    fn it_opens_on_the_last_message() {
        let harness = harness(State::default(), layout);
        let view = viewport(&harness, "/List").rect;
        let rows = rows(&harness, "/List");
        let (last, rect) = rows.last().expect("messages built");
        assert_eq!(last, &format!("MessageId({})", HISTORY - 1));
        assert!(
            (rect.bottom() - view.bottom()).abs() < 1.0,
            "it ends the viewport: {rect:?} in {view:?}"
        );
    }

    #[test]
    fn arriving_messages_show_at_the_bottom() {
        let mut harness = harness(State::default(), layout);
        let steps = (ARRIVAL / 0.05).ceil() as usize + 2;
        harness.run_steps(steps);
        harness.run_steps(4);
        assert_eq!(
            last_seen(&harness),
            Some(format!("MessageId({HISTORY})")),
            "the first arrival is the last message seen"
        );
        let view = viewport(&harness, "/List").rect;
        let (_, rect) = rows(&harness, "/List").last().cloned().expect("built");
        assert!(
            (rect.bottom() - view.bottom()).abs() < 1.0,
            "and it ends the viewport: {rect:?} in {view:?}"
        );
    }
}
