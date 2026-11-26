//! The box a field sits in: a bordered, ringed row that holds a text field
//! and what goes beside it.

use crate::builder::{Scope, Slot};
use crate::contract::Size;
use crate::{Builder, NodeUi as _, tw};

use super::options::{Configure, Interactive, Options, Resolved, Respond};
use super::{Enablement, OnResponse, input, text_sized, tipped_whole};

/// A box in the style's input look. Start one with [`Builder::field_box`] and
/// finish it with [`children`](Self::children).
///
/// Its shared options come from [`Configure`], [`Interactive`] and
/// [`Respond`]. Its [`enablement`](Interactive::enablement) is that of the
/// box and what is inside it, and its tooltip shows while the pointer is
/// anywhere over it. [`focus_on_open`](Interactive::focus_on_open) gives the
/// overlay's opening focus to the first control inside it; the box itself
/// takes no focus, so [`focus_now`](Interactive::focus_now) does nothing.
#[must_use = "a field box is not added until you call `.children(..)`"]
pub struct Chain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    opts: Options,
    size: Option<Size>,
    on_response: OnResponse<A>,
}

impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// TODO(prose): a box in the style's input look, for a text field with something beside it.
    pub fn field_box<K: crate::Key>(&mut self, key: K) -> Chain<'_, 'f, A> {
        let slot = self.slot(&key);
        Chain::new(self.reborrow(), slot)
    }
}

impl<A> Configure for Chain<'_, '_, A> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A> Interactive for Chain<'_, '_, A> {}

impl<A> Respond for Chain<'_, '_, A> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'a, 'f, A: 'static> Chain<'a, 'f, A> {
    pub(crate) fn new(b: Builder<'a, 'f, A>, slot: Slot) -> Self {
        Self {
            b,
            slot,
            opts: Options::new(),
            size: None,
            on_response: OnResponse::default(),
        }
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): builds `body` inside the box.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Self {
            mut b,
            slot,
            opts,
            size,
            on_response,
        } = self;
        let size = b.size_of(size);
        let Resolved {
            name,
            classes,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let style = b.style().clone();
        let metrics = *style.input.sizes.lookup(b.ctx(), size);
        let keys = super::input::keys(
            style.input.sizes.key_for(size),
            enablement == Enablement::Disabled,
        );
        let own = match enablement {
            Enablement::Enabled => tw(),
            Enablement::Disabled => tw().inert(),
        };
        let classes = b.part(
            input::GroupPart::Root,
            &keys,
            own.control_size(size)
                .merge(text_sized(&b, metrics.text))
                .merge(classes),
        );
        let classes = match tooltip {
            Some(_) => classes.sense_hover(),
            None => classes,
        };
        let node = b
            .div_id_with(&slot, classes, body)
            .open_focus(autofocus.open_focus());
        let node = match name {
            Some(name) => node.named(name),
            None => node,
        };
        node.draw(move |app, ui| {
            tipped_whole(ui, &tooltip);
            on_response.run(app, &ui.node_response());
        });
    }
}
