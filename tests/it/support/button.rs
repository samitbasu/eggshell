//! What shadcn's Button classes say each variant looks like, read off the
//! pinned source (`registry/styles/style-vega.css` 145-203; Lyra's colours are
//! the same): the oracle the generated sheets are tested against, written
//! apart from the generator so a mapping error cannot agree with itself.

use eggshell_ui::contract::Variant;
use eggshell_ui_shadcn::{Tokens, variant};
use egui::{Color32, Theme};

/// A variant's colours at rest and under the pointer. Text a variant does
/// not colour is the page's foreground, which it inherits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub fill: Color32,
    pub text: Color32,
    pub border: Color32,
    pub hover_fill: Color32,
    pub hover_text: Color32,
}

/// `variant`'s look in `theme`; a button that names no variant is
/// `secondary`.
pub fn look(theme: Theme, name: Variant) -> Look {
    let k = Tokens::for_theme(theme);
    let dark = theme == Theme::Dark;
    let clear = Color32::TRANSPARENT;
    let flat = |fill, text, hover_fill, hover_text| Look {
        fill,
        text,
        border: clear,
        hover_fill,
        hover_text,
    };
    match name {
        n if n == variant::PRIMARY => flat(
            k.primary,
            k.primary_foreground,
            k.primary.gamma_multiply(0.8),
            k.primary_foreground,
        ),
        n if n == variant::OUTLINE => Look {
            fill: if dark {
                k.input.gamma_multiply(0.3)
            } else {
                k.background
            },
            text: k.foreground,
            border: if dark { k.input } else { k.border },
            hover_fill: if dark {
                k.input.gamma_multiply(0.5)
            } else {
                k.muted
            },
            hover_text: k.foreground,
        },
        n if n == variant::GHOST => flat(
            clear,
            k.foreground,
            if dark {
                k.muted.gamma_multiply(0.5)
            } else {
                k.muted
            },
            k.foreground,
        ),
        n if n == variant::DESTRUCTIVE => flat(
            k.destructive.gamma_multiply(if dark { 0.2 } else { 0.1 }),
            k.destructive,
            k.destructive.gamma_multiply(if dark { 0.3 } else { 0.2 }),
            k.destructive,
        ),
        n if n == variant::LINK => flat(clear, k.primary, clear, k.primary),
        _ => {
            let hover = k.secondary.lerp_to_gamma(k.foreground, 0.05);
            flat(
                k.secondary,
                k.secondary_foreground,
                hover,
                k.secondary_foreground,
            )
        }
    }
}

/// The classes `style`'s sheet gives a button of `variant` and `size`.
pub fn classes(
    style: &eggshell_ui::Style,
    variant: Variant,
    size: eggshell_ui::contract::Size,
) -> eggshell_ui::Tw {
    style.sheet.resolve(
        eggshell_ui::owned::ButtonPart::Root,
        &[
            eggshell_ui::sheet::Key::Variant(variant),
            eggshell_ui::sheet::Key::Size(size),
        ],
        &style.tokens,
    )
}

/// The height `style`'s sheet gives a button of `size`.
pub fn height(style: &eggshell_ui::Style, size: eggshell_ui::contract::Size) -> f32 {
    classes(style, Variant::DEFAULT, size)
        .style_at(style.tokens.spacing)
        .size
        .height
        .into_option()
        .expect("a button's height is a length")
}
