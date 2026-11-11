//! `tabular-nums`: every digit as wide as the widest, so a readout sized by
//! its content stops moving as its value changes.

use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, Placed, TextSpec, Tw, tw};
use egui::{Event, FontData, FontDefinitions, FontFamily, OutputCommand, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use crate::support::{self, Rig, drag, rect_of};

#[path = "../../examples/common/mod.rs"]
mod common;

/// egui's own fonts give every digit one width, so the tests bring a font whose digits differ.
const PROPORTIONAL_DIGITS: &[u8] = include_bytes!("../fixtures/fonts/Inter-subset.ttf");

fn inter() -> FontFamily {
    FontFamily::Name("inter".into())
}

fn role() -> TextSpec {
    TextSpec {
        family: inter(),
        ..common::sized(14.0)
    }
}

fn headless() -> Headless {
    let mut base = common::base_text(egui::Theme::Light);
    base.spec.family = inter();
    Headless::new(vec2(600.0, 200.0))
        .base_text(base)
        .prepare(|ctx| {
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert(
                "inter".into(),
                FontData::from_static(PROPORTIONAL_DIGITS).into(),
            );
            fonts.families.insert(inter(), vec!["inter".into()]);
            ctx.set_fonts(fonts);
        })
}

/// One content-sized span per text, all with the classes `style`.
fn lay_out(texts: &[&str], style: Tw) -> Placed<()> {
    headless().layout(&(), |b, ()| {
        b.div(tw().flex().flex_col().items_start().role(role()), |b| {
            for (i, text) in texts.iter().enumerate() {
                b.span(i, style.clone().whitespace_nowrap(), (*text).to_owned());
            }
        });
    })
}

fn width(placed: &Placed<()>, i: usize) -> f32 {
    placed.at(i).rect().width()
}

#[test]
fn tabular_digits_lay_out_at_equal_width() {
    let texts = ["111", "888", "1.11", "8.88", "-1,111", "-8,888"];
    let proportional = lay_out(&texts, tw());
    assert!(
        width(&proportional, 0) < width(&proportional, 1) - 1.0,
        "the default font has narrow ones: {} vs {}",
        width(&proportional, 0),
        width(&proportional, 1)
    );
    assert!(width(&proportional, 2) < width(&proportional, 3) - 1.0);
    let tabular = lay_out(&texts, tw().tabular_nums());
    for (a, b) in [(0, 1), (2, 3), (4, 5)] {
        assert!(
            (width(&tabular, a) - width(&tabular, b)).abs() < 0.01,
            "{texts:?}: {} vs {}",
            width(&tabular, a),
            width(&tabular, b)
        );
    }
}

#[test]
fn every_digit_has_the_same_width() {
    let texts: Vec<String> = (0..10).map(|d| d.to_string().repeat(3)).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let tabular = lay_out(&refs, tw().tabular_nums());
    for i in 1..10 {
        assert!(
            (width(&tabular, 0) - width(&tabular, i)).abs() < 0.01,
            "digit {i}"
        );
    }
}

/// A readout whose value differs in every frame, in a row with a neighbour.
fn readout(value: &str, style: Tw) -> Placed<()> {
    headless().layout(&(), |b, ()| {
        b.div_id(
            "row",
            tw().flex().items_center().gap(2.0).role(role()),
            |b| {
                b.span("value", style.clone().whitespace_nowrap(), value.to_owned());
                b.span("unit", tw(), "ms");
            },
        );
    })
}

/// Values whose digits differ widely but whose character count is fixed.
const VALUES: [&str; 6] = ["111.11", "888.88", "010.10", "987.65", "123.45", "808.08"];

#[test]
fn a_tabular_readout_keeps_its_rect_as_the_value_changes() {
    let first = readout(VALUES[0], tw().tabular_nums());
    for value in &VALUES[1..] {
        let next = readout(value, tw().tabular_nums());
        let changes = first.diff(&next);
        assert!(!changes.iter().any(|c| c.moved()), "{value}: {changes:?}");
    }
}

#[test]
fn a_proportional_twin_moves() {
    let first = readout(VALUES[0], tw());
    let moved = VALUES[1..]
        .iter()
        .any(|value| first.diff(&readout(value, tw())).iter().any(|c| c.moved()));
    assert!(moved);
}

#[test]
fn numerals_are_inherited_and_a_child_can_override() {
    let placed = headless().layout(&(), |b, ()| {
        b.div(
            tw().flex()
                .flex_col()
                .items_start()
                .tabular_nums()
                .role(role()),
            |b| {
                b.span("a", tw(), "111");
                b.span("b", tw(), "888");
                b.span("c", tw().proportional_nums(), "111");
                b.span("d", tw().proportional_nums(), "888");
            },
        );
    });
    let w = |k: &'static str| placed.at(k).rect().width();
    assert!((w("a") - w("b")).abs() < 0.01);
    assert!(w("c") < w("d") - 1.0);
}

#[test]
fn the_later_class_wins() {
    let tabular_last = lay_out(&["111", "888"], tw().proportional_nums().tabular_nums());
    assert!((width(&tabular_last, 0) - width(&tabular_last, 1)).abs() < 0.01);
    let proportional_last = lay_out(&["111", "888"], tw().tabular_nums().proportional_nums());
    assert!(width(&proportional_last, 0) < width(&proportional_last, 1) - 1.0);
}

#[test]
fn other_characters_keep_their_advances() {
    let texts = ["v.-,: ms", "v.-,: ms"];
    let proportional = lay_out(&texts, tw());
    let tabular = lay_out(&texts, tw().tabular_nums());
    assert!((width(&proportional, 0) - width(&tabular, 0)).abs() < 0.01);
}

#[test]
fn mixed_text_still_lays_out() {
    let proportional = lay_out(&["v1.2 \u{2014} 340 ms"], tw());
    let tabular = lay_out(&["v1.2 \u{2014} 340 ms"], tw().tabular_nums());
    let (p, t) = (width(&proportional, 0), width(&tabular, 0));
    assert!(p > 40.0 && t > 40.0 && (p - t).abs() < 40.0, "{p} vs {t}");
}

#[test]
fn wrapping_is_unchanged_beyond_widths() {
    let wrapped = |digit: &str, style: Tw| {
        let text = [
            digit.repeat(3),
            digit.repeat(3),
            digit.repeat(3),
            digit.repeat(3),
        ]
        .join(" ");
        headless().layout(&(), move |b, ()| {
            b.div_id("box", tw().w_px(100.0).role(role()), |b| {
                b.span("p", style.clone(), text.clone());
            });
        })
    };
    let wide = wrapped("8", tw().tabular_nums());
    let narrow = wrapped("1", tw().tabular_nums());
    let line = wide.at("box").at("p").rect().height();
    let unwrapped = lay_out(&["888"], tw().tabular_nums())
        .at(0usize)
        .rect()
        .height();
    assert!(
        line > unwrapped * 1.5,
        "the text wrapped: {line} vs {unwrapped}"
    );
    assert_eq!(
        wide.at("box").at("p").rect(),
        narrow.at("box").at("p").rect()
    );
    let words = lay_out(
        &["888 888 888 888"],
        tw().tabular_nums().whitespace_nowrap(),
    );
    assert!(width(&words, 0) > 100.0, "it would not fit unwrapped");
}

const TEXT: &str = "Reading 1,234.56 ms";

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("box", tw().p(4.0).tabular_nums(), |b| b.text(tw(), TEXT));
}

fn selectable() -> Harness<'static, Rig<()>> {
    let tokens = eggshell_ui_shadcn::Tokens::light();
    let mut harness = support::harness(
        vec2(300.0, 100.0),
        eggshell_ui::TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        view,
    );
    harness.run_steps(6);
    harness
}

#[test]
fn selecting_and_copying_read_the_original_text() {
    let mut harness = selectable();
    let rect = rect_of(&harness, "/\"box\"/<text>");
    assert!(rect.width() > 60.0);
    drag(
        &mut harness,
        rect.left_center() + vec2(1.0, 0.0),
        rect.right_center() - vec2(1.0, 0.0),
    );
    harness.event(Event::Copy);
    harness.step();
    let copied =
        harness
            .output()
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            });
    assert_eq!(copied.as_deref(), Some(TEXT));
}

#[test]
fn accessibility_reads_the_original_text() {
    let harness = selectable();
    let _ = harness.get_by_label(TEXT);
}
