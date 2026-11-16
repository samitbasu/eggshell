//! A text field: single- or multi-line, with hint text, and a multi-line one
//! that grows with its content up to a cap.
//!
//! This is [`widget::TextEdit`](crate::widget::TextEdit) under the
//! primitive's pair-of-functions shape. Its egui id derives from the leaf's
//! `Ui`, so focus and the cursor survive a reorder of keyed rows.

use egui::{Response, Ui};
use taffy::Size;

use crate::builder::Scope;
use crate::builder::Slot;
use crate::contract::Size as ControlSize;
use crate::focus::{Autofocus, Token};
use crate::overlay_input::Blur;
use crate::owned::FieldHooks;
use crate::owned::{FieldBoxChain, text_sized};
use crate::widget::Stock as _;
use crate::{Builder, Icon, Key, Measure, Tw, tw};

pub use crate::widget::{Enter, Lines, TextEdit as Field};

/// A leaf's measure for `field`: its width from the leaf, its height from
/// its text at that width.
pub fn measure(m: &Measure<'_>, field: &Field) -> Size<f32> {
    field.measure(m)
}

/// Edits `text` in the leaf's content rect.
pub fn show(ui: &mut Ui, text: &mut String, field: &Field) -> Response {
    field.show(ui, text)
}

/// Selects all of `text` in the field `response` is of, in the frame it gains
/// focus, so typing replaces it.
pub(crate) fn select_on_focus(ui: &Ui, response: &Response, text: &str) {
    if !response.gained_focus() {
        return;
    }
    let mut state = egui::TextEdit::load_state(ui.ctx(), response.id).unwrap_or_default();
    let all = egui::text::CCursorRange::two(
        egui::text::CCursor::new(0),
        egui::text::CCursor::new(text.chars().count()),
    );
    state.cursor.set_char_range(Some(all));
    state.store(ui.ctx(), response.id);
}

impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// An identified leaf that edits `text`, the string of the application's
    /// state as it is when the frame is built. The field edits a copy, which
    /// [`on_change`](TextFieldChain::on_change) hands back when it differs.
    /// Assistive technology sees a `Role::TextInput`, named by
    /// [`named`](TextFieldChain::named).
    pub fn text_field<K: Key>(
        &mut self,
        key: K,
        text: impl Into<String>,
    ) -> TextFieldChain<'_, 'f, A> {
        self.text_field_with(key, Field::singleline(text))
    }

    /// [`text_field`](Self::text_field) for a `field` configured elsewhere,
    /// which holds the text, for a helper that passes one on.
    pub fn text_field_with<K: Key>(&mut self, key: K, field: Field) -> TextFieldChain<'_, 'f, A> {
        TextFieldChain {
            slot: self.slot(&key),
            b: self.reborrow(),
            text: field.text().to_owned(),
            field,
            classes: tw(),
            hooks: FieldHooks::default(),
            autofocus: Autofocus::Never,
            size: None,
            leading: None,
            trailing: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Leading,
    Field,
    Trailing,
}

/// A text field leaf. Start one with [`Builder::text_field`] and finish it
/// with [`on_change`](Self::on_change).
#[must_use = "a text field is not added until you call `.on_change(..)`"]
pub struct TextFieldChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    field: Field,
    text: String,
    classes: Tw,
    hooks: FieldHooks<A>,
    autofocus: Autofocus,
    size: Option<ControlSize>,
    leading: Option<Icon>,
    trailing: Option<Icon>,
}

impl<A: 'static> TextFieldChain<'_, '_, A> {
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

    /// Takes keyboard focus in the first frame the field is built after a
    /// frame in which it was not: an inline edit or a sheet's name field that
    /// appears on a click. A field that stays built keeps whatever focus it
    /// has.
    pub fn focus_on_appear(mut self) -> Self {
        self.autofocus = Autofocus::OnAppear;
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): what Enter does in this one-line field.
    ///
    /// ```
    /// use eggshell_ui::Builder;
    /// use eggshell_ui::prim::text_field::Enter;
    ///
    /// struct App { name: String }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    ///     b.text_field("name", &app.name)
    ///         .on_enter(Enter::KeepFocus)
    ///         .when_submitted(|app| println!("wrote {}", app.name))
    ///         .on_change(|app, name| app.name = name);
    /// }
    /// ```
    pub fn on_enter(mut self, enter: Enter) -> Self {
        self.field = self.field.on_enter(enter);
        self
    }

    /// TODO(prose): takes focus once each time `token` differs from the last frame's.
    ///
    /// ```
    /// use eggshell_ui::Builder;
    ///
    /// struct App { name: String, focus_name: u32 }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    ///     b.text_field("name", &app.name)
    ///         .focus_on_change(app.focus_name)
    ///         .on_change(|app, name| app.name = name);
    /// }
    /// ```
    pub fn focus_on_change(mut self, token: impl std::hash::Hash) -> Self {
        self.autofocus = Autofocus::OnChange(Token::of(&token));
        self
    }

    /// TODO(prose): `icon` before the text, inside the field's box. Display only; a multi-line field shows none.
    pub fn leading(mut self, icon: Icon) -> Self {
        self.leading = Some(icon);
        self
    }

    /// TODO(prose): `icon` after the text, inside the field's box. Display only; a multi-line field shows none.
    pub fn trailing(mut self, icon: Icon) -> Self {
        self.trailing = Some(icon);
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

    /// Runs `then` with why, when the field loses focus. Enter blurs it too:
    /// `when_submitted` runs first, then this with [`Blur::Submitted`].
    pub fn when_blurred(mut self, then: impl Fn(&mut A, Blur) + 'static) -> Self {
        self.hooks.blurred(then);
        self
    }

    /// Adds the field. `on_change` runs with the new text in the frame the
    /// user edits it, and not otherwise.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, String) + 'static) {
        let Self {
            mut b,
            slot,
            field,
            text,
            classes,
            hooks,
            autofocus,
            size,
            leading,
            trailing,
        } = self;
        let size = b.size_of(size);
        let field = field.sized(size);
        let measured = field.clone();
        let style = crate::Style::of(b.ctx());
        // TODO(#279): report icons given to a multi-line field, which shows none.
        if field.is_single_line() && (leading.is_some() || trailing.is_some()) {
            let metrics = *style.input.sizes.get(size);
            let (side, tint) = (metrics.icon_size, style.input.icon);
            let put = |s: &mut Scope<'_, '_, A>, part, icon| {
                s.icon(part, tw().size_px(side).shrink_0().text(tint), icon);
            };
            let field = field.bare();
            let measured = field.clone();
            FieldBoxChain::new(b, slot)
                .size(size)
                .classes(classes)
                .children(|s| {
                    if let Some(icon) = leading {
                        put(s, Part::Leading, icon);
                    }
                    s.div(tw().flex().items_center().grow().min_w_0().h_full(), |b| {
                        b.leaf(Part::Field, tw().w_full().h_full(), move |m| {
                            measure(m, &measured)
                        })
                        .draw_widgets(move |app, ui| {
                            let mut edited = text;
                            let response = show(ui, &mut edited, &field);
                            autofocus.apply(ui.ctx(), &response);
                            if response.changed() {
                                on_change(app, edited);
                            }
                            hooks.run(app, ui, &response, field.enter());
                        });
                    });
                    if let Some(icon) = trailing {
                        put(s, Part::Trailing, icon);
                    }
                });
            return;
        }
        let classes = text_sized(&b, style.input.sizes.get(size).text).merge(classes);
        let classes = if field.is_bare() {
            classes
        } else {
            crate::owned::text_field_classes(&style, size).merge(classes)
        };
        b.leaf_with(slot.id, &slot.label, classes, move |m| {
            measure(m, &measured)
        })
        .draw_widgets(move |app, ui| {
            let mut edited = text;
            let response = show(ui, &mut edited, &field);
            autofocus.apply(ui.ctx(), &response);
            if response.changed() {
                on_change(app, edited);
            }
            hooks.run(app, ui, &response, field.enter());
        });
    }
}
