//! Dismissal, focus and keyboard navigation of overlays through the real path:
//! events dispatched by `egui_kittest::Harness`, build, layout, draw, and the
//! judgement of the frame's input after the draw.

use eggshell_ui::overlay::{Anchor, DismissOn, Focus, Nav, Outside, Placement};
use eggshell_ui::{Builder, Scope, TextSpec, TextStyle, tw};
use egui::{Color32, Event, FontFamily, Key, Pos2, Role, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, id_of, rect_of};

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Page,
    Top,
    Trigger,
    MenuTrigger,
    Other,
    Dnd,
    Zone,
    Zone2,
    Source,
    Popover,
    Child,
    Box,
    A,
    B,
    PopZone,
    PopSource,
    Menu,
    Item(u8),
    More,
    Submenu,
    SubItem(u8),
    Dialog,
    List,
    Option(u8),
}

struct Card;

#[derive(Default)]
struct App {
    popover: bool,
    child: bool,
    menu: bool,
    submenu: bool,
    dialog: bool,
    list: bool,
    hide_trigger: bool,
    popover_dismiss: DismissOn,
    popover_outside: Option<Outside>,
    menu_outside: Option<Outside>,
    popover_focus: Option<Focus>,
    disabled_item: Option<u8>,
    page_clicks: u32,
    log: Vec<String>,
    drops: Vec<&'static str>,
}

const SCREEN: egui::Vec2 = vec2(400.0, 300.0);

const EMPTY: Pos2 = pos2(350.0, 280.0);

fn box_classes() -> eggshell_ui::Tw {
    tw().flex()
        .flex_col()
        .gap(1.0)
        .p(1.0)
        .bg(Color32::WHITE)
        .border(1.0)
        .border_color(Color32::GRAY)
}

fn item_classes() -> eggshell_ui::Tw {
    tw().w_px(110.0)
        .h_px(24.0)
        .sense_click()
        .hover(|p| p.bg(Color32::LIGHT_BLUE))
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        Part::Page,
        tw().flex()
            .flex_col()
            .gap(2.0)
            .w_full()
            .h_full()
            .p(2.0)
            .sense_click()
            .focusable_none(),
        |s| {
            s.div_id(Part::Top, tw().flex().gap(2.0).h_px(30.0), |s| {
                if !app.hide_trigger {
                    s.button(Part::Trigger, "Trigger")
                        .on_click(|app: &mut App| app.popover = !app.popover);
                }
                s.button(Part::MenuTrigger, "Menu")
                    .on_click(|app: &mut App| app.menu = !app.menu);
                s.button(Part::Other, "Other")
                    .on_click(|app: &mut App| app.log.push("other clicked".into()));
                if app.popover {
                    popover(s, app);
                }
                if app.menu {
                    menu(s, app);
                }
                if app.dialog {
                    dialog(s);
                }
                if app.list {
                    list(s);
                }
            });
            s.div_id(Part::Dnd, tw().flex().gap(4.0).mt(4.0), |s| {
                s.div_id(
                    Part::Zone,
                    tw().w_px(120.0)
                        .h_px(80.0)
                        .bg(Color32::from_rgb(200, 230, 200)),
                    |_| {},
                )
                .drop_target::<Card>(|app: &mut App, _| app.drops.push("zone"));
                s.div_id(
                    Part::Zone2,
                    tw().w_px(60.0)
                        .h_px(80.0)
                        .bg(Color32::from_rgb(200, 200, 230)),
                    |_| {},
                )
                .drop_target::<Card>(|app: &mut App, _| app.drops.push("zone2"));
                s.div_id(
                    Part::Source,
                    tw().w_px(40.0)
                        .h_px(40.0)
                        .bg(Color32::from_rgb(230, 150, 150)),
                    |_| {},
                )
                .drag_source(Card);
            });
        },
    )
    .on_click(|app: &mut App| app.page_clicks += 1);
}

fn popover(s: &mut Scope<'_, '_, App>, app: &App) {
    let trigger = s.id_of(Part::Trigger);
    let mut chain = s
        .overlay(Part::Popover)
        .anchor(Anchor::Node(trigger))
        .placement(Placement::below())
        .role(Role::Dialog)
        .named("Popover")
        .dismiss_on(app.popover_dismiss)
        .focus(app.popover_focus.unwrap_or_default())
        .on_dismiss(|app: &mut App| {
            app.popover = false;
            app.log.push("popover dismissed".into());
        });
    if let Some(outside) = app.popover_outside {
        chain = chain.outside(outside);
    }
    chain.children(|s| {
        s.div_id(Part::Box, box_classes().w_px(130.0), |s| {
            s.button(Part::A, "A").add();
            s.button(Part::B, "B").add();
            s.div_id(
                Part::PopZone,
                tw().w_px(60.0)
                    .h_px(30.0)
                    .bg(Color32::from_rgb(200, 230, 200)),
                |_| {},
            )
            .drop_target::<Card>(|app: &mut App, _| app.drops.push("popover zone"));
            s.div_id(
                Part::PopSource,
                tw().w_px(30.0)
                    .h_px(30.0)
                    .bg(Color32::from_rgb(230, 150, 150)),
                |_| {},
            )
            .drag_source(Card);
            if app.child {
                let a = s.id_of(Part::A);
                s.overlay(Part::Child)
                    .anchor(Anchor::Node(a))
                    .placement(Placement::submenu())
                    .role(Role::Dialog)
                    .named("Child")
                    .on_dismiss(|app: &mut App| {
                        app.child = false;
                        app.log.push("child dismissed".into());
                    })
                    .children(|s| {
                        s.div_id(Part::Box, box_classes().w_px(60.0).h_px(40.0), |_| {});
                    });
            }
        });
    });
}

fn menu_item(s: &mut Scope<'_, '_, App>, app: &App, index: u8, label: &'static str) {
    let classes = if app.disabled_item == Some(index) {
        item_classes().inert()
    } else {
        item_classes()
    };
    s.div_id(Part::Item(index), classes, |s| s.text(tw(), label))
        .role(Role::MenuItem)
        .named(label)
        .on_click(move |app: &mut App| app.log.push(format!("{label} chosen")));
}

fn menu(s: &mut Scope<'_, '_, App>, app: &App) {
    let trigger = s.id_of(Part::MenuTrigger);
    let mut chain = s
        .overlay(Part::Menu)
        .anchor(Anchor::Node(trigger))
        .placement(Placement::below())
        .nav(Nav::Menu)
        .role(Role::Menu)
        .named("Menu")
        .on_dismiss(|app: &mut App| {
            app.menu = false;
            app.log.push("menu dismissed".into());
        });
    if let Some(outside) = app.menu_outside {
        chain = chain.outside(outside);
    }
    chain.children(|s| {
        s.div_id(Part::Box, box_classes(), |s| {
            menu_item(s, app, 0, "One");
            menu_item(s, app, 1, "Two");
            menu_item(s, app, 2, "Three");
            s.div_id(Part::More, item_classes(), |s| s.text(tw(), "More"))
                .role(Role::MenuItem)
                .named("More")
                .on_click(|app: &mut App| app.submenu = !app.submenu);
            s.div_id(Part::Dialog, item_classes(), |s| {
                s.text(tw(), "Open dialog")
            })
            .role(Role::MenuItem)
            .named("Open dialog")
            .on_click(|app: &mut App| app.dialog = true);
            if app.submenu {
                let more = s.id_of(Part::More);
                s.overlay(Part::Submenu)
                    .anchor(Anchor::Node(more))
                    .placement(Placement::submenu())
                    .nav(Nav::Submenu)
                    .role(Role::Menu)
                    .named("More")
                    .on_dismiss(|app: &mut App| {
                        app.submenu = false;
                        app.log.push("submenu dismissed".into());
                    })
                    .children(|s| {
                        s.div_id(Part::Box, box_classes(), |s| {
                            for index in 0..2 {
                                s.div_id(Part::SubItem(index), item_classes(), |s| {
                                    s.text(tw(), "Sub");
                                })
                                .role(Role::MenuItem)
                                .named("Sub")
                                .on_click(move |app: &mut App| {
                                    app.log.push(format!("sub {index} chosen"));
                                });
                            }
                        });
                    });
            }
        });
    });
}

fn list(s: &mut Scope<'_, '_, App>) {
    let trigger = s.id_of(Part::Other);
    s.overlay(Part::List)
        .anchor(Anchor::Node(trigger))
        .focus(Focus::Stay)
        .nav(Nav::Listbox)
        .role(Role::ListBox)
        .named("Options")
        .on_dismiss(|app: &mut App| {
            app.list = false;
            app.log.push("list dismissed".into());
        })
        .children(|s| {
            s.div_id(Part::Box, box_classes(), |s| {
                for index in 0..3 {
                    let disabled = index == 1;
                    let classes = if disabled {
                        item_classes().inert()
                    } else {
                        item_classes()
                    };
                    s.div_id(Part::Option(index), classes, |s| s.text(tw(), "Option"))
                        .role(Role::ListBoxOption)
                        .named("Option")
                        .on_click(move |app: &mut App| {
                            app.log.push(format!("option {index} chosen"));
                        });
                }
            });
        });
}

fn dialog(s: &mut Scope<'_, '_, App>) {
    s.overlay(Part::Dialog)
        .anchor(Anchor::Point(pos2(200.0, 150.0)))
        .role(Role::Dialog)
        .named("Dialog")
        .on_dismiss(|app: &mut App| {
            app.dialog = false;
            app.log.push("dialog dismissed".into());
        })
        .children(|s| {
            s.div_id(Part::Box, box_classes().w_px(100.0).h_px(60.0), |_| {});
        });
}

fn harness(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(SCREEN, base_text(), app, view);
    harness.run_steps(4);
    harness
}

fn settle(harness: &mut Harness<'_, Rig<App>>) {
    harness.run_steps(4);
}

fn click_at(harness: &mut Harness<'_, Rig<App>>, at: Pos2) {
    support::hover_over(harness, at);
    support::press_and_release(harness, at);
    settle(harness);
}

fn open_popover(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, "/Page/Top/Trigger");
    settle(harness);
    assert!(harness.state().app.popover, "precondition: open");
}

fn open_menu(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, "/Page/Top/MenuTrigger");
    settle(harness);
    assert!(harness.state().app.menu, "precondition: open");
}

/// Opens the menu with Enter on its trigger, so that focus enters it: a menu
/// opened with the pointer takes none.
fn open_menu_from_the_keyboard(harness: &mut Harness<'_, Rig<App>>) {
    for _ in 0..12 {
        if focused_path(harness).as_deref() == Some("/Page/Top/MenuTrigger") {
            break;
        }
        harness.key_press(Key::Tab);
        settle(harness);
    }
    harness.key_press(Key::Enter);
    settle(harness);
    assert!(harness.state().app.menu, "precondition: open");
}

fn log(harness: &Harness<'_, Rig<App>>) -> Vec<String> {
    harness.state().app.log.clone()
}

fn outside_both(harness: &Harness<'_, Rig<App>>, overlay: &str) -> Pos2 {
    let rect = rect_of(harness, overlay);
    assert!(
        !rect.contains(EMPTY),
        "precondition: the spot is outside the overlay"
    );
    EMPTY
}

mod popover {
    use super::*;

    #[test]
    fn a_press_outside_dismisses_it_and_reaches_the_page() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        outside_both(&harness, "/Page/Top/Popover");
        click_at(&mut harness, EMPTY);
        let app = &harness.state().app;
        assert!(!app.popover);
        assert_eq!(app.log, ["popover dismissed"]);
        assert_eq!(app.page_clicks, 1, "a popover lets the press through");
    }

    #[test]
    fn a_press_inside_dismisses_nothing() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        let inside = rect_of(&harness, "/Page/Top/Popover/Box").center();
        click_at(&mut harness, inside);
        assert!(harness.state().app.popover);
        assert!(log(&harness).is_empty());
    }

    #[test]
    fn the_trigger_toggles_it_and_is_no_dismissal() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        click_node(&mut harness, "/Page/Top/Trigger");
        settle(&mut harness);
        let app = &harness.state().app;
        assert!(!app.popover, "the trigger's own click closed it");
        assert!(
            app.log.is_empty(),
            "dismiss then reopen would have shown here: {:?}",
            app.log
        );
    }

    #[test]
    fn the_press_that_dismisses_a_popover_can_activate_what_is_under_it() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        click_node(&mut harness, "/Page/Top/Other");
        settle(&mut harness);
        assert_eq!(log(&harness), ["popover dismissed", "other clicked"]);
    }

    #[test]
    fn outside_can_be_overridden_to_swallow() {
        let mut harness = harness(App {
            popover_outside: Some(Outside::Swallow),
            ..App::default()
        });
        open_popover(&mut harness);
        click_node(&mut harness, "/Page/Top/Other");
        settle(&mut harness);
        assert_eq!(log(&harness), ["popover dismissed"]);
    }

    #[test]
    fn it_is_dismissed_when_its_anchor_leaves_the_frame() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        harness.state_mut().app.hide_trigger = true;
        settle(&mut harness);
        assert_eq!(log(&harness), ["popover dismissed"]);
        assert!(!harness.state().app.popover);
    }

    #[test]
    fn escape_dismisses_it() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert_eq!(log(&harness), ["popover dismissed"]);
    }

    #[test]
    fn esc_only_ignores_the_press_and_takes_the_key() {
        let mut harness = harness(App {
            popover_dismiss: DismissOn::EscOnly,
            ..App::default()
        });
        open_popover(&mut harness);
        click_at(&mut harness, EMPTY);
        assert!(harness.state().app.popover);
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert!(!harness.state().app.popover);
    }

    #[test]
    fn never_ignores_both() {
        let mut harness = harness(App {
            popover_dismiss: DismissOn::Never,
            ..App::default()
        });
        open_popover(&mut harness);
        click_at(&mut harness, EMPTY);
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert!(harness.state().app.popover);
        assert!(log(&harness).is_empty());
    }

    #[test]
    fn a_press_inside_that_drags_out_and_releases_outside_dismisses_nothing() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        let inside = rect_of(&harness, "/Page/Top/Popover/Box").center();
        support::drag(&mut harness, inside, EMPTY);
        assert!(harness.state().app.popover);
        assert!(log(&harness).is_empty());
    }

    #[test]
    fn the_window_losing_focus_dismisses_it() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        harness.event(Event::WindowFocused(false));
        settle(&mut harness);
        assert_eq!(log(&harness), ["popover dismissed"]);
    }

    #[test]
    fn tabbing_out_of_it_dismisses_it() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Popover/Box/A"),
            "precondition: focus entered"
        );
        harness.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
        settle(&mut harness);
        let landed = focused_path(&harness).unwrap_or_default();
        assert!(
            landed.starts_with("/Page/") && !landed.starts_with("/Page/Top/Popover"),
            "precondition: focus left for the page: {landed}"
        );
        assert_eq!(log(&harness), ["popover dismissed"]);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(landed.as_str()),
            "where Tab put it stays"
        );
    }

    #[test]
    fn tabbing_within_it_does_not() {
        let mut harness = harness(App::default());
        open_popover(&mut harness);
        harness.key_press(Key::Tab);
        settle(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Popover/Box/B")
        );
        assert!(harness.state().app.popover);
    }
}

mod nested {
    use super::*;

    fn open() -> Harness<'static, Rig<App>> {
        let mut harness = harness(App {
            child: true,
            ..App::default()
        });
        open_popover(&mut harness);
        assert!(
            harness
                .state()
                .nodes
                .iter()
                .any(|(path, _)| path == "/Page/Top/Popover/Box/Child"),
            "precondition: the child is open"
        );
        harness
    }

    #[test]
    fn a_press_in_the_child_dismisses_nothing() {
        let mut harness = open();
        let child = rect_of(&harness, "/Page/Top/Popover/Box/Child/Box").center();
        click_at(&mut harness, child);
        assert!(log(&harness).is_empty());
    }

    #[test]
    fn a_press_in_the_parent_closes_only_the_child() {
        let mut harness = open();
        let parent = rect_of(&harness, "/Page/Top/Popover/Box/B").center();
        assert!(
            !rect_of(&harness, "/Page/Top/Popover/Box/Child").contains(parent),
            "precondition: outside the child"
        );
        click_at(&mut harness, parent);
        assert_eq!(log(&harness), ["child dismissed"]);
    }

    #[test]
    fn a_press_on_the_page_closes_both_child_first() {
        let mut harness = open();
        click_at(&mut harness, EMPTY);
        assert_eq!(log(&harness), ["child dismissed", "popover dismissed"]);
    }

    #[test]
    fn escape_closes_the_child_and_then_the_parent() {
        let mut harness = open();
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert_eq!(log(&harness), ["child dismissed"]);
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert_eq!(log(&harness), ["child dismissed", "popover dismissed"]);
    }
}

mod menu {
    use super::*;

    #[test]
    fn a_press_outside_dismisses_it_and_the_page_never_sees_it() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        click_at(&mut harness, EMPTY);
        let app = &harness.state().app;
        assert_eq!(app.log, ["menu dismissed"]);
        assert_eq!(app.page_clicks, 0, "the menu swallowed the press");
    }

    #[test]
    fn what_is_under_a_menu_does_not_get_the_dismissing_press() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/Other");
        settle(&mut harness);
        assert_eq!(log(&harness), ["menu dismissed"]);
    }

    #[test]
    fn the_trigger_closes_it_and_does_not_reopen_it() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/MenuTrigger");
        settle(&mut harness);
        assert!(!harness.state().app.menu);
        assert_eq!(log(&harness), ["menu dismissed"]);
    }

    #[test]
    fn outside_can_be_overridden_to_pass_through() {
        let mut harness = harness(App {
            menu_outside: Some(Outside::PassThrough),
            ..App::default()
        });
        open_menu(&mut harness);
        click_at(&mut harness, EMPTY);
        let app = &harness.state().app;
        assert_eq!(app.log, ["menu dismissed"]);
        assert_eq!(app.page_clicks, 1);
    }

    #[test]
    fn a_press_on_an_item_chooses_it_and_dismisses_nothing() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/Menu/Box/Item(1)");
        settle(&mut harness);
        assert_eq!(log(&harness), ["Two chosen"]);
        assert!(harness.state().app.menu, "closing is the app's business");
    }

    #[test]
    fn a_press_outside_the_submenu_on_the_page_closes_both_and_stops() {
        let mut harness = harness(App {
            submenu: true,
            ..App::default()
        });
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/Other");
        settle(&mut harness);
        assert_eq!(log(&harness), ["submenu dismissed", "menu dismissed"]);
    }

    #[test]
    fn a_press_in_the_menu_closes_only_the_submenu() {
        let mut harness = harness(App {
            submenu: true,
            ..App::default()
        });
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/Menu/Box/Item(0)");
        settle(&mut harness);
        assert_eq!(
            log(&harness),
            ["submenu dismissed", "One chosen"],
            "dismissed at the press, chosen at the release"
        );
        assert!(harness.state().app.menu);
    }

    #[test]
    fn menus_are_exclusive_a_popover_opening_dismisses_the_menu() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        harness.state_mut().app.popover = true;
        settle(&mut harness);
        assert_eq!(log(&harness), ["menu dismissed"]);
        assert!(harness.state().app.popover, "the new overlay stays");
    }

    #[test]
    fn popovers_coexist() {
        let mut harness = harness(App {
            child: false,
            ..App::default()
        });
        open_popover(&mut harness);
        harness.state_mut().app.dialog = true;
        settle(&mut harness);
        assert!(harness.state().app.popover && harness.state().app.dialog);
        assert!(log(&harness).is_empty());
    }

    #[test]
    fn a_menu_item_that_opens_a_dialog_needs_no_explicit_close() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/Menu/Box/Dialog");
        settle(&mut harness);
        assert_eq!(log(&harness), ["menu dismissed"]);
        assert!(harness.state().app.dialog);
    }

    #[test]
    fn a_submenu_opening_does_not_dismiss_its_menu() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        click_node(&mut harness, "/Page/Top/Menu/Box/More");
        settle(&mut harness);
        assert!(harness.state().app.submenu);
        assert!(harness.state().app.menu);
        assert!(log(&harness).is_empty());
    }

    #[test]
    fn a_menu_covers_the_page_for_the_pointer_without_drawing() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        let layer = egui::LayerId::new(
            egui::Order::Foreground,
            id_of(&harness, "/Page/Top/Menu").with("overlay"),
        );
        assert_eq!(
            harness.ctx.layer_id_at(EMPTY),
            Some(layer),
            "its layer is what the pointer meets on the page"
        );
    }
}

mod focus {
    use super::*;

    fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
        for _ in 0..12 {
            if focused_path(harness).as_deref() == Some(path) {
                return;
            }
            harness.key_press(Key::Tab);
            settle(harness);
        }
        panic!("Tab never reached {path}: {:?}", focused_path(harness));
    }

    fn open_from_the_keyboard(harness: &mut Harness<'_, Rig<App>>) {
        tab_to(harness, "/Page/Top/Trigger");
        harness.key_press(Key::Enter);
        settle(harness);
        assert!(harness.state().app.popover, "precondition: open");
    }

    #[test]
    fn enter_moves_focus_into_the_overlay_and_escape_gives_it_back_to_the_trigger() {
        let mut harness = harness(App::default());
        open_from_the_keyboard(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Popover/Box/A")
        );
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert!(!harness.state().app.popover);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Trigger"),
            "Esc blurred it inside egui; the library gave it back"
        );
    }

    #[test]
    fn focus_returns_after_a_swallowed_press() {
        let mut harness = harness(App::default());
        tab_to(&mut harness, "/Page/Top/MenuTrigger");
        harness.key_press(Key::Enter);
        settle(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Menu/Box/Item(0)"),
            "precondition: focus entered the menu"
        );
        click_at(&mut harness, EMPTY);
        assert!(!harness.state().app.menu);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/MenuTrigger")
        );
    }

    #[test]
    fn stay_leaves_focus_on_the_trigger_and_escape_restores_it() {
        let mut harness = harness(App {
            popover_focus: Some(Focus::Stay),
            ..App::default()
        });
        open_from_the_keyboard(&mut harness);
        assert_eq!(focused_path(&harness).as_deref(), Some("/Page/Top/Trigger"));
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert!(!harness.state().app.popover);
        assert_eq!(focused_path(&harness).as_deref(), Some("/Page/Top/Trigger"));
    }

    #[test]
    fn none_neither_takes_nor_gives_back_focus() {
        let mut harness = harness(App {
            popover_focus: Some(Focus::None),
            ..App::default()
        });
        open_from_the_keyboard(&mut harness);
        assert_eq!(focused_path(&harness).as_deref(), Some("/Page/Top/Trigger"));
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert!(!harness.state().app.popover);
        assert_eq!(focused_path(&harness), None);
    }

    #[test]
    fn a_submenu_gives_focus_back_to_the_item_that_opened_it() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        for _ in 0..3 {
            harness.key_press(Key::ArrowDown);
            settle(&mut harness);
        }
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Menu/Box/More")
        );
        harness.key_press(Key::Enter);
        settle(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Menu/Box/Submenu/Box/SubItem(0)"),
            "focus entered the submenu"
        );
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert_eq!(log(&harness), ["submenu dismissed"]);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Menu/Box/More")
        );
    }
}

mod keys {
    use super::*;

    fn focused_item(harness: &Harness<'_, Rig<App>>) -> String {
        focused_path(harness).unwrap_or_default()
    }

    fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
        harness.key_press(key);
        settle(harness);
    }

    const ONE: &str = "/Page/Top/Menu/Box/Item(0)";
    const TWO: &str = "/Page/Top/Menu/Box/Item(1)";
    const THREE: &str = "/Page/Top/Menu/Box/Item(2)";
    const DIALOG: &str = "/Page/Top/Menu/Box/Dialog";

    #[test]
    fn a_menu_opens_with_its_first_item_focused() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        assert_eq!(focused_item(&harness), ONE);
    }

    #[test]
    fn down_and_up_move_between_items_and_wrap() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        press(&mut harness, Key::ArrowDown);
        assert_eq!(focused_item(&harness), TWO);
        press(&mut harness, Key::ArrowUp);
        assert_eq!(focused_item(&harness), ONE);
        press(&mut harness, Key::ArrowUp);
        assert_eq!(focused_item(&harness), DIALOG, "wraps to the last");
        press(&mut harness, Key::ArrowDown);
        assert_eq!(focused_item(&harness), ONE, "wraps to the first");
    }

    #[test]
    fn home_and_end_go_to_the_ends() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        press(&mut harness, Key::End);
        assert_eq!(focused_item(&harness), DIALOG);
        press(&mut harness, Key::Home);
        assert_eq!(focused_item(&harness), ONE);
    }

    #[test]
    fn a_disabled_item_is_skipped() {
        let mut harness = harness(App {
            disabled_item: Some(1),
            ..App::default()
        });
        open_menu_from_the_keyboard(&mut harness);
        press(&mut harness, Key::ArrowDown);
        assert_eq!(focused_item(&harness), THREE);
        press(&mut harness, Key::ArrowUp);
        assert_eq!(focused_item(&harness), ONE);
    }

    #[test]
    fn the_arrows_do_not_leave_the_menu_for_the_page() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        for _ in 0..8 {
            press(&mut harness, Key::ArrowDown);
            assert!(
                focused_item(&harness).starts_with("/Page/Top/Menu/"),
                "{}",
                focused_item(&harness)
            );
        }
    }

    #[test]
    fn enter_chooses_the_focused_item() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::Enter);
        assert_eq!(log(&harness), ["Two chosen"]);
    }

    #[test]
    fn the_keys_belong_to_the_submenu_while_focus_is_in_it() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        for _ in 0..4 {
            if focused_item(&harness) == "/Page/Top/Menu/Box/More" {
                break;
            }
            press(&mut harness, Key::ArrowDown);
        }
        press(&mut harness, Key::Enter);
        let sub = |index: u8| format!("/Page/Top/Menu/Box/Submenu/Box/SubItem({index})");
        assert_eq!(focused_item(&harness), sub(0));
        press(&mut harness, Key::ArrowDown);
        assert_eq!(focused_item(&harness), sub(1));
        press(&mut harness, Key::ArrowDown);
        assert_eq!(focused_item(&harness), sub(0), "wraps inside the submenu");
    }

    #[test]
    fn left_closes_the_submenu_and_focuses_its_item() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        click_node(&mut harness, "/Page/Top/Menu/Box/More");
        settle(&mut harness);
        press(&mut harness, Key::ArrowLeft);
        assert_eq!(log(&harness), ["submenu dismissed"]);
        assert!(harness.state().app.menu, "the parent stays");
        assert_eq!(focused_item(&harness), "/Page/Top/Menu/Box/More");
    }

    #[test]
    fn left_in_a_menu_that_is_not_a_submenu_does_nothing() {
        let mut harness = harness(App::default());
        open_menu_from_the_keyboard(&mut harness);
        press(&mut harness, Key::ArrowLeft);
        assert!(harness.state().app.menu);
        assert_eq!(focused_item(&harness), ONE);
    }
}

mod listbox {
    use super::*;

    fn path(index: u8) -> String {
        format!("/Page/Top/List/Box/Option({index})")
    }

    fn open() -> Harness<'static, Rig<App>> {
        let mut harness = harness(App::default());
        support::past_bar(&mut harness);
        for _ in 0..3 {
            harness.key_press(Key::Tab);
        }
        settle(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Other"),
            "precondition: the trigger has focus"
        );
        harness.state_mut().app.list = true;
        settle(&mut harness);
        harness
    }

    fn highlighted(harness: &Harness<'_, Rig<App>>) -> Vec<u8> {
        (0..3)
            .filter(|&index| support::fill_of(harness, &path(index)) == Some(Color32::LIGHT_BLUE))
            .collect()
    }

    #[test]
    fn focus_stays_on_the_trigger_and_down_highlights_the_first_option() {
        let mut harness = open();
        assert!(highlighted(&harness).is_empty());
        harness.key_press(Key::ArrowDown);
        settle(&mut harness);
        assert_eq!(highlighted(&harness), [0]);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some("/Page/Top/Other"),
            "the highlight is virtual"
        );
    }

    #[test]
    fn the_highlight_skips_disabled_options_and_wraps() {
        let mut harness = open();
        for (key, expected) in [
            (Key::ArrowDown, 0),
            (Key::ArrowDown, 2),
            (Key::ArrowDown, 0),
            (Key::ArrowUp, 2),
            (Key::Home, 0),
            (Key::End, 2),
        ] {
            harness.key_press(key);
            settle(&mut harness);
            assert_eq!(highlighted(&harness), [expected], "after {key:?}");
        }
    }

    #[test]
    fn enter_activates_the_highlighted_option() {
        let mut harness = open();
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowDown);
        settle(&mut harness);
        harness.key_press(Key::Enter);
        settle(&mut harness);
        assert_eq!(log(&harness), ["option 2 chosen"]);
    }

    #[test]
    fn once_it_is_closed_enter_is_the_triggers_again() {
        let mut harness = open();
        harness.key_press(Key::ArrowDown);
        settle(&mut harness);
        harness.key_press(Key::Escape);
        settle(&mut harness);
        assert!(!harness.state().app.list);
        harness.key_press(Key::Enter);
        settle(&mut harness);
        assert_eq!(log(&harness), ["list dismissed", "other clicked"]);
    }

    #[test]
    fn enter_with_nothing_highlighted_activates_nothing() {
        let mut harness = open();
        harness.key_press(Key::Enter);
        settle(&mut harness);
        assert!(!log(&harness).iter().any(|entry| entry.contains("chosen")));
    }

    #[test]
    fn the_trigger_is_inside_so_its_press_is_no_dismissal() {
        let mut harness = open();
        click_node(&mut harness, "/Page/Top/Other");
        settle(&mut harness);
        assert!(!log(&harness).contains(&"list dismissed".to_owned()));
    }
}

mod drag_and_drop {
    use super::*;

    fn centre(harness: &Harness<'_, Rig<App>>, path: &str) -> Pos2 {
        rect_of(harness, path).center()
    }

    fn with_popover(dismiss_on: DismissOn) -> Harness<'static, Rig<App>> {
        let mut harness = harness(App {
            popover: true,
            popover_dismiss: dismiss_on,
            ..App::default()
        });
        settle(&mut harness);
        harness
    }

    #[test]
    fn a_target_under_a_popover_is_unreachable() {
        let mut open = with_popover(DismissOn::Never);
        let zone = rect_of(&open, "/Page/Dnd/Zone");
        let popover = rect_of(&open, "/Page/Top/Popover/Box");
        let target = pos2(zone.min.x + 3.0, zone.center().y);
        assert!(
            zone.contains(target)
                && popover.contains(target)
                && !rect_of(&open, "/Page/Top/Popover/Box/PopZone").contains(target),
            "precondition: the point is on the zone under the popover"
        );
        let source = centre(&open, "/Page/Dnd/Source");
        assert!(
            !popover.contains(source),
            "precondition: the source is clear of the popover"
        );
        support::drag(&mut open, source, target);
        assert!(open.state().app.drops.is_empty());

        let mut closed = harness(App::default());
        support::drag(&mut closed, source, target);
        assert_eq!(
            closed.state().app.drops,
            ["zone"],
            "the control: with no popover the same drag lands"
        );
    }

    #[test]
    fn a_target_inside_a_popover_is_reachable() {
        let mut harness = with_popover(DismissOn::Never);
        let source = centre(&harness, "/Page/Dnd/Source");
        let target = centre(&harness, "/Page/Top/Popover/Box/PopZone");
        support::drag(&mut harness, source, target);
        assert_eq!(harness.state().app.drops, ["popover zone"]);
    }

    #[test]
    fn a_drag_from_a_popover_to_the_page_lands_and_dismisses_nothing() {
        let mut harness = with_popover(DismissOn::Default);
        let source = centre(&harness, "/Page/Top/Popover/Box/PopSource");
        let target = centre(&harness, "/Page/Dnd/Zone2");
        assert!(
            !rect_of(&harness, "/Page/Top/Popover/Box").contains(target),
            "precondition: the target is on the page"
        );
        support::drag(&mut harness, source, target);
        assert_eq!(harness.state().app.drops, ["zone2"]);
        assert!(harness.state().app.popover);
        assert!(log(&harness).is_empty());
    }
}

mod accessibility {
    use super::*;

    #[test]
    fn the_overlay_has_its_role_and_name() {
        let mut harness = harness(App::default());
        open_menu(&mut harness);
        harness.get_by_role_and_label(Role::Menu, "Menu");
        harness.get_by_role_and_label(Role::MenuItem, "One");
    }

    #[test]
    fn the_trigger_says_its_popup_is_open() {
        let mut harness = harness(App::default());
        let trigger = |harness: &Harness<'_, Rig<App>>| {
            let node = harness.get_by_role_and_label(Role::Button, "Menu");
            let data = node.accesskit_node().data().clone();
            (data.is_expanded(), data.has_popup())
        };
        assert_eq!(
            trigger(&harness),
            (None, None),
            "closed: its widget's business"
        );
        open_menu(&mut harness);
        assert_eq!(
            trigger(&harness),
            (Some(true), Some(egui::accesskit::HasPopup::Menu))
        );
    }
}

mod egui_facts {
    //! What the design relies on in egui itself: kept as tests so that a new
    //! egui that changes it is noticed.

    use egui::{Modifiers, PointerButton, Ui};

    use super::*;

    #[derive(Default)]
    struct World {
        text: String,
        focused: Option<bool>,
    }

    fn click(harness: &mut Harness<'_, World>, at: Pos2) {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.step();
        }
    }

    #[test]
    fn escape_blurs_the_focused_widget_before_any_overlay_code_runs() {
        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .build_ui_state(
                |ui: &mut Ui, world: &mut World| {
                    let field = ui.text_edit_singleline(&mut world.text);
                    world.focused = Some(field.has_focus());
                    let escape = ui.input(|input| input.key_pressed(Key::Escape));
                    if escape {
                        world.text = "saw escape".into();
                    }
                },
                World::default(),
            );
        harness.run_steps(2);
        click(&mut harness, pos2(30.0, 12.0));
        harness.run_steps(2);
        assert_eq!(harness.state().focused, Some(true));
        harness.key_press(Key::Escape);
        harness.run_steps(2);
        assert_eq!(
            harness.state().focused,
            Some(false),
            "Esc surrenders focus in egui itself: an overlay that closes on Esc must give focus \
             back to the trigger or field it came from"
        );
        assert_eq!(
            harness.state().text,
            "saw escape",
            "and the key is still there for library code to read after the draw"
        );
    }

    #[test]
    fn a_press_keeps_its_origin_through_a_drag_that_ends_elsewhere() {
        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .build_ui_state(|_: &mut Ui, _: &mut World| {}, World::default());
        harness.run_steps(2);
        let inside = pos2(50.0, 50.0);
        let outside = pos2(300.0, 150.0);
        harness.event(Event::PointerButton {
            pos: inside,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        let origin = |h: &Harness<'_, World>| h.ctx.input(|i| i.pointer.press_origin());
        let pressed = |h: &Harness<'_, World>| h.ctx.input(|i| i.pointer.any_pressed());
        assert!(pressed(&harness));
        assert_eq!(origin(&harness), Some(inside));
        harness.hover_at(outside);
        harness.step();
        assert!(
            !pressed(&harness),
            "any_pressed is true on the press frame only"
        );
        assert_eq!(
            origin(&harness),
            Some(inside),
            "the origin outlives the move"
        );
        harness.event(Event::PointerButton {
            pos: outside,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        assert!(
            !pressed(&harness),
            "a release is not a press, so dismissal judged at the press never sees it"
        );
    }
}
