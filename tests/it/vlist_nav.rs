//! A virtual list navigated as a listbox: one Tab stop, a virtual highlight
//! the arrow, Home, End and Page keys move over all the rows, Enter and
//! Space activate (#183).

use std::cell::RefCell;
use std::collections::BTreeSet;

use eggshell_ui::overlay::Nav;
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Color32, Key, Pos2, Rect, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_node, rect_of};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Row(u32);

struct App {
    rows: Vec<Row>,
    activated: Vec<Row>,
    after: u32,
    listbox: bool,
    active_seen: RefCell<BTreeSet<Row>>,
    built: RefCell<BTreeSet<Row>>,
}

const HEIGHT: f32 = 20.0;
const VIEWPORT: f32 = 200.0;
const HIGHLIGHT: Color32 = Color32::from_rgb(255, 200, 0);
const LIST: &str = r#"/"Page"/"list""#;

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    app.active_seen.borrow_mut().clear();
    app.built.borrow_mut().clear();
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(2.0).p(2.0).w_px(240.0),
        |s| {
            let list = s
                .virtual_list("list", tw().w_px(220.0).h_px(VIEWPORT), &app.rows)
                .fixed_px(HEIGHT)
                .named("Rows")
                .nav(if app.listbox { Nav::Listbox } else { Nav::None })
                .row_classes(tw().hover(|p| p.bg(HIGHLIGHT)))
                .row_name(|row| format!("row {}", row.0))
                .on_activate(|app: &mut App, row| app.activated.push(row));
            list.items(move |s, row| {
                app.built.borrow_mut().insert(row);
                if s.is_active() {
                    app.active_seen.borrow_mut().insert(row);
                }
                s.text(tw(), format!("row {}", row.0));
            });
            s.button("after", "After")
                .on_click(|app: &mut App| app.after += 1);
        },
    );
}

fn harness() -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(300.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App {
            rows: (0..1000).map(Row).collect(),
            activated: vec![],
            after: 0,
            listbox: true,
            active_seen: RefCell::default(),
            built: RefCell::default(),
        },
        view,
    );
    harness.run_steps(6);
    harness
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(8);
}

fn focus_list(harness: &mut Harness<'_, Rig<App>>) {
    support::past_bar(harness);
    press(harness, Key::Tab);
    assert_eq!(
        focused_node(harness).as_deref(),
        Some(LIST),
        "Tab reaches the list: {:?}",
        support::focused_path(harness)
    );
}

fn active(harness: &Harness<'_, Rig<App>>) -> Vec<u32> {
    harness
        .state()
        .app
        .active_seen
        .borrow()
        .iter()
        .map(|row| row.0)
        .collect()
}

fn row_path(row: u32) -> String {
    format!("{LIST}/<rows>/Row({row})")
}

fn row_rect(harness: &Harness<'_, Rig<App>>, row: u32) -> Rect {
    rect_of(harness, &row_path(row))
}

#[test]
fn the_list_is_one_tab_stop_before_what_follows() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::Tab);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some(r#"/"Page"/"after""#)
    );
}

#[test]
fn nothing_is_highlighted_until_a_key_is_pressed() {
    let mut harness = harness();
    focus_list(&mut harness);
    assert!(active(&harness).is_empty());
}

#[test]
fn down_highlights_the_first_row_shown_and_moves_one_row_at_a_time() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness), [0]);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness), [2]);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(active(&harness), [1]);
}

#[test]
fn up_stops_at_the_first_row_and_end_goes_to_the_last_one_unbuilt_before() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(active(&harness), [0], "no wrap");
    assert!(
        !harness.state().app.built.borrow().contains(&Row(999)),
        "precondition: the last row is not built"
    );
    press(&mut harness, Key::End);
    assert_eq!(active(&harness), [999]);
    assert!(harness.state().app.built.borrow().contains(&Row(999)));
    press(&mut harness, Key::Home);
    assert_eq!(active(&harness), [0]);
}

#[test]
fn the_highlighted_row_is_scrolled_into_view() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::End);
    let list = rect_of(&harness, LIST);
    let row = row_rect(&harness, 999);
    assert!(list.contains_rect(row.shrink(0.5)), "{row:?} in {list:?}");
    for _ in 0..15 {
        press(&mut harness, Key::ArrowUp);
    }
    assert_eq!(active(&harness), [984]);
    let list = rect_of(&harness, LIST);
    let row = row_rect(&harness, 984);
    assert!(list.contains_rect(row.shrink(0.5)), "{row:?} in {list:?}");
}

#[test]
fn a_page_moves_a_screenful_of_rows() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::PageDown);
    assert_eq!(active(&harness), [10], "200 points of 20 point rows");
    press(&mut harness, Key::PageUp);
    assert_eq!(active(&harness), [0]);
}

#[test]
fn enter_and_space_activate_the_highlighted_row() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::End);
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.activated, [Row(999)]);
    press(&mut harness, Key::ArrowUp);
    press(&mut harness, Key::Space);
    assert_eq!(harness.state().app.activated, [Row(999), Row(998)]);
}

#[test]
fn enter_with_nothing_highlighted_activates_nothing() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::Enter);
    assert!(harness.state().app.activated.is_empty());
}

#[test]
fn a_click_activates_and_highlights_its_row() {
    let mut harness = harness();
    click_node(&mut harness, &row_path(3));
    harness.run_steps(6);
    assert_eq!(harness.state().app.activated, [Row(3)]);
    assert_eq!(active(&harness), [3]);
}

#[test]
fn only_the_highlighted_row_takes_the_hover_look() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    let lit = |harness: &Harness<'_, Rig<App>>, row: u32| {
        support::fill_of(harness, &row_path(row)) == Some(HIGHLIGHT)
    };
    assert!(lit(&harness, 1));
    assert!(!lit(&harness, 0));
    assert!(!lit(&harness, 2));
}

#[test]
fn the_pointer_moves_the_highlight_so_two_rows_never_look_hovered() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::ArrowDown);
    let from: Pos2 = row_rect(&harness, 2).center();
    support::hover_over(&mut harness, from);
    let at: Pos2 = row_rect(&harness, 4).center();
    support::hover_over(&mut harness, at);
    harness.run_steps(6);
    assert_eq!(active(&harness), [4]);
    let lit: Vec<u32> = (0..10)
        .filter(|&row| support::fill_of(&harness, &row_path(row)) == Some(HIGHLIGHT))
        .collect();
    assert_eq!(lit, [4]);
}

#[test]
fn the_highlighted_row_stays_built_when_the_list_is_scrolled_away() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::End);
    support::shell(&harness).scroll_to_row(support::id_of(&harness, LIST), Row(0), None);
    harness.run_steps(8);
    assert!(harness.state().app.built.borrow().contains(&Row(999)));
    assert_eq!(active(&harness), [999]);
}

#[test]
fn the_list_announces_the_highlighted_row_as_its_active_descendant() {
    let mut harness = harness();
    focus_list(&mut harness);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    let list = harness.get_by_role(egui::accesskit::Role::ListBox);
    let row = support::id_of(&harness, &row_path(1));
    let announced = list
        .accesskit_node()
        .active_descendant()
        .map(|node| (u128::from(node.id()) >> 64) as u64);
    assert_eq!(announced, Some(row.accesskit_id().0));
}

#[test]
fn a_list_that_does_not_navigate_leaves_its_rows_alone() {
    let mut harness = harness();
    harness.state_mut().app.listbox = false;
    harness.run_steps(4);
    press(&mut harness, Key::ArrowDown);
    assert!(active(&harness).is_empty());
}

#[test]
fn a_still_pointer_over_another_row_does_not_light_it_while_keys_move_the_highlight() {
    let mut harness = harness();
    focus_list(&mut harness);
    let from: Pos2 = row_rect(&harness, 5).center();
    support::hover_over(&mut harness, from);
    let over: Pos2 = row_rect(&harness, 6).center();
    support::hover_over(&mut harness, over);
    assert_eq!(active(&harness), [6], "precondition: the pointer set it");
    press(&mut harness, Key::ArrowUp);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(active(&harness), [4]);
    let lit: Vec<u32> = (0..10)
        .filter(|&row| support::fill_of(&harness, &row_path(row)) == Some(HIGHLIGHT))
        .collect();
    assert_eq!(lit, [4], "the row under the pointer is not lit as well");
}
