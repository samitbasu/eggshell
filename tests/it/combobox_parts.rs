//! The core parts a combobox or a command palette is built from, through the
//! real path: Enter in a listbox overlay with no anchor, a text field's
//! role, and a command read and run from outside the library.

use eggshell_ui::overlay::{Modal, Nav, Wrap};
use eggshell_ui::{Builder, Command, Commands, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::Role as AccessRole;
use egui::{Color32, FontFamily, Key, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, rect_of};

const FIELD: &str = r#"/"palette"/"Box"/"query""#;
const TYPING: &str = "<widget>";
const ITEMS: [&str; 3] = ["Open", "Save", "Close"];

#[derive(Default)]
struct App {
    query: String,
    picked: Vec<&'static str>,
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

/// A centred modal listbox whose own search field has the focus: a command
/// palette with nothing to anchor it.
fn palette(b: &mut Builder<'_, '_, App>, app: &App) {
    b.overlay("palette")
        .modal(Modal::Center)
        .nav(Nav::Listbox(Wrap::Around))
        .role(Role::ListBox)
        .named("Commands")
        .children(|s| {
            s.div_id(
                "Box",
                tw().flex().flex_col().w_px(200.0).bg(Color32::WHITE),
                |s| {
                    s.text_field("query", app.query.clone())
                        .role(Role::ComboBox)
                        .named("Search")
                        .classes(tw().w_full())
                        .on_change(|app: &mut App, query| app.query = query);
                    for item in ITEMS {
                        s.div_id(item, tw().h_px(24.0).sense_click(), |s| s.text(tw(), item))
                            .role(Role::ListBoxOption)
                            .named(item)
                            .on_click(move |app: &mut App| app.picked.push(item));
                    }
                },
            );
        });
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(400.0, 300.0), base_text(), App::default(), palette);
    harness.run_steps(4);
    click_node(&mut harness, FIELD);
    harness.run_steps(4);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "precondition: the palette's field has the focus"
    );
    assert!(
        rect_of(&harness, FIELD).height() > 0.0,
        "precondition: the palette is showing"
    );
    harness
}

#[test]
fn enter_picks_the_highlight_of_a_listbox_with_no_anchor() {
    let mut harness = harness();
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(4);
    assert_eq!(harness.state().app.picked, ["Save"]);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "the field keeps the focus"
    );
}

#[test]
fn enter_with_nothing_highlighted_is_the_fields() {
    let mut harness = harness();
    harness.key_press(Key::Enter);
    harness.run_steps(4);
    assert!(harness.state().app.picked.is_empty());
}

#[test]
fn a_field_says_the_role_it_is_given() {
    let harness = harness();
    let field = harness.get_by_label("Search");
    assert_eq!(field.accesskit_node().role(), AccessRole::ComboBox);
}

#[test]
fn a_field_with_no_role_given_is_a_text_input() {
    let mut harness = support::harness(
        vec2(300.0, 100.0),
        base_text(),
        App::default(),
        |b: &mut Builder<'_, '_, App>, app: &App| {
            b.text_field("query", app.query.clone())
                .named("Search")
                .on_change(|app: &mut App, query| app.query = query);
        },
    );
    harness.run_steps(2);
    let field = harness.get_by_label("Search");
    assert_eq!(field.accesskit_node().role(), AccessRole::TextInput);
}

#[derive(Default)]
struct Doc {
    saved: u32,
    dirty: bool,
}

#[test]
fn a_command_is_read_and_run_from_outside_the_library() {
    let table: Commands<Doc> = Commands::new().add(
        "save",
        Command::new("Save", |doc: &mut Doc| doc.saved += 1).enabled(|doc: &Doc| doc.dirty),
    );
    let save = table.get("save").expect("precondition: the table has Save");
    let shell = support::shell(&support::harness(
        vec2(10.0, 10.0),
        base_text(),
        (),
        |_: &mut Builder<'_, '_, ()>, _: &()| {},
    ));
    let mut doc = Doc::default();
    assert!(!save.is_enabled(&doc));
    doc.dirty = true;
    assert!(save.is_enabled(&doc));
    save.run(&mut doc, &shell);
    assert_eq!(doc.saved, 1);
}
