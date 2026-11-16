//! The Markdown widget (#254) through the real path: its test page, built,
//! laid out and drawn by the rig, clicked as a user would.

use eggshell_ui::Style;
use eggshell_ui_extras::markdown::{Level, MarkdownStyle, headings};
use egui::epaint::Shape;
use egui::{Rect, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, press_and_release, rect_of};

#[path = "../../examples/common/mod.rs"]
mod common;

#[path = "../../examples/markdown.rs"]
mod page;

const PARAGRAPH: &str = "A paragraph with strong, emphasis, both, inline code and links: internal, relative and external. A soft break joins lines; a hard break\nstarts a new one.";
const CODE: &str = "fn main() {\n    println!(\"plain for now\");\n}";

fn harness() -> Harness<'static, Rig<page::State>> {
    harness_sized(vec2(720.0, 1600.0))
}

fn harness_sized(size: egui::Vec2) -> Harness<'static, Rig<page::State>> {
    let mut harness = support::harness(
        size,
        common::base_text(Theme::Light),
        page::State::default(),
        page::layout,
    );
    harness.run_steps(4);
    harness
}

fn style(harness: &Harness<'_, Rig<page::State>>) -> std::sync::Arc<MarkdownStyle> {
    MarkdownStyle::of(&harness.ctx)
}

fn doc(path: &str) -> String {
    format!("/Page/Doc{path}")
}

/// Every rect painted in `color`.
fn filled(harness: &Harness<'_, Rig<page::State>>, color: egui::Color32) -> Vec<Rect> {
    support::shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == color => Some(rect.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn the_page_shows_every_text_block() {
    let harness = harness();
    assert!(
        rect_of(&harness, "/Page/Followed").max.y < 1600.0,
        "precondition: the whole page is in view"
    );
    for text in [
        "Markdown in eggshell",
        PARAGRAPH,
        "nested twice",
        "A block quote holds blocks.",
        CODE,
        "Not shown: table",
        "Not shown: HTML",
        "Image not shown: A missing swatch",
        "No example named \"nonexistent\"",
    ] {
        support::galley_of(&harness, text);
    }
}

#[test]
fn headings_take_their_level_style_and_anchor_keys() {
    let harness = harness();
    let style = style(&harness);
    assert_ne!(
        style.heading(Level::H1).text.size,
        common::base_text(Theme::Light).spec.size,
        "precondition: headings are not body text"
    );
    for (level, anchor, text) in [
        (Level::H1, "markdown-in-eggshell", "Markdown in eggshell"),
        (Level::H2, "lists", "Lists"),
        (Level::H6, "what-degrades", "What degrades"),
    ] {
        rect_of(&harness, &doc(&format!("/Heading({anchor:?})")));
        let (galley, _) = support::galley_of(&harness, text);
        let size = galley.job.sections[0].format.font_id.size;
        assert_eq!(size, style.heading(level).text.size, "{text}");
    }
}

#[test]
fn headings_list_the_page_outline() {
    let outline: Vec<String> = headings(page::SAMPLE)
        .into_iter()
        .map(|heading| heading.anchor)
        .collect();
    assert_eq!(
        outline,
        [
            "markdown-in-eggshell",
            "lists",
            "quotes-and-code",
            "images",
            "directives",
            "what-degrades"
        ]
    );
}

#[test]
fn an_h2_has_a_rule_below() {
    let harness = harness();
    let heading = rect_of(&harness, &doc("/Heading(\"lists\")"));
    let rule = filled(&harness, style(&harness).rule)
        .into_iter()
        .find(|rect| (rect.max.y - heading.max.y).abs() < 0.5 && rect.height() <= 1.5)
        .expect("a rule along the heading's bottom edge");
    assert!(rule.width() >= heading.width() - 0.5);
}

#[test]
fn list_markers_sit_on_their_first_line() {
    let harness = harness();
    let item = doc("/Block(3)/Item(1)");
    let (marker, content) = (
        rect_of(&harness, &format!("{item}/Marker")),
        rect_of(&harness, &format!("{item}/Content/Block(0)")),
    );
    assert!(
        rect_of(&harness, &item).height() > 2.0 * content.height(),
        "precondition: the item has a nested list below its first line"
    );
    assert_eq!(
        marker.height(),
        content.height(),
        "a marker is one line tall"
    );
    assert_eq!(marker.min.y, content.min.y);
    let painted = support::shapes(&harness)
        .into_iter()
        .any(|shape| matches!(shape, Shape::Circle(circle) if marker.contains(circle.center)));
    assert!(painted, "a disc is painted in the marker's line");
}

#[test]
fn an_ordered_list_counts_from_its_start() {
    let harness = harness();
    support::galley_of(&harness, "3.");
    support::galley_of(&harness, "4.");
}

#[test]
fn a_quote_has_a_border_on_its_left() {
    let harness = harness();
    let quote = rect_of(&harness, &doc("/Block(6)"));
    let look = &style(&harness).quote;
    let border = filled(&harness, look.border)
        .into_iter()
        .find(|rect| {
            (rect.min.x - quote.min.x).abs() < 0.5 && rect.height() >= quote.height() - 0.5
        })
        .expect("a border along the quote's left edge");
    assert!((border.width() - look.border_width).abs() < 0.5);
}

#[test]
fn a_code_block_is_monospace_on_its_fill() {
    let harness = harness();
    let block = rect_of(&harness, &doc("/Block(7)"));
    let look = &style(&harness).code;
    assert!(
        filled(&harness, look.fill)
            .iter()
            .any(|rect| support::same(*rect, block)),
        "the block is filled"
    );
    let (galley, _) = support::galley_of(&harness, CODE);
    assert_eq!(
        galley.job.sections[0].format.font_id.family,
        egui::FontFamily::Monospace
    );
}

#[test]
fn an_image_takes_its_texture_size() {
    let harness = harness();
    let image = rect_of(&harness, &doc("/Block(9)/<image>"));
    assert_eq!(image.size(), vec2(240.0, 64.0));
}

#[test]
fn clicking_an_internal_link_scrolls_to_its_heading() {
    let mut harness = harness_sized(vec2(720.0, 360.0));
    let heading = || doc("/Heading(\"directives\")");
    let page = rect_of(&harness, "/Page");
    assert!(
        rect_of(&harness, &heading()).min.y > page.max.y,
        "precondition: the heading starts out of view"
    );
    let link = support::part_rect(&harness, PARAGRAPH, "internal");
    press_and_release(&mut harness, link.center());
    harness.run_steps(4);
    let top = rect_of(&harness, &heading()).min.y;
    assert!(
        (page.min.y..page.min.y + 1.0).contains(&top),
        "the heading is scrolled to the page's top, at {top}"
    );
    assert!(
        harness.state().app.followed.is_empty(),
        "an anchor the document holds is not passed on"
    );
}

#[test]
fn clicking_a_relative_link_calls_on_link() {
    let mut harness = harness();
    let link = support::part_rect(&harness, PARAGRAPH, "relative");
    press_and_release(&mut harness, link.center());
    harness.run_steps(2);
    assert_eq!(harness.state().app.followed, ["guide.md"]);
    support::galley_of(&harness, "Followed: guide.md; counted 0");
}

#[test]
fn clicking_an_external_link_opens_it() {
    let mut harness = harness();
    let link = support::part_rect(&harness, PARAGRAPH, "external");
    support::hover_over(&mut harness, link.center());
    let mut opened = Vec::new();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: link.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
        opened.extend(support::opened_urls(&harness));
    }
    assert_eq!(opened, ["https://github.com/samitbasu/eggshell"]);
    assert!(harness.state().app.followed.is_empty());
}

#[test]
fn a_registered_directive_mounts_a_live_example() {
    let mut harness = harness();
    support::click_node(&mut harness, &doc("/Block(13)/\"add\""));
    harness.run_steps(2);
    assert_eq!(harness.state().app.count, 1);
}

#[test]
fn the_shadcn_preset_fills_the_markdown_style() {
    let harness = harness();
    let tokens = common::tokens(&harness.ctx);
    let preset = eggshell_ui_shadcn::Shadcn::of(&harness.ctx);
    let installed = style(&harness);
    assert_eq!(*installed, preset.markdown(&Style::of(&harness.ctx)));
    assert_eq!(installed.rule, tokens.border);
    assert_eq!(installed.code.fill, tokens.muted);
}

/// egui's scroll areas drag to scroll only once a touch has been seen.
fn touch_screen(harness: &mut Harness<'_, Rig<page::State>>) {
    harness.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Cancel,
        pos: egui::Pos2::ZERO,
        force: None,
    });
    harness.step();
}

fn page_offset(harness: &Harness<'_, Rig<page::State>>) -> f32 {
    support::shell(harness)
        .viewport(support::id_of(harness, "/Page"))
        .expect("laid out")
        .offset()
        .y
}

#[test]
fn a_touch_drag_that_starts_on_any_block_scrolls_the_page() {
    let mut harness = harness_sized(vec2(720.0, 400.0));
    touch_screen(&mut harness);
    let blocks: Vec<String> = harness
        .state()
        .nodes
        .iter()
        .map(|(path, _)| path.clone())
        .filter(|path| {
            path.strip_prefix("/Page/Doc/")
                .is_some_and(|rest| !rest.contains('/') && !rest.starts_with('<'))
        })
        .collect();
    assert!(
        blocks.len() > 10,
        "precondition: the blocks are drawn: {blocks:?}"
    );
    let mut stuck = Vec::new();
    let mut swept = 0;
    for block in &blocks {
        support::shell(&harness)
            .scroll_into_view(support::id_of(&harness, block), Some(egui::Align::Center));
        harness.run_steps(4);
        let page = rect_of(&harness, "/Page");
        let rect = rect_of(&harness, block).intersect(page.shrink(24.0));
        if !rect.is_positive() {
            continue;
        }
        swept += 1;
        let before = page_offset(&harness);
        let up = if before > 100.0 { 60.0 } else { -60.0 };
        support::drag(&mut harness, rect.center(), rect.center() + vec2(0.0, up));
        let moved = (page_offset(&harness) - before).abs();
        if moved < 20.0 {
            stuck.push(format!("{block}: moved {moved}"));
        }
    }
    assert_eq!(swept, blocks.len(), "precondition: every block was reached");
    assert!(
        stuck.is_empty(),
        "a touch drag did not scroll from {stuck:#?}"
    );
}

#[test]
fn on_a_touch_screen_a_tap_on_a_link_still_follows_it() {
    let mut harness = harness();
    touch_screen(&mut harness);
    let link = support::part_rect(&harness, PARAGRAPH, "relative");
    press_and_release(&mut harness, link.center());
    harness.run_steps(2);
    assert_eq!(harness.state().app.followed, ["guide.md"]);
}
