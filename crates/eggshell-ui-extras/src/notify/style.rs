//! How notifications look: the card, the stack and the buttons in it.

use std::sync::Arc;

use eggshell_ui::contract::{Size, Variant};
use eggshell_ui::{Distance, Style, TextSpec};
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
    /// TODO(prose): the variant of an action's button.
    pub action_variant: Variant,
    /// TODO(prose): the size of an action's button.
    pub action_size: Size,
    /// TODO(prose): the variant of the × button.
    pub close_variant: Variant,
    /// TODO(prose): the size of the × button.
    pub close_size: Size,
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
            action_variant: Variant::DEFAULT,
            action_size: Size::DEFAULT,
            close_variant: Variant::DEFAULT,
            close_size: Size::DEFAULT,
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
