//! The sizes egui gives stock widgets, computed without running them.
//!
//! egui 0.37 builds `Button`, `Checkbox`, `RadioButton`, a `ComboBox` header
//! and a `TextEdit` out of one atom layout: a frame, a gap, and a row of
//! fixed-size pieces and text galleys, of which the first text shrinks to
//! the width on offer. [`Sizer::lay_out`] repeats that arithmetic from the
//! egui [`Style`], taking galleys and frames from egui itself. Each widget
//! states which pieces and style values it feeds in, and `tests/sizing.rs`
//! asserts the result equals what egui allocates, so an egui upgrade that
//! changes a widget fails the build instead of drifting.
//!
//! The style is the context's global style with the node's font override,
//! which is what the draw phase gives a node's `Ui`. Custom themes that
//! change widget frames are not followed.

use std::sync::Arc;

use egui::epaint::text::TextWrapping;
use egui::{
    Align, Context, FontId, Frame, Galley, NumExt as _, Style, TextWrapMode, Vec2, WidgetText,
};

use crate::TextStyle;

/// A width a widget will be given, for wrapping its text and growing its
/// height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Width(f32);

impl Width {
    /// A width of `points`.
    pub fn new(points: f32) -> Self {
        Self(points)
    }
}

impl From<Width> for f32 {
    fn from(width: Width) -> f32 {
        width.0
    }
}

/// One piece of an atom layout.
pub(crate) enum Piece {
    Fixed(Vec2),
    Text(WidgetText),
}

/// A piece after sizing.
pub(crate) enum Measured {
    Fixed(Vec2),
    Text(Arc<Galley>),
}

impl Measured {
    pub fn size(&self) -> Vec2 {
        match self {
            Self::Fixed(size) => *size,
            Self::Text(galley) => galley.size(),
        }
    }
}

/// What egui's `ContainerAtom::measure` is given.
pub(crate) struct Row {
    pub pieces: Vec<Piece>,
    pub gap: f32,
    pub frame: Frame,
    pub min_size: Vec2,
    pub wrap: TextWrapMode,
}

/// A measured row.
#[allow(
    dead_code,
    reason = "painted by the switch, which the next commit adds"
)]
pub(crate) struct Laid {
    pub pieces: Vec<Measured>,
    pub outer: Vec2,
}

/// The egui style and fonts a node's widgets are drawn with.
pub(crate) struct Sizer<'a> {
    pub ctx: &'a Context,
    pub style: Style,
    pub font: FontId,
}

impl<'a> Sizer<'a> {
    pub fn new(ctx: &'a Context, text: &TextStyle) -> Self {
        let font = text.spec.font_id();
        let mut style = (*ctx.global_style()).clone();
        style.override_font_id = Some(font.clone());
        Self { ctx, style, font }
    }

    pub fn row_height(&self, font: &FontId) -> f32 {
        self.ctx.fonts_mut(|fonts| fonts.row_height(font))
    }

    /// The line height of the node's font, as egui's text edit spaces lines.
    pub fn line_height(&self) -> f32 {
        self.row_height(&self.font) + self.style.spacing.extra_text_line_spacing
    }

    /// A text galley as an atom layout makes it.
    pub fn galley(&self, text: WidgetText, wrap: TextWrapMode, width: f32) -> Arc<Galley> {
        text.into_galley_impl(
            self.ctx,
            &self.style,
            TextWrapping::from_wrap_mode_and_width(wrap, width),
            egui::FontSelection::Default,
            Align::Center,
        )
    }

    /// The frame of a `Button` in its inactive state, with the stroke and
    /// expansion folded into the margin so that they never shift the layout.
    pub fn button_frame(&self) -> Frame {
        if !self.style.visuals.button_frame {
            return Frame::new();
        }
        let visuals = self.style.visuals.widgets.inactive;
        Frame {
            fill: visuals.weak_bg_fill,
            corner_radius: visuals.corner_radius,
            inner_margin: self.style.spacing.button_padding.into(),
            ..Frame::default()
        }
        .apply_stroke_and_expansion_without_layout_shift(visuals.bg_stroke, visuals.expansion)
    }

    /// `ContainerAtom::measure`, for a layout allocated in a `Ui` that has
    /// `width` (or any width) left.
    pub fn lay_out(&self, row: Row, width: Option<Width>) -> Laid {
        let Row {
            pieces,
            gap,
            frame,
            min_size,
            wrap,
        } = row;
        let margin = frame.total_margin().sum();
        let available =
            Vec2::new(width.map_or(f32::INFINITY, f32::from), f32::INFINITY).at_least(min_size);
        let inner_available = available - margin;

        let shrinking = if wrap == TextWrapMode::Extend {
            None
        } else {
            pieces
                .iter()
                .position(|piece| matches!(piece, Piece::Text(_)))
        };

        let mut main = gap * pieces.len().saturating_sub(1) as f32;
        let mut cross = 0.0f32;
        let mut measured: Vec<Option<Measured>> = Vec::new();
        let mut shrink = None;
        for (index, piece) in pieces.into_iter().enumerate() {
            if Some(index) == shrinking {
                shrink = Some(piece);
                measured.push(None);
                continue;
            }
            let piece = match piece {
                Piece::Fixed(size) => Measured::Fixed(size),
                Piece::Text(text) => {
                    Measured::Text(self.galley(text, TextWrapMode::Extend, inner_available.x))
                }
            };
            main += piece.size().x;
            cross = cross.max(piece.size().y);
            measured.push(Some(piece));
        }
        if let (Some(index), Some(Piece::Text(text))) = (shrinking, shrink) {
            let galley = self.galley(text, wrap, inner_available.x - main);
            main += galley.size().x;
            cross = cross.max(galley.size().y);
            measured[index] = Some(Measured::Text(galley));
        }

        Laid {
            pieces: measured.into_iter().flatten().collect(),
            outer: (Vec2::new(main, cross) + margin).at_least(min_size),
        }
    }
}
