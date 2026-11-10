//! A text field: single- or multi-line, with hint text, and a multi-line one
//! that grows with its content up to a cap.
//!
//! This is [`widget::TextEdit`](crate::widget::TextEdit) under the
//! primitive's pair-of-functions shape. Its egui id derives from the leaf's
//! `Ui`, so focus and the cursor survive a reorder of keyed rows.

use egui::{Response, Ui};
use taffy::Size;

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
    /// An identified leaf that edits a string of the application's state,
    /// sized by `text`, which is the string as it is when the frame is built.
    pub fn text_field<K: Key>(
        &mut self,
        key: K,
        text: impl Into<String>,
    ) -> TextFieldBuilder<'_, 'b, 'f, A, K> {
        self.text_field_with(key, Field::singleline(text))
    }

    /// [`text_field`](Self::text_field) sized by a `field` configured
    /// elsewhere, for a helper that passes one on.
    pub fn text_field_with<K: Key>(
        &mut self,
        key: K,
        field: Field,
    ) -> TextFieldBuilder<'_, 'b, 'f, A, K> {
        TextFieldBuilder {
            b: self,
            key,
            field,
            classes: tw(),
            on_response: None,
        }
    }
}

type OnResponse<A> = Box<dyn FnOnce(&mut A, &Ui, &Response)>;

/// A text field leaf. Start one with [`Builder::text_field`] and finish it
/// with [`bind`](Self::bind).
#[must_use = "a text field is not added until you call `.bind(..)`"]
pub struct TextFieldBuilder<'a, 'b, 'f, A, K> {
    b: &'a mut Builder<'b, 'f, A>,
    key: K,
    field: Field,
    classes: Tw,
    on_response: Option<OnResponse<A>>,
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

    /// Runs `then` after the field is shown, with its response: the place to
    /// describe it to assistive technology or to act on a key chord.
    pub fn on_response(mut self, then: impl FnOnce(&mut A, &Ui, &Response) + 'static) -> Self {
        self.on_response = Some(Box::new(then));
        self
    }

    /// Adds the field, which edits the string that `lens` picks out of the
    /// application's state.
    pub fn bind(self, lens: impl Fn(&mut A) -> &mut String + 'static) {
        let Self {
            b,
            key,
            field,
            classes,
            on_response,
        } = self;
        let measured = field.clone();
        b.leaf(
            key,
            classes,
            move |m| measure(m, &measured),
            move |app, ui| {
                let response = show(ui, lens(app), &field);
                if let Some(then) = on_response {
                    then(app, ui, &response);
                }
            },
        );
    }
}
