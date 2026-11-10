//! Selectable text (issue #39) through the real path: pointer drags and
//! `Event::Copy` dispatched by kittest into egui's label selection.

use bullseye::{Builder, TextStyle, tw};
use egui::{Event, OutputCommand, Pos2, Rect, pos2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, drag, press_and_release, rect_of};

const FIRST: &str = "Alpha bravo charlie";
const SECOND: &str = "Delta echo foxtrot";

#[derive(Default)]
struct App {
    row_clicks: u32,
    select: Select,
}

#[derive(Clone, Copy, Default)]
enum Select {
    #[default]
    Auto,
    Text,
    None,
}

/// ```html
/// <div class="flex flex-col gap-4 p-4">
///   <p>Alpha bravo charlie</p>
///   <p>Delta echo foxtrot</p>
///   <button>Clickable row</button>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let mode = |classes: bullseye::Tw| match app.select {
        Select::Auto => classes,
        Select::Text => classes.select_text(),
        Select::None => classes.select_none(),
    };
    b.div(tw().flex().flex_col().gap(4.0).p(4.0), |b| {
        b.div_id("first", mode(tw()), |b| b.text(tw(), FIRST))
            .draw(|_, _| {});
        b.div_id("second", mode(tw()), |b| b.text(tw(), SECOND))
            .draw(|_, _| {});
        b.span("row", mode(tw().sense_click()), "Clickable row")
            .draw(|app, ui| {
                if bullseye::NodeUi::node_response(ui).clicked() {
                    app.row_clicks += 1;
                }
            });
    });
}

fn base() -> TextStyle {
    let tokens = bullseye_shadcn::Tokens::light();
    TextStyle::new(tokens.text_role(), tokens.foreground)
}

fn harness(select: Select) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(
        vec2(300.0, 200.0),
        base(),
        App {
            select,
            ..App::default()
        },
        view,
    );
    harness.run_steps(6);
    harness
}

fn text_of(harness: &Harness<'_, Rig<App>>, node: &str) -> Rect {
    rect_of(harness, &format!("/\"{node}\"/<text>"))
}

fn copied(harness: &mut Harness<'_, Rig<App>>) -> Option<String> {
    harness.event(Event::Copy);
    harness.step();
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

fn inside(rect: Rect, x: f32) -> Pos2 {
    pos2(rect.min.x + x, rect.center().y)
}

#[test]
fn dragging_across_a_paragraph_selects_and_copies_it() {
    let mut harness = harness(Select::Auto);
    let rect = text_of(&harness, "first");
    assert!(
        rect.width() > 60.0,
        "the paragraph is wide enough to drag in"
    );
    drag(
        &mut harness,
        inside(rect, 1.0),
        inside(rect, rect.width() - 1.0),
    );
    assert_eq!(copied(&mut harness).as_deref(), Some(FIRST));
}

#[test]
fn dragging_from_one_paragraph_to_the_next_copies_both() {
    let mut harness = harness(Select::Auto);
    let (first, second) = (text_of(&harness, "first"), text_of(&harness, "second"));
    assert!(first.max.y < second.min.y, "the paragraphs are stacked");
    drag(
        &mut harness,
        inside(first, 1.0),
        inside(second, second.width() - 1.0),
    );
    let copied = copied(&mut harness).expect("something was copied");
    assert!(
        copied.contains(FIRST) && copied.contains(SECOND),
        "{copied:?}"
    );
}

#[test]
fn a_click_on_text_inside_a_clickable_row_activates_it_and_selects_nothing() {
    let mut harness = harness(Select::Auto);
    let row = rect_of(&harness, "/\"row\"");
    assert!(row.width() > 20.0);
    drag(
        &mut harness,
        inside(row, 1.0),
        inside(row, row.width() - 1.0),
    );
    press_and_release(&mut harness, row.center());
    assert_eq!(harness.state().app.row_clicks, 1);
    assert_eq!(copied(&mut harness), None);
}

#[test]
fn select_none_text_cannot_be_selected() {
    let mut harness = harness(Select::None);
    let rect = text_of(&harness, "first");
    drag(
        &mut harness,
        inside(rect, 1.0),
        inside(rect, rect.width() - 1.0),
    );
    assert_eq!(copied(&mut harness), None);
}

#[test]
fn select_text_makes_a_clickable_row_selectable() {
    let mut harness = harness(Select::Text);
    let row = rect_of(&harness, "/\"row\"");
    drag(
        &mut harness,
        inside(row, 1.0),
        inside(row, row.width() - 1.0),
    );
    assert_eq!(copied(&mut harness).as_deref(), Some("Clickable row"));
}

#[test]
fn selecting_does_not_change_layout() {
    let rects = |select: Select| {
        let harness = harness(select);
        harness.state().nodes.clone()
    };
    let selectable = rects(Select::Text);
    assert_eq!(selectable, rects(Select::None));
    assert!(!selectable.is_empty());
}

#[test]
fn a_live_selection_does_not_move_anything() {
    let mut harness = harness(Select::Auto);
    let before = harness.state().nodes.clone();
    let rect = text_of(&harness, "first");
    drag(
        &mut harness,
        inside(rect, 1.0),
        inside(rect, rect.width() - 1.0),
    );
    assert!(copied(&mut harness).is_some(), "there is a selection");
    assert_eq!(harness.state().nodes, before);
}
