//! Rich inline text (#249) through the real path: styled spans measured and
//! drawn as one galley, links hit-tested per span, inline code on a tint.

use std::sync::Arc;

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, FontStyle, Span, Spans, Weight, tw};
use egui::epaint::Shape;
use egui::{
    Color32, CursorIcon, Event, FontFamily, Galley, OutputCommand, Pos2, Rect, Theme, pos2, vec2,
};

use crate::support::{self, press_and_release};

use eggshell_examples::common;

#[path = "../../examples/rich_text.rs"]
mod rich_text;

const HREF: &str = "#docs";
const LINK: &str = "the docs";
const URL: &str = "https://example.com/source";
const EXTERNAL: &str = "the source";
const BOLD: &str = "bold";
const CODE: &str = "code()";
const LONG: &str = "A sentence long enough to wrap over several lines in a narrow column.";

#[derive(Clone, Copy, Default)]
enum Select {
    #[default]
    Auto,
    None,
}

#[derive(Default)]
struct App {
    select: Select,
    followed: Vec<String>,
}

fn sample() -> Spans {
    Spans::from(vec![
        Span::new("Read "),
        Span::new(LINK).link(HREF),
        Span::new(" or "),
        Span::new(EXTERNAL).link(URL),
        Span::new(" with "),
        Span::new(BOLD).weight(Weight::Bold),
        Span::new(", "),
        Span::new("slanted").font_style(FontStyle::Italic),
        Span::new(" and "),
        Span::new(CODE).code(),
        Span::new(" spans."),
    ])
}

fn sample_text() -> String {
    format!("Read {LINK} or {EXTERNAL} with {BOLD}, slanted and {CODE} spans.")
}

/// ```html
/// <div class="flex flex-col gap-4 p-4">
///   <p>Read <a href="…">the docs</a> with <b>bold</b>, <i>slanted</i> and <code>code()</code> spans.</p>
///   <div class="w-40"><p>A sentence long…</p><p>A sentence long…</p></div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let select = match app.select {
        Select::Auto => tw(),
        Select::None => tw().select_none(),
    };
    b.div(tw().flex().flex_col().gap(4.0).p(4.0), |b| {
        b.spans("para", select, sample())
            .on_link(|app: &mut App, href: &str| app.followed.push(href.to_owned()));
        b.div(tw().flex().flex_row().gap(4.0), |b| {
            b.div(tw().w_px(120.0), |b| {
                b.spans("spans", tw(), vec![Span::new(LONG)]);
            });
            b.div(tw().w_px(120.0), |b| {
                b.span("plain", tw(), LONG);
            });
        });
    });
}

fn driver(select: Select) -> Driver<App> {
    let mut d = support::driver(
        vec2(520.0, 300.0),
        common::base_text(Theme::Light),
        App {
            select,
            ..App::default()
        },
        view,
    )
    .build();
    d.frames(4);
    d
}

/// The screen rect of `part` within the paragraph, which must sit on one row.
fn rect_of_part(d: &Driver<App>, part: &str) -> Rect {
    d.painted()
        .expect(&sample_text())
        .part_rect(part)
        .expect("the part is on one row")
}

fn galley_of(d: &Driver<App>, text: &str) -> (Arc<Galley>, Pos2) {
    let shown = d.painted().expect(text);
    let galley = shown.galley();
    (
        Arc::new(galley.clone()),
        shown.rect().min - galley.rect.min.to_vec2(),
    )
}

#[test]
fn the_spans_are_one_galley_with_a_section_per_span() {
    let d = driver(Select::Auto);
    let galley = d.painted().expect(&sample_text()).galley().clone();
    assert_eq!(galley.job.sections.len(), sample().len());
}

#[test]
fn a_span_weight_draws_the_face_of_its_weight() {
    let d = driver(Select::Auto);
    let bold = FontFamily::Name(format!("Geist/{}", Weight::Bold.number()).into());
    assert!(
        d.ctx().fonts(|fonts| fonts.families().contains(&bold)),
        "precondition: the preset registered Geist Bold"
    );
    let galley = d.painted().expect(&sample_text()).galley().clone();
    assert_eq!(support::section_of(&galley, BOLD).font_id.family, bold);
    assert_eq!(
        support::section_of(&galley, "Read").font_id.family,
        FontFamily::Proportional
    );
    assert!(support::section_of(&galley, "slanted").italics);
}

#[test]
fn a_link_takes_the_style_link_colour() {
    let d = driver(Select::Auto);
    let galley = d.painted().expect(&sample_text()).galley().clone();
    let link = d.shell().style().rich.link;
    assert_eq!(support::section_of(&galley, LINK).color, link);
    assert_ne!(support::section_of(&galley, "Read").color, link);
}

/// The code tint painted over `glyphs`.
fn tint_over(d: &Driver<App>, glyphs: Rect) -> egui::epaint::RectShape {
    let fill = d.shell().style().rich.code.fill;
    support::shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == fill && rect.rect.intersects(glyphs) => {
                Some(rect.clone())
            }
            _ => None,
        })
        .expect("a tint was painted over the code glyphs")
}

#[test]
fn inline_code_is_monospace_on_a_rounded_tint_behind_its_glyphs() {
    let d = driver(Select::Auto);
    let galley = d.painted().expect(&sample_text()).galley().clone();
    assert_eq!(
        support::section_of(&galley, CODE).font_id.family,
        FontFamily::Monospace
    );
    let glyphs = rect_of_part(&d, CODE);
    let fill = d.shell().style().rich.code.fill;
    assert_ne!(fill, Color32::TRANSPARENT, "precondition: code has a tint");
    let tint = tint_over(&d, glyphs);
    assert!(
        tint.rect.min.x <= glyphs.min.x + 0.5
            && tint.rect.max.x >= glyphs.max.x - 0.5
            && tint.rect.y_range().contains(glyphs.center().y),
        "the tint {:?} covers the code glyphs {glyphs:?}",
        tint.rect
    );
    assert!(
        tint.rect.width() < glyphs.width() + 20.0,
        "the tint hugs the code, not the row"
    );
    assert!(tint.corner_radius.nw > 0, "the tint is rounded");
}

/// The screen rect of every glyph of the paragraph, in order; the paragraph
/// has no line breaks.
fn glyph_rects(d: &Driver<App>) -> Vec<Rect> {
    let (galley, at) = galley_of(d, &sample_text());
    galley
        .rows
        .iter()
        .flat_map(|placed| {
            placed.row.glyphs.iter().map(move |glyph| {
                Rect::from_x_y_ranges(
                    placed.pos.x + glyph.pos.x..=placed.pos.x + glyph.max_x(),
                    placed.rect().y_range(),
                )
                .translate(at.to_vec2())
            })
        })
        .collect()
}

#[test]
fn the_code_tint_does_not_cover_its_neighbours() {
    let d = driver(Select::Auto);
    let text = sample_text();
    let glyphs = glyph_rects(&d);
    assert_eq!(
        glyphs.len(),
        text.chars().count(),
        "precondition: one row of glyphs"
    );
    let first = text[..text.find(CODE).expect("code in the sample")]
        .chars()
        .count();
    let (before, after) = (glyphs[first - 1], glyphs[first + CODE.chars().count()]);
    let tint = tint_over(&d, rect_of_part(&d, CODE)).rect;
    assert!(
        tint.min.x >= before.max.x - 0.5,
        "the tint {tint:?} starts after the glyph before it, {before:?}"
    );
    assert!(
        tint.max.x <= after.min.x + 0.5,
        "the tint {tint:?} ends before the glyph after it, {after:?}"
    );
}

#[test]
fn clicking_an_internal_link_calls_on_link_with_its_href() {
    let mut d = driver(Select::Auto);
    let link = rect_of_part(&d, LINK);
    press_and_release(&mut d, link.center());
    d.frames(2);
    assert_eq!(d.app().followed, vec![HREF.to_owned()]);
    assert!(
        support::opened_urls(&d).is_empty(),
        "an internal link opens nothing"
    );
}

#[test]
fn clicking_an_external_link_opens_the_url_and_not_on_link() {
    let mut d = driver(Select::Auto);
    let link = rect_of_part(&d, EXTERNAL);
    d.hover(link.center());
    let mut opened = Vec::new();
    for pressed in [true, false] {
        d.event(Event::PointerButton {
            pos: link.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        opened.extend(support::opened_urls(&d));
    }
    d.frames(2);
    assert_eq!(opened, vec![URL.to_owned()]);
    assert!(d.app().followed.is_empty());
}

#[test]
fn clicking_beside_a_link_follows_nothing() {
    let mut d = driver(Select::Auto);
    let plain = rect_of_part(&d, "Read");
    press_and_release(&mut d, plain.center());
    d.frames(2);
    assert!(d.app().followed.is_empty());
}

#[test]
fn a_link_in_unselectable_text_still_follows() {
    let mut d = driver(Select::None);
    let link = rect_of_part(&d, LINK);
    press_and_release(&mut d, link.center());
    d.frames(2);
    assert_eq!(d.app().followed, vec![HREF.to_owned()]);
}

#[test]
fn hovering_a_link_shows_the_pointing_hand() {
    let mut d = driver(Select::Auto);
    let (link, plain) = (rect_of_part(&d, LINK), rect_of_part(&d, "Read"));
    d.hover(link.center());
    assert_eq!(
        d.harness().output().platform_output.cursor_icon,
        CursorIcon::PointingHand
    );
    d.hover(plain.center());
    assert_ne!(
        d.harness().output().platform_output.cursor_icon,
        CursorIcon::PointingHand
    );
}

#[test]
fn dragging_from_a_link_selects_and_copies_the_plain_text() {
    let mut d = driver(Select::Auto);
    let link = rect_of_part(&d, LINK);
    let paragraph = d.rect(Target::suffix("/\"para\""));
    d.drag(
        pos2(link.min.x + 1.0, link.center().y),
        pos2(paragraph.max.x - 1.0, link.center().y),
    );
    assert!(d.app().followed.is_empty(), "a drag is not a click");
    d.event(Event::Copy);
    let copied = d
        .harness()
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .expect("something was copied");
    assert!(
        copied.starts_with(LINK) && copied.contains(CODE),
        "copied {copied:?}"
    );
}

#[test]
fn a_single_span_wraps_exactly_as_plain_text() {
    let d = driver(Select::Auto);
    let (spans, plain) = (
        d.rect(Target::suffix("/\"spans\"")),
        d.rect(Target::suffix("/\"plain\"")),
    );
    let line = common::base_text(Theme::Light).spec.line_height;
    assert!(spans.height() > 2.0 * line, "precondition: the text wraps");
    assert_eq!(spans.size(), plain.size());
}

#[test]
fn the_example_width_slider_reflows_its_paragraphs() {
    let mut d = support::driver(
        vec2(720.0, 600.0),
        common::base_text(Theme::Light),
        rich_text::State::default(),
        rich_text::layout,
    )
    .build();
    d.frames(4);
    let paragraph = |d: &Driver<rich_text::State>| d.rect(Target::suffix("/Article/Wrapping"));
    let before = paragraph(&d);
    let slider = d.rect(Target::suffix("/Width/Track"));
    press_and_release(&mut d, pos2(slider.min.x + 1.0, slider.center().y));
    d.frames(3);
    let width = d.app().width;
    assert!(
        width < before.width() / 2.0,
        "pressing the track's start narrows the article, to {width}"
    );
    let after = paragraph(&d);
    assert!(
        (after.width() - width).abs() < 0.5,
        "{after:?} is {width} wide"
    );
    assert!(
        after.height() > before.height(),
        "the narrower paragraph wraps onto more rows"
    );
}
