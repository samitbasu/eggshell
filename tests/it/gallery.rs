//! The controls gallery through the full path: every cell's text input is
//! sized by the library and drawn by egui, and the two agree at every width,
//! text size, style and hint the controls offer.

use egui::{Vec2, vec2};
use egui_kittest::Harness;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/controls_gallery.rs"]
mod controls_gallery;

use crate::support;

use controls_gallery::{
    CellWidth, Control, Interact, Label, Padding, State, TextSize, layout, prepare,
};
use support::{Rig, click_node, drag, rect_of};

const WINDOW: Vec2 = vec2(1500.0, 1100.0);

fn harness(state: State) -> Harness<'static, Rig<State>> {
    let mut harness = support::harness_prepared(
        WINDOW,
        controls_gallery::base_text(),
        state,
        layout,
        prepare,
    );
    harness.run_steps(4);
    harness
}

/// The cells whose rects disagree, with the rects.
fn mismatches(harness: &Harness<'_, Rig<State>>) -> Vec<String> {
    harness
        .state()
        .app
        .reports
        .iter()
        .filter(|(_, report)| report.mismatch().is_some())
        .map(|(control, report)| format!("{control:?}: {report:?}"))
        .collect()
}

/// Every checked control reported, none of them with an empty rect.
#[track_caller]
fn assert_all_report(harness: &Harness<'_, Rig<State>>) {
    let reports = &harness.state().app.reports;
    for control in Control::ALL.into_iter() {
        let report = reports
            .get(&control)
            .unwrap_or_else(|| panic!("{control:?} did not draw: {:?}", reports.keys()));
        assert!(
            report.computed.width() > 4.0 && report.computed.height() > 4.0,
            "{control:?} has a real rect: {report:?}"
        );
    }
}

#[test]
fn the_library_sizes_every_widget_as_egui_draws_it() {
    let mut harness = harness(State::default());
    let mut cases = 0;
    for width in [90.0, 200.0, 320.0] {
        for text_size in [TextSize::Small, TextSize::Body, TextSize::Large] {
            for padding in [Padding::Tight, Padding::Standard, Padding::Roomy] {
                for interact in [Interact::Small, Interact::Standard, Interact::Tall] {
                    for label in [Label::Short, Label::Long, Label::Wrapping] {
                        let state = &mut harness.state_mut().app;
                        state.cell_width = CellWidth::new(width);
                        (state.text_size, state.padding) = (text_size, padding);
                        (state.interact, state.label) = (interact, label);
                        state.reports.clear();
                        harness.run_steps(3);
                        assert_all_report(&harness);
                        assert_eq!(
                            mismatches(&harness),
                            Vec::<String>::new(),
                            "{width} {text_size:?} {padding:?} {interact:?} {label:?}"
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 3 * 81);
}

#[test]
fn a_hint_and_empty_fields_are_sized_too() {
    let state = State {
        single: String::new(),
        multi: String::new(),
        field_multi: String::new(),
        ..State::default()
    };
    let harness = harness(state);
    assert_all_report(&harness);
    assert_eq!(mismatches(&harness), Vec::<String>::new());
}

#[test]
fn the_estimate_can_be_wrong_and_the_gallery_says_so() {
    use controls_gallery::Report;
    use egui::Rect;
    let computed = Rect::from_min_size(egui::pos2(10.0, 10.0), vec2(80.0, 20.0));
    let within = Report {
        computed,
        drawn: Rect::from_min_size(computed.min, vec2(80.9, 20.5)),
    };
    let beyond = Report {
        computed,
        drawn: Rect::from_min_size(computed.min, vec2(83.0, 20.0)),
    };
    assert_eq!(within.mismatch(), None);
    assert_eq!(beyond.mismatch(), Some(vec2(3.0, 0.0)));
}

#[test]
fn dragging_a_cells_handle_changes_every_cells_width() {
    let mut harness = harness(State::default());
    let stage = |harness: &Harness<'_, Rig<State>>| {
        rect_of(harness, &format!("/{:?}", Control::Edit)).width()
    };
    let before = stage(&harness);
    let handle = rect_of(&harness, &format!("/(\"handle\", {:?})", Control::Edit));
    drag(
        &mut harness,
        handle.center(),
        handle.center() + vec2(60.0, 0.0),
    );

    let after = stage(&harness);
    assert!(after > before + 40.0, "{before} -> {after}");
    assert_eq!(
        f32::from(harness.state().app.cell_width),
        200.0 + 60.0,
        "the drag moved the width by what the pointer did"
    );
    assert_eq!(mismatches(&harness), Vec::<String>::new());
}

#[test]
fn a_choice_changes_the_hint_and_the_fields_follow() {
    let mut harness = harness(State {
        multi: String::new(),
        ..State::default()
    });
    let height = |harness: &Harness<'_, Rig<State>>| {
        rect_of(harness, &format!("/{:?}", Control::EditMulti)).height()
    };
    let before = height(&harness);
    click_node(&mut harness, "/\"label\"/Wrapping");
    harness.run_steps(3);
    assert_eq!(harness.state().app.label, Label::Wrapping);
    assert!(
        height(&harness) > before,
        "a hint that wraps makes the empty field taller"
    );
    assert_eq!(mismatches(&harness), Vec::<String>::new());
}
