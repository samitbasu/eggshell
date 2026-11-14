//! Measurement of node content for taffy, answered by egui's galley layout.

use std::sync::Arc;

use egui::epaint::FontsView;
use egui::epaint::text::{ByteIndex, LayoutJob, LayoutSection};
use egui::{Align, Color32, FontSelection, Stroke, TextFormat, WidgetText};
use taffy::{AvailableSpace, Baselines, LayoutInput, LayoutOutput, Size};

use crate::prim::Prim;
use crate::text::{FontStyle, Numerals, TextAlign, TextDecoration, TextStyle, WhiteSpace};

/// What an application leaf's measure closure is asked.
///
/// Wraps the query taffy made: the dimensions already known, the space on
/// offer, and the leaf's resolved text style. The closure answers with the
/// size of the leaf's content; padding and border are added by taffy.
pub struct Measure<'a> {
    ctx: &'a egui::Context,
    input: &'a LayoutInput,
    text: &'a TextStyle,
}

impl<'a> Measure<'a> {
    pub(crate) fn new(ctx: &'a egui::Context, input: &'a LayoutInput, text: &'a TextStyle) -> Self {
        Self { ctx, input, text }
    }

    /// The egui context, for fonts and other shared state.
    pub fn ctx(&self) -> &egui::Context {
        self.ctx
    }

    /// Dimensions fixed by layout. A closure's answer for a known dimension is ignored.
    pub fn known(&self) -> Size<Option<f32>> {
        self.input.known_dimensions
    }

    /// The space on offer: definite, or the min-content or max-content probe.
    pub fn available(&self) -> Size<AvailableSpace> {
        self.input.available_space
    }

    /// The leaf's resolved text style.
    pub fn text(&self) -> &TextStyle {
        self.text
    }

    /// The known or definite width, if there is one.
    pub fn width(&self) -> Option<f32> {
        match (
            self.input.known_dimensions.width,
            self.input.available_space.width,
        ) {
            (Some(width), _) | (None, AvailableSpace::Definite(width)) => Some(width),
            _ => None,
        }
    }

    /// The size of `text` in the leaf's text style, wrapped as the query asks.
    pub fn text_size(&self, text: &str) -> Size<f32> {
        let leaf = TextLeaf::plain(text, self.text);
        self.ctx
            .fonts_mut(|fonts| measure_text(fonts, &leaf, self.input))
            .size
    }
}

/// A measure for leaves sized entirely by classes or by their parent.
pub fn fill(_: &Measure<'_>) -> Size<f32> {
    Size::ZERO
}

/// A text leaf: what it shows and the style it is measured in.
pub(crate) struct TextLeaf<'a> {
    pub text: Shown<'a>,
    pub style: &'a TextStyle,
}

/// The text a leaf shows.
pub(crate) enum Shown<'a> {
    /// Text in the leaf's style.
    Plain(&'a str),
    /// Text formatted by egui, laid out unwrapped.
    Rich(LayoutJob),
}

enum Wrap {
    Off,
    At(f32),
}

impl<'a> TextLeaf<'a> {
    /// A leaf showing `text` in `style`.
    pub(crate) fn plain(text: &'a str, style: &'a TextStyle) -> Self {
        Self {
            text: Shown::Plain(text),
            style,
        }
    }

    /// A leaf showing `text` with egui's formatting, in `style`'s font and
    /// colour where `text` does not set its own.
    pub(crate) fn rich(ctx: &egui::Context, text: &WidgetText, style: &'a TextStyle) -> Self {
        let job = text.clone().into_layout_job(
            &ink_style(ctx, style.color),
            FontSelection::FontId(style.spec.font_id()),
            Align::Min,
        );
        let mut job = Arc::unwrap_or_clone(job);
        for section in &mut job.sections {
            let format = &mut section.format;
            format.line_height = format.line_height.or(Some(
                style.spec.line_height * format.font_id.size / style.spec.size,
            ));
        }
        Self {
            text: Shown::Rich(job),
            style,
        }
    }

    /// The unwrapped job measurement and painting share.
    fn job(&self, fonts: &mut FontsView<'_>) -> LayoutJob {
        match &self.text {
            Shown::Plain(text) => self.plain_job(fonts, &self.style.case.apply(text)),
            Shown::Rich(job) => job.clone(),
        }
    }

    /// Under tabular numerals each digit is its own section, pushed right by
    /// the gap between it and the widest digit, so that every digit advances
    /// alike.
    fn plain_job(&self, fonts: &mut FontsView<'_>, text: &str) -> LayoutJob {
        let underline = match self.style.decoration {
            TextDecoration::None => Stroke::NONE,
            TextDecoration::Underline => Stroke::new(1.0, Color32::PLACEHOLDER),
        };
        let font_id = self.style.spec.font_id();
        let format = TextFormat {
            font_id: font_id.clone(),
            extra_letter_spacing: self.style.tracking.into(),
            line_height: Some(self.style.spec.line_height),
            italics: self.style.font_style == FontStyle::Italic,
            color: Color32::PLACEHOLDER,
            underline,
            ..TextFormat::default()
        };
        if self.style.numerals == Numerals::Proportional {
            return LayoutJob::single_section(text.to_owned(), format);
        }
        let widest = ('0'..='9')
            .map(|digit| fonts.glyph_width(&font_id, digit))
            .fold(0.0, f32::max);
        let mut job = LayoutJob {
            break_on_newline: true,
            ..LayoutJob::default()
        };
        for (at, c) in text.char_indices() {
            let end = at + c.len_utf8();
            let gap = if c.is_ascii_digit() {
                widest - fonts.glyph_width(&font_id, c)
            } else {
                0.0
            };
            job.append(&text[at..end], gap, format.clone());
        }
        job
    }

    fn galley(
        &self,
        fonts: &mut FontsView<'_>,
        mut job: LayoutJob,
        wrap: Wrap,
    ) -> std::sync::Arc<egui::Galley> {
        match wrap {
            Wrap::Off => job.wrap.max_width = f32::INFINITY,
            Wrap::At(width) => job.wrap.max_width = width,
        }
        self.clamp_rows(&mut job);
        fonts.layout_job(job)
    }

    /// The galley to paint in a content box `width` wide, anchored at the box
    /// edge the text aligns to. Plain text is coloured
    /// [`Color32::PLACEHOLDER`], for the painter to resolve.
    pub(crate) fn drawn_galley(
        &self,
        fonts: &mut FontsView<'_>,
        width: f32,
    ) -> std::sync::Arc<egui::Galley> {
        let mut job = self.job(fonts);
        job.halign = match self.style.align {
            TextAlign::Left => Align::LEFT,
            TextAlign::Center => Align::Center,
            TextAlign::Right => Align::RIGHT,
        };
        match self.style.white_space {
            WhiteSpace::Normal | WhiteSpace::Clamp(_) => job.wrap.max_width = width,
            WhiteSpace::NoWrap => job.wrap.max_width = f32::INFINITY,
            WhiteSpace::Truncate => {
                job.wrap.max_width = width;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
            }
        }
        self.clamp_rows(&mut job);
        fonts.layout_job(job)
    }

    fn clamp_rows(&self, job: &mut LayoutJob) {
        if let WhiteSpace::Clamp(rows) = self.style.white_space {
            job.wrap.max_rows = rows as usize;
        }
    }

    /// Width of the widest whitespace-separated word, unwrapped.
    fn min_content_width(&self, fonts: &mut FontsView<'_>, job: &LayoutJob) -> f32 {
        words(job)
            .map(|word| self.galley(fonts, word, Wrap::Off).size().x)
            .fold(0.0, f32::max)
    }

    /// How to wrap, and the width to report when it is not the galley's.
    ///
    /// A galley wrapped at the widest word can come out a fraction wider than
    /// that word, so the min-content width is reported exactly.
    fn plan(
        &self,
        fonts: &mut FontsView<'_>,
        job: &LayoutJob,
        input: &LayoutInput,
    ) -> (Wrap, Option<f32>) {
        if !matches!(
            self.style.white_space,
            WhiteSpace::Normal | WhiteSpace::Clamp(_)
        ) {
            return (Wrap::Off, None);
        }
        match (input.known_dimensions.width, input.available_space.width) {
            (Some(width), _) | (None, AvailableSpace::Definite(width)) => (Wrap::At(width), None),
            (None, AvailableSpace::MinContent) => {
                let width = self.min_content_width(fonts, job);
                (Wrap::At(width), Some(width))
            }
            (None, AvailableSpace::MaxContent) => (Wrap::Off, None),
        }
    }
}

/// An egui style in which egui's own text colours are `ink`: rich text's
/// `strong` and `weak` would otherwise take egui's visuals, which a box with
/// colours of its own (an inverted tooltip) does not follow.
fn ink_style(ctx: &egui::Context, ink: Color32) -> egui::Style {
    let mut style = (*ctx.global_style()).clone();
    style.override_font_id = None;
    style.override_text_style = None;
    let visuals = &mut style.visuals;
    visuals.override_text_color = Some(ink);
    visuals.widgets.active.fg_stroke.color = ink;
    visuals.weak_text_color = None;
    style
}

/// Each whitespace-separated word of `job`, as a job of its own that keeps
/// the formatting of the sections it spans.
fn words(job: &LayoutJob) -> impl Iterator<Item = LayoutJob> + '_ {
    let text = job.text.as_str();
    text.split_whitespace().map(move |word| {
        let start = ByteIndex(word.as_ptr() as usize - text.as_ptr() as usize);
        let end = start + word.len();
        let sections = job
            .sections
            .iter()
            .filter_map(|section| {
                let from = section.byte_range.start.max(start);
                let to = section.byte_range.end.min(end);
                (from < to).then(|| LayoutSection {
                    leading_space: if section.byte_range.start >= start {
                        section.leading_space
                    } else {
                        0.0
                    },
                    byte_range: from - start.0..to - start.0,
                    format: section.format.clone(),
                })
            })
            .collect();
        LayoutJob {
            text: word.to_owned(),
            sections,
            ..LayoutJob::default()
        }
    })
}

/// Answers a taffy measure query for a text leaf.
///
/// Baselines are reported when the text has at least one glyph.
pub(crate) fn measure_text(
    fonts: &mut FontsView<'_>,
    leaf: &TextLeaf<'_>,
    input: &LayoutInput,
) -> LayoutOutput {
    let job = leaf.job(fonts);
    let (wrap, exact_width) = leaf.plan(fonts, &job, input);
    let galley = leaf.galley(fonts, job, wrap);
    let natural = galley.size();
    let size = Size {
        width: input
            .known_dimensions
            .width
            .or(exact_width)
            .unwrap_or(natural.x),
        height: input.known_dimensions.height.unwrap_or(natural.y),
    };
    let first_baseline = galley
        .rows
        .first()
        .and_then(|placed| Some(placed.pos.y + placed.row.glyphs.first()?.pos.y));
    LayoutOutput::from_sizes_and_baselines(
        size,
        taffy::Rect::ZERO,
        Baselines::from_first(first_baseline),
    )
}

fn known_or(input: &LayoutInput, size: Size<f32>) -> LayoutOutput {
    LayoutOutput::from_outer_size(Size {
        width: input.known_dimensions.width.unwrap_or(size.width),
        height: input.known_dimensions.height.unwrap_or(size.height),
    })
}

/// A node's content as measured by the library or the application.
pub(crate) enum Content<'a> {
    /// Display content measured by the library.
    Prim(&'a Prim),
    /// An application leaf's measure.
    Leaf(&'a dyn Fn(&Measure<'_>) -> Size<f32>),
    /// Nothing: sized by layout alone.
    Empty,
}

/// Answers a taffy measure query for any node.
///
/// Icons and glyphs are one em square. Images have the size of their texture
/// when egui knows it, and none otherwise. Paint callbacks have no size.
pub(crate) fn measure_content(
    ctx: &egui::Context,
    content: Content<'_>,
    text: &TextStyle,
    input: &LayoutInput,
) -> LayoutOutput {
    match content {
        Content::Prim(Prim::Text(string)) => {
            let leaf = TextLeaf::plain(string, text);
            ctx.fonts_mut(|fonts| measure_text(fonts, &leaf, input))
        }
        Content::Prim(Prim::Rich(rich)) => {
            let leaf = TextLeaf::rich(ctx, rich, text);
            ctx.fonts_mut(|fonts| measure_text(fonts, &leaf, input))
        }
        Content::Prim(Prim::Icon(_)) => known_or(
            input,
            Size {
                width: text.spec.size,
                height: text.spec.size,
            },
        ),
        Content::Prim(Prim::Image(egui::ImageSource::Texture(texture))) => known_or(
            input,
            Size {
                width: texture.size.x,
                height: texture.size.y,
            },
        ),
        Content::Prim(Prim::Image(_) | Prim::Paint(_)) | Content::Empty => {
            known_or(input, Size::ZERO)
        }
        Content::Leaf(measure) => known_or(input, measure(&Measure::new(ctx, input, text))),
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::text::TextSpec;
    use egui::FontFamily;

    /// A context whose fonts are ready for measurement.
    pub fn fonts_ready_context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |_| {})
            .drop_without_applying_deltas();
        ctx
    }

    pub fn body_style() -> TextStyle {
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 18.0,
                family: FontFamily::Proportional,
            },
            Color32::BLACK,
        )
    }

    pub fn input(known: Size<Option<f32>>, available: Size<AvailableSpace>) -> LayoutInput {
        LayoutInput {
            known_dimensions: known,
            available_space: available,
            ..LayoutInput::HIDDEN
        }
    }

    pub fn measure(
        ctx: &egui::Context,
        text: &str,
        known: Size<Option<f32>>,
        available: Size<AvailableSpace>,
    ) -> LayoutOutput {
        let style = body_style();
        let leaf = TextLeaf::plain(text, &style);
        ctx.fonts_mut(|fonts| measure_text(fonts, &leaf, &input(known, available)))
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    const UNKNOWN: Size<Option<f32>> = Size {
        width: None,
        height: None,
    };
    const MAX: Size<AvailableSpace> = Size {
        width: AvailableSpace::MaxContent,
        height: AvailableSpace::MaxContent,
    };

    fn width_with(avail: AvailableSpace, text: &str, ctx: &egui::Context) -> LayoutOutput {
        measure(
            ctx,
            text,
            UNKNOWN,
            Size {
                width: avail,
                height: AvailableSpace::MaxContent,
            },
        )
    }

    #[test]
    fn max_content_is_single_line_width() {
        let ctx = fonts_ready_context();
        let one = width_with(AvailableSpace::MaxContent, "alpha", &ctx).size;
        let two = width_with(AvailableSpace::MaxContent, "alpha alpha", &ctx).size;
        assert!(one.width > 0.0 && one.height > 0.0);
        assert!(two.width > one.width);
        assert_eq!(two.height, one.height, "single line");
    }

    #[test]
    fn min_content_is_widest_word() {
        let ctx = fonts_ready_context();
        let widest = width_with(AvailableSpace::MaxContent, "extraordinary", &ctx).size;
        let short = width_with(AvailableSpace::MaxContent, "an", &ctx).size;
        assert!(widest.width > short.width, "precondition: words differ");

        let min = width_with(AvailableSpace::MinContent, "an extraordinary an", &ctx).size;
        assert_eq!(min.width, widest.width);
        assert!(min.height > widest.height, "each word on its own row");
    }

    #[test]
    fn definite_width_wraps() {
        let ctx = fonts_ready_context();
        let text = "one two three four five six seven eight";
        let single = width_with(AvailableSpace::MaxContent, text, &ctx).size;
        let narrow = single.width / 3.0;
        let min = width_with(AvailableSpace::MinContent, text, &ctx).size;
        assert!(
            narrow > min.width,
            "precondition: narrow fits the longest word"
        );

        let wrapped = width_with(AvailableSpace::Definite(narrow), text, &ctx).size;
        assert!(wrapped.width <= narrow);
        assert!(wrapped.height > single.height);

        let known = measure(
            &ctx,
            text,
            Size {
                width: Some(narrow),
                height: None,
            },
            MAX,
        )
        .size;
        assert_eq!(
            known,
            Size {
                width: narrow,
                height: wrapped.height
            }
        );
    }

    #[test]
    fn both_known_dimensions_are_returned_unchanged() {
        let ctx = fonts_ready_context();
        let known = Size {
            width: Some(123.0),
            height: Some(45.0),
        };
        let out = measure(&ctx, "anything at all", known, MAX);
        assert_eq!(
            out.size,
            Size {
                width: 123.0,
                height: 45.0
            }
        );
    }

    #[test]
    fn first_baseline_lies_within_the_line() {
        let ctx = fonts_ready_context();
        let out = measure(&ctx, "Hello", UNKNOWN, MAX);
        let baseline = out.baselines.first.expect("text has a baseline");
        assert!(baseline > 0.0 && baseline < out.size.height);
    }

    #[test]
    fn empty_text_has_no_baseline() {
        let ctx = fonts_ready_context();
        assert_eq!(measure(&ctx, "", UNKNOWN, MAX).baselines.first, None);
    }

    /// Run with `cargo test --release -- --ignored --nocapture min_content_cost`.
    #[test]
    #[ignore = "timing report, not a correctness test"]
    fn min_content_cost() {
        use std::time::Instant;
        let ctx = fonts_ready_context();
        let paragraph = |salt: usize| {
            (0..50)
                .map(|i| format!("word{salt}x{i}longer"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let cold_runs = 200;
        let texts: Vec<_> = (0..cold_runs).map(paragraph).collect();
        let start = Instant::now();
        for text in &texts {
            width_with(AvailableSpace::MinContent, text, &ctx);
        }
        let cold = start.elapsed() / cold_runs as u32;

        let warm_runs = 2000;
        let start = Instant::now();
        for _ in 0..warm_runs {
            width_with(AvailableSpace::MinContent, &texts[0], &ctx);
        }
        let warm = start.elapsed() / warm_runs;
        println!("50-word MinContent: cold {cold:?}, warm {warm:?}");
    }
}
