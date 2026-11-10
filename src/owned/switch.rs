//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Role, Vec2};

use super::Mark;
use super::{Enablement, Tip, choice_label, choice_row, described};
use crate::contract::Size;
use crate::transition;
use crate::{Builder, Easing, Progress, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Track,
    Label,
}

/// A switch drawn from the [`Style`], with an optional label. Start one with
/// [`Builder::switch`] and finish it with [`bind`](Self::bind) or
/// [`on_toggle`](Self::on_toggle).
///
/// The thumb scales with the track's height and travels its width, so
/// [`track`](Self::track) can resize the track of a size without redrawing
/// anything.
#[must_use = "a switch is not added until you call `.bind(..)` or `.on_toggle(..)`"]
pub struct SwitchBuilder<'a, 'b, 'f, A, K> {
    b: &'a mut Builder<'b, 'f, A>,
    key: K,
    label: Option<String>,
    name: Option<String>,
    tooltip: Tip,
    mark: Mark,
    size: Size,
    track: Option<Vec2>,
    classes: Tw,
    enablement: Enablement,
}

impl<'a, 'b, 'f, A: 'static, K: crate::Key> SwitchBuilder<'a, 'b, 'f, A, K> {
    pub(super) fn new(
        b: &'a mut Builder<'b, 'f, A>,
        key: K,
        label: Option<String>,
        mark: Mark,
    ) -> Self {
        Self {
            b,
            key,
            label,
            name: None,
            tooltip: None,
            mark,
            size: Size::DEFAULT,
            track: None,
            classes: tw(),
            enablement: Enablement::Enabled,
        }
    }

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

    /// Shown while the pointer hovers the control, as egui's `on_hover_text`
    /// shows it: with egui's timing and style, and also when the control is
    /// disabled, as an HTML `title` is. It adds to the control's accessible
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

    /// Adds the switch, which flips the bool that `lens` picks out of the
    /// application's state when it is toggled.
    pub fn bind(self, lens: impl Fn(&mut A) -> &mut bool + Copy + 'static) {
        self.on_toggle(move |app| {
            let on = lens(app);
            *on = !*on;
        });
    }

    /// Adds the switch. `on_toggle` runs when the track or the label is
    /// clicked, or the track is activated with Space or Enter.
    pub fn on_toggle(self, on_toggle: impl Fn(&mut A) + Copy + 'static) {
        let Self {
            b,
            key,
            label,
            name,
            tooltip,
            mark,
            size,
            track,
            classes: caller,
            enablement,
        } = self;
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
        b.scope(key, |b| {
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
            b.div(row, |b| {
                b.div_id(
                    Part::Track,
                    classes,
                    move |app, ui| {
                        if described(ui, Role::Switch, Some(mark), &name, &tooltip).clicked() {
                            on_toggle(app);
                        }
                    },
                    |b| {
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
                    },
                );
                if let Some(label) = label {
                    choice_label(b, Part::Label, control, label, enablement, on_toggle);
                }
            });
        });
    }
}
