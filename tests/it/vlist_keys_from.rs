//! A virtual list whose keys come from a search field (`ListChain::keys_from`),
//! through the real path: the field keeps the focus and types, its arrows
//! move the list's highlight, Enter activates it, and the field announces
//! the list and the highlighted row.

use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::Role as AccessRole;
use egui::{Color32, FontFamily, Key, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path};

const FIELD: &str = r#"/"Page"/"query""#;
const TYPING: &str = "<widget>";

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Row(u32);

#[derive(Default)]
struct App {
    query: String,
    picked: Vec<u32>,
    /// Rows that are disabled.
    disabled: Vec<u32>,
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

/// The rows whose number starts with the query: built in the frame, and
/// handed to the list as its own.
fn matching(query: &str) -> Vec<Row> {
    (0..200)
        .filter(|row: &u32| row.to_string().starts_with(query))
        .map(Row)
        .collect()
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().w_px(200.0), |s| {
        s.text_field("query", app.query.clone())
            .role(Role::ComboBox)
            .named("Search")
            .classes(tw().w_full())
            .on_change(|app: &mut App, query| app.query = query);
        let field = s.id_of("query");
        let disabled = app.disabled.clone();
        s.virtual_list("results", tw().h_px(200.0), matching(&app.query))
            .keys_from(field)
            .highlight_first()
            .fixed_px(20.0)
            .named("Results")
            .row_name(|row: Row| format!("Row {}", row.0))
            .row_enablement(move |row: Row| Enablement::from(!disabled.contains(&row.0)))
            .row_mark(|row: Row| (row.0 == 5).then_some(Mark::On))
            .on_activate(|app: &mut App, row: Row| app.picked.push(row.0))
            .items(|s, row| s.text(tw().h_px(20.0), format!("Row {}", row.0)));
    });
}

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(300.0, 300.0), base_text(), app, view);
    harness.run_steps(4);
    click_node(&mut harness, FIELD);
    harness.run_steps(4);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "precondition: the field's text edit has the focus"
    );
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(App::default())
}

fn press<S>(harness: &mut Harness<'_, Rig<S>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(4);
}

fn active(harness: &Harness<'_, Rig<App>>) -> Option<String> {
    harness
        .get_by_role_and_label(AccessRole::ComboBox, "Search")
        .accesskit_node()
        .active_descendant()
        .and_then(|node| node.label())
}

#[test]
fn the_first_row_is_highlighted_as_the_list_opens() {
    let harness = harness();
    assert_eq!(active(&harness).as_deref(), Some("Row 0"));
}

#[test]
fn the_fields_arrows_move_the_highlight_and_the_field_keeps_the_focus() {
    let mut harness = harness();
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("Row 2"));
    press(&mut harness, Key::ArrowUp);
    assert_eq!(active(&harness).as_deref(), Some("Row 1"));
    assert_eq!(focused_path(&harness).as_deref(), Some(TYPING));
}

#[test]
fn enter_activates_the_highlighted_row() {
    let mut harness = harness();
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.picked, [1]);
    assert_eq!(focused_path(&harness).as_deref(), Some(TYPING));
}

#[test]
fn home_and_end_stay_the_fields() {
    let mut harness = harness();
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::End);
    assert_eq!(active(&harness).as_deref(), Some("Row 1"));
}

#[test]
fn page_down_moves_a_page_of_rows() {
    let mut harness = harness();
    press(&mut harness, Key::PageDown);
    let after = active(&harness).unwrap_or_default();
    assert!(
        after != "Row 0" && after != "Row 1",
        "a page is more than a row: {after}"
    );
}

#[test]
fn typing_narrows_the_rows_and_highlights_the_first_again() {
    let mut harness = harness();
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    harness
        .get_by_role_and_label(AccessRole::ComboBox, "Search")
        .type_text("1");
    harness.run_steps(6);
    assert_eq!(
        harness.state().app.query,
        "1",
        "precondition: the field typed"
    );
    assert_eq!(active(&harness).as_deref(), Some("Row 1"));
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("Row 10"));
}

#[test]
fn the_field_controls_the_list() {
    let harness = harness();
    let controlled: Vec<(AccessRole, Option<String>)> = harness
        .get_by_role_and_label(AccessRole::ComboBox, "Search")
        .accesskit_node()
        .controls()
        .map(|node| (node.role(), node.label()))
        .collect();
    assert_eq!(
        controlled,
        [(AccessRole::ListBox, Some("Results".to_owned()))]
    );
}

#[test]
fn the_arrows_pass_disabled_rows_by() {
    let mut harness = harness_with(App {
        disabled: vec![0, 2, 3],
        ..App::default()
    });
    assert_eq!(
        active(&harness).as_deref(),
        Some("Row 1"),
        "the first enabled row is highlighted"
    );
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("Row 4"));
    press(&mut harness, Key::ArrowUp);
    assert_eq!(active(&harness).as_deref(), Some("Row 1"));
    let row = harness.get_by_label("Row 2");
    assert!(row.accesskit_node().is_disabled(), "a disabled row says so");
}

#[test]
fn a_row_with_a_mark_announces_it() {
    let harness = harness();
    let toggled = |label: &str| harness.get_by_label(label).accesskit_node().toggled();
    assert_eq!(toggled("Row 5"), Some(egui::accesskit::Toggled::True));
    assert_eq!(toggled("Row 4"), None);
}

#[test]
fn a_click_on_a_row_activates_it_and_leaves_the_focus_in_the_field() {
    let mut harness = harness();
    harness.get_by_label("Row 3").click();
    harness.run_steps(6);
    assert_eq!(harness.state().app.picked, [3]);
    assert_eq!(focused_path(&harness).as_deref(), Some(TYPING));
}

#[derive(Default)]
struct Modal {
    query: String,
    picked: Vec<u32>,
}

/// The list and its field in a centred modal dialog that takes the focus.
fn modal(b: &mut Builder<'_, '_, Modal>, app: &Modal) {
    b.overlay("dialog")
        .modal(eggshell_ui::overlay::Modal::Center)
        .role(Role::Dialog)
        .named("Palette")
        .children(|s| {
            s.div_id(
                "Box",
                tw().flex().flex_col().w_px(200.0).bg(Color32::WHITE),
                |s| {
                    s.text_field("query", app.query.clone())
                        .role(Role::ComboBox)
                        .named("Search")
                        .on_change(|app: &mut Modal, query| app.query = query);
                    let field = s.id_of("query");
                    s.virtual_list("results", tw().max_h_px(200.0), matching(&app.query))
                        .keys_from(field)
                        .highlight_first()
                        .fixed_px(20.0)
                        .named("Results")
                        .row_name(|row: Row| format!("Row {}", row.0))
                        .on_activate(|app: &mut Modal, row: Row| app.picked.push(row.0))
                        .items(|s, row| s.text(tw().h_px(20.0), format!("Row {}", row.0)));
                },
            );
        });
}

#[test]
fn in_a_dialog_the_first_row_is_highlighted_and_enter_picks_it() {
    let mut harness = support::harness(vec2(400.0, 400.0), base_text(), Modal::default(), modal);
    harness.run_steps(20);
    click_node(&mut harness, r#"/"dialog"/"Box"/"query""#);
    harness.run_steps(6);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "precondition: the dialog's field has the focus"
    );
    let field = harness.get_by_role_and_label(AccessRole::ComboBox, "Search");
    assert_eq!(
        field
            .accesskit_node()
            .active_descendant()
            .and_then(|node| node.label())
            .as_deref(),
        Some("Row 0")
    );
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.picked, [0]);
}
