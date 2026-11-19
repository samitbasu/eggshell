//! A widget style per theme, installed in the context like core's
//! [`Style`](eggshell_ui::Style), else derived from it.

use std::sync::Arc;

use eggshell_ui::Style;
use egui::{Context, Id, Theme};

fn id<T: 'static>(theme: Theme) -> Id {
    Id::NULL.with((
        "eggshell-ui-extras-style",
        std::any::type_name::<T>(),
        theme,
    ))
}

/// The `T` installed in `ctx` for its theme, else `derive` of its [`Style`].
pub(crate) fn of<T: Clone + Send + Sync + 'static>(
    ctx: &Context,
    derive: impl FnOnce(&Style) -> T,
) -> Arc<T> {
    ctx.data(|data| data.get_temp::<Arc<T>>(id::<T>(ctx.theme())))
        .unwrap_or_else(|| Arc::new(derive(&Style::of(ctx))))
}

/// Makes `style` the `T` of `ctx` while its theme is `theme`.
pub(crate) fn install_for<T: Clone + Send + Sync + 'static>(style: T, ctx: &Context, theme: Theme) {
    ctx.data_mut(|data| data.insert_temp(id::<T>(theme), Arc::new(style)));
}

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
