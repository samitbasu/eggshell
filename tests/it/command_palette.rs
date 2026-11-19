//! The extras `command` feature through the real path: the example's
//! palette, combobox and autosuggest, driven by kittest's keys and pointer,
//! read back through the accessibility tree.

#[path = "../../examples/command_palette.rs"]
#[allow(dead_code)]
mod example;

use eggshell_ui::{TextSpec, TextStyle, Weight};
use egui::accesskit::{Role, Toggled};
use egui::{Color32, FontFamily, Key, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig};
use example::State;

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

fn harness() -> Harness<'static, Rig<State>> {
    let mut harness = support::harness(
        vec2(800.0, 700.0),
        base_text(),
        State::default(),
        example::layout,
    );
    harness.run_steps(4);
    harness
}

fn step(harness: &mut Harness<'_, Rig<State>>) {
    harness.run_steps(6);
}

fn press(harness: &mut Harness<'_, Rig<State>>, key: Key) {
    harness.key_press(key);
    step(harness);
}

fn open_palette(harness: &mut Harness<'_, Rig<State>>) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(20);
    assert!(
        harness.state().app.palette,
        "precondition: Ctrl+K opened it"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Dialog, "Command palette")
            .is_some(),
        "precondition: the palette is showing"
    );
}

/// The label of the item the field of the combobox named `name` announces
/// as its active descendant.
fn active(harness: &Harness<'_, Rig<State>>, name: &str) -> Option<String> {
    harness
        .get_by_role_and_label(Role::ComboBox, name)
        .accesskit_node()
        .active_descendant()
        .and_then(|node| node.label())
}

fn options(harness: &Harness<'_, Rig<State>>) -> Vec<String> {
    harness
        .query_all_by_role(Role::ListBoxOption)
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

#[test]
fn ctrl_k_opens_the_palette_with_its_field_focused_and_the_first_item_highlighted() {
    let mut harness = harness();
    open_palette(&mut harness);
    assert_eq!(
        support::focused_path(&harness).as_deref(),
        Some("<widget>"),
        "the search field's text edit has the focus"
    );
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Getting started")
    );
}

#[test]
fn typing_filters_by_word_prefix_and_keywords() {
    let mut harness = harness();
    open_palette(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("lay");
    step(&mut harness);
    assert_eq!(options(&harness), ["Layout"]);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("x");
    step(&mut harness);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("dialog");
    step(&mut harness);
    assert_eq!(options(&harness), ["Overlays"], "a keyword finds its item");
}

#[test]
fn nothing_matching_shows_the_empty_state() {
    let mut harness = harness();
    open_palette(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("zzz");
    step(&mut harness);
    assert!(options(&harness).is_empty());
    assert!(harness.query_by_label("No results found.").is_some());
}

#[test]
fn enter_picks_the_highlighted_item_and_closes_the_palette() {
    let mut harness = harness();
    open_palette(&mut harness);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Layout")
    );
    press(&mut harness, Key::Enter);
    harness.run_steps(20);
    assert_eq!(harness.state().app.said, "Opened Layout");
    assert!(!harness.state().app.palette, "a pick closes the palette");
}

#[test]
fn a_picked_command_runs() {
    let mut harness = harness();
    open_palette(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("show");
    step(&mut harness);
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Show grid")
    );
    press(&mut harness, Key::Enter);
    assert!(harness.state().app.grid);
    assert!(!harness.state().app.palette);
}

#[test]
fn a_checked_command_says_so_and_a_disabled_one_is_passed_by() {
    let mut harness = harness();
    harness.state_mut().app.grid = true;
    open_palette(&mut harness);
    let grid = harness.get_by_role_and_label(Role::ListBoxOption, "Show grid");
    assert_eq!(grid.accesskit_node().toggled(), Some(Toggled::True));
    let delete = harness.get_by_role_and_label(Role::ListBoxOption, "Delete");
    assert!(delete.accesskit_node().is_disabled());
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("show");
    step(&mut harness);
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Show grid")
    );
    harness.key_press(Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state().app.palette, "Escape closes the palette");
}

#[test]
fn the_combobox_opens_filters_and_picks() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "Framework")
        .click();
    harness.run_steps(20);
    assert_eq!(
        support::focused_path(&harness).as_deref(),
        Some("<widget>"),
        "the popover's search field took the focus"
    );
    harness
        .query_all_by_role_and_label(Role::ComboBox, "Framework")
        .last()
        .expect("the popover's field")
        .type_text("sv");
    step(&mut harness);
    assert_eq!(options(&harness), ["SvelteKit"]);
    press(&mut harness, Key::Enter);
    harness.run_steps(20);
    assert_eq!(
        harness.state().app.framework,
        Some(example_framework("SvelteKit"))
    );
    assert!(options(&harness).is_empty(), "a pick closes the popover");
    let trigger = harness.get_by_role_and_label(Role::ComboBox, "Framework");
    assert_eq!(
        trigger.accesskit_node().value().as_deref(),
        Some("SvelteKit")
    );
}

fn example_framework(label: &str) -> example::Framework {
    example::FRAMEWORKS
        .iter()
        .find(|(_, name)| *name == label)
        .map(|(value, _)| *value)
        .expect("a framework of that name")
}

#[test]
fn the_autosuggest_suggests_without_a_highlight_and_takes_a_pick() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("b");
    step(&mut harness);
    assert_eq!(options(&harness), ["Berlin", "Bern"]);
    assert_eq!(active(&harness, "City"), None, "nothing is highlighted");
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness, "City").as_deref(), Some("Bern"));
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.city, "Bern");
    assert!(
        options(&harness).is_empty(),
        "a pick puts the suggestions away"
    );
}

#[test]
fn the_autosuggest_shows_nothing_when_nothing_matches() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("q");
    step(&mut harness);
    assert_eq!(harness.state().app.city, "q", "precondition: it typed");
    assert!(options(&harness).is_empty());
    assert!(harness.query_by_label("No results found.").is_none());
}

#[test]
fn escape_puts_the_suggestions_away_until_the_next_edit() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("b");
    step(&mut harness);
    assert!(!options(&harness).is_empty(), "precondition: suggesting");
    harness.key_press(Key::Escape);
    harness.run_steps(20);
    assert!(
        options(&harness).is_empty(),
        "focus after Escape: {:?}",
        support::focused_path(&harness)
    );
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("e");
    step(&mut harness);
    assert_eq!(options(&harness), ["Berlin", "Bern"]);
}
