//! The `clipboard` example: what arrives, and which way, are on screen.

use std::sync::Arc;

use eggshell_ui::ClipboardError;
use eggshell_ui::testing::FakeClipboard;
use egui::{ColorImage, Event, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/clipboard.rs"]
mod example;

use crate::support::{self, Rig, click_node};

const BUTTON: &str = "/Page/Button";

fn harness() -> (Harness<'static, Rig<example::State>>, FakeClipboard) {
    let fake = FakeClipboard::default();
    let mut harness = support::harness_prepared(
        vec2(360.0, 240.0),
        common::base_text(egui::Theme::Light),
        example::State::default(),
        example::layout,
        |ctx, _| common::use_preset(ctx),
    );
    harness
        .state()
        .layout
        .set_clipboard(Some(Arc::new(fake.clone())));
    harness.run_steps(4);
    (harness, fake)
}

fn has(harness: &Harness<'_, Rig<example::State>>, part: &str) -> bool {
    harness
        .state()
        .nodes
        .iter()
        .any(|(path, _)| path.ends_with(part))
}

#[test]
fn the_button_pastes_an_image_by_a_read_and_the_chord_by_its_event() {
    let (mut harness, fake) = harness();
    fake.set_image(ColorImage::example());
    click_node(&mut harness, BUTTON);
    harness.run_steps(4);
    let arrived = harness.state().app.arrived.as_ref().expect("arrived");
    assert_eq!(arrived.via, eggshell_ui::Via::Read);
    assert!(
        example::describe(&arrived.paste).starts_with("image, 128 x 64"),
        "{}",
        example::describe(&arrived.paste)
    );
    assert!(has(&harness, "Thumbnail"), "a thumbnail of the image");

    harness.event(Event::Paste("hello".into()));
    harness.run_steps(4);
    let arrived = harness.state().app.arrived.as_ref().expect("arrived");
    assert_eq!(arrived.via, eggshell_ui::Via::Event);
    assert!(!has(&harness, "Thumbnail"));
}

#[test]
fn a_refused_read_shows_an_error_line_and_no_arrival() {
    let (mut harness, fake) = harness();
    fake.refuse(Some(ClipboardError::Unavailable));
    click_node(&mut harness, BUTTON);
    harness.run_steps(4);
    assert!(harness.state().app.arrived.is_none());
    assert!(has(&harness, "Error"), "an error line");
}
