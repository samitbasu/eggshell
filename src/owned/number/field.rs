//! A number edited as text, shared by the stepper's field and the slider's
//! readout: one session from the edit's start to Enter, leaving the field,
//! or Escape. Enter and leaving commit what was typed; Escape reverts it.

use egui::{Context, Id, Ui};

use crate::overlay_input::Blur;

use super::{Edit, Edits};

/// How a session ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    Commit,
    Revert,
}

/// The session of the field `editing` names: the text typed so far, while
/// one is open.
#[derive(Clone, Copy, Debug)]
pub(in crate::owned) struct Session {
    typed: Id,
    /// The text the field showed as the session opened.
    opened: Id,
}

impl Session {
    pub(in crate::owned) fn of(editing: Id) -> Self {
        Self {
            typed: editing,
            opened: editing.with("opened"),
        }
    }

    /// The text typed in the open session, if one is open.
    pub(in crate::owned) fn typed(self, ctx: &Context) -> Option<String> {
        ctx.data(|data| data.get_temp::<String>(self.typed))
    }

    pub(in crate::owned) fn is_open(self, ctx: &Context) -> bool {
        ctx.data(|data| data.get_temp::<String>(self.opened))
            .is_some()
    }

    /// Opens a session on a field that showed `opened`: the edit begins.
    pub(in crate::owned) fn open<A>(
        self,
        app: &mut A,
        ctx: &Context,
        opened: &str,
        edits: &Edits<A>,
    ) {
        if self.is_open(ctx) {
            return;
        }
        edits.emit(app, Edit::Began);
        ctx.data_mut(|data| {
            data.insert_temp(self.opened, opened.to_owned());
            data.insert_temp(self.typed, opened.to_owned());
        });
    }

    /// Keeps `text` as what is typed.
    pub(in crate::owned) fn keep(self, ctx: &Context, text: &str) {
        ctx.data_mut(|data| data.insert_temp(self.typed, text.to_owned()));
    }

    /// Forgets what is typed, keeping the session open: the field shows the
    /// value again.
    pub(in crate::owned) fn forget_typed(self, ctx: &Context) {
        ctx.data_mut(|data| data.remove::<String>(self.typed));
    }

    /// Ends the session as the field lost focus: Escape reverts, anything
    /// else commits `text` through `accept` with `set`, when it was edited
    /// and gives a value other than `current`. The edit ends after the value
    /// is set.
    pub(in crate::owned) fn close<A>(
        self,
        app: &mut A,
        ui: &Ui,
        (text, current): (&str, f64),
        accept: impl Fn(&str) -> Option<f64>,
        (edits, set): (&Edits<A>, impl Fn(&mut A, f64)),
    ) {
        let opened = ui.data_mut(|data| {
            data.remove::<String>(self.typed);
            data.remove_temp::<String>(self.opened)
        });
        let Some(opened) = opened else {
            return;
        };
        let ending = match crate::draw::blur_cause(ui) {
            Blur::Escaped => Ending::Revert,
            Blur::Submitted | Blur::Elsewhere => Ending::Commit,
        };
        let committed = match ending {
            Ending::Commit if opened != text => accept(text).filter(|&value| value != current),
            Ending::Commit | Ending::Revert => None,
        };
        if let Some(value) = committed {
            set(app, value);
        }
        edits.emit(app, Edit::Ended);
    }
}
