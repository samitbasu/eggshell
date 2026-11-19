//! The hello_egui ports through the full path: layout relations between
//! their items, and the controls and widgets inside them driven by pointer
//! and keyboard events.

use eggshell_ui::testing::{Driver, Target};
use egui::{Rect, Vec2, vec2};
use egui_kittest::kittest::Queryable as _;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flex_demos.rs"]
mod flex_demos;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flex_example.rs"]
mod flex_example;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/signup_form.rs"]
mod signup_form;

use crate::support;

use support::focused_node;

fn harness<A: 'static>(size: Vec2, app: A, view: support::View<A>) -> Driver<A> {
    let mut d = support::driver(size, common::base_text(egui::Theme::Light), app, view).build();
    d.frames(3);
    d
}

fn painted_text<A>(d: &Driver<A>) -> Vec<String> {
    d.painted()
        .texts()
        .map(|text| text.text().to_owned())
        .collect()
}

mod demos {
    use super::*;
    use flex_demos::{ALIGNS, Align, Direction, Part, State};

    fn demos(state: State) -> Driver<State> {
        harness(vec2(1280.0, 900.0), state, flex_demos::layout)
    }

    fn rect(d: &Driver<State>, part: Part) -> Rect {
        d.rect(Target::suffix(&format!("/{part:?}")))
    }

    #[test]
    fn horizontal_items_sit_in_a_row_and_each_aligns_across_it() {
        let d = demos(State::default());
        let [start, center, end, stretch] = ALIGNS.map(|a| rect(&d, Part::Aligned(a)));
        assert!(
            start.max.x < center.min.x && center.max.x < end.min.x && end.max.x < stretch.min.x
        );
        assert!(
            stretch.height() > start.height(),
            "precondition: the line is taller than the small items"
        );
        assert_eq!(start.min.y, stretch.min.y, "start at the top");
        assert_eq!(end.max.y, stretch.max.y, "end at the bottom");
        assert!(
            center.center().y > start.center().y && center.center().y < end.center().y,
            "centre between them: {start:?} {center:?} {end:?}"
        );
    }

    #[test]
    fn the_vertical_direction_stacks_them_and_aligns_across_the_width() {
        let mut d = demos(State::default());
        d.click(Target::suffix("/Direction/1")).frames(3);
        assert_eq!(d.app().direction, Direction::Vertical);
        let [start, center, end, stretch] = ALIGNS.map(|a| rect(&d, Part::Aligned(a)));
        assert!(
            start.max.y < center.min.y && center.max.y < end.min.y && end.max.y < stretch.min.y
        );
        assert!(
            stretch.width() > start.width(),
            "precondition: the column is wider than the small items"
        );
        assert_eq!(start.min.x, stretch.min.x);
        assert_eq!(end.max.x, stretch.max.x);
    }

    #[test]
    fn the_grow_checkbox_makes_the_items_fill_their_container() {
        let mut d = demos(State::default());
        let before = rect(&d, Part::Aligned(Align::Start)).width();
        d.click(Target::suffix("/Grow/Box")).frames(3);
        assert!(d.app().grow);
        let after = rect(&d, Part::Aligned(Align::Start)).width();
        assert!(after > before + 5.0, "{before} -> {after}");
    }

    #[test]
    fn grow_factors_share_the_free_space_in_proportion() {
        let d = demos(State::default());
        let widths: Vec<f32> = (0..4)
            .map(|factor| {
                let font = common::base_text(egui::Theme::Light).spec.font_id(d.ctx());
                let digit = d.ctx().fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(factor.to_string(), font, egui::Color32::WHITE)
                        .size()
                        .x
                });
                rect(&d, Part::Growing(factor)).width() - digit
            })
            .collect();
        let unit = widths[1] - widths[0];
        assert!(unit > 5.0, "precondition: free space to share: {widths:?}");
        assert!(
            (widths[2] - widths[0] - 2.0 * unit).abs() < 1.0,
            "{widths:?}"
        );
        assert!(
            (widths[3] - widths[0] - 3.0 * unit).abs() < 1.0,
            "{widths:?}"
        );
    }

    #[test]
    fn bases_set_the_starting_widths() {
        let d = demos(State::default());
        let widths: Vec<f32> = (0..4)
            .map(|index| rect(&d, Part::Based(index)).width())
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] < pair[1]),
            "{widths:?}"
        );
        assert!(widths[3] >= 200.0, "{widths:?}");
    }

    #[test]
    fn the_select_moves_the_content_inside_each_item() {
        let mut d = demos(State::default());
        let stretch = Part::SelfContent(Align::Stretch);
        let content = |d: &Driver<State>| {
            let item = rect(d, stretch);
            let text = d.rect(Target::suffix(&format!("/{stretch:?}/<text>")));
            (item, text)
        };
        let (item, text) = content(&d);
        assert!(
            item.height() > text.height() + 30.0,
            "precondition: room to move"
        );
        assert!(
            text.min.y - item.min.y < 10.0,
            "left top: {item:?} {text:?}"
        );

        d.click(Target::suffix("/Content")).frames(3);
        d.harness().get_by_label("Right bottom").scroll_to_me();
        d.frames(3);
        d.harness().get_by_label("Right bottom").click();
        d.frames(3);
        assert_eq!(d.app().anchor, 8);
        let (item, text) = content(&d);
        assert!(item.max.y - text.max.y < 10.0, "bottom: {item:?} {text:?}");
        assert!(item.max.x - text.max.x < 10.0, "right: {item:?} {text:?}");
    }
}

mod example {
    use super::*;
    use flex_example::State;

    fn example(width: f32) -> Driver<State> {
        harness(vec2(width, 700.0), State::default(), flex_example::layout)
    }

    /// The distinct tops of the boxes of the wrapping row.
    fn lines(d: &Driver<State>) -> Vec<i32> {
        let mut tops: Vec<i32> = support::nodes(d)
            .iter()
            .filter(|(path, _)| path == "/<div>")
            .map(|(_, rect)| rect.min.y.round() as i32)
            .collect();
        tops.sort_unstable();
        tops.dedup();
        tops
    }

    #[test]
    fn the_row_wraps_and_a_narrower_window_wraps_more() {
        let wide = lines(&example(1400.0)).len();
        let narrow = lines(&example(500.0)).len();
        assert!(wide >= 2, "precondition: even a wide window wraps: {wide}");
        assert!(narrow > wide, "{wide} lines wide, {narrow} narrow");
    }

    #[test]
    fn growing_items_reach_the_end_of_their_line() {
        let d = example(900.0);
        let toggle = d.rect(Target::suffix("/Toggle"));
        let line: Vec<Rect> = support::nodes(&d)
            .iter()
            .filter(|(path, rect)| path == "/<div>" && (rect.min.y - toggle.min.y).abs() < 1.0)
            .map(|(_, rect)| *rect)
            .collect();
        assert!(line.len() > 2, "precondition: several boxes on its line");
        let edge = line.iter().map(|rect| rect.max.x).fold(0.0, f32::max);
        assert!(toggle.max.x <= edge);
        assert!(edge > 800.0, "the line fills the window: {edge}");
    }

    #[test]
    fn the_toggle_button_changes_its_label_and_its_size() {
        let mut d = example(900.0);
        let width = |d: &Driver<State>| d.rect(Target::suffix("/Toggle")).width();
        let before = width(&d);
        d.click(Target::suffix("/Toggle")).frames(3);
        assert!(d.app().toggle);
        assert_ne!(width(&d), before, "the row re-wrapped around it");
    }

    #[test]
    fn the_text_field_edits_and_the_slider_drags() {
        let mut d = example(900.0);
        d.click(Target::suffix("/Text")).frames(2);
        d.type_text("!!").frames(2);
        assert_eq!(d.app().text, "Hello, world!!!");

        let rail = d.rect(Target::suffix("/Slider/Track"));
        assert!(rail.width() > 60.0, "precondition: a real rail");
        d.drag(rail.center(), rail.right_center() + vec2(30.0, 0.0));
        assert!(d.app().level > 900.0, "{}", d.app().level);
    }

    #[test]
    fn a_checkbox_toggles() {
        let mut d = example(900.0);
        d.click(Target::suffix("/Checkbox/Box")).frames(2);
        assert!(d.app().checked);
    }
}

mod signup {
    use super::*;
    use signup_form::{Part, Progress, State};

    fn signup(width: f32) -> Driver<State> {
        harness(vec2(width, 900.0), State::default(), signup_form::layout)
    }

    fn fill<A>(d: &mut Driver<A>, part: Part, text: &str) {
        d.click(Target::suffix(&format!("/Form/{part:?}")))
            .frames(2);
        d.type_text(text).frames(2);
    }

    /// Every field, required or not.
    fn all_parts() -> Vec<Part> {
        let mut parts = signup_form::Fields::REQUIRED.to_vec();
        parts.push(Part::Newsletter);
        parts
    }

    /// How far down the form the Submit button is.
    fn height(d: &Driver<State>) -> f32 {
        d.rect(Target::suffix("/Form/Submit")).min.y
    }

    fn fill_validly(d: &mut Driver<State>) {
        fill(d, Part::FirstName, "Ada");
        fill(d, Part::LastName, "Lovelace");
        fill(d, Part::Email, "ada@example.com");
        fill(d, Part::Password, "Sup3r-secret");
        fill(d, Part::Repeat, "Sup3r-secret");
        d.click(Target::suffix("/Form/Terms/Box")).frames(2);
        d.click(Target::suffix("/Form/Age/Box/Field")).frames(2);
        d.type_text("30").frames(2);
    }

    #[test]
    fn errors_show_for_a_field_once_it_has_been_changed_without_moving_the_layout() {
        let mut d = signup(640.0);
        for part in all_parts() {
            assert_eq!(d.app().visible_error(part), None, "{part:?}");
        }
        let resting = height(&d);
        let password = d.rect(Target::suffix("/Form/Password")).min.y;

        fill(&mut d, Part::Email, "x");
        assert_eq!(d.app().visible_error(Part::Email), Some("Invalid email"));
        assert_eq!(d.app().visible_error(Part::FirstName), None);
        assert_eq!(
            d.rect(Target::suffix("/Form/Password")).min.y,
            password,
            "a one-line error fills the line reserved for it, so the rows below stay put"
        );
        assert_eq!(height(&d), resting);

        d.type_text("@example.com").frames(2);
        assert_eq!(d.app().visible_error(Part::Email), None);
        assert_eq!(height(&d), resting, "and clearing it changes nothing");
    }

    #[test]
    fn submitting_something_invalid_shows_every_error() {
        let mut d = signup(640.0);
        d.click(Target::suffix("/Form/Submit")).frames(3);
        assert_eq!(d.app().progress, Progress::Rejected);
        for part in all_parts() {
            let shown = d.app().visible_error(part);
            let required = signup_form::Fields::REQUIRED.contains(&part);
            assert_eq!(shown.is_some(), required, "{part:?}");
        }
    }

    #[test]
    fn mismatched_passwords_are_reported() {
        let mut d = signup(640.0);
        fill(&mut d, Part::Password, "Sup3r-secret");
        fill(&mut d, Part::Repeat, "Sup3r-secreT");
        assert_eq!(
            d.app().visible_error(Part::Repeat),
            Some("Passwords do not match")
        );
        assert_eq!(d.app().visible_error(Part::Password), None);
    }

    #[test]
    fn passwords_are_painted_as_dots() {
        let mut d = signup(640.0);
        fill(&mut d, Part::Password, "Sup3r-secret");
        let painted = painted_text(&d).concat();
        assert!(!painted.contains("Sup3r"), "{painted:?}");
        assert!(painted.contains('•'));
    }

    #[test]
    fn a_valid_form_submits_and_resets() {
        let mut d = signup(640.0);
        fill_validly(&mut d);
        assert!(d.app().fields.is_valid(), "{:?}", d.app().fields);
        assert!(d.app().fields.age >= 18.0, "dragged an age");

        d.click(Target::suffix("/Form/Submit")).frames(3);
        assert_eq!(d.app().progress, Progress::Submitted);
        d.click(Target::suffix("/Thanks/Reset")).frames(3);
        assert_eq!(d.app().progress, Progress::Editing);
        assert!(d.app().fields.first_name.is_empty());
    }

    #[test]
    fn the_age_shows_whole_numbers() {
        let mut d = signup(640.0);
        d.app_mut().fields.age = 42.7;
        d.frames(3);
        let painted = painted_text(&d);
        assert!(painted.iter().any(|text| text == "43"), "{painted:?}");
    }

    #[test]
    fn the_name_columns_stack_when_the_window_is_narrow() {
        let wide = signup(900.0);
        let (first, last) = (
            wide.rect(Target::suffix("/Form/FirstName")),
            wide.rect(Target::suffix("/Form/LastName")),
        );
        assert_eq!(first.min.y, last.min.y, "side by side: {first:?} {last:?}");
        assert!(first.max.x < last.min.x);

        let narrow = signup(360.0);
        let (first, last) = (
            narrow.rect(Target::suffix("/Form/FirstName")),
            narrow.rect(Target::suffix("/Form/LastName")),
        );
        assert!(last.min.y > first.max.y, "stacked: {first:?} {last:?}");
    }

    #[test]
    fn tab_walks_the_form_in_order() {
        let mut d = signup(640.0);
        let mut order = Vec::new();
        support::drive_past_bar(&mut d);
        for _ in 0..12 {
            d.press(egui::Key::Tab).frames(1);
            let Some(stop) = focused_node(&d) else {
                break;
            };
            order.push(stop);
        }
        let expected: Vec<String> = [
            Part::FirstName,
            Part::LastName,
            Part::Email,
            Part::Password,
            Part::Repeat,
        ]
        .map(|part| format!("/Form/{part:?}/\"field\""))
        .into_iter()
        .chain(
            [
                "/Form/Age/Box/Field",
                "/Form/Terms/Box",
                "/Form/Newsletter/Box",
                "/Form/Submit",
            ]
            .map(String::from),
        )
        .collect();
        assert_eq!(order[..expected.len()], expected[..]);
    }
}
