//! The box a field sits in: a bordered, ringed row that holds a text field
//! and what goes beside it.

use crate::builder::{Scope, Slot};
use crate::contract::{Size, Style};
use crate::{Builder, Tw, tw};

use super::{Enablement, field_box_classes, text_sized};

/// A box in the style's input look. Start one with [`Builder::field_box`] and
/// finish it with [`children`](Self::children).
#[must_use = "a field box is not added until you call `.children(..)`"]
pub struct FieldBoxChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    classes: Tw,
    enablement: Enablement,
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
            size: None,
        }
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Whether the box and what is inside it takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Classes merged over the styled ones, such as its width; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
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
            size,
        } = self;
        let size = b.size_of(size);
        let enablement = b.enablement_of(enablement);
        let style = Style::of(b.ctx());
        let metrics = *style.input.sizes.lookup(b.ctx(), size);
        let classes = field_box_classes(
            &style.input.look,
            &style,
            metrics.radius,
            metrics.height,
            enablement,
        )
        .items_center()
        .gap(metrics.icon_gap / 4.0)
        .px(metrics.padding.x / 4.0)
        .control_size(size)
        .merge(text_sized(&b, metrics.text))
        .merge(classes);
        b.div_id_with(&slot, classes, body);
    }
}
