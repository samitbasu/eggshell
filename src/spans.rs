//! Rich inline text: one wrapping text node made of styled spans.

use egui::Stroke;
use egui::epaint::text::{ByteIndex, LayoutJob, LayoutSection};
use egui::{Color32, FontFamily, FontId, Galley, Pos2, Rect, TextFormat, pos2};

use crate::contract::RichTextStyle;
use crate::text::{FontStyle, TextDecoration, TextSpec, TextStyle, Weight};

/// TODO(prose)
///
/// ```
/// use eggshell_ui::{Span, Weight};
///
/// let span = Span::new("bold").weight(Weight::Bold);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    text: String,
    weight: Option<Weight>,
    font_style: Option<FontStyle>,
    color: Option<Color32>,
    font: SpanFont,
    link: Option<String>,
}

/// The font a span is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SpanFont {
    #[default]
    Text,
    Monospace,
    /// Monospace on the style's inline-code tint.
    Code,
}

impl Span {
    /// TODO(prose)
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            weight: None,
            font_style: None,
            color: None,
            font: SpanFont::Text,
            link: None,
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn weight(self, weight: Weight) -> Self {
        Self {
            weight: Some(weight),
            ..self
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn font_style(self, font_style: FontStyle) -> Self {
        Self {
            font_style: Some(font_style),
            ..self
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn color(self, color: Color32) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn monospace(self) -> Self {
        Self {
            font: SpanFont::Monospace,
            ..self
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn code(self) -> Self {
        Self {
            font: SpanFont::Code,
            ..self
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn link(self, href: impl Into<String>) -> Self {
        Self {
            link: Some(href.into()),
            ..self
        }
    }

    /// TODO(prose)
    pub fn text(&self) -> &str {
        &self.text
    }

    /// TODO(prose)
    pub fn href(&self) -> Option<&str> {
        self.link.as_deref()
    }
}

impl<S: Into<String>> From<S> for Span {
    fn from(text: S) -> Self {
        Self::new(text)
    }
}

/// TODO(prose)
///
/// Empty spans are dropped.
///
/// ```
/// use eggshell_ui::{Span, Spans};
///
/// let spans = Spans::from(vec![Span::new("see "), Span::new("the docs").link("#docs")]);
/// assert_eq!(spans.text(), "see the docs");
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spans(Vec<Span>);

impl Spans {
    /// TODO(prose)
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO(prose)
    pub fn push(&mut self, span: impl Into<Span>) {
        let span = span.into();
        if !span.text.is_empty() {
            self.0.push(span);
        }
    }

    /// TODO(prose)
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// TODO(prose)
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// TODO(prose)
    pub fn iter(&self) -> impl Iterator<Item = &Span> {
        self.0.iter()
    }

    /// TODO(prose)
    pub fn text(&self) -> String {
        self.0.iter().map(|span| span.text.as_str()).collect()
    }

    /// One section per span over `style` in `rich`'s looks, and what each
    /// one marks. Inline code takes the tint's padding as layout space on
    /// both sides.
    pub(crate) fn laid_out(
        &self,
        ctx: &egui::Context,
        style: &TextStyle,
        rich: &RichTextStyle,
    ) -> (LayoutJob, Vec<Mark>) {
        let mut job = LayoutJob::default();
        let mut marks = Vec::with_capacity(self.0.len());
        let mut pad_after = 0.0;
        for span in &self.0 {
            let pad = match span.font {
                SpanFont::Code => rich.code.padding_x,
                SpanFont::Text | SpanFont::Monospace => 0.0,
            };
            let text = style.case.apply(&span.text);
            let start = job.text.len();
            job.text.push_str(&text);
            job.sections.push(LayoutSection {
                leading_space: pad_after + pad,
                byte_range: ByteIndex(start)..ByteIndex(job.text.len()),
                format: span.format(ctx, style, rich),
            });
            marks.push(Mark {
                chars: text.chars().count(),
                font: span.font,
                link: span.link.clone(),
            });
            pad_after = pad;
        }
        (job, marks)
    }
}

impl Span {
    fn format(
        &self,
        ctx: &egui::Context,
        inherited: &TextStyle,
        rich: &RichTextStyle,
    ) -> TextFormat {
        let style = TextStyle {
            spec: TextSpec {
                weight: self.weight.unwrap_or(match self.link {
                    Some(_) => rich.link_weight.max(inherited.spec.weight),
                    None => inherited.spec.weight,
                }),
                ..inherited.spec.clone()
            },
            font_style: self.font_style.unwrap_or(inherited.font_style),
            decoration: match self.link {
                Some(_) => TextDecoration::Underline,
                None => inherited.decoration,
            },
            ..inherited.clone()
        };
        let font_id = match self.font {
            SpanFont::Text => style.spec.font_id(ctx),
            SpanFont::Monospace | SpanFont::Code => FontId::new(
                style.spec.size * f32::from(rich.code.size),
                FontFamily::Monospace,
            ),
        };
        let mut format = crate::measure::text_format(&style, font_id);
        if let Some(color) = self.color.or(self.link.as_ref().map(|_| rich.link)) {
            format.color = color;
            format.underline.color = match self.link {
                Some(_) => rich.link_underline.unwrap_or(color),
                None => color,
            };
        }
        format
    }
}

impl<S: Into<Span>> FromIterator<S> for Spans {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        let mut spans = Self::new();
        iter.into_iter().for_each(|span| spans.push(span));
        spans
    }
}

impl From<Vec<Span>> for Spans {
    fn from(spans: Vec<Span>) -> Self {
        spans.into_iter().collect()
    }
}

impl From<Span> for Spans {
    fn from(span: Span) -> Self {
        std::iter::once(span).collect()
    }
}

impl From<&str> for Spans {
    fn from(text: &str) -> Self {
        Span::new(text).into()
    }
}

impl From<String> for Spans {
    fn from(text: String) -> Self {
        Span::new(text).into()
    }
}

/// What one span of a drawn galley marks.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Mark {
    chars: usize,
    font: SpanFont,
    link: Option<String>,
}

/// One row's stretch of consecutive glyphs of one span, in galley space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Run {
    pub span: usize,
    /// From the first glyph's left edge to the last one's right edge, the
    /// height of the row.
    pub row: Rect,
    /// The same stretch, the height of the font's own box.
    pub ink: Rect,
}

/// Every run of `galley`, whose sections are `marks`' spans in order.
pub(crate) fn runs(galley: &Galley, marks: &[Mark]) -> Vec<Run> {
    runs_of(galley, marks.iter().map(|mark| mark.chars))
}

/// Every run of `galley`: the glyphs of one row that belong to the same
/// owner, where owner `n` is the next `chars[n]` characters.
fn runs_of(galley: &Galley, chars: impl IntoIterator<Item = usize>) -> Vec<Run> {
    let owners: Vec<usize> = chars
        .into_iter()
        .enumerate()
        .flat_map(|(owner, chars)| std::iter::repeat_n(owner, chars))
        .collect();
    let mut runs: Vec<Run> = Vec::new();
    let mut char_at = 0;
    for placed in &galley.rows {
        let mut open: Option<Run> = None;
        for glyph in &placed.row.glyphs {
            let Some(&span) = owners.get(char_at) else {
                break;
            };
            char_at += 1;
            let left = placed.pos.x + glyph.pos.x;
            let right = placed.pos.x + glyph.max_x();
            let ink_top = placed.pos.y + glyph.pos.y - glyph.font_ascent;
            let ink = Rect::from_x_y_ranges(left..=right, ink_top..=ink_top + glyph.font_height);
            let row = Rect::from_x_y_ranges(left..=right, placed.rect().y_range());
            match &mut open {
                Some(run) if run.span == span => {
                    run.row = run.row.union(row);
                    run.ink = run.ink.union(ink);
                }
                _ => {
                    runs.extend(open.take());
                    open = Some(Run { span, row, ink });
                }
            }
        }
        runs.extend(open);
        if placed.ends_with_newline {
            char_at += 1;
        }
    }
    runs
}

/// The underlines of `galley`, which was laid out without them, its
/// sections underlined by `strokes`: one line along the foot of each run's
/// font box, where egui draws an underline when the line is no taller than
/// the font. egui's own sits at the foot of the line box, which leading
/// lowers towards the next line.
pub(crate) fn underlines(
    galley: &Galley,
    strokes: &[Stroke],
    anchor: Pos2,
) -> Vec<([Pos2; 2], Stroke)> {
    let job = &galley.job;
    let chars = job.sections.iter().map(|section| {
        job.text
            .get(section.byte_range.start.0..section.byte_range.end.0)
            .map_or(0, |text| text.chars().count())
    });
    let mut lines: Vec<([Pos2; 2], Stroke)> = Vec::new();
    for run in runs_of(galley, chars) {
        let Some(&stroke) = strokes.get(run.span).filter(|stroke| !stroke.is_empty()) else {
            continue;
        };
        let y = anchor.y + run.ink.bottom();
        let (left, right) = (anchor.x + run.ink.left(), anchor.x + run.ink.right());
        match lines.last_mut() {
            Some(([_, end], last))
                if *last == stroke && end.y == y && (end.x - left).abs() < 0.5 =>
            {
                end.x = right;
            }
            _ => lines.push(([pos2(left, y), pos2(right, y)], stroke)),
        }
    }
    lines
}

/// The tints behind `galley`'s inline code, drawn at `anchor`.
pub(crate) fn code_tints(
    runs: &[Run],
    marks: &[Mark],
    rich: &RichTextStyle,
    anchor: Pos2,
) -> Vec<Rect> {
    runs.iter()
        .filter(|run| {
            marks
                .get(run.span)
                .is_some_and(|mark| mark.font == SpanFont::Code)
        })
        .map(|run| {
            run.ink
                .expand2(egui::vec2(rich.code.padding_x, rich.code.padding_y))
                .translate(anchor.to_vec2())
        })
        .collect()
}

/// The link under `pointer`, for `galley` drawn at `anchor`.
pub(crate) fn link_at<'m>(
    runs: &[Run],
    marks: &'m [Mark],
    anchor: Pos2,
    pointer: Pos2,
) -> Option<&'m str> {
    let local = pos2(pointer.x - anchor.x, pointer.y - anchor.y);
    runs.iter()
        .find(|run| run.row.contains(local))
        .and_then(|run| marks.get(run.span)?.link.as_deref())
}

/// Shows the pointing hand over a link, and follows the link `response`'s
/// click lands on: the platform opens an external one, and the node's
/// `on_link` takes an internal one.
pub(crate) fn follow(
    response: &egui::Response,
    runs: &[Run],
    marks: &[Mark],
    anchor: Pos2,
    node: egui::Id,
) {
    let over = |pointer| link_at(runs, marks, anchor, pointer);
    let ctx = &response.ctx;
    if response.hover_pos().and_then(over).is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if !response.clicked() {
        return;
    }
    let Some(href) = response.interact_pointer_pos().and_then(over) else {
        return;
    };
    if is_external(href) {
        ctx.open_url(egui::OpenUrl {
            url: href.to_owned(),
            new_tab: ctx.input(|input| input.modifiers.any()),
        });
    } else {
        record_followed(ctx, node, href);
    }
}

/// Whether `href` names a URL scheme (`https:`, `mailto:`), as RFC 3986
/// spells one, rather than a place within the application.
pub(crate) fn is_external(href: &str) -> bool {
    let Some((scheme, _)) = href.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// Whether any span is a link.
pub(crate) fn has_links(marks: &[Mark]) -> bool {
    marks.iter().any(|mark| mark.link.is_some())
}

#[derive(Clone, Default)]
struct Followed(String);

fn followed_id(node: egui::Id) -> egui::Id {
    node.with("eggshell-followed-link")
}

/// Leaves `href` for the node's `on_link` to take this pass.
pub(crate) fn record_followed(ctx: &egui::Context, node: egui::Id, href: &str) {
    ctx.data_mut(|data| data.insert_temp(followed_id(node), Followed(href.to_owned())));
}

/// The link the node's text followed this pass, if any.
pub(crate) fn take_followed(ctx: &egui::Context, node: egui::Id) -> Option<String> {
    ctx.data_mut(|data| data.remove_temp::<Followed>(followed_id(node)))
        .map(|followed| followed.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::testing::{body_style, fonts_ready_context};
    use crate::text::TextCase;

    #[test]
    fn runs_follow_the_cased_text() {
        let ctx = fonts_ready_context();
        let style = TextStyle {
            case: TextCase::Upper,
            ..body_style()
        };
        let spans = Spans::from(vec![Span::new("ßß"), Span::new("x").code()]);
        let (job, marks) = spans.laid_out(
            &ctx,
            &style,
            &crate::Style::from_egui(&ctx.global_style()).rich,
        );
        assert_eq!(job.text, "SSSSX", "precondition: casing lengthens the text");
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
        let runs = runs(&galley, &marks);
        assert_eq!(runs.len(), 2);
        let glyphs = &galley.rows[0].row.glyphs;
        assert_eq!(
            runs[0].row.max.x,
            glyphs[3].max_x(),
            "SSSS is the first span"
        );
        assert_eq!(runs[1].row.min.x, glyphs[4].pos.x, "X is the code span");
    }

    #[test]
    fn a_scheme_makes_a_link_external() {
        for href in ["https://example.com", "mailto:a@b.c", "git+ssh://host/repo"] {
            assert!(is_external(href), "{href}");
        }
        for href in [
            "#anchor",
            "guide/intro.md",
            "../up",
            "/root",
            "1:2",
            ":x",
            "",
        ] {
            assert!(!is_external(href), "{href}");
        }
    }
}
