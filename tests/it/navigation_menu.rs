//! The navigation menu (#380): its panels on the row's hover intent, the
//! row's Tab and arrow order, and what assistive technology is told, through
//! real events on egui's clock.

use core::time::Duration;
use eggshell_ui::prelude::*;

use eggshell_ui::testing::{Driver, Focused, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::accesskit::{AriaCurrent, Role};
use egui::{Event, Key, KeyboardShortcut, Modifiers, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

const OPEN: Duration = Duration::from_millis(200);
const SKIP: Duration = Duration::from_millis(300);
const CLOSE: Duration = Duration::from_millis(150);
/// A frame: short, so the delays are measured to within a few frames.
const STEP: Duration = Duration::from_millis(4);

#[derive(Default)]
struct App {
    page: &'static str,
    quick: bool,
}

fn still(ctx: &egui::Context, _: &App) {
    ctx.global_style_mut(|style| style.animation_time = 0.0);
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let page = app.page;
    b.div(tw().flex().flex_col().gap(4.0).p(4.0).items_start(), |b| {
        b.button("before", "Before").on_click(|_: &mut App| {});
        let menu = b
            .navigation_menu("site")
            .named("Main")
            .panel("start", "Start", move |p| {
                p.div(tw().grid().grid_cols(2).gap(2.0).w_px(400.0), |g| {
                    for title in ["Intro", "Install", "Usage"] {
                        g.nav_link(title, title)
                            .description(format!("About {title}"))
                            .current(page == title)
                            .on_click(move |app: &mut App| app.page = title);
                    }
                });
            })
            .panel("more", "More", |p| {
                p.nav_link("Blog", "Blog")
                    .on_click(|app: &mut App| app.page = "Blog");
            })
            .link("docs", "Docs", move |link| {
                link.current(page == "Docs")
                    .on_click(|app: &mut App| app.page = "Docs");
            });
        if app.quick {
            menu.open_delay(Duration::from_millis(50)).add();
        } else {
            menu.add();
        }
        b.button("after", "After").on_click(|_: &mut App| {});
    });
}

fn harness_with(app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::prepared_driver(
        vec2(700.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
        still,
    )
    .step(STEP)
    .build();
    d.frames(4);
    d
}

fn harness() -> Driver<App> {
    harness_with(App::default())
}

fn trigger(item: &str) -> Target {
    Target::suffix(&format!("/\"{item}\"/Trigger"))
}

fn panel(item: &str) -> Target {
    Target::suffix(&format!("/\"{item}\"/Panel/Content"))
}

fn link(item: &str, title: &str) -> Target {
    Target::suffix(&format!("/\"{item}\"/Panel/Content/\"{title}\""))
}

const DOCS: &str = "/\"docs\"/\"Link\"";

/// Whether `item`'s panel is open: its trigger says so, as a leaving panel's
/// does not.
fn is_open(d: &Driver<App>, item: &str) -> bool {
    let label = match item {
        "start" => "Start",
        "more" => "More",
        other => panic!("no trigger {other}"),
    };
    let expanded = d
        .harness()
        .get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .data()
        .is_expanded();
    if expanded == Some(true) {
        assert!(d.exists(panel(item)), "an open trigger's panel is shown");
    }
    expanded == Some(true)
}

/// Rests the pointer on `at`, and returns how long that took.
fn rest(d: &mut Driver<App>, at: Target) -> Duration {
    let from = d.now();
    d.hover(at);
    d.now() - from
}

fn focused(d: &Driver<App>) -> String {
    match d.focused() {
        Some(Focused::Node(path)) => path,
        other => format!("{other:?}"),
    }
}

fn press(d: &mut Driver<App>, key: Key) {
    d.press(key).frames(3);
}

fn shift_tab(d: &mut Driver<App>) {
    d.press(KeyboardShortcut::new(Modifiers::SHIFT, Key::Tab))
        .frames(3);
}

fn open_by_pointer(d: &mut Driver<App>, item: &str) {
    d.hover(trigger(item))
        .advance(OPEN + Duration::from_millis(20));
    assert!(
        is_open(d, item),
        "precondition: {item} opened by the pointer"
    );
}

/// Focuses `item`'s trigger and opens it with Enter.
fn open_by_keys(d: &mut Driver<App>, item: &str) {
    d.focus(trigger(item)).frames(2);
    press(d, Key::Enter);
    assert!(is_open(d, item), "precondition: Enter opened {item}");
}

#[test]
fn the_style_holds_radixs_delays() {
    let d = harness();
    let style = d.shell().style().navigation_menu.clone();
    assert_eq!(style.open_delay, OPEN, "delayDuration");
    assert_eq!(style.skip_delay, SKIP, "skipDelayDuration");
    assert_eq!(style.close_delay, CLOSE, "Radix's close timer");
}

#[test]
fn resting_on_a_trigger_opens_its_panel_after_the_delay_below_it() {
    let mut d = harness();
    let spent = rest(&mut d, trigger("start"));
    assert!(
        spent < OPEN - Duration::from_millis(40),
        "precondition: {spent:?}"
    );
    d.advance(OPEN - spent - Duration::from_millis(30));
    assert!(!is_open(&d, "start"), "waits for 200 ms");
    d.advance(Duration::from_millis(50));
    let box_ = d.find(panel("start")).expect("open");
    let under = d.rect(trigger("start"));
    assert!(
        (box_.min.y - under.max.y - 6.0).abs() < 0.5,
        "6 pt below: {box_:?} {under:?}"
    );
    assert!(
        (box_.min.x - under.min.x).abs() < 0.5,
        "starts at the trigger's left edge: {box_:?} {under:?}"
    );
}

#[test]
fn a_menus_own_open_delay_overrides_the_style() {
    let mut d = harness_with(App {
        quick: true,
        ..App::default()
    });
    d.hover(trigger("start")).advance(Duration::from_millis(70));
    assert!(is_open(&d, "start"));
}

#[test]
fn while_one_is_open_the_pointer_on_a_neighbour_switches_at_once() {
    let mut d = harness();
    open_by_pointer(&mut d, "start");
    d.hover(trigger("more")).frames(2);
    assert!(is_open(&d, "more"), "no delay while one is open");
    assert!(!is_open(&d, "start"), "one at a time");
}

#[test]
fn leaving_closes_after_the_close_delay_and_the_skip_delay_follows_it() {
    let mut d = harness();
    open_by_pointer(&mut d, "start");
    let spent = rest(&mut d, Target::suffix("/\"before\""));
    assert!(
        spent < CLOSE - Duration::from_millis(60),
        "precondition: {spent:?}"
    );
    d.advance(CLOSE - spent - Duration::from_millis(40));
    assert!(is_open(&d, "start"), "still open within 150 ms");
    d.advance(Duration::from_millis(80));
    assert!(!is_open(&d, "start"), "closed after 150 ms");
    d.hover(trigger("more")).frames(2);
    assert!(
        is_open(&d, "more"),
        "within 300 ms of the close it opens at once"
    );
    d.hover(Target::suffix("/\"before\""))
        .advance(CLOSE + SKIP + Duration::from_millis(100));
    assert!(
        !is_open(&d, "more"),
        "precondition: closed, the skip delay gone"
    );
    d.hover(trigger("start")).frames(2);
    assert!(!is_open(&d, "start"), "after the skip delay it waits again");
}

#[test]
fn the_pointer_may_cross_into_the_panel_and_press_a_link_which_closes_it() {
    let mut d = harness();
    open_by_pointer(&mut d, "start");
    d.hover(link("start", "Install")).advance(CLOSE * 3);
    assert!(is_open(&d, "start"), "the panel holds it open");
    d.click(link("start", "Install")).frames(4);
    assert_eq!(d.app().page, "Install");
    assert!(!is_open(&d, "start"), "a link pressed in a panel closes it");
}

#[test]
fn a_click_opens_and_keeps_it_open_and_a_second_click_closes_it() {
    let mut d = harness();
    d.click(trigger("start")).frames(3);
    assert!(is_open(&d, "start"), "the click opens it at once");
    d.hover(Target::suffix("/\"before\"")).advance(CLOSE * 4);
    assert!(is_open(&d, "start"), "a pressed panel stays when left");
    d.click(trigger("start")).frames(3);
    assert!(!is_open(&d, "start"), "the second click closes it");
    d.advance(OPEN * 3);
    assert!(
        !is_open(&d, "start"),
        "the pointer resting on it does not reopen it"
    );
}

#[test]
fn a_click_on_a_hover_opened_trigger_closes_it() {
    let mut d = harness();
    open_by_pointer(&mut d, "start");
    d.click(trigger("start")).frames(3);
    assert!(!is_open(&d, "start"), "as Radix: the click toggles");
}

#[test]
fn a_press_outside_closes_a_pressed_panel() {
    let mut d = harness();
    d.click(trigger("start")).frames(3);
    assert!(is_open(&d, "start"), "precondition: open");
    d.click(Target::suffix("/\"before\"")).frames(3);
    assert!(!is_open(&d, "start"));
}

/// A tap: the touch, then the pointer events it makes.
fn tap(d: &mut Driver<App>, at: Target) {
    let point = d.rect(at).center();
    d.event(Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: point,
        force: None,
    });
    d.click(point).frames(3);
}

#[test]
fn a_tap_toggles_a_panel_and_resting_after_it_opens_nothing() {
    let mut d = harness();
    tap(&mut d, trigger("more"));
    assert!(is_open(&d, "more"), "the tap opens it");
    tap(&mut d, trigger("more"));
    assert!(!is_open(&d, "more"), "a second tap closes it");
    d.advance(OPEN * 3);
    assert!(!is_open(&d, "more"), "a touch never opens it by resting");
}

#[test]
fn every_trigger_and_link_is_a_tab_stop() {
    let mut d = harness();
    d.focus(Target::suffix("/\"before\"")).frames(2);
    let mut order = Vec::new();
    for _ in 0..4 {
        press(&mut d, Key::Tab);
        order.push(focused(&d));
    }
    assert!(order[0].ends_with("/\"start\"/Trigger"), "{order:?}");
    assert!(order[1].ends_with("/\"more\"/Trigger"), "{order:?}");
    assert!(order[2].ends_with(DOCS), "{order:?}");
    assert!(order[3].ends_with("/\"after\""), "{order:?}");
    assert!(!is_open(&d, "start"), "focus does not open a panel");
}

#[test]
fn the_arrows_move_along_the_row_and_home_and_end_go_to_its_ends() {
    let mut d = harness();
    d.focus(trigger("start")).frames(2);
    press(&mut d, Key::ArrowRight);
    assert!(
        focused(&d).ends_with("/\"more\"/Trigger"),
        "{}",
        focused(&d)
    );
    press(&mut d, Key::ArrowRight);
    assert!(focused(&d).ends_with(DOCS), "{}", focused(&d));
    press(&mut d, Key::Home);
    assert!(
        focused(&d).ends_with("/\"start\"/Trigger"),
        "{}",
        focused(&d)
    );
    press(&mut d, Key::End);
    assert!(focused(&d).ends_with(DOCS), "{}", focused(&d));
    press(&mut d, Key::ArrowLeft);
    assert!(
        focused(&d).ends_with("/\"more\"/Trigger"),
        "{}",
        focused(&d)
    );
}

#[test]
fn tab_goes_into_the_open_panel_and_out_past_it_to_the_next_item() {
    let mut d = harness();
    open_by_keys(&mut d, "start");
    press(&mut d, Key::Tab);
    assert!(
        focused(&d).ends_with("/Panel/Content/\"Intro\""),
        "into the panel: {}",
        focused(&d)
    );
    shift_tab(&mut d);
    assert!(
        focused(&d).ends_with("/\"start\"/Trigger"),
        "Shift+Tab from its first link goes back: {}",
        focused(&d)
    );
    press(&mut d, Key::Tab);
    press(&mut d, Key::Tab);
    press(&mut d, Key::Tab);
    assert!(
        focused(&d).ends_with("/Panel/Content/\"Usage\""),
        "precondition: on the last link: {}",
        focused(&d)
    );
    press(&mut d, Key::Tab);
    assert!(
        focused(&d).ends_with("/\"more\"/Trigger"),
        "past the panel to the next item: {}",
        focused(&d)
    );
    assert!(!is_open(&d, "start"), "and the panel closes");
    assert!(!is_open(&d, "more"), "Tab opens nothing");
}

#[test]
fn down_opens_the_panel_and_focuses_its_first_link_and_the_arrows_move_in_it() {
    let mut d = harness();
    d.focus(trigger("start")).frames(2);
    press(&mut d, Key::ArrowDown);
    d.frames(2);
    assert!(is_open(&d, "start"));
    assert!(
        focused(&d).ends_with("/Panel/Content/\"Intro\""),
        "{}",
        focused(&d)
    );
    press(&mut d, Key::ArrowRight);
    assert!(
        focused(&d).ends_with("/Panel/Content/\"Install\""),
        "the grid's arrows: {}",
        focused(&d)
    );
    press(&mut d, Key::ArrowDown);
    assert!(
        focused(&d).ends_with("/Panel/Content/\"Usage\""),
        "{}",
        focused(&d)
    );
}

#[test]
fn esc_closes_the_panel_and_gives_the_focus_back_to_its_trigger() {
    for inside in [false, true] {
        let mut d = harness();
        open_by_keys(&mut d, "start");
        if inside {
            press(&mut d, Key::Tab);
            assert!(
                focused(&d).contains("/Panel/"),
                "precondition: in the panel: {}",
                focused(&d)
            );
        }
        press(&mut d, Key::Escape);
        d.frames(2);
        assert!(!is_open(&d, "start"), "inside: {inside}");
        assert!(
            focused(&d).ends_with("/\"start\"/Trigger"),
            "inside: {inside}: {}",
            focused(&d)
        );
    }
}

#[test]
fn enter_toggles_and_the_arrows_switch_the_open_panel_along_the_row() {
    let mut d = harness();
    open_by_keys(&mut d, "start");
    press(&mut d, Key::ArrowRight);
    assert!(
        focused(&d).ends_with("/\"more\"/Trigger"),
        "{}",
        focused(&d)
    );
    assert!(is_open(&d, "more"), "the neighbour's opens");
    assert!(!is_open(&d, "start"), "one at a time");
    press(&mut d, Key::ArrowRight);
    assert!(focused(&d).ends_with(DOCS), "{}", focused(&d));
    assert!(!is_open(&d, "more"), "a link has no panel: it closes");
    press(&mut d, Key::ArrowLeft);
    assert!(
        !is_open(&d, "more"),
        "with none open, arriving opens nothing"
    );
    press(&mut d, Key::Enter);
    assert!(is_open(&d, "more"));
    press(&mut d, Key::Enter);
    assert!(!is_open(&d, "more"), "Enter toggles");
}

#[test]
fn the_menu_is_a_named_navigation_list_of_items() {
    let d = harness();
    d.harness().get_by_role_and_label(Role::Navigation, "Main");
    assert_eq!(d.harness().query_all_by_role(Role::List).count(), 1);
    assert_eq!(d.harness().query_all_by_role(Role::ListItem).count(), 3);
    for name in ["Start", "More"] {
        d.harness().get_by_role_and_label(Role::Button, name);
    }
    d.harness().get_by_role_and_label(Role::Link, "Docs");
}

/// The id of the node whose path ends in `suffix`.
fn id_at(d: &Driver<App>, suffix: &str) -> egui::Id {
    let path = support::nodes(d)
        .into_iter()
        .map(|(path, _)| path)
        .find(|path| path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no node ends in {suffix}"));
    support::id_of(d, &path)
}

#[test]
fn a_trigger_is_an_expanded_button_controlling_its_open_panel() {
    let mut d = harness();
    let controls = |d: &Driver<App>| {
        let start = d.harness().get_by_role_and_label(Role::Button, "Start");
        let node = start.accesskit_node();
        (node.data().is_expanded(), node.data().controls().to_vec())
    };
    assert_eq!(controls(&d), (Some(false), vec![]), "no panel, no relation");
    open_by_pointer(&mut d, "start");
    let content = id_at(&d, "/\"start\"/Panel/Content");
    assert_eq!(
        controls(&d),
        (Some(true), vec![content.accesskit_id()]),
        "aria-controls names the panel"
    );
}

#[test]
fn the_current_link_is_announced_as_the_current_page() {
    let mut d = harness_with(App {
        page: "Docs",
        ..App::default()
    });
    let docs = d.harness().get_by_role_and_label(Role::Link, "Docs");
    assert_eq!(
        docs.accesskit_node().data().aria_current(),
        Some(AriaCurrent::Page)
    );
    d.app_mut().page = "Install";
    d.frames(2);
    let docs = d.harness().get_by_role_and_label(Role::Link, "Docs");
    assert_eq!(docs.accesskit_node().data().aria_current(), None);
    open_by_pointer(&mut d, "start");
    let install = d.harness().get_by_role_and_label(Role::Link, "Install");
    assert_eq!(
        install.accesskit_node().data().aria_current(),
        Some(AriaCurrent::Page)
    );
    assert_eq!(
        install.accesskit_node().description().as_deref(),
        Some("About Install"),
        "the description is the link's"
    );
}

#[test]
fn shift_tab_from_the_item_after_an_open_trigger_goes_to_its_panels_last_link() {
    let mut d = harness();
    open_by_pointer(&mut d, "more");
    d.focus(Target::suffix(DOCS)).frames(2);
    assert!(is_open(&d, "more"), "precondition: still open");
    shift_tab(&mut d);
    assert!(
        focused(&d).ends_with("/\"more\"/Panel/Content/\"Blog\""),
        "{}",
        focused(&d)
    );
    assert!(is_open(&d, "more"));
}

/// Where `key` from `from` puts the focus, with the "start" panel opened
/// by the pointer or not.
fn tab_from(from: &str, key: KeyboardShortcut, opened: bool) -> String {
    let mut d = harness();
    if opened {
        open_by_pointer(&mut d, "start");
    }
    d.focus(Target::suffix(from)).frames(2);
    assert_eq!(is_open(&d, "start"), opened, "precondition");
    d.press(key).frames(5);
    assert_eq!(is_open(&d, "start"), opened, "the pointer holds it");
    focused(&d)
}

#[test]
fn tab_round_the_page_passes_a_hover_opened_panel_by() {
    let tab = KeyboardShortcut::new(Modifiers::NONE, Key::Tab);
    let back = KeyboardShortcut::new(Modifiers::SHIFT, Key::Tab);
    for (from, key) in [("/\"after\"", tab), ("/\"before\"", back)] {
        let with = tab_from(from, key, true);
        assert!(!with.contains("/Panel/"), "{from}: {with}");
        assert_eq!(
            with,
            tab_from(from, key, false),
            "{from}: where it goes with no panel open"
        );
    }
}
