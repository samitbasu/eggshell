//! TODO(prose): an accordion: sections in a column, opened one at a time or
//! any number at once, as the mode's type says.
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui_extras::accordion::{AccordionExt, Single};
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
//! enum Faq { Access, Motion }
//!
//! struct App { faq: Option<Faq> }
//!
//! fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
//!     b.accordion("faq", Single::collapsible(app.faq))
//!         .on_change(|app: &mut App, open| app.faq = open)
//!         .items(|acc| {
//!             acc.item(Faq::Access, "Is it accessible?")
//!                 .children(|b| b.text(tw(), "Yes."));
//!             acc.item(Faq::Motion, "Is it animated?")
//!                 .children(|b| b.text(tw(), "Yes."));
//!         });
//! }
//! ```

use std::marker::PhantomData;
use std::rc::Rc;

use eggshell_ui::overlay::Shown;
use eggshell_ui::owned::{Enablement, Face};
use eggshell_ui::{Builder, Key, Scope, Tw, tw};

use crate::disclosure::{DisclosureStyle, SectionExt};

/// TODO(prose): an accordion item's value: its key under the accordion's.
pub trait Value: Key + Copy + PartialEq {}

impl<T: Key + Copy + PartialEq> Value for T {}

/// What an accordion's mode answers for each item.
pub trait Mode<T> {
    /// What `on_change` is handed.
    type Change: Clone + 'static;

    /// Whether `item` is open.
    fn shown(&self, item: T) -> Shown;

    /// What a header asking to put `item` in state `to` changes, if
    /// anything.
    fn change(&self, item: T, to: Shown) -> Option<Self::Change>;
}

/// TODO(prose): at most one item open.
pub struct Single;

/// TODO(prose): [`Single::collapsible`]: one item open, or none.
#[derive(Clone, Copy, Debug)]
pub struct Collapsible<T>(Option<T>);

/// TODO(prose): [`Single::always`]: exactly one item open. Its header does
/// nothing and is announced as disabled, as Radix's `collapsible={false}`.
#[derive(Clone, Copy, Debug)]
pub struct Always<T>(T);

impl Single {
    /// TODO(prose): `open` is the open item, if one is; `on_change` is
    /// handed the new one, `None` once the open one is closed.
    pub fn collapsible<T>(open: Option<T>) -> Collapsible<T> {
        Collapsible(open)
    }

    /// TODO(prose): `open` is the open item; `on_change` is handed the item
    /// opened in its place.
    pub fn always<T>(open: T) -> Always<T> {
        Always(open)
    }
}

impl<T: Value> Mode<T> for Collapsible<T> {
    type Change = Option<T>;

    fn shown(&self, item: T) -> Shown {
        Shown::from(self.0 == Some(item))
    }

    fn change(&self, item: T, to: Shown) -> Option<Option<T>> {
        Some(to.is_open().then_some(item))
    }
}

impl<T: Value> Mode<T> for Always<T> {
    type Change = T;

    fn shown(&self, item: T) -> Shown {
        Shown::from(self.0 == item)
    }

    fn change(&self, item: T, to: Shown) -> Option<T> {
        to.is_open().then_some(item)
    }
}

/// TODO(prose): any number of items open, as `shown` says of each, read
/// from the app's own storage; `on_change` is handed an item and the state
/// it asks for.
pub struct Multiple<F>(F);

impl<F> Multiple<F> {
    /// TODO(prose): an accordion whose item `item` is open as `shown(item)`
    /// says.
    pub fn new(shown: F) -> Self {
        Self(shown)
    }
}

impl<T: Value, F: Fn(T) -> Shown> Mode<T> for Multiple<F> {
    type Change = (T, Shown);

    fn shown(&self, item: T) -> Shown {
        (self.0)(item)
    }

    fn change(&self, item: T, to: Shown) -> Option<(T, Shown)> {
        Some((item, to))
    }
}

/// TODO(prose): adds [`accordion`](Self::accordion) to the builder.
pub trait AccordionExt<'b, 'f, A> {
    /// TODO(prose): an accordion whose items are open as `mode` says. It is
    /// built by [`items`](AccordionChain::items) once it has an
    /// [`on_change`](AccordionChain::on_change).
    fn accordion<'s, K: Key, M>(
        &'s mut self,
        key: K,
        mode: M,
    ) -> AccordionChain<'s, 'b, 'f, A, K, M>;
}

impl<'b, 'f, A: 'static> AccordionExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn accordion<'s, K: Key, M>(
        &'s mut self,
        key: K,
        mode: M,
    ) -> AccordionChain<'s, 'b, 'f, A, K, M> {
        AccordionChain {
            b: self,
            key,
            mode,
            classes: tw(),
        }
    }
}

/// TODO(prose): an accordion not yet given its `on_change`.
#[must_use = "an accordion is built by `.on_change(..).items(..)`"]
pub struct AccordionChain<'s, 'b, 'f, A, K, M> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    mode: M,
    classes: Tw,
}

impl<'s, 'b, 'f, A: 'static, K: Key, M> AccordionChain<'s, 'b, 'f, A, K, M> {
    /// TODO(prose): classes merged over the column's styled ones.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = std::mem::replace(&mut self.classes, tw()).merge(classes);
        self
    }

    /// TODO(prose): `on_change` is handed what the mode says a header asked
    /// for.
    pub fn on_change<T: Value, C>(
        self,
        on_change: impl Fn(&mut A, C) + 'static,
    ) -> BoundAccordion<'s, 'b, 'f, A, K, M, T, C>
    where
        M: Mode<T, Change = C>,
    {
        BoundAccordion {
            b: self.b,
            key: self.key,
            mode: self.mode,
            classes: self.classes,
            on_change: Rc::new(on_change),
            value: PhantomData,
        }
    }
}

/// TODO(prose): an accordion with its `on_change`: build its items with
/// [`items`](Self::items).
#[must_use = "an accordion is built by `.items(..)`"]
pub struct BoundAccordion<'s, 'b, 'f, A, K, M, T, C> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    mode: M,
    classes: Tw,
    on_change: Rc<dyn Fn(&mut A, C)>,
    value: PhantomData<fn(T)>,
}

impl<'f, A: 'static, K: Key, M: Mode<T, Change = C>, T: Value, C: 'static>
    BoundAccordion<'_, '_, 'f, A, K, M, T, C>
{
    /// TODO(prose): builds the accordion; `items` adds its items in order.
    pub fn items(self, items: impl for<'i, 'x> FnOnce(&mut Items<'i, 'x, 'f, A, M, T, C>)) {
        let Self {
            b,
            key,
            mode,
            classes,
            on_change,
            value: _,
        } = self;
        let style = DisclosureStyle::of(b.ctx());
        let column = tw().flex().flex_col();
        let column = match style.separator {
            Some(line) => column.divide_y(1.0, line),
            None => column,
        };
        b.div_id(key, column.merge(classes), |b| {
            items(&mut Items {
                b,
                mode: Rc::new(mode),
                on_change,
                value: PhantomData,
            });
        });
    }
}

/// TODO(prose): adds an accordion's items: see [`item`](Self::item).
pub struct Items<'i, 'b, 'f, A, M, T, C> {
    b: &'i mut Builder<'b, 'f, A>,
    mode: Rc<M>,
    on_change: Rc<dyn Fn(&mut A, C)>,
    value: PhantomData<fn(T)>,
}

impl<'i, 'b, 'f, A: 'static, M: Mode<T, Change = C>, T: Value, C: Clone + 'static>
    Items<'i, 'b, 'f, A, M, T, C>
{
    /// TODO(prose): an item headed by `face`; its value is its key, so
    /// items need no keys of their own. Build it with
    /// [`children`](ItemChain::children).
    pub fn item(&mut self, value: T, face: impl Into<Face>) -> ItemChain<'_, 'b, 'f, A, T> {
        let shown = self.mode.shown(value);
        let opened = self.mode.change(value, Shown::Open);
        let closed = self.mode.change(value, Shown::Closed);
        let pinned = shown.is_open() && closed.is_none();
        let on_change = Rc::clone(&self.on_change);
        let section = self
            .b
            .section(value, face, shown)
            .on_change(move |app: &mut A, to| {
                let change = match to {
                    Shown::Open => opened.clone(),
                    Shown::Closed => closed.clone(),
                };
                if let Some(change) = change {
                    on_change(app, change);
                }
            });
        ItemChain {
            section: if pinned { section.pinned() } else { section },
        }
    }
}

/// TODO(prose): an accordion item: the options of a
/// [`section`](crate::disclosure::SectionExt::section).
#[must_use = "an item is built by `.children(..)`"]
pub struct ItemChain<'s, 'b, 'f, A, T> {
    section: crate::disclosure::SectionChain<'s, 'b, 'f, A, T, crate::disclosure::OnChange<A>>,
}

impl<'f, A: 'static, T: Value> ItemChain<'_, '_, 'f, A, T> {
    /// TODO(prose): classes merged over the header's styled ones.
    pub fn classes(self, classes: Tw) -> Self {
        Self {
            section: self.section.classes(classes),
        }
    }

    /// TODO(prose): classes merged over the content block's styled ones.
    pub fn content(self, content: Tw) -> Self {
        Self {
            section: self.section.content(content),
        }
    }

    /// TODO(prose): whether the header takes input; a disabled item is
    /// faded and skipped.
    pub fn enablement(self, enablement: impl Into<Enablement>) -> Self {
        Self {
            section: self.section.enablement(enablement),
        }
    }

    /// TODO(prose): builds more into the header after the face.
    pub fn header_children(self, body: impl for<'x> FnOnce(&mut Builder<'x, 'f, A>) + 'f) -> Self {
        Self {
            section: self.section.header_children(body),
        }
    }

    /// TODO(prose): builds the item; `body` builds its content.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        self.section.children(body);
    }
}
