//! The stock menus through the real path: kittest's pointer and key events,
//! build, layout, draw and the judgement of the frame's input.

use eggshell_ui::overlay::{Anchor, Nav, Placement, Wrap};
use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, NodeUi as _, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::{HasPopup, Toggled};
use egui::{Color32, FontFamily, Key, Modifiers, PointerButton, Role, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path, press_and_release};

const SCREEN: egui::Vec2 = vec2(500.0, 640.0);

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
                        m.item("plain", "Plain").inset().on_click(say("plain"));
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

fn harness() -> Driver<App> {
    let mut d = support::driver(SCREEN, base_text(), App::default(), view).build();
    d.frames(4);
    d
}

fn has(d: &Driver<App>, path: &str) -> bool {
    support::nodes(d)
        .iter()
        .any(|(candidate, _)| candidate == path)
}

fn item(menu: &str, key: &str) -> String {
    format!("{menu}/{key}")
}

fn open_file(d: &mut Driver<App>) {
    d.click(Target::suffix(FILE)).frames(5);
    assert!(has(d, MENU), "precondition: the menu is open");
}

fn open_more(d: &mut Driver<App>) {
    d.click(Target::suffix(&item(MENU, r#""more""#))).frames(5);
    assert!(has(d, SUB), "precondition: the submenu is open");
}

#[test]
fn clicking_the_trigger_opens_the_menu_and_clicking_it_again_closes_it() {
    let mut d = harness();
    assert!(!has(&d, MENU));
    d.click(Target::suffix(FILE)).frames(5);
    assert!(has(&d, MENU), "a click opens");
    d.click(Target::suffix(FILE)).frames(5);
    assert!(!has(&d, MENU), "a second click closes");
}

#[test]
fn the_trigger_says_whether_its_menu_is_open() {
    let mut d = harness();
    let trigger = |d: &Driver<App>| {
        let data = d
            .harness()
            .get_by_role_and_label(Role::Button, "File")
            .accesskit_node()
            .data()
            .clone();
        (data.is_expanded(), data.has_popup())
    };
    assert_eq!(trigger(&d), (Some(false), Some(HasPopup::Menu)));
    open_file(&mut d);
    assert_eq!(trigger(&d), (Some(true), Some(HasPopup::Menu)));
}

#[test]
fn items_have_the_menu_item_roles_and_the_checked_state() {
    let mut d = harness();
    d.app_mut().grid = true;
    d.app_mut().size = 1;
    open_file(&mut d);
    d.harness().get_by_role_and_label(Role::Menu, "File");
    d.harness().get_by_role_and_label(Role::MenuItem, "Copy");
    let toggled = |d: &Driver<App>, role, label| {
        d.harness()
            .get_by_role_and_label(role, label)
            .accesskit_node()
            .toggled()
    };
    assert_eq!(
        toggled(&d, Role::MenuItemCheckBox, "Show grid"),
        Some(Toggled::True)
    );
    assert_eq!(
        toggled(&d, Role::MenuItemRadio, "Large"),
        Some(Toggled::True)
    );
    assert_eq!(
        toggled(&d, Role::MenuItemRadio, "Small"),
        Some(Toggled::False)
    );
    let more = d
        .harness()
        .get_by_role_and_label(Role::MenuItem, "More")
        .accesskit_node()
        .data()
        .clone();
    assert_eq!(more.has_popup(), Some(HasPopup::Menu));
    assert_eq!(more.is_expanded(), Some(false));
}

#[test]
fn separators_have_no_node_and_the_arrows_pass_them() {
    let mut d = harness();
    open_file(&mut d);
    assert_eq!(
        d.harness().query_all_by_role(Role::Splitter).count(),
        0,
        "a separator is painted, not announced"
    );
    for _ in 0..10 {
        if focused_path(&d) == Some(item(MENU, r#""plain""#)) {
            break;
        }
        d.press(Key::ArrowDown).frames(4);
    }
    assert_eq!(
        focused_path(&d),
        Some(item(MENU, r#""plain""#)),
        "precondition: the item before the separator is focused"
    );
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(
        focused_path(&d),
        Some(item(MENU, r#""delete""#)),
        "Down passes the separator"
    );
    d.press(Key::ArrowUp).frames(4);
    assert_eq!(focused_path(&d), Some(item(MENU, r#""plain""#)));
}

#[test]
fn activating_an_item_runs_its_handler_and_closes_the_menu() {
    let mut d = harness();
    open_file(&mut d);
    d.click(Target::suffix(&item(MENU, r#""copy""#))).frames(5);
    assert_eq!(d.app().log, ["copy"]);
    assert!(!has(&d, MENU));
}

#[test]
fn checkbox_and_radio_items_report_their_new_value_and_close() {
    let mut d = harness();
    open_file(&mut d);
    d.click(Target::suffix(&item(MENU, r#""grid""#))).frames(5);
    assert!(d.app().grid, "the checkbox moves to on");
    assert!(!has(&d, MENU));
    open_file(&mut d);
    d.click(Target::suffix(&item(MENU, r#"("size", 1)"#)))
        .frames(5);
    assert_eq!(d.app().size, 1);
    open_file(&mut d);
    d.click(Target::suffix(&item(MENU, r#""grid""#))).frames(5);
    assert!(!d.app().grid, "and back to off");
}

#[test]
fn a_disabled_item_takes_no_click_and_the_arrows_skip_it() {
    let mut d = harness();
    open_file(&mut d);
    d.click(Target::suffix(&item(MENU, r#""paste""#))).frames(5);
    assert!(d.app().log.is_empty());
    assert!(has(&d, MENU), "the menu stays open");
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(focused_path(&d), Some(item(MENU, r#""copy""#)));
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(
        focused_path(&d),
        Some(item(MENU, r#""grid""#)),
        "Paste is skipped"
    );
}

#[test]
fn activating_an_item_of_a_submenu_closes_the_whole_chain() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    d.click(Target::suffix(&item(SUB, r#""deeper""#))).frames(5);
    assert_eq!(d.app().log, ["deeper"]);
    assert!(!has(&d, MENU) && !has(&d, SUB));
    open_file(&mut d);
    assert!(!has(&d, SUB), "it opens with its submenu closed");
}

#[test]
fn a_click_opens_the_submenu_beside_its_item_with_nothing_focused() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    let parent = d.rect(Target::suffix(&item(MENU, r#""more""#)));
    let sub = d.rect(Target::suffix(SUB));
    assert!(sub.min.x > parent.center().x, "beside the item");
    assert_eq!(focused_path(&d), None);
    assert_eq!(
        d.harness()
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
        let mut d = harness();
        open_file(&mut d);
        for _ in 0..6 {
            d.press(Key::ArrowDown).frames(4);
            if focused_path(&d) == Some(item(MENU, r#""more""#)) {
                break;
            }
        }
        assert_eq!(
            focused_path(&d),
            Some(item(MENU, r#""more""#)),
            "precondition: the item is focused"
        );
        d.press(key).frames(4);
        assert!(has(&d, SUB), "{key:?} opens the submenu");
        assert_eq!(
            focused_path(&d),
            Some(item(SUB, r#""deep""#)),
            "{key:?} focuses the first item"
        );
    }
}

#[test]
fn left_closes_the_submenu_and_focuses_its_item_but_escape_goes_one_level_at_a_time() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(focused_path(&d), Some(item(SUB, r#""deep""#)));
    d.press(Key::ArrowLeft).frames(4);
    assert!(!has(&d, SUB) && has(&d, MENU), "Left closes it");
    assert_eq!(focused_path(&d), Some(item(MENU, r#""more""#)));

    d.press(Key::ArrowRight).frames(4);
    assert!(has(&d, SUB));
    d.press(Key::Escape).frames(4);
    assert!(!has(&d, SUB) && has(&d, MENU), "Esc closes it");
    assert_eq!(focused_path(&d), Some(item(MENU, r#""more""#)));
    d.press(Key::Escape).frames(4);
    assert!(!has(&d, MENU), "the next Esc closes the menu");
}

#[test]
fn with_a_submenu_open_the_pointer_on_a_sibling_submenu_item_switches_to_it() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    d.hover(Target::suffix(&item(MENU, r#""export""#)))
        .frames(5);
    assert!(has(&d, OTHER_SUB), "the Export submenu is open");
    assert!(!has(&d, SUB), "and More is closed");
}

#[test]
fn with_a_submenu_open_the_pointer_on_a_plain_item_closes_it() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    d.hover(Target::suffix(&item(MENU, r#""plain""#))).frames(5);
    assert!(!has(&d, SUB), "the submenu is closed");
    assert!(has(&d, MENU), "the menu is not");
}

#[test]
fn hovering_a_submenu_item_does_not_open_it_from_closed() {
    let mut d = harness();
    open_file(&mut d);
    d.hover(Target::suffix(&item(MENU, r#""more""#))).frames(5);
    d.hover(Target::suffix(&item(MENU, r#""export""#)))
        .frames(5);
    assert!(!has(&d, SUB) && !has(&d, OTHER_SUB));
}

#[test]
fn the_pointer_on_the_open_submenus_own_item_keeps_it_open() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    d.hover(Target::suffix(&item(MENU, r#""more""#))).frames(5);
    assert!(has(&d, SUB));
}

#[test]
fn a_press_outside_closes_the_whole_chain_and_nothing_under_it_sees_it() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    let other = d.rect(Target::suffix(r#"/"Page"/"Top"/"other""#)).center();
    assert!(
        !d.rect(Target::suffix(MENU)).contains(other)
            && !d.rect(Target::suffix(SUB)).contains(other),
        "precondition: the press is outside both"
    );
    press_and_release(&mut d, other);
    d.frames(5);
    assert!(!has(&d, MENU) && !has(&d, SUB));
}

#[test]
fn a_press_on_a_parent_item_with_a_submenu_open_reaches_the_item() {
    let mut d = harness();
    open_file(&mut d);
    open_more(&mut d);
    d.click(Target::suffix(&item(MENU, r#""copy""#))).frames(5);
    assert_eq!(d.app().log, ["copy"], "one click is enough");
}

fn tab_to(d: &mut Driver<App>, path: &str) {
    for _ in 0..8 {
        if focused_path(d).as_deref() == Some(path) {
            return;
        }
        d.press(Key::Tab).frames(4);
    }
    panic!("could not tab to {path}");
}

#[test]
fn enter_space_and_down_on_the_trigger_open_the_menu_and_focus_its_first_item() {
    for key in [Key::Enter, Key::Space, Key::ArrowDown] {
        let mut d = harness();
        tab_to(&mut d, FILE);
        d.press(key).frames(4);
        assert!(has(&d, MENU), "{key:?} opens");
        assert_eq!(
            focused_path(&d),
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
    let mut d = support::driver(
        SCREEN,
        base_text(),
        App {
            size: 2,
            ..App::default()
        },
        marked,
    )
    .build();
    d.frames(4);
    tab_to(&mut d, r#"/"Page"/"file""#);
    d.press(Key::Enter).frames(4);
    assert_eq!(
        focused_path(&d),
        Some(r#"/"Page"/("file", Menu)/Box/("size", 2)"#.to_owned())
    );
}

/// Down in the frame right after the menu gave its first item focus moves
/// once, as in a grid (#226).
#[test]
fn down_in_the_frame_after_the_first_item_gets_focus_moves_once() {
    let mut d = harness();
    tab_to(&mut d, FILE);
    d.harness_mut().key_press(Key::Enter);
    let first = item(MENU, r#""copy""#);
    let mut frames = 0;
    while focused_path(&d).as_deref() != Some(first.as_str()) {
        d.frames(1);
        frames += 1;
        assert!(frames < 10, "precondition: Enter focuses the first item");
    }
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(
        focused_path(&d),
        Some(item(MENU, r#""grid""#)),
        "past the disabled Paste to Show grid, and no further"
    );
}

#[test]
fn escape_closes_the_menu_and_gives_the_focus_back_to_the_trigger() {
    let mut d = harness();
    tab_to(&mut d, FILE);
    d.press(Key::Enter).frames(4);
    d.press(Key::Escape).frames(4);
    assert!(!has(&d, MENU));
    assert_eq!(focused_path(&d).as_deref(), Some(FILE));
}

#[test]
fn a_trigger_that_becomes_disabled_closes_its_menu() {
    let mut d = harness();
    open_file(&mut d);
    d.app_mut().disabled = true;
    d.frames(5);
    assert!(!has(&d, MENU));
}

#[test]
fn a_secondary_click_opens_the_context_menu_at_the_pointer() {
    let mut d = harness();
    let canvas = d.rect(Target::suffix(CANVAS));
    let at = canvas.min + vec2(30.0, 20.0);
    d.click_with(at, PointerButton::Secondary, Modifiers::NONE)
        .frames(5);
    let menu = support::nodes(&d)
        .iter()
        .find(|(path, _)| path.starts_with(CONTEXT) && path.ends_with("/Box"))
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("the context menu is open: {:?}", support::nodes(&d)));
    assert!(
        (menu.min - at).length() < 1.5,
        "its corner is at the pointer: {menu:?} for {at:?}"
    );
    d.harness()
        .get_by_role_and_label(Role::Menu, "Context menu");
}

#[test]
fn a_context_menu_item_runs_and_closes_the_menu_and_escape_closes_it_too() {
    let mut d = harness();
    let canvas = d.rect(Target::suffix(CANVAS));
    d.click_with(
        canvas.min + vec2(30.0, 20.0),
        PointerButton::Secondary,
        Modifiers::NONE,
    )
    .frames(5);
    d.harness()
        .get_by_role_and_label(Role::MenuItem, "Cut")
        .click();
    d.frames(5);
    assert_eq!(d.app().log, ["cut"]);
    assert!(
        !support::nodes(&d)
            .iter()
            .any(|(path, _)| path.starts_with(CONTEXT)),
        "closed"
    );

    d.click_with(
        canvas.min + vec2(30.0, 20.0),
        PointerButton::Secondary,
        Modifiers::NONE,
    )
    .frames(5);
    d.press(Key::Escape).frames(4);
    assert!(
        !support::nodes(&d)
            .iter()
            .any(|(path, _)| path.starts_with(CONTEXT)),
        "Esc closes"
    );
}

#[test]
fn a_context_menu_has_submenus_too() {
    let mut d = harness();
    let canvas = d.rect(Target::suffix(CANVAS));
    d.click_with(
        canvas.min + vec2(30.0, 20.0),
        PointerButton::Secondary,
        Modifiers::NONE,
    )
    .frames(5);
    d.harness()
        .get_by_role_and_label(Role::MenuItem, "More")
        .click();
    d.frames(5);
    d.harness()
        .get_by_role_and_label(Role::MenuItem, "Deep")
        .click();
    d.frames(5);
    assert_eq!(d.app().log, ["context deep"]);
}

#[test]
fn a_secondary_click_on_a_node_while_a_menu_is_open_only_closes_the_menu() {
    let mut d = harness();
    open_file(&mut d);
    let canvas = d.rect(Target::suffix(CANVAS));
    d.click_with(
        canvas.min + vec2(30.0, 20.0),
        PointerButton::Secondary,
        Modifiers::NONE,
    )
    .frames(5);
    assert!(!has(&d, MENU), "the dropdown closed");
    assert!(
        !support::nodes(&d)
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

    fn harness() -> Driver<App> {
        let mut d = support::driver(SCREEN, base_text(), App::default(), view).build();
        d.frames(4);
        d.click(Target::suffix(r#"/"Page"/"trigger""#)).frames(5);
        d
    }

    #[test]
    fn a_custom_painted_swatch_with_an_item_role_is_reached_by_the_arrows_and_activated() {
        let mut d = harness();
        d.harness()
            .get_by_role_and_label(Role::MenuItemRadio, "Green");
        d.press(Key::ArrowDown).frames(4);
        d.press(Key::ArrowDown).frames(4);
        let path = r#"/"Page"/("swatches")/"Box"/("swatch", 1)"#;
        let focused = focused_path(&d).expect("a swatch has focus");
        assert!(focused.ends_with(r#"("swatch", 1)"#), "{focused} of {path}");
        d.press(Key::Enter).frames(4);
        assert_eq!(d.app().swatch, 1);
    }
}

mod trigger {
    use eggshell_ui::owned::Face;
    use eggshell_ui::{Icon, Opacity};

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

    fn opened(view: support::View<App>) -> Driver<App> {
        let mut d = support::driver(SCREEN, base_text(), App::default(), view).build();
        d.frames(4);
        d
    }

    fn under(d: &Driver<App>, parent: &str, part: &str) -> Option<egui::Rect> {
        let path = format!("{parent}/{part}");
        support::nodes(d)
            .iter()
            .find(|(candidate, _)| *candidate == path)
            .map(|(_, rect)| *rect)
    }

    fn glyph(d: &Driver<App>, parent: &str) -> egui::Rect {
        under(d, parent, "<glyph>").expect("the trigger draws a chevron")
    }

    /// The most opaque vertex the chevron of the dropdown is drawn with.
    fn chevron_alpha(d: &Driver<App>) -> u8 {
        let at = glyph(d, DD);
        support::shapes(d)
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
        let d = opened(chevron_and_select);
        let (label, chevron) = (
            under(&d, DD, "<text>").expect("the trigger has its label"),
            glyph(&d, DD),
        );
        let iconed = r#"/"iconed""#;
        let gap = under(&d, iconed, "<text>").expect("label").min.x - glyph(&d, iconed).max.x;
        assert!(gap > 0.0, "precondition: the trigger has a gap: {gap}");
        assert!(
            (chevron.min.x - label.max.x - gap).abs() < 0.01,
            "the chevron is {} after the label, the trigger's gap is {gap}",
            chevron.min.x - label.max.x
        );
        assert_eq!(chevron.size(), glyph(&d, SEL).size());
        d.harness().get_by_role_and_label(Role::Button, "File");
    }

    #[test]
    fn the_chevron_takes_the_selects_size_and_tone_from_the_style() {
        let restyled = |chevron: f32, opacity: f32| {
            let mut d = opened(chevron_and_select);
            d.set_style(support::shadcn_with(|style| {
                *style = support::sections_of(style.clone());
                let mut select = style.select().clone();
                select.chevron = chevron;
                select.chevron_opacity = Opacity::new(opacity);
                style.set_select(select);
            }));
            d.frames(4);
            d
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
        let d = opened(chevron_and_select);
        assert!(
            under(&d, r#"/"plain""#, "<text>").is_some(),
            "precondition: the trigger is there"
        );
        assert!(under(&d, r#"/"plain""#, "<glyph>").is_none());
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
        let mut d = opened(swatch);
        let trigger = d.rect(Target::suffix(DD));
        let dot = under(&d, DD, r#""dot""#).expect("the child is in the trigger");
        assert!(trigger.contains_rect(dot), "{dot:?} in {trigger:?}");
        assert!(
            dot.max.x <= glyph(&d, DD).min.x,
            "the chevron follows the children"
        );
        d.harness()
            .get_by_role_and_label(Role::Button, "Accent")
            .click();
        d.frames(5);
        d.harness().get_by_role_and_label(Role::Menu, "Accent");
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
        let mut d = opened(wide);
        let width = |d: &mut Driver<App>, key: &str| {
            d.click(Target::suffix(&format!(r#"/"Top"/"{key}""#)))
                .frames(5);
            let width = d
                .rect(Target::suffix(&format!(r#"/"Top"/("{key}", Menu)/Box"#)))
                .width();
            d.press(Key::Escape).frames(9);
            width
        };
        assert_eq!(width(&mut d, "wide"), 300.0);
        let plain = width(&mut d, "plain");
        assert!(
            plain < 300.0,
            "the other menu keeps the style's width: {plain}"
        );
    }
}

mod open_query {
    use std::cell::RefCell;
    use std::rc::Rc;

    use eggshell_ui::Placed;
    use eggshell_ui::testing::After;

    use super::*;

    /// Each frame's answer to `open` and whether a menu overlay was leaving.
    type Frames = Rc<RefCell<Vec<(bool, bool)>>>;

    /// What sees each frame as it is placed, before it is drawn.
    type Observer<A> = Rc<RefCell<Option<Box<dyn FnMut(&Placed<A>)>>>>;

    /// [`support::driver`] with an observer of each placed frame, 4 frames
    /// in.
    fn observable<A: 'static>(app: A, view: support::View<A>) -> (Driver<A>, Observer<A>) {
        let observer: Observer<A> = Rc::default();
        let observe = Rc::clone(&observer);
        let mut bar = support::Bar::new(base_text());
        let mut d = Driver::whole_frame(app, move |ui, app, layout| {
            let area = ui.max_rect();
            if eggshell_ui_shadcn::tokens(ui.ctx()).is_none() {
                eggshell_ui_shadcn::Shadcn::apply_both(ui.ctx());
            }
            bar.show(ui);
            let placed = layout.build(ui.ctx(), |b| view(b, &*app)).layout(area);
            if let Some(observe) = observe.borrow_mut().as_mut() {
                observe(&placed);
            }
            placed.draw(app, ui)
        })
        .size(SCREEN)
        .base_text(base_text())
        .style(support::shadcn())
        .step(support::STEP)
        .after(After::Frames(0))
        .settle_within(support::STEP * 4)
        .build();
        d.frames(4);
        (d, observer)
    }

    fn observed<A: 'static>(
        observer: &Observer<A>,
        open: impl Fn(&Placed<A>) -> bool + 'static,
    ) -> Frames {
        let frames: Frames = Rc::default();
        let seen = Rc::clone(&frames);
        *observer.borrow_mut() = Some(Box::new(move |placed| {
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
    fn closes_before_it_fades<A>(d: &mut Driver<A>, frames: &Frames) {
        frames.borrow_mut().clear();
        d.press(Key::Escape).frames(7);
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
        let (mut d, observer) = observable(App::default(), view);
        let trigger = support::id_of(&d, FILE);
        let frames = observed(&observer, move |placed| placed.menu_open_with(trigger));
        d.frames(5);
        assert!(!last(&frames));
        open_file(&mut d);
        assert!(last(&frames));
        closes_before_it_fades(&mut d, &frames);
    }

    #[test]
    fn a_dropdown_at_the_top_of_the_view_is_found_by_its_key() {
        fn top(b: &mut Builder<'_, '_, App>, _: &App) {
            b.dropdown_menu("top", "Top")
                .items(|m| m.item("copy", "Copy").on_click(say("copy")));
        }
        let (mut d, observer) = observable(App::default(), top);
        let frames = observed(&observer, |placed| placed.menu_open("top"));
        d.frames(5);
        assert!(!last(&frames));
        d.click(Target::suffix(r#"/"top""#)).frames(5);
        assert!(last(&frames));
        closes_before_it_fades(&mut d, &frames);
    }

    #[test]
    fn a_context_menu_is_found_by_its_node() {
        let (mut d, observer) = observable(App::default(), view);
        let canvas = support::id_of(&d, CANVAS);
        let frames = observed(&observer, move |placed| placed.menu_open_with(canvas));
        d.frames(5);
        assert!(!last(&frames));
        let at = d.rect(Target::suffix(CANVAS)).min + vec2(30.0, 20.0);
        d.click_with(at, PointerButton::Secondary, Modifiers::NONE)
            .frames(5);
        assert!(last(&frames));
        d.press(Key::Escape).frames(4);
        assert!(!last(&frames));
    }
}

mod look {
    //! shadcn's dropdown-menu, as the preset draws it (`style-vega.css`).
    use super::*;
    use eggshell_ui_shadcn::Tokens;

    fn tokens(d: &Driver<App>) -> Tokens {
        Tokens::for_theme(d.ctx().theme())
    }

    fn glyph_in(d: &Driver<App>, row: &str) -> Option<egui::Rect> {
        support::nodes(d)
            .into_iter()
            .find(|(path, _)| path.contains(&format!("{row}/")) && path.ends_with("<glyph>"))
            .map(|(_, rect)| rect)
    }

    #[test]
    fn a_chosen_checkbox_or_radio_item_shows_a_check_at_the_rows_right() {
        let mut d = harness();
        open_file(&mut d);
        let small = item(MENU, r#"("size", 0)"#);
        let row = d.rect(Target::suffix(&small));
        let check = glyph_in(&d, &small).expect("the chosen one's check");
        assert_eq!(check.width(), 16.0, "size-4");
        assert_eq!(row.right() - check.right(), 8.0, "absolute right-2");
        assert_eq!(
            glyph_in(&d, &item(MENU, r#"("size", 1)"#)),
            None,
            "the other has none"
        );
    }

    #[test]
    fn an_inset_item_starts_its_label_past_the_marks() {
        let mut d = harness();
        open_file(&mut d);
        let row = d.rect(Target::suffix(&item(MENU, r#""plain""#)));
        let label = support::text_left(&d, row, "Plain");
        assert_eq!(label - row.left(), 32.0, "data-inset:pl-8");
    }

    #[test]
    fn the_highlighted_item_takes_the_accent_and_a_destructive_one_its_tint() {
        let mut d = harness();
        open_file(&mut d);
        let t = tokens(&d);
        for (key, fill) in [
            (r#""copy""#, t.accent),
            (r#""more""#, t.accent),
            (
                r#""delete""#,
                match d.ctx().theme() {
                    egui::Theme::Light => t.destructive.gamma_multiply(0.1),
                    egui::Theme::Dark => t.destructive.gamma_multiply(0.2),
                },
            ),
        ] {
            let path = item(MENU, key);
            d.hover(Target::suffix(&path)).frames(5);
            assert_eq!(support::fill_of(&d, &path), Some(fill), "{key}");
        }
    }
}
