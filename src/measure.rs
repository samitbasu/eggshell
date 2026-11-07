//! Text measurement for taffy leaves, answered by egui's galley layout.

use egui::{Color32, FontId, epaint::FontsView};
use taffy::{AvailableSpace, Baselines, LayoutInput, LayoutOutput, Size};

/// Node context for a text leaf.
pub(crate) struct TextLeaf {
    pub text: String,
    pub font: FontId,
}

enum Wrap {
    Off,
    At(f32),
}

impl TextLeaf {
    fn galley(&self, fonts: &mut FontsView<'_>, wrap: Wrap) -> std::sync::Arc<egui::Galley> {
        let text = self.text.clone();
        let font = self.font.clone();
        match wrap {
            Wrap::Off => fonts.layout_no_wrap(text, font, Color32::PLACEHOLDER),
            Wrap::At(width) => fonts.layout(text, font, Color32::PLACEHOLDER, width),
        }
    }

    /// Width of the widest whitespace-separated word, unwrapped.
    fn min_content_width(&self, fonts: &mut FontsView<'_>) -> f32 {
        self.text
            .split_whitespace()
            .map(|word| {
                let word = Self {
                    text: word.to_owned(),
                    font: self.font.clone(),
                };
                word.galley(fonts, Wrap::Off).size().x
            })
            .fold(0.0, f32::max)
    }

    /// How to wrap, and the width to report when it is not the galley's.
    ///
    /// A galley wrapped at the widest word can come out a fraction wider than
    /// that word, so the min-content width is reported exactly.
    fn plan(&self, fonts: &mut FontsView<'_>, input: &LayoutInput) -> (Wrap, Option<f32>) {
        match (input.known_dimensions.width, input.available_space.width) {
            (Some(width), _) | (None, AvailableSpace::Definite(width)) => (Wrap::At(width), None),
            (None, AvailableSpace::MinContent) => {
                let width = self.min_content_width(fonts);
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
    leaf: &TextLeaf,
    input: &LayoutInput,
) -> LayoutOutput {
    let (wrap, exact_width) = leaf.plan(fonts, input);
    let galley = leaf.galley(fonts, wrap);
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

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use taffy::{NodeId, Style, TaffyTree};

    /// A context whose fonts are ready for measurement.
    pub fn fonts_ready_context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |_| {})
            .drop_without_applying_deltas();
        ctx
    }

    pub fn body_font() -> FontId {
        FontId::proportional(14.0)
    }

    pub fn leaf(text: &str) -> TextLeaf {
        TextLeaf {
            text: text.to_owned(),
            font: body_font(),
        }
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
        leaf: &TextLeaf,
        known: Size<Option<f32>>,
        available: Size<AvailableSpace>,
    ) -> LayoutOutput {
        ctx.fonts_mut(|fonts| measure_text(fonts, leaf, &input(known, available)))
    }

    pub fn compute(
        ctx: &egui::Context,
        tree: &mut TaffyTree<TextLeaf>,
        root: NodeId,
        available: Size<AvailableSpace>,
    ) {
        tree.compute_layout_with_measure(root, available, |input, _, leaf, _: &Style| match leaf {
            Some(leaf) => ctx.fonts_mut(|fonts| measure_text(fonts, leaf, &input)),
            None => LayoutOutput::from_outer_size(Size::ZERO),
        })
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use taffy::prelude::TaffyMaxContent;
    use taffy::{
        AlignItems, Display, JustifyContent, Rect, Style, TaffyTree, style_helpers::length,
    };

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
            &leaf(text),
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
            &leaf(text),
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
        let out = measure(&ctx, &leaf("anything at all"), known, MAX);
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
        let out = measure(&ctx, &leaf("Hello"), UNKNOWN, MAX);
        let baseline = out.baselines.first.expect("text has a baseline");
        assert!(baseline > 0.0 && baseline < out.size.height);
    }

    #[test]
    fn empty_text_has_no_baseline() {
        let ctx = fonts_ready_context();
        assert_eq!(measure(&ctx, &leaf(""), UNKNOWN, MAX).baselines.first, None);
    }

    #[test]
    fn hello_is_centred_in_a_flex_container() {
        let ctx = fonts_ready_context();
        let measured = measure(&ctx, &leaf("Hello"), UNKNOWN, MAX).size;
        assert!(measured.width > 0.0 && measured.height > 0.0);

        let mut tree = TaffyTree::<TextLeaf>::new();
        tree.disable_rounding();
        let text = tree
            .new_leaf_with_context(Style::default(), leaf("Hello"))
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    display: Display::Flex,
                    size: Size {
                        width: length(800.0),
                        height: length(600.0),
                    },
                    align_items: AlignItems::CENTER,
                    justify_content: JustifyContent::CENTER,
                    ..Default::default()
                },
                &[text],
            )
            .unwrap();
        compute(&ctx, &mut tree, root, Size::<AvailableSpace>::MAX_CONTENT);

        let layout = tree.layout(text).unwrap();
        assert_eq!(layout.size, measured);
        assert!(measured.width < 800.0 && measured.height < 600.0);
        let rect = Rect {
            left: layout.location.x,
            right: layout.location.x + layout.size.width,
            top: layout.location.y,
            bottom: layout.location.y + layout.size.height,
        };
        assert!((rect.left + rect.right - 800.0).abs() < 1e-3);
        assert!((rect.top + rect.bottom - 600.0).abs() < 1e-3);
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
