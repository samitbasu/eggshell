//! `min_w_text(Reserve)` (#129): a node as wide as the widest of a set of
//! templates, measured in its text style through egui, so that its
//! neighbours never move whichever of them it shows.

use eggshell_ui::testing::{Headless, assert_no_reports};
use eggshell_ui::{Builder, Placed, Reserve, Tw, tw};
use egui::vec2;
use egui_kittest::kittest::Queryable as _;

#[path = "../../examples/common/mod.rs"]
mod common;

const LINK: [&str; 4] = ["Not connected", "Listening", "Live 00.0 kHz", "Lost 000 s"];

fn headless() -> Headless {
    Headless::new(vec2(800.0, 100.0)).base_text(common::base_text(egui::Theme::Light))
}

/// A row: the word in `word`'s classes, then a neighbour.
fn row(headless: &mut Headless, word: &str, classes: Tw) -> Placed<()> {
    let word = word.to_owned();
    headless.layout(&(), move |b: &mut Builder<'_, '_, ()>, ()| {
        b.div(tw().flex().items_center().gap(2.0), |b| {
            b.span("word", classes.clone(), word.clone());
            b.span("after", tw(), "after");
        });
    })
}

fn width(placed: &Placed<()>, key: &'static str) -> f32 {
    placed.at(key).rect().width()
}

fn reserved() -> Tw {
    tw().min_w_text(Reserve::widest_of(LINK))
}

/// The widest of `templates` laid out by egui in the row's text with tabular
/// numerals, through a node of its own.
fn widest_alone(headless: &mut Headless, templates: &[&str]) -> f32 {
    templates
        .iter()
        .map(|template| width(&row(headless, template, tw().tabular_nums()), "word"))
        .fold(0.0, f32::max)
}

#[test]
fn a_neighbour_stays_put_whichever_template_shows() {
    let mut headless = headless();
    let natural: Vec<f32> = LINK
        .iter()
        .map(|word| width(&row(&mut headless, word, tw()), "word"))
        .collect();
    assert!(
        natural
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() > 1.0),
        "precondition: the words' own widths differ: {natural:?}"
    );
    let after: Vec<f32> = LINK
        .iter()
        .map(|word| {
            row(&mut headless, word, reserved())
                .at("after")
                .rect()
                .left()
        })
        .collect();
    assert!(
        after.iter().all(|&x| x == after[0]),
        "the neighbour moved: {after:?}"
    );
    assert_no_reports(headless.ctx());
}

#[test]
fn the_reservation_is_the_widest_template_as_egui_lays_it_out() {
    let mut headless = headless();
    let expected = widest_alone(&mut headless, &LINK);
    assert_eq!(
        width(&row(&mut headless, "Listening", reserved()), "word"),
        expected
    );
}

#[test]
fn the_widest_template_counts_wherever_it_is_in_the_set() {
    let mut headless = headless();
    let set = ["Off", "On", "Reconnecting"];
    let first = widest_alone(&mut headless, &set[..1]);
    let widest = widest_alone(&mut headless, &set);
    assert!(widest > first + 1.0, "precondition: the last is the widest");
    let placed = row(
        &mut headless,
        "On",
        tw().min_w_text(Reserve::widest_of(set)),
    );
    assert_eq!(width(&placed, "word"), widest);
}

#[test]
fn a_digit_in_a_template_stands_for_any_digit() {
    let mut headless = headless();
    let template = tw().min_w_text(Reserve::widest_of(["Lost 000 s"]));
    let widths: Vec<f32> = ["Lost 000 s", "Lost 987 s", "Lost 111 s"]
        .iter()
        .map(|text| width(&row(&mut headless, text, template.clone()), "word"))
        .collect();
    assert!(widths.iter().all(|&w| w == widths[0]), "{widths:?}");
    assert_no_reports(headless.ctx());
}

#[test]
fn digits_reserves_n_tabular_zeros_and_zero_reserves_nothing() {
    let mut headless = headless();
    let digits = width(
        &row(&mut headless, "7", tw().min_w_text(Reserve::digits(6))),
        "word",
    );
    assert_eq!(digits, widest_alone(&mut headless, &["000000"]));
    let none = width(
        &row(&mut headless, "7", tw().min_w_text(Reserve::digits(0))),
        "word",
    );
    let empty = Reserve::widest_of(Vec::<&'static str>::new());
    let unset = width(&row(&mut headless, "7", tw().min_w_text(empty)), "word");
    let natural = width(&row(&mut headless, "7", tw()), "word");
    assert_eq!((none, unset), (natural, natural));
}

#[test]
fn the_reservation_is_a_content_width_inside_padding_and_border() {
    let mut headless = headless();
    let content = widest_alone(&mut headless, &LINK);
    let framed = reserved().px(2.0).border(3.0);
    let placed = row(&mut headless, "Listening", framed);
    assert_eq!(width(&placed, "word"), content + 2.0 * 8.0 + 2.0 * 3.0);
}

#[test]
fn a_later_min_width_class_wins_either_way() {
    let mut headless = headless();
    let content = widest_alone(&mut headless, &LINK);
    let px_over = reserved().min_w_px(300.0);
    assert_eq!(width(&row(&mut headless, "x", px_over), "word"), 300.0);
    let text_over = tw().min_w_px(300.0).merge(reserved());
    assert_eq!(width(&row(&mut headless, "x", text_over), "word"), content);
}

#[cfg(debug_assertions)]
#[test]
fn text_wider_than_the_reservation_grows_the_node_and_is_reported() {
    let mut headless = headless();
    let longer = "Reconnecting after a timeout";
    let natural = width(&row(&mut headless, longer, tw()), "word");
    let placed = row(&mut headless, longer, reserved());
    assert_eq!(width(&placed, "word"), natural, "it grows to its text");
    let report =
        eggshell_ui::testing::assert_reported(headless.ctx(), eggshell_ui::Problem::OutgrewReserve);
    assert!(
        report.message.contains(longer) && report.message.contains("Live 00.0 kHz"),
        "{}",
        report.message
    );
}

#[test]
fn only_the_text_shown_is_announced() {
    fn view(b: &mut Builder<'_, '_, ()>, (): &()) {
        b.span(
            "word",
            tw().min_w_text(Reserve::widest_of(LINK)),
            "Listening",
        );
    }
    let harness = crate::support::harness(
        vec2(400.0, 100.0),
        common::base_text(egui::Theme::Light),
        (),
        view,
    );
    assert!(harness.query_by_label("Listening").is_some());
    for template in ["Not connected", "Live 00.0 kHz", "Lost 000 s"] {
        assert!(
            harness.query_by_label(template).is_none(),
            "{template} is not in the tree"
        );
    }
}
