//! A virtual list navigated as a listbox: one Tab stop, a virtual highlight
//! the arrow, Home, End and Page keys move over all the rows, Enter and
//! Space activate (#183).

use std::cell::RefCell;
use std::collections::BTreeSet;

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::vlist::ListNav;
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Color32, Key, Pos2, Rect, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_node};

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
                .nav(if app.listbox {
                    ListNav::Listbox
                } else {
                    ListNav::None
                })
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

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
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
    )
    .build();
    d.frames(6);
    d
}

fn focus_list(d: &mut Driver<App>) {
    support::drive_past_bar(d);
    d.press(Key::Tab).frames(7);
    assert_eq!(
        focused_node(d).as_deref(),
        Some(LIST),
        "Tab reaches the list: {:?}",
        support::focused_path(d)
    );
}

fn active(d: &Driver<App>) -> Vec<u32> {
    d.app()
        .active_seen
        .borrow()
        .iter()
        .map(|row| row.0)
        .collect()
}

fn row_path(row: u32) -> String {
    format!("{LIST}/<rows>/Row({row})")
}

fn row_rect(d: &Driver<App>, row: u32) -> Rect {
    d.rect(Target::suffix(&row_path(row)))
}

#[test]
fn the_list_is_one_tab_stop_before_what_follows() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::Tab).frames(7);
    assert_eq!(focused_node(&d).as_deref(), Some(r#"/"Page"/"after""#));
}

#[test]
fn nothing_is_highlighted_until_a_key_is_pressed() {
    let mut d = harness();
    focus_list(&mut d);
    assert!(active(&d).is_empty());
}

#[test]
fn down_highlights_the_first_row_shown_and_moves_one_row_at_a_time() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::ArrowDown).frames(7);
    assert_eq!(active(&d), [0]);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    assert_eq!(active(&d), [2]);
    d.press(Key::ArrowUp).frames(7);
    assert_eq!(active(&d), [1]);
}

#[test]
fn up_stops_at_the_first_row_and_end_goes_to_the_last_one_unbuilt_before() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowUp).frames(7);
    assert_eq!(active(&d), [0], "no wrap");
    assert!(
        !d.app().built.borrow().contains(&Row(999)),
        "precondition: the last row is not built"
    );
    d.press(Key::End).frames(7);
    assert_eq!(active(&d), [999]);
    assert!(d.app().built.borrow().contains(&Row(999)));
    d.press(Key::Home).frames(7);
    assert_eq!(active(&d), [0]);
}

#[test]
fn the_highlighted_row_is_scrolled_into_view() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::End).frames(7);
    let list = d.rect(Target::suffix(LIST));
    let row = row_rect(&d, 999);
    assert!(list.contains_rect(row.shrink(0.5)), "{row:?} in {list:?}");
    for _ in 0..15 {
        d.press(Key::ArrowUp).frames(7);
    }
    assert_eq!(active(&d), [984]);
    let list = d.rect(Target::suffix(LIST));
    let row = row_rect(&d, 984);
    assert!(list.contains_rect(row.shrink(0.5)), "{row:?} in {list:?}");
}

#[test]
fn a_page_moves_a_screenful_of_rows() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::PageDown).frames(7);
    assert_eq!(active(&d), [10], "200 points of 20 point rows");
    d.press(Key::PageUp).frames(7);
    assert_eq!(active(&d), [0]);
}

#[test]
fn enter_and_space_activate_the_highlighted_row() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::End).frames(7);
    d.press(Key::Enter).frames(7);
    assert_eq!(d.app().activated, [Row(999)]);
    d.press(Key::ArrowUp).frames(7);
    d.press(Key::Space).frames(7);
    assert_eq!(d.app().activated, [Row(999), Row(998)]);
}

#[test]
fn enter_with_nothing_highlighted_activates_nothing() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::Enter).frames(7);
    assert!(d.app().activated.is_empty());
}

#[test]
fn a_click_activates_and_highlights_its_row() {
    let mut d = harness();
    d.click(Target::suffix(&row_path(3))).frames(6);
    assert_eq!(d.app().activated, [Row(3)]);
    assert_eq!(active(&d), [3]);
}

#[test]
fn only_the_highlighted_row_takes_the_hover_look() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    let lit = |d: &Driver<App>, row: u32| support::fill_of(d, &row_path(row)) == Some(HIGHLIGHT);
    assert!(lit(&d, 1));
    assert!(!lit(&d, 0));
    assert!(!lit(&d, 2));
}

#[test]
fn the_pointer_moves_the_highlight_so_two_rows_never_look_hovered() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::ArrowDown).frames(7);
    let from: Pos2 = row_rect(&d, 2).center();
    d.hover(from);
    let at: Pos2 = row_rect(&d, 4).center();
    d.hover(at).frames(6);
    assert_eq!(active(&d), [4]);
    let lit: Vec<u32> = (0..10)
        .filter(|&row| support::fill_of(&d, &row_path(row)) == Some(HIGHLIGHT))
        .collect();
    assert_eq!(lit, [4]);
}

#[test]
fn the_highlighted_row_stays_built_when_the_list_is_scrolled_away() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::End).frames(7);
    d.shell()
        .scroll_to_row(support::id_of(&d, LIST), Row(0), None);
    d.frames(8);
    assert!(d.app().built.borrow().contains(&Row(999)));
    assert_eq!(active(&d), [999]);
}

#[test]
fn the_list_announces_the_highlighted_row_as_its_active_descendant() {
    let mut d = harness();
    focus_list(&mut d);
    d.press(Key::ArrowDown).frames(7);
    d.press(Key::ArrowDown).frames(7);
    let list = d.harness().get_by_role(egui::accesskit::Role::ListBox);
    let row = support::id_of(&d, &row_path(1));
    let announced = list
        .accesskit_node()
        .active_descendant()
        .map(|node| (u128::from(node.id()) >> 64) as u64);
    assert_eq!(announced, Some(row.accesskit_id().0));
}

#[test]
fn a_list_that_does_not_navigate_leaves_its_rows_alone() {
    let mut d = harness();
    d.app_mut().listbox = false;
    d.frames(4);
    d.press(Key::ArrowDown).frames(7);
    assert!(active(&d).is_empty());
}

#[test]
fn a_still_pointer_over_another_row_does_not_light_it_while_keys_move_the_highlight() {
    let mut d = harness();
    focus_list(&mut d);
    let from: Pos2 = row_rect(&d, 5).center();
    d.hover(from);
    let over: Pos2 = row_rect(&d, 6).center();
    d.hover(over);
    assert_eq!(active(&d), [6], "precondition: the pointer set it");
    d.press(Key::ArrowUp).frames(7);
    d.press(Key::ArrowUp).frames(7);
    assert_eq!(active(&d), [4]);
    let lit: Vec<u32> = (0..10)
        .filter(|&row| support::fill_of(&d, &row_path(row)) == Some(HIGHLIGHT))
        .collect();
    assert_eq!(lit, [4], "the row under the pointer is not lit as well");
}
