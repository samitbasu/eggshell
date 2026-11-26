//! The options the owned controls share, resolved in one place: the
//! [`Options`] value every chain holds, the [`Configure`], [`Interactive`]
//! and [`Respond`] traits that set it, a tooltip, a refusal, validity, and
//! the conditional `when` and `when_some`.
//!
//! The traits' methods are in scope through the prelude:
//!
//! ```
//! use eggshell_ui::prelude::*;
//!
//! struct App { saved: bool }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.button("save", "Save")
//!         .tooltip("Writes the file")
//!         .refused(app.saved.then_some("Nothing to save"))
//!         .on_response(|_, response| {
//!             let _ = response.hovered();
//!         })
//!         .add();
//! }
//! ```

use std::borrow::Cow;
use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{Galley, Id, Response, RichText, WidgetText};

use super::{Enablement, OnResponse, Tip, TipContent};
use crate::focus::{Autofocus, Select};
use crate::owned::tooltip::{Content, Tooltip};
use crate::{Builder, Tw, tw};

/// TODO(prose): text for a tooltip or a refusal, or `None` for none.
///
/// Implemented for egui's text types and an [`Option`] of each, so a call
/// site can pass what it has:
///
/// ```
/// use eggshell_ui::prelude::*;
///
/// struct App { saved: bool }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.button("save", "Save")
///         .refused((!app.saved).then_some("Nothing to save"))
///         .add();
///     b.button("load", "Load").refused("Offline").add();
/// }
/// ```
pub trait IntoTip {
    /// TODO(prose): the text, if there is one.
    fn into_tip(self) -> Option<WidgetText>;
}

macro_rules! into_tip {
    ($( [$($life:lifetime)?] $text:ty ),* $(,)?) => {$(
        impl<$($life)?> IntoTip for $text {
            fn into_tip(self) -> Option<WidgetText> {
                Some(self.into())
            }
        }

        impl<$($life)?> IntoTip for Option<$text> {
            fn into_tip(self) -> Option<WidgetText> {
                self.map(Into::into)
            }
        }
    )*};
}

into_tip!(
    [] WidgetText,
    ['s] &'s str,
    ['s] &'s String,
    [] String,
    ['s] &'s Box<str>,
    [] Box<str>,
    ['s] Cow<'s, str>,
    [] RichText,
    [] Arc<RichText>,
    [] LayoutJob,
    [] Arc<LayoutJob>,
    [] Arc<Galley>,
);

/// The options every owned control's chain holds, which [`Configure`] and
/// [`Interactive`] set: its accessible name, classes, tooltip, refusal,
/// enablement and focus. A chain hands its own to the traits through
/// [`Configure::options`]; nothing else reads it.
#[derive(Clone)]
pub struct Options {
    name: Option<String>,
    classes: Tw,
    gate: Gate,
    enablement: Option<Enablement>,
    autofocus: Autofocus,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

/// A chain's [`Options`] as its terminator builds it: the enablement with
/// the subtree's and the refusal's applied, and the tooltip the refusal's
/// while there is one.
pub(crate) struct Resolved {
    pub(crate) name: Option<String>,
    pub(crate) classes: Tw,
    pub(crate) enablement: Enablement,
    pub(crate) tooltip: Tip,
    pub(crate) refused: Refused,
    pub(crate) autofocus: Autofocus,
}

impl Options {
    pub(crate) fn new() -> Self {
        Self::with_classes(tw())
    }

    /// Options with `classes` and nothing else set.
    pub(crate) fn with_classes(classes: Tw) -> Self {
        Self {
            name: None,
            classes,
            gate: Gate::default(),
            enablement: None,
            autofocus: Autofocus::Never,
        }
    }

    /// `self`, with what `other` has set in place of it and the classes of
    /// both merged, `other`'s winning.
    pub(crate) fn merge(self, other: Self) -> Self {
        Self {
            name: other.name.or(self.name),
            classes: self.classes.merge(other.classes),
            gate: self.gate.merge(other.gate),
            enablement: other.enablement.or(self.enablement),
            autofocus: match other.autofocus {
                Autofocus::Never => self.autofocus,
                asked => asked,
            },
        }
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The enablement the chain was given, enabled unless it was set.
    pub(crate) fn own_enablement(&self) -> Enablement {
        self.enablement.unwrap_or(Enablement::Enabled)
    }

    pub(crate) fn set_enablement(&mut self, enablement: Enablement) {
        self.enablement = Some(enablement);
    }

    /// Disabled while the options refuse the control.
    pub(crate) fn refusal(&self) -> Enablement {
        match self.gate.refused() {
            Refused::Yes => Enablement::Disabled,
            Refused::No => Enablement::Enabled,
        }
    }

    pub(crate) fn set_autofocus(&mut self, autofocus: Autofocus) {
        self.autofocus = autofocus;
    }

    /// The options of the control `trigger` that `b` builds, as
    /// [`Gate::resolve`] resolves them.
    pub(crate) fn resolve<A: 'static>(self, b: &mut Builder<'_, '_, A>, trigger: Id) -> Resolved {
        let own = self.own_enablement();
        let Self {
            name,
            classes,
            gate,
            autofocus,
            ..
        } = self;
        let refused = gate.refused();
        let (enablement, tooltip) = gate.resolve(b, trigger, own);
        Resolved {
            name,
            classes,
            enablement,
            tooltip,
            refused,
            autofocus,
        }
    }
}

/// The options every owned control's chain has: its accessible name, its
/// classes, its tooltip, and the conditional [`when`](Self::when) and
/// [`when_some`](Self::when_some). In scope through
/// [`prelude`](crate::prelude).
///
/// ```
/// use eggshell_ui::prelude::*;
///
/// fn view(b: &mut Builder<'_, '_, ()>, wide: bool, hint: Option<&str>) {
///     b.button("save", "Save")
///         .named("Save the file")
///         .when(wide, |chain| chain.classes(tw().w_full()))
///         .when_some(hint, |chain, hint| chain.tooltip(hint))
///         .add();
/// }
/// ```
pub trait Configure: Sized {
    /// The chain's options, which the other methods set. A chain returns
    /// its own; an application has no reason to call it.
    fn options(&mut self) -> &mut Options;

    /// TODO(prose): the accessible name, as `aria-label` is, in place of
    /// what the control's face or label says.
    fn named(mut self, name: impl Into<String>) -> Self {
        self.options().name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the control's outermost node,
    /// such as its width or placement; the later class wins
    /// ([`Tw::merge`]), and calls merge too. The style's hover and pressed
    /// looks still apply over them in their states: see
    /// [the module docs](crate::owned#classes-and-the-styles-state-looks).
    fn classes(mut self, classes: Tw) -> Self {
        let options = self.options();
        options.classes = std::mem::replace(&mut options.classes, tw()).merge(classes);
        self
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control
    /// is disabled, as an HTML `title` is. Rich text keeps egui's
    /// formatting. It adds to the control's accessible name and does not
    /// replace it. A [refusal](Interactive::refused) shows in its place.
    fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.options().gate.tooltip(text);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside
    /// the tooltip's box, such as text and a [`kbd`](crate::Builder::kbd)
    /// chip. The content takes no input, and its text describes the control
    /// to assistive technology.
    fn tooltip_with(
        mut self,
        build: impl Fn(&mut Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.options().gate.tooltip_with(build);
        self
    }

    /// TODO(prose): the chain through `then`, when `condition` holds.
    fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is
    /// one.
    fn when_some<T>(self, value: Option<T>, then: impl FnOnce(Self, T) -> Self) -> Self {
        when_some(self, value, then)
    }
}

/// The options of an owned control that takes input: whether it does, why
/// not, and when it takes focus. In scope through
/// [`prelude`](crate::prelude).
///
/// ```
/// use eggshell_ui::prelude::*;
///
/// struct App { online: bool }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.button("send", "Send")
///         .refused((!app.online).then_some("Offline"))
///         .focus_on_open()
///         .add();
/// }
/// ```
pub trait Interactive: Configure {
    /// Whether the control takes input.
    fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        self.options().enablement = Some(enablement.into());
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the
    /// last call wins.
    fn refused(mut self, reason: impl IntoTip) -> Self {
        self.options().gate.refuse(reason);
        self
    }

    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    fn focus_on_open(mut self) -> Self {
        self.options().autofocus = Autofocus::OnOverlayOpen;
        self
    }

    /// TODO(prose): takes focus on every frame this is set.
    fn focus_now(mut self) -> Self {
        self.options().autofocus = Autofocus::Now(Select::None);
        self
    }
}

/// The escape hatch of an owned control: its response each frame. In scope
/// through [`prelude`](crate::prelude).
pub trait Respond: Sized {
    /// The application the control's handlers change.
    type App;

    /// The chain's response hook, which [`on_response`](Self::on_response)
    /// sets. A chain returns its own; an application has no reason to call
    /// it.
    fn responses(&mut self) -> &mut OnResponse<Self::App>;

    /// Runs `then` with the response of the control's focusable part each
    /// frame (each option's, for a group): the escape hatch for what the
    /// builder does not offer, such as hover.
    fn on_response(mut self, then: impl Fn(&mut Self::App, &Response) + 'static) -> Self {
        self.responses().set(then);
        self
    }
}

/// Whether a control is refused. A refused group's options show its reason,
/// once, and not their own tooltips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    No,
    Yes,
}

/// A control's tooltip and the reason it is refused, until it is built.
#[derive(Clone, Default)]
pub(crate) struct Gate {
    tooltip: TipContent,
    refusal: Option<WidgetText>,
}

impl Gate {
    pub(crate) fn tooltip(&mut self, text: impl IntoTip) {
        self.tooltip = text.into_tip().map(Content::Text);
    }

    pub(crate) fn tooltip_with(
        &mut self,
        build: impl Fn(&mut Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) {
        self.tooltip = Some(Content::built(build));
    }

    pub(crate) fn refuse(&mut self, reason: impl IntoTip) {
        self.refusal = reason.into_tip();
    }

    /// `self`, with what `other` has set in place of it.
    pub(crate) fn merge(self, other: Self) -> Self {
        Self {
            tooltip: other.tooltip.or(self.tooltip),
            refusal: other.refusal.or(self.refusal),
        }
    }

    pub(crate) fn refused(&self) -> Refused {
        match self.refusal {
            Some(_) => Refused::Yes,
            None => Refused::No,
        }
    }

    /// The enablement of the control `trigger` that `b` builds: `own`, and
    /// disabled inside an inert subtree or while refused. And its tooltip: the
    /// refusal's while refused, which is also its accessible description.
    pub(crate) fn resolve<A: 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        trigger: Id,
        own: Enablement,
    ) -> (Enablement, Tip) {
        let refused = match self.refusal {
            Some(_) => Enablement::Disabled,
            None => Enablement::Enabled,
        };
        let enablement = b.enablement_of(own.and(refused));
        let tip = match self.refusal {
            Some(reason) => Some(Tooltip::refusal(reason)),
            None => super::tip(b, trigger, self.tooltip),
        };
        (enablement, tip)
    }
}

/// Whether a control's value is accepted, as `aria-invalid` says, with the
/// error that says why not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Validity {
    #[default]
    Valid,
    Invalid(String),
}

impl Validity {
    pub(crate) fn of(error: Option<impl Into<String>>) -> Self {
        error.map_or(Self::Valid, |error| Self::Invalid(error.into()))
    }

    /// The ARIA state of an invalid control's look. A disabled or refused
    /// control takes no input, so its disabled look wins and it has none.
    pub(crate) fn aria(&self, enablement: Enablement) -> Option<crate::Aria> {
        match (self, enablement) {
            (Self::Invalid(_), Enablement::Enabled) => Some(crate::Aria::Invalid),
            _ => None,
        }
    }

    /// The key of the field around an invalid control, shadcn's
    /// `data-invalid` on its `Field`, on the same terms as
    /// [`aria`](Self::aria).
    pub(crate) fn field_key(&self, enablement: Enablement) -> Option<crate::sheet::Key> {
        self.aria(enablement)
            .map(|_| crate::sheet::Key::Attr("invalid", "true"))
    }

    /// [`aria`](Self::aria) as the key of a part's look.
    pub(crate) fn key(&self, enablement: Enablement) -> Option<crate::sheet::Key> {
        self.aria(enablement).map(crate::sheet::Key::Aria)
    }

    /// Tells assistive technology that the control `id` is invalid, enabled
    /// or not, described by its error before any other description.
    pub(crate) fn describe(&self, ctx: &egui::Context, id: Id) {
        let Self::Invalid(error) = self else {
            return;
        };
        ctx.accesskit_node_builder(id, |node| {
            node.set_invalid(egui::accesskit::Invalid::True);
            let description = match node.description() {
                Some(other) => format!("{error}. {other}"),
                None => error.clone(),
            };
            node.set_description(description);
        });
    }
}

/// `chain`, through `then` when `condition` holds.
pub(crate) fn when<C>(chain: C, condition: bool, then: impl FnOnce(C) -> C) -> C {
    if condition { then(chain) } else { chain }
}

/// `chain`, through `then` with the value when there is one.
pub(crate) fn when_some<C, T>(chain: C, value: Option<T>, then: impl FnOnce(C, T) -> C) -> C {
    match value {
        Some(value) => then(chain, value),
        None => chain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn some_text_is_a_tip_and_none_is_none() {
        assert_eq!(
            "Offline".into_tip().map(|text| text.text().to_owned()),
            Some("Offline".to_owned())
        );
        assert!(None::<&str>.into_tip().is_none());
        assert!(Some(RichText::new("Busy")).into_tip().is_some());
    }

    #[test]
    fn the_last_refusal_wins_and_none_clears_it() {
        let mut gate = Gate::default();
        gate.refuse("Offline");
        assert_eq!(gate.refused(), Refused::Yes);
        gate.refuse(None::<String>);
        assert_eq!(gate.refused(), Refused::No);
    }

    #[test]
    fn when_and_when_some_apply_only_when_they_hold() {
        assert_eq!(when(1, true, |n| n + 1), 2);
        assert_eq!(when(1, false, |n| n + 1), 1);
        assert_eq!(when_some(1, Some(5), |n, v| n + v), 6);
        assert_eq!(when_some(1, None::<i32>, |n, v| n + v), 1);
    }
}
