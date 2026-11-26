//! Dropdown menus linked into a bar (`Builder::menu_link`,
//! `menu::Chain::linked`) in a `Nav::Row` group, built from the public API
//! alone, through kittest's real pointer and key events.

use eggshell_ui::overlay::{Distance, Nav, Wrap};
use eggshell_ui::owned::Enablement;
use eggshell_ui::owned::menu::{Chain, MenuLink};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Color32, Key, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, hover_node, rect_of};

const FILE: &str = r#"/"Page"/"Bar"/"file""#;
const EDIT: &str = r#"/"Page"/"Bar"/"edit""#;
const VIEW: &str = r#"/"Page"/"Bar"/"view""#;
const HELP: &str = r#"/"Page"/"Bar"/"help""#;
const CANVAS: &str = r#"/"Page"/"canvas""#;

fn menu(trigger: &str) -> String {
    let (bar, key) = trigger.rsplit_once('/').unwrap_or_default();
    format!("{bar}/({key}, Menu)/Box")
}

fn item(trigger: &str, key: &str) -> String {
    format!("{}/\"{key}\"", menu(trigger))
}

fn sub() -> String {
    format!("{}/(\"recent\", Submenu)/Box", menu(FILE))
}

struct App {
    log: Vec<String>,
    view_disabled: bool,
    linked: bool,
    canvas_presses: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            view_disabled: false,
            linked: true,
            canvas_presses: 0,
        }
    }
}

fn say(entry: &'static str) -> impl FnOnce(&mut App) + 'static {
    move |app| app.log.push(entry.to_owned())
}

/// ```html
/// <div class="flex flex-col gap-2 p-2">
///   <div role="menubar" class="flex gap-1">
///     <button role="menuitem">File</button> <button role="menuitem">Edit</button> …
///   </div>
///   <div class="ml-40 w-44 h-24 bg-gray-300"></div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        "Page",
        tw().flex()
            .flex_col()
            .items_start()
            .gap(2.0)
            .p(2.0)
            .w_full()
            .h_full(),
        |s| {
            let link = app.linked.then(|| s.menu_link("bar"));
            s.div_id("Bar", tw().flex().gap(1.0).sense_hover(), |s| {
                dropdown(s, ("file", "File"), link).items(|m| {
                    m.item("new", "New").on_click(say("new"));
                    m.submenu("recent", "Open Recent", |m| {
                        m.item("one", "One").on_click(say("one"));
                    });
                    m.item("quit", "Quit").on_click(say("quit"));
                });
                dropdown(s, ("edit", "Edit"), link).items(|m| {
                    m.item("undo", "Undo").on_click(say("undo"));
                    m.item("redo", "Redo").on_click(say("redo"));
                });
                dropdown(s, ("view", "View"), link)
                    .enablement(if app.view_disabled {
                        Enablement::Disabled
                    } else {
                        Enablement::Enabled
                    })
                    .items(|m| m.item("zoom", "Zoom").on_click(say("zoom")));
                dropdown(s, ("help", "Help"), link).items(|m| {
                    m.item("about", "About").on_click(say("about"));
                });
                if s.menu_open_with(s.id_of("file")) {
                    s.div_id("file-open", tw(), |_| {});
                }
            })
            .role(Role::MenuBar)
            .named("Menu bar")
            .nav(Nav::Row(Wrap::Around));
            s.div_id(
                "canvas",
                tw().ml(40.0)
                    .mt(40.0)
                    .w_px(180.0)
                    .h_px(100.0)
                    .bg(Color32::LIGHT_GRAY)
                    .sense_click(),
                |_| {},
            )
            .on_click(|app: &mut App| app.canvas_presses += 1);
        },
    );
}

fn dropdown<'a, 'f>(
    s: &'a mut Builder<'_, 'f, App>,
    (key, face): (&'static str, &'static str),
    link: Option<MenuLink>,
) -> Chain<'a, 'f, App, &'static str> {
    let chain = s.dropdown_menu(key, face);
    match link {
        Some(link) => chain.linked(link),
        None => chain,
    }
}

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(520.0, 360.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(6);
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(App::default())
}

fn settle(harness: &mut Harness<'_, Rig<App>>) {
    harness.run_steps(6);
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    settle(harness);
}

fn has(harness: &Harness<'_, Rig<App>>, path: &str) -> bool {
    harness
        .state()
        .nodes
        .iter()
        .any(|(candidate, _)| candidate == path)
}

/// The triggers whose menus are open.
fn open(harness: &Harness<'_, Rig<App>>) -> Vec<&'static str> {
    [FILE, EDIT, VIEW, HELP]
        .into_iter()
        .filter(|trigger| has(harness, &menu(trigger)))
        .collect()
}

fn open_with_click(harness: &mut Harness<'_, Rig<App>>, trigger: &str) {
    click_node(harness, trigger);
    settle(harness);
    assert_eq!(open(harness), [trigger], "precondition: {trigger} is open");
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..6 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        press(harness, Key::Tab);
    }
    panic!("could not tab to {path}: {:?}", focused_path(harness));
}

#[test]
fn a_linked_trigger_is_a_collapsed_menu_item() {
    let harness = harness();
    let bar = harness.get_by_role(Role::MenuBar);
    assert_eq!(bar.accesskit_node().label().as_deref(), Some("Menu bar"));
    let file = harness.get_by_role_and_label(Role::MenuItem, "File");
    assert_eq!(file.accesskit_node().data().is_expanded(), Some(false));
}

#[test]
fn a_click_opens_one_menu_and_focuses_its_first_item() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    assert_eq!(focused_path(&harness), Some(item(FILE, "new")));
    assert_eq!(
        harness
            .get_by_role_and_label(Role::MenuItem, "File")
            .accesskit_node()
            .data()
            .is_expanded(),
        Some(true)
    );
}

#[test]
fn hover_switches_menus_only_while_one_is_open() {
    let mut harness = harness();
    hover_node(&mut harness, EDIT);
    settle(&mut harness);
    assert!(
        open(&harness).is_empty(),
        "hover on a closed bar opens nothing"
    );
    open_with_click(&mut harness, FILE);
    hover_node(&mut harness, EDIT);
    settle(&mut harness);
    assert_eq!(open(&harness), [EDIT]);
    hover_node(&mut harness, HELP);
    settle(&mut harness);
    assert_eq!(open(&harness), [HELP]);
}

#[test]
fn hover_closes_the_old_menus_submenus() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    click_node(&mut harness, &item(FILE, "recent"));
    settle(&mut harness);
    assert!(has(&harness, &sub()), "precondition: the submenu is open");
    hover_node(&mut harness, EDIT);
    settle(&mut harness);
    assert_eq!(open(&harness), [EDIT]);
    hover_node(&mut harness, FILE);
    settle(&mut harness);
    assert_eq!(open(&harness), [FILE]);
    assert!(
        !has(&harness, &sub()),
        "File comes back with its submenu closed"
    );
}

#[test]
fn a_press_on_another_trigger_switches_to_its_menu() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    click_node(&mut harness, EDIT);
    settle(&mut harness);
    assert_eq!(
        open(&harness),
        [EDIT],
        "the hover and the press both say Edit"
    );
    click_node(&mut harness, EDIT);
    settle(&mut harness);
    assert!(open(&harness).is_empty(), "a second press closes it");
}

#[test]
fn a_press_without_a_hover_on_another_trigger_switches_to_its_menu() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    let edit = rect_of(&harness, EDIT).center();
    support::press_and_release(&mut harness, edit);
    settle(&mut harness);
    assert_eq!(open(&harness), [EDIT]);
}

#[test]
fn a_press_on_the_open_menus_own_trigger_closes_it() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    click_node(&mut harness, FILE);
    settle(&mut harness);
    assert!(open(&harness).is_empty());
}

#[test]
fn a_press_on_the_page_closes_the_menu_and_reaches_the_page() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    let canvas = rect_of(&harness, CANVAS);
    assert!(
        !rect_of(&harness, &menu(FILE)).intersects(canvas),
        "precondition: the press is outside the menu"
    );
    click_node(&mut harness, CANVAS);
    settle(&mut harness);
    assert!(open(&harness).is_empty());
    assert_eq!(harness.state().app.canvas_presses, 1);
}

#[test]
fn right_and_left_on_plain_items_open_the_neighbouring_menus_at_their_first_item() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [EDIT]);
    assert_eq!(focused_path(&harness), Some(item(EDIT, "undo")));
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(open(&harness), [FILE]);
    assert_eq!(focused_path(&harness), Some(item(FILE, "new")));
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(open(&harness), [HELP], "Left wraps from the first menu");
}

#[test]
fn right_on_an_item_with_a_submenu_opens_it_and_left_there_closes_only_it() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(focused_path(&harness), Some(item(FILE, "recent")));
    press(&mut harness, Key::ArrowRight);
    assert!(has(&harness, &sub()), "Right opens the submenu");
    assert_eq!(open(&harness), [FILE]);
    press(&mut harness, Key::ArrowLeft);
    assert!(!has(&harness, &sub()), "Left closes the submenu");
    assert_eq!(open(&harness), [FILE], "and only the submenu");
    assert_eq!(focused_path(&harness), Some(item(FILE, "recent")));
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(
        open(&harness),
        [HELP],
        "Left on the submenu's item: previous"
    );
}

#[test]
fn right_on_a_plain_item_of_a_submenu_closes_the_chain_and_opens_the_next_menu() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(
        focused_path(&harness),
        Some(format!("{}/\"one\"", sub())),
        "precondition: in the submenu"
    );
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [EDIT]);
    assert!(!has(&harness, &sub()));
    assert_eq!(focused_path(&harness), Some(item(EDIT, "undo")));
}

#[test]
fn the_arrows_skip_a_disabled_menu() {
    let mut harness = harness_with(App {
        view_disabled: true,
        ..App::default()
    });
    open_with_click(&mut harness, EDIT);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [HELP]);
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(open(&harness), [EDIT]);
    hover_node(&mut harness, VIEW);
    settle(&mut harness);
    assert_eq!(open(&harness), [EDIT], "nor does the pointer open it");
}

#[test]
fn escape_focuses_the_trigger_after_a_pointer_open_and_after_a_switch() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    press(&mut harness, Key::Escape);
    assert!(open(&harness).is_empty());
    assert_eq!(focused_path(&harness).as_deref(), Some(FILE));

    open_with_click(&mut harness, FILE);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [EDIT], "precondition: switched");
    press(&mut harness, Key::Escape);
    assert!(open(&harness).is_empty());
    assert_eq!(focused_path(&harness).as_deref(), Some(EDIT));
    press(&mut harness, Key::ArrowRight);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(VIEW),
        "the closed bar's arrows walk the triggers"
    );
    assert!(open(&harness).is_empty());

    open_with_click(&mut harness, FILE);
    hover_node(&mut harness, HELP);
    settle(&mut harness);
    press(&mut harness, Key::Escape);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(HELP),
        "after a hover"
    );
}

#[test]
fn activating_an_item_gives_the_focus_back_to_its_trigger() {
    let mut harness = harness();
    open_with_click(&mut harness, FILE);
    press(&mut harness, Key::ArrowRight);
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.log, ["undo"]);
    assert!(open(&harness).is_empty());
    assert_eq!(focused_path(&harness).as_deref(), Some(EDIT));
}

#[test]
fn the_closed_bar_is_one_tab_stop_and_up_on_a_trigger_opens_nothing() {
    let mut harness = harness();
    tab_to(&mut harness, FILE);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(focused_path(&harness).as_deref(), Some(EDIT));
    assert!(open(&harness).is_empty());
    press(&mut harness, Key::ArrowUp);
    assert!(open(&harness).is_empty(), "Up does nothing");
    press(&mut harness, Key::ArrowDown);
    assert_eq!(open(&harness), [EDIT], "Down opens");
    assert_eq!(focused_path(&harness), Some(item(EDIT, "undo")));
}

#[test]
fn unlinked_dropdowns_neither_switch_nor_take_the_side_arrows() {
    let mut harness = harness_with(App {
        linked: false,
        ..App::default()
    });
    click_node(&mut harness, FILE);
    settle(&mut harness);
    assert_eq!(open(&harness), [FILE]);
    assert_eq!(
        focused_path(&harness),
        None,
        "a pointer open focuses nothing"
    );
    hover_node(&mut harness, EDIT);
    settle(&mut harness);
    assert_eq!(open(&harness), [FILE], "the menu swallows the pointer");
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [FILE]);
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "File")
            .accesskit_node()
            .role(),
        Role::Button
    );
}

#[test]
fn the_builder_says_whether_a_menu_is_open_while_the_view_is_built() {
    const MARK: &str = r#"/"Page"/"Bar"/"file-open""#;
    let mut harness = harness();
    assert!(!has(&harness, MARK));
    open_with_click(&mut harness, FILE);
    assert!(has(&harness, MARK));
    press(&mut harness, Key::ArrowRight);
    assert!(!has(&harness, MARK));
}

#[test]
fn align_shift_moves_the_menu_along_the_triggers_edge() {
    fn shifted(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div_id("Page", tw().p(20.0), |s| {
            s.dropdown_menu("plain", "Plain")
                .items(|m| m.item("a", "A").on_click(|_| {}));
            s.dropdown_menu("shifted", "Shifted")
                .align_shift(Distance::new(-4.0))
                .items(|m| m.item("a", "A").on_click(|_| {}));
        });
    }
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        shifted,
    );
    harness.run_steps(4);
    for trigger in ["plain", "shifted"] {
        let path = format!(r#"/"Page"/"{trigger}""#);
        click_node(&mut harness, &path);
        settle(&mut harness);
        let box_path = format!(r#"/"Page"/("{trigger}", Menu)/Box"#);
        let gap = rect_of(&harness, &box_path).min.x - rect_of(&harness, &path).min.x;
        let want = if trigger == "plain" { 0.0 } else { -4.0 };
        assert!((gap - want).abs() < 0.01, "{trigger}: {gap}");
        press(&mut harness, Key::Escape);
    }
}
