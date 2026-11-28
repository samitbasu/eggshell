//! The examples the docs embed, driven as their `main`s run them: hello's
//! label and the counter's button.

use eggshell_examples::{common, counter, hello};
use egui::{Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

#[test]
fn hello_shows_its_label() {
    let d = support::driver(
        vec2(320.0, 120.0),
        common::base_text(Theme::Light),
        (),
        hello::view,
    )
    .build();
    assert!(
        d.harness().query_by_label("Hello, eggshell").is_some(),
        "the label is shown"
    );
}

#[test]
fn the_counter_counts_each_click() {
    let mut d = support::driver(
        vec2(320.0, 120.0),
        common::base_text(Theme::Light),
        counter::State::default(),
        counter::view,
    )
    .build();
    d.frames(2);
    assert!(d.harness().query_by_label("Counted 0").is_some());
    for _ in 0..2 {
        d.harness().get_by_label("Add one").click();
        d.frames(2);
    }
    assert_eq!(d.app().count, 2);
    assert!(d.harness().query_by_label("Counted 2").is_some());
}
