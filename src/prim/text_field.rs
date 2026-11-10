//! A text field: single- or multi-line, with hint text, and a multi-line one
//! that grows with its content up to a cap.
//!
//! This is [`widget::TextEdit`](crate::widget::TextEdit) under the
//! primitive's pair-of-functions shape. Its egui id derives from the leaf's
//! `Ui`, so focus and the cursor survive a reorder of keyed rows.

use egui::{Response, Ui};
use taffy::Size;

use crate::owned::FieldHooks;
use crate::widget::Stock as _;
use crate::{Builder, Key, Measure, Tw, tw};

pub use crate::widget::{Lines, TextEdit as Field};

/// A leaf's measure for `field`: its width from the leaf, its height from
/// its text at that width.
pub fn measure(m: &Measure<'_>, field: &Field) -> Size<f32> {
    field.measure(m)
}

/// Edits `text` in the leaf's content rect.
pub fn show(ui: &mut Ui, text: &mut String, field: &Field) -> Response {
    field.show(ui, text)
}

impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// An identified leaf that edits `text`, the string of the application's
    /// state as it is when the frame is built. The field edits a copy, which
    /// [`on_change`](TextFieldBuilder::on_change) hands back when it differs.
    pub fn text_field<K: Key>(
        &mut self,
        key: K,
        text: impl Into<String>,
    ) -> TextFieldBuilder<'_, 'b, 'f, A, K> {
        self.text_field_with(key, Field::singleline(text))
    }

    /// [`text_field`](Self::text_field) for a `field` configured elsewhere,
    /// which holds the text, for a helper that passes one on.
    pub fn text_field_with<K: Key>(
        &mut self,
        key: K,
        field: Field,
    ) -> TextFieldBuilder<'_, 'b, 'f, A, K> {
        TextFieldBuilder {
            b: self,
            key,
            text: field.text().to_owned(),
            field,
            classes: tw(),
            hooks: FieldHooks::default(),
        }
    }
}

/// A text field leaf. Start one with [`Builder::text_field`] and finish it
/// with [`on_change`](Self::on_change).
#[must_use = "a text field is not added until you call `.on_change(..)`"]
pub struct TextFieldBuilder<'a, 'b, 'f, A, K> {
    b: &'a mut Builder<'b, 'f, A>,
    key: K,
    field: Field,
    text: String,
    classes: Tw,
    hooks: FieldHooks<A>,
}

impl<A: 'static, K: Key> TextFieldBuilder<'_, '_, '_, A, K> {
    /// Greyed text shown while the field is empty.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.field = self.field.hint(hint);
        self
    }

    /// Wraps the text over at least `min` rows, and grows with it.
    pub fn multiline(mut self, min: usize) -> Self {
        self.field = self.field.multiline_rows(min);
        self
    }

    /// Stops a multi-line field growing at `max` rows. It scrolls instead.
    pub fn max_rows(mut self, max: usize) -> Self {
        self.field = self.field.max_rows(max);
        self
    }

    /// Shows dots for what is typed, like a password field.
    pub fn masked(mut self) -> Self {
        self.field = self.field.masked();
        self
    }

    /// Draws no frame, margin or background: the field is its text, for a
    /// field that sits inside a box of the application's own.
    pub fn bare(mut self) -> Self {
        self.field = self.field.bare();
        self
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.field = self.field.named(name);
        self
    }

    /// Classes of the leaf, such as its width.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Runs `then` with the field's response each frame: the escape hatch
    /// for what the builder does not offer, such as describing it to
    /// assistive technology or acting on a key chord.
    pub fn on_response(mut self, then: impl Fn(&mut A, &Response) + 'static) -> Self {
        self.hooks.response.set(then);
        self
    }

    /// Runs `then` when Enter is pressed in a single-line field.
    pub fn when_submitted(mut self, then: impl Fn(&mut A) + 'static) -> Self {
        self.hooks.submitted(then);
        self
    }

    /// Runs `then` when the field loses focus, also when Enter does it.
    pub fn when_blurred(mut self, then: impl Fn(&mut A) + 'static) -> Self {
        self.hooks.blurred(then);
        self
    }

    /// Adds the field. `on_change` runs with the new text in the frame the
    /// user edits it, and not otherwise.
    pub fn on_change(self, on_change: impl Fn(&mut A, String) + 'static) {
        let Self {
            b,
            key,
            field,
            text,
            classes,
            hooks,
        } = self;
        let measured = field.clone();
        b.leaf(
            key,
            classes,
            move |m| measure(m, &measured),
            move |app, ui| {
                let mut edited = text;
                let response = show(ui, &mut edited, &field);
                if response.changed() {
                    on_change(app, edited);
                }
                hooks.run(app, ui, &response);
            },
        );
    }
}
