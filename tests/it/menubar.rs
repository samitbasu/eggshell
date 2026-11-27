//! The `menubar` example (`owned::menubar`) through the real
//! path: kittest's pointer and key events, build, layout, draw and the
//! judgement of the frame's input.

use egui::{Key, Modifiers, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{AccessKitNode, NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, hover_node};

use eggshell_examples::common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/menubar.rs"]
mod example;

use example::{Density, State};

const FILE: &str = "/Page/Bar/File";
const EDIT: &str = "/Page/Bar/Edit";
const VIEW: &str = "/Page/Bar/View";

fn menu(trigger: &str) -> String {
    let (bar, key) = trigger.rsplit_once('/').unwrap_or_default();
    format!("{bar}/({key}, Menu)/Box")
}

fn item(trigger: &str, key: &str) -> String {
    format!("{}/{key}", menu(trigger))
}

fn harness() -> Harness<'static, Rig<State>> {
    let mut harness = support::harness(
        vec2(640.0, 480.0),
        common::base_text(egui::Theme::Light),
        State::default(),
        example::layout,
    );
    harness.ctx.set_theme(egui::Theme::Light);
    harness.run_steps(6);
    harness
}

fn settle(harness: &mut Harness<'_, Rig<State>>) {
    harness.run_steps(6);
}

fn press(harness: &mut Harness<'_, Rig<State>>, key: Key) {
    harness.key_press(key);
    settle(harness);
}

fn has(harness: &Harness<'_, Rig<State>>, path: &str) -> bool {
    harness
        .state()
        .nodes
        .iter()
        .any(|(candidate, _)| candidate == path)
}

fn open(harness: &Harness<'_, Rig<State>>) -> Vec<&'static str> {
    [FILE, EDIT, VIEW]
        .into_iter()
        .filter(|trigger| has(harness, &menu(trigger)))
        .collect()
}

/// The role and name of the nearest ancestor of `node` that is more than a
/// container.
fn owner(node: AccessKitNode<'_>) -> Option<(Role, Option<String>)> {
    let mut at = node.parent();
    while let Some(node) = at {
        if node.role() != Role::GenericContainer {
            return Some((node.role(), node.label()));
        }
        at = node.parent();
    }
    None
}

#[test]
fn the_bar_is_a_named_menu_bar_and_its_triggers_menu_items() {
    let harness = harness();
    let bar = harness.get_by_role(Role::MenuBar);
    assert_eq!(bar.accesskit_node().label().as_deref(), Some("Menu bar"));
    assert_eq!(
        bar.accesskit_node().data().orientation(),
        Some(egui::accesskit::Orientation::Horizontal)
    );
    for name in ["File", "Edit", "View", "Help"] {
        let trigger = harness.get_by_role_and_label(Role::MenuItem, name);
        assert_eq!(
            trigger.accesskit_node().data().has_popup(),
            Some(egui::accesskit::HasPopup::Menu),
            "{name}"
        );
        assert_eq!(
            owner(trigger.accesskit_node()),
            Some((Role::MenuBar, Some("Menu bar".to_owned()))),
            "{name} is the bar's"
        );
    }
    assert!(
        harness
            .get_by_role_and_label(Role::MenuItem, "Help")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_click_opens_a_menu_and_the_pointer_switches_while_it_is_open() {
    let mut harness = harness();
    click_node(&mut harness, FILE);
    settle(&mut harness);
    assert_eq!(open(&harness), [FILE]);
    assert_eq!(focused_path(&harness), Some(item(FILE, "Cmd(New)")));
    hover_node(&mut harness, VIEW);
    settle(&mut harness);
    assert_eq!(open(&harness), [VIEW]);
    click_node(&mut harness, EDIT);
    settle(&mut harness);
    assert_eq!(open(&harness), [EDIT], "a press on another trigger");
}

#[test]
fn the_arrows_switch_menus_and_skip_the_disabled_one() {
    let mut harness = harness();
    click_node(&mut harness, FILE);
    settle(&mut harness);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [EDIT]);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(open(&harness), [VIEW]);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(
        open(&harness),
        [FILE],
        "past the disabled Help, round to File"
    );
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(open(&harness), [VIEW]);
}

#[test]
fn escape_returns_to_the_trigger_and_the_closed_bar_walks_with_the_arrows() {
    let mut harness = harness();
    click_node(&mut harness, FILE);
    settle(&mut harness);
    press(&mut harness, Key::ArrowRight);
    press(&mut harness, Key::Escape);
    assert!(open(&harness).is_empty());
    assert_eq!(focused_path(&harness).as_deref(), Some(EDIT));
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(focused_path(&harness).as_deref(), Some(FILE));
    press(&mut harness, Key::ArrowDown);
    assert_eq!(open(&harness), [FILE]);
}

#[test]
fn the_open_trigger_takes_the_open_look() {
    let mut harness = harness();
    let fill_of = |harness: &Harness<'_, Rig<State>>| support::fill_of(harness, FILE);
    let closed = fill_of(&harness);
    click_node(&mut harness, FILE);
    settle(&mut harness);
    hover_node(&mut harness, "/Page/Log");
    settle(&mut harness);
    assert_eq!(open(&harness), [FILE], "precondition: still open");
    assert_ne!(fill_of(&harness), closed);
}

#[test]
fn the_view_menus_commands_show_their_marks_and_run() {
    let mut harness = harness();
    let chord = Modifiers {
        alt: true,
        ..Modifiers::COMMAND
    };
    harness.key_press_modifiers(chord, Key::G);
    settle(&mut harness);
    assert!(
        harness.state().app.grid,
        "the chord works with every menu closed"
    );
    click_node(&mut harness, VIEW);
    settle(&mut harness);
    let grid = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Show Grid");
    assert_eq!(
        grid.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    click_node(&mut harness, &item(VIEW, "Cmd(Compact)"));
    settle(&mut harness);
    assert_eq!(harness.state().app.density, Density::Compact);
    assert!(open(&harness).is_empty());
    assert_eq!(focused_path(&harness).as_deref(), Some(VIEW));
}

#[test]
fn tab_is_held_while_a_menu_is_open_as_the_dropdowns_is() {
    let mut harness = harness();
    click_node(&mut harness, FILE);
    settle(&mut harness);
    assert_eq!(open(&harness), [FILE], "precondition: open");
    let first = focused_path(&harness);
    press(&mut harness, Key::Tab);
    assert_eq!(open(&harness), [FILE]);
    assert_eq!(focused_path(&harness), first);
}

#[test]
fn a_menu_of_the_bar_vanishes_at_once_so_a_switch_shows_one_menu() {
    let mut harness = harness();
    click_node(&mut harness, FILE);
    settle(&mut harness);
    let frame = support::Drives::frame_dt(&harness);
    let edit = support::rect_of(&harness, EDIT).center();
    harness.hover_at(edit);
    harness.run_steps(2);
    assert!(
        2.0 * frame < 0.15,
        "precondition: less time than a dropdown's 150 ms exit"
    );
    assert_eq!(open(&harness), [EDIT], "File is gone at once");
    harness.key_press(Key::Escape);
    harness.run_steps(1);
    assert!(open(&harness).is_empty(), "Edit is gone as Esc closes it");
}
