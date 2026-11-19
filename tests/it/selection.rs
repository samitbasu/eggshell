//! Selectable text (issue #39) through the real path: pointer drags and
//! `Event::Copy` dispatched by kittest into egui's label selection.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Event, OutputCommand, Pos2, Rect, pos2, vec2};

use crate::support::{self, press_and_release};

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
    let mode = |classes: eggshell_ui::Tw| match app.select {
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
            .on_click(|app| {
                app.row_clicks += 1;
            });
    });
}

fn base() -> TextStyle {
    let tokens = eggshell_ui_shadcn::Tokens::light();
    TextStyle::new(tokens.text_role(), tokens.foreground)
}

fn harness(select: Select) -> Driver<App> {
    let mut d = support::driver(
        vec2(300.0, 200.0),
        base(),
        App {
            select,
            ..App::default()
        },
        view,
    )
    .build();
    d.frames(6);
    d
}

fn text_of(d: &Driver<App>, node: &str) -> Rect {
    d.rect(Target::suffix(&format!("/\"{node}\"/<text>")))
}

fn copied(d: &mut Driver<App>) -> Option<String> {
    d.event(Event::Copy);
    d.harness()
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
    let mut d = harness(Select::Auto);
    let rect = text_of(&d, "first");
    assert!(
        rect.width() > 60.0,
        "the paragraph is wide enough to drag in"
    );
    d.drag(inside(rect, 1.0), inside(rect, rect.width() - 1.0));
    assert_eq!(copied(&mut d).as_deref(), Some(FIRST));
}

#[test]
fn dragging_from_one_paragraph_to_the_next_copies_both() {
    let mut d = harness(Select::Auto);
    let (first, second) = (text_of(&d, "first"), text_of(&d, "second"));
    assert!(first.max.y < second.min.y, "the paragraphs are stacked");
    d.drag(inside(first, 1.0), inside(second, second.width() - 1.0));
    let copied = copied(&mut d).expect("something was copied");
    assert!(
        copied.contains(FIRST) && copied.contains(SECOND),
        "{copied:?}"
    );
}

#[test]
fn a_click_on_text_inside_a_clickable_row_activates_it_and_selects_nothing() {
    let mut d = harness(Select::Auto);
    let row = d.rect(Target::suffix("/\"row\""));
    assert!(row.width() > 20.0);
    d.drag(inside(row, 1.0), inside(row, row.width() - 1.0));
    press_and_release(&mut d, row.center());
    assert_eq!(d.app().row_clicks, 1);
    assert_eq!(copied(&mut d), None);
}

#[test]
fn select_none_text_cannot_be_selected() {
    let mut d = harness(Select::None);
    let rect = text_of(&d, "first");
    d.drag(inside(rect, 1.0), inside(rect, rect.width() - 1.0));
    assert_eq!(copied(&mut d), None);
}

#[test]
fn select_text_makes_a_clickable_row_selectable() {
    let mut d = harness(Select::Text);
    let row = d.rect(Target::suffix("/\"row\""));
    d.drag(inside(row, 1.0), inside(row, row.width() - 1.0));
    assert_eq!(copied(&mut d).as_deref(), Some("Clickable row"));
}

#[test]
fn selecting_does_not_change_layout() {
    let rects = |select: Select| {
        let d = harness(select);
        support::nodes(&d)
    };
    let selectable = rects(Select::Text);
    assert_eq!(selectable, rects(Select::None));
    assert!(!selectable.is_empty());
}

#[test]
fn a_live_selection_does_not_move_anything() {
    let mut d = harness(Select::Auto);
    let before = support::nodes(&d);
    let rect = text_of(&d, "first");
    d.drag(inside(rect, 1.0), inside(rect, rect.width() - 1.0));
    assert!(copied(&mut d).is_some(), "there is a selection");
    assert_eq!(support::nodes(&d), before);
}
