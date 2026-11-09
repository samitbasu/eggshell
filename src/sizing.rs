//! What a text edit's size is computed from: the egui style with the node's
//! font, and the galleys and padding egui itself would use.
//!
//! The style is the context's global style with the node's font override,
//! which is what the draw phase gives a node's `Ui`. Custom themes that
//! change widget frames are not followed.

use std::sync::Arc;

use egui::epaint::text::TextWrapping;
use egui::{Align, Context, FontId, Galley, Style, TextWrapMode, Ui, Vec2, WidgetText};

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

    /// The sizer for widgets drawn in `ui`, whose style the draw phase has
    /// already given the node's font.
    pub fn for_ui(ui: &'a Ui) -> Self {
        let style = (**ui.style()).clone();
        let font = style
            .override_font_id
            .clone()
            .unwrap_or_else(|| egui::TextStyle::Body.resolve(&style));
        Self {
            ctx: ui.ctx(),
            style,
            font,
        }
    }

    /// The line height of the node's font, as egui's text edit spaces lines.
    pub fn line_height(&self) -> f32 {
        self.ctx.fonts_mut(|fonts| fonts.row_height(&self.font))
            + self.style.spacing.extra_text_line_spacing
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

    /// The padding of inputs: the preset's, which differs from a button's, and
    /// otherwise egui's `button_padding`.
    pub fn input_padding(&self) -> Vec2 {
        crate::style::input_padding(self.ctx).unwrap_or(self.style.spacing.button_padding)
    }
}
