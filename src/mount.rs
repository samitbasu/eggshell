//! A view over part of the application data, built into the page's layout:
//! [`Builder::mount`](crate::Builder::mount).
//!
//! The mount's builder declares into the page's frame. Each closure it
//! declares takes the mount's data, so it is wrapped, as it is declared, in
//! one that reaches that data from the page's through the mount's lens. A
//! region or a virtual list declared in a mount is built later, with a
//! builder of the page's data; its body is wrapped to build under the mount
//! again.

use std::cell::RefCell;
use std::rc::Rc;

use egui::Id;

use crate::commands::{Commands, Declared, Table, Within, look_up};
use crate::pipeline::{DismissFn, DrawFn, FrameCore, Sink, Twice};
use crate::region::RegionDecl;
use crate::tooltip::{BuildBuilt, build_built};
use crate::window::Hooks;

/// Reaches a mount's data `S` in the data `R` around it.
pub(crate) type Lens<R, S> = Rc<dyn Fn(&mut R) -> &mut S>;

/// What a mount's builder needs, wherever in the frame it is built: its lens,
/// where it is, and what it declared that its own builders look up.
pub(crate) struct Mount<'f, R, S> {
    lens: Lens<R, S>,
    within: Within,
    declared: Rc<RefCell<Declarations<'f, S>>>,
}

impl<R, S> Clone for Mount<'_, R, S> {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            within: self.within,
            declared: self.declared.clone(),
        }
    }
}

/// What a mount declared that its builders read back: its command tables,
/// by root, and the data their enablement is read from.
struct Declarations<'f, S> {
    tables: Vec<(usize, Commands<S>)>,
    app: Option<&'f S>,
}

impl<'f, R: 'static, S: 'static> Mount<'f, R, S> {
    pub(crate) fn new(lens: Lens<R, S>, within: Within) -> Self {
        Self {
            lens,
            within,
            declared: Rc::new(RefCell::new(Declarations {
                tables: Vec::new(),
                app: None,
            })),
        }
    }

    pub(crate) fn lens(&self) -> Lens<R, S> {
        self.lens.clone()
    }

    fn lift(&self, then: DismissFn<S>) -> DismissFn<R> {
        let lens = self.lens();
        Box::new(move |app| then(lens(app)))
    }

    fn lift_table(&self, table: Table<S>) -> Table<R> {
        let lens = self.lens();
        Table::Lifted(match table {
            Table::Own(table) => Box::new(move |ctx, shell, typing, app| {
                crate::commands::dispatch(ctx, shell, typing, &table, lens(app));
            }),
            Table::Lifted(dispatch) => {
                Box::new(move |ctx, shell, typing, app| dispatch(ctx, shell, typing, lens(app)))
            }
        })
    }
}

/// A mount's builder's frame: the frame around it, seen through the lens.
pub(crate) struct Lensed<'s, 'f, R, S> {
    outer: &'s mut dyn Sink<'f, R>,
    mount: Mount<'f, R, S>,
}

impl<'s, 'f, R, S> Lensed<'s, 'f, R, S> {
    pub(crate) fn new(outer: &'s mut dyn Sink<'f, R>, mount: Mount<'f, R, S>) -> Self {
        Self { outer, mount }
    }
}

impl<'f, R, S> std::ops::Deref for Lensed<'_, 'f, R, S> {
    type Target = FrameCore<'f>;

    fn deref(&self) -> &FrameCore<'f> {
        self.outer
    }
}

impl<'f, R, S> std::ops::DerefMut for Lensed<'_, 'f, R, S> {
    fn deref_mut(&mut self) -> &mut FrameCore<'f> {
        self.outer
    }
}

impl<'f, R: 'static, S: 'static> Sink<'f, S> for Lensed<'_, 'f, R, S> {
    fn attach_draw(&mut self, slot: Option<usize>, draw: DrawFn<S>) -> usize {
        let lens = self.mount.lens();
        self.outer
            .attach_draw(slot, Box::new(move |app, ui| draw(lens(app), ui)))
    }

    fn add_drop(&mut self, decl: crate::dnd::DropDecl<S>) -> usize {
        self.outer.add_drop(decl.lift(self.mount.lens()))
    }

    fn add_end(&mut self, decl: crate::dnd::EndDecl<S>) {
        self.outer.add_end(decl.lift(self.mount.lens()));
    }

    fn add_region(&mut self, region: RegionDecl<'f, S>) {
        self.outer.add_region(region.lift(self.mount.clone()));
    }

    fn on_dismiss(&mut self, overlay: Id, then: DismissFn<S>) {
        self.outer.on_dismiss(overlay, self.mount.lift(then));
    }

    fn on_exited(&mut self, then: DismissFn<S>) {
        self.outer.on_exited(self.mount.lift(then));
    }

    fn window_hooks(&mut self, window: Id, hooks: Hooks<S>) {
        self.outer
            .window_hooks(window, hooks.lift(self.mount.lens()));
    }

    fn build_built(&self) -> Option<BuildBuilt<S>> {
        self.outer
            .build_built()
            .map(|_| build_built::<S> as BuildBuilt<S>)
    }

    /// The tooltips are declared with the page's builder, so it is the page's
    /// frame that must know how to build their content.
    fn set_build_built(&mut self, _: BuildBuilt<S>) {
        self.outer.set_build_built(build_built::<R>);
    }

    fn declare_commands(&mut self, table: &Commands<S>, app: &'f S) -> Result<(), Twice> {
        let root = self.outer.current;
        let Ok(mut declared) = self.mount.declared.try_borrow_mut() else {
            return Ok(());
        };
        if declared.tables.iter().any(|(at, _)| *at == root) {
            return Err(Twice);
        }
        declared.app = Some(app);
        declared.tables.push((root, table.clone()));
        drop(declared);
        let within = Some(self.mount.within);
        let table = self.mount.lift_table(Table::Own(table.clone()));
        self.outer.dispatch(Declared {
            root,
            within,
            table,
        });
        Ok(())
    }

    fn command(&self, id: Id) -> Option<(crate::Command<S>, bool)> {
        let declared = self.mount.declared.try_borrow().ok()?;
        look_up(self, &declared.tables, declared.app?, id)
    }

    fn dispatch(&mut self, declared: Declared<S>) {
        let Declared {
            root,
            within,
            table,
        } = declared;
        let table = self.mount.lift_table(table);
        self.outer.dispatch(Declared {
            root,
            within,
            table,
        });
    }

    fn depth(&self) -> usize {
        self.mount.within.depth + 1
    }
}
