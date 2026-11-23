//! The options the owned controls share, resolved in one place: a tooltip,
//! a refusal, validity, and the conditional `when` and `when_some`.

use std::borrow::Cow;
use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{Galley, Id, RichText, WidgetText};

use super::{Content, Enablement, Tip, TipContent, Tooltip};
use crate::Builder;

/// TODO(prose): text for a tooltip or a refusal, or `None` for none.
///
/// Implemented for egui's text types and an [`Option`] of each, so a call
/// site can pass what it has:
///
/// ```
/// use eggshell_ui::Builder;
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
