//! The owned slider: a track with a range and a thumb, which a pointer drags
//! and the keyboard steps.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Context, Key, Rect, Ui, WidgetInfo};

use super::number::readout::{Format, Parse, Readout, Set};
use super::number::{self, Edit, Edits, Step, ValueRange};
use super::options::{Configure, Interactive, Options, Resolved, Respond};
use super::{Enablement, OnResponse, painted, text_sized, tip_describes, tipped_whole};
use crate::builder::Slot;
use crate::contract::{Colors, Size};
use crate::finite::clamp;
use crate::focus::Autofocus;
use crate::sheet::Key as SheetKey;
use crate::{Builder, NodeUi as _, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Node {
    Box,
    Header,
    Label,
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
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its classes are the whole
/// slider's, and its response is the thumb's.
#[must_use = "a slider is not added until you call `.on_change(..)`"]
pub struct Chain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    value: f64,
    range: ValueRange,
    typed: Option<ValueRange>,
    step: Step,
    opts: Options,
    size: Option<Size>,
    header: Header,
    on_response: OnResponse<A>,
    edits: Edits<A>,
}

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
    /// The range a typed value reaches: `range` unless the slider widens it.
    typed: ValueRange,
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
            (None, steps) => {
                let bounds = match self.range.contains(self.value) {
                    true => self.range,
                    false => self.typed,
                };
                let moved = self.value + f64::from(steps) * self.step.size();
                Some(self.step.snap_from(self.range.start(), moved, bounds))
            }
        }
    }
}

/// TODO(prose): a slider's anatomy for the [`Sheet`](crate::sheet::Sheet),
/// as shadcn's `slider.tsx` names it.
///
/// TODO(prose): the keys its rules see: `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the slider: the track and its thumb, the room the
    /// pointer takes.
    Root,
    /// TODO(prose): the bar the thumb moves along.
    Track,
    /// TODO(prose): the filled part of the track, up to the value.
    Range,
    /// TODO(prose): the handle.
    Thumb,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "slider.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "slider",
            Self::Track => "slider-track",
            Self::Range => "slider-range",
            Self::Thumb => "slider-thumb",
        }
    }
}

/// Writes the slider's recorded exception over a preset's look: a disabled
/// slider does not fade at its root, as eggshell fades shape by shape (#479)
/// and the thumb would show the track through it; its parts draw opaque
/// blends instead (#40).
pub(crate) fn write_exceptions(sheet: &mut crate::sheet::Sheet) {
    use crate::contract::States;
    use crate::sheet::When;
    use egui::Color32;
    let disabled = || When::always().aria(crate::Aria::Disabled);
    let faded = |k: &crate::theme::Tokens, fill, border, text| {
        States::still(Colors { fill, border, text }, k.background).disabled
    };
    let clear = Color32::TRANSPARENT;
    sheet.rule_when(Part::Root, disabled(), |t, _| {
        t.opacity(crate::Opacity::new(1.0))
    });
    sheet.rule_when(Part::Track, disabled(), move |t, k| {
        t.bg(faded(k, k.input, clear, k.foreground).fill)
    });
    sheet.rule_when(Part::Range, disabled(), move |t, k| {
        t.bg(faded(k, k.primary, clear, k.primary_foreground).fill)
    });
    sheet.rule_when(Part::Thumb, disabled(), move |t, k| {
        let thumb = faded(k, Color32::WHITE, k.primary, k.foreground);
        t.bg(thumb.fill).border_color(thumb.border)
    });
}

impl<'a, 'f, A: 'static> Chain<'a, 'f, A> {
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
            typed: None,
            step: Step::ONE,
            opts: Options::new(),
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

    /// TODO(prose): the range a value typed in the [`editable`](Self::editable)
    /// readout reaches, wider than the track's: a typed value is clamped to
    /// it, and stays on the track's step lattice. Outside the track the thumb
    /// pins at its end and the arrow keys step from where the value is. A
    /// range that does not hold the track's is widened to it and reported as
    /// [`BadRange`](crate::Problem::BadRange).
    pub fn typed_range(mut self, typed: impl Into<ValueRange>) -> Self {
        self.typed = Some(typed.into());
        self
    }

    /// Adds the slider, as wide as the space it is given. `set` runs with the
    /// new value when the pointer or the keyboard changes it.
    #[track_caller]
    pub fn on_change(self, set: impl Fn(&mut A, f64) + 'static) {
        self.on_change_with(move |app, _, value| set(app, value));
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, set: impl Fn(&mut A, &Context, f64) + 'static) {
        let set: Set<A> = Rc::new(set);
        let Self {
            mut b,
            slot,
            value,
            range,
            typed,
            step,
            opts,
            size,
            header,
            on_response,
            edits,
        } = self;
        let value = range.checked(b.ctx(), slot.id, value);
        let typed = typed_within(b.ctx(), slot.id, range, typed);
        let Resolved {
            name,
            classes: caller,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
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
        let keys: &[SheetKey] = match enablement {
            Enablement::Enabled => &[],
            Enablement::Disabled => &[SheetKey::Aria(crate::Aria::Disabled)],
        };
        // The thickness of the track and the size of the thumb are the
        // size's metrics: shadcn's slider has one size.
        let own = tw()
            .relative()
            .flex()
            .items_center()
            .w_full()
            .h_px(thumb)
            .sense_click_drag()
            .focusable_none();
        let own = match enablement {
            Enablement::Enabled => own,
            Enablement::Disabled => own.inert(),
        };
        let hit = b.part(Part::Root, keys, own);
        let bar = b.part(Part::Track, keys, tw().w_full().h_px(metrics.track_height));
        let filled = b.part(
            Part::Range,
            keys,
            // eggshell clips to a rectangle, where CSS clips the range to the
            // track's rounding: the range rounds itself as the track does.
            tw().h_full()
                .w_frac(fraction, 1.0)
                .rounded_px(f32::from(metrics.radius)),
        );
        let travel = tw().absolute().left_px(0.0).right_px(thumb).inset_y(0.0);
        let knob = b.part(
            Part::Thumb,
            keys,
            tw().absolute()
                .top_px(0.0)
                .left_frac(fraction, 1.0)
                .size_px(thumb)
                .sense_click_drag(),
        );
        let scale = Scale {
            value,
            range,
            typed,
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
            typed,
            name: name.clone(),
            enablement,
            colors: (
                enablement.colors(slider.label),
                enablement.colors(slider.readout),
            ),
            text: text_sized(&b, metrics.text),
            size: b.size_of(size),
        };
        let boxed = caller.map(|caller| {
            tw().flex()
                .flex_col()
                .gap_px(metrics.header_gap)
                .w_full()
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
                let track = b.id_of(Node::Track);
                b.div_id(Node::Track, hit, |b| {
                    b.div(bar, |b| b.div(filled, |_| {}));
                    b.div(travel, |b| {
                        b.div_id(Node::Thumb, knob, |_| {})
                            .open_focus(autofocus.open_focus())
                            .draw(move |app, ui| {
                                let response = ui.node_response();
                                autofocus.apply(ui.ctx(), &response);
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
                                let changed =
                                    |value: Option<f64>| value.filter(|&v| v != scale.value);
                                let down = (response.id, response.is_pointer_button_down_on());
                                thumb_edits.pointer(app, ui, down, |app| {
                                    if let Some(value) =
                                        changed(keys.is_none().then_some(pointer).flatten())
                                    {
                                        on_thumb(app, ui.ctx(), value);
                                    }
                                });
                                if let Some(value) = changed(keys) {
                                    thumb_edits.once(app, |app| on_thumb(app, ui.ctx(), value));
                                }
                                on_response.run(app, &response);
                                response.widget_info(|| {
                                    WidgetInfo::slider(ui.is_enabled(), scale.value, &name)
                                });
                                let (low, high) = (
                                    scale.range.start().min(scale.value),
                                    scale.range.end().max(scale.value),
                                );
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
                                set(app, ui.ctx(), value);
                            }
                        }
                    });
                });
            };
            match boxed {
                None => track_node(b),
                Some(boxed) => {
                    b.div_id(Node::Box, boxed, |b| {
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

/// The row above the track.
struct Row {
    header: Header,
    value: f64,
    step: Step,
    range: ValueRange,
    typed: ValueRange,
    name: String,
    enablement: Enablement,
    colors: (Colors, Colors),
    text: Tw,
    size: Size,
}

impl Row {
    fn show<A: 'static>(self, b: &mut Builder<'_, '_, A>, set: Set<A>, edits: Edits<A>) {
        let Self {
            header,
            value,
            step,
            range,
            typed,
            name,
            enablement,
            colors: (label, readout),
            text,
            size,
        } = self;
        let style = b.style().clone();
        let row = tw().flex().items_center().justify_between().w_full();
        let row = if header.parse.is_some() {
            row.min_h_px(super::input_frame(&style, size).height)
        } else {
            row
        };
        b.div_id(Node::Header, row, |b| {
            match &header.label {
                Some(label_text) => {
                    let classes = painted(text.clone(), label);
                    b.div_id(Node::Label, classes, |b| b.text(tw(), label_text.clone()));
                }
                None => b.div(tw(), |_| {}),
            }
            if header.readout.is_none() && header.parse.is_none() {
                return;
            }
            Readout {
                value,
                format: header.readout,
                parse: header.parse,
                step,
                origin: range.start(),
                reach: typed,
                name,
                enablement,
                text,
                paint: Some(readout),
                size,
                describes: None,
                autofocus: Autofocus::Never,
            }
            .show(b, set, edits, OnResponse::default());
        });
    }
}

/// The range a typed value reaches: `typed`, widened to hold `range` and
/// reported if it did not, or `range` when the slider has none.
#[track_caller]
fn typed_within(
    ctx: &egui::Context,
    subject: egui::Id,
    range: ValueRange,
    typed: Option<ValueRange>,
) -> ValueRange {
    let Some(typed) = typed else {
        return range;
    };
    if !typed.holds(range) {
        crate::report::report(ctx, crate::Problem::BadRange, Some(subject), || {
            format!(
                "the typed range {:?}..={:?} does not hold the track's {:?}..={:?}; \
                 it is widened to hold it",
                typed.start(),
                typed.end(),
                range.start(),
                range.end()
            )
        });
    }
    typed.union(range)
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

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A slider at `value` within `range`. Assistive technology sees a
    /// `Role::Slider`.
    pub fn slider<K: crate::Key>(
        &mut self,
        key: K,
        value: f64,
        range: impl Into<ValueRange>,
    ) -> Chain<'_, 'f, A> {
        Chain::new(self.reborrow(), key, value, range.into())
    }
}
