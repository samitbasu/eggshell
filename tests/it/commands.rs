//! App commands through the real path: key events dispatched by
//! `egui_kittest::Harness`, build, layout, draw and the dispatcher that runs
//! after the library has claimed Esc and Return.

use eggshell_ui::overlay::{Anchor, Modal};
use eggshell_ui::prim::text_field::Field;
use eggshell_ui::{Builder, Command, Commands, InText, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Key, KeyboardShortcut, Modifiers, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, rect_of};

const NAME: &str = r#"/"Page"/"name""#;
const FILE: &str = r#"/"Page"/"file""#;
const MENU: &str = r#"/"Page"/("file", Menu)/Box"#;

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
}

const fn chord(modifiers: Modifiers, key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(modifiers, key)
}

const SAVE: KeyboardShortcut = chord(Modifiers::COMMAND, Key::S);
const SAVE_AS: KeyboardShortcut = chord(
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
            m.command(Cmd::Save);
            m.command(Cmd::Copy);
        });
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
                                .items(move |m| m.command(command));
                        }
                    },
                );
            });
    }
}

fn harness_of(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(500.0, 400.0), base_text(), app, view);
    harness.run_steps(4);
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_of(App {
        can_save: true,
        ..App::default()
    })
}

fn press(harness: &mut Harness<'_, Rig<App>>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run_steps(4);
}

fn log(harness: &Harness<'_, Rig<App>>) -> Vec<&'static str> {
    harness.state().app.log.clone()
}

fn start_typing(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, NAME);
    harness.run_steps(6);
    assert_eq!(
        focused_path(harness).as_deref(),
        Some("<widget>"),
        "precondition: the field has focus"
    );
}

#[test]
fn a_chord_runs_its_command() {
    let mut harness = harness();
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(log(&harness), ["save"]);
}

#[test]
fn shift_command_s_does_not_run_the_command_bound_to_command_s() {
    let mut harness = harness();
    press(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    assert!(log(&harness).is_empty(), "{:?}", log(&harness));
}

#[test]
fn shift_command_s_runs_its_own_command_and_not_command_s() {
    let mut harness = harness_of(App {
        can_save: true,
        with_save_as: true,
        ..App::default()
    });
    press(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    assert_eq!(log(&harness), ["save as"]);
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(log(&harness), ["save as", "save"]);
}

#[test]
fn a_chord_a_command_takes_is_not_seen_by_a_custom_node() {
    let mut harness = harness();
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(log(&harness), ["save"], "precondition: the command ran");
    assert_eq!(harness.state().app.canvas_saw_s, 0);
}

#[test]
fn a_disabled_command_does_not_run_and_leaves_its_chord_to_the_rest() {
    let mut harness = harness_of(App::default());
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert!(log(&harness).is_empty());
    assert_eq!(
        harness.state().app.canvas_saw_s,
        1,
        "the chord was not consumed"
    );
}

#[test]
fn copy_is_a_press_of_command_c() {
    let mut harness = harness();
    harness.event(egui::Event::Copy);
    harness.run_steps(4);
    assert_eq!(log(&harness), ["copy"]);
}

#[test]
fn paste_is_a_press_of_command_v_where_it_fires_in_a_text_field() {
    let mut harness = harness();
    start_typing(&mut harness);
    harness.event(egui::Event::Paste("text".into()));
    harness.run_steps(4);
    assert_eq!(log(&harness), ["paste"]);
}

#[test]
fn a_chord_that_is_not_pressed_exactly_does_not_run() {
    let mut harness = harness();
    press(&mut harness, Modifiers::NONE, Key::S);
    press(&mut harness, Modifiers::ALT | Modifiers::COMMAND, Key::S);
    assert!(log(&harness).is_empty(), "{:?}", log(&harness));
}

mod modals {
    use super::*;

    fn opened() -> Harness<'static, Rig<App>> {
        let mut harness = harness();
        harness.state_mut().app.dialog = true;
        harness.run_steps(12);
        assert!(
            harness
                .state()
                .nodes
                .iter()
                .any(|(path, _)| path.starts_with(r#"/"dialog""#)),
            "precondition: the dialog is open"
        );
        harness
    }

    #[test]
    fn a_page_command_does_not_run_under_a_modal() {
        let mut harness = opened();
        press(&mut harness, Modifiers::COMMAND, Key::S);
        assert!(log(&harness).is_empty(), "{:?}", log(&harness));
    }

    #[test]
    fn the_modals_own_commands_run() {
        let mut harness = opened();
        press(&mut harness, Modifiers::COMMAND, Key::Enter);
        assert_eq!(log(&harness), ["confirm"]);
    }

    fn open_dialog_menu(item: Cmd) -> Harness<'static, Rig<App>> {
        let mut harness = harness_of(App {
            can_save: true,
            dialog: true,
            dialog_item: Some(item),
            ..App::default()
        });
        harness.run_steps(12);
        harness
            .get_by_role_and_label(Role::Button, "Dialog menu")
            .click();
        harness.run_steps(6);
        harness
    }

    #[test]
    fn a_menu_in_a_dialog_offers_the_dialogs_commands() {
        let mut harness = open_dialog_menu(Cmd::Confirm);
        harness
            .get_by_role_and_label(Role::MenuItem, "Confirm")
            .click();
        harness.run_steps(6);
        assert_eq!(log(&harness), ["confirm"]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "no command of that key is declared")]
    fn a_menu_in_a_dialog_does_not_offer_a_page_command_whose_chord_is_dead() {
        open_dialog_menu(Cmd::Save);
    }

    #[test]
    fn the_modals_commands_do_not_run_once_it_is_closed() {
        let mut harness = opened();
        harness.state_mut().app.dialog = false;
        harness.run_steps(30);
        press(&mut harness, Modifiers::COMMAND, Key::Enter);
        assert!(log(&harness).is_empty(), "{:?}", log(&harness));
        press(&mut harness, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&harness), ["save"], "the page's commands are back");
    }
}

mod popovers {
    use super::*;

    fn opened() -> Harness<'static, Rig<App>> {
        let mut harness = harness();
        harness.state_mut().app.popover = true;
        harness.run_steps(12);
        assert!(
            harness
                .state()
                .nodes
                .iter()
                .any(|(path, _)| path.starts_with(r#"/"popover""#)),
            "precondition: the popover is open"
        );
        harness
    }

    #[test]
    fn a_popovers_commands_run_beside_the_pages() {
        let mut harness = opened();
        press(&mut harness, Modifiers::COMMAND, Key::L);
        press(&mut harness, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&harness), ["local", "save"]);
    }

    #[test]
    fn a_chord_in_both_tables_runs_the_nearest_command_only() {
        let mut harness = opened();
        press(&mut harness, Modifiers::COMMAND, Key::C);
        assert_eq!(log(&harness), ["shadow"]);
    }
}

mod text_fields {
    use super::*;

    #[test]
    fn a_command_chord_runs_in_a_text_field() {
        let mut harness = harness();
        start_typing(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&harness), ["save"]);
    }

    #[test]
    fn a_chord_a_text_field_uses_does_not_run() {
        let mut harness = harness();
        start_typing(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::C);
        assert!(log(&harness).is_empty(), "{:?}", log(&harness));
    }

    #[test]
    fn a_bare_key_does_not_run_while_typing_and_does_otherwise() {
        let mut harness = harness();
        start_typing(&mut harness);
        press(&mut harness, Modifiers::NONE, Key::H);
        assert!(log(&harness).is_empty(), "typing");
        harness.key_press(Key::Escape);
        harness.run_steps(6);
        assert_eq!(focused_path(&harness), None, "precondition: no focus");
        press(&mut harness, Modifiers::NONE, Key::H);
        assert_eq!(log(&harness), ["help"]);
    }

    #[test]
    fn fire_overrides_a_chord_a_text_field_uses() {
        let mut harness = harness();
        start_typing(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::V);
        press(&mut harness, Modifiers::NONE, Key::J);
        assert_eq!(log(&harness), ["paste", "jump"]);
    }

    #[test]
    fn yield_overrides_a_chord_a_text_field_leaves_alone() {
        let mut harness = harness();
        start_typing(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::O);
        assert!(log(&harness).is_empty(), "typing");
        harness.key_press(Key::Escape);
        harness.run_steps(6);
        press(&mut harness, Modifiers::COMMAND, Key::O);
        assert_eq!(log(&harness), ["open"]);
    }
}

mod menus {
    use super::*;

    fn open_file(harness: &mut Harness<'_, Rig<App>>) {
        click_node(harness, FILE);
        harness.run_steps(5);
        assert!(
            harness.state().nodes.iter().any(|(path, _)| path == MENU),
            "precondition: the menu is open"
        );
    }

    #[test]
    fn an_item_takes_its_title_and_the_shortcut_as_the_platform_writes_it() {
        let mut harness = harness();
        open_file(&mut harness);
        harness.get_by_role_and_label(Role::MenuItem, "Save");
        let written = harness.ctx.format_shortcut(&SAVE);
        let menu = rect_of(&harness, MENU);
        let shown = support::shapes(&harness).into_iter().any(|shape| {
            matches!(shape, egui::Shape::Text(text)
                if text.galley.text() == written && menu.contains(text.pos))
        });
        assert!(shown, "{written:?} is not in the menu");
    }

    #[test]
    fn a_menu_item_shows_on_a_mac_with_the_shortcut_in_symbols() {
        let mut harness = harness();
        harness.ctx.set_os(egui::os::OperatingSystem::Mac);
        open_file(&mut harness);
        let written = harness.ctx.format_shortcut(&SAVE);
        assert!(
            written.contains('\u{2318}'),
            "{written:?} is not in symbols"
        );
        let menu = rect_of(&harness, MENU);
        let shown = support::shapes(&harness).into_iter().any(|shape| {
            matches!(shape, egui::Shape::Text(text)
                if text.galley.text() == written && menu.contains(text.pos))
        });
        assert!(shown, "{written:?} is not in the menu");
    }

    #[test]
    fn a_key_chip_draws_the_mac_modifier_symbols() {
        let chip = "\u{2318}\u{21e7}S";
        let mut harness = support::harness(vec2(200.0, 60.0), base_text(), (), |b, _| {
            b.kbd("\u{2318}\u{21e7}S")
        });
        harness.ctx.set_os(egui::os::OperatingSystem::Mac);
        harness.run_steps(3);
        let drawn = support::shapes(&harness)
            .into_iter()
            .any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == chip));
        assert!(drawn, "{chip:?} is not drawn");
    }

    #[test]
    fn every_key_symbol_has_a_glyph_in_the_proportional_family() {
        let harness = harness();
        let missing: Vec<char> = "\u{2318}\u{21e7}\u{2325}\u{2303}\u{232b}\u{21b5}\u{21e5}\u{2190}\u{2191}\u{2192}\u{2193}\u{23ce}"
            .chars()
            .filter(|symbol| {
                let font = egui::FontId::proportional(14.0);
                !harness
                    .ctx
                    .fonts_mut(|fonts| fonts.has_glyphs(&font, &symbol.to_string()))
            })
            .collect();
        assert!(missing.is_empty(), "no glyph for {missing:?}");
    }

    #[test]
    fn an_item_is_disabled_with_its_command() {
        let mut harness = harness_of(App::default());
        open_file(&mut harness);
        let disabled = |harness: &Harness<'_, Rig<App>>, label| {
            harness
                .get_by_role_and_label(Role::MenuItem, label)
                .accesskit_node()
                .data()
                .is_disabled()
        };
        assert!(disabled(&harness, "Save"));
        assert!(!disabled(&harness, "Copy"));
    }

    #[test]
    fn activating_an_item_runs_the_command() {
        let mut harness = harness();
        open_file(&mut harness);
        click_node(&mut harness, &format!("{MENU}/Copy"));
        harness.run_steps(5);
        assert_eq!(log(&harness), ["copy"]);
    }

    #[test]
    fn a_chord_runs_its_command_under_an_open_menu() {
        let mut harness = harness();
        open_file(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::S);
        assert_eq!(log(&harness), ["save"]);
    }
}

#[cfg(debug_assertions)]
mod validation {
    use super::*;

    fn declared(table: fn() -> Commands<App>) {
        harness_of(App {
            table: Some(table),
            ..App::default()
        });
    }

    fn noop(_: &mut App) {}

    #[test]
    #[should_panic(expected = "bound to the same chord")]
    fn two_commands_on_one_chord_panic() {
        declared(|| {
            Commands::new()
                .add(Cmd::Save, Command::new("Save", noop).shortcut(SAVE))
                .add(Cmd::Copy, Command::new("Copy", noop).shortcut(SAVE))
        });
    }

    #[test]
    #[should_panic(expected = "added twice")]
    fn a_key_added_twice_panics() {
        declared(|| {
            Commands::new()
                .add(Cmd::Save, Command::new("Save", noop))
                .add(Cmd::Save, Command::new("Save again", noop))
        });
    }

    #[test]
    #[should_panic(expected = "bare Escape")]
    fn a_bare_escape_panics() {
        declared(|| {
            Commands::new().add(
                Cmd::Save,
                Command::new("Save", noop).shortcut(chord(Modifiers::NONE, Key::Escape)),
            )
        });
    }

    #[test]
    #[should_panic(expected = "bare Enter")]
    fn a_bare_enter_panics() {
        declared(|| {
            Commands::new().add(
                Cmd::Save,
                Command::new("Save", noop).shortcut(chord(Modifiers::NONE, Key::Enter)),
            )
        });
    }

    #[test]
    #[should_panic(expected = "no command of that key is declared")]
    fn a_menu_item_for_an_unknown_command_panics() {
        let mut harness = harness_of(App {
            table: Some(|| Commands::new().add(Cmd::Save, Command::new("Save", noop))),
            ..App::default()
        });
        click_node(&mut harness, FILE);
        harness.run_steps(5);
    }

    #[test]
    #[should_panic(expected = "declared twice in one root")]
    fn declaring_commands_twice_in_a_root_panics() {
        harness_of(App {
            twice: true,
            ..App::default()
        });
    }
}
