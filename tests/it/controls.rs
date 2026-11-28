//! The controls the demos keep inside their trees, through real pointer
//! events: stock sliders, buttons and segmented controls change the demo's
//! state, and the demo follows.

use egui::{Vec2, vec2};

use eggshell_examples::common;

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

use eggshell_ui::testing::{Driver, Target};

const WINDOW: Vec2 = vec2(1100.0, 700.0);

fn harness<A>(app: A, view: support::View<A>) -> Driver<A> {
    let mut d = support::driver(WINDOW, common::base_text(egui::Theme::Light), app, view).build();
    d.frames(2);
    d
}

/// Drags the slider leaf `path` from its middle to its right end.
fn drag_to_the_end<A>(d: &mut Driver<A>, path: &str) {
    let rail = d.rect(Target::suffix(path));
    assert!(rail.width() > 100.0, "precondition: a real rail: {rail:?}");
    d.drag(rail.center(), rail.right_center() + vec2(40.0, 0.0));
}

#[test]
fn a_slider_in_the_tree_drives_the_demo_it_sits_beside() {
    let mut d = harness(grow::State::default(), grow::view);
    let before = d.app().gap;
    let item = |d: &Driver<grow::State>| {
        support::nodes(d)
            .iter()
            .filter(|(path, _)| path.contains("<div>"))
            .map(|(_, rect)| rect.min.y)
            .collect::<Vec<_>>()
    };
    let positions = item(&d);

    drag_to_the_end(&mut d, "/\"gap\"/Track");
    assert!(d.app().gap > before + 3.0);
    assert_eq!(d.app().gap, 12.0, "the end of the range");
    assert_ne!(item(&d), positions, "the demo re-laid out");
}

#[test]
fn the_box_size_slider_resizes_the_boxes() {
    let mut d = harness(custom_background::State::default(), custom_background::view);
    let before = d.app().size;
    drag_to_the_end(&mut d, "/\"size\"/Track");
    assert!(d.app().size > before);
    assert_eq!(d.app().size, 300.0);
}

#[test]
fn a_segmented_choice_changes_the_wrapping() {
    let mut d = harness(flex_wrap::State::default(), flex_wrap::view);
    let wraps = |d: &Driver<flex_wrap::State>| {
        let rows: Vec<f32> = support::nodes(d)
            .iter()
            .filter(|(path, _)| path.contains("<div>"))
            .map(|(_, rect)| rect.min.y.round())
            .collect();
        rows.windows(2).any(|pair| pair[1] > pair[0])
    };
    assert_eq!(d.app().wrapping, flex_wrap::Wrapping::Wrap);

    d.click(Target::suffix("/\"wrapping\"/1")).frames(3);
    assert_eq!(d.app().wrapping, flex_wrap::Wrapping::NoWrap);
    d.click(Target::suffix("/\"wrapping\"/0")).frames(3);
    assert_eq!(d.app().wrapping, flex_wrap::Wrapping::Wrap);
    assert!(wraps(&d));
}

#[test]
fn the_reset_button_is_a_stock_widget_that_clears_the_demo() {
    let mut d = harness(button::State::default(), button::view);
    d.click(Target::suffix("/\"button\""));
    d.click(Target::suffix("/\"selectable\""));
    assert_eq!(d.app().clicks, 1);
    assert!(d.app().selected);

    d.click(Target::suffix("/\"reset\"")).frames(3);
    assert_eq!(d.app().clicks, 0);
    assert!(!d.app().selected);
}

#[test]
fn the_guide_selector_switches_demo_and_value() {
    let mut d = harness(flexbox_guide::State::default(), flexbox_guide::view);
    assert_eq!(d.app().demo, 0);

    d.click(Target::suffix("/\"flex-wrap\"")).frames(3);
    let state = d.app();
    assert_eq!(state.demos[state.demo].title, "flex-wrap");
    assert_eq!(state.choice, 0);

    d.click(Target::suffix("/\"choices\"/2")).frames(3);
    assert_eq!(d.app().choice, 2);

    d.click(Target::suffix("/\"flex-direction\"")).frames(3);
    assert_eq!(d.app().choice, 0, "a new demo starts at its first value");
}
