use core::time::Duration;

use egui::{Context, InputOptions};

/// TODO(prose)
///
/// ```
/// use std::time::Duration;
///
/// assert_eq!(eggshell_ui::DOUBLE_CLICK, Duration::from_millis(500));
/// ```
pub const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// egui's own window splits a relaxed double click, one the desktop counts,
/// into two clicks. An app that set a window of its own keeps it.
pub(crate) fn widen_double_click(ctx: &Context) {
    let egui_default = InputOptions::default().max_double_click_delay;
    let unset = ctx.options(|options| options.input_options.max_double_click_delay == egui_default);
    if unset {
        ctx.options_mut(|options| {
            options.input_options.max_double_click_delay = DOUBLE_CLICK.as_secs_f64();
        });
    }
}
