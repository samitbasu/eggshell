//! The owned slider: a track with a range and a thumb, which a pointer drags
//! and the keyboard steps.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Key, Rect, Role, Ui, WidgetInfo};

use super::number::{self, Edit, Edits, Step, ValueRange};
use super::{
    Enablement, FieldBoxChain, Gate, IntoTip, OnResponse, described, options, painted, text_sized,
    tip_describes, tipped_whole,
};
use crate::builder::Slot;
use crate::contract::{Colors, Size};
use crate::finite::clamp;
use crate::focus::{Autofocus, Select};
use crate::input_trace::{self, Shown};
use crate::prim::text_field::{self, Field, Opening};
use crate::{Builder, NodeUi as _, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Header,
    Label,
    Readout,
    Field,
    Track,
    Thumb,
}

/// A horizontal slider drawn from the [`Style`](crate::Style). Start one with
/// [`Builder::slider`] and finish it with [`on_change`](Self::on_change).
///
/// Pressing the track jumps the thumb there, and dragging the track or the
/// thumb moves it. With the thumb focused, the arrow keys move it a step and
/// PageUp and PageDown ten, and Home and End go to the ends of the range.
///
/// ```html
/// <span class="relative flex w-full items-center h-4">
///   <span class="h-1.5 w-full rounded-full bg-muted overflow-hidden">
///     <span class="h-full rounded-full bg-primary" style="width: 40%"></span>
///   </span>
///   <span role="slider" class="absolute size-4 rounded-full border border-primary bg-background shadow-sm
///     focus-visible:ring-[3px] focus-visible:ring-ring/50"></span>
/// </span>
/// ```
#[must_use = "a slider is not added until you call `.on_change(..)`"]
pub struct SliderChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    value: f64,
    range: ValueRange,
    step: Step,
    name: Option<String>,
    gate: Gate,
    classes: Tw,
    enablement: Enablement,
    size: Option<Size>,
    header: Header,
    on_response: OnResponse<A>,
    edits: Edits<A>,
}

type Format = Rc<dyn Fn(f64) -> String>;
type Parse = Rc<dyn Fn(&str) -> Option<f64>>;
type Set<A> = Rc<dyn Fn(&mut A, f64)>;

/// What a slider says above its track.
#[derive(Clone, Default)]
struct Header {
    label: Option<String>,
    readout: Option<Format>,
    parse: Option<Parse>,
}

/// What a draw closure needs to turn the pointer and the keys into a value.
#[derive(Clone, Debug)]
struct Scale {
    value: f64,
    range: ValueRange,
    step: Step,
    thumb: f32,
}

impl Scale {
    /// The value under horizontal position `x` of a track occupying `track`;
    /// the thumb's centre travels the track less one thumb.
    fn at(&self, x: f32, track: Rect) -> f64 {
        let (low, high) = (self.range.start(), self.range.end());
        let travel = track.width() - self.thumb;
        if travel <= 0.0 {
            return low;
        }
        let along = clamp((x - track.left() - self.thumb / 2.0) / travel, 0.0, 1.0);
        self.step
            .snap(low + f64::from(along) * (high - low), self.range)
    }

    /// The value after the keys pressed this frame, if any.
    fn after_keys(&self, ui: &Ui) -> Option<f64> {
        number::hold_arrows(ui);
        let ends = [
            (Key::Home, self.range.start()),
            (Key::End, self.range.end()),
        ];
        let end = ends.into_iter().fold(None, |end, (key, value)| {
            number::pressed(ui, key).then_some(value).or(end)
        });
        let steps = number::steps_pressed(ui);
        match (end, steps) {
            (Some(value), _) => Some(value),
            (None, 0) => None,
            (None, steps) => Some(self.step.moved(self.value, steps, self.range)),
        }
    }
}

/// The classes of a track `height` thick, filled with `colors`.
pub(super) fn track_classes((height, radius): (f32, u8), colors: Colors) -> Tw {
    painted(
        tw().w_full()
            .h_px(height)
            .rounded_px(f32::from(radius))
            .overflow_hidden(),
        colors,
    )
}

/// The classes of a track and of the part of it that is filled, up to
/// `fraction`, with the colours of `track` and `range`.
fn track_and_range(shape: (f32, u8), (track, range): (Colors, Colors), fraction: f32) -> (Tw, Tw) {
    let range = painted(
        tw().h_full()
            .w_frac(fraction, 1.0)
            .rounded_px(f32::from(shape.1)),
        range,
    );
    (track_classes(shape, track), range)
}

impl<'a, 'f, A: 'static> SliderChain<'a, 'f, A> {
    /// A slider at `value` within `range`, moving by [`Step::ONE`].
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        value: f64,
        range: ValueRange,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            value,
            range,
            step: Step::ONE,
            name: None,
            gate: Gate::default(),
            classes: tw(),
            enablement: Enablement::Enabled,
            size: None,
            header: Header::default(),
            on_response: OnResponse::default(),
            edits: Edits::default(),
        }
    }

    /// The distance between the values the slider takes.
    pub fn step(mut self, step: Step) -> Self {
        self.step = step;
        self
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the whole slider (its hit area, which is as wide as the space it is given), such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control is
    /// disabled, as an HTML `title` is. Rich text keeps egui's formatting. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.gate.tooltip(text);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box, such as text and a [`kbd`](crate::Builder::kbd) chip.
    /// The content takes no input, and its text describes the control to
    /// assistive technology.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut crate::Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.gate.tooltip_with(build);
        self
    }

    /// Runs `then` with the thumb's response each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Runs `then` as each edit begins and ends (see [`Edit`]): once for a
    /// press on the thumb or the track, however far it drags, and once for
    /// each key press.
    pub fn on_edit(mut self, then: impl Fn(&mut A, Edit) + 'static) -> Self {
        self.edits.set(then);
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): `label` on the left of a row above the track.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.header.label = Some(label.into());
        self
    }

    /// TODO(prose): the value as `format` writes it, on the right of the row above the track.
    pub fn readout(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.header.readout = Some(Rc::new(format));
        self
    }

    /// TODO(prose): a click on the readout edits the value as text that `parse` reads.
    pub fn editable(mut self, parse: impl Fn(&str) -> Option<f64> + 'static) -> Self {
        self.header.parse = Some(Rc::new(parse));
        self
    }

    /// Whether the slider takes input.
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

    /// Adds the slider, as wide as the space it is given. `set` runs with the
    /// new value when the pointer or the keyboard changes it.
    #[track_caller]
    pub fn on_change(self, set: impl Fn(&mut A, f64) + 'static) {
        let set = Rc::new(set);
        let Self {
            mut b,
            slot,
            value,
            range,
            step,
            name,
            gate,
            classes: caller,
            enablement,
            size,
            header,
            on_response,
            edits,
        } = self;
        let value = range.checked(b.ctx(), slot.id, value);
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let style = b.style().clone();
        let slider = &style.slider;
        let metrics = *slider.sizes.lookup(b.ctx(), b.size_of(size));
        let (low, high) = (range.start(), range.end());
        let fraction = if high > low {
            clamp((value - low) / (high - low), 0.0, 1.0) as f32
        } else {
            0.0
        };
        let thumb = metrics.thumb_size;
        let hit = tw()
            .relative()
            .flex()
            .items_center()
            .w_full()
            .h_px(thumb)
            .sense_click_drag()
            .focusable_none();
        let hit = enablement.apply(hit, &style);
        let (bar, filled) = track_and_range(
            (metrics.track_height, metrics.radius),
            (
                enablement.colors(slider.track, &style),
                enablement.colors(slider.range, &style),
            ),
            fraction,
        );
        let travel = tw().absolute().left_px(0.0).right_px(thumb).inset_y(0.0);
        let knob = slider
            .thumb
            .classes(&style, metrics.radius, enablement, slider.transition)
            .absolute()
            .top_px(0.0)
            .left_frac(fraction, 1.0)
            .size_px(thumb)
            .sense_click_drag();
        let scale = Scale {
            value,
            range,
            step,
            thumb,
        };
        let name = name.or_else(|| header.label.clone()).unwrap_or_default();
        let (track_tip, thumb_tip) = (tooltip.clone(), tooltip);
        let shown = header.is_shown();
        let (hit, caller) = if shown {
            (hit, Some(caller))
        } else {
            (hit.merge(caller), None)
        };
        let row = Row {
            header,
            value: scale.value,
            step,
            range: scale.range,
            name: name.clone(),
            enablement,
            colors: (
                enablement.colors(slider.label, &style),
                enablement.colors(slider.readout, &style),
            ),
            text: text_sized(&b, metrics.text),
            size: b.size_of(size),
        };
        let boxed = caller.map(|caller| {
            enablement
                .fade(
                    tw().flex()
                        .flex_col()
                        .gap(metrics.header_gap / 4.0)
                        .w_full(),
                    &style,
                )
                .merge(caller)
        });
        b.claim(&slot);
        b.scope_with(&slot, |b| {
            let on_track = scale.clone();
            let on_thumb = set.clone();
            let thumb_edits = edits.clone();
            let set_readout = set.clone();
            let readout_edits = edits.clone();
            let track_node = |b: &mut Builder<'_, '_, A>| {
                let track = b.id_of(Part::Track);
                b.div_id(Part::Track, hit, |b| {
                    b.div(bar, |b| b.div(filled, |_| {}));
                    b.div(travel, |b| {
                        b.div_id(Part::Thumb, knob, |_| {}).draw(move |app, ui| {
                            let response = ui.node_response();
                            tip_describes(ui, &response, &thumb_tip);
                            let track = ui.ctx().read_response(track).map(|r| r.rect);
                            let pointer = response
                                .interact_pointer_pos()
                                .zip(track)
                                .filter(|_| response.is_pointer_button_down_on())
                                .map(|(at, track)| scale.at(at.x, track));
                            let keys = if response.has_focus() {
                                scale.after_keys(ui)
                            } else {
                                None
                            };
                            let changed = |value: Option<f64>| value.filter(|&v| v != scale.value);
                            let down = (response.id, response.is_pointer_button_down_on());
                            thumb_edits.pointer(app, ui, down, |app| {
                                if let Some(value) =
                                    changed(keys.is_none().then_some(pointer).flatten())
                                {
                                    on_thumb(app, value);
                                }
                            });
                            if let Some(value) = changed(keys) {
                                thumb_edits.once(app, |app| on_thumb(app, value));
                            }
                            on_response.run(app, &response);
                            response.widget_info(|| {
                                WidgetInfo::slider(ui.is_enabled(), scale.value, &name)
                            });
                            let (low, high) = (scale.range.start(), scale.range.end());
                            ui.ctx().accesskit_node_builder(response.id, |node| {
                                node.set_min_numeric_value(low);
                                node.set_max_numeric_value(high);
                            });
                        });
                    });
                })
                .draw(move |app, ui| {
                    tipped_whole(ui, &track_tip);
                    let response = ui.node_response();
                    let pressed = response.is_pointer_button_down_on();
                    edits.pointer(app, ui, (response.id, pressed), |app| {
                        if let (true, Some(at)) = (pressed, response.interact_pointer_pos()) {
                            let value = on_track.at(at.x, response.rect);
                            if value != on_track.value {
                                set(app, value);
                            }
                        }
                    });
                });
            };
            match boxed {
                None => track_node(b),
                Some(boxed) => {
                    b.div_id(Part::Box, boxed, |b| {
                        row.show(b, set_readout, readout_edits);
                        track_node(b);
                    });
                }
            }
        });
    }
}

impl Header {
    fn is_shown(&self) -> bool {
        self.label.is_some() || self.readout.is_some() || self.parse.is_some()
    }
}

const MIN_READOUT: f32 = 48.0;

/// The row above the track.
struct Row {
    header: Header,
    value: f64,
    step: Step,
    range: ValueRange,
    name: String,
    enablement: Enablement,
    colors: (Colors, Colors),
    text: Tw,
    size: Size,
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

impl Row {
    fn show<A: 'static>(self, b: &mut Builder<'_, '_, A>, set: Set<A>, edits: Edits<A>) {
        let Self {
            header,
            value,
            step,
            range,
            name,
            enablement,
            colors: (label, readout),
            text,
            size,
        } = self;
        let style = b.style().clone();
        let input = *style.input.sizes.lookup(b.ctx(), size);
        let border = style.input.look.border_width;
        let row = tw().flex().items_center().justify_between().w_full();
        let row = if header.parse.is_some() {
            row.min_h_px(input.height)
        } else {
            row
        };
        let shown = header.readout.as_ref().map_or_else(
            || format!("{value:.*}", step.decimals()),
            |format| format(value),
        );
        let editing = b.id_of(Part::Readout).with("editing");
        let typed = b.ctx().data(|data| data.get_temp::<String>(editing));
        b.div_id(Part::Header, row, |b| {
            match &header.label {
                Some(label_text) => {
                    let classes = painted(text.clone(), label);
                    b.div_id(Part::Label, classes, |b| b.text(tw(), label_text.clone()));
                }
                None => b.div(tw(), |_| {}),
            }
            if header.readout.is_none() && header.parse.is_none() {
                return;
            }
            let classes = painted(text.clone(), readout).tabular_nums().shrink_0();
            let Some(parse) = header.parse else {
                b.div_id(Part::Readout, classes, |b| b.text(tw(), shown));
                return;
            };
            let accept =
                move |text: &str| parse(text).map(|number| step.snap(range.clamp(number), range));
            match typed {
                None => {
                    let classes = enablement.apply(classes.sense_click(), &style);
                    let label = format!("Edit {name}");
                    let current = shown.clone();
                    b.div_id(Part::Readout, classes, |b| b.text(tw(), shown))
                        .announces(Role::Button)
                        .draw(move |_, ui| {
                            let response = described(ui, Role::Button, None, label.trim(), &None);
                            let opened = response.clicked().then(|| {
                                let width = response.rect.width();
                                let opening = opening_of(ui, &response, &current);
                                ui.data_mut(|data| {
                                    data.insert_temp(editing, current.clone());
                                    data.insert_temp(editing.with("opened"), current.clone());
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
                }
                Some(_) => {
                    let field = Field::singleline("").sized(size).inset();
                    let measured = field.clone();
                    let slot = b.slot(&Part::Readout);
                    let shown_width = b
                        .ctx()
                        .data(|data| data.get_temp::<f32>(editing.with("width")))
                        .unwrap_or(0.0);
                    let outset = input.padding.x + border;
                    let width = shown_width.max(MIN_READOUT) + 2.0 * outset;
                    // The box grows past the row's edge so its text stays where the readout drew it.
                    FieldBoxChain::new(b.reborrow(), slot)
                        .size(size)
                        .classes(
                            tw().w_px(width)
                                .mr(-outset / 4.0)
                                .px(0.0)
                                .shrink_0()
                                .tabular_nums(),
                        )
                        .children(|s| {
                            let leaf = tw().w_full().h_full().text_right();
                            s.leaf(Part::Field, leaf, move |m| {
                                text_field::measure(m, &measured)
                            })
                            .draw_widgets(move |app, ui| {
                                let mut text = ui
                                    .data(|data| data.get_temp::<String>(editing))
                                    .unwrap_or_default();
                                let response = text_field::show(ui, &mut text, &field);
                                Autofocus::OnAppear(Select::None).apply(ui.ctx(), &response);
                                let opening = response.gained_focus().then(|| {
                                    ui.data(|data| data.get_temp(editing.with("opening")))
                                        .unwrap_or(Opening::All)
                                });
                                let decision =
                                    text_field::select_number(ui, &response, &text, opening);
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
                                if response.changed() {
                                    ui.data_mut(|data| data.insert_temp(editing, text.clone()));
                                }
                                if response.lost_focus() {
                                    let entered = ui.input(|input| input.key_pressed(Key::Enter));
                                    let opened = ui.data_mut(|data| {
                                        data.remove::<String>(editing);
                                        data.remove_temp::<String>(editing.with("opened"))
                                    });
                                    let edited = opened.as_ref() != Some(&text);
                                    if let Some(value) = accept(&text).filter(|_| entered && edited)
                                    {
                                        edits.once(app, |app| set(app, value));
                                    }
                                }
                            });
                        });
                }
            }
        });
    }
}
