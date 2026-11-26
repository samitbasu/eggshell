//! App commands through the real path: key events dispatched by
//! `egui_kittest::Harness`, build, layout, draw and the dispatcher that runs
//! after the library has claimed Esc and Return.

use eggshell_ui::overlay::{Anchor, Modal};
use eggshell_ui::owned::text_field::Field;
use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{
    Builder, Chord, Command, CommandState, Commands, Icon, InText, Shell, TextSpec, TextStyle,
    Weight, tw,
};
use egui::{Color32, FontFamily, Key, Modifiers, Role, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path};

const NAME: &str = r#"/"Page"/"name""#;
const FILE: &str = r#"/"Page"/"file""#;
const MENU: &str = r#"/"Page"/("file", Menu)/Box"#;
const VIEW: &str = r#"/"Page"/"view""#;
const VIEW_MENU: &str = r#"/"Page"/("view", Menu)/Box"#;

#[derive(Hash, Debug, Clone, Copy)]
enum Cmd {
    Save,
    SaveAs,
    Copy,
    Paste,
    Open,
    Help,
    Jump,
    Confirm,
    Local,
    Shadow,
    Grid,
    Small,
    Large,
    Shelled,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Size {
    #[default]
    Small,
    Large,
}

#[derive(Default)]
struct App {
    log: Vec<&'static str>,
    can_save: bool,
    with_save_as: bool,
    dialog: bool,
    name: String,
    canvas_saw_s: u32,
    table: Option<fn() -> Commands<App>>,
    twice: bool,
    dialog_item: Option<Cmd>,
    popover: bool,
    button: Option<Cmd>,
    view_menu: bool,
    grid: bool,
    size: Size,
    shell_width: Option<f32>,
}

const fn chord(modifiers: Modifiers, key: Key) -> Chord {
    Chord::new(modifiers, key).expect("not a chord the library keeps")
}

const SAVE: Chord = chord(Modifiers::COMMAND, Key::S);
const SAVE_AS: Chord = chord(
    Modifiers {
        shift: true,
        ..Modifiers::COMMAND
    },
    Key::S,
);

fn page_commands(app: &App) -> Commands<App> {
    let table = Commands::new()
        .add(
            Cmd::Save,
            Command::new("Save", |app: &mut App| app.log.push("save"))
                .shortcut(SAVE)
                .enabled(|app| app.can_save),
        )
        .add(
            Cmd::Copy,
            Command::new("Copy", |app: &mut App| app.log.push("copy"))
                .shortcut(chord(Modifiers::COMMAND, Key::C)),
        )
        .add(
            Cmd::Paste,
            Command::new("Paste", |app: &mut App| app.log.push("paste"))
                .shortcut(chord(Modifiers::COMMAND, Key::V))
                .in_text(InText::Fire),
        )
        .add(
            Cmd::Open,
            Command::new("Open", |app: &mut App| app.log.push("open"))
                .shortcut(chord(Modifiers::COMMAND, Key::O))
                .in_text(InText::Yield),
        )
        .add(
            Cmd::Help,
            Command::new("Help", |app: &mut App| app.log.push("help"))
                .shortcut(chord(Modifiers::NONE, Key::H)),
        )
        .add(
            Cmd::Jump,
            Command::new("Jump", |app: &mut App| app.log.push("jump"))
                .shortcut(chord(Modifiers::NONE, Key::J))
                .in_text(InText::Fire),
        )
        .add(
            Cmd::Grid,
            Command::new("Grid", |app: &mut App| app.grid = !app.grid)
                .checked(|app| Mark::from(app.grid)),
        )
        .add(
            Cmd::Small,
            Command::new("Small", |app: &mut App| app.size = Size::Small)
                .chosen(|app| Mark::from(app.size == Size::Small)),
        )
        .add(
            Cmd::Large,
            Command::new("Large", |app: &mut App| app.size = Size::Large)
                .chosen(|app| Mark::from(app.size == Size::Large)),
        )
        .add(
            Cmd::Shelled,
            Command::with_shell("Shelled", |app: &mut App, shell: &Shell| {
                app.log.push("shelled");
                app.shell_width = Some(shell.ctx().content_rect().width());
            })
            .shortcut(chord(Modifiers::COMMAND, Key::K)),
        );
    if app.with_save_as {
        table.add(
            Cmd::SaveAs,
            Command::new("Save as", |app: &mut App| app.log.push("save as")).shortcut(SAVE_AS),
        )
    } else {
        table
    }
}

fn dialog_commands() -> Commands<App> {
    Commands::new().add(
        Cmd::Confirm,
        Command::new("Confirm", |app: &mut App| app.log.push("confirm"))
            .shortcut(chord(Modifiers::COMMAND, Key::Enter)),
    )
}

fn popover_commands() -> Commands<App> {
    Commands::new()
        .add(
            Cmd::Local,
            Command::new("Local", |app: &mut App| app.log.push("local"))
                .shortcut(chord(Modifiers::COMMAND, Key::L)),
        )
        .add(
            Cmd::Shadow,
            Command::new("Shadow", |app: &mut App| app.log.push("shadow"))
                .shortcut(chord(Modifiers::COMMAND, Key::C)),
        )
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

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    let table = app
        .table
        .map_or_else(|| page_commands(app), |table| table());
    b.commands(&table, app);
    if app.twice {
        b.commands(&table, app);
    }
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.text_field_with("name", Field::singleline(&app.name).named("Name"))
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.name = text);
        s.dropdown_menu("file", "File").items(|m| {
            m.command(Cmd::Save).add();
            m.command(Cmd::Copy).add();
        });
        if let Some(command) = app.button {
            s.button("run", "Run").command(command);
        }
        if app.view_menu {
            s.dropdown_menu("view", "View").items(|m| {
                m.command(Cmd::Grid).icon(icon()).add();
                m.command(Cmd::Small).add();
                m.command(Cmd::Large).add();
                m.command(Cmd::Shelled).inset().add();
                m.command(Cmd::Copy).enablement(Enablement::Disabled).add();
                m.command(Cmd::Save).enablement(Enablement::Enabled).add();
            });
        }
        s.div_id("canvas", tw().w_px(160.0).h_px(60.0), |_| {})
            .draw(|app: &mut App, ui| {
                if ui.input(|input| input.key_pressed(Key::S)) {
                    app.canvas_saw_s += 1;
                }
            });
    });
    if app.popover {
        b.overlay("popover").anchor(Anchor::Screen).children(|s| {
            s.commands(&popover_commands(), app);
            s.div_id(
                "Pop",
                tw().w_px(100.0).h_px(50.0).bg(Color32::WHITE),
                |_| {},
            );
        });
    }
    if app.dialog {
        b.overlay("dialog")
            .modal(Modal::Center)
            .on_dismiss(|app: &mut App| app.dialog = false)
            .children(|s| {
                s.commands(&dialog_commands(), app);
                s.div_id(
                    "Box",
                    tw().w_px(200.0).h_px(100.0).bg(Color32::WHITE),
                    |s| {
                        if let Some(command) = app.dialog_item {
                            s.dropdown_menu("dialog menu", "Dialog menu")
                                .items(move |m| m.command(command).add());
                        }
                    },
                );
            });
    }
}

fn icon() -> Icon {
    Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
        egui::TextureId::default(),
        vec2(4.0, 4.0),
    )))
}

fn harness_of(app: App) -> Driver<App> {
    let mut d = support::driver(vec2(500.0, 400.0), base_text(), app, view).build();
    d.frames(4);
    d
}

fn harness() -> Driver<App> {
    harness_of(App {
        can_save: true,
        ..App::default()
    })
}

/// `key` with `modifiers` held, sent as a chord even with none held.
fn press(d: &mut Driver<App>, modifiers: Modifiers, key: Key) {
    d.harness().key_press_modifiers(modifiers, key);
    d.frames(4);
}

fn log(d: &Driver<App>) -> Vec<&'static str> {
    d.app().log.clone()
}

fn start_typing(d: &mut Driver<App>) {
    d.click(Target::suffix(NAME)).frames(6);
    assert_eq!(
        focused_path(d).as_deref(),
        Some("<widget>"),
        "precondition: the field has focus"
    );
}

#[test]
fn a_chord_runs_its_command() {
    let mut d = harness();
    press(&mut d, Modifiers::COMMAND, Key::S);
    assert_eq!(log(&d), ["save"]);
}

#[test]
fn shift_command_s_does_not_run_the_command_bound_to_command_s() {
    let mut d = harness();
    press(&mut d, Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    assert!(log(&d).is_empty(), "{:?}", log(&d));
}

#[test]
fn shift_command_s_runs_its_own_command_and_not_command_s() {
    let mut d = harness_of(App {
        can_save: true,
        with_save_as: true,
        ..App::default()
    });
    press(&mut d, Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    assert_eq!(log(&d), ["save as"]);
    press(&mut d, Modifiers::COMMAND, Key::S);
    assert_eq!(log(&d), ["save as", "save"]);
}

#[test]
fn a_chord_a_command_takes_is_not_seen_by_a_custom_node() {
    let mut d = harness();
    press(&mut d, Modifiers::COMMAND, Key::S);
    assert_eq!(log(&d), ["save"], "precondition: the command ran");
    assert_eq!(d.app().canvas_saw_s, 0);
}

#[test]
fn a_disabled_command_does_not_run_and_leaves_its_chord_to_the_rest() {
    let mut d = harness_of(App::default());
    press(&mut d, Modifiers::COMMAND, Key::S);
    assert!(log(&d).is_empty());
    assert_eq!(d.app().canvas_saw_s, 1, "the chord was not consumed");
}

#[test]
fn copy_is_a_press_of_command_c() {
    let mut d = harness();
    d.event(egui::Event::Copy).frames(3);
    assert_eq!(log(&d), ["copy"]);
}

#[test]
fn paste_is_a_press_of_command_v_where_it_fires_in_a_text_field() {
    let mut d = harness();
    start_typing(&mut d);
    d.event(egui::Event::Paste("text".into())).frames(3);
    assert_eq!(log(&d), ["paste"]);
}

#[test]
fn a_chord_that_is_not_pressed_exactly_does_not_run() {
    let mut d = harness();
    press(&mut d, Modifiers::NONE, Key::S);
    press(&mut d, Modifiers::ALT | Modifiers::COMMAND, Key::S);
    assert!(log(&d).is_empty(), "{:?}", log(&d));
}

mod claimed {
    use eggshell_ui::web::ClaimedChords;
    use egui::KeyboardShortcut;

    use super::*;

    fn claims(d: &Driver<App>, modifiers: Modifiers, key: Key) -> bool {
        ClaimedChords::of(d.ctx()).claims(KeyboardShortcut::new(modifiers, key))
    }

    #[test]
    fn an_enabled_commands_chord_is_claimed_and_no_other() {
        let d = harness();
        assert!(claims(&d, Modifiers::COMMAND, Key::S));
        assert!(claims(&d, Modifiers::MAC_CMD | Modifiers::COMMAND, Key::S));
        assert!(!claims(&d, Modifiers::COMMAND, Key::G));
        assert!(!claims(&d, Modifiers::COMMAND | Modifiers::SHIFT, Key::S));
    }

    #[test]
    fn a_disabled_commands_chord_is_not_claimed() {
        let mut d = harness();
        assert!(
            claims(&d, Modifiers::COMMAND, Key::S),
            "precondition: claimed while enabled"
        );
        d.app_mut().can_save = false;
        d.frames(2);
        assert!(!claims(&d, Modifiers::COMMAND, Key::S));
        assert!(claims(&d, Modifiers::COMMAND, Key::K));
    }

    #[test]
    fn a_chord_that_yields_to_a_text_field_is_not_claimed_while_typing() {
        let mut d = harness();
        assert!(
            claims(&d, Modifiers::COMMAND, Key::O),
            "precondition: claimed before typing"
        );
        start_typing(&mut d);
        assert!(!claims(&d, Modifiers::COMMAND, Key::O));
        assert!(claims(&d, Modifiers::NONE, Key::J));
    }

    #[test]
    fn copy_and_paste_are_left_to_the_browser() {
        let d = harness();
        assert!(!claims(&d, Modifiers::COMMAND, Key::C));
        assert!(!claims(&d, Modifiers::COMMAND, Key::V));
    }

    #[test]
    fn under_a_modal_only_its_own_chords_are_claimed() {
        let mut d = harness();
        d.app_mut().dialog = true;
        d.frames(12);
        assert!(claims(&d, Modifiers::COMMAND, Key::Enter));
        assert!(!claims(&d, Modifiers::COMMAND, Key::S));
    }
}

mod modals {
    use super::*;

    fn opened() -> Driver<App> {
        let mut d = harness();
        d.app_mut().dialog = true;
        d.frames(12);
        assert!(
            support::nodes(&d)
                .iter()
                .any(|(path, _)| path.starts_with(r#"/"dialog""#)),
            "precondition: the dialog is open"
        );
        d
    }

    #[test]
    fn a_page_command_does_not_run_under_a_modal() {
        let mut d = opened();
        press(&mut d, Modifiers::COMMAND, Key::S);
        assert!(log(&d).is_empty(), "{:?}", log(&d));
    }

    #[test]
    fn the_modals_own_commands_run() {
        let mut d = opened();
        press(&mut d, Modifiers::COMMAND, Key::Enter);
        assert_eq!(log(&d), ["confirm"]);
    }

    fn open_dialog_menu(item: Cmd) -> Driver<App> {
        let mut d = harness_of(App {
            can_save: true,
            dialog: true,
            dialog_item: Some(item),
            ..App::default()
        });
        d.frames(12);
        d.harness()
            .get_by_role_and_label(Role::Button, "Dialog menu")
            .click();
        d.frames(6);
        d
    }

    #[test]
    fn a_menu_in_a_dialog_offers_the_dialogs_commands() {
        let mut d = open_dialog_menu(Cmd::Confirm);
        d.harness()
            .get_by_role_and_label(Role::MenuItem, "Confirm")
            .click();
        d.frames(6);
        assert_eq!(log(&d), ["confirm"]);
    }

    #[test]
    fn a_menu_in_a_dialog_does_not_offer_a_page_command_whose_chord_is_dead() {
        let mut d = open_dialog_menu(Cmd::Save);
        let report = d.expect_reported(eggshell_ui::Problem::UnknownCommand);
        assert!(report.message.contains("Save"), "{}", report.message);
        let item = d.harness().get_by_role_and_label(Role::MenuItem, "Save");
        assert!(item.accesskit_node().data().is_disabled());
        item.click();
        d.frames(6);
        assert!(log(&d).is_empty(), "{:?}", log(&d));
    }

    #[test]
    fn the_modals_commands_do_not_run_once_it_is_closed() {
        let mut d = opened();
        d.app_mut().dialog = false;
        d.frames(30);
        press(&mut d, Modifiers::COMMAND, Key::Enter);
        assert!(log(&d).is_empty(), "{:?}", log(&d));
        press(&mut d, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&d), ["save"], "the page's commands are back");
    }
}

mod popovers {
    use super::*;

    fn opened() -> Driver<App> {
        let mut d = harness();
        d.app_mut().popover = true;
        d.frames(12);
        assert!(
            support::nodes(&d)
                .iter()
                .any(|(path, _)| path.starts_with(r#"/"popover""#)),
            "precondition: the popover is open"
        );
        d
    }

    #[test]
    fn a_popovers_commands_run_beside_the_pages() {
        let mut d = opened();
        press(&mut d, Modifiers::COMMAND, Key::L);
        press(&mut d, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&d), ["local", "save"]);
    }

    #[test]
    fn a_chord_in_both_tables_runs_the_nearest_command_only() {
        let mut d = opened();
        press(&mut d, Modifiers::COMMAND, Key::C);
        assert_eq!(log(&d), ["shadow"]);
    }
}

mod text_fields {
    use super::*;

    #[test]
    fn a_command_chord_runs_in_a_text_field() {
        let mut d = harness();
        start_typing(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&d), ["save"]);
    }

    #[test]
    fn a_chord_a_text_field_uses_does_not_run() {
        let mut d = harness();
        start_typing(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::C);
        assert!(log(&d).is_empty(), "{:?}", log(&d));
    }

    #[test]
    fn a_bare_key_does_not_run_while_typing_and_does_otherwise() {
        let mut d = harness();
        start_typing(&mut d);
        press(&mut d, Modifiers::NONE, Key::H);
        assert!(log(&d).is_empty(), "typing");
        d.press(Key::Escape).frames(5);
        assert_eq!(focused_path(&d), None, "precondition: no focus");
        press(&mut d, Modifiers::NONE, Key::H);
        assert_eq!(log(&d), ["help"]);
    }

    #[test]
    fn fire_overrides_a_chord_a_text_field_uses() {
        let mut d = harness();
        start_typing(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::V);
        press(&mut d, Modifiers::NONE, Key::J);
        assert_eq!(log(&d), ["paste", "jump"]);
    }

    #[test]
    fn yield_overrides_a_chord_a_text_field_leaves_alone() {
        let mut d = harness();
        start_typing(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::O);
        assert!(log(&d).is_empty(), "typing");
        d.press(Key::Escape).frames(5);
        press(&mut d, Modifiers::COMMAND, Key::O);
        assert_eq!(log(&d), ["open"]);
    }
}

mod menus {
    use super::*;

    fn open_file(d: &mut Driver<App>) {
        d.click(Target::suffix(FILE)).frames(5);
        assert!(
            support::nodes(d).iter().any(|(path, _)| path == MENU),
            "precondition: the menu is open"
        );
    }

    #[test]
    fn an_item_takes_its_title_and_the_shortcut_as_the_platform_writes_it() {
        let mut d = harness();
        open_file(&mut d);
        d.harness().get_by_role_and_label(Role::MenuItem, "Save");
        let written = d.ctx().format_shortcut(&SAVE.into());
        let menu = d.rect(Target::suffix(MENU));
        let shown = support::shapes(&d).into_iter().any(|shape| {
            matches!(shape, egui::Shape::Text(text)
                if text.galley.text() == written && menu.contains(text.pos))
        });
        assert!(shown, "{written:?} is not in the menu");
    }

    #[test]
    fn a_menu_item_shows_on_a_mac_with_the_shortcut_in_symbols() {
        let mut d = harness();
        d.ctx().set_os(egui::os::OperatingSystem::Mac);
        open_file(&mut d);
        let written = d.ctx().format_shortcut(&SAVE.into());
        assert!(
            written.contains('\u{2318}'),
            "{written:?} is not in symbols"
        );
        let menu = d.rect(Target::suffix(MENU));
        let shown = support::shapes(&d).into_iter().any(|shape| {
            matches!(shape, egui::Shape::Text(text)
                if text.galley.text() == written && menu.contains(text.pos))
        });
        assert!(shown, "{written:?} is not in the menu");
    }

    #[test]
    fn a_key_chip_draws_the_mac_modifier_symbols() {
        let chip = "\u{2318}\u{21e7}S";
        let mut d = support::driver(vec2(200.0, 60.0), base_text(), (), |b, _| {
            b.kbd("\u{2318}\u{21e7}S")
        })
        .build();
        d.ctx().set_os(egui::os::OperatingSystem::Mac);
        d.frames(3);
        let drawn = support::shapes(&d)
            .into_iter()
            .any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == chip));
        assert!(drawn, "{chip:?} is not drawn");
    }

    #[test]
    fn every_key_symbol_has_a_glyph_in_the_proportional_family() {
        let d = harness();
        let missing: Vec<char> = "\u{2318}\u{21e7}\u{2325}\u{2303}\u{232b}\u{21b5}\u{21e5}\u{2190}\u{2191}\u{2192}\u{2193}\u{23ce}"
            .chars()
            .filter(|symbol| {
                let font = egui::FontId::proportional(14.0);
                !d
                    .ctx()
                    .fonts_mut(|fonts| fonts.has_glyphs(&font, &symbol.to_string()))
            })
            .collect();
        assert!(missing.is_empty(), "no glyph for {missing:?}");
    }

    #[test]
    fn an_item_is_disabled_with_its_command() {
        let mut d = harness_of(App::default());
        open_file(&mut d);
        let disabled = |d: &Driver<App>, label| {
            d.harness()
                .get_by_role_and_label(Role::MenuItem, label)
                .accesskit_node()
                .data()
                .is_disabled()
        };
        assert!(disabled(&d, "Save"));
        assert!(!disabled(&d, "Copy"));
    }

    #[test]
    fn activating_an_item_runs_the_command() {
        let mut d = harness();
        open_file(&mut d);
        d.click(Target::suffix(&format!("{MENU}/Copy"))).frames(5);
        assert_eq!(log(&d), ["copy"]);
    }

    #[test]
    fn a_chord_runs_its_command_under_an_open_menu() {
        let mut d = harness();
        open_file(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&d), ["save"]);
    }
}

mod command_items {
    use super::*;

    fn open_view(app: App) -> Driver<App> {
        let mut d = harness_of(App {
            view_menu: true,
            ..app
        });
        d.click(Target::suffix(VIEW)).frames(5);
        assert!(
            support::nodes(&d).iter().any(|(path, _)| path == VIEW_MENU),
            "precondition: the menu is open"
        );
        d
    }

    fn toggled(d: &Driver<App>, role: Role, label: &str) -> Option<egui::accesskit::Toggled> {
        d.harness()
            .get_by_role_and_label(role, label)
            .accesskit_node()
            .data()
            .toggled()
    }

    fn disabled(d: &Driver<App>, label: &str) -> bool {
        d.harness()
            .query_by_label(label)
            .expect("the item is in the menu")
            .accesskit_node()
            .data()
            .is_disabled()
    }

    #[test]
    fn a_checked_commands_item_is_a_checkbox_marked_as_the_app_has_it() {
        use egui::accesskit::Toggled;
        let off = open_view(App::default());
        assert_eq!(
            toggled(&off, Role::MenuItemCheckBox, "Grid"),
            Some(Toggled::False)
        );
        let on = open_view(App {
            grid: true,
            ..App::default()
        });
        assert_eq!(
            toggled(&on, Role::MenuItemCheckBox, "Grid"),
            Some(Toggled::True)
        );
    }

    #[test]
    fn an_icon_does_not_replace_a_checked_commands_mark() {
        let d = open_view(App {
            grid: true,
            ..App::default()
        });
        let item = d.rect(Target::suffix(&format!("{VIEW_MENU}/Grid")));
        let checks = support::shapes(&d)
            .into_iter()
            .filter(|shape| item.contains_rect(shape.visual_bounding_rect()))
            .count();
        let unchecked = open_view(App::default());
        let bare = support::shapes(&unchecked)
            .into_iter()
            .filter(|shape| item.contains_rect(shape.visual_bounding_rect()))
            .count();
        assert!(
            checks > bare,
            "the check is drawn: {checks} shapes against {bare}"
        );
    }

    #[test]
    fn a_chosen_commands_item_is_a_radio_with_the_dot_on_the_choice() {
        use egui::accesskit::Toggled;
        let d = open_view(App {
            size: Size::Large,
            ..App::default()
        });
        assert_eq!(
            toggled(&d, Role::MenuItemRadio, "Small"),
            Some(Toggled::False)
        );
        assert_eq!(
            toggled(&d, Role::MenuItemRadio, "Large"),
            Some(Toggled::True)
        );
    }

    #[test]
    fn activating_a_checked_commands_item_runs_the_command() {
        let mut d = open_view(App::default());
        d.click(Target::suffix(&format!("{VIEW_MENU}/Grid")))
            .frames(5);
        assert!(d.app().grid);
    }

    #[test]
    fn an_item_can_disable_an_enabled_command() {
        let d = open_view(App {
            can_save: true,
            ..App::default()
        });
        assert!(
            !disabled(&d, "Save"),
            "precondition: an enabled item is not disabled"
        );
        assert!(disabled(&d, "Copy"));
    }

    #[test]
    fn an_item_cannot_enable_a_disabled_command() {
        let d = open_view(App::default());
        assert!(disabled(&d, "Save"));
    }

    #[test]
    fn the_shell_reaches_a_command_from_a_chord_an_item_and_a_button() {
        let mut d = harness();
        press(&mut d, Modifiers::COMMAND, Key::K);
        assert_eq!(log(&d), ["shelled"]);
        let width = d.ctx().content_rect().width();
        assert_eq!(d.app().shell_width, Some(width));

        let mut d = open_view(App::default());
        d.click(Target::suffix(&format!("{VIEW_MENU}/Shelled")))
            .frames(5);
        assert_eq!(log(&d), ["shelled"]);

        let mut d = harness_of(App {
            button: Some(Cmd::Shelled),
            ..App::default()
        });
        d.harness()
            .get_by_role_and_label(Role::Button, "Run")
            .click();
        d.frames(5);
        assert_eq!(log(&d), ["shelled"]);
    }

    #[test]
    fn a_table_gives_a_command_by_its_key_and_its_shortcut_as_the_platform_writes_it() {
        let d = harness();
        let app = App {
            grid: true,
            ..App::default()
        };
        let table = page_commands(&app);
        let save = table.get(Cmd::Save).expect("Save is in the table");
        assert_eq!(save.title(), "Save");
        assert_eq!(
            save.shortcut_text(d.ctx()),
            Some(d.ctx().format_shortcut(&SAVE.into()))
        );
        assert_eq!(
            table.get(Cmd::Grid).map(|grid| grid.state(&app)),
            Some(CommandState::Checked(Mark::On))
        );
        assert_eq!(
            table.get(Cmd::Save).map(|save| save.state(&app)),
            Some(CommandState::Plain)
        );
        assert!(table.get(Cmd::Confirm).is_none());
    }
}

mod validation {
    use eggshell_ui::Problem;

    use super::*;

    fn declared(table: fn() -> Commands<App>) -> Driver<App> {
        harness_of(App {
            table: Some(table),
            ..App::default()
        })
    }

    fn noop(_: &mut App) {}

    #[test]
    fn two_commands_on_one_chord_are_reported() {
        let mut d = declared(|| {
            Commands::new()
                .add(Cmd::Save, Command::new("Save", noop).shortcut(SAVE))
                .add(Cmd::Copy, Command::new("Copy", noop).shortcut(SAVE))
        });
        let report = d.expect_reported(Problem::DuplicateChord);
        assert!(
            report.message.contains("bound to the same chord"),
            "{}",
            report.message
        );
    }

    #[test]
    fn a_key_added_twice_is_reported() {
        let mut d = declared(|| {
            Commands::new()
                .add(Cmd::Save, Command::new("Save", noop))
                .add(Cmd::Save, Command::new("Save again", noop))
        });
        let report = d.expect_reported(Problem::DuplicateCommand);
        assert!(report.message.contains("added twice"), "{}", report.message);
    }

    #[test]
    fn a_bare_escape_or_enter_is_no_chord() {
        for key in [Key::Escape, Key::Enter] {
            assert_eq!(Chord::new(Modifiers::NONE, key), None, "{key:?}");
            assert!(Chord::new(Modifiers::COMMAND, key).is_some(), "{key:?}");
            assert!(Chord::new(Modifiers::SHIFT, key).is_some(), "{key:?}");
        }
        assert!(Chord::new(Modifiers::NONE, Key::H).is_some());
    }

    #[test]
    fn a_valid_table_reports_nothing() {
        let _d = declared(|| {
            Commands::new()
                .add(Cmd::Save, Command::new("Save", noop).shortcut(SAVE))
                .add(Cmd::Copy, Command::new("Copy", noop))
        });
    }

    #[test]
    fn a_menu_item_for_an_unknown_command_is_reported_and_disabled() {
        let mut d = harness_of(App {
            table: Some(|| Commands::new().add(Cmd::Save, Command::new("Save", noop))),
            ..App::default()
        });
        d.click(Target::suffix(FILE)).frames(5);
        let report = d.expect_reported(Problem::UnknownCommand);
        assert!(report.message.contains("Copy"), "{}", report.message);
        let item = d.harness().get_by_role_and_label(Role::MenuItem, "Copy");
        assert!(item.accesskit_node().data().is_disabled());
    }

    #[test]
    fn a_button_for_an_unknown_command_is_reported_and_disabled() {
        let mut d = harness_of(App {
            table: Some(|| Commands::new().add(Cmd::Save, Command::new("Save", noop))),
            button: Some(Cmd::Copy),
            ..App::default()
        });
        let report = d.expect_reported(Problem::UnknownCommand);
        assert!(report.message.contains("Copy"), "{}", report.message);
        let button = d.harness().get_by_role_and_label(Role::Button, "Run");
        assert!(button.accesskit_node().data().is_disabled());
        button.click();
        d.frames(5);
        assert!(log(&d).is_empty(), "{:?}", log(&d));
    }

    #[test]
    fn declaring_commands_twice_in_a_root_is_reported() {
        let mut d = harness_of(App {
            twice: true,
            ..App::default()
        });
        let report = d.expect_reported(Problem::CommandsTwice);
        assert!(
            report.message.contains("declared twice in one root"),
            "{}",
            report.message
        );
    }
}
