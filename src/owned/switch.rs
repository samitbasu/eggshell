//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Role, Vec2};

use super::Mark;
use super::{
    Enablement, Gate, IntoTip, OnResponse, Unset, choice_label, choice_row, described, flip,
    options,
};
use crate::builder::Slot;
use crate::contract::Size;
use crate::focus::{Autofocus, Select};
use crate::transition;
use crate::{Builder, Easing, Progress, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Track,
    Label,
}

/// A switch drawn from the [`Style`](crate::Style), with an optional label. Start one with
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
    gate: Gate,
    current: T,
    values: V,
    size: Option<Size>,
    track: Option<Vec2>,
    classes: Tw,
    enablement: Enablement,
    on_response: OnResponse<A>,
    autofocus: Autofocus,
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
            gate: Gate::default(),
            current,
            values: Unset,
            size: None,
            track: None,
            classes: tw(),
            enablement: Enablement::Enabled,
            on_response: OnResponse::default(),
            autofocus: Autofocus::Never,
        }
    }

    /// The values the switch moves between: `off` when off and `on` when on.
    pub fn values(self, off: T, on: T) -> SwitchChain<'a, 'f, A, T, [T; 2]> {
        SwitchChain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            name: self.name,
            gate: self.gate,
            current: self.current,
            values: [off, on],
            size: self.size,
            track: self.track,
            classes: self.classes,
            enablement: self.enablement,
            on_response: self.on_response,
            autofocus: self.autofocus,
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
        self.size = Some(size);
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

    /// Whether the switch takes input.
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

    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    pub fn focus_on_open(mut self) -> Self {
        self.autofocus = Autofocus::OnOverlayOpen;
        self
    }

    /// TODO(prose): takes focus on every frame this is set.
    pub fn focus_now(mut self) -> Self {
        self.autofocus = Autofocus::Now(Select::None);
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
            gate,
            current,
            values: [off, on_value],
            size,
            track,
            classes: caller,
            enablement,
            on_response,
            autofocus,
        } = self;
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let (mark, toggled) = flip([off, on_value], current);
        let on_toggle = Rc::new(move |app: &mut A| on_change(app, toggled));
        let style = b.style().clone();
        let switch = &style.switch;
        let metrics = *switch.sizes.lookup(b.ctx(), b.size_of(size));
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
        b.scope_with(&slot, |b| {
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
            let row = choice_row(
                &style,
                &style.role_sized(&switch.text, metrics.text),
                enablement,
                switch.gap,
            )
            .merge(caller);
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
                .open_focus(autofocus.open_focus())
                .draw(move |app, ui| {
                    let response = described(ui, Role::Switch, Some(mark), &name, &tooltip);
                    autofocus.apply(ui.ctx(), &response);
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
