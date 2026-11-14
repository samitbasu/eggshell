//! The `accordion` example: opening a section closes the open one, the list
//! below moves with both, and the whole is continuous.

use eggshell_ui::testing::{Timeline, record};
use egui::vec2;

use crate::support::{self, rect_of};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/accordion.rs"]
mod example;

use example::State;

const ROW: &str = r#"/Page/List/Row(0)"#;

fn body(index: usize) -> String {
    format!("/Page/Accordion/Item({index})/Body({index})")
}

fn harness() -> egui_kittest::Harness<'static, support::Rig<State>> {
    let mut harness = support::harness_stepped(
        vec2(500.0, 500.0),
        0.01,
        common::base_text(egui::Theme::Light),
        State::default(),
        example::layout,
    );
    harness.run_steps(4);
    harness
}

fn height(harness: &egui_kittest::Harness<'_, support::Rig<State>>, index: usize) -> f32 {
    rect_of(harness, &body(index)).height()
}

#[test]
fn opening_a_section_closes_the_open_one_and_the_list_follows() {
    let mut harness = harness();
    record(&harness.ctx);
    harness.state_mut().app.toggle(0);
    harness.run_steps(40);
    let first = height(&harness, 0);
    assert!(
        first > 20.0,
        "precondition: the first section is open: {first}"
    );
    let list_with_first = rect_of(&harness, ROW).min.y;

    harness.state_mut().app.toggle(1);
    harness.run_steps(8);
    let (closing, opening) = (height(&harness, 0), height(&harness, 1));
    assert!(
        closing > 0.0 && closing < first && opening > 0.0,
        "precondition: both are moving: {closing} {opening}"
    );
    harness.run_steps(40);
    assert_eq!(height(&harness, 0), 0.0);
    let second = height(&harness, 1);
    assert!(second > 20.0, "{second}");
    assert_eq!(
        rect_of(&harness, ROW).min.y,
        list_with_first - first + second
    );

    Timeline::recorded(&harness.ctx).assert_continuous();
}
