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
/// [`ShellExt::shell`] on the `Ui`. Cloning is cheap and it outlives the
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
    /// The shell of the layout whose draw closure this `Ui` belongs to.
    ///
    /// # Panics
    ///
    /// Outside a draw closure of [`Placed::draw`](crate::Placed::draw).
    ///
    /// # Example
    ///
    /// ```
    /// use eggshell_ui::{Builder, LayoutState, Shell, ShellExt, tw};
    ///
    /// let ctx = egui::Context::default();
    /// ctx.run_ui(Default::default(), |ui| {
    ///     let mut layout = LayoutState::new();
    ///     let placed = layout
    ///         .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
    ///             b.div_id("pad", tw(), |_| {}).draw(|_, ui| {
    ///                 // Not egui's `Ui::layout`: a `Shell`, resolved through the trait.
    ///                 let _: Shell = ui.shell();
    ///             });
    ///         })
    ///         .layout(ui.max_rect());
    ///     placed.draw(&mut (), ui);
    /// })
    /// .drop_without_applying_deltas();
    /// ```
    fn shell(&self) -> Shell;
}

impl ShellExt for Ui {
    fn shell(&self) -> Shell {
        let kept = self
            .ctx()
            .data(|data| data.get_temp::<Handle>(drawing_key()))
            .expect("`Ui::shell` is for the draw closures of a `Placed`");
        Shell::new(self.ctx(), &kept)
    }
}

fn drawing_key() -> Id {
    crate::key::root_id().with("drawing-shell")
}

/// Names the layout being drawn for the draw closures it runs, which receive
/// only a `Ui`. Drawing is sequential, so the name is the context's for as
/// long as the guard lives and no longer.
pub(crate) struct Drawing(Context);

impl Drawing {
    pub(crate) fn enter(ctx: &Context, kept: &Handle) -> Self {
        ctx.data_mut(|data| data.insert_temp(drawing_key(), kept.clone()));
        Self(ctx.clone())
    }
}

impl Drop for Drawing {
    fn drop(&mut self) {
        self.0.data_mut(|data| data.remove::<Handle>(drawing_key()));
    }
}
