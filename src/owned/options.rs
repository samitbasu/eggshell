//! The options the owned controls share, resolved in one place: a tooltip,
//! a refusal, and the conditional `when` and `when_some`.

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
            Some(reason) => Some(Tooltip::text(reason)),
            None => super::tip(b, trigger, self.tooltip),
        };
        (enablement, tip)
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
