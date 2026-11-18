//! The documentation page with mounted examples: its Markdown directives
//! mount each app over the page's state, and the two counters share theirs.

use egui::{Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/docs_embed.rs"]
mod page;

fn harness() -> Harness<'static, Rig<page::State>> {
    let mut harness = support::harness(
        vec2(720.0, 900.0),
        common::base_text(Theme::Light),
        page::State::default(),
        page::layout,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn the_directives_mount_every_example() {
    let harness = harness();
    for label in ["Counted 0", "Actions", "Name", "Row 1"] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "{label} is mounted"
        );
    }
    assert!(
        harness
            .query_by_label_contains("No example named")
            .is_none()
    );
}

#[test]
fn the_two_counters_count_together() {
    let mut harness = harness();
    harness
        .query_all_by_label("Add one")
        .next()
        .expect("a counter is mounted")
        .click();
    harness.run_steps(4);
    assert_eq!(harness.state().app.clicks.count, 1);
    assert_eq!(harness.query_all_by_label("Counted 1").count(), 2);
}
