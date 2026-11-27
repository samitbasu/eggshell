//! The documentation page with mounted examples: its Markdown directives
//! mount each app over the page's state, and the two counters share theirs.

use eggshell_ui::testing::Driver;
use egui::{Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

use eggshell_examples::common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/docs_embed.rs"]
mod page;

fn harness() -> Driver<page::State> {
    let mut d = support::driver(
        vec2(720.0, 900.0),
        common::base_text(Theme::Light),
        page::State::default(),
        page::view,
    )
    .build();
    d.frames(4);
    d
}

#[test]
fn the_directives_mount_every_example() {
    let d = harness();
    for label in ["Counted 0", "Actions", "Name", "Row 1"] {
        assert!(
            d.harness().query_all_by_label(label).next().is_some(),
            "{label} is mounted"
        );
    }
    assert!(
        d.harness()
            .query_by_label_contains("No example named")
            .is_none()
    );
}

#[test]
fn the_two_counters_count_together() {
    let mut d = harness();
    d.harness()
        .query_all_by_label("Add one")
        .next()
        .expect("a counter is mounted")
        .click();
    d.frames(4);
    assert_eq!(d.app().clicks.count, 1);
    assert_eq!(d.harness().query_all_by_label("Counted 1").count(), 2);
}
