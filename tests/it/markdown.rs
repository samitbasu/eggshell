//! The Markdown widget (#254) through the real path: its test page, built,
//! laid out and drawn by the rig, clicked as a user would.

use eggshell_ui::owned::markdown::{Ext as _, Level, MarkdownStyle, heading_id, headings};
use eggshell_ui::testing::Headless;
use eggshell_ui::testing::{Driver, Target};
use egui::epaint::Shape;
use egui::{Rect, Theme, vec2};

use crate::support::{self, press_and_release};

#[path = "../../examples/common/mod.rs"]
mod common;

#[path = "../../examples/markdown.rs"]
mod page;

const PARAGRAPH: &str = "A paragraph with strong, emphasis, both, inline code and links: internal, relative and external. A soft break joins lines; a hard break\nstarts a new one.";
const CODE: &str = "fn main() {\n    println!(\"plain for now\");\n}";

fn driver() -> Driver<page::State> {
    driver_sized(vec2(720.0, 1700.0))
}

fn driver_sized(size: egui::Vec2) -> Driver<page::State> {
    let mut d = support::driver(
        size,
        common::base_text(Theme::Light),
        page::State::default(),
        page::layout,
    )
    .build();
    d.frames(4);
    d
}

fn style(d: &Driver<page::State>) -> MarkdownStyle {
    d.shell().style().markdown.clone()
}

fn doc(path: &str) -> String {
    format!("/Page/Doc{path}")
}

/// Every rect painted in `color`.
fn filled(d: &Driver<page::State>, color: egui::Color32) -> Vec<Rect> {
    support::shapes(d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == color => Some(rect.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn the_page_shows_every_text_block() {
    let d = driver();
    assert!(
        d.rect(Target::suffix("/Page/Followed")).max.y < 1700.0,
        "precondition: the whole page is in view"
    );
    for text in [
        "Markdown in eggshell",
        PARAGRAPH,
        "nested twice",
        "A block quote holds blocks.",
        CODE,
        "Not shown: HTML",
        "Image not shown: A missing swatch",
        "No example named \"nonexistent\"",
    ] {
        d.painted().expect(text);
    }
}

#[test]
fn headings_take_their_level_style_and_anchor_keys() {
    let d = driver();
    let style = style(&d);
    assert_ne!(
        style.heading(Level::H1).text.size,
        common::base_text(Theme::Light).spec.size,
        "precondition: headings are not body text"
    );
    for (level, anchor, text) in [
        (Level::H1, "markdown-in-eggshell", "Markdown in eggshell"),
        (Level::H2, "lists", "Lists"),
        (Level::H6, "what-degrades", "WHAT DEGRADES"),
    ] {
        d.rect(Target::suffix(&doc(&format!("/Heading({anchor:?})"))));
        let galley = d.painted().expect(text).galley().clone();
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
            "tables",
            "what-degrades"
        ]
    );
}

#[test]
fn a_typeset_h2_has_no_rule_below() {
    let d = driver();
    let heading = d.rect(Target::suffix(&doc("/Heading(\"lists\")")));
    let rule = filled(&d, style(&d).rule)
        .into_iter()
        .find(|rect| (rect.max.y - heading.max.y).abs() < 0.5 && rect.height() <= 1.5);
    assert_eq!(rule, None);
}

#[test]
fn list_markers_sit_on_their_first_line() {
    let d = driver();
    let item = doc("/Block(3)/Item(1)");
    let (marker, content) = (
        d.rect(Target::suffix(&format!("{item}/Marker"))),
        d.rect(Target::suffix(&format!("{item}/Content/Block(0)"))),
    );
    assert!(
        d.rect(Target::suffix(&item)).height() > 2.0 * content.height(),
        "precondition: the item has a nested list below its first line"
    );
    assert_eq!(
        marker.height(),
        content.height(),
        "a marker is one line tall"
    );
    assert_eq!(marker.min.y, content.min.y);
    let painted = support::shapes(&d)
        .into_iter()
        .any(|shape| matches!(shape, Shape::Circle(circle) if marker.contains(circle.center)));
    assert!(painted, "a disc is painted in the marker's line");
}

#[test]
fn an_ordered_list_counts_from_its_start() {
    let d = driver();
    d.painted().expect("3.");
    d.painted().expect("4.");
}

#[test]
fn a_quote_has_a_border_on_its_left() {
    let d = driver();
    let quote = d.rect(Target::suffix(&doc("/Block(6)")));
    let look = &style(&d).quote;
    let border = filled(&d, look.border)
        .into_iter()
        .find(|rect| {
            (rect.min.x - quote.min.x).abs() < 0.5 && rect.height() >= quote.height() - 0.5
        })
        .expect("a border along the quote's left edge");
    assert!((border.width() - look.border_width).abs() < 0.5);
}

#[test]
fn a_code_block_is_monospace_on_its_fill() {
    let d = driver();
    let block = d.rect(Target::suffix(&doc("/Block(7)")));
    let look = &style(&d).code;
    assert!(
        filled(&d, look.fill)
            .iter()
            .any(|rect| support::same(*rect, block)),
        "the block is filled"
    );
    let galley = d.painted().expect(CODE).galley().clone();
    assert_eq!(
        galley.job.sections[0].format.font_id.family,
        egui::FontFamily::Monospace
    );
}

#[test]
fn an_image_takes_its_texture_size() {
    let d = driver();
    let image = d.rect(Target::suffix(&doc("/Block(9)/<image>")));
    assert_eq!(image.size(), vec2(240.0, 64.0));
}

#[test]
fn clicking_an_internal_link_scrolls_to_its_heading() {
    let mut d = driver_sized(vec2(720.0, 360.0));
    let heading = || doc("/Heading(\"directives\")");
    let page = d.rect(Target::suffix("/Page"));
    assert!(
        d.rect(Target::suffix(&heading())).min.y > page.max.y,
        "precondition: the heading starts out of view"
    );
    let link = d
        .painted()
        .expect(PARAGRAPH)
        .part_rect("internal")
        .expect("the part is on one row");
    press_and_release(&mut d, link.center());
    d.frames(4);
    let top = d.rect(Target::suffix(&heading())).min.y;
    assert!(
        (page.min.y..page.min.y + 1.0).contains(&top),
        "the heading is scrolled to the page's top, at {top}"
    );
    assert!(
        d.app().followed.is_empty(),
        "an anchor the document holds is not passed on"
    );
}

#[test]
fn clicking_a_relative_link_calls_on_link() {
    let mut d = driver();
    let link = d
        .painted()
        .expect(PARAGRAPH)
        .part_rect("relative")
        .expect("the part is on one row");
    press_and_release(&mut d, link.center());
    d.frames(2);
    assert_eq!(d.app().followed, ["guide.md"]);
    d.painted().expect("Followed: guide.md; counted 0");
}

#[test]
fn clicking_an_external_link_opens_it() {
    let mut d = driver();
    let link = d
        .painted()
        .expect(PARAGRAPH)
        .part_rect("external")
        .expect("the part is on one row");
    d.hover(link.center());
    let mut opened = Vec::new();
    for pressed in [true, false] {
        d.event(egui::Event::PointerButton {
            pos: link.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        opened.extend(support::opened_urls(&d));
    }
    assert_eq!(opened, ["https://github.com/samitbasu/eggshell"]);
    assert!(d.app().followed.is_empty());
}

#[test]
fn a_registered_directive_mounts_a_live_example() {
    let mut d = driver();
    d.click(Target::suffix(&doc("/Block(13)/\"add\"")));
    d.frames(2);
    assert_eq!(d.app().count, 1);
}

#[test]
fn the_shadcn_preset_fills_the_markdown_style() {
    let d = driver();
    let tokens = common::tokens(d.ctx());
    let preset = eggshell_ui::theme::Shadcn::of(d.ctx());
    let installed = style(&d);
    assert_eq!(installed, preset.style().markdown);
    assert_eq!(installed.rule, tokens.border);
    assert_eq!(installed.code.fill, tokens.muted);
}

/// egui's scroll areas drag to scroll only once a touch has been seen.
fn touch_screen(d: &mut Driver<page::State>) {
    d.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Cancel,
        pos: egui::Pos2::ZERO,
        force: None,
    });
}

fn page_offset(d: &Driver<page::State>) -> f32 {
    d.shell()
        .viewport(support::id_of(d, "/Page"))
        .expect("laid out")
        .offset()
        .y
}

#[test]
fn a_touch_drag_that_starts_on_any_block_scrolls_the_page() {
    let mut d = driver_sized(vec2(720.0, 400.0));
    touch_screen(&mut d);
    let blocks: Vec<String> = support::nodes(&d)
        .into_iter()
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
        d.shell()
            .scroll_into_view(support::id_of(&d, block), Some(egui::Align::Center));
        d.frames(4);
        let page = d.rect(Target::suffix("/Page"));
        let rect = d.rect(Target::suffix(block)).intersect(page.shrink(24.0));
        if !rect.is_positive() {
            continue;
        }
        swept += 1;
        let before = page_offset(&d);
        let up = if before > 100.0 { 60.0 } else { -60.0 };
        d.drag(rect.center(), rect.center() + vec2(0.0, up));
        let moved = (page_offset(&d) - before).abs();
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
    let mut d = driver();
    touch_screen(&mut d);
    let link = d
        .painted()
        .expect(PARAGRAPH)
        .part_rect("relative")
        .expect("the part is on one row");
    press_and_release(&mut d, link.center());
    d.frames(2);
    assert_eq!(d.app().followed, ["guide.md"]);
}

fn copied(d: &mut Driver<page::State>) -> Option<String> {
    d.event(egui::Event::Copy);
    d.harness()
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

/// The same drag on the first paragraph, upward from its last row, with a
/// mouse or on a touch screen: what it copied and how far it scrolled.
fn drag_on_the_paragraph(touch: bool) -> (Option<String>, f32) {
    let mut d = driver_sized(vec2(720.0, 400.0));
    if touch {
        touch_screen(&mut d);
    }
    let paragraph = d.painted().expect(PARAGRAPH).rect();
    let from = egui::pos2(paragraph.max.x - 4.0, paragraph.max.y - 4.0);
    d.drag(from, from - vec2(0.0, 60.0));
    let scrolled = page_offset(&d);
    (copied(&mut d), scrolled)
}

#[test]
fn with_a_mouse_a_drag_on_a_paragraph_selects_its_text_and_scrolls_nothing() {
    let (copied, scrolled) = drag_on_the_paragraph(false);
    assert!(
        copied
            .as_deref()
            .is_some_and(|text| text.len() > 10 && PARAGRAPH.contains(text.trim())),
        "copied {copied:?}"
    );
    assert_eq!(scrolled, 0.0);
}

#[test]
fn on_a_touch_screen_the_same_drag_scrolls_and_selects_nothing() {
    let (copied, scrolled) = drag_on_the_paragraph(true);
    assert!(scrolled > 20.0, "scrolled {scrolled}");
    assert_eq!(copied, None);
}

const TABLE: &str = "/Page/Doc/Block(16)/Table";

fn cell(row: &str, column: usize) -> String {
    format!("{TABLE}/({row}, {column})")
}

#[test]
fn a_gfm_table_is_a_table_not_a_degrade_box() {
    let d = driver();
    d.rect(Target::suffix(TABLE));
    for heading in ["Left", "Centred", "Right"] {
        d.painted().expect(heading);
    }
    let degraded = support::shapes(&d).into_iter().any(
        |shape| matches!(shape, Shape::Text(text) if text.galley.text().starts_with("Not shown: table")),
    );
    assert!(!degraded);
}

#[test]
fn a_cell_keeps_its_inline_styles() {
    let d = driver();
    let galley = d.painted().expect("emphasis and code").galley().clone();
    let families: Vec<_> = galley
        .job
        .sections
        .iter()
        .map(|section| section.format.font_id.family.clone())
        .collect();
    assert!(
        families.contains(&egui::FontFamily::Monospace),
        "{families:?}"
    );
    assert!(
        galley
            .job
            .sections
            .iter()
            .any(|section| section.format.italics)
    );
}

#[test]
fn a_link_in_a_cell_calls_on_link() {
    let mut d = driver();
    let link = d
        .painted()
        .expect("a relative link")
        .part_rect("relative")
        .expect("the part is on one row");
    press_and_release(&mut d, link.center());
    d.frames(2);
    assert_eq!(d.app().followed, ["table.md"]);
}

#[test]
fn every_row_lines_up_with_the_header_and_its_alignment() {
    let d = driver();
    for column in 0..3 {
        let header = d.rect(Target::suffix(&cell("Header", column)));
        for row in ["Row(0)", "Row(1)", "Row(2)"] {
            let at = d.rect(Target::suffix(&cell(row, column)));
            assert_eq!(at.x_range(), header.x_range(), "{row} column {column}");
        }
    }
    let content =
        |row: &str, column| d.rect(Target::suffix(&format!("{}/Cell", cell(row, column))));
    let (left, centred, right) = (
        d.rect(Target::suffix(&cell("Row(0)", 0))),
        d.rect(Target::suffix(&cell("Row(0)", 1))),
        d.rect(Target::suffix(&cell("Row(0)", 2))),
    );
    let inset = right.right() - content("Row(0)", 2).right();
    assert!(inset > 0.0, "precondition: cells are padded");
    assert!((content("Row(1)", 1).center().x - centred.center().x).abs() < 0.6);
    assert!(
        (content("Row(0)", 0).left() - left.left()).abs() < 0.6,
        "typeset's first column is flush"
    );
}

#[test]
fn a_wide_cell_wraps_and_the_table_fits_the_page() {
    let d = driver();
    let table = d.rect(Target::suffix(TABLE));
    let article = d.rect(Target::suffix("/Page/Doc"));
    assert!(
        table.width() <= article.width() + 0.5,
        "{table:?} in {article:?}"
    );
    let one_line = d
        .rect(Target::suffix(&format!("{}/Cell", cell("Row(0)", 0))))
        .height();
    let wide = d
        .rect(Target::suffix(&format!("{}/Cell", cell("Row(1)", 0))))
        .height();
    assert!(
        wide >= 2.0 * one_line,
        "the long cell wraps: {wide} vs {one_line}"
    );
}

#[test]
fn headless_places_the_table_where_the_frame_draws_it() {
    let d = driver();
    let placed = Headless::new(vec2(720.0, 1700.0) - vec2(16.0, 16.0))
        .base_text(common::base_text(Theme::Light))
        .prepare(common::use_preset)
        .style(crate::support::shadcn())
        .layout(&page::State::default(), page::layout);
    let drawn: Vec<_> = support::nodes(&d)
        .into_iter()
        .filter(|(path, _)| path.starts_with(TABLE))
        .collect();
    assert!(
        drawn.len() > 12,
        "precondition: the table's cells are drawn"
    );
    let table = placed
        .nodes()
        .find(|node| node.path() == TABLE)
        .expect("headless has the table");
    let shift = d.rect(Target::suffix(TABLE)).min - table.rect().min;
    for (path, rect) in drawn {
        let node = placed
            .nodes()
            .find(|node| node.path() == path)
            .unwrap_or_else(|| panic!("headless lacks {path}"));
        assert_eq!(node.rect().translate(shift), rect, "{path}");
    }
}

const GUIDE: &str = "A sibling page, shown by the demo when a relative link names it. From here:";
const BACK: &str = "Back";

fn painted(d: &Driver<page::State>, text: &str) -> bool {
    support::shapes(d)
        .into_iter()
        .any(|shape| matches!(shape, Shape::Text(shown) if shown.galley.text() == text))
}

fn follow(d: &mut Driver<page::State>, text: &str, part: &str) {
    let link = d
        .painted()
        .expect(text)
        .part_rect(part)
        .expect("the part is on one row");
    press_and_release(d, link.center());
    d.frames(4);
}

#[test]
fn a_relative_link_to_a_sibling_page_shows_it_from_its_top() {
    let mut d = driver_sized(vec2(720.0, 360.0));
    d.shell().scroll_into_view(
        support::id_of(&d, &cell("Row(0)", 1)),
        Some(egui::Align::Max),
    );
    d.frames(4);
    let page = d.rect(Target::suffix("/Page"));
    assert!(
        d.rect(Target::suffix("/Page/Doc")).min.y < page.min.y - 600.0,
        "precondition: the sample is scrolled far down: {:?}",
        d.rect(Target::suffix("/Page/Doc"))
    );
    follow(&mut d, "a relative link", "relative");
    assert!(painted(&d, "Table page"), "table.md is shown");
    assert!(!painted(&d, PARAGRAPH), "the sample is not");
    let back = d.rect(Target::suffix("/Page/Back"));
    assert!(
        (back.min.y - (page.min.y + 24.0)).abs() < 1.0,
        "the new page starts at the top: {back:?} in {page:?}"
    );
}

#[test]
fn back_retraces_the_pages_followed() {
    let mut d = driver();
    assert!(!painted(&d, BACK), "the sample has nowhere to go back to");
    follow(&mut d, PARAGRAPH, "relative");
    assert!(painted(&d, GUIDE));
    follow(&mut d, "the table page", "the table page");
    assert!(painted(&d, "Table page"));
    follow(&mut d, BACK, BACK);
    assert!(painted(&d, GUIDE), "back to guide.md");
    follow(&mut d, BACK, BACK);
    assert!(painted(&d, PARAGRAPH), "back to the sample");
    assert!(!painted(&d, BACK));
}

#[test]
fn a_link_to_a_page_the_demo_lacks_stays_on_the_page() {
    let mut d = driver();
    follow(&mut d, PARAGRAPH, "relative");
    follow(
        &mut d,
        "a page that is not there",
        "a page that is not there",
    );
    assert!(painted(&d, GUIDE));
    assert_eq!(
        d.app().followed.last().map(String::as_str),
        Some("missing.md")
    );
}

const NESTED: &str = "# Top\n\n> ## Quoted\n\n- ### Listed\n\n## Top 1\n\n> ## Top\n";

fn nested(b: &mut eggshell_ui::Builder<'_, '_, ()>, _: &()) {
    b.markdown("doc", NESTED).add();
}

#[test]
fn heading_id_names_every_heading_however_deep_and_repeated() {
    let d = support::driver(
        vec2(400.0, 300.0),
        common::base_text(Theme::Light),
        (),
        nested,
    )
    .build();
    let doc = support::id_of(&d, "/\"doc\"");
    for anchor in ["top", "quoted", "listed", "top-1", "top-2"] {
        let heading = format!("Heading({anchor:?})");
        let (path, _) = support::nodes(&d)
            .into_iter()
            .find(|(path, _)| path.ends_with(&heading))
            .unwrap_or_else(|| panic!("precondition: {anchor} is drawn"));
        assert_eq!(support::id_of(&d, &path), heading_id(doc, anchor), "{path}");
    }
    assert!(
        support::nodes(&d)
            .into_iter()
            .any(|(path, _)| path.contains("Item(0)") && path.ends_with("Heading(\"listed\")")),
        "precondition: a heading sits in a list item"
    );
}

#[test]
fn a_link_to_a_heading_on_a_sibling_page_lands_on_it() {
    let mut d = driver_sized(vec2(720.0, 360.0));
    follow(&mut d, PARAGRAPH, "relative");
    follow(&mut d, "its alignments", "its alignments");
    let page = d.rect(Target::suffix("/Page"));
    let heading = d.rect(Target::suffix("/Page/Doc/Heading(\"alignments\")"));
    assert!(
        (page.min.y..page.min.y + 1.0).contains(&heading.min.y),
        "the heading is at the page's top: {heading:?} in {page:?}"
    );
}
