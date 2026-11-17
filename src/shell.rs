//! The route from code that has only an egui context or a `Ui` to the state
//! of the [`LayoutState`](crate::LayoutState) that laid a frame out.
//!
//! Two layouts can share an egui context (a bar and a page), and each keeps
//! its scroll viewports, requests and the like itself, so a request made of
//! one is not applied by, or read from, the other. A [`Shell`] names one.
//!
//! The name is not `Layout`, which is egui's, and a `Ui` already has an
//! inherent `layout()` that an extension trait could not shadow.

use egui::{Context, Id, Ui};

use crate::kept::Handle;

/// One [`LayoutState`](crate::LayoutState)'s state, reached from where only
/// the egui context is at hand: a test, a draw closure, an event handler.
///
/// From [`LayoutState::shell`](crate::LayoutState::shell),
/// [`Builder::shell`](crate::Builder::shell),
/// [`Placed::shell`](crate::Placed::shell) or, in a draw closure,
/// [`ShellExt::try_shell`] on the `Ui`. Cloning is cheap and it outlives the
/// borrow of the layout it came from.
///
/// It reaches a scroll container's viewport and asks it to scroll
/// ([`viewport`](Self::viewport), [`scroll_by`](Self::scroll_by),
/// [`scroll_to_rect`](Self::scroll_to_rect),
/// [`scroll_into_view`](Self::scroll_into_view)), brings a virtual list's row
/// into view ([`scroll_to_row`](Self::scroll_to_row)), and reads a live drag
/// ([`drag_pointer`](Self::drag_pointer), [`drag_grab`](Self::drag_grab)).
#[derive(Clone)]
pub struct Shell {
    pub(crate) ctx: Context,
    pub(crate) kept: Handle,
}

impl Shell {
    /// The egui context the layout is laid out in, for a handler that holds a
    /// `Shell` and needs the context too.
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    pub(crate) fn new(ctx: &Context, kept: &Handle) -> Self {
        Self {
            ctx: ctx.clone(),
            kept: kept.clone(),
        }
    }
}

/// [`Shell`] for the `Ui` of a draw closure.
pub trait ShellExt {
    /// The shell of the layout being drawn, or `None` outside every draw
    /// closure of a [`Placed::draw`](crate::Placed::draw).
    ///
    /// Layouts may nest: a draw closure of one layout may build and draw
    /// another (an egui `Popup` or `Area` with eggshell content, say). While
    /// the inner [`Placed::draw`](crate::Placed::draw) runs, its closures see
    /// the inner layout; once it returns, the outer layout's closures see the
    /// outer one again.
    ///
    /// # Example
    ///
    /// ```
    /// use eggshell_ui::{Builder, LayoutState, Shell, ShellExt, tw};
    ///
    /// let ctx = egui::Context::default();
    /// ctx.run_ui(Default::default(), |ui| {
    ///     assert!(ui.try_shell().is_none());
    ///     let mut layout = LayoutState::new();
    ///     let placed = layout
    ///         .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
    ///             b.div_id("pad", tw(), |_| {}).draw(|_, ui| {
    ///                 let Some(shell): Option<Shell> = ui.try_shell() else {
    ///                     return;
    ///                 };
    ///                 let _ = shell.ctx();
    ///             });
    ///         })
    ///         .layout(ui.max_rect());
    ///     placed.draw(&mut (), ui);
    /// })
    /// .drop_without_applying_deltas();
    /// ```
    fn try_shell(&self) -> Option<Shell>;
}

impl ShellExt for Ui {
    fn try_shell(&self) -> Option<Shell> {
        let key = drawing_key(self.ctx());
        let kept = self.ctx().data(|data| data.get_temp::<Handle>(key))?;
        Some(Shell::new(self.ctx(), &kept))
    }
}

/// The shell for a library widget that has only a `Ui`, or `None`, reported
/// as [`ShellOutsideDraw`](crate::Problem::ShellOutsideDraw), with the step
/// `what` skipped. A library widget drawn outside every `Placed::draw` loses
/// that step instead of panicking.
#[track_caller]
pub(crate) fn shell_or_report(ui: &Ui, what: &'static str) -> Option<Shell> {
    let shell = ui.try_shell();
    if shell.is_none() {
        crate::report::report(ui.ctx(), crate::Problem::ShellOutsideDraw, None, || {
            format!(
                "{what} needs the layout being drawn, and this `Ui` is outside every \
                 `Placed::draw`; skipped"
            )
        });
    }
    shell
}

fn drawing_key(ctx: &Context) -> Id {
    crate::key::viewport_id(ctx, "drawing-shell")
}

/// Names the layout being drawn for the draw closures it runs, which receive
/// only a `Ui`. Drawing is sequential, so the name is the context's for as
/// long as the guard lives and no longer.
///
/// Guards nest: a layout drawn from a draw closure of another enters its own
/// guard, and dropping it puts back the name it replaced, so the outer
/// layout's remaining closures see the outer layout again.
pub(crate) struct Drawing {
    ctx: Context,
    key: Id,
    outer: Option<Handle>,
}

impl Drawing {
    pub(crate) fn enter(ctx: &Context, kept: &Handle) -> Self {
        let key = drawing_key(ctx);
        let outer = ctx.data_mut(|data| {
            let outer = data.get_temp::<Handle>(key);
            data.insert_temp(key, kept.clone());
            outer
        });
        Self {
            ctx: ctx.clone(),
            key,
            outer,
        }
    }
}

impl Drop for Drawing {
    fn drop(&mut self) {
        let key = self.key;
        let outer = self.outer.take();
        self.ctx.data_mut(|data| match outer {
            Some(outer) => {
                data.insert_temp(key, outer);
            }
            None => {
                data.remove::<Handle>(key);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_widget_step_outside_every_draw_is_reported_and_skipped() {
        let ctx = Context::default();
        let mut shells = Vec::new();
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                shells.push(shell_or_report(ui, "a test step").is_some());
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(shells, [false, false]);
        let reports = crate::report::reports(&ctx);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert_eq!(reports[0].problem, crate::Problem::ShellOutsideDraw);
        assert_eq!(reports[0].count, 2);
        assert!(reports[0].message.contains("a test step"), "{reports:?}");
    }
}
