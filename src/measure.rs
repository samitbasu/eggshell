//! Measurement of node content for taffy, answered by egui's galley layout.

use egui::epaint::{FontsView, text::LayoutJob};
use egui::{Align, Color32, Stroke, TextFormat};
use taffy::{AvailableSpace, Baselines, LayoutInput, LayoutOutput, Size};

use crate::prim::Prim;
use crate::text::{FontStyle, TextAlign, TextDecoration, TextStyle, WhiteSpace};

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
        let leaf = TextLeaf {
            text,
            style: self.text,
        };
        self.ctx
            .fonts_mut(|fonts| measure_text(fonts, &leaf, self.input))
            .size
    }
}

/// A measure for leaves sized entirely by classes or by their parent.
pub fn fill(_: &Measure<'_>) -> Size<f32> {
    Size::ZERO
}

/// A text leaf: its text and the style it is measured in.
pub(crate) struct TextLeaf<'a> {
    pub text: &'a str,
    pub style: &'a TextStyle,
}

enum Wrap {
    Off,
    At(f32),
}

impl TextLeaf<'_> {
    fn job(&self, text: &str) -> LayoutJob {
        let underline = match self.style.decoration {
            TextDecoration::None => Stroke::NONE,
            TextDecoration::Underline => Stroke::new(1.0, Color32::PLACEHOLDER),
        };
        LayoutJob::single_section(
            text.to_owned(),
            TextFormat {
                font_id: self.style.spec.font_id(),
                extra_letter_spacing: self.style.tracking.into(),
                line_height: Some(self.style.spec.line_height),
                italics: self.style.font_style == FontStyle::Italic,
                color: Color32::PLACEHOLDER,
                underline,
                ..TextFormat::default()
            },
        )
    }

    fn galley(
        &self,
        fonts: &mut FontsView<'_>,
        text: &str,
        wrap: Wrap,
    ) -> std::sync::Arc<egui::Galley> {
        let mut job = self.job(text);
        match wrap {
            Wrap::Off => job.wrap.max_width = f32::INFINITY,
            Wrap::At(width) => job.wrap.max_width = width,
        }
        fonts.layout_job(job)
    }

    /// The galley to paint in a content box `width` wide, anchored at the box
    /// edge the text aligns to. Its colour is [`Color32::PLACEHOLDER`], for
    /// the painter to resolve.
    pub(crate) fn drawn_galley(
        &self,
        fonts: &mut FontsView<'_>,
        width: f32,
    ) -> std::sync::Arc<egui::Galley> {
        let text = self.style.case.apply(self.text);
        let mut job = self.job(&text);
        job.halign = match self.style.align {
            TextAlign::Left => Align::LEFT,
            TextAlign::Center => Align::Center,
            TextAlign::Right => Align::RIGHT,
        };
        match self.style.white_space {
            WhiteSpace::Normal => job.wrap.max_width = width,
            WhiteSpace::NoWrap => job.wrap.max_width = f32::INFINITY,
            WhiteSpace::Truncate => {
                job.wrap.max_width = width;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
            }
        }
        fonts.layout_job(job)
    }

    /// Width of the widest whitespace-separated word, unwrapped.
    fn min_content_width(&self, fonts: &mut FontsView<'_>, text: &str) -> f32 {
        text.split_whitespace()
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
        text: &str,
        input: &LayoutInput,
    ) -> (Wrap, Option<f32>) {
        if self.style.white_space != WhiteSpace::Normal {
            return (Wrap::Off, None);
        }
        match (input.known_dimensions.width, input.available_space.width) {
            (Some(width), _) | (None, AvailableSpace::Definite(width)) => (Wrap::At(width), None),
            (None, AvailableSpace::MinContent) => {
                let width = self.min_content_width(fonts, text);
                (Wrap::At(width), Some(width))
            }
            (None, AvailableSpace::MaxContent) => (Wrap::Off, None),
        }
    }
}

/// Answers a taffy measure query for a text leaf.
///
/// Baselines are reported when the text has at least one glyph.
pub(crate) fn measure_text(
    fonts: &mut FontsView<'_>,
    leaf: &TextLeaf<'_>,
    input: &LayoutInput,
) -> LayoutOutput {
    let text = leaf.style.case.apply(leaf.text);
    let (wrap, exact_width) = leaf.plan(fonts, &text, input);
    let galley = leaf.galley(fonts, &text, wrap);
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
            let leaf = TextLeaf {
                text: string,
                style: text,
            };
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
        let leaf = TextLeaf {
            text,
            style: &style,
        };
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
