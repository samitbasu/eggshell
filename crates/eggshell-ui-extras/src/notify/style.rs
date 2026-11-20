//! How notifications look: the card, the stack and the buttons in it.

use std::sync::Arc;

use eggshell_ui::{Distance, Style, TextSpec, Weight};
use egui::epaint::Shadow;
use egui::{Color32, Context, Theme};

/// TODO(prose): how a notice and its stack look, after sonner's toast and
/// the shadcn skin of it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct NotifyStyle {
    /// TODO(prose): the card's fill.
    pub fill: Color32,
    /// TODO(prose): the card's border.
    pub border: Color32,
    /// TODO(prose): the card's text.
    pub text: Color32,
    /// TODO(prose): how thick the border is.
    pub border_width: f32,
    /// TODO(prose): the card's corner radius.
    pub radius: u8,
    /// TODO(prose): the card's shadow, if it has one.
    pub shadow: Option<Shadow>,
    /// TODO(prose): the text of a notice's sentence.
    pub role: TextSpec,
    /// TODO(prose): the width of a card.
    pub width: f32,
    /// TODO(prose): the padding inside a card.
    pub padding: f32,
    /// TODO(prose): the gap between the parts of a card.
    pub gap: f32,
    /// TODO(prose): the gap between the cards of a stack.
    pub stack_gap: f32,
    /// TODO(prose): how many of the newest notices of a stack show at once.
    pub visible: usize,
    /// TODO(prose): how far a stack sits from the window's edges unless set.
    pub inset: Distance,
    /// TODO(prose): an action's button.
    pub action: ActionStyle,
    /// TODO(prose): the × on a card's corner.
    pub close: CloseStyle,
}

/// TODO(prose): an action's button, a filled box in the card's colours
/// swapped.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ActionStyle {
    /// TODO(prose): the button's fill.
    pub fill: Color32,
    /// TODO(prose): the button's text.
    pub text: Color32,
    /// TODO(prose): the button's corner radius.
    pub radius: u8,
    /// TODO(prose): the button's height.
    pub height: f32,
    /// TODO(prose): the button's padding at its sides.
    pub padding_x: f32,
    /// TODO(prose): the button's text role.
    pub role: TextSpec,
}

/// TODO(prose): the × button, a small circle half out over a corner of its
/// card.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CloseStyle {
    /// TODO(prose): the circle's diameter.
    pub size: f32,
    /// TODO(prose): how far the circle sits out past the card's corner.
    pub outset: f32,
    /// TODO(prose): the side of the ×.
    pub glyph: f32,
    /// TODO(prose): the circle's fill.
    pub fill: Color32,
    /// TODO(prose): the circle's border.
    pub border: Color32,
    /// TODO(prose): the ×'s colour.
    pub text: Color32,
    /// TODO(prose): the fill under the pointer.
    pub hover_fill: Color32,
    /// TODO(prose): the border under the pointer.
    pub hover_border: Color32,
}

impl NotifyStyle {
    /// TODO(prose): the page's popover surface, at sonner's sizes: 356 points
    /// wide, 16 points of padding, 6 between a card's parts, 14 between
    /// cards, 3 on show and 24 points from the window's edges.
    pub fn from_style(style: &Style) -> Self {
        let popup = &style.menu.popup;
        let colors = popup.paint.states.rest;
        Self {
            fill: colors.fill,
            border: colors.border,
            text: colors.text,
            border_width: popup.border_width,
            radius: style.menu.radius,
            shadow: popup.paint.shadow,
            role: style
                .menu
                .text
                .clone()
                .unwrap_or_else(|| style.text.clone()),
            width: 356.0,
            padding: 16.0,
            gap: 6.0,
            stack_gap: 14.0,
            visible: 3,
            inset: Distance::new(24.0),
            action: ActionStyle {
                fill: colors.text,
                text: colors.fill,
                radius: 4,
                height: 24.0,
                padding_x: 8.0,
                role: TextSpec {
                    size: 12.0,
                    line_height: 16.0,
                    weight: Weight::Medium,
                    ..style.text.clone()
                },
            },
            close: CloseStyle {
                size: 20.0,
                outset: 7.0,
                glyph: 12.0,
                fill: colors.fill,
                border: colors.border,
                text: colors.text,
                hover_fill: colors.fill,
                hover_border: colors.border,
            },
        }
    }

    /// TODO(prose): the notify style installed for the context's theme, else
    /// [`from_style`](Self::from_style) of its [`Style`].
    pub fn of(ctx: &Context) -> Arc<Self> {
        crate::styles::of(ctx, Self::from_style)
    }

    /// TODO(prose): makes `self` the notify style of `ctx` while its theme is
    /// `theme`.
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        crate::styles::install_for(self, ctx, theme);
    }
}
