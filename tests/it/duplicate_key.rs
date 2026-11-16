//! A duplicate key path is reported at the builder call that repeats it.

use eggshell_ui::testing::{Headless, assert_reported};
use eggshell_ui::{Builder, Problem, Report, tw};
use egui::vec2;

/// The duplicate-key report that `build` raises.
fn report_of(build: impl FnOnce(&mut Builder<'_, '_, ()>)) -> Report {
    let mut headless = Headless::new(vec2(200.0, 100.0));
    let mut build = Some(build);
    headless.layout(&(), |b, _| {
        if let Some(build) = build.take() {
            build(b);
        }
    });
    assert_reported(headless.ctx(), Problem::DuplicateKey)
}

fn assert_names_call(report: &Report, line: u32) {
    assert!(
        report.message.contains("duplicate key path"),
        "{}",
        report.message
    );
    assert_eq!(
        (report.at.file(), report.at.line()),
        (file!(), line),
        "reported at the repeating call"
    );
}

#[test]
fn a_duplicate_div_id_names_the_second_call() {
    let mut line = 0;
    let report = report_of(|b| {
        b.div_id("row", tw(), |_| {});
        line = line!() + 1;
        b.div_id("row", tw(), |_| {});
    });
    assert_names_call(&report, line);
}

#[test]
fn a_duplicate_control_names_the_call_that_finishes_it() {
    let mut line = 0;
    let report = report_of(|b| {
        b.slider("volume", 1.0, 0.0..=2.0).on_change(|_, _| {});
        line = line!() + 1;
        b.slider("volume", 1.0, 0.0..=2.0).on_change(|_, _| {});
    });
    assert_names_call(&report, line);
    let report = report_of(|b| {
        b.button("ok", "OK").on_click(|_| {});
        line = line!() + 1;
        b.button("ok", "OK").on_click(|_| {});
    });
    assert_names_call(&report, line);
    let report = report_of(|b| {
        b.text_field("name", "").on_change(|_, _| {});
        line = line!() + 1;
        b.text_field("name", "").on_change(|_, _| {});
    });
    assert_names_call(&report, line);
    let report = report_of(|b| {
        b.segmented("seg", 0, [(0, "A"), (1, "B")])
            .on_change(|_, _| {});
        line = line!() + 2;
        b.segmented("seg", 0, [(0, "A"), (1, "B")])
            .on_change(|_, _| {});
    });
    assert_names_call(&report, line);
    let report = report_of(|b| {
        b.button("ok", "OK").on_click_with(|_, _| {});
        line = line!() + 1;
        b.button("ok", "OK").on_click_with(|_, _| {});
    });
    assert_names_call(&report, line);
}
