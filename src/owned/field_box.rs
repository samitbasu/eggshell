//! The box a field sits in: a bordered, ringed row that holds a text field
//! and what goes beside it.

use crate::builder::{Scope, Slot};
use crate::contract::Size;
use crate::{Builder, Tw, tw};

use super::{Enablement, Gate, InputGroupPart, IntoTip, options, text_sized, tipped_whole};

/// A box in the style's input look. Start one with [`Builder::field_box`] and
/// finish it with [`children`](Self::children).
#[must_use = "a field box is not added until you call `.children(..)`"]
pub struct FieldBoxChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    classes: Tw,
    enablement: Enablement,
    gate: Gate,
    size: Option<Size>,
}

impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// TODO(prose): a box in the style's input look, for a text field with something beside it.
    pub fn field_box<K: crate::Key>(&mut self, key: K) -> FieldBoxChain<'_, 'f, A> {
        let slot = self.slot(&key);
        FieldBoxChain::new(self.reborrow(), slot)
    }
}

impl<'a, 'f, A: 'static> FieldBoxChain<'a, 'f, A> {
    pub(crate) fn new(b: Builder<'a, 'f, A>, slot: Slot) -> Self {
        Self {
            b,
            slot,
            classes: tw(),
            enablement: Enablement::Enabled,
            gate: Gate::default(),
            size: None,
        }
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Whether the box and what is inside it takes input.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.enablement = enablement;
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the last
    /// call wins.
    pub fn refused(mut self, reason: impl IntoTip) -> Self {
        self.gate.refuse(reason);
        self
    }

    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        options::when_some(self, value, then)
    }

    /// Classes merged over the styled ones, such as its width; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// TODO(prose): builds `body` inside the box.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Self {
            mut b,
            slot,
            classes,
            enablement,
            gate,
            size,
        } = self;
        let size = b.size_of(size);
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let style = b.style().clone();
        let metrics = *style.input.sizes.lookup(b.ctx(), size);
        let keys = super::input_keys(
            style.input.sizes.key_for(size),
            enablement == Enablement::Disabled,
        );
        let own = match enablement {
            Enablement::Enabled => tw(),
            Enablement::Disabled => tw().inert(),
        };
        let classes = b.part(
            InputGroupPart::Root,
            &keys,
            own.control_size(size)
                .merge(text_sized(&b, metrics.text))
                .merge(classes),
        );
        let classes = match tooltip {
            Some(_) => classes.sense_hover(),
            None => classes,
        };
        b.div_id_with(&slot, classes, body)
            .draw(move |_, ui| tipped_whole(ui, &tooltip));
    }
}
