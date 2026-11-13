//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Role, Vec2};

use super::Mark;
use super::{Enablement, OnResponse, Tip, Unset, choice_label, choice_row, described, flip};
use crate::builder::Slot;
use crate::contract::Size;
use crate::transition;
use crate::{Builder, Easing, Progress, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Track,
    Label,
}

/// A switch drawn from the [`Style`], with an optional label. Start one with
/// [`Builder::switch`] and finish it with [`on_change`](Self::on_change).
///
/// It shows `current` and is on when that is its `on` value: `true` for a
/// `bool`, or the second of [`values`](Self::values). A click passes the other
/// value to `on_change`.
///
/// The thumb scales with the track's height and travels its width, so
/// [`track`](Self::track) can resize the track of a size without redrawing
/// anything.
#[must_use = "a switch is not added until you call `.on_change(..)`"]
pub struct SwitchChain<'a, 'f, A, T, V = Unset> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: Option<String>,
    name: Option<String>,
    tooltip: Tip,
    current: T,
    values: V,
    size: Size,
    track: Option<Vec2>,
    classes: Tw,
    enablement: Enablement,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> SwitchChain<'a, 'f, A, T> {
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        label: Option<String>,
        current: T,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            label,
            name: None,
            tooltip: None,
            current,
            values: Unset,
            size: Size::DEFAULT,
            track: None,
            classes: tw(),
            enablement: Enablement::Enabled,
            on_response: OnResponse::default(),
        }
    }

    /// The values the switch moves between: `off` when off and `on` when on.
    pub fn values(self, off: T, on: T) -> SwitchChain<'a, 'f, A, T, [T; 2]> {
        SwitchChain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            name: self.name,
            tooltip: self.tooltip,
            current: self.current,
            values: [off, on],
            size: self.size,
            track: self.track,
            classes: self.classes,
            enablement: self.enablement,
            on_response: self.on_response,
        }
    }
}

impl<'a, 'f, A: 'static, T, V> SwitchChain<'a, 'f, A, T, V> {
    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Draws the switch at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Overrides the track's width and height, in points.
    pub fn track(mut self, track: Vec2) -> Self {
        self.track = Some(track);
        self
    }

    /// Classes merged over the styled ones of the whole row, such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control is
    /// disabled, as an HTML `title` is. Rich text is shown as plain text. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Whether the switch takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Runs `then` with the track's response each frame: the escape hatch for
    /// what the builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }
}

impl<'a, 'f, A: 'static> SwitchChain<'a, 'f, A, bool> {
    /// Adds the switch. `on_change` runs with the new value when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> SwitchChain<'a, 'f, A, T, [T; 2]> {
    /// Adds the switch. `on_change` runs with the new value when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            name,
            tooltip,
            current,
            values: [off, on_value],
            size,
            track,
            classes: caller,
            enablement,
            on_response,
        } = self;
        let enablement = b.enablement_of(enablement);
        let (mark, toggled) = flip([off, on_value], current);
        let on_toggle = Rc::new(move |app: &mut A| on_change(app, toggled));
        let style = Style::of(b.ctx());
        let switch = &style.switch;
        let metrics = *switch.sizes.get(size);
        let track = track.unwrap_or(metrics.track);
        let on = mark == Mark::On;
        let slide = switch.slide;
        let target = if on { Progress::END } else { Progress::START };
        let thumb_scale = track.y / metrics.track.y;
        let (thumb_off, thumb_on) = (
            metrics.thumb_off * thumb_scale,
            metrics.thumb_on * thumb_scale,
        );
        let name = name.unwrap_or_default();
        b.claim(&slot);
        b.scope_with(slot.id, &slot.label, |b| {
            let control = b.id_of(Part::Track);
            let tint = transition::animate(
                b.ctx(),
                control.with("slide"),
                target,
                slide,
                Easing::Default,
            );
            let look = switch.off.towards(switch.on, tint);
            let thumb_color = enablement.colors(look.states, &style).text;
            let classes = look
                .classes(
                    switch.border_width,
                    &style,
                    u8::MAX,
                    enablement,
                    switch.transition,
                )
                .flex()
                .items_center()
                .shrink_0()
                .w_px(track.x)
                .h_px(track.y);
            let row = choice_row(&style, &switch.text, enablement, switch.gap).merge(caller);
            let on_press = on_toggle.clone();
            b.div(row, |b| {
                b.div_id(Part::Track, classes, |b| {
                    b.paint(tw().w_full().h_full(), move |painter, rect, _| {
                        let t = f32::from(transition::animate(
                            painter.ctx(),
                            control.with("slide"),
                            target,
                            slide,
                            Easing::Default,
                        ));
                        let diameter = thumb_off + (thumb_on - thumb_off) * t;
                        let inset = rect.height() / 2.0;
                        let x = egui::lerp(rect.left() + inset..=rect.right() - inset, t);
                        painter.circle_filled(
                            egui::pos2(x, rect.center().y),
                            diameter / 2.0,
                            thumb_color,
                        );
                    });
                })
                .draw(move |app, ui| {
                    let response = described(ui, Role::Switch, Some(mark), &name, &tooltip);
                    if response.clicked() {
                        on_press(app);
                    }
                    on_response.run(app, &response);
                });
                if let Some(label) = label {
                    choice_label(b, Part::Label, control, label, enablement, move |app| {
                        on_toggle(app)
                    });
                }
            });
        });
    }
}
