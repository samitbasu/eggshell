//! The hello_egui ports through the full path: layout relations between
//! their items, and the controls and widgets inside them driven by pointer
//! and keyboard events.

use egui::{Rect, Vec2, vec2};
use egui_kittest::Harness;
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

use support::{Rig, click_node, drag, focused_node, rect_of};

fn harness<A: 'static>(size: Vec2, app: A, view: support::View<A>) -> Harness<'static, Rig<A>> {
    let mut harness = support::harness(size, common::base_text(egui::Theme::Light), app, view);
    harness.run_steps(3);
    harness
}

fn type_text<A>(harness: &mut Harness<'_, Rig<A>>, text: &str) {
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run_steps(3);
}

fn painted_text<A>(harness: &Harness<'_, Rig<A>>) -> Vec<String> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

mod demos {
    use super::*;
    use flex_demos::{ALIGNS, Align, Direction, Part, State};

    fn demos(state: State) -> Harness<'static, Rig<State>> {
        harness(vec2(1280.0, 900.0), state, flex_demos::layout)
    }

    fn rect(harness: &Harness<'_, Rig<State>>, part: Part) -> Rect {
        rect_of(harness, &format!("/{part:?}"))
    }

    #[test]
    fn horizontal_items_sit_in_a_row_and_each_aligns_across_it() {
        let harness = demos(State::default());
        let [start, center, end, stretch] = ALIGNS.map(|a| rect(&harness, Part::Aligned(a)));
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
        let mut harness = demos(State::default());
        click_node(&mut harness, "/Direction/1");
        harness.run_steps(3);
        assert_eq!(harness.state().app.direction, Direction::Vertical);
        let [start, center, end, stretch] = ALIGNS.map(|a| rect(&harness, Part::Aligned(a)));
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
        let mut harness = demos(State::default());
        let before = rect(&harness, Part::Aligned(Align::Start)).width();
        click_node(&mut harness, "/Grow/Box");
        harness.run_steps(3);
        assert!(harness.state().app.grow);
        let after = rect(&harness, Part::Aligned(Align::Start)).width();
        assert!(after > before + 5.0, "{before} -> {after}");
    }

    #[test]
    fn grow_factors_share_the_free_space_in_proportion() {
        let harness = demos(State::default());
        let widths: Vec<f32> = (0..4)
            .map(|factor| rect(&harness, Part::Growing(factor)).width())
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
        let harness = demos(State::default());
        let widths: Vec<f32> = (0..4)
            .map(|index| rect(&harness, Part::Based(index)).width())
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] < pair[1]),
            "{widths:?}"
        );
        assert!(widths[3] >= 200.0, "{widths:?}");
    }

    #[test]
    fn the_select_moves_the_content_inside_each_item() {
        let mut harness = demos(State::default());
        let stretch = Part::SelfContent(Align::Stretch);
        let content = |harness: &Harness<'_, Rig<State>>| {
            let item = rect(harness, stretch);
            let text = rect_of(harness, &format!("/{stretch:?}/<text>"));
            (item, text)
        };
        let (item, text) = content(&harness);
        assert!(
            item.height() > text.height() + 30.0,
            "precondition: room to move"
        );
        assert!(
            text.min.y - item.min.y < 10.0,
            "left top: {item:?} {text:?}"
        );

        click_node(&mut harness, "/Content");
        harness.run_steps(3);
        harness.get_by_label("Right bottom").scroll_to_me();
        harness.run_steps(3);
        harness.get_by_label("Right bottom").click();
        harness.run_steps(3);
        assert_eq!(harness.state().app.anchor, 8);
        let (item, text) = content(&harness);
        assert!(item.max.y - text.max.y < 10.0, "bottom: {item:?} {text:?}");
        assert!(item.max.x - text.max.x < 10.0, "right: {item:?} {text:?}");
    }
}

mod example {
    use super::*;
    use flex_example::State;

    fn example(width: f32) -> Harness<'static, Rig<State>> {
        harness(vec2(width, 700.0), State::default(), flex_example::layout)
    }

    /// The distinct tops of the boxes of the wrapping row.
    fn lines(harness: &Harness<'_, Rig<State>>) -> Vec<i32> {
        let mut tops: Vec<i32> = harness
            .state()
            .nodes
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
        let harness = example(900.0);
        let toggle = rect_of(&harness, "/Toggle");
        let line: Vec<Rect> = harness
            .state()
            .nodes
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
        let mut harness = example(900.0);
        let width = |harness: &Harness<'_, Rig<State>>| rect_of(harness, "/Toggle").width();
        let before = width(&harness);
        click_node(&mut harness, "/Toggle");
        harness.run_steps(3);
        assert!(harness.state().app.toggle);
        assert_ne!(width(&harness), before, "the row re-wrapped around it");
    }

    #[test]
    fn the_text_field_edits_and_the_slider_drags() {
        let mut harness = example(900.0);
        click_node(&mut harness, "/Text");
        harness.run_steps(2);
        type_text(&mut harness, "!!");
        assert_eq!(harness.state().app.text, "Hello, world!!!");

        let rail = rect_of(&harness, "/Slider/Track");
        assert!(rail.width() > 60.0, "precondition: a real rail");
        drag(
            &mut harness,
            rail.center(),
            rail.right_center() + vec2(30.0, 0.0),
        );
        assert!(
            harness.state().app.level > 900.0,
            "{}",
            harness.state().app.level
        );
    }

    #[test]
    fn a_checkbox_toggles() {
        let mut harness = example(900.0);
        click_node(&mut harness, "/Checkbox/Box");
        harness.run_steps(2);
        assert!(harness.state().app.checked);
    }
}

mod signup {
    use super::*;
    use signup_form::{Part, Progress, State};

    fn signup(width: f32) -> Harness<'static, Rig<State>> {
        harness(vec2(width, 900.0), State::default(), signup_form::layout)
    }

    fn fill<A>(harness: &mut Harness<'_, Rig<A>>, part: Part, text: &str) {
        click_node(harness, &format!("/Form/{part:?}"));
        harness.run_steps(2);
        type_text(harness, text);
    }

    /// Every field, required or not.
    fn all_parts() -> Vec<Part> {
        let mut parts = signup_form::Fields::REQUIRED.to_vec();
        parts.push(Part::Newsletter);
        parts
    }

    /// How far down the form the Submit button is.
    fn height(harness: &Harness<'_, Rig<State>>) -> f32 {
        rect_of(harness, "/Form/Submit").min.y
    }

    fn fill_validly(harness: &mut Harness<'_, Rig<State>>) {
        fill(harness, Part::FirstName, "Ada");
        fill(harness, Part::LastName, "Lovelace");
        fill(harness, Part::Email, "ada@example.com");
        fill(harness, Part::Password, "Sup3r-secret");
        fill(harness, Part::Repeat, "Sup3r-secret");
        click_node(harness, "/Form/Terms/Box");
        harness.run_steps(2);
        click_node(harness, "/Form/Age/Box/Field");
        harness.run_steps(2);
        type_text(harness, "30");
    }

    #[test]
    fn errors_show_for_a_field_once_it_has_been_changed_without_moving_the_layout() {
        let mut harness = signup(640.0);
        for part in all_parts() {
            assert_eq!(harness.state().app.visible_error(part), None, "{part:?}");
        }
        let resting = height(&harness);
        let password = rect_of(&harness, "/Form/Password").min.y;

        fill(&mut harness, Part::Email, "x");
        assert_eq!(
            harness.state().app.visible_error(Part::Email),
            Some("Invalid email")
        );
        assert_eq!(harness.state().app.visible_error(Part::FirstName), None);
        assert_eq!(
            rect_of(&harness, "/Form/Password").min.y,
            password,
            "a one-line error fills the line reserved for it, so the rows below stay put"
        );
        assert_eq!(height(&harness), resting);

        type_text(&mut harness, "@example.com");
        assert_eq!(harness.state().app.visible_error(Part::Email), None);
        assert_eq!(height(&harness), resting, "and clearing it changes nothing");
    }

    #[test]
    fn submitting_something_invalid_shows_every_error() {
        let mut harness = signup(640.0);
        click_node(&mut harness, "/Form/Submit");
        harness.run_steps(3);
        assert_eq!(harness.state().app.progress, Progress::Rejected);
        for part in all_parts() {
            let shown = harness.state().app.visible_error(part);
            let required = signup_form::Fields::REQUIRED.contains(&part);
            assert_eq!(shown.is_some(), required, "{part:?}");
        }
    }

    #[test]
    fn mismatched_passwords_are_reported() {
        let mut harness = signup(640.0);
        fill(&mut harness, Part::Password, "Sup3r-secret");
        fill(&mut harness, Part::Repeat, "Sup3r-secreT");
        assert_eq!(
            harness.state().app.visible_error(Part::Repeat),
            Some("Passwords do not match")
        );
        assert_eq!(harness.state().app.visible_error(Part::Password), None);
    }

    #[test]
    fn passwords_are_painted_as_dots() {
        let mut harness = signup(640.0);
        fill(&mut harness, Part::Password, "Sup3r-secret");
        let painted = painted_text(&harness).concat();
        assert!(!painted.contains("Sup3r"), "{painted:?}");
        assert!(painted.contains('•'));
    }

    #[test]
    fn a_valid_form_submits_and_resets() {
        let mut harness = signup(640.0);
        fill_validly(&mut harness);
        assert!(
            harness.state().app.fields.is_valid(),
            "{:?}",
            harness.state().app.fields
        );
        assert!(harness.state().app.fields.age >= 18.0, "dragged an age");

        click_node(&mut harness, "/Form/Submit");
        harness.run_steps(3);
        assert_eq!(harness.state().app.progress, Progress::Submitted);
        click_node(&mut harness, "/Thanks/Reset");
        harness.run_steps(3);
        assert_eq!(harness.state().app.progress, Progress::Editing);
        assert!(harness.state().app.fields.first_name.is_empty());
    }

    #[test]
    fn the_age_shows_whole_numbers() {
        let mut harness = signup(640.0);
        harness.state_mut().app.fields.age = 42.7;
        harness.run_steps(3);
        let painted = painted_text(&harness);
        assert!(painted.iter().any(|text| text == "43"), "{painted:?}");
    }

    #[test]
    fn the_name_columns_stack_when_the_window_is_narrow() {
        let wide = signup(900.0);
        let (first, last) = (
            rect_of(&wide, "/Form/FirstName"),
            rect_of(&wide, "/Form/LastName"),
        );
        assert_eq!(first.min.y, last.min.y, "side by side: {first:?} {last:?}");
        assert!(first.max.x < last.min.x);

        let narrow = signup(360.0);
        let (first, last) = (
            rect_of(&narrow, "/Form/FirstName"),
            rect_of(&narrow, "/Form/LastName"),
        );
        assert!(last.min.y > first.max.y, "stacked: {first:?} {last:?}");
    }

    #[test]
    fn tab_walks_the_form_in_order() {
        let mut harness = signup(640.0);
        let mut order = Vec::new();
        support::past_bar(&mut harness);
        for _ in 0..12 {
            harness.key_press(egui::Key::Tab);
            harness.run_steps(2);
            let Some(stop) = focused_node(&harness) else {
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
