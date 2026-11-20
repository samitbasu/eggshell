//! A scoped style (#355): `b.styled(edit, body)` gives the nodes `body`
//! builds a style edited from their parent's, read at build, measure and
//! draw, and leaves every node outside it alone.

use eggshell_ui::owned::Column;
use eggshell_ui::{Builder, NodeUi as _, Span, Spans, Style, TextStyle, Track, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Color32, Theme, Vec2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, rect_of};

const WINDOW: Vec2 = vec2(900.0, 600.0);
const PADDING: f32 = 20.0;
const MORE: f32 = 10.0;
const CODE_PADDING: f32 = 12.0;
const LINK: Color32 = Color32::from_rgb(200, 30, 40);

fn table(b: &mut Builder<'_, '_, ()>, key: &'static str) {
    b.table(key)
        .columns([Column::new(Track::auto())])
        .header(|h| h.cell_text("Head"))
        .rows(&[1], |row| *row, |cells, _| cells.cell_text("Body"));
}

fn spans() -> Spans {
    [
        Span::new("call "),
        Span::new("run()").code(),
        Span::new(" or "),
        Span::new("read the guide").link("guide"),
    ]
    .into_iter()
    .collect()
}

/// Notes the table padding a draw closure's node is drawn with.
fn seen(ui: &egui::Ui, at: &'static str) {
    let padding = ui.node_style().table.cell_padding;
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::NULL.with(at), padding));
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        table(b, "plain");
        b.styled(
            |style| style.table.cell_padding = PADDING,
            |b| {
                table(b, "styled");
                b.styled(
                    |style| style.table.cell_padding += MORE,
                    |b| table(b, "nested"),
                );
            },
        );
        b.span("probe", tw(), "probe")
            .draw(|_, ui| seen(ui, "outside"));
        b.styled(
            |style| style.table.cell_padding = PADDING,
            |b| {
                b.span("scoped probe", tw(), "probe")
                    .draw(|_, ui| seen(ui, "inside"));
            },
        );
        b.spans("outside", tw(), spans());
        b.styled(
            |style| {
                style.rich.code.padding_x = CODE_PADDING;
                style.rich.link = LINK;
            },
            |b| {
                b.spans("inside", tw(), spans());
            },
        );
    });
}

fn harness() -> Harness<'static, Rig<()>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        view,
    );
    harness.run_steps(3);
    harness
}

/// The height of the body cell of the table `key`.
fn body_cell(harness: &Harness<'_, Rig<()>>, key: &str) -> f32 {
    rect_of(harness, &format!("/\"{key}\"/(Row(1), 0)")).height()
}

#[test]
fn a_table_inside_styled_takes_the_override_and_one_outside_does_not() {
    let harness = harness();
    let global = Style::of(&harness.ctx).table.cell_padding;
    assert_ne!(global, PADDING, "precondition: the override differs");
    let (plain, styled) = (body_cell(&harness, "plain"), body_cell(&harness, "styled"));
    assert!(
        (styled - plain - 2.0 * (PADDING - global)).abs() < 0.6,
        "{styled} is {plain} with {PADDING} for {global} padding"
    );
    assert_eq!(
        Style::of(&harness.ctx).table.cell_padding,
        global,
        "the context's style is untouched"
    );
}

#[test]
fn a_nested_scope_edits_the_style_of_the_one_around_it() {
    let harness = harness();
    let (styled, nested) = (body_cell(&harness, "styled"), body_cell(&harness, "nested"));
    assert!(
        (nested - styled - 2.0 * MORE).abs() < 0.6,
        "{nested} is {styled} with {MORE} more padding on each side"
    );
}

#[test]
fn rich_text_inside_styled_is_measured_and_drawn_with_the_override() {
    let harness = harness();
    let global = Style::of(&harness.ctx).rich.clone();
    assert_ne!(global.code.padding_x, CODE_PADDING, "precondition");
    assert_ne!(global.link, LINK, "precondition");
    let (outside, inside) = (
        rect_of(&harness, "/\"outside\"").width(),
        rect_of(&harness, "/\"inside\"").width(),
    );
    assert!(
        (inside - outside - 2.0 * (CODE_PADDING - global.code.padding_x)).abs() < 0.6,
        "the code's padding is layout space: {inside} against {outside}"
    );
    let colours: Vec<Color32> = support::galleys_of(&harness, &spans().text())
        .iter()
        .map(|galley| support::section_of(galley, "read the guide").color)
        .collect();
    assert_eq!(colours.len(), 2, "precondition: both paragraphs are drawn");
    assert!(colours.contains(&LINK), "the scoped link: {colours:?}");
    assert!(
        colours.contains(&global.link),
        "the global link: {colours:?}"
    );
}

#[test]
fn a_draw_closure_reads_the_style_its_node_was_built_in() {
    let harness = harness();
    let seen = |at: &str| {
        harness
            .ctx
            .data(|data| data.get_temp::<f32>(egui::Id::NULL.with(at)))
    };
    assert_eq!(seen("inside"), Some(PADDING));
    assert_eq!(
        seen("outside"),
        Some(Style::of(&harness.ctx).table.cell_padding)
    );
}

const TIP_PADDING: Vec2 = vec2(40.0, 20.0);

fn tooltips(b: &mut Builder<'_, '_, ()>, _: &()) {
    fn tip(b: &mut Builder<'_, '_, ()>) {
        b.span("tip", tw(), "Tip").draw(|_, ui| {
            let padding = ui.node_style().tooltip.padding;
            ui.ctx()
                .data_mut(|data| data.insert_temp(egui::Id::NULL.with("tip"), padding));
        });
    }
    b.div(tw().flex().gap(16.0).p(16.0).pt(24.0), |b| {
        b.button("outside", "Outside").tooltip_with(tip).add();
        b.styled(
            |style| style.tooltip.padding = TIP_PADDING,
            |b| {
                b.button("inside", "Inside").tooltip_with(tip).add();
            },
        );
    });
}

/// The box of the one tooltip open after resting on `trigger`, the tooltip
/// padding its content was drawn with, and the context's.
fn tooltip_of(trigger: &str) -> (egui::Rect, Option<Vec2>, Vec2) {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        tooltips,
    );
    harness.run_steps(3);
    support::hover_node(&mut harness, &format!("/\"{trigger}\""));
    support::rest_for_tooltip(&mut harness);
    let boxes: Vec<egui::Rect> = harness
        .state()
        .nodes
        .iter()
        .filter(|(path, _)| path.contains("Tooltip(") && path.ends_with("/\"Box\""))
        .map(|(_, rect)| *rect)
        .collect();
    assert_eq!(boxes.len(), 1, "precondition: one tooltip is open");
    let seen = harness
        .ctx
        .data(|data| data.get_temp::<Vec2>(egui::Id::NULL.with("tip")));
    (boxes[0], seen, Style::of(&harness.ctx).tooltip.padding)
}

#[test]
fn a_tooltip_inside_styled_draws_with_the_override_and_one_outside_does_not() {
    let (outside, outside_seen, global) = tooltip_of("outside");
    let (inside, inside_seen, _) = tooltip_of("inside");
    assert_eq!(outside_seen, Some(global), "the content outside");
    assert_ne!(global, TIP_PADDING, "precondition: the override differs");
    assert_eq!(inside_seen, Some(TIP_PADDING), "the content inside");
    let grown = inside.size() - outside.size();
    let expected = 2.0 * (TIP_PADDING - global);
    assert!(
        (grown - expected).length() < 0.6,
        "the box grows by the padding: {grown:?} against {expected:?}"
    );
}
