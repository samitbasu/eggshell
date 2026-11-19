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
pub use crate::focus::Select;
use crate::focus::{Autofocus, Token};
use crate::overlay_input::Blur;
use crate::owned::FieldHooks;
use crate::owned::options::{self, Gate};
use crate::owned::{Enablement, FieldBoxChain, IntoTip, text_sized, tipped};
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

/// How a field opens when focus arrives in it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Opening {
    /// The whole text selected.
    All,
    /// The caret at a place in the text.
    Caret(egui::text::CCursor),
    /// The caret where the field keeps it, or a click puts it.
    Kept,
}

impl From<Select> for Opening {
    fn from(select: Select) -> Self {
        match select {
            Select::All => Self::All,
            Select::None => Self::Kept,
        }
    }
}

impl Opening {
    /// A number field's: as the application asked when it focused the
    /// field, otherwise the caret where a click put it, otherwise all of it,
    /// as Tab does. `None` in a frame focus did not arrive.
    pub(crate) fn of_number(response: &Response, asked: Option<Select>) -> Option<Self> {
        match asked {
            Some(select) => Some(select.into()),
            None if !response.gained_focus() => None,
            None if response.is_pointer_button_down_on() => Some(Self::Kept),
            None => Some(Self::All),
        }
    }

    /// Selects as this says in the field `response` is of.
    pub(crate) fn open(self, ui: &Ui, response: &Response, text: &str) {
        let range = match self {
            Self::All => all(text),
            Self::Caret(caret) => egui::text::CCursorRange::one(caret),
            Self::Kept => return,
        };
        select(ui, response, range);
    }
}

/// The selection rule of a field that edits a number, the stepper's and the
/// slider readout's: focus arriving opens it as `opening` says, and a double
/// click selects all of it. Returns what it did, for the input trace.
pub(crate) fn select_number(
    ui: &Ui,
    response: &Response,
    text: &str,
    opening: Option<Opening>,
) -> &'static str {
    if let Some(opening) = opening {
        opening.open(ui, response, text);
        return match opening {
            Opening::All => "focus, select all",
            Opening::Caret(_) | Opening::Kept => "focus, caret",
        };
    }
    if response.double_clicked() {
        select(ui, response, all(text));
        return "double click, select all";
    }
    "none"
}

/// The whole of `text`, as a selection.
pub(crate) fn all(text: &str) -> egui::text::CCursorRange {
    egui::text::CCursorRange::two(
        egui::text::CCursor::new(0),
        egui::text::CCursor::new(text.chars().count()),
    )
}

/// Sets the selection of the field `response` is of to `range`.
pub(crate) fn select(ui: &Ui, response: &Response, range: egui::text::CCursorRange) {
    let mut state = egui::TextEdit::load_state(ui.ctx(), response.id).unwrap_or_default();
    state.cursor.set_char_range(Some(range));
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
            enablement: Enablement::Enabled,
            gate: Gate::default(),
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
    enablement: Enablement,
    gate: Gate,
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
    pub fn focus_on_appear(self) -> Self {
        self.focus_on_appear_selecting(Select::None)
    }

    /// TODO(prose): [`focus_on_appear`](Self::focus_on_appear), selecting as
    /// `select` says when it takes focus.
    pub fn focus_on_appear_selecting(mut self, select: Select) -> Self {
        self.autofocus = Autofocus::OnAppear(select);
        self
    }

    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    pub fn focus_on_open(mut self) -> Self {
        self.autofocus = Autofocus::OnOverlayOpen;
        self
    }

    /// TODO(prose): takes focus on every frame this is set, selecting as
    /// `select` says.
    pub fn focus_now(mut self, select: Select) -> Self {
        self.autofocus = Autofocus::Now(select);
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
    pub fn focus_on_change(self, token: impl std::hash::Hash) -> Self {
        self.focus_on_change_selecting(token, Select::None)
    }

    /// TODO(prose): [`focus_on_change`](Self::focus_on_change), selecting as
    /// `select` says when it takes focus: with [`Select::All`], typing
    /// replaces the text, as a browser's `input.select()` arranges.
    ///
    /// ```
    /// use eggshell_ui::Builder;
    /// use eggshell_ui::prim::text_field::Select;
    ///
    /// struct App { name: String, focus_name: u32 }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    ///     b.text_field("name", &app.name)
    ///         .focus_on_change_selecting(app.focus_name, Select::All)
    ///         .on_change(|app, name| app.name = name);
    /// }
    /// ```
    pub fn focus_on_change_selecting(
        mut self,
        token: impl std::hash::Hash,
        select: Select,
    ) -> Self {
        self.autofocus = Autofocus::OnChange(Token::of(&token), select);
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

    /// Classes of the leaf, such as its width; the later class wins
    /// ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// TODO(prose): shown while the pointer hovers the field or it has
    /// keyboard focus, also when it is disabled.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.gate.tooltip(text);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.gate.tooltip_with(build);
        self
    }

    /// TODO(prose): whether the field takes input. A disabled field is inert,
    /// as an HTML `disabled` input is: it takes no focus, and its text cannot
    /// be selected.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        self.enablement = enablement.into();
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
            enablement,
            gate,
            hooks,
            autofocus,
            size,
            leading,
            trailing,
        } = self;
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let size = b.size_of(size);
        let field = field.sized(size);
        let measured = field.clone();
        let style = b.style().clone();
        if !field.is_single_line() && (leading.is_some() || trailing.is_some()) {
            crate::report::report(b.ctx(), crate::Problem::IgnoredIcons, Some(slot.id), || {
                format!(
                    "the multi-line field {} shows no icons; the ones given are not drawn",
                    slot.readable()
                )
            });
        }
        let icons = leading.is_some() || trailing.is_some();
        let disabled_frame = enablement == Enablement::Disabled && !field.is_bare();
        // A disabled field takes the input look's disabled colours through the
        // field box, as the stepper's does; egui's own frame has none.
        if field.is_single_line() && (icons || disabled_frame) {
            let metrics = *style.input.sizes.get(size);
            let (side, tint) = (metrics.icon_size, style.input.icon);
            let put = |s: &mut Scope<'_, '_, A>, part, icon| {
                s.icon(part, tw().size_px(side).shrink_0().text(tint), icon);
            };
            let field = field.bare();
            let measured = field.clone();
            let tip = tooltip.clone();
            FieldBoxChain::new(b, slot)
                .size(size)
                .enablement(enablement)
                .classes(classes)
                .children(|s| {
                    if let Some(icon) = leading {
                        put(s, Part::Leading, icon);
                    }
                    s.div(tw().flex().items_center().grow().min_w_0().h_full(), |b| {
                        b.leaf(Part::Field, tw().w_full().h_full(), move |m| {
                            measure(m, &measured)
                        })
                        .open_focus(autofocus.open_focus())
                        .draw_widgets(move |app, ui| {
                            let mut edited = text;
                            let shown = show(ui, &mut edited, &field);
                            let response = tipped(ui, shown, &tip);
                            if let Some(select) = autofocus.apply(ui.ctx(), &response) {
                                Opening::from(select).open(ui, &response, &edited);
                            }
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
        let classes = text_sized(&b, style.input.sizes.lookup(b.ctx(), size).text).merge(classes);
        let classes = if field.is_bare() {
            classes
        } else {
            crate::owned::text_field_classes(&style, size).merge(classes)
        };
        let classes = enablement.apply(classes, &style);
        b.leaf_with(&slot, classes, move |m| measure(m, &measured))
            .open_focus(autofocus.open_focus())
            .draw_widgets(move |app, ui| {
                let mut edited = text;
                let shown = show(ui, &mut edited, &field);
                let response = tipped(ui, shown, &tooltip);
                if let Some(select) = autofocus.apply(ui.ctx(), &response) {
                    Opening::from(select).open(ui, &response, &edited);
                }
                if response.changed() {
                    on_change(app, edited);
                }
                hooks.run(app, ui, &response, field.enter());
            });
    }
}
