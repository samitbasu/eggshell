//! A duplicate key path panics at the builder call that repeats it, and the
//! message names that call, so that a process that cannot unwind still says
//! where.

use std::panic::{self, AssertUnwindSafe};

use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, tw};
use egui::vec2;

/// The message of the panic that `build` raises.
fn panic_message(build: impl FnOnce(&mut Builder<'_, '_, ()>)) -> String {
    let mut build = Some(build);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        Headless::new(vec2(200.0, 100.0)).layout(&(), |b, _| (build.take().unwrap())(b));
    }));
    let payload = result.expect_err("debug builds panic on a duplicate key");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
        .expect("a string payload")
}

fn assert_names_call(message: &str, line: u32) {
    assert!(message.contains("duplicate key path"), "{message}");
    assert!(
        message.contains(&format!("duplicate_key.rs:{line}:")),
        "expected line {line} in: {message}"
    );
}

#[test]
fn a_duplicate_div_id_names_the_second_call() {
    let mut line = 0;
    let message = panic_message(|b| {
        b.div_id("row", tw(), |_| {});
        line = line!() + 1;
        b.div_id("row", tw(), |_| {});
    });
    assert_names_call(&message, line);
}

#[test]
fn a_duplicate_control_names_the_call_that_finishes_it() {
    let mut line = 0;
    let message = panic_message(|b| {
        b.slider("volume", 1.0, 0.0..=2.0).on_change(|_, _| {});
        line = line!() + 1;
        b.slider("volume", 1.0, 0.0..=2.0).on_change(|_, _| {});
    });
    assert_names_call(&message, line);
    let message = panic_message(|b| {
        b.button("ok", "OK").on_click(|_| {});
        line = line!() + 1;
        b.button("ok", "OK").on_click(|_| {});
    });
    assert_names_call(&message, line);
    let message = panic_message(|b| {
        b.text_field("name", "").on_change(|_, _| {});
        line = line!() + 1;
        b.text_field("name", "").on_change(|_, _| {});
    });
    assert_names_call(&message, line);
}
