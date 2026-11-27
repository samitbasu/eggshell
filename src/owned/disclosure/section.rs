//! The section: a header that opens and closes a collapsible region.

use crate::owned::accordion;
use std::rc::Rc;

use crate::focus::Autofocus;
use crate::overlay::Shown;
use crate::owned::options::{Configure, Interactive, Options, Resolved, Respond};
use crate::owned::{Enablement, OnResponse, Tip, button::Face, hooked};
use crate::{Builder, Key, NodeUi as _, Progress, Scope, Tw, child_id, tw};
use egui::{Context, Role};

use crate::sheet::Key as SheetKey;

use super::{ChangeFn, OnChange, Style, Unset, name_of, toggled};

/// The parts of a section, under its key.
#[derive(Clone, Copy, Debug, Hash)]
enum Node {
    Header,
    Region,
}

/// Whether a header asks to change its section's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(feature = "accordion"),
    expect(dead_code, reason = "only an accordion pins a section open")
)]
enum Toggle {
    /// A click, Enter or Space asks for the other state.
    Free,
    /// The section stays open: the header does nothing and is announced as
    /// disabled, though it does not look it.
    Pinned,
}

/// Whether the header is an item of a group of headers the arrow keys move
/// between.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(feature = "accordion"),
    expect(dead_code, reason = "only an accordion groups its headers")
)]
enum Membership {
    /// A section of its own.
    Alone,
    /// One of an accordion's headers.
    Item,
}

/// What builds the inside of a header after its face.
type HeaderChildren<'f, A> = Box<dyn for<'s> FnOnce(&mut Builder<'s, 'f, A>) + 'f>;

/// TODO(prose): a section: see [`SectionExt::section`](super::SectionExt::section).
///
/// The header is a button named by the face, announced as expanded while
/// the section is open and as controlling the region while the region is
/// built. Click, Enter or Space asks [`on_change`](Self::on_change) for the
/// other state. The region is a [`Region`](Role::Region) named as the header
/// is, and opens and closes by its height; the chevron turns with it, and
/// both snap when the style has no animation time.
///
/// Its shared options come from [`Configure`], [`Interactive`] and
/// [`Respond`], in scope through the [`prelude`](crate::prelude), and are
/// the header's: [`named`](Configure::named) names the header and the
/// region in place of the face, and [`classes`](Configure::classes) are
/// merged over the header's styled ones.
#[must_use = "a section is built by `.children(..)`"]
pub struct SectionChain<'s, 'b, 'f, A, K, H> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    face: Face,
    current: Shown,
    on_change: H,
    opts: Options,
    on_response: OnResponse<A>,
    content: Tw,
    toggle: Toggle,
    membership: Membership,
    header_children: Option<HeaderChildren<'f, A>>,
}

impl<'s, 'b, 'f, A: 'static, K: Key> SectionChain<'s, 'b, 'f, A, K, Unset> {
    pub(super) fn new(b: &'s mut Builder<'b, 'f, A>, key: K, face: Face, current: Shown) -> Self {
        Self {
            b,
            key,
            face,
            current,
            on_change: Unset,
            opts: Options::new(),
            on_response: OnResponse::default(),
            content: tw(),
            toggle: Toggle::Free,
            membership: Membership::Alone,
            header_children: None,
        }
    }

    /// TODO(prose): `on_change` runs with the state a click, Enter or Space
    /// on the header asks for. The open state is the app's: the section
    /// stores none.
    pub fn on_change(
        self,
        on_change: impl Fn(&mut A, Shown) + 'static,
    ) -> SectionChain<'s, 'b, 'f, A, K, OnChange<A>> {
        self.on_change_with(move |app, _, shown| on_change(app, shown))
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    pub fn on_change_with(
        self,
        on_change: impl Fn(&mut A, &Context, Shown) + 'static,
    ) -> SectionChain<'s, 'b, 'f, A, K, OnChange<A>> {
        SectionChain {
            b: self.b,
            key: self.key,
            face: self.face,
            current: self.current,
            on_change: OnChange(Rc::new(on_change)),
            opts: self.opts,
            on_response: self.on_response,
            content: self.content,
            toggle: self.toggle,
            membership: self.membership,
            header_children: self.header_children,
        }
    }
}

impl<A, K, H> Configure for SectionChain<'_, '_, '_, A, K, H> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, K, H> Interactive for SectionChain<'_, '_, '_, A, K, H> {}

impl<A, K, H> Respond for SectionChain<'_, '_, '_, A, K, H> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'f, A: 'static, K: Key, H> SectionChain<'_, '_, 'f, A, K, H> {
    /// TODO(prose): classes merged over the content block's styled ones:
    /// see [`CollapsibleChain::content`](crate::CollapsibleChain::content).
    pub fn content(mut self, content: Tw) -> Self {
        self.content = std::mem::replace(&mut self.content, tw()).merge(content);
        self
    }

    /// Keeps the open section open: [`Toggle::Pinned`].
    #[cfg(feature = "accordion")]
    pub(crate) fn pinned(mut self) -> Self {
        self.toggle = Toggle::Pinned;
        self
    }

    /// Makes the header an item of the accordion's column.
    #[cfg(feature = "accordion")]
    pub(crate) fn in_column(mut self) -> Self {
        self.membership = Membership::Item;
        self
    }

    /// TODO(prose): builds more into the header after the face and before
    /// the chevron, such as a count; with [`Face::Empty`] it is the whole
    /// title.
    pub fn header_children(
        mut self,
        body: impl for<'s> FnOnce(&mut Builder<'s, 'f, A>) + 'f,
    ) -> Self {
        self.header_children = Some(Box::new(body));
        self
    }
}

impl<'f, A: 'static, K: Key> SectionChain<'_, '_, 'f, A, K, OnChange<A>> {
    /// TODO(prose): builds the section; `body` builds the content, which is
    /// not built while the section is closed and not moving.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Self {
            b,
            key,
            face,
            current,
            on_change: OnChange(on_change),
            opts,
            on_response,
            content,
            toggle,
            membership,
            header_children,
        } = self;
        let style = b.style().disclosure.clone();
        let slot = b.slot(&key);
        let trigger = child_id(slot.id, &Node::Header);
        let Resolved {
            name,
            classes,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(b, trigger);
        let name = name.unwrap_or_else(|| name_of(&face).to_owned());
        let header = Header {
            style: &style,
            face,
            name: name.clone(),
            header_children,
            classes,
            enablement,
            hooks: (tooltip, autofocus, on_response),
            toggle,
            membership,
            current,
            on_change,
        };
        let keys = keys_of(enablement);
        let content = b.part(accordion::Part::ContentInner, &[], content);
        let item = b.part(accordion::Part::Item, keys, tw().flex().flex_col());
        b.div_id_with(&slot, item, |b| {
            let region = b.id_of(Node::Region);
            let region_classes = if current.is_open() {
                tw().sense_hover().focusable_none()
            } else {
                tw()
            };
            let region_classes = b.part(accordion::Part::Content, keys, region_classes);
            b.collapsible(Node::Region, current)
                .classes(region_classes)
                .content(content)
                .children_with_header(|b, open| header.build(b, open, region), body)
                .role(Role::Region)
                .named(name);
        });
    }
}

/// A section's header, ready to build.
struct Header<'a, 'f, A> {
    style: &'a Style,
    face: Face,
    name: String,
    header_children: Option<HeaderChildren<'f, A>>,
    classes: Tw,
    enablement: Enablement,
    hooks: (Tip, Autofocus, OnResponse<A>),
    toggle: Toggle,
    membership: Membership,
    current: Shown,
    on_change: ChangeFn<A>,
}

impl<'f, A: 'static> Header<'_, 'f, A> {
    fn build(self, b: &mut Scope<'_, 'f, A>, open: Progress, region: egui::Id) {
        let Self {
            style,
            face,
            name,
            header_children,
            classes,
            enablement,
            hooks: (tooltip, autofocus, on_response),
            toggle,
            membership,
            current,
            on_change,
        } = self;
        let sense = match enablement {
            Enablement::Enabled => tw().sense_click(),
            Enablement::Disabled => tw().sense_click().inert(),
        };
        let keys = keys_of(enablement);
        let box_classes = b.part(accordion::Part::Trigger, keys, sense);
        let glyph = style.section.at(open);
        let glyph_classes = b.part(accordion::Part::TriggerIcon, keys, tw());
        let handle = b
            .div_id(Node::Header, box_classes.merge(classes), |b| {
                b.div(
                    tw().flex().items_center().gap(2.0).flex_1().min_w_0(),
                    |b| {
                        match face {
                            Face::Label(text) => b.text(tw(), text),
                            Face::Icon(icon, _) => b.glyph(tw().shrink_0(), icon),
                            Face::Empty(_) => {}
                        }
                        if let Some(children) = header_children {
                            b.div(tw().flex().items_center().gap(2.0), children);
                        }
                    },
                );
                b.glyph(glyph_classes, glyph);
            })
            .role(Role::Button)
            .named(name)
            .expanded(current.is_open())
            .open_focus(autofocus.open_focus())
            .draw(hooked(tooltip, autofocus, on_response));
        let handle = match membership {
            Membership::Item => handle.nav_item(),
            Membership::Alone => handle,
        };
        let built = current.is_open() || f32::from(open) > 0.0;
        let handle = if built {
            handle.controls(region)
        } else {
            handle
        };
        match (enablement, toggle) {
            (Enablement::Enabled, Toggle::Free) => {
                handle.on_click_with(move |app: &mut A, ctx| on_change(app, ctx, toggled(current)));
            }
            (Enablement::Enabled, Toggle::Pinned) => {
                handle.draw(|_, ui| {
                    let id = ui.node_response().id;
                    ui.ctx()
                        .accesskit_node_builder(id, |node| node.set_disabled());
                });
            }
            (Enablement::Disabled, _) => {}
        }
    }
}

/// The keys a section's parts carry for the style's sheet.
fn keys_of(enablement: Enablement) -> &'static [SheetKey] {
    match enablement {
        Enablement::Enabled => &[],
        Enablement::Disabled => &[SheetKey::Aria(crate::Aria::Disabled)],
    }
}
