//! TODO(prose): the spinner, a loading indicator after shadcn/ui's Spinner:
//! lucide's `loader-circle`, turning.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui_extras::spinner::SpinnerExt;
//!
//! fn view(b: &mut Builder<'_, '_, ()>) {
//!     b.spinner("loading").add();
//!     b.spinner("saving").named("Saving").add();
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

use eggshell_ui::contract::{Keyed, Size};
use eggshell_ui::{Angle, Builder, Icon, Key, NodeUi as _, Style, Tw, tw};
use egui::{Context, ImageSource, Role, Theme, WidgetInfo, include_image};

/// Lucide's `loader-circle` (ISC, `LUCIDE-LICENSE` beside it), the icon of
/// shadcn's `Loader2Icon`.
const LOADER: ImageSource<'static> = include_image!("loader-circle.svg");

/// TODO(prose): the look of [`spinner`](SpinnerExt::spinner)s.
///
/// ```
/// use std::time::Duration;
/// use eggshell_ui_extras::spinner::SpinnerStyle;
///
/// let ctx = egui::Context::default();
/// let mut style = (*SpinnerStyle::of(&ctx)).clone();
/// style.period = Duration::from_secs(2);
/// style.install_for(&ctx, ctx.theme());
/// assert_eq!(SpinnerStyle::of(&ctx).period, Duration::from_secs(2));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SpinnerStyle {
    /// Side of the spinner per control size, in points.
    pub sizes: Keyed<Size, f32>,
    /// How long one turn takes.
    pub period: Duration,
}

impl SpinnerStyle {
    /// TODO(prose): a spinner as large as a button's icon at each of the
    /// style's sizes, turning once a second, as Tailwind's `animate-spin`.
    pub fn from_style(style: &Style) -> Self {
        let buttons = &style.button.sizes;
        let mut sizes = Keyed::new(
            buttons.default_key(),
            buttons.get(buttons.default_key()).glyph,
        );
        for size in buttons.keys() {
            sizes.insert(size, buttons.get(size).glyph);
        }
        Self {
            sizes,
            period: Duration::from_secs(1),
        }
    }

    /// TODO(prose): the spinner style installed for the context's theme,
    /// else [`from_style`](Self::from_style) of its [`Style`].
    pub fn of(ctx: &Context) -> Arc<Self> {
        crate::styles::of(ctx, Self::from_style)
    }

    /// TODO(prose): makes `self` the spinner style of `ctx` while its theme
    /// is `theme`.
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        crate::styles::install_for(self, ctx, theme);
    }
}

/// TODO(prose): adds [`spinner`](Self::spinner) to the builder.
pub trait SpinnerExt<'b, 'f, A> {
    /// TODO(prose): a spinner, drawn in the text colour at the control size
    /// and announced as a `Role::Status` named "Loading". Under reduced
    /// motion it stands still. Finish it with [`add`](SpinnerChain::add).
    fn spinner<'s, K: Key>(&'s mut self, key: K) -> SpinnerChain<'s, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> SpinnerExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn spinner<'s, K: Key>(&'s mut self, key: K) -> SpinnerChain<'s, 'b, 'f, A, K> {
        SpinnerChain {
            b: self,
            key,
            size: None,
            name: None,
            classes: tw(),
        }
    }
}

/// TODO(prose): a spinner. Start one with [`SpinnerExt::spinner`] and finish
/// it with [`add`](Self::add).
///
/// ```html
/// <svg role="status" aria-label="Loading" class="size-4 animate-spin">…loader-circle…</svg>
/// ```
#[must_use = "a spinner is not added until you call `.add()`"]
pub struct SpinnerChain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    size: Option<Size>,
    name: Option<String>,
    classes: Tw,
}

impl<A: 'static, K: Key> SpinnerChain<'_, '_, '_, A, K> {
    /// TODO(prose): draws the spinner at the style's size `size`, in place of
    /// the control size it inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): the accessible name, in place of "Loading", as
    /// `aria-label` is.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): classes merged over the styled ones; the later class
    /// wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// TODO(prose): adds the spinner.
    #[track_caller]
    pub fn add(self) {
        let Self {
            b,
            key,
            size,
            name,
            classes,
        } = self;
        let ctx = b.ctx().clone();
        let style = SpinnerStyle::of(&ctx);
        let size = size.unwrap_or_else(|| b.control_size());
        let side = *crate::styles::entry(&ctx, &style.sizes, size);
        let name = name.unwrap_or_else(|| "Loading".to_owned());
        let classes = tw()
            .size_px(side)
            .shrink_0()
            .sense_hover()
            .focusable_none()
            .merge(classes);
        let icon = Icon::new(LOADER).rotated(turned(&ctx, style.period));
        let _ = b.icon(key, classes, icon).draw(move |_, ui| {
            let enabled = ui.is_enabled();
            ui.node_response()
                .widget_info(|| WidgetInfo::labeled(Role::Status, enabled, &name));
        });
    }
}

/// How far a spinner has turned now, asking for the next frame while it
/// turns: not at all under reduced motion, egui's `animation_time` of zero.
fn turned(ctx: &Context, period: Duration) -> Angle {
    if ctx.global_style().animation_time == 0.0 {
        return Angle::ZERO;
    }
    ctx.request_repaint();
    turn_at(ctx.input(|input| input.time), period)
}

/// How far a spinner turning once a `period` has turned after `time` seconds.
fn turn_at(time: f64, period: Duration) -> Angle {
    let period = period.as_secs_f64();
    if period <= 0.0 {
        return Angle::ZERO;
    }
    let turns = (time / period).rem_euclid(1.0);
    Angle::degrees((turns * 360.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn degrees(angle: Angle) -> f32 {
        f32::from(angle).to_degrees()
    }

    #[test]
    fn a_turn_takes_one_period() {
        let period = Duration::from_millis(1000);
        assert_eq!(degrees(turn_at(0.0, period)), 0.0);
        assert!((degrees(turn_at(0.25, period)) - 90.0).abs() < 1e-3);
        assert!((degrees(turn_at(1.5, period)) - 180.0).abs() < 1e-3);
    }

    #[test]
    fn a_zero_period_stands_still() {
        assert_eq!(degrees(turn_at(12.3, Duration::ZERO)), 0.0);
    }
}
