//! Typeset (#352): shadcn's prose parameters generate the Markdown widget's
//! style, measured on the page as drawn. Every expected length is derived
//! from typeset.css at shadcn's defaults: 15 pt, leading 1.75, flow 1.25em.

use eggshell_ui::{Builder, Style, TextSpec, TextStyle, Weight, tw};
use eggshell_ui_extras::markdown::{Directives, MarkdownExt, MarkdownStyle, Typeset, TypesetInk};
use egui::{Color32, FontFamily, Rect, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, rect_of};

const BASE: f32 = 15.0;
const FLOW: f32 = 1.25 * BASE;
const H2: f32 = 1.25 * BASE;

const SOURCE: &str = "First paragraph.

Second paragraph.

## Section

After heading.

---

## After rule

```probe
```
";

/// The page's own text, which a directive's output keeps.
const PAGE_TEXT: TextSpec = TextSpec {
    size: 13.0,
    line_height: 17.0,
    family: FontFamily::Proportional,
    weight: Weight::Normal,
};

fn ink() -> TypesetInk {
    TypesetInk {
        foreground: Color32::from_gray(10),
        muted: Color32::from_gray(110),
        rule: Color32::from_gray(220),
        code_fill: Color32::from_gray(245),
        radius: 10.0,
    }
}

fn install(ctx: &egui::Context, _: &()) {
    for theme in [Theme::Light, Theme::Dark] {
        Typeset::default()
            .markdown_style(&ink())
            .install_for(ctx, theme);
    }
}

fn view<'f>(b: &mut Builder<'_, 'f, ()>, app: &'f ()) {
    let directives = Directives::new().with("probe", |s, _, _| {
        s.span("probe", tw(), "probe");
    });
    b.div(tw().w_px(600.0), |b| {
        b.markdown("doc", SOURCE).directives(&directives, app).add();
    });
}

fn harness() -> Harness<'static, Rig<()>> {
    harness_installing(install)
}

fn harness_installing(install: fn(&egui::Context, &())) -> Harness<'static, Rig<()>> {
    let mut harness = support::harness_prepared(
        vec2(700.0, 600.0),
        TextStyle::new(PAGE_TEXT, Color32::BLACK),
        (),
        view,
        install,
    );
    harness.run_steps(4);
    harness
}

fn block(harness: &Harness<'_, Rig<()>>, path: &str) -> Rect {
    rect_of(harness, &format!("/\"doc\"/{path}"))
}

/// The space between the bottom of `above` and the top of `below`.
fn gap(above: Rect, below: Rect) -> f32 {
    below.top() - above.bottom()
}

/// Within half a point: egui sets a line of text on whole points, so a
/// 26.25 pt line is 26, and an edge lands on the nearest tenth.
#[track_caller]
fn assert_near(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 0.5,
        "{what}: {actual} is not {expected}"
    );
}

#[test]
fn a_paragraph_is_set_at_the_base_size_and_leading() {
    let harness = harness();
    assert_near(
        block(&harness, "Block(0)").height(),
        BASE * 1.75,
        "one line of body text",
    );
}

#[test]
fn paragraphs_are_a_flow_apart() {
    let harness = harness();
    let (first, second) = (block(&harness, "Block(0)"), block(&harness, "Block(1)"));
    assert_near(gap(first, second), FLOW, "between paragraphs");
}

#[test]
fn an_h2_takes_its_ratios_and_owns_the_space_below_it() {
    let harness = harness();
    let above = block(&harness, "Block(1)");
    let heading = block(&harness, "Heading(\"section\")");
    let below = block(&harness, "Block(3)");
    assert_near(heading.height(), H2 * 1.4, "an h2's line");
    assert_near(gap(above, heading), FLOW * 1.4 * 1.25, "above an h2");
    assert_near(gap(heading, below), BASE, "a paragraph after a heading");
}

#[test]
fn a_rule_has_2_4_flows_above_and_an_h2_after_it_one_flow_of_its_own() {
    let harness = harness();
    let above = block(&harness, "Block(3)");
    let rule = block(&harness, "<div>");
    let heading = block(&harness, "Heading(\"after-rule\")");
    assert_near(gap(above, rule), FLOW * 2.4, "above a rule");
    assert_near(gap(rule, heading), 1.25 * H2, "an h2 after a rule");
}

#[test]
fn a_directive_keeps_the_pages_text_and_sits_a_flow_below_a_heading() {
    let harness = harness();
    let heading = block(&harness, "Heading(\"after-rule\")");
    let directive = block(&harness, "Block(6)");
    assert_near(gap(heading, directive), FLOW, "a component in the flow");
    assert_near(
        block(&harness, "Block(6)/\"probe\"").height(),
        PAGE_TEXT.line_height,
        "the directive's text is the page's, not the typeset's",
    );
}

#[test]
fn the_measure_is_its_width_in_ems_of_the_base() {
    assert_near(
        Typeset::default().measure_width(),
        37.0 * BASE,
        "80 is 37em",
    );
}

/// The style without a typeset, which rules an h2 off below.
fn install_unset(ctx: &egui::Context, _: &()) {
    for theme in [Theme::Light, Theme::Dark] {
        MarkdownStyle::from_style(&Style::of(ctx)).install_for(ctx, theme);
    }
}

#[test]
fn without_a_typeset_an_h2_is_ruled_off_below() {
    let harness = harness_installing(install_unset);
    let heading = block(&harness, "Heading(\"section\")");
    let rule = MarkdownStyle::of(&harness.ctx).rule;
    let ruled = support::shapes(&harness)
        .into_iter()
        .any(|shape| match shape {
            egui::epaint::Shape::Rect(rect) => {
                rect.fill == rule
                    && (rect.rect.max.y - heading.max.y).abs() < 0.5
                    && rect.rect.height() <= 1.5
            }
            _ => false,
        });
    assert!(ruled, "a rule along the heading's bottom edge");
}

/// A typeset table, inline code and a link, and a table outside the
/// document, which keeps the page's style.
const RICH: &str = "Call `run()` or read [the guide](guide).

| Name | Kind |
|---|---|
| alpha | beta |
";

fn rich_view<'f>(b: &mut Builder<'_, 'f, ()>, _: &'f ()) {
    b.div(tw().flex().flex_col().w_px(600.0), |b| {
        b.markdown("doc", RICH).add();
        b.table("outside")
            .columns([eggshell_ui::owned::Column::new(eggshell_ui::Track::auto())])
            .header(|h| h.cell_text("Head"))
            .rows(&[1], |row| *row, |cells, _| cells.cell_text("Body"))
            .add();
    });
}

fn rich_harness() -> Harness<'static, Rig<()>> {
    let mut harness = support::harness_prepared(
        vec2(700.0, 600.0),
        TextStyle::new(PAGE_TEXT, Color32::BLACK),
        (),
        rich_view,
        install,
    );
    harness.run_steps(4);
    harness
}

fn cell(harness: &Harness<'_, Rig<()>>, band: &str, column: usize) -> Rect {
    block(harness, &format!("Block(1)/Table/({band}, {column})"))
}

#[test]
fn a_tables_cells_are_padded_in_ems_and_its_first_column_is_flush() {
    let harness = rich_harness();
    let (first, second) = (cell(&harness, "Row(0)", 0), cell(&harness, "Row(0)", 1));
    assert_near(
        first.height(),
        1.5 * BASE + 2.0 * 0.75 * BASE,
        "a body cell",
    );
    assert_near(
        cell(&harness, "Header", 0).height(),
        1.5 * BASE + 2.0 * 0.65 * BASE,
        "a header cell",
    );
    let (_, alpha) = support::galley_of(&harness, "alpha");
    let (_, beta) = support::galley_of(&harness, "beta");
    assert_near(alpha.x - first.left(), 0.0, "the first column is flush");
    assert_near(beta.x - second.left(), BASE, "the others are padded an em");
}

#[test]
fn a_table_outside_the_document_keeps_the_pages_style() {
    let harness = rich_harness();
    let page = Style::of(&harness.ctx).table.clone();
    let body = rect_of(&harness, "/\"outside\"/(Row(1), 0)");
    let (_, text) = support::galley_of(&harness, "Body");
    assert_near(
        text.x - body.left(),
        page.cell_padding,
        "the page's padding",
    );
}

#[test]
fn inline_code_is_0_85_em_and_a_links_underline_is_a_faint_foreground() {
    let harness = rich_harness();
    let source = "Call run() or read the guide.";
    let (galley, _) = support::galley_of(&harness, source);
    let code = support::section_of(&galley, "run()");
    assert_near(code.font_id.size, 0.85 * BASE, "inline code");
    let link = support::section_of(&galley, "the guide");
    assert_eq!(link.color, ink().foreground, "a link is the text's colour");
    let faint = ink().foreground.gamma_multiply(0.3);
    let underline = support::shapes(&harness).into_iter().any(|shape| {
        matches!(shape, egui::epaint::Shape::LineSegment { stroke, .. } if stroke.color == faint)
    });
    assert!(underline, "the link's underline is the foreground at 30%");
}
