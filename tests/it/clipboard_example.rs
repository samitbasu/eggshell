//! The `clipboard` example: what arrives, and which way, are on screen.

use std::sync::Arc;

use eggshell_ui::ClipboardError;
use eggshell_ui::testing::{Driver, FakeClipboard, Target};
use egui::{ColorImage, Event, vec2};

use eggshell_examples::common;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/clipboard.rs"]
mod example;

use crate::support;

const BUTTON: &str = "/Page/Button";

fn harness() -> (Driver<example::State>, FakeClipboard) {
    let fake = FakeClipboard::default();
    let mut d = support::prepared_driver(
        vec2(360.0, 240.0),
        common::base_text(egui::Theme::Light),
        example::State::default(),
        example::view,
        |_, _| {},
    )
    .build();
    d.layout().set_clipboard(Some(Arc::new(fake.clone())));
    d.frames(4);
    (d, fake)
}

fn has(d: &Driver<example::State>, part: &str) -> bool {
    support::nodes(d)
        .iter()
        .any(|(path, _)| path.ends_with(part))
}

#[test]
fn the_button_pastes_an_image_by_a_read_and_the_chord_by_its_event() {
    let (mut d, fake) = harness();
    fake.set_image(ColorImage::example());
    d.click(Target::suffix(BUTTON)).frames(4);
    let arrived = d.app().arrived.as_ref().expect("arrived");
    assert_eq!(arrived.via, eggshell_ui::Via::Read);
    assert!(
        example::describe(&arrived.paste).starts_with("image, 128 x 64"),
        "{}",
        example::describe(&arrived.paste)
    );
    assert!(has(&d, "Thumbnail"), "a thumbnail of the image");

    d.event(Event::Paste("hello".into())).frames(3);
    let arrived = d.app().arrived.as_ref().expect("arrived");
    assert_eq!(arrived.via, eggshell_ui::Via::Event);
    assert!(!has(&d, "Thumbnail"));
}

#[test]
fn a_refused_read_shows_an_error_line_and_no_arrival() {
    let (mut d, fake) = harness();
    fake.refuse(Some(ClipboardError::Unavailable));
    d.click(Target::suffix(BUTTON)).frames(4);
    assert!(d.app().arrived.is_none());
    assert!(has(&d, "Error"), "an error line");
}
