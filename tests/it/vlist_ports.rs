//! The examples ported onto `virtual_list`: egui_taffy's virtual grid and
//! hello_egui's virtual list example, its stable-id test, the chat and the
//! stargazers. Each builds only the rows in and near its viewport, and the
//! rows it shows are the ones its data puts there.

use eggshell_ui::testing::Driver;
use eggshell_ui::{TextStyle, scroll};
use eggshell_ui_shadcn::Tokens;
use egui::{Align, Rect, Theme, vec2};

use crate::support::{self, View};

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

fn driver<A>(app: A, view: View<A>) -> Driver<A> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(640.0, 480.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .build();
    support::set_companion(&d, support::Companion::Alone);
    d.frames(4);
    d
}

/// The rows of the list `list` laid out in the last frame, by the readable
/// path's last segment, with their rects, in order.
fn rows<A>(d: &Driver<A>, list: &str) -> Vec<(String, Rect)> {
    let prefix = format!("{list}/<rows>/");
    support::nodes(d)
        .into_iter()
        .filter_map(|(path, rect)| {
            let row = path.strip_prefix(&prefix)?.rsplit('/').next()?;
            let keyed = path
                .strip_prefix(&prefix)?
                .split('/')
                .filter(|part| !part.starts_with('<'))
                .count()
                == 1;
            (keyed && !row.starts_with('<')).then(|| (row.to_owned(), rect))
        })
        .collect()
}

fn viewport<A>(d: &Driver<A>, list: &str) -> scroll::Viewport {
    d.shell()
        .viewport(support::id_of(d, list))
        .expect("the list was laid out")
}

/// The rows of `list` that can be seen: inside its viewport, give or take a
/// pixel of rounding.
fn seen<A>(d: &Driver<A>, list: &str) -> Vec<String> {
    let view = viewport(d, list).rect;
    rows(d, list)
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
        let d = driver(State::default(), layout);
        let seen = seen(&d, "/Grid");
        assert_eq!(seen.first().map(String::as_str), Some("Row(0)"));
        assert!(seen.len() > 5, "a screenful: {seen:?}");
        let built = rows(&d, "/Grid").len();
        assert!(built < 60, "{built} of {ROWS} rows built");
    }

    #[test]
    fn odd_rows_have_two_cells_and_even_ones_one_that_spans_both() {
        let d = driver(State::default(), layout);
        let cells = |row: u32| {
            support::nodes(&d)
                .into_iter()
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
        let mut d = driver(State::default(), layout);
        let list = support::id_of(&d, "/Grid");
        d.shell().scroll_to_row(list, Row(60_001), Some(Align::Min));
        d.frames(3);
        let seen = seen(&d, "/Grid");
        assert_eq!(seen.first().map(String::as_str), Some("Row(60001)"));
    }
}

mod items {
    use super::*;
    use virtual_list_example::{ITEMS, Item, State, extra, layout};

    #[test]
    fn each_item_is_as_tall_as_its_index_makes_it() {
        let d = driver(State::default(), layout);
        let rows = rows(&d, "/List");
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
        let mut d = driver(State::default(), layout);
        let list = support::id_of(&d, "/List");
        let last = Item(ITEMS - 1);
        d.shell().scroll_to_row(list, last, Some(Align::Max));
        d.frames(4);
        let seen = seen(&d, "/List");
        assert!(
            seen.contains(&format!("{last:?}")),
            "{last:?} is seen: {seen:?}"
        );
        assert!(rows(&d, "/List").len() < 40, "a screenful built");
    }
}

mod stable_ids {
    use super::*;
    use virtual_list_stable_id::{Row, State, layout};

    /// The id the button of row `row` registered, and the label beside it.
    fn hello(d: &Driver<State>, row: u32) -> (egui::Id, String) {
        let id = support::id_of(d, &format!("/List/<rows>/Row({row})/Hello"));
        let label = support::shapes(d)
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
        let mut d = driver(State::default(), layout);
        let before = hello(&d, 3);
        assert!(
            before.1.contains(&format!("{:016X}", before.0.value())),
            "the label names the button's id: {before:?}"
        );
        let list = support::id_of(&d, "/List");
        d.shell().scroll_to_row(list, Row(99), Some(Align::Max));
        d.frames(4);
        assert!(
            !rows(&d, "/List").iter().any(|(row, _)| row == "Row(3)"),
            "precondition: row 3 is no longer built"
        );
        d.shell().scroll_to_row(list, Row(0), Some(Align::Min));
        d.frames(4);
        assert!(seen(&d, "/List").contains(&"Row(3)".to_owned()));
        assert_eq!(hello(&d, 3), before);
    }
}

mod people {
    use super::*;
    use stargazers::{PEOPLE, State, layout};

    #[test]
    fn it_shows_the_first_people_and_builds_only_a_screenful() {
        let d = driver(State::default(), layout);
        let seen = seen(&d, "/List");
        assert_eq!(seen.first().map(String::as_str), Some("Person(0)"));
        let built = rows(&d, "/List").len();
        assert!(built < 40, "{built} of {PEOPLE} built");
    }
}

mod messages {
    use super::*;
    use chat::{ARRIVAL, HISTORY, State, layout};

    fn last_seen(d: &Driver<State>) -> Option<String> {
        seen(d, "/List").last().cloned()
    }

    #[test]
    fn it_opens_on_the_last_message() {
        let d = driver(State::default(), layout);
        let view = viewport(&d, "/List").rect;
        let rows = rows(&d, "/List");
        let (last, rect) = rows.last().expect("messages built");
        assert_eq!(last, &format!("MessageId({})", HISTORY - 1));
        assert!(
            (rect.bottom() - view.bottom()).abs() < 1.0,
            "it ends the viewport: {rect:?} in {view:?}"
        );
    }

    #[test]
    fn arriving_messages_show_at_the_bottom() {
        let mut d = driver(State::default(), layout);
        let steps = (ARRIVAL / 0.05).ceil() as usize + 2;
        d.frames(steps);
        d.frames(4);
        assert_eq!(
            last_seen(&d),
            Some(format!("MessageId({HISTORY})")),
            "the first arrival is the last message seen"
        );
        let view = viewport(&d, "/List").rect;
        let (_, rect) = rows(&d, "/List").last().cloned().expect("built");
        assert!(
            (rect.bottom() - view.bottom()).abs() < 1.0,
            "and it ends the viewport: {rect:?} in {view:?}"
        );
    }
}
