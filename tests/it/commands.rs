//! App commands through the real path: key events dispatched by
//! `egui_kittest::Harness`, build, layout, draw and the dispatcher that runs
//! after the library has claimed Esc and Return.

use eggshell_ui::overlay::Modal;
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
    if app.dialog {
        b.overlay("dialog")
            .modal(Modal::Center)
            .on_dismiss(|app: &mut App| app.dialog = false)
            .children(|s| {
                s.commands(&dialog_commands(), app);
                s.div_id(
                    "Box",
                    tw().w_px(200.0).h_px(100.0).bg(Color32::WHITE),
                    |_| {},
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
