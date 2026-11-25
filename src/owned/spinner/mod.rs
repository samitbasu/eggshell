//! TODO(prose): the spinner, a loading indicator after shadcn/ui's Spinner:
//! lucide's `loader-circle`, turning.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::owned::spinner::SpinnerExt;
//!
//! fn view(b: &mut Builder<'_, '_, ()>) {
//!     b.spinner("loading").add();
//!     b.spinner("saving").named("Saving").add();
//! }
//! ```

use std::time::Duration;

use crate::contract::{Keyed, Size};
use crate::{Angle, Builder, Icon, Key, NodeUi as _, Style, Tw, cycle, tw};
use egui::{Context, ImageSource, Role, WidgetInfo, include_image};

/// Lucide's `loader-circle` (ISC, `LUCIDE-LICENSE` beside it), the icon of
/// shadcn's `Loader2Icon`.
const LOADER: ImageSource<'static> = include_image!("loader-circle.svg");

/// TODO(prose): the look of [`spinner`](SpinnerExt::spinner)s.
///
/// ```
/// use std::time::Duration;
/// use eggshell_ui::owned::spinner::SpinnerStyle;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = (*style.extension::<SpinnerStyle>()).clone();
/// look.period = Duration::from_secs(2);
/// style.set_extension(look);
/// assert_eq!(style.extension::<SpinnerStyle>().period, Duration::from_secs(2));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SpinnerStyle {
    /// Side of the spinner per control size, in points.
    pub sizes: Keyed<Size, f32>,
    /// How long one turn takes.
    pub period: Duration,
}

/// TODO(prose): a spinner for the style's sheet, as shadcn's `spinner.tsx`
/// names it. Its turn is the widget's own motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpinnerPart {
    /// TODO(prose): the icon.
    Root,
}

impl crate::sheet::Part for SpinnerPart {
    const SOURCE: &'static str = "spinner.tsx";

    fn slot(self) -> &'static str {
        "spinner"
    }
}

impl crate::contract::Extension for SpinnerStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl SpinnerStyle {
    /// TODO(prose): a spinner as large as a control's icon at each of the
    /// style's sizes, turning once a second, as Tailwind's `animate-spin`.
    pub fn from_style(style: &Style) -> Self {
        let buttons = &style.toggle.sizes;
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
        let style = b.style().extension::<SpinnerStyle>();
        let size = size.unwrap_or_else(|| b.control_size());
        let side = *crate::owned::styles::entry(&ctx, &style.sizes, size);
        let name = name.unwrap_or_else(|| "Loading".to_owned());
        // The size is the widget's, from its size's entry (#348), over the
        // sheet's `size-4`.
        let classes = b.part_over(
            SpinnerPart::Root,
            &[],
            crate::sheet::Defaults(tw()),
            tw().size_px(side)
                .shrink_0()
                .sense_hover()
                .focusable_none()
                .merge(classes),
        );
        let icon = Icon::new(LOADER).rotated(turned(&ctx, style.period));
        let _ = b.icon(key, classes, icon).draw(move |_, ui| {
            let enabled = ui.is_enabled();
            ui.node_response()
                .widget_info(|| WidgetInfo::labeled(Role::Status, enabled, &name));
        });
    }
}

/// How far a spinner has turned now: not at all under reduced motion.
fn turned(ctx: &Context, period: Duration) -> Angle {
    cycle(ctx, period).map_or(Angle::ZERO, |phase| {
        Angle::degrees(f32::from(phase) * 360.0)
    })
}
