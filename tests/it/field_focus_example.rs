//! The `field_focus` example: Enter sends and the field keeps focus, and
//! Rename focuses the name field.

use eggshell_ui::testing::{Driver, Target};
use egui::{Event, Key, vec2};

use eggshell_examples::common;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/field_focus.rs"]
mod example;

use crate::support::{self, focused_node};

const MESSAGE: &str = "/Page/Message";
const NAME: &str = "/Page/Name";
const RENAME: &str = "/Page/Rename";

fn harness() -> Driver<example::State> {
    let mut d = support::prepared_driver(
        vec2(360.0, 240.0),
        common::base_text(egui::Theme::Light),
        example::State::default(),
        example::view,
        |_, _| {},
    )
    .build();
    d.frames(4);
    d
}

#[test]
fn enter_sends_the_message_and_the_field_keeps_focus_for_the_next() {
    let mut d = harness();
    d.click(Target::suffix(MESSAGE)).frames(6);
    for message in ["hello", "world"] {
        d.event(Event::Text(message.into())).frames(3);
        d.press(Key::Enter).frames(5);
        assert_eq!(focused_node(&d).as_deref(), Some(MESSAGE), "{message}");
    }
    assert_eq!(d.app().sent, ["hello", "world"]);
    assert_eq!(d.app().draft, "");
}

#[test]
fn rename_focuses_the_name_field_from_wherever_focus_was() {
    let mut d = harness();
    d.click(Target::suffix(MESSAGE)).frames(6);
    assert_eq!(focused_node(&d).as_deref(), Some(MESSAGE));
    d.click(Target::suffix(RENAME)).frames(6);
    assert_eq!(focused_node(&d).as_deref(), Some(NAME));
}

#[test]
fn rename_selects_the_name_so_typing_replaces_it() {
    let mut d = harness();
    d.app_mut().name = "Item 1".to_owned();
    d.click(Target::suffix(RENAME)).frames(6);
    assert_eq!(
        focused_node(&d).as_deref(),
        Some(NAME),
        "precondition: Rename focused the name field"
    );
    d.event(egui::Event::Text("Box".into())).frames(3);
    assert_eq!(d.app().name, "Box");
}
