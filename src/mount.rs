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
use std::collections::HashMap;
use std::rc::Rc;

use egui::{Id, Pos2};

use crate::NodeUi as _;
use crate::builder::{Builder, NodeHandle, Slot};
use crate::commands::{Commands, Declared, Table, Within, look_up};
use crate::pipeline::{DismissFn, DrawFn, FrameCore, Sink, Twice};
use crate::region::RegionDecl;
use crate::tooltip::{BuildBuilt, build_built};
use crate::window::Hooks;

/// The parts of a mount the library adds around its view.
#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Caption,
    Frame,
}

/// TODO(prose): a mount being built: [`Builder::mount`].
#[must_use = "a mount is not added until you call `.add()`"]
pub struct MountChain<'a, 'f, A, S, L, V> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    classes: crate::Tw,
    data: &'f S,
    lens: L,
    view: V,
    caption: Option<String>,
    name: Option<String>,
}

impl<'a, 'f, A, S, L, V> MountChain<'a, 'f, A, S, L, V>
where
    A: 'static,
    S: 'static,
    L: Fn(&mut A) -> &mut S + 'static,
    V: FnOnce(&mut Builder<'_, 'f, S>, &'f S),
{
    pub(crate) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        classes: crate::Tw,
        (data, lens, view): (&'f S, L, V),
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            classes,
            data,
            lens,
            view,
            caption: None,
            name: None,
        }
    }

    /// TODO(prose): names the mount to assistive technology, where it has no caption.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): shows `caption` above the mount, which is framed.
    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// TODO(prose): adds the mount.
    pub fn add(self) -> NodeHandle<'a, 'f, A> {
        let Self {
            mut b,
            slot,
            classes,
            data,
            lens,
            view,
            caption,
            name,
        } = self;
        let mount = Mount::new(Rc::new(lens), b.mount_within(&slot));
        let selected = b.declare_mount(&slot);
        let style = b.style().clone();
        let look = &style.mount;
        let frame = crate::tw()
            .grow()
            .min_h_0()
            .rounded_px(f32::from(look.radius));
        let frame = match caption {
            None => frame.p(look.bare_padding / 4.0),
            Some(_) => frame
                .border(look.border_width)
                .border_color(look.border)
                .p(look.padding / 4.0),
        };
        let frame = match selected {
            Selected::Yes => frame.ring(style.focus.width, look.selected_ring),
            Selected::No => frame,
        };
        let label = caption.clone().or(name);
        let caption_classes = crate::tw().text(look.caption);
        let caption_classes = match look.caption_text.clone() {
            Some(spec) => caption_classes.role(spec),
            None => caption_classes,
        };
        let outer = crate::tw()
            .flex()
            .flex_col()
            .gap(look.gap / 4.0)
            .merge(classes);
        let node = b
            .div_id_with(&slot, outer, |s| {
                if let Some(caption) = caption {
                    s.span(Part::Caption, caption_classes, caption);
                }
                s.div_id(Part::Frame, frame, |s| s.within(&mount, |b| view(b, data)));
            })
            .draw(move |_, ui| {
                let response = ui.node_response();
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_role(egui::accesskit::Role::Group);
                    if let Some(label) = &label {
                        node.set_label(label.as_str());
                    }
                    if selected == Selected::Yes {
                        node.set_selected(true);
                    }
                });
            })
            .node();
        b.into_handle(node, slot.id)
    }
}

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

    fn command(&self, id: Id) -> Option<crate::commands::Bound<S>> {
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

    fn mount(&self) -> Option<Id> {
        Some(self.mount.within.mount)
    }
}

/// A mount as a frame declared it: its id, and the mount it is built in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MountDecl {
    pub(crate) id: Id,
    pub(crate) parent: Option<Id>,
}

/// Which mount is selected: the one a press or keyboard focus last landed
/// in. A selected mount's ancestors are selected with it.
#[derive(Default)]
pub(crate) struct Selection {
    /// The mounts of the last frame built, each with the one it is in.
    parents: HashMap<Id, Option<Id>>,
    selected: Option<Id>,
    /// The widget that had focus when the selection last settled.
    focus_seen: Option<Id>,
}

impl Selection {
    /// The mounts the frame just built; a selected mount that is gone is
    /// deselected.
    pub(crate) fn set_mounts(&mut self, mounts: &[MountDecl]) {
        self.parents = mounts
            .iter()
            .map(|mount| (mount.id, mount.parent))
            .collect();
        if self
            .selected
            .is_some_and(|selected| !self.parents.contains_key(&selected))
        {
            self.selected = None;
        }
    }

    fn ids(&self) -> Vec<Id> {
        self.parents.keys().copied().collect()
    }

    /// How many mounts `mount` is in.
    fn depth(&self, mount: Id) -> usize {
        let mut at = self.parents.get(&mount).copied().flatten();
        let mut depth = 0;
        while let Some(id) = at
            && depth <= self.parents.len()
        {
            depth += 1;
            at = self.parents.get(&id).copied().flatten();
        }
        depth
    }

    pub(crate) fn is_mount(&self, id: Id) -> bool {
        self.parents.contains_key(&id)
    }

    /// Whether `mount` is selected or holds the selected mount.
    pub(crate) fn is_selected(&self, mount: Id) -> bool {
        let mut at = self.selected;
        for _ in 0..=self.parents.len() {
            match at {
                Some(id) if id == mount => return true,
                Some(id) => at = self.parents.get(&id).copied().flatten(),
                None => return false,
            }
        }
        false
    }

    /// Moves the selection to what this pass's input says: a press selects
    /// the innermost mount under it, or none; keyboard focus arriving
    /// somewhere selects the innermost mount holding it, or none; Escape
    /// that nothing else takes steps out of the selected mount.
    pub(crate) fn settle(&mut self, input: Settling, at: impl Fn(Pos2) -> Option<Id>) {
        let arrived = input.focused.is_some() && input.focused != self.focus_seen;
        if let Some(point) = input.press {
            self.selected = at(point);
        } else if arrived {
            self.selected = input.focus_in.or_else(|| input.focus_at.and_then(&at));
        } else if input.escape == Escape::Unclaimed {
            self.selected = self
                .selected
                .and_then(|selected| self.parents.get(&selected).copied().flatten());
        }
        self.focus_seen = input.focused;
    }
}

/// Whether a mount is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selected {
    No,
    Yes,
}

/// Whether Escape was pressed this pass with nothing else to take it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Escape {
    Unpressed,
    Claimed,
    Unclaimed,
}

/// What a pass's input says that moves the selection.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settling {
    /// Where a press began this pass.
    pub(crate) press: Option<Pos2>,
    /// The widget with keyboard focus, the innermost mount it is
    /// registered in, and where it is, for focus in an overlay.
    pub(crate) focused: Option<Id>,
    pub(crate) focus_in: Option<Id>,
    pub(crate) focus_at: Option<Pos2>,
    pub(crate) escape: Escape,
}

impl crate::Shell {
    /// Whether `mount` is selected or holds the selected mount.
    pub(crate) fn mount_selected(&self, mount: Id) -> bool {
        self.kept.with(|kept| kept.mounts.is_selected(mount))
    }

    /// The innermost mount of the node at `index` in `nodes`, if it is in
    /// one.
    pub(crate) fn mount_of(&self, nodes: &crate::nodes::Nodes, index: usize) -> Option<Id> {
        self.kept.with(|kept| {
            nodes
                .ancestors(index)
                .filter_map(|above| nodes.get(above)?.id())
                .find(|&id| kept.mounts.is_mount(id))
        })
    }
}

/// Moves the selection by this pass's input, over the frame placed in
/// `nodes` and `roots`: what is under a point is the innermost mount whose
/// node holds it where the pointer reaches it, or, in an overlay, the mount
/// the overlay was declared in.
pub(crate) fn settle(
    ctx: &egui::Context,
    kept: &crate::kept::Handle,
    (nodes, by_id): (&crate::nodes::Nodes, &crate::roots::ById),
    (roots, here): (&[crate::roots::PlacedRoot], crate::roots::Here),
    typing: crate::overlay_input::Typing,
) {
    let mounts = kept.with(|kept| kept.mounts.ids());
    let focused = ctx.memory(|memory| memory.focused());
    let shell = crate::Shell::new(ctx, kept);
    let holding: Vec<Id> = focused
        .map(|_| {
            mounts
                .iter()
                .copied()
                .filter(|&mount| shell.focus_within(mount))
                .collect()
        })
        .unwrap_or_default();
    let focus_in = kept.with(|kept| {
        holding
            .iter()
            .copied()
            .max_by_key(|&mount| kept.mounts.depth(mount))
    });
    let escape = if !ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        Escape::Unpressed
    } else if typing == crate::overlay_input::Typing::Yes
        || crate::overlay_input::topmost_root(roots, here) != here.root
    {
        Escape::Claimed
    } else {
        Escape::Unclaimed
    };
    let input = Settling {
        press: crate::overlay_input::press_origin(ctx),
        focused,
        focus_in,
        // Read from the pass's widgets: `read_response` builds a response,
        // which moves egui's interaction state.
        focus_at: focused.and_then(|id| {
            ctx.viewport(|viewport| {
                viewport
                    .this_pass
                    .widgets
                    .get(id)
                    .map(|widget| widget.rect.center())
            })
        }),
        escape,
    };
    let at = |point: Pos2| {
        let root = roots.get(crate::roots::root_at(roots, nodes, point, here))?;
        let span = nodes.span(root.first);
        mounts
            .iter()
            .filter_map(|&id| Some((*by_id.get(&id)?, id)))
            .filter(|(index, _)| span.contains(index))
            .filter(|&(index, _)| {
                crate::pipeline::reachable(nodes, root.first, index, egui::Rect::EVERYTHING)
                    .contains(point)
            })
            .max_by_key(|&(index, _)| index)
            .map(|(_, id)| id)
            .or(root.mount)
    };
    kept.with(|kept| kept.mounts.settle(input, at));
}
