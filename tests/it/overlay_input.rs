//! Dismissal, focus and keyboard navigation of overlays through the real path:
//! events dispatched by `egui_kittest::Harness`, build, layout, draw, and the
//! judgement of the frame's input after the draw.

use eggshell_ui::overlay::{
    Anchor, DismissOn, Dismissed, Focus, Nav, Outside, OverlayChain, Placement, Wrap,
};
use eggshell_ui::testing::{Driver, Target, WindowFocus};
use eggshell_ui::{Builder, Scope, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Event, FontFamily, Key, Pos2, Role, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path, id_of};

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
    Close,
}

struct Card;

/// Which handler the popover, the menu and the submenu are given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Told {
    /// `on_dismiss`.
    #[default]
    Nothing,
    /// `on_dismiss_with`, which records why.
    Why,
    /// `on_dismiss_with`, then `on_dismiss`, which replaces it.
    Replaced,
}

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
    told: Told,
    /// The popover holds a button that dismisses it.
    closer: bool,
    whys: Vec<Dismissed>,
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
    let chain = s
        .overlay(Part::Popover)
        .anchor(Anchor::Node(trigger))
        .placement(Placement::below())
        .role(Role::Dialog)
        .named("Popover")
        .dismiss_on(app.popover_dismiss)
        .focus(app.popover_focus.unwrap_or_default());
    let mut chain = closing(chain, app.told, |app| {
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
            if app.closer {
                s.button(Part::Close, "Close").dismisses().add();
            }
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
    let chain = s
        .overlay(Part::Menu)
        .anchor(Anchor::Node(trigger))
        .placement(Placement::below())
        .nav(Nav::Menu(Wrap::Around))
        .role(Role::Menu)
        .named("Menu");
    let mut chain = closing(chain, app.told, |app| {
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
                let chain = s
                    .overlay(Part::Submenu)
                    .anchor(Anchor::Node(more))
                    .placement(Placement::submenu())
                    .nav(Nav::Submenu(Wrap::Around))
                    .role(Role::Menu)
                    .named("More");
                closing(chain, app.told, |app| {
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

/// Gives `chain` the handler `told` says, which runs `close`.
fn closing<'b, 'f, State, Motion>(
    chain: OverlayChain<'b, 'f, App, State, Motion>,
    told: Told,
    close: fn(&mut App),
) -> OverlayChain<'b, 'f, App, State, Motion> {
    let with_why = move |chain: OverlayChain<'b, 'f, App, State, Motion>| {
        chain.on_dismiss_with(move |app: &mut App, why| {
            close(app);
            app.whys.push(why);
        })
    };
    match told {
        Told::Nothing => chain.on_dismiss(close),
        Told::Why => with_why(chain),
        Told::Replaced => with_why(chain).on_dismiss(close),
    }
}

fn list(s: &mut Scope<'_, '_, App>) {
    let trigger = s.id_of(Part::Other);
    s.overlay(Part::List)
        .anchor(Anchor::Node(trigger))
        .focus(Focus::Stay)
        .nav(Nav::Listbox(Wrap::Around))
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

fn harness(app: App) -> Driver<App> {
    let mut d = support::driver(SCREEN, base_text(), app, view).build();
    d.frames(4);
    d
}

fn open_popover(d: &mut Driver<App>) {
    d.click(Target::suffix("/Page/Top/Trigger")).frames(4);
    assert!(d.app().popover, "precondition: open");
}

fn open_menu(d: &mut Driver<App>) {
    d.click(Target::suffix("/Page/Top/MenuTrigger")).frames(4);
    assert!(d.app().menu, "precondition: open");
}

/// Opens the menu with Enter on its trigger, so that focus enters it: a menu
/// opened with the pointer takes none.
fn open_menu_from_the_keyboard(d: &mut Driver<App>) {
    for _ in 0..12 {
        if focused_path(d).as_deref() == Some("/Page/Top/MenuTrigger") {
            break;
        }
        d.press(Key::Tab).frames(3);
    }
    d.press(Key::Enter).frames(3);
    assert!(d.app().menu, "precondition: open");
}

fn log(d: &Driver<App>) -> Vec<String> {
    d.app().log.clone()
}

fn outside_both(d: &Driver<App>, overlay: &str) -> Pos2 {
    let rect = d.rect(Target::suffix(overlay));
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
        let mut d = harness(App::default());
        open_popover(&mut d);
        outside_both(&d, "/Page/Top/Popover");
        d.click(EMPTY).frames(4);
        let app = &d.app();
        assert!(!app.popover);
        assert_eq!(app.log, ["popover dismissed"]);
        assert_eq!(app.page_clicks, 1, "a popover lets the press through");
    }

    #[test]
    fn a_press_inside_dismisses_nothing() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        let inside = d.rect(Target::suffix("/Page/Top/Popover/Box")).center();
        d.click(inside).frames(4);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn the_trigger_toggles_it_and_is_no_dismissal() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        d.click(Target::suffix("/Page/Top/Trigger")).frames(4);
        let app = &d.app();
        assert!(!app.popover, "the trigger's own click closed it");
        assert!(
            app.log.is_empty(),
            "dismiss then reopen would have shown here: {:?}",
            app.log
        );
    }

    #[test]
    fn the_press_that_dismisses_a_popover_can_activate_what_is_under_it() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        d.click(Target::suffix("/Page/Top/Other")).frames(4);
        assert_eq!(log(&d), ["popover dismissed", "other clicked"]);
    }

    #[test]
    fn outside_can_be_overridden_to_swallow() {
        let mut d = harness(App {
            popover_outside: Some(Outside::Swallow),
            ..App::default()
        });
        open_popover(&mut d);
        d.click(Target::suffix("/Page/Top/Other")).frames(4);
        assert_eq!(log(&d), ["popover dismissed"]);
    }

    #[test]
    fn it_is_dismissed_when_its_anchor_leaves_the_frame() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        d.app_mut().hide_trigger = true;
        d.frames(4);
        assert_eq!(log(&d), ["popover dismissed"]);
        assert!(!d.app().popover);
    }

    #[test]
    fn escape_dismisses_it() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        d.press(Key::Escape).frames(3);
        assert_eq!(log(&d), ["popover dismissed"]);
    }

    #[test]
    fn esc_only_ignores_the_press_and_takes_the_key() {
        let mut d = harness(App {
            popover_dismiss: DismissOn::EscOnly,
            ..App::default()
        });
        open_popover(&mut d);
        d.click(EMPTY).frames(4);
        assert!(d.app().popover);
        d.press(Key::Escape).frames(3);
        assert!(!d.app().popover);
    }

    #[test]
    fn never_ignores_both() {
        let mut d = harness(App {
            popover_dismiss: DismissOn::Never,
            ..App::default()
        });
        open_popover(&mut d);
        d.click(EMPTY).frames(4);
        d.press(Key::Escape).frames(3);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn a_press_inside_that_drags_out_and_releases_outside_dismisses_nothing() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        let inside = d.rect(Target::suffix("/Page/Top/Popover/Box")).center();
        d.drag(inside, EMPTY);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn the_window_losing_focus_dismisses_it() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        d.event(Event::WindowFocused(false)).frames(3);
        assert_eq!(log(&d), ["popover dismissed"]);
    }

    /// Sets whether the window has the keyboard focus, as egui-winit does,
    /// with the event it sends alongside.
    fn window_focus(d: &mut Driver<App>, focus: WindowFocus) {
        d.window_focus(focus).frames(3);
        assert_eq!(
            d.ctx().input(|input| input.focused),
            focus == WindowFocus::Focused,
            "precondition: the window's focus"
        );
    }

    /// A screenshot run's window can start without the focus, and still be
    /// told it lost it (#326).
    #[test]
    fn a_window_that_never_had_focus_while_it_was_open_does_not_dismiss_it() {
        let mut d = harness(App::default());
        window_focus(&mut d, WindowFocus::Blurred);
        open_popover(&mut d);
        d.event(Event::WindowFocused(false)).frames(3);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn a_window_that_gains_focus_while_it_is_open_dismisses_it_on_losing_it() {
        let mut d = harness(App::default());
        window_focus(&mut d, WindowFocus::Blurred);
        open_popover(&mut d);
        window_focus(&mut d, WindowFocus::Focused);
        assert!(d.app().popover, "gaining focus dismisses nothing");
        window_focus(&mut d, WindowFocus::Blurred);
        assert_eq!(log(&d), ["popover dismissed"]);
    }

    #[test]
    fn tabbing_out_of_it_dismisses_it() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some("/Page/Top/Popover/Box/A"),
            "precondition: focus entered"
        );
        d.press(egui::KeyboardShortcut::new(
            egui::Modifiers::SHIFT,
            Key::Tab,
        ))
        .frames(3);
        let landed = focused_path(&d).unwrap_or_default();
        assert!(
            landed.starts_with("/Page/") && !landed.starts_with("/Page/Top/Popover"),
            "precondition: focus left for the page: {landed}"
        );
        assert_eq!(log(&d), ["popover dismissed"]);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some(landed.as_str()),
            "where Tab put it stays"
        );
    }

    #[test]
    fn tabbing_within_it_does_not() {
        let mut d = harness(App::default());
        open_popover(&mut d);
        d.press(Key::Tab).frames(3);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Popover/Box/B"));
        assert!(d.app().popover);
    }
}

mod nested {
    use super::*;

    fn open() -> Driver<App> {
        let mut d = harness(App {
            child: true,
            ..App::default()
        });
        open_popover(&mut d);
        assert!(
            support::nodes(&d)
                .iter()
                .any(|(path, _)| path == "/Page/Top/Popover/Box/Child"),
            "precondition: the child is open"
        );
        d
    }

    #[test]
    fn a_press_in_the_child_dismisses_nothing() {
        let mut d = open();
        let child = d
            .rect(Target::suffix("/Page/Top/Popover/Box/Child/Box"))
            .center();
        d.click(child).frames(4);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn a_press_in_the_parent_closes_only_the_child() {
        let mut d = open();
        let parent = d.rect(Target::suffix("/Page/Top/Popover/Box/B")).center();
        assert!(
            !d.rect(Target::suffix("/Page/Top/Popover/Box/Child"))
                .contains(parent),
            "precondition: outside the child"
        );
        d.click(parent).frames(4);
        assert_eq!(log(&d), ["child dismissed"]);
    }

    #[test]
    fn a_press_on_the_page_closes_both_child_first() {
        let mut d = open();
        d.click(EMPTY).frames(4);
        assert_eq!(log(&d), ["child dismissed", "popover dismissed"]);
    }

    #[test]
    fn escape_closes_the_child_and_then_the_parent() {
        let mut d = open();
        d.press(Key::Escape).frames(3);
        assert_eq!(log(&d), ["child dismissed"]);
        d.press(Key::Escape).frames(3);
        assert_eq!(log(&d), ["child dismissed", "popover dismissed"]);
    }
}

mod menu {
    use super::*;

    #[test]
    fn a_press_outside_dismisses_it_and_the_page_never_sees_it() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.click(EMPTY).frames(4);
        let app = &d.app();
        assert_eq!(app.log, ["menu dismissed"]);
        assert_eq!(app.page_clicks, 0, "the menu swallowed the press");
    }

    #[test]
    fn what_is_under_a_menu_does_not_get_the_dismissing_press() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/Other")).frames(4);
        assert_eq!(log(&d), ["menu dismissed"]);
    }

    #[test]
    fn the_trigger_closes_it_and_does_not_reopen_it() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/MenuTrigger")).frames(4);
        assert!(!d.app().menu);
        assert_eq!(log(&d), ["menu dismissed"]);
    }

    #[test]
    fn outside_can_be_overridden_to_pass_through() {
        let mut d = harness(App {
            menu_outside: Some(Outside::PassThrough),
            ..App::default()
        });
        open_menu(&mut d);
        d.click(EMPTY).frames(4);
        let app = &d.app();
        assert_eq!(app.log, ["menu dismissed"]);
        assert_eq!(app.page_clicks, 1);
    }

    #[test]
    fn a_press_on_an_item_chooses_it_and_dismisses_nothing() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/Menu/Box/Item(1)"))
            .frames(4);
        assert_eq!(log(&d), ["Two chosen"]);
        assert!(d.app().menu, "closing is the app's business");
    }

    #[test]
    fn a_press_outside_the_submenu_on_the_page_closes_both_and_stops() {
        let mut d = harness(App {
            submenu: true,
            ..App::default()
        });
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/Other")).frames(4);
        assert_eq!(log(&d), ["submenu dismissed", "menu dismissed"]);
    }

    #[test]
    fn a_press_in_the_menu_closes_only_the_submenu() {
        let mut d = harness(App {
            submenu: true,
            ..App::default()
        });
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/Menu/Box/Item(0)"))
            .frames(4);
        assert_eq!(
            log(&d),
            ["submenu dismissed", "One chosen"],
            "dismissed at the press, chosen at the release"
        );
        assert!(d.app().menu);
    }

    #[test]
    fn menus_are_exclusive_a_popover_opening_dismisses_the_menu() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.app_mut().popover = true;
        d.frames(4);
        assert_eq!(log(&d), ["menu dismissed"]);
        assert!(d.app().popover, "the new overlay stays");
    }

    #[test]
    fn popovers_coexist() {
        let mut d = harness(App {
            child: false,
            ..App::default()
        });
        open_popover(&mut d);
        d.app_mut().dialog = true;
        d.frames(4);
        assert!(d.app().popover && d.app().dialog);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn a_menu_item_that_opens_a_dialog_needs_no_explicit_close() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/Menu/Box/Dialog"))
            .frames(4);
        assert_eq!(log(&d), ["menu dismissed"]);
        assert!(d.app().dialog);
    }

    #[test]
    fn a_submenu_opening_does_not_dismiss_its_menu() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.click(Target::suffix("/Page/Top/Menu/Box/More")).frames(4);
        assert!(d.app().submenu);
        assert!(d.app().menu);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn a_menu_covers_the_page_for_the_pointer_without_drawing() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        let layer = egui::LayerId::new(
            egui::Order::Foreground,
            id_of(&d, "/Page/Top/Menu").with("overlay"),
        );
        assert_eq!(
            d.ctx().layer_id_at(EMPTY),
            Some(layer),
            "its layer is what the pointer meets on the page"
        );
    }
}

mod focus {
    use super::*;

    fn tab_to(d: &mut Driver<App>, path: &str) {
        for _ in 0..12 {
            if focused_path(d).as_deref() == Some(path) {
                return;
            }
            d.press(Key::Tab).frames(3);
        }
        panic!("Tab never reached {path}: {:?}", focused_path(d));
    }

    fn open_from_the_keyboard(d: &mut Driver<App>) {
        tab_to(d, "/Page/Top/Trigger");
        d.press(Key::Enter).frames(3);
        assert!(d.app().popover, "precondition: open");
    }

    #[test]
    fn enter_moves_focus_into_the_overlay_and_escape_gives_it_back_to_the_trigger() {
        let mut d = harness(App::default());
        open_from_the_keyboard(&mut d);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Popover/Box/A"));
        d.press(Key::Escape).frames(3);
        assert!(!d.app().popover);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some("/Page/Top/Trigger"),
            "Esc blurred it inside egui; the library gave it back"
        );
    }

    #[test]
    fn focus_returns_after_a_swallowed_press() {
        let mut d = harness(App::default());
        tab_to(&mut d, "/Page/Top/MenuTrigger");
        d.press(Key::Enter).frames(3);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some("/Page/Top/Menu/Box/Item(0)"),
            "precondition: focus entered the menu"
        );
        d.click(EMPTY).frames(4);
        assert!(!d.app().menu);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/MenuTrigger"));
    }

    #[test]
    fn stay_leaves_focus_on_the_trigger_and_escape_restores_it() {
        let mut d = harness(App {
            popover_focus: Some(Focus::Stay),
            ..App::default()
        });
        open_from_the_keyboard(&mut d);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Trigger"));
        d.press(Key::Escape).frames(3);
        assert!(!d.app().popover);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Trigger"));
    }

    #[test]
    fn none_neither_takes_nor_gives_back_focus() {
        let mut d = harness(App {
            popover_focus: Some(Focus::None),
            ..App::default()
        });
        open_from_the_keyboard(&mut d);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Trigger"));
        d.press(Key::Escape).frames(3);
        assert!(!d.app().popover);
        assert_eq!(focused_path(&d), None);
    }

    #[test]
    fn a_submenu_gives_focus_back_to_the_item_that_opened_it() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        for _ in 0..3 {
            d.press(Key::ArrowDown).frames(3);
        }
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Menu/Box/More"));
        d.press(Key::Enter).frames(3);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some("/Page/Top/Menu/Box/Submenu/Box/SubItem(0)"),
            "focus entered the submenu"
        );
        d.press(Key::Escape).frames(3);
        assert_eq!(log(&d), ["submenu dismissed"]);
        assert_eq!(focused_path(&d).as_deref(), Some("/Page/Top/Menu/Box/More"));
    }
}

mod keys {
    use super::*;

    fn focused_item(d: &Driver<App>) -> String {
        focused_path(d).unwrap_or_default()
    }

    const ONE: &str = "/Page/Top/Menu/Box/Item(0)";
    const TWO: &str = "/Page/Top/Menu/Box/Item(1)";
    const THREE: &str = "/Page/Top/Menu/Box/Item(2)";
    const DIALOG: &str = "/Page/Top/Menu/Box/Dialog";

    #[test]
    fn a_menu_opens_with_its_first_item_focused() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        assert_eq!(focused_item(&d), ONE);
    }

    #[test]
    fn down_and_up_move_between_items_and_wrap() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_item(&d), TWO);
        d.press(Key::ArrowUp).frames(3);
        assert_eq!(focused_item(&d), ONE);
        d.press(Key::ArrowUp).frames(3);
        assert_eq!(focused_item(&d), DIALOG, "wraps to the last");
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_item(&d), ONE, "wraps to the first");
    }

    #[test]
    fn home_and_end_go_to_the_ends() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        d.press(Key::End).frames(3);
        assert_eq!(focused_item(&d), DIALOG);
        d.press(Key::Home).frames(3);
        assert_eq!(focused_item(&d), ONE);
    }

    #[test]
    fn a_disabled_item_is_skipped() {
        let mut d = harness(App {
            disabled_item: Some(1),
            ..App::default()
        });
        open_menu_from_the_keyboard(&mut d);
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_item(&d), THREE);
        d.press(Key::ArrowUp).frames(3);
        assert_eq!(focused_item(&d), ONE);
    }

    #[test]
    fn the_arrows_do_not_leave_the_menu_for_the_page() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        for _ in 0..8 {
            d.press(Key::ArrowDown).frames(3);
            assert!(
                focused_item(&d).starts_with("/Page/Top/Menu/"),
                "{}",
                focused_item(&d)
            );
        }
    }

    #[test]
    fn enter_chooses_the_focused_item() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        d.press(Key::ArrowDown).frames(3);
        d.press(Key::Enter).frames(3);
        assert_eq!(log(&d), ["Two chosen"]);
    }

    #[test]
    fn the_keys_belong_to_the_submenu_while_focus_is_in_it() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        for _ in 0..4 {
            if focused_item(&d) == "/Page/Top/Menu/Box/More" {
                break;
            }
            d.press(Key::ArrowDown).frames(3);
        }
        d.press(Key::Enter).frames(3);
        let sub = |index: u8| format!("/Page/Top/Menu/Box/Submenu/Box/SubItem({index})");
        assert_eq!(focused_item(&d), sub(0));
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_item(&d), sub(1));
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(focused_item(&d), sub(0), "wraps inside the submenu");
    }

    #[test]
    fn left_closes_the_submenu_and_focuses_its_item() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        d.click(Target::suffix("/Page/Top/Menu/Box/More")).frames(4);
        d.press(Key::ArrowLeft).frames(3);
        assert_eq!(log(&d), ["submenu dismissed"]);
        assert!(d.app().menu, "the parent stays");
        assert_eq!(focused_item(&d), "/Page/Top/Menu/Box/More");
    }

    #[test]
    fn left_in_a_menu_that_is_not_a_submenu_does_nothing() {
        let mut d = harness(App::default());
        open_menu_from_the_keyboard(&mut d);
        d.press(Key::ArrowLeft).frames(3);
        assert!(d.app().menu);
        assert_eq!(focused_item(&d), ONE);
    }
}

mod listbox {
    use super::*;

    fn path(index: u8) -> String {
        format!("/Page/Top/List/Box/Option({index})")
    }

    fn open() -> Driver<App> {
        let mut d = harness(App::default());
        support::drive_past_bar(&mut d);
        for _ in 0..3 {
            d.press(Key::Tab);
        }
        d.frames(3);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some("/Page/Top/Other"),
            "precondition: the trigger has focus"
        );
        d.app_mut().list = true;
        d.frames(4);
        d
    }

    fn highlighted(d: &Driver<App>) -> Vec<u8> {
        (0..3)
            .filter(|&index| support::fill_of(d, &path(index)) == Some(Color32::LIGHT_BLUE))
            .collect()
    }

    #[test]
    fn focus_stays_on_the_trigger_and_down_highlights_the_first_option() {
        let mut d = open();
        assert!(highlighted(&d).is_empty());
        d.press(Key::ArrowDown).frames(3);
        assert_eq!(highlighted(&d), [0]);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some("/Page/Top/Other"),
            "the highlight is virtual"
        );
    }

    #[test]
    fn the_highlight_skips_disabled_options_and_wraps() {
        let mut d = open();
        for (key, expected) in [
            (Key::ArrowDown, 0),
            (Key::ArrowDown, 2),
            (Key::ArrowDown, 0),
            (Key::ArrowUp, 2),
            (Key::Home, 0),
            (Key::End, 2),
        ] {
            d.press(key).frames(3);
            assert_eq!(highlighted(&d), [expected], "after {key:?}");
        }
    }

    #[test]
    fn enter_activates_the_highlighted_option() {
        let mut d = open();
        d.press(Key::ArrowDown);
        d.press(Key::ArrowDown).frames(3);
        d.press(Key::Enter).frames(3);
        assert_eq!(log(&d), ["option 2 chosen"]);
    }

    #[test]
    fn once_it_is_closed_enter_is_the_triggers_again() {
        let mut d = open();
        d.press(Key::ArrowDown).frames(3);
        d.press(Key::Escape).frames(3);
        assert!(!d.app().list);
        d.press(Key::Enter).frames(3);
        assert_eq!(log(&d), ["list dismissed", "other clicked"]);
    }

    #[test]
    fn enter_with_nothing_highlighted_activates_nothing() {
        let mut d = open();
        d.press(Key::Enter).frames(3);
        assert!(!log(&d).iter().any(|entry| entry.contains("chosen")));
    }

    #[test]
    fn the_trigger_is_inside_so_its_press_is_no_dismissal() {
        let mut d = open();
        d.click(Target::suffix("/Page/Top/Other")).frames(4);
        assert!(!log(&d).contains(&"list dismissed".to_owned()));
    }
}

mod drag_and_drop {
    use super::*;

    fn centre(d: &Driver<App>, path: &str) -> Pos2 {
        d.rect(Target::suffix(path)).center()
    }

    fn with_popover(dismiss_on: DismissOn) -> Driver<App> {
        let mut d = harness(App {
            popover: true,
            popover_dismiss: dismiss_on,
            ..App::default()
        });
        d.frames(4);
        d
    }

    #[test]
    fn a_target_under_a_popover_is_unreachable() {
        let mut open = with_popover(DismissOn::Never);
        let zone = open.rect(Target::suffix("/Page/Dnd/Zone"));
        let popover = open.rect(Target::suffix("/Page/Top/Popover/Box"));
        let target = pos2(zone.min.x + 3.0, zone.center().y);
        assert!(
            zone.contains(target)
                && popover.contains(target)
                && !open
                    .rect(Target::suffix("/Page/Top/Popover/Box/PopZone"))
                    .contains(target),
            "precondition: the point is on the zone under the popover"
        );
        let source = centre(&open, "/Page/Dnd/Source");
        assert!(
            !popover.contains(source),
            "precondition: the source is clear of the popover"
        );
        open.drag(source, target);
        assert!(open.app().drops.is_empty());

        let mut closed = harness(App::default());
        closed.drag(source, target);
        assert_eq!(
            closed.app().drops,
            ["zone"],
            "the control: with no popover the same drag lands"
        );
    }

    #[test]
    fn a_target_inside_a_popover_is_reachable() {
        let mut d = with_popover(DismissOn::Never);
        let source = centre(&d, "/Page/Dnd/Source");
        let target = centre(&d, "/Page/Top/Popover/Box/PopZone");
        d.drag(source, target);
        assert_eq!(d.app().drops, ["popover zone"]);
    }

    #[test]
    fn a_drag_from_a_popover_to_the_page_lands_and_dismisses_nothing() {
        let mut d = with_popover(DismissOn::Default);
        let source = centre(&d, "/Page/Top/Popover/Box/PopSource");
        let target = centre(&d, "/Page/Dnd/Zone2");
        assert!(
            !d.rect(Target::suffix("/Page/Top/Popover/Box"))
                .contains(target),
            "precondition: the target is on the page"
        );
        d.drag(source, target);
        assert_eq!(d.app().drops, ["zone2"]);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }
}

mod accessibility {
    use super::*;

    #[test]
    fn the_overlay_has_its_role_and_name() {
        let mut d = harness(App::default());
        open_menu(&mut d);
        d.harness().get_by_role_and_label(Role::Menu, "Menu");
        d.harness().get_by_role_and_label(Role::MenuItem, "One");
    }

    #[test]
    fn the_trigger_says_its_popup_is_open() {
        let mut d = harness(App::default());
        let trigger = |d: &Driver<App>| {
            let node = d.harness().get_by_role_and_label(Role::Button, "Menu");
            let data = node.accesskit_node().data().clone();
            (data.is_expanded(), data.has_popup())
        };
        assert_eq!(trigger(&d), (None, None), "closed: its widget's business");
        open_menu(&mut d);
        assert_eq!(
            trigger(&d),
            (Some(true), Some(egui::accesskit::HasPopup::Menu))
        );
    }
}

mod egui_facts {
    //! What the design relies on in egui itself: kept as tests so that a new
    //! egui that changes it is noticed.

    use egui::{Modifiers, PointerButton, Ui};
    use egui_kittest::Harness;

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

/// `on_dismiss_with` is told why, and `DismissOn::Outside` (#326).
mod dismissed {
    use super::*;

    fn told() -> App {
        App {
            told: Told::Why,
            ..App::default()
        }
    }

    fn whys(d: &Driver<App>) -> Vec<Dismissed> {
        d.app().whys.clone()
    }

    /// Shift+Tab from the popover's first button, which takes focus out to
    /// the page.
    fn tab_out_of_the_popover(d: &mut Driver<App>) {
        assert_eq!(
            focused_path(d).as_deref(),
            Some("/Page/Top/Popover/Box/A"),
            "precondition: focus entered"
        );
        d.press(egui::KeyboardShortcut::new(
            egui::Modifiers::SHIFT,
            Key::Tab,
        ))
        .frames(3);
        let landed = focused_path(d).unwrap_or_default();
        assert!(
            landed.starts_with("/Page/") && !landed.starts_with("/Page/Top/Popover"),
            "precondition: focus left for the page: {landed}"
        );
    }

    fn blur_the_window(d: &mut Driver<App>) {
        assert!(
            d.ctx().input(|input| input.focused),
            "precondition: the window has had focus while the popover was open"
        );
        d.event(Event::WindowFocused(false)).frames(3);
    }

    #[test]
    fn a_press_outside() {
        let mut d = harness(told());
        open_popover(&mut d);
        outside_both(&d, "/Page/Top/Popover");
        d.click(EMPTY).frames(4);
        assert_eq!(whys(&d), [Dismissed::Outside]);
        assert_eq!(log(&d), ["popover dismissed"]);
    }

    #[test]
    fn escape() {
        let mut d = harness(told());
        open_popover(&mut d);
        d.press(Key::Escape).frames(3);
        assert_eq!(whys(&d), [Dismissed::Esc]);
    }

    #[test]
    fn focus_leaving() {
        let mut d = harness(told());
        open_popover(&mut d);
        tab_out_of_the_popover(&mut d);
        assert_eq!(whys(&d), [Dismissed::FocusOut]);
    }

    #[test]
    fn the_window_losing_focus() {
        let mut d = harness(told());
        open_popover(&mut d);
        blur_the_window(&mut d);
        assert_eq!(whys(&d), [Dismissed::WindowBlur]);
    }

    #[test]
    fn the_anchor_leaving_the_frame() {
        let mut d = harness(told());
        open_popover(&mut d);
        d.app_mut().hide_trigger = true;
        d.frames(4);
        assert_eq!(whys(&d), [Dismissed::AnchorGone]);
    }

    #[test]
    fn another_overlay_opening_over_a_menu() {
        let mut d = harness(told());
        open_menu(&mut d);
        d.app_mut().popover = true;
        d.frames(4);
        assert_eq!(log(&d), ["menu dismissed"]);
        assert_eq!(whys(&d), [Dismissed::Exclusive]);
    }

    #[test]
    fn a_menus_navigation_keys() {
        let mut d = harness(told());
        open_menu_from_the_keyboard(&mut d);
        d.click(Target::suffix("/Page/Top/Menu/Box/More")).frames(4);
        assert!(d.app().submenu, "precondition: the submenu is open");
        d.press(Key::ArrowLeft).frames(3);
        assert_eq!(log(&d), ["submenu dismissed"]);
        assert_eq!(whys(&d), [Dismissed::Keys]);
    }

    #[test]
    fn a_button_that_dismisses() {
        let mut d = harness(App {
            closer: true,
            ..told()
        });
        open_popover(&mut d);
        d.click(Target::suffix("/Page/Top/Popover/Box/Close"))
            .frames(4);
        assert_eq!(whys(&d), [Dismissed::Closed]);
        assert!(!d.app().popover);
    }

    #[test]
    fn on_dismiss_set_last_replaces_on_dismiss_with() {
        let mut d = harness(App {
            told: Told::Replaced,
            ..App::default()
        });
        open_popover(&mut d);
        d.press(Key::Escape).frames(3);
        assert_eq!(log(&d), ["popover dismissed"]);
        assert!(whys(&d).is_empty(), "the handler that records why is gone");
    }

    fn outside_only() -> App {
        App {
            popover_dismiss: DismissOn::Outside,
            ..told()
        }
    }

    #[test]
    fn outside_ignores_focus_leaving() {
        let mut d = harness(outside_only());
        open_popover(&mut d);
        tab_out_of_the_popover(&mut d);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn outside_ignores_the_window_losing_focus() {
        let mut d = harness(outside_only());
        open_popover(&mut d);
        blur_the_window(&mut d);
        assert!(d.app().popover);
        assert!(log(&d).is_empty());
    }

    #[test]
    fn outside_still_closes_on_a_press_outside_after_focus_left() {
        let mut d = harness(outside_only());
        open_popover(&mut d);
        tab_out_of_the_popover(&mut d);
        assert!(d.app().popover, "precondition: still open");
        outside_both(&d, "/Page/Top/Popover");
        d.click(EMPTY).frames(4);
        assert_eq!(whys(&d), [Dismissed::Outside]);
        assert!(!d.app().popover);
    }

    #[test]
    fn outside_still_closes_on_escape() {
        let mut d = harness(outside_only());
        open_popover(&mut d);
        d.press(Key::Escape).frames(3);
        assert_eq!(whys(&d), [Dismissed::Esc]);
        assert!(!d.app().popover);
    }
}
