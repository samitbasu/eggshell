//! A scoped style (#355): `b.styled(edit, body)` gives the nodes `body`
//! builds a style edited from their parent's, read at build, measure and
//! draw, and leaves every node outside it alone.

use eggshell_ui::owned::Column;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, NodeUi as _, Span, Spans, TextStyle, Track, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Color32, Theme, Vec2, vec2};

use crate::support::{self, rest_for_tooltip};

const WINDOW: Vec2 = vec2(900.0, 600.0);
const PADDING: f32 = 20.0;
const MORE: f32 = 10.0;
const CODE_PADDING: f32 = 12.0;
const LINK: Color32 = Color32::from_rgb(200, 30, 40);

fn table(b: &mut Builder<'_, '_, ()>, key: &'static str) {
    b.table(key)
        .columns([Column::new(Track::auto())])
        .header(|h| h.cell_text("Head"))
        .rows(&[1], |row| *row, |cells, _| cells.cell_text("Body"))
        .add();
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
    let padding = ui.node_style().table().cell_padding;
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::NULL.with(at), padding));
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        table(b, "plain");
        b.styled(
            |style| {
                let mut table = style.table().clone();
                table.cell_padding = PADDING;
                style.at_current_layer(|style| style.set_table(table));
            },
            |b| {
                table(b, "styled");
                b.styled(
                    |style| {
                        let mut table = style.table().clone();
                        table.cell_padding += MORE;
                        style.at_current_layer(|style| style.set_table(table));
                    },
                    |b| table(b, "nested"),
                );
            },
        );
        b.span("probe", tw(), "probe")
            .draw(|_, ui| seen(ui, "outside"));
        b.styled(
            |style| {
                let mut table = style.table().clone();
                table.cell_padding = PADDING;
                style.at_current_layer(|style| style.set_table(table));
            },
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

fn harness() -> Driver<()> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        view,
    )
    .build();
    d.frames(3);
    d
}

/// The height of the body cell of the table `key`.
fn body_cell(d: &Driver<()>, key: &str) -> f32 {
    d.rect(Target::suffix(&format!("/\"{key}\"/(Row(1), 0)")))
        .height()
}

#[test]
fn a_table_inside_styled_takes_the_override_and_one_outside_does_not() {
    let d = harness();
    let global = d.shell().style().table().cell_padding;
    assert_ne!(global, PADDING, "precondition: the override differs");
    let (plain, styled) = (body_cell(&d, "plain"), body_cell(&d, "styled"));
    assert!(
        (styled - plain - 2.0 * (PADDING - global)).abs() < 0.6,
        "{styled} is {plain} with {PADDING} for {global} padding"
    );
    assert_eq!(
        d.shell().style().table().cell_padding,
        global,
        "the context's style is untouched"
    );
}

#[test]
fn a_nested_scope_edits_the_style_of_the_one_around_it() {
    let d = harness();
    let (styled, nested) = (body_cell(&d, "styled"), body_cell(&d, "nested"));
    assert!(
        (nested - styled - 2.0 * MORE).abs() < 0.6,
        "{nested} is {styled} with {MORE} more padding on each side"
    );
}

#[test]
fn rich_text_inside_styled_is_measured_and_drawn_with_the_override() {
    let d = harness();
    let global = d.shell().style().rich.clone();
    assert_ne!(global.code.padding_x, CODE_PADDING, "precondition");
    assert_ne!(global.link, LINK, "precondition");
    let (outside, inside) = (
        d.rect(Target::suffix("/\"outside\"")).width(),
        d.rect(Target::suffix("/\"inside\"")).width(),
    );
    assert!(
        (inside - outside - 2.0 * (CODE_PADDING - global.code.padding_x)).abs() < 0.6,
        "the code's padding is layout space: {inside} against {outside}"
    );
    let colours: Vec<Color32> = d
        .painted()
        .all(&spans().text())
        .iter()
        .map(|text| support::section_of(text.galley(), "read the guide").color)
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
    let d = harness();
    let seen = |at: &str| {
        d.ctx()
            .data(|data| data.get_temp::<f32>(egui::Id::NULL.with(at)))
    };
    assert_eq!(seen("inside"), Some(PADDING));
    assert_eq!(
        seen("outside"),
        Some(d.shell().style().table().cell_padding)
    );
}

const TIP_PADDING: Vec2 = vec2(40.0, 20.0);

fn tooltips(b: &mut Builder<'_, '_, ()>, _: &()) {
    fn tip(b: &mut Builder<'_, '_, ()>) {
        b.span("tip", tw(), "Tip").draw(|_, ui| {
            let padding = ui.node_style().tooltip().padding;
            ui.ctx()
                .data_mut(|data| data.insert_temp(egui::Id::NULL.with("tip"), padding));
        });
    }
    b.div(tw().flex().gap(16.0).p(16.0).pt(24.0), |b| {
        b.button("outside", "Outside").tooltip_with(tip).add();
        b.styled(
            |style| {
                let mut tooltip = style.tooltip().clone();
                tooltip.padding = TIP_PADDING;
                style.set_tooltip(tooltip);
                // Under the preset, the section's padding is the widgets'
                // default; the scope's rule restyles the box.
                style
                    .sheet
                    .rule(eggshell_ui::owned::TooltipPart::Content, |t, _| {
                        t.px_px(TIP_PADDING.x).py_px(TIP_PADDING.y)
                    });
            },
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
    let mut d = support::driver(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        tooltips,
    )
    .build();
    d.frames(3);
    d.hover(Target::suffix(&format!("/\"{trigger}\"")));
    rest_for_tooltip(&mut d);
    let boxes: Vec<egui::Rect> = support::nodes(&d)
        .into_iter()
        .filter(|(path, _)| path.contains("Tooltip(") && path.ends_with("/\"Box\""))
        .map(|(_, rect)| rect)
        .collect();
    assert_eq!(boxes.len(), 1, "precondition: one tooltip is open");
    let seen = d
        .ctx()
        .data(|data| data.get_temp::<Vec2>(egui::Id::NULL.with("tip")));
    (boxes[0], seen, d.shell().style().tooltip().padding)
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
