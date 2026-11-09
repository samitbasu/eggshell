//! The controls the demos keep inside their trees, through real pointer
//! events: stock sliders, buttons and segmented controls change the demo's
//! state, and the demo follows.

use egui::{Vec2, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/button.rs"]
mod button;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/custom_background.rs"]
mod custom_background;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flex_wrap.rs"]
mod flex_wrap;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flexbox_guide.rs"]
mod flexbox_guide;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/grow.rs"]
mod grow;

use crate::support;

use support::{Rig, click_node, drag, rect_of};

const WINDOW: Vec2 = vec2(1100.0, 700.0);

fn harness<A: 'static>(app: A, view: support::View<A>) -> Harness<'static, Rig<A>> {
    let mut harness = support::harness(WINDOW, common::base_text(egui::Theme::Light), app, view);
    harness.run_steps(2);
    harness
}

/// Drags the slider leaf `path` from its middle to its right end.
fn drag_to_the_end<A>(harness: &mut Harness<'_, Rig<A>>, path: &str) {
    let rail = rect_of(harness, path);
    assert!(rail.width() > 100.0, "precondition: a real rail: {rail:?}");
    drag(
        harness,
        rail.center(),
        rail.right_center() + vec2(40.0, 0.0),
    );
}

#[test]
fn a_slider_in_the_tree_drives_the_demo_it_sits_beside() {
    let mut harness = harness(grow::State::default(), grow::layout);
    let before = harness.state().app.gap;
    let item = |harness: &Harness<'_, Rig<grow::State>>| {
        harness
            .state()
            .nodes
            .iter()
            .filter(|(path, _)| path.contains("<div>"))
            .map(|(_, rect)| rect.min.y)
            .collect::<Vec<_>>()
    };
    let positions = item(&harness);

    drag_to_the_end(&mut harness, "/\"gap\"/Track");
    assert!(harness.state().app.gap > before + 3.0);
    assert_eq!(harness.state().app.gap, 12.0, "the end of the range");
    assert_ne!(item(&harness), positions, "the demo re-laid out");
}

#[test]
fn the_box_size_slider_resizes_the_boxes() {
    let mut harness = harness(
        custom_background::State::default(),
        custom_background::layout,
    );
    let before = harness.state().app.size;
    drag_to_the_end(&mut harness, "/\"size\"/Track");
    assert!(harness.state().app.size > before);
    assert_eq!(harness.state().app.size, 300.0);
}

#[test]
fn a_segmented_choice_changes_the_wrapping() {
    let mut harness = harness(flex_wrap::State::default(), flex_wrap::layout);
    let wraps = |harness: &Harness<'_, Rig<flex_wrap::State>>| {
        let rows: Vec<f32> = harness
            .state()
            .nodes
            .iter()
            .filter(|(path, _)| path.contains("<div>"))
            .map(|(_, rect)| rect.min.y.round())
            .collect();
        rows.windows(2).any(|pair| pair[1] > pair[0])
    };
    assert_eq!(harness.state().app.wrapping, flex_wrap::Wrapping::Wrap);

    click_node(&mut harness, "/\"wrapping\"/NoWrap");
    harness.run_steps(3);
    assert_eq!(harness.state().app.wrapping, flex_wrap::Wrapping::NoWrap);
    click_node(&mut harness, "/\"wrapping\"/Wrap");
    harness.run_steps(3);
    assert_eq!(harness.state().app.wrapping, flex_wrap::Wrapping::Wrap);
    assert!(wraps(&harness));
}

#[test]
fn the_reset_button_is_a_stock_widget_that_clears_the_demo() {
    let mut harness = harness(button::State::default(), button::layout);
    click_node(&mut harness, "/\"button\"");
    click_node(&mut harness, "/\"selectable\"");
    assert_eq!(harness.state().app.clicks, 1);
    assert!(harness.state().app.selected);

    click_node(&mut harness, "/\"reset\"");
    harness.run_steps(3);
    assert_eq!(harness.state().app.clicks, 0);
    assert!(!harness.state().app.selected);
}

#[test]
fn the_guide_selector_switches_demo_and_value() {
    let mut harness = harness(flexbox_guide::State::default(), flexbox_guide::layout);
    assert_eq!(harness.state().app.demo, 0);

    click_node(&mut harness, "/\"flex-wrap\"");
    harness.run_steps(3);
    let state = &harness.state().app;
    assert_eq!(state.demos[state.demo].title, "flex-wrap");
    assert_eq!(state.choice, 0);

    click_node(&mut harness, "/\"choices\"/2");
    harness.run_steps(3);
    assert_eq!(harness.state().app.choice, 2);

    click_node(&mut harness, "/\"flex-direction\"");
    harness.run_steps(3);
    assert_eq!(
        harness.state().app.choice,
        0,
        "a new demo starts at its first value"
    );
}
