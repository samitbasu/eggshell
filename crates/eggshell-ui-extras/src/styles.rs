//! What the widgets' styles share.

use egui::{Context, Id};

#[cfg(any(feature = "badge", feature = "spinner"))]
/// The entry of `keyed` under `key`, reporting a key the style does not
/// define as core's own widgets do.
#[track_caller]
pub(crate) fn entry<'k, K: Ord + Copy + std::fmt::Debug, T>(
    ctx: &Context,
    keyed: &'k eggshell_ui::contract::Keyed<K, T>,
    key: K,
) -> &'k T {
    if !keyed.keys().any(|defined| defined == key) {
        eggshell_ui::report(
            ctx,
            eggshell_ui::Problem::MissingStyleEntry,
            Some(Id::NULL.with(("style-entry", format!("{key:?}")))),
            || format!("the style defines no {key:?}; drawing its default entry"),
        );
    }
    keyed.get(key)
}

#[cfg(any(feature = "command", feature = "markdown"))]
/// The border colour `style`'s sheet gives a text field at rest.
pub(crate) fn input_border(style: &eggshell_ui::Style) -> egui::Color32 {
    style
        .sheet
        .resolve(eggshell_ui::owned::InputPart::Root, &[], &style.tokens)
        .paint_classes()
        .border_colors
        .top
        .unwrap_or(egui::Color32::TRANSPARENT)
}
