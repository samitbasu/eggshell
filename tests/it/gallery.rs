//! The controls gallery through the full path: every cell's text input is
//! sized by the library and drawn by egui, and the two agree at every width,
//! text size, style and hint the controls offer.

use eggshell_ui::testing::{Driver, Target};
use egui::{Vec2, vec2};
use egui_kittest::kittest::Queryable as _;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/controls_gallery.rs"]
mod controls_gallery;

use crate::support;

#[path = "../../examples/common/mod.rs"]
mod common;

use controls_gallery::{
    CellWidth, Control, Interact, Label, Padding, State, TextSize, layout, prepare,
};

const WINDOW: Vec2 = vec2(1500.0, 1100.0);

fn driver(state: State) -> Driver<State> {
    let mut d = support::prepared_driver(
        WINDOW,
        controls_gallery::base_text(),
        state,
        layout,
        prepare,
    )
    .build();
    d.frames(4);
    d
}

/// The cells whose rects disagree, with the rects.
fn mismatches(d: &Driver<State>) -> Vec<String> {
    d.app()
        .reports
        .iter()
        .filter(|(_, report)| report.mismatch().is_some())
        .map(|(control, report)| format!("{control:?}: {report:?}"))
        .collect()
}

/// Every checked control reported, none of them with an empty rect.
#[track_caller]
fn assert_all_report(d: &Driver<State>) {
    let reports = &d.app().reports;
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
    let mut d = driver(State::default());
    let mut cases = 0;
    for width in [90.0, 200.0, 320.0] {
        for text_size in [TextSize::Small, TextSize::Body, TextSize::Large] {
            for padding in [Padding::Tight, Padding::Standard, Padding::Roomy] {
                for interact in [Interact::Small, Interact::Standard, Interact::Tall] {
                    for label in [Label::Short, Label::Long, Label::Wrapping] {
                        let state = &mut d.app_mut();
                        state.cell_width = CellWidth::new(width);
                        (state.text_size, state.padding) = (text_size, padding);
                        (state.interact, state.label) = (interact, label);
                        state.reports.clear();
                        d.frames(3);
                        assert_all_report(&d);
                        assert_eq!(
                            mismatches(&d),
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
    let d = driver(state);
    assert_all_report(&d);
    assert_eq!(mismatches(&d), Vec::<String>::new());
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
    let mut d = driver(State::default());
    let stage = |d: &Driver<State>| {
        d.rect(Target::suffix(&format!("/{:?}", Control::Edit)))
            .width()
    };
    let before = stage(&d);
    let handle = d.rect(Target::suffix(&format!(
        "/(\"handle\", {:?})",
        Control::Edit
    )));
    d.drag(handle.center(), handle.center() + vec2(60.0, 0.0));

    let after = stage(&d);
    assert!(after > before + 40.0, "{before} -> {after}");
    assert_eq!(
        f32::from(d.app().cell_width),
        200.0 + 60.0,
        "the drag moved the width by what the pointer did"
    );
    assert_eq!(mismatches(&d), Vec::<String>::new());
}

#[test]
fn a_choice_changes_the_hint_and_the_fields_follow() {
    let mut d = driver(State {
        multi: String::new(),
        ..State::default()
    });
    let height = |d: &Driver<State>| {
        d.rect(Target::suffix(&format!("/{:?}", Control::EditMulti)))
            .height()
    };
    let before = height(&d);
    d.click(Target::suffix("/\"label\"/2"));
    d.frames(3);
    assert_eq!(d.app().label, Label::Wrapping);
    assert!(
        height(&d) > before,
        "a hint that wraps makes the empty field taller"
    );
    assert_eq!(mismatches(&d), Vec::<String>::new());
}

/// The border boxes of the section control's segments.
fn section_cells(d: &Driver<State>) -> Vec<egui::Rect> {
    support::nodes(d)
        .into_iter()
        .filter(|(path, _)| path.starts_with("/\"section\"/"))
        .map(|(_, rect)| rect)
        .collect()
}

#[test]
fn the_sections_wrap_inside_the_aside() {
    let d = driver(State::default());
    let track = d.rect(Target::suffix("/\"section\""));
    let caption = support::nodes(&d)
        .into_iter()
        .find(|(path, _)| path == "/<text>")
        .expect("the first caption")
        .1;
    assert!(track.right() <= caption.right(), "{track:?} in {caption:?}");
    let cells = section_cells(&d);
    assert!(cells.len() >= 5, "the five segments exist: {cells:?}");
    for label in ["text", "button", "choice", "value", "label"] {
        d.harness().get_by_label(label);
    }
    assert!(
        cells.iter().all(|cell| track.contains_rect(*cell)),
        "{cells:?} in {track:?}"
    );
}

#[test]
fn only_the_cells_section_shows_the_cell_controls() {
    use controls_gallery::Section;
    for (section, shown) in [
        (Section::Cells, true),
        (Section::Buttons, false),
        (Section::Choices, false),
        (Section::Values, false),
    ] {
        let d = driver(State {
            section,
            ..State::default()
        });
        for path in [
            "/\"text_size\"",
            "/\"padding\"",
            "/\"interact\"",
            "/\"label\"",
        ] {
            let present = support::nodes(&d).into_iter().any(|(p, _)| p == path);
            assert_eq!(present, shown, "{path} in {section:?}");
        }
        assert_eq!(
            d.harness()
                .query_all_by_label("Cell width (points)")
                .count()
                > 0,
            shown,
            "the width slider in {section:?}"
        );
        assert!(d.rect(Target::suffix("/\"section\"")).height() > 0.0);
        assert_eq!(
            d.harness()
                .query_all_by_label_contains("Blue: the rect")
                .count()
                > 0,
            shown,
            "the legend in {section:?}"
        );
    }
}

/// The name column of every row in `d`'s section.
fn name_cells(d: &Driver<State>) -> Vec<egui::Rect> {
    let width = controls_gallery::NAME_COLUMN * 4.0;
    support::nodes(d)
        .into_iter()
        .map(|(_, rect)| rect)
        .filter(|rect| (rect.width() - width).abs() < 0.01)
        .collect()
}

#[test]
fn every_row_name_fits_on_one_line() {
    use controls_gallery::Section;
    let line = controls_gallery::base_text().spec.line_height;
    for style in eggshell_ui_shadcn::ShadcnStyle::ALL {
        for section in [Section::Buttons, Section::Choices, Section::Values] {
            let mut d = driver(State {
                section,
                ..State::default()
            });
            common::set_style(d.ctx(), style);
            d.frames(4);
            let names = name_cells(&d);
            assert!(
                names.len() >= 6,
                "precondition: the {section:?} rows have name cells: {names:?}"
            );
            for name in names {
                assert!(
                    name.height() <= line + 0.01,
                    "{style:?} {section:?}: a name wraps in {name:?}"
                );
            }
        }
    }
}

/// A control's tooltip opens over the control and not beside it, where egui
/// still counts the pointer as near enough to hover it.
#[test]
fn a_controls_tooltip_opens_over_it_and_not_in_the_row_beside_it() {
    let mut d = driver(State {
        section: controls_gallery::Section::Choices,
        ..State::default()
    });
    let control = "/Choices/\"segmented\"";
    let track = d.rect(Target::suffix(control));
    let beside = egui::pos2(track.max.x + 4.0, track.center().y);
    let parts: Vec<_> = support::nodes(&d)
        .into_iter()
        .filter(|(path, _)| path.starts_with(control))
        .map(|(_, rect)| rect)
        .collect();
    assert!(parts.len() > 1, "precondition: the control and its cells");
    assert!(
        parts.iter().all(|rect| !rect.contains(beside)),
        "precondition: the point is outside every node of the control"
    );
    let open = |d: &Driver<State>| {
        support::nodes(d)
            .into_iter()
            .any(|(path, _)| path.contains("Tooltip("))
    };
    d.hover(beside).frames(37);
    assert!(!open(&d), "nothing opens beside the control");
    d.hover(egui::pos2(track.max.x - 4.0, track.center().y))
        .frames(37);
    assert!(open(&d), "its tooltip opens over it");
}
