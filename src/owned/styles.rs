//! What the widgets' styles share.

use egui::{Context, Id};

#[cfg(any(feature = "badge", feature = "spinner"))]
/// The entry of `keyed` under `key`, reporting a key the style does not
/// define as core's own widgets do.
#[track_caller]
pub(crate) fn entry<'k, K: Ord + Copy + std::fmt::Debug, T>(
    ctx: &Context,
    keyed: &'k crate::contract::Keyed<K, T>,
    key: K,
) -> &'k T {
    if !keyed.keys().any(|defined| defined == key) {
        crate::report(
            ctx,
            crate::Problem::MissingStyleEntry,
            Some(Id::NULL.with(("style-entry", format!("{key:?}")))),
            || format!("the style defines no {key:?}; drawing its default entry"),
        );
    }
    keyed.get(key)
}

#[cfg(any(feature = "command", feature = "markdown"))]
/// The border colour `style`'s sheet gives a text field at rest.
pub(crate) fn input_border(style: &crate::Style) -> egui::Color32 {
    style
        .sheet
        .resolve(crate::owned::InputPart::Root, &[], &style.tokens)
        .paint_classes()
        .border_colors
        .top
        .unwrap_or(egui::Color32::TRANSPARENT)
}

#[cfg(any(
    feature = "command",
    feature = "dialog",
    feature = "form",
    feature = "hover-card",
    feature = "notify",
    feature = "skeleton"
))]
/// TODO(prose): the box a widget draws as `style`'s menus draw theirs: the popover's colours and shadow from the tokens, a 1 pt border, and the radius of the sheet's menu box.
pub(crate) struct Popover {
    pub colors: crate::contract::Colors,
    pub border_width: f32,
    pub radius: u8,
    pub shadow: egui::Shadow,
}

#[cfg(any(
    feature = "command",
    feature = "dialog",
    feature = "form",
    feature = "hover-card",
    feature = "notify",
    feature = "skeleton"
))]
impl Popover {
    pub(crate) fn of(style: &crate::Style) -> Self {
        let k = &style.tokens;
        Self {
            colors: crate::contract::Colors {
                fill: k.popover,
                border: k.border,
                text: k.popover_foreground,
            },
            border_width: 1.0,
            radius: menu_part(style, crate::owned::MenuPart::Content, &[])
                .paint_classes()
                .radius
                .nw,
            shadow: k.shadow_md,
        }
    }

    /// The box as a [`Look`](crate::contract::Look), the same in every
    /// state.
    #[cfg(feature = "hover-card")]
    pub(crate) fn look(&self) -> crate::contract::Look {
        crate::contract::Look::new(crate::contract::States::still(
            self.colors,
            self.colors.fill,
        ))
        .bordered(self.border_width)
        .shadowed(self.shadow)
    }
}

#[cfg(any(
    feature = "command",
    feature = "dialog",
    feature = "form",
    feature = "hover-card",
    feature = "notify",
    feature = "skeleton"
))]
/// The classes `style`'s sheet gives the menu's `part` under `keys`.
pub(crate) fn menu_part(
    style: &crate::Style,
    part: crate::owned::MenuPart,
    keys: &[crate::sheet::Key],
) -> crate::Tw {
    style.sheet.resolve(part, keys, &style.tokens)
}
