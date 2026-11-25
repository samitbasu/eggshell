//! A group of items on the page that the arrow keys move through, with one
//! Tab stop: `NodeHandle::nav` with `Nav::Row` or `Nav::Radio`, and
//! `NodeHandle::selected`, built from the public API alone, through kittest's
//! real key events.

use eggshell_ui::overlay::{Nav, Wrap};
use eggshell_ui::owned::Mark;
use eggshell_ui::testing::Driver;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Problem, TextStyle, tw};
use egui::{Key, KeyboardShortcut, Modifiers, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path};

const TABS: [&str; 4] = ["One", "Two", "Three", "Four"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Follows {
    Focus,
    Selection,
}

struct App {
    nav: Nav,
    follows: Follows,
    selected: usize,
    disabled: Option<usize>,
    clicks: usize,
}

impl App {
    fn new(nav: Nav, follows: Follows) -> Self {
        Self {
            nav,
            follows,
            selected: 1,
            disabled: None,
            clicks: 0,
        }
    }
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <button>Before</button>
///   <div role="tablist" aria-label="Pages" class="flex gap-2">
///     <div role="tab">One</div> <div role="tab" aria-selected="true">Two</div> …
///   </div>
///   <button>After</button>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.button("before", "Before").on_click(|_: &mut App| {});
        let list = b
            .div_id("tabs", tw().flex().gap(2.0).sense_hover(), |b| {
                for (index, label) in TABS.into_iter().enumerate() {
                    let classes = tw().sense_click().px(8.0).py(4.0);
                    let classes = if app.disabled == Some(index) {
                        classes.inert()
                    } else {
                        classes
                    };
                    b.div_id(index, classes, |b| b.text(tw(), label))
                        .role(Role::Tab)
                        .named(label)
                        .selected(Mark::from(app.selected == index))
                        .on_click(move |app: &mut App| {
                            app.selected = index;
                            app.clicks += 1;
                        });
                }
            })
            .role(Role::TabList)
            .named("Pages")
            .nav(app.nav);
        if app.follows == Follows::Selection {
            list.selection_follows_focus();
        }
        b.button("after", "After").on_click(|_: &mut App| {});
    });
}

fn driver(app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(8);
    d
}

fn row(follows: Follows) -> Driver<App> {
    driver(App::new(Nav::Row(Wrap::Around), follows))
}

fn tab(index: usize) -> String {
    format!("/\"tabs\"/{index}")
}

/// Tabs from the top of the page to the first tab of the list.
fn enter_list(d: &mut Driver<App>) {
    for _ in 0..8 {
        d.press(Key::Tab).frames(3);
        if focused_path(d).is_some_and(|path| path.starts_with("/\"tabs\"/")) {
            return;
        }
    }
    panic!("Tab never reached the list: {:?}", focused_path(d));
}

#[test]
fn the_list_is_one_tab_stop_on_the_selected_item() {
    let mut d = row(Follows::Focus);
    enter_list(&mut d);
    assert_eq!(
        focused_path(&d),
        Some(tab(1)),
        "Tab enters on the selected tab"
    );
    let mut visited = Vec::new();
    for _ in 0..8 {
        d.press(Key::Tab).frames(3);
        visited.extend(focused_path(&d));
    }
    assert!(
        visited.iter().any(|path| path.contains("after")),
        "precondition: Tab goes round the page: {visited:?}"
    );
    assert!(
        visited
            .iter()
            .filter(|path| path.starts_with("/\"tabs\"/"))
            .all(|path| *path == tab(1)),
        "no other tab is a Tab stop: {visited:?}"
    );
}

#[test]
fn with_nothing_selected_the_first_enabled_item_is_the_tab_stop() {
    let mut app = App::new(Nav::Row(Wrap::Around), Follows::Focus);
    app.selected = TABS.len();
    app.disabled = Some(0);
    let mut d = driver(app);
    enter_list(&mut d);
    assert_eq!(focused_path(&d), Some(tab(1)));
}

#[test]
fn left_and_right_move_the_focus_and_wrap_and_home_and_end_go_to_the_ends() {
    let mut d = row(Follows::Focus);
    enter_list(&mut d);
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(focused_path(&d), Some(tab(2)));
    d.press(Key::ArrowRight).frames(3);
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(focused_path(&d), Some(tab(0)), "Right wraps");
    d.press(Key::ArrowLeft).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)), "Left wraps");
    d.press(Key::Home).frames(3);
    assert_eq!(focused_path(&d), Some(tab(0)));
    d.press(Key::End).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)));
    assert_eq!(d.app().selected, 1, "the focus alone moves");
}

#[test]
fn up_and_down_do_nothing_in_a_row() {
    let mut d = row(Follows::Focus);
    enter_list(&mut d);
    d.press(Key::ArrowDown).frames(3);
    assert_eq!(focused_path(&d), Some(tab(1)));
    d.press(Key::ArrowUp).frames(3);
    assert_eq!(focused_path(&d), Some(tab(1)));
}

#[test]
fn a_row_that_stops_stays_on_its_ends() {
    let mut d = driver(App::new(Nav::Row(Wrap::Stop), Follows::Focus));
    enter_list(&mut d);
    d.press(Key::ArrowLeft).frames(3);
    d.press(Key::ArrowLeft).frames(3);
    assert_eq!(focused_path(&d), Some(tab(0)));
    d.press(Key::End).frames(3);
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)));
}

#[test]
fn the_arrows_skip_a_disabled_item() {
    let mut app = App::new(Nav::Row(Wrap::Around), Follows::Focus);
    app.disabled = Some(2);
    let mut d = driver(app);
    enter_list(&mut d);
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)), "Three is skipped");
    d.press(Key::ArrowLeft).frames(3);
    assert_eq!(focused_path(&d), Some(tab(1)), "backwards too");
}

#[test]
fn tab_leaves_from_the_focused_item_and_comes_back_to_the_selected_one() {
    let mut d = row(Follows::Focus);
    enter_list(&mut d);
    d.press(Key::ArrowRight).frames(3);
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)), "precondition");
    d.press(Key::Tab).frames(3);
    assert!(
        focused_path(&d).is_some_and(|path| path.contains("after")),
        "Tab leaves the list: {:?}",
        focused_path(&d)
    );
    d.press(KeyboardShortcut::new(Modifiers::SHIFT, Key::Tab))
        .frames(3);
    assert_eq!(
        focused_path(&d),
        Some(tab(1)),
        "Shift+Tab comes back to the selected tab"
    );
}

#[test]
fn enter_and_space_activate_the_focused_item() {
    let mut d = row(Follows::Focus);
    enter_list(&mut d);
    d.press(Key::ArrowRight).frames(3);
    d.press(Key::Enter).frames(3);
    assert_eq!(d.app().selected, 2);
    d.press(Key::ArrowRight).frames(3);
    d.press(Key::Space).frames(3);
    assert_eq!(d.app().selected, 3);
}

#[test]
fn when_selection_follows_focus_an_arrow_activates_the_item_it_reaches() {
    let mut d = row(Follows::Selection);
    enter_list(&mut d);
    assert_eq!(d.app().clicks, 0, "entering activates nothing");
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(d.app().selected, 2);
    assert_eq!(focused_path(&d), Some(tab(2)));
    d.press(Key::End).frames(3);
    assert_eq!(d.app().selected, 3);
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(d.app().selected, 0, "and wraps");
    assert_eq!(d.app().clicks, 3, "one activation a key");
}

#[test]
fn a_radio_nav_moves_on_all_four_arrows_and_leaves_home_and_end_alone() {
    let mut d = driver(App::new(Nav::Radio(Wrap::Around), Follows::Focus));
    enter_list(&mut d);
    d.press(Key::ArrowDown).frames(3);
    assert_eq!(focused_path(&d), Some(tab(2)));
    d.press(Key::ArrowRight).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)));
    d.press(Key::ArrowDown).frames(3);
    assert_eq!(focused_path(&d), Some(tab(0)), "wraps");
    d.press(Key::ArrowUp).frames(3);
    assert_eq!(focused_path(&d), Some(tab(3)));
    d.press(Key::ArrowLeft).frames(3);
    assert_eq!(focused_path(&d), Some(tab(2)));
    d.press(Key::Home).frames(3);
    assert_eq!(focused_path(&d), Some(tab(2)), "Home is not the group's");
    d.press(Key::End).frames(3);
    assert_eq!(focused_path(&d), Some(tab(2)), "nor is End");
}

#[test]
fn the_selected_tab_is_announced_as_selected() {
    let d = row(Follows::Focus);
    let selected = |label: &str| {
        d.harness()
            .get_by_role_and_label(Role::Tab, label)
            .accesskit_node()
            .is_selected()
    };
    assert_eq!(selected("Two"), Some(true));
    assert_eq!(selected("One"), Some(false));
    assert_eq!(
        d.harness()
            .get_by_role_and_label(Role::TabList, "Pages")
            .accesskit_node()
            .role(),
        Role::TabList
    );
}

#[test]
fn a_listbox_nav_on_a_page_node_is_reported_and_makes_no_group() {
    let mut d = driver(App::new(Nav::Listbox(Wrap::Around), Follows::Focus));
    d.expect_reported(Problem::NavNeedsOverlay);
    enter_list(&mut d);
    assert_eq!(focused_path(&d), Some(tab(0)), "every tab is a Tab stop");
    d.press(Key::Tab).frames(3);
    assert_eq!(focused_path(&d), Some(tab(1)));
}

/// A radio group inside a menu: the menu's Up and Down pass over the group's
/// circles, which are the group's items, not the menu's.
mod in_a_menu {
    use eggshell_ui::overlay::{Anchor, Placement};

    use super::*;

    #[derive(Default)]
    struct Menu {
        unit: usize,
    }

    fn item(s: &mut Builder<'_, '_, Menu>, key: &'static str) {
        s.div_id(key, tw().sense_click().px(8.0).py(4.0), |s| {
            s.text(tw(), key)
        })
        .role(Role::MenuItem)
        .named(key);
    }

    fn view(b: &mut Builder<'_, '_, Menu>, app: &Menu) {
        b.div_id("Page", tw().p(2.0), |s| {
            s.button("trigger", "Units").on_click(|_: &mut Menu| {});
            let anchor = s.id_of("trigger");
            s.overlay("menu")
                .anchor(Anchor::Node(anchor))
                .placement(Placement::below())
                .nav(Nav::Menu(Wrap::Around))
                .role(Role::Menu)
                .named("Units")
                .children(|s| {
                    s.div_id("Box", tw().flex().flex_col().p(1.0), |s| {
                        item(s, "first");
                        s.radio_group("unit", app.unit, [(0, "Metric"), (1, "Imperial")])
                            .on_change(|app: &mut Menu, unit| app.unit = unit);
                        item(s, "last");
                    });
                });
        });
    }

    #[test]
    fn the_menus_arrows_pass_over_a_group_inside_it() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(400.0, 300.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            Menu::default(),
            view,
        )
        .build();
        d.frames(12);
        let first = "/\"Page\"/\"menu\"/\"Box\"/\"first\"";
        let last = "/\"Page\"/\"menu\"/\"Box\"/\"last\"";
        for _ in 0..8 {
            if focused_path(&d).as_deref() == Some(first) {
                break;
            }
            d.press(Key::Tab).frames(3);
        }
        assert_eq!(focused_path(&d).as_deref(), Some(first), "precondition");
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_path(&d).as_deref(), Some(last));
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_path(&d).as_deref(), Some(first), "and wraps");
    }
}
