//! The `field_focus` example: Enter sends and the field keeps focus, and
//! Rename focuses the name field.

use egui::{Event, Key, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/field_focus.rs"]
mod example;

use crate::support::{self, Rig, click_node, focused_node};

const MESSAGE: &str = "/Page/Message";
const NAME: &str = "/Page/Name";
const RENAME: &str = "/Page/Rename";

fn harness() -> Harness<'static, Rig<example::State>> {
    let mut harness = support::harness_prepared(
        vec2(360.0, 240.0),
        common::base_text(egui::Theme::Light),
        example::State::default(),
        example::layout,
        |ctx, _| common::use_preset(ctx),
    );
    harness.run_steps(4);
    harness
}

#[test]
fn enter_sends_the_message_and_the_field_keeps_focus_for_the_next() {
    let mut harness = harness();
    click_node(&mut harness, MESSAGE);
    harness.run_steps(6);
    for message in ["hello", "world"] {
        harness.event(Event::Text(message.into()));
        harness.run_steps(4);
        harness.key_press(Key::Enter);
        harness.run_steps(6);
        assert_eq!(
            focused_node(&harness).as_deref(),
            Some(MESSAGE),
            "{message}"
        );
    }
    assert_eq!(harness.state().app.sent, ["hello", "world"]);
    assert_eq!(harness.state().app.draft, "");
}

#[test]
fn rename_focuses_the_name_field_from_wherever_focus_was() {
    let mut harness = harness();
    click_node(&mut harness, MESSAGE);
    harness.run_steps(6);
    assert_eq!(focused_node(&harness).as_deref(), Some(MESSAGE));
    click_node(&mut harness, RENAME);
    harness.run_steps(6);
    assert_eq!(focused_node(&harness).as_deref(), Some(NAME));
}

#[test]
fn rename_selects_the_name_so_typing_replaces_it() {
    let mut harness = harness();
    harness.state_mut().app.name = "Item 1".to_owned();
    click_node(&mut harness, RENAME);
    harness.run_steps(6);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some(NAME),
        "precondition: Rename focused the name field"
    );
    harness.event(egui::Event::Text("Box".into()));
    harness.run_steps(4);
    assert_eq!(harness.state().app.name, "Box");
}
