//! A text field: single- or multi-line, with hint text, and a multi-line one
//! that grows with its content up to a cap.
//!
//! This is [`widget::TextEdit`](crate::widget::TextEdit) under the
//! primitive's pair-of-functions shape. Its egui id derives from the leaf's
//! `Ui`, so focus and the cursor survive a reorder of keyed rows.

use egui::{Response, Ui};
use taffy::Size;

use crate::widget::Stock as _;
use crate::{Builder, Key, Measure, Tw};

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

impl<A: 'static> Builder<'_, '_, A> {
    /// An identified leaf that edits the string `text` picks out of the
    /// application's state, sized by `field`.
    ///
    /// `then` runs after the field is shown, with its response: the place to
    /// describe it to assistive technology or to act on a key chord.
    pub fn text_field<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        field: Field,
        text: fn(&mut A) -> &mut String,
        then: impl FnOnce(&mut A, &Ui, &Response) + 'static,
    ) {
        let measured = field.clone();
        self.leaf(
            key,
            tw,
            move |m| measure(m, &measured),
            move |app, ui| {
                let response = show(ui, text(app), &field);
                then(app, ui, &response);
            },
        );
    }
}
