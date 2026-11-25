//! The chip: a badge that can be clicked, removed, or both.

use std::rc::Rc;

use crate::contract::{Look, Variant};
use crate::owned::Enablement;
use crate::{Builder, Icon, Key, NodeUi as _, Style, Tw, icons, tw};
use egui::{Modifiers, Role};

use super::{BadgeStyle, End, Face, Options};

/// TODO(prose): a chip handler not yet given.
pub struct Unset;

/// TODO(prose): a chip handler given with [`on_click`](ChipChain::on_click)
/// or [`on_remove`](ChipChain::on_remove).
pub struct Handler<A>(Rc<dyn Fn(&mut A)>);

/// What a chip does: a chip that does nothing is a badge.
enum Actions<A> {
    Click(Handler<A>),
    Remove(Handler<A>),
    Both {
        click: Handler<A>,
        remove: Handler<A>,
    },
}

impl<A> Actions<A> {
    fn split(self) -> (Option<Handler<A>>, Option<Handler<A>>) {
        match self {
            Self::Click(click) => (Some(click), None),
            Self::Remove(remove) => (None, Some(remove)),
            Self::Both { click, remove } => (Some(click), Some(remove)),
        }
    }
}

/// Which part of a chip is its Tab stop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// The chip, which is a button.
    Chip,
    /// The ×, on a chip that is only removed.
    Remove,
}

/// TODO(prose): a chip, a [`badge`](super::Ext::badge) that takes
/// input. Start one with [`Ext::chip`](super::Ext::chip), give it
/// an [`on_click`](Self::on_click), an [`on_remove`](Self::on_remove) or
/// both, and finish it with [`add`](Self::add).
///
/// A clickable chip is a button and a Tab stop; a removable one ends in a ×,
/// a button named "Remove \<label\>", and Backspace or Delete removes it
/// while it has focus. A chip that is only removable has the × as its Tab
/// stop. Either way the focus ring surrounds the whole chip.
///
/// A chip with neither handler is not a chip, and cannot be added:
///
/// ```compile_fail
/// use eggshell_ui::Builder;
/// use eggshell_ui::prelude::*;
///
/// fn view(b: &mut Builder<'_, '_, ()>) {
///     b.chip("tag", "Rust").add();
/// }
/// ```
#[must_use = "a chip is not added until you call `.add()`"]
pub struct ChipChain<'s, 'b, 'f, A, K, C, R> {
    parts: Parts<'s, 'b, 'f, A, K>,
    click: C,
    remove: R,
}

/// What a chip is, apart from what it does.
struct Parts<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    face: Face,
    name: Option<String>,
}

impl<'s, 'b, 'f, A: 'static, K: Key> ChipChain<'s, 'b, 'f, A, K, Unset, Unset> {
    pub(super) fn new(b: &'s mut Builder<'b, 'f, A>, key: K, label: String) -> Self {
        Self {
            parts: Parts {
                b,
                key,
                face: Face {
                    label,
                    options: Options::new(),
                },
                name: None,
            },
            click: Unset,
            remove: Unset,
        }
    }
}

impl<A: 'static, K: Key, C, R> ChipChain<'_, '_, '_, A, K, C, R> {
    /// TODO(prose): draws the chip as the style's variant `variant`.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.parts.face.options.variant = Some(variant);
        self
    }

    /// TODO(prose): puts `icon` before the label, in the label's colour.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.parts.face.options.icon = Some(icon);
        self
    }

    /// TODO(prose): classes merged over the styled ones; the later class
    /// wins ([`Tw::merge`]). The style's state looks still apply over them.
    pub fn classes(mut self, classes: Tw) -> Self {
        let options = &mut self.parts.face.options;
        options.classes = std::mem::replace(&mut options.classes, tw()).merge(classes);
        self
    }

    /// TODO(prose): the accessible name, in place of the label, as
    /// `aria-label` is. The × is named "Remove \<name\>".
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.parts.name = Some(name.into());
        self
    }
}

impl<'s, 'b, 'f, A: 'static, K: Key, R> ChipChain<'s, 'b, 'f, A, K, Unset, R> {
    /// TODO(prose): makes the chip a button: `on_click` runs when it is
    /// clicked or activated with Space or Enter.
    pub fn on_click(
        self,
        on_click: impl Fn(&mut A) + 'static,
    ) -> ChipChain<'s, 'b, 'f, A, K, Handler<A>, R> {
        ChipChain {
            parts: self.parts,
            click: Handler(Rc::new(on_click)),
            remove: self.remove,
        }
    }
}

impl<'s, 'b, 'f, A: 'static, K: Key, C> ChipChain<'s, 'b, 'f, A, K, C, Unset> {
    /// TODO(prose): ends the chip in a ×: `on_remove` runs when the × is
    /// clicked, or Backspace or Delete is pressed while the chip has focus.
    pub fn on_remove(
        self,
        on_remove: impl Fn(&mut A) + 'static,
    ) -> ChipChain<'s, 'b, 'f, A, K, C, Handler<A>> {
        ChipChain {
            parts: self.parts,
            click: self.click,
            remove: Handler(Rc::new(on_remove)),
        }
    }
}

impl<A: 'static, K: Key> ChipChain<'_, '_, '_, A, K, Handler<A>, Unset> {
    /// TODO(prose): adds the chip.
    #[track_caller]
    pub fn add(self) {
        self.parts.finish(Actions::Click(self.click));
    }
}

impl<A: 'static, K: Key> ChipChain<'_, '_, '_, A, K, Unset, Handler<A>> {
    /// TODO(prose): adds the chip.
    #[track_caller]
    pub fn add(self) {
        self.parts.finish(Actions::Remove(self.remove));
    }
}

impl<A: 'static, K: Key> ChipChain<'_, '_, '_, A, K, Handler<A>, Handler<A>> {
    /// TODO(prose): adds the chip.
    #[track_caller]
    pub fn add(self) {
        self.parts.finish(Actions::Both {
            click: self.click,
            remove: self.remove,
        });
    }
}

impl<A: 'static, K: Key> Parts<'_, '_, '_, A, K> {
    #[track_caller]
    fn finish(self, actions: Actions<A>) {
        let Self {
            b,
            key,
            mut face,
            name,
        } = self;
        let ctx = b.ctx().clone();
        let (style, core) = (b.style().badge.clone(), b.style().clone());
        let look = face.look(&ctx, &style);
        let name = name.unwrap_or_else(|| face.label.clone());
        #[cfg(debug_assertions)]
        if name.is_empty() {
            let id = egui::Id::NULL.with(("unnamed-chip", format!("{key:?}")));
            crate::report(&ctx, crate::Problem::Unnamed, Some(id), || {
                format!(
                    "the chip {key:?} has no label and no name: assistive technology cannot read it"
                )
            });
        }
        let enablement = b.enablement();
        let (click, remove) = actions.split();
        let stop = match click {
            Some(_) => Stop::Chip,
            None => Stop::Remove,
        };
        let _end = match remove {
            Some(_) => End::Remove,
            None => End::Label,
        };
        // The ring while the × has the focus is the chip's own, not a look.
        let ring = match stop {
            Stop::Chip => tw(),
            Stop::Remove => focus_ring(look, &core, enablement),
        };
        let root = match stop {
            Stop::Chip => super::Root::Clickable,
            Stop::Remove => super::Root::Static,
        };
        let classes = face.part(b, &style, root, ring);
        let removes = remove.map(|Handler(remove)| remove);
        let node = b.div_id(key, classes, |s| {
            face.content(s);
            if let Some(remove) = &removes {
                remove_button(s, &style, &name, stop, remove.clone());
            }
        });
        if let Some(Handler(click)) = click {
            let node = node
                .role(Role::Button)
                .named(name)
                .on_click(move |app| click(app));
            if let Some(remove) = removes {
                let _ = node.draw(removes_on_key(remove));
            }
        }
    }
}

/// The ring of a chip whose × is its Tab stop, shown while the × has focus.
fn focus_ring(look: Look, core: &Style, enablement: Enablement) -> Tw {
    match enablement {
        Enablement::Enabled => {
            let ring = look.paint.focus.unwrap_or(core.tokens.focus());
            let border = look.paint.states.focused.border;
            tw().focus_within(|p| p.border_color(border).ring(ring.width, ring.color))
        }
        Enablement::Disabled => tw(),
    }
}

/// The × of a removable chip named `name`.
fn remove_button<A: 'static>(
    b: &mut Builder<'_, '_, A>,
    style: &BadgeStyle,
    name: &str,
    stop: Stop,
    remove: Rc<dyn Fn(&mut A)>,
) {
    let looks = style.remove;
    let classes = tw()
        .flex()
        .items_center()
        .justify_center()
        .shrink_0()
        .sense_click()
        .opacity(looks.rest)
        .hover(|p| p.opacity(looks.hover))
        .transition(style.transition)
        .attr("icon", "inline-end");
    let classes = match stop {
        Stop::Chip => classes.focusable_none(),
        Stop::Remove => classes,
    };
    let clicked = remove.clone();
    let node = b
        .div_id("remove", classes, |s| {
            s.glyph(tw().size_px(looks.glyph), Icon::new(icons::CLOSE));
        })
        .role(Role::Button)
        .named(format!("Remove {name}"))
        .on_click(move |app| clicked(app));
    if stop == Stop::Remove {
        let _ = node.draw(removes_on_key(remove));
    }
}

/// Runs `remove` when Backspace or Delete is pressed while the node has
/// focus.
fn removes_on_key<A: 'static>(
    remove: Rc<dyn Fn(&mut A)>,
) -> impl FnOnce(&mut A, &mut egui::Ui) + 'static {
    move |app, ui| {
        if !ui.node_response().has_focus() {
            return;
        }
        let pressed = ui.input_mut(|input| {
            input.consume_key(Modifiers::NONE, egui::Key::Backspace)
                || input.consume_key(Modifiers::NONE, egui::Key::Delete)
        });
        if pressed {
            remove(app);
        }
    }
}
