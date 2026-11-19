//! The stock menus through the real path: kittest's pointer and key events,
//! build, layout, draw and the judgement of the frame's input.

use eggshell_ui::overlay::{Anchor, Nav, Placement, Wrap};
use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::{Builder, NodeUi as _, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::{HasPopup, Toggled};
use egui::{Color32, Event, FontFamily, Key, Modifiers, PointerButton, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, hover_node, rect_of};

const SCREEN: egui::Vec2 = vec2(500.0, 400.0);

const FILE: &str = r#"/"Page"/"Top"/"file""#;
const MENU: &str = r#"/"Page"/"Top"/("file", Menu)/Box"#;
const SUB: &str = r#"/"Page"/"Top"/("file", Menu)/Box/("more", Submenu)/Box"#;
const OTHER_SUB: &str = r#"/"Page"/"Top"/("file", Menu)/Box/("export", Submenu)/Box"#;
const CANVAS: &str = r#"/"Page"/"canvas""#;
const CONTEXT: &str = r#"/"Page"/ContextMenu("#;

#[derive(Default)]
struct App {
    log: Vec<String>,
    grid: bool,
    size: u8,
    disabled: bool,
    swatch: u8,
}

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn say(entry: &'static str) -> impl FnOnce(&mut App) + 'static {
    move |app| app.log.push(entry.to_owned())
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(2.0).p(2.0).w_full().h_full(),
        |s| {
            s.div_id("Top", tw().flex().gap(2.0), |s| {
                let (grid, size) = (app.grid, app.size);
                s.dropdown_menu("file", "File")
                    .enablement(if app.disabled {
                        Enablement::Disabled
                    } else {
                        Enablement::Enabled
                    })
                    .items(|m| {
                        m.item("copy", "Copy")
                            .shortcut("Ctrl+C")
                            .on_click(say("copy"));
                        m.item("paste", "Paste")
                            .enablement(Enablement::Disabled)
                            .on_click(say("paste"));
                        m.separator();
                        m.label("View");
                        m.checkbox_item("grid", "Show grid", Mark::from(grid))
                            .on_change(|app: &mut App, mark| app.grid = mark == Mark::On);
                        for each in 0..2u8 {
                            m.radio_item(
                                ("size", each),
                                if each == 0 { "Small" } else { "Large" },
                                Mark::from(each == size),
                            )
                            .on_click(move |app: &mut App| app.size = each);
                        }
                        m.submenu("more", "More", |m| {
                            m.item("deep", "Deep").on_click(say("deep"));
                            m.item("deeper", "Deeper").on_click(say("deeper"));
                        });
                        m.submenu("export", "Export", |m| {
                            m.item("pdf", "PDF").on_click(say("pdf"));
                        });
                        m.item("plain", "Plain").on_click(say("plain"));
                        m.separator();
                        m.item("delete", "Delete")
                            .destructive()
                            .on_click(say("delete"));
                    });
                s.button("other", "Other").add();
            });
            s.div_id(
                "canvas",
                tw().ml(60.0)
                    .w_px(180.0)
                    .h_px(100.0)
                    .bg(Color32::LIGHT_GRAY),
                |_| {},
            )
            .context_menu(|m| {
                m.item("cut", "Cut").on_click(say("cut"));
                m.submenu("more", "More", |m| {
                    m.item("deep", "Deep").on_click(say("context deep"));
                });
            });
        },
    );
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(SCREEN, base_text(), App::default(), view);
    harness.run_steps(4);
    harness
}

fn settle(harness: &mut Harness<'_, Rig<App>>) {
    harness.run_steps(5);
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

fn item(menu: &str, key: &str) -> String {
    format!("{menu}/{key}")
}

fn open_file(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, FILE);
    settle(harness);
    assert!(has(harness, MENU), "precondition: the menu is open");
}

fn open_more(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, &item(MENU, r#""more""#));
    settle(harness);
    assert!(has(harness, SUB), "precondition: the submenu is open");
}

fn secondary_click(harness: &mut Harness<'_, Rig<App>>, at: egui::Pos2) {
    support::hover_over(harness, at);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    settle(harness);
}

#[test]
fn clicking_the_trigger_opens_the_menu_and_clicking_it_again_closes_it() {
    let mut harness = harness();
    assert!(!has(&harness, MENU));
    click_node(&mut harness, FILE);
    settle(&mut harness);
    assert!(has(&harness, MENU), "a click opens");
    click_node(&mut harness, FILE);
    settle(&mut harness);
    assert!(!has(&harness, MENU), "a second click closes");
}

#[test]
fn the_trigger_says_whether_its_menu_is_open() {
    let mut harness = harness();
    let trigger = |harness: &Harness<'_, Rig<App>>| {
        let data = harness
            .get_by_role_and_label(Role::Button, "File")
            .accesskit_node()
            .data()
            .clone();
        (data.is_expanded(), data.has_popup())
    };
    assert_eq!(trigger(&harness), (Some(false), Some(HasPopup::Menu)));
    open_file(&mut harness);
    assert_eq!(trigger(&harness), (Some(true), Some(HasPopup::Menu)));
}

#[test]
fn items_have_the_menu_item_roles_and_the_checked_state() {
    let mut harness = harness();
    harness.state_mut().app.grid = true;
    harness.state_mut().app.size = 1;
    open_file(&mut harness);
    harness.get_by_role_and_label(Role::Menu, "File");
    harness.get_by_role_and_label(Role::MenuItem, "Copy");
    let toggled = |harness: &Harness<'_, Rig<App>>, role, label| {
        harness
            .get_by_role_and_label(role, label)
            .accesskit_node()
            .toggled()
    };
    assert_eq!(
        toggled(&harness, Role::MenuItemCheckBox, "Show grid"),
        Some(Toggled::True)
    );
    assert_eq!(
        toggled(&harness, Role::MenuItemRadio, "Large"),
        Some(Toggled::True)
    );
    assert_eq!(
        toggled(&harness, Role::MenuItemRadio, "Small"),
        Some(Toggled::False)
    );
    let more = harness
        .get_by_role_and_label(Role::MenuItem, "More")
        .accesskit_node()
        .data()
        .clone();
    assert_eq!(more.has_popup(), Some(HasPopup::Menu));
    assert_eq!(more.is_expanded(), Some(false));
}

#[test]
fn separators_have_no_node_and_the_arrows_pass_them() {
    let mut harness = harness();
    open_file(&mut harness);
    assert_eq!(
        harness.query_all_by_role(Role::Splitter).count(),
        0,
        "a separator is painted, not announced"
    );
    for _ in 0..10 {
        if focused_path(&harness) == Some(item(MENU, r#""plain""#)) {
            break;
        }
        press(&mut harness, Key::ArrowDown);
    }
    assert_eq!(
        focused_path(&harness),
        Some(item(MENU, r#""plain""#)),
        "precondition: the item before the separator is focused"
    );
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        focused_path(&harness),
        Some(item(MENU, r#""delete""#)),
        "Down passes the separator"
    );
    press(&mut harness, Key::ArrowUp);
    assert_eq!(focused_path(&harness), Some(item(MENU, r#""plain""#)));
}

#[test]
fn activating_an_item_runs_its_handler_and_closes_the_menu() {
    let mut harness = harness();
    open_file(&mut harness);
    click_node(&mut harness, &item(MENU, r#""copy""#));
    settle(&mut harness);
    assert_eq!(harness.state().app.log, ["copy"]);
    assert!(!has(&harness, MENU));
}

#[test]
fn checkbox_and_radio_items_report_their_new_value_and_close() {
    let mut harness = harness();
    open_file(&mut harness);
    click_node(&mut harness, &item(MENU, r#""grid""#));
    settle(&mut harness);
    assert!(harness.state().app.grid, "the checkbox moves to on");
    assert!(!has(&harness, MENU));
    open_file(&mut harness);
    click_node(&mut harness, &item(MENU, r#"("size", 1)"#));
    settle(&mut harness);
    assert_eq!(harness.state().app.size, 1);
    open_file(&mut harness);
    click_node(&mut harness, &item(MENU, r#""grid""#));
    settle(&mut harness);
    assert!(!harness.state().app.grid, "and back to off");
}

#[test]
fn a_disabled_item_takes_no_click_and_the_arrows_skip_it() {
    let mut harness = harness();
    open_file(&mut harness);
    click_node(&mut harness, &item(MENU, r#""paste""#));
    settle(&mut harness);
    assert!(harness.state().app.log.is_empty());
    assert!(has(&harness, MENU), "the menu stays open");
    press(&mut harness, Key::ArrowDown);
    assert_eq!(focused_path(&harness), Some(item(MENU, r#""copy""#)));
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        focused_path(&harness),
        Some(item(MENU, r#""grid""#)),
        "Paste is skipped"
    );
}

#[test]
fn activating_an_item_of_a_submenu_closes_the_whole_chain() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    click_node(&mut harness, &item(SUB, r#""deeper""#));
    settle(&mut harness);
    assert_eq!(harness.state().app.log, ["deeper"]);
    assert!(!has(&harness, MENU) && !has(&harness, SUB));
    open_file(&mut harness);
    assert!(!has(&harness, SUB), "it opens with its submenu closed");
}

#[test]
fn a_click_opens_the_submenu_beside_its_item_with_nothing_focused() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    let parent = rect_of(&harness, &item(MENU, r#""more""#));
    let sub = rect_of(&harness, SUB);
    assert!(sub.min.x > parent.center().x, "beside the item");
    assert_eq!(focused_path(&harness), None);
    assert_eq!(
        harness
            .get_by_role_and_label(Role::MenuItem, "More")
            .accesskit_node()
            .data()
            .is_expanded(),
        Some(true)
    );
}

#[test]
fn right_and_enter_open_a_submenu_and_focus_its_first_item() {
    for key in [Key::ArrowRight, Key::Enter] {
        let mut harness = harness();
        open_file(&mut harness);
        for _ in 0..6 {
            press(&mut harness, Key::ArrowDown);
            if focused_path(&harness) == Some(item(MENU, r#""more""#)) {
                break;
            }
        }
        assert_eq!(
            focused_path(&harness),
            Some(item(MENU, r#""more""#)),
            "precondition: the item is focused"
        );
        press(&mut harness, key);
        assert!(has(&harness, SUB), "{key:?} opens the submenu");
        assert_eq!(
            focused_path(&harness),
            Some(item(SUB, r#""deep""#)),
            "{key:?} focuses the first item"
        );
    }
}

#[test]
fn left_closes_the_submenu_and_focuses_its_item_but_escape_goes_one_level_at_a_time() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(focused_path(&harness), Some(item(SUB, r#""deep""#)));
    press(&mut harness, Key::ArrowLeft);
    assert!(!has(&harness, SUB) && has(&harness, MENU), "Left closes it");
    assert_eq!(focused_path(&harness), Some(item(MENU, r#""more""#)));

    press(&mut harness, Key::ArrowRight);
    assert!(has(&harness, SUB));
    press(&mut harness, Key::Escape);
    assert!(!has(&harness, SUB) && has(&harness, MENU), "Esc closes it");
    assert_eq!(focused_path(&harness), Some(item(MENU, r#""more""#)));
    press(&mut harness, Key::Escape);
    assert!(!has(&harness, MENU), "the next Esc closes the menu");
}

#[test]
fn with_a_submenu_open_the_pointer_on_a_sibling_submenu_item_switches_to_it() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    hover_node(&mut harness, &item(MENU, r#""export""#));
    settle(&mut harness);
    assert!(has(&harness, OTHER_SUB), "the Export submenu is open");
    assert!(!has(&harness, SUB), "and More is closed");
}

#[test]
fn with_a_submenu_open_the_pointer_on_a_plain_item_closes_it() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    hover_node(&mut harness, &item(MENU, r#""plain""#));
    settle(&mut harness);
    assert!(!has(&harness, SUB), "the submenu is closed");
    assert!(has(&harness, MENU), "the menu is not");
}

#[test]
fn hovering_a_submenu_item_does_not_open_it_from_closed() {
    let mut harness = harness();
    open_file(&mut harness);
    hover_node(&mut harness, &item(MENU, r#""more""#));
    settle(&mut harness);
    hover_node(&mut harness, &item(MENU, r#""export""#));
    settle(&mut harness);
    assert!(!has(&harness, SUB) && !has(&harness, OTHER_SUB));
}

#[test]
fn the_pointer_on_the_open_submenus_own_item_keeps_it_open() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    hover_node(&mut harness, &item(MENU, r#""more""#));
    settle(&mut harness);
    assert!(has(&harness, SUB));
}

#[test]
fn a_press_outside_closes_the_whole_chain_and_nothing_under_it_sees_it() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    let other = rect_of(&harness, r#"/"Page"/"Top"/"other""#).center();
    assert!(
        !rect_of(&harness, MENU).contains(other) && !rect_of(&harness, SUB).contains(other),
        "precondition: the press is outside both"
    );
    support::press_and_release(&mut harness, other);
    settle(&mut harness);
    assert!(!has(&harness, MENU) && !has(&harness, SUB));
}

#[test]
fn a_press_on_a_parent_item_with_a_submenu_open_reaches_the_item() {
    let mut harness = harness();
    open_file(&mut harness);
    open_more(&mut harness);
    click_node(&mut harness, &item(MENU, r#""copy""#));
    settle(&mut harness);
    assert_eq!(harness.state().app.log, ["copy"], "one click is enough");
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..8 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        harness.key_press(Key::Tab);
        settle(harness);
    }
    panic!("could not tab to {path}");
}

#[test]
fn enter_space_and_down_on_the_trigger_open_the_menu_and_focus_its_first_item() {
    for key in [Key::Enter, Key::Space, Key::ArrowDown] {
        let mut harness = harness();
        tab_to(&mut harness, FILE);
        press(&mut harness, key);
        assert!(has(&harness, MENU), "{key:?} opens");
        assert_eq!(
            focused_path(&harness),
            Some(item(MENU, r#""copy""#)),
            "{key:?} focuses the first item"
        );
    }
}

/// A menu opened from the keyboard focuses the item marked
/// `focus_on_open`, not its first: the current choice.
#[test]
fn a_menu_opens_on_the_item_marked_focus_on_open() {
    fn marked(b: &mut Builder<'_, '_, App>, app: &App) {
        let size = app.size;
        b.div_id("Page", tw().p(2.0), |s| {
            s.dropdown_menu("file", "File").items(|m| {
                for each in 0..3u8 {
                    let item = m.radio_item(
                        ("size", each),
                        format!("Size {each}"),
                        Mark::from(each == size),
                    );
                    let item = if each == size {
                        item.focus_on_open()
                    } else {
                        item
                    };
                    item.on_click(move |app: &mut App| app.size = each);
                }
            });
        });
    }
    let mut harness = support::harness(
        SCREEN,
        base_text(),
        App {
            size: 2,
            ..App::default()
        },
        marked,
    );
    harness.run_steps(4);
    tab_to(&mut harness, r#"/"Page"/"file""#);
    press(&mut harness, Key::Enter);
    assert_eq!(
        focused_path(&harness),
        Some(r#"/"Page"/("file", Menu)/Box/("size", 2)"#.to_owned())
    );
}

/// Down in the frame right after the menu gave its first item focus moves
/// once, as in a grid (#226).
#[test]
fn down_in_the_frame_after_the_first_item_gets_focus_moves_once() {
    let mut harness = harness();
    tab_to(&mut harness, FILE);
    harness.key_press(Key::Enter);
    let first = item(MENU, r#""copy""#);
    let mut frames = 0;
    while focused_path(&harness).as_deref() != Some(first.as_str()) {
        harness.step();
        frames += 1;
        assert!(frames < 10, "precondition: Enter focuses the first item");
    }
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        focused_path(&harness),
        Some(item(MENU, r#""grid""#)),
        "past the disabled Paste to Show grid, and no further"
    );
}

#[test]
fn escape_closes_the_menu_and_gives_the_focus_back_to_the_trigger() {
    let mut harness = harness();
    tab_to(&mut harness, FILE);
    press(&mut harness, Key::Enter);
    press(&mut harness, Key::Escape);
    assert!(!has(&harness, MENU));
    assert_eq!(focused_path(&harness).as_deref(), Some(FILE));
}

#[test]
fn a_trigger_that_becomes_disabled_closes_its_menu() {
    let mut harness = harness();
    open_file(&mut harness);
    harness.state_mut().app.disabled = true;
    settle(&mut harness);
    assert!(!has(&harness, MENU));
}

#[test]
fn a_secondary_click_opens_the_context_menu_at_the_pointer() {
    let mut harness = harness();
    let canvas = rect_of(&harness, CANVAS);
    let at = canvas.min + vec2(30.0, 20.0);
    secondary_click(&mut harness, at);
    let menu = harness
        .state()
        .nodes
        .iter()
        .find(|(path, _)| path.starts_with(CONTEXT) && path.ends_with("/Box"))
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("the context menu is open: {:?}", harness.state().nodes));
    assert!(
        (menu.min - at).length() < 1.5,
        "its corner is at the pointer: {menu:?} for {at:?}"
    );
    harness.get_by_role_and_label(Role::Menu, "Context menu");
}

#[test]
fn a_context_menu_item_runs_and_closes_the_menu_and_escape_closes_it_too() {
    let mut harness = harness();
    let canvas = rect_of(&harness, CANVAS);
    secondary_click(&mut harness, canvas.min + vec2(30.0, 20.0));
    harness.get_by_role_and_label(Role::MenuItem, "Cut").click();
    settle(&mut harness);
    assert_eq!(harness.state().app.log, ["cut"]);
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.starts_with(CONTEXT)),
        "closed"
    );

    secondary_click(&mut harness, canvas.min + vec2(30.0, 20.0));
    press(&mut harness, Key::Escape);
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.starts_with(CONTEXT)),
        "Esc closes"
    );
}

#[test]
fn a_context_menu_has_submenus_too() {
    let mut harness = harness();
    let canvas = rect_of(&harness, CANVAS);
    secondary_click(&mut harness, canvas.min + vec2(30.0, 20.0));
    harness
        .get_by_role_and_label(Role::MenuItem, "More")
        .click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::MenuItem, "Deep")
        .click();
    settle(&mut harness);
    assert_eq!(harness.state().app.log, ["context deep"]);
}

#[test]
fn a_secondary_click_on_a_node_while_a_menu_is_open_only_closes_the_menu() {
    let mut harness = harness();
    open_file(&mut harness);
    let canvas = rect_of(&harness, CANVAS);
    secondary_click(&mut harness, canvas.min + vec2(30.0, 20.0));
    assert!(!has(&harness, MENU), "the dropdown closed");
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.starts_with(CONTEXT)),
        "the node never saw the press"
    );
}

mod custom_item {
    use super::*;

    const SWATCHES: [&str; 3] = ["Red", "Green", "Blue"];

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id("Page", tw().p(2.0), |s| {
            s.button("trigger", "Colour")
                .opens_popup()
                .on_click(|app: &mut App| app.disabled = !app.disabled);
            if app.disabled {
                let anchor = s.id_of("trigger");
                s.overlay("swatches")
                    .anchor(Anchor::Node(anchor))
                    .placement(Placement::below())
                    .nav(Nav::Menu(Wrap::Around))
                    .role(Role::Menu)
                    .named("Colours")
                    .on_dismiss(|app: &mut App| app.disabled = false)
                    .children(|s| {
                        s.div_id("Box", tw().flex().gap(1.0).p(1.0), |s| {
                            for (index, name) in SWATCHES.into_iter().enumerate() {
                                s.custom(
                                    ("swatch", index),
                                    tw().sense_click().w_px(24.0).h_px(24.0),
                                )
                                .role(Role::MenuItemRadio)
                                .named(name)
                                .ui(move |app, ui| {
                                    let rect = ui.max_rect();
                                    let painter = ui.painter();
                                    painter.rect_filled(
                                        rect,
                                        4.0,
                                        Color32::from_rgb(index as u8 * 100, 50, 50),
                                    );
                                    if ui.node_response().clicked() {
                                        app.swatch = index as u8;
                                        app.disabled = false;
                                    }
                                });
                            }
                        });
                    });
            }
        });
    }

    fn harness() -> Harness<'static, Rig<App>> {
        let mut harness = support::harness(SCREEN, base_text(), App::default(), view);
        harness.run_steps(4);
        click_node(&mut harness, r#"/"Page"/"trigger""#);
        settle(&mut harness);
        harness
    }

    #[test]
    fn a_custom_painted_swatch_with_an_item_role_is_reached_by_the_arrows_and_activated() {
        let mut harness = harness();
        harness.get_by_role_and_label(Role::MenuItemRadio, "Green");
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::ArrowDown);
        let path = r#"/"Page"/("swatches")/"Box"/("swatch", 1)"#;
        let focused = focused_path(&harness).expect("a swatch has focus");
        assert!(focused.ends_with(r#"("swatch", 1)"#), "{focused} of {path}");
        press(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.swatch, 1);
    }
}

mod trigger {
    use eggshell_ui::owned::Face;
    use eggshell_ui::{Icon, Opacity, Style};

    use super::*;

    const DD: &str = r#"/"dd""#;
    const SEL: &str = r#"/"sel""#;

    fn icon() -> Icon {
        Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
            egui::TextureId::default(),
            vec2(4.0, 4.0),
        )))
    }

    fn chevron_and_select(b: &mut Builder<'_, '_, App>, _: &App) {
        b.dropdown_menu("dd", "File")
            .chevron()
            .items(|m| m.item("x", "X").on_click(say("x")));
        b.dropdown_menu("iconed", "Edit")
            .icon(icon())
            .items(|m| m.item("x", "X").on_click(say("x")));
        b.dropdown_menu("plain", "View")
            .items(|m| m.item("x", "X").on_click(say("x")));
        b.select("sel", 0u8, [(0u8, "Medium")])
            .named("Size")
            .on_change(|_, _| {});
    }

    fn opened(view: support::View<App>) -> Harness<'static, Rig<App>> {
        let mut harness = support::harness(SCREEN, base_text(), App::default(), view);
        harness.run_steps(4);
        harness
    }

    fn under(harness: &Harness<'_, Rig<App>>, parent: &str, part: &str) -> Option<egui::Rect> {
        let path = format!("{parent}/{part}");
        harness
            .state()
            .nodes
            .iter()
            .find(|(candidate, _)| *candidate == path)
            .map(|(_, rect)| *rect)
    }

    fn glyph(harness: &Harness<'_, Rig<App>>, parent: &str) -> egui::Rect {
        under(harness, parent, "<glyph>").expect("the trigger draws a chevron")
    }

    /// The most opaque vertex the chevron of the dropdown is drawn with.
    fn chevron_alpha(harness: &Harness<'_, Rig<App>>) -> u8 {
        let at = glyph(harness, DD);
        support::shapes(harness)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Mesh(mesh) if at.expand(0.5).contains_rect(mesh.calc_bounds()) => {
                    mesh.vertices.iter().map(|vertex| vertex.color.a()).max()
                }
                _ => None,
            })
            .max()
            .unwrap_or_default()
    }

    #[test]
    fn a_chevron_trails_the_label_at_the_triggers_gap_and_is_as_big_as_a_selects() {
        let harness = opened(chevron_and_select);
        let (label, chevron) = (
            under(&harness, DD, "<text>").expect("the trigger has its label"),
            glyph(&harness, DD),
        );
        let iconed = r#"/"iconed""#;
        let gap =
            under(&harness, iconed, "<text>").expect("label").min.x - glyph(&harness, iconed).max.x;
        assert!(gap > 0.0, "precondition: the trigger has a gap: {gap}");
        assert!(
            (chevron.min.x - label.max.x - gap).abs() < 0.01,
            "the chevron is {} after the label, the trigger's gap is {gap}",
            chevron.min.x - label.max.x
        );
        assert_eq!(chevron.size(), glyph(&harness, SEL).size());
        harness.get_by_role_and_label(Role::Button, "File");
    }

    #[test]
    fn the_chevron_takes_the_selects_size_and_tone_from_the_style() {
        let restyled = |chevron: f32, opacity: f32| {
            let mut harness = opened(chevron_and_select);
            let mut style = (*Style::of(&harness.ctx)).clone();
            style.select.chevron = chevron;
            style.select.chevron_opacity = Opacity::new(opacity);
            style.install(&harness.ctx);
            harness.run_steps(4);
            harness
        };
        let (faint, full) = (restyled(20.0, 0.25), restyled(20.0, 1.0));
        assert_eq!(glyph(&full, DD).size(), vec2(20.0, 20.0));
        assert_eq!(glyph(&full, SEL).size(), vec2(20.0, 20.0));
        let (faint, full) = (chevron_alpha(&faint), chevron_alpha(&full));
        assert!(full > 0, "precondition: the chevron is drawn");
        assert!(
            faint < full / 2,
            "a quarter opacity draws fainter: {faint} against {full}"
        );
    }

    #[test]
    fn a_trigger_has_no_chevron_unless_asked() {
        let harness = opened(chevron_and_select);
        assert!(
            under(&harness, r#"/"plain""#, "<text>").is_some(),
            "precondition: the trigger is there"
        );
        assert!(under(&harness, r#"/"plain""#, "<glyph>").is_none());
    }

    #[test]
    fn children_fill_a_trigger_whose_face_is_empty_and_it_is_named_by_the_face() {
        fn swatch(b: &mut Builder<'_, '_, App>, _: &App) {
            b.dropdown_menu("dd", Face::Empty("Accent".into()))
                .children(|s| {
                    s.div_id("dot", tw().size_px(12.0).bg(Color32::RED), |_| {});
                })
                .chevron()
                .items(|m| m.item("x", "X").on_click(say("x")));
        }
        let mut harness = opened(swatch);
        let trigger = rect_of(&harness, DD);
        let dot = under(&harness, DD, r#""dot""#).expect("the child is in the trigger");
        assert!(trigger.contains_rect(dot), "{dot:?} in {trigger:?}");
        assert!(
            dot.max.x <= glyph(&harness, DD).min.x,
            "the chevron follows the children"
        );
        harness
            .get_by_role_and_label(Role::Button, "Accent")
            .click();
        settle(&mut harness);
        harness.get_by_role_and_label(Role::Menu, "Accent");
    }

    #[test]
    fn menu_classes_reach_that_menus_box_alone() {
        fn wide(b: &mut Builder<'_, '_, App>, _: &App) {
            b.div_id("Top", tw().flex().gap(2.0), |s| {
                s.dropdown_menu("wide", "Wide")
                    .menu_classes(tw().min_w_px(300.0))
                    .items(|m| m.item("x", "X").on_click(say("x")));
                s.dropdown_menu("plain", "Plain")
                    .items(|m| m.item("x", "X").on_click(say("x")));
            });
        }
        let mut harness = opened(wide);
        let width = |harness: &mut Harness<'_, Rig<App>>, key: &str| {
            click_node(harness, &format!(r#"/"Top"/"{key}""#));
            settle(harness);
            let width = rect_of(harness, &format!(r#"/"Top"/("{key}", Menu)/Box"#)).width();
            press(harness, Key::Escape);
            settle(harness);
            width
        };
        assert_eq!(width(&mut harness, "wide"), 300.0);
        let plain = width(&mut harness, "plain");
        assert!(
            plain < 300.0,
            "the other menu keeps the style's width: {plain}"
        );
    }
}

mod open_query {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    /// Each frame's answer to `open` and whether a menu overlay was leaving.
    type Frames = Rc<RefCell<Vec<(bool, bool)>>>;

    fn observed<A: 'static>(
        harness: &mut Harness<'_, Rig<A>>,
        open: impl Fn(&eggshell_ui::Placed<A>) -> bool + 'static,
    ) -> Frames {
        let frames: Frames = Rc::default();
        let seen = Rc::clone(&frames);
        harness.state_mut().observe = Some(Box::new(move |placed| {
            let leaving = placed.overlays().iter().any(|overlay| overlay.is_exiting());
            seen.borrow_mut().push((open(placed), leaving));
        }));
        frames
    }

    fn last(frames: &Frames) -> bool {
        last_of(&frames.borrow())
    }

    fn last_of(frames: &[(bool, bool)]) -> bool {
        frames.last().is_some_and(|(open, _)| *open)
    }

    /// Escape closes the menu: it is not open on any frame it is still on
    /// screen for, fading out.
    fn closes_before_it_fades<A>(harness: &mut Harness<'_, Rig<A>>, frames: &Frames) {
        frames.borrow_mut().clear();
        harness.key_press(Key::Escape);
        harness.run_steps(8);
        let frames = frames.borrow();
        assert!(
            frames.iter().any(|(_, leaving)| *leaving),
            "precondition: the menu fades out over frames: {frames:?}"
        );
        assert!(
            frames
                .iter()
                .filter(|(_, leaving)| *leaving)
                .all(|(open, _)| !open),
            "open while fading: {frames:?}"
        );
        assert!(!last_of(&frames), "open after it closed: {frames:?}");
    }

    #[test]
    fn a_dropdowns_menu_is_open_from_the_click_and_closed_from_the_frame_it_closes() {
        let mut harness = harness();
        let trigger = support::id_of(&harness, FILE);
        let frames = observed(&mut harness, move |placed| placed.menu_open_with(trigger));
        settle(&mut harness);
        assert!(!last(&frames));
        open_file(&mut harness);
        assert!(last(&frames));
        closes_before_it_fades(&mut harness, &frames);
    }

    #[test]
    fn a_dropdown_at_the_top_of_the_view_is_found_by_its_key() {
        fn top(b: &mut Builder<'_, '_, App>, _: &App) {
            b.dropdown_menu("top", "Top")
                .items(|m| m.item("copy", "Copy").on_click(say("copy")));
        }
        let mut harness = support::harness(SCREEN, base_text(), App::default(), top);
        harness.run_steps(4);
        let frames = observed(&mut harness, |placed| placed.menu_open("top"));
        settle(&mut harness);
        assert!(!last(&frames));
        click_node(&mut harness, r#"/"top""#);
        settle(&mut harness);
        assert!(last(&frames));
        closes_before_it_fades(&mut harness, &frames);
    }

    #[test]
    fn a_context_menu_is_found_by_its_node() {
        let mut harness = harness();
        let canvas = support::id_of(&harness, CANVAS);
        let frames = observed(&mut harness, move |placed| placed.menu_open_with(canvas));
        settle(&mut harness);
        assert!(!last(&frames));
        let at = rect_of(&harness, CANVAS).min + vec2(30.0, 20.0);
        secondary_click(&mut harness, at);
        assert!(last(&frames));
        press(&mut harness, Key::Escape);
        assert!(!last(&frames));
    }
}
