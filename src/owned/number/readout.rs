//! A number shown as text that a click edits: the readout of a slider's
//! header and the standalone [`Chain`]. One implementation serves both.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Role, Ui};

use super::field::Session;
use super::{Edit, Edits, Step, ValueRange};
use crate::builder::Slot;
use crate::contract::{Colors, Size};
use crate::focus::{Autofocus, Select};
use crate::input_trace::{self, Shown};
use crate::owned::options::{Configure, Interactive, Options, Resolved, Respond};
use crate::owned::text_field::{self, Field, Opening};
use crate::owned::{
    Enablement, OnResponse, Tip, described, field_box, painted, tip_describes, tipped_whole,
};
use crate::{Builder, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Node {
    Readout,
    Field,
}

pub(in crate::owned) type Format = Rc<dyn Fn(f64) -> String>;
pub(in crate::owned) type Parse = Rc<dyn Fn(&str) -> Option<f64>>;
pub(in crate::owned) type Set<A> = Rc<dyn Fn(&mut A, f64)>;

/// The narrowest the readout's field opens, so a short value can be typed
/// over.
const MIN_FIELD: f32 = 48.0;

/// A readout as its owner configured it.
pub(in crate::owned) struct Readout {
    pub(in crate::owned) value: f64,
    pub(in crate::owned) format: Option<Format>,
    /// Editable when it has one.
    pub(in crate::owned) parse: Option<Parse>,
    pub(in crate::owned) step: Step,
    /// Where the step's lattice starts: a slider's track start.
    pub(in crate::owned) origin: f64,
    /// The range a typed value reaches.
    pub(in crate::owned) reach: ValueRange,
    pub(in crate::owned) name: String,
    pub(in crate::owned) enablement: Enablement,
    /// Its text classes: size and, for a slider's, colour.
    pub(in crate::owned) text: Tw,
    pub(in crate::owned) paint: Option<Colors>,
    pub(in crate::owned) size: Size,
    /// The tooltip that describes its button, where the readout is the
    /// whole control.
    pub(in crate::owned) describes: Tip,
    /// When its button takes focus.
    pub(in crate::owned) autofocus: Autofocus,
}

impl Readout {
    fn shown(&self) -> String {
        self.format.as_ref().map_or_else(
            || format!("{:.*}", self.step.decimals(), self.value),
            |format| format(self.value),
        )
    }

    /// Builds the readout under `Node::Readout`: its text, a button that
    /// opens it while it is editable, or the field it opens into.
    pub(in crate::owned) fn show<A: 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        set: Set<A>,
        edits: Edits<A>,
        responses: OnResponse<A>,
    ) {
        let shown = self.shown();
        let Self {
            value,
            parse,
            step,
            origin,
            reach,
            name,
            enablement,
            text,
            paint,
            size,
            describes,
            autofocus,
            ..
        } = self;
        let style = b.style().clone();
        let classes = match paint {
            Some(colors) => painted(text, colors),
            None => text,
        }
        .tabular_nums()
        .shrink_0();
        let Some(parse) = parse else {
            b.div_id(Node::Readout, classes, |b| b.text(tw(), shown));
            return;
        };
        let editing = b.id_of(Node::Readout).with("editing");
        let session = Session::of(editing);
        let accept = move |text: &str| {
            parse(text).map(|number| step.snap_from(origin, reach.clamp(number), reach))
        };
        if session.typed(b.ctx()).is_none() {
            let classes = enablement.apply(classes.sense_click());
            let label = format!("Edit {name}");
            let current = shown.clone();
            b.div_id(Node::Readout, classes, |b| b.text(tw(), shown))
                .announces(Role::Button)
                .open_focus(autofocus.open_focus())
                .draw(move |app, ui| {
                    let response = described(ui, Role::Button, None, label.trim(), &None);
                    tip_describes(ui, &response, &describes);
                    autofocus.apply(ui.ctx(), &response);
                    responses.run(app, &response);
                    let opened = response.clicked().then(|| {
                        let width = response.rect.width();
                        let opening = opening_of(ui, &response, &current);
                        session.open(app, ui.ctx(), &current, &edits);
                        ui.data_mut(|data| {
                            data.insert_temp(editing.with("width"), width);
                            data.insert_temp(editing.with("opening"), opening);
                        });
                        opening
                    });
                    let decision = match opened {
                        Some(Opening::All) => "open, select all",
                        Some(Opening::Caret(_) | Opening::Kept) => "open, caret",
                        None => "none",
                    };
                    input_trace::pointer(
                        ui,
                        editing,
                        label.trim(),
                        Shown::Text,
                        &response,
                        decision,
                    );
                });
            return;
        }
        let frame = crate::owned::input_frame(&style, size);
        let field = Field::singleline("").sized(size).inset();
        let measured = field.clone();
        let slot = b.slot(&Node::Readout);
        let shown_width = b
            .ctx()
            .data(|data| data.get_temp::<f32>(editing.with("width")))
            .unwrap_or(0.0);
        let outset = frame.padding.x + frame.border_width;
        let width = shown_width.max(MIN_FIELD) + 2.0 * outset;
        // The box grows past the readout's edge so its text stays where the readout drew it.
        field_box::Chain::new(b.reborrow(), slot)
            .size(size)
            .classes(
                tw().w_px(width)
                    .mr_px(-outset)
                    .px(0.0)
                    .shrink_0()
                    .tabular_nums(),
            )
            .children(|s| {
                let keys = crate::owned::input::keys(style.input.sizes.key_for(size), false);
                let leaf = s.part(
                    crate::owned::input::GroupPart::Input,
                    &keys,
                    // The field is inset by egui's margin already, so its
                    // text stays where the readout drew it.
                    tw().w_full().h_full().p(0.0).text_right(),
                );
                s.leaf(Node::Field, leaf, move |m| {
                    text_field::measure(m, &measured)
                })
                .draw_widgets(move |app, ui| {
                    let mut text = session.typed(ui.ctx()).unwrap_or_default();
                    let response = text_field::show(ui, &mut text, &field);
                    Autofocus::OnAppear(Select::None).apply(ui.ctx(), &response);
                    let opening = response.gained_focus().then(|| {
                        ui.data(|data| data.get_temp(editing.with("opening")))
                            .unwrap_or(Opening::All)
                    });
                    let decision = text_field::select_number(ui, &response, &text, opening);
                    if input_trace::enabled() {
                        let label = format!("Edit {name}");
                        input_trace::pointer(
                            ui,
                            editing,
                            label.trim(),
                            Shown::Field,
                            &response,
                            decision,
                        );
                    }
                    responses.run(app, &response);
                    if response.changed() {
                        session.keep(ui.ctx(), &text);
                    }
                    if response.lost_focus() {
                        session.close(app, ui, (&text, value), &accept, (&edits, &*set));
                    }
                });
            });
    }
}

/// Where a click on the readout puts the field's caret: where it landed.
/// The keyboard opens it with the whole value selected, as Tab does.
fn opening_of(ui: &Ui, response: &egui::Response, shown: &str) -> Opening {
    let at = response
        .interact_pointer_pos()
        .filter(|_| response.clicked_by(egui::PointerButton::Primary));
    let Some(at) = at else {
        return Opening::All;
    };
    let style = crate::draw::node_text(ui);
    let leaf = crate::measure::TextLeaf::plain(ui.ctx(), shown, &style);
    let galley = ui.ctx().fonts_mut(|fonts| leaf.editing_galley(fonts, None));
    Opening::Caret(galley.cursor_from_pos(at - response.rect.min))
}

/// TODO(prose): a number shown as text, which a click edits when it is
/// [`editable`](Self::editable): the readout of a slider, without the track.
/// Start one with [`Builder::number`] and finish it with
/// [`on_change`](Self::on_change), or [`add`](Self::add) for one that only
/// shows the value.
///
/// It takes the colour of the text around it.
///
/// ```
/// use eggshell_ui::prelude::*;
/// use eggshell_ui::Builder;
///
/// struct App { gain: f64 }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.number("gain", app.gain, -380.0..=380.0)
///         .format(|v| format!("{v:.1} dB"))
///         .editable(|text| text.trim_end_matches("dB").trim().parse().ok())
///         .named("Gain")
///         .on_change(|app, v| app.gain = v);
/// }
/// ```
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its focus and response are those
/// of the button that opens it, or of the field while it is open.
#[must_use = "a number is not added until you call `.on_change(..)` or `.add()`"]
pub struct Chain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    value: f64,
    range: ValueRange,
    step: Step,
    format: Option<Format>,
    parse: Option<Parse>,
    opts: Options,
    size: Option<Size>,
    edits: Edits<A>,
    on_response: OnResponse<A>,
}

impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// TODO(prose): `value` within `range`, shown as text; see [`Chain`].
    pub fn number<K: crate::Key>(
        &mut self,
        key: K,
        value: f64,
        range: impl Into<ValueRange>,
    ) -> Chain<'_, 'f, A> {
        Chain {
            slot: self.slot(&key),
            b: self.reborrow(),
            value,
            range: range.into(),
            step: Step::ONE,
            format: None,
            parse: None,
            opts: Options::new(),
            size: None,
            edits: Edits::default(),
            on_response: OnResponse::default(),
        }
    }
}

impl<A: 'static> Chain<'_, '_, A> {
    /// TODO(prose): the value as `format` writes it; the step's decimals until then.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Some(Rc::new(format));
        self
    }

    /// TODO(prose): a click edits the value as text that `parse` reads. Enter
    /// or leaving the field commits it, clamped to the range and snapped to
    /// the step; Escape reverts it.
    pub fn editable(mut self, parse: impl Fn(&str) -> Option<f64> + 'static) -> Self {
        self.parse = Some(Rc::new(parse));
        self
    }

    /// TODO(prose): the distance between the values a typed one snaps to.
    pub fn step(mut self, step: Step) -> Self {
        self.step = step;
        self
    }

    /// TODO(prose): the size of the field it opens into, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Runs `then` as each edit begins and ends (see [`Edit`]): Began as it
    /// opens, Ended as it closes, the value set between.
    pub fn on_edit(mut self, then: impl Fn(&mut A, Edit) + 'static) -> Self {
        self.edits.set(then);
        self
    }

    /// Adds it. `set` runs with the value typed in it.
    #[track_caller]
    pub fn on_change(self, set: impl Fn(&mut A, f64) + 'static) {
        self.finish(Rc::new(set));
    }

    /// Adds it, showing the value only.
    #[track_caller]
    pub fn add(mut self) {
        self.parse = None;
        self.finish(Rc::new(|_: &mut A, _| {}));
    }

    #[track_caller]
    fn finish(self, set: Set<A>) {
        let Self {
            mut b,
            slot,
            value,
            range,
            step,
            format,
            parse,
            opts,
            size,
            edits,
            on_response,
        } = self;
        let value = range.checked(b.ctx(), slot.id, value);
        let Resolved {
            name,
            classes,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let readout = Readout {
            value,
            format,
            parse,
            step,
            origin: range.start(),
            reach: range,
            name: name.unwrap_or_default(),
            enablement,
            text: tw(),
            paint: None,
            size: b.size_of(size),
            describes: tooltip.clone(),
            autofocus,
        };
        let wrapper = tw().flex().items_center().merge(classes);
        let wrapper = match tooltip {
            Some(_) => wrapper.sense_hover(),
            None => wrapper,
        };
        b.div_id_with(&slot, wrapper, |b| readout.show(b, set, edits, on_response))
            .draw(move |_, ui| tipped_whole(ui, &tooltip));
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
