//! How notifications look: the card, the stack and the buttons in it.

use crate::{Distance, TextSpec};
use egui::Color32;
use egui::epaint::Shadow;

/// TODO(prose): how a notice and its stack look, after sonner's toast and
/// the shadcn skin of it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
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
}

impl Style {
    /// TODO(prose): the page's popover surface, at sonner's sizes: 356 points
    /// wide, 16 points of padding, 6 between a card's parts, 14 between
    /// cards, 3 on show and 24 points from the window's edges.
    pub(crate) fn from_base(style: &crate::contract::Base<'_>) -> Self {
        let popup = crate::owned::styles::Popover::of_tokens(style.tokens);
        let colors = popup.colors;
        Self {
            fill: colors.fill,
            border: colors.border,
            text: colors.text,
            border_width: popup.border_width,
            radius: popup.radius,
            shadow: Some(popup.shadow),
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
        }
    }
}
