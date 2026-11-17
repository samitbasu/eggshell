//! TODO(prose)
//!
//! ```
//! use eggshell_ui::{Icon, icons};
//! let close = Icon::new(icons::CLOSE);
//! ```

use std::sync::Arc;

use egui::{Context, ImageSource, include_image};
use egui_extras::loaders::svg_loader::SvgLoader;

pub(crate) fn install(ctx: &Context) {
    if !ctx.is_loader_installed(SvgLoader::ID) {
        ctx.add_image_loader(Arc::new(SvgLoader::default()));
    }
}

/// TODO(prose): lucide `x`.
pub const CLOSE: ImageSource<'static> = include_image!("lucide/x.svg");
/// TODO(prose): lucide `check`.
pub const CHECK: ImageSource<'static> = include_image!("lucide/check.svg");
/// TODO(prose): lucide `chevron-down`.
pub const CHEVRON_DOWN: ImageSource<'static> = include_image!("lucide/chevron-down.svg");
/// TODO(prose): lucide `chevron-up`.
pub const CHEVRON_UP: ImageSource<'static> = include_image!("lucide/chevron-up.svg");
/// TODO(prose): lucide `chevron-left`.
pub const CHEVRON_LEFT: ImageSource<'static> = include_image!("lucide/chevron-left.svg");
/// TODO(prose): lucide `chevron-right`.
pub const CHEVRON_RIGHT: ImageSource<'static> = include_image!("lucide/chevron-right.svg");
/// TODO(prose): lucide `plus`.
pub const PLUS: ImageSource<'static> = include_image!("lucide/plus.svg");
/// TODO(prose): lucide `minus`.
pub const MINUS: ImageSource<'static> = include_image!("lucide/minus.svg");
/// TODO(prose): lucide `maximize-2`.
pub const MAXIMIZE: ImageSource<'static> = include_image!("lucide/maximize-2.svg");
/// TODO(prose): lucide `external-link`, the pop-out.
pub const POP_OUT: ImageSource<'static> = include_image!("lucide/external-link.svg");
/// TODO(prose): lucide `pause`.
pub const PAUSE: ImageSource<'static> = include_image!("lucide/pause.svg");
/// TODO(prose): lucide `play`.
pub const PLAY: ImageSource<'static> = include_image!("lucide/play.svg");
/// TODO(prose): lucide `triangle-exclamation-point` (formerly `triangle-alert`).
pub const WARNING: ImageSource<'static> = include_image!("lucide/triangle-exclamation-point.svg");
/// TODO(prose): lucide `search`.
pub const SEARCH: ImageSource<'static> = include_image!("lucide/search.svg");
/// TODO(prose): lucide `circle-question-mark` (formerly `circle-help`).
pub const HELP: ImageSource<'static> = include_image!("lucide/circle-question-mark.svg");
/// TODO(prose): lucide `settings`, the gear.
pub const GEAR: ImageSource<'static> = include_image!("lucide/settings.svg");
