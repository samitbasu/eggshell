//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Role, Vec2};

use super::Mark;
use super::{Enablement, choice_label, choice_row, described};
use crate::contract::Size;
use crate::transition;
use crate::{Builder, Easing, Progress, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Track,
    Label,
}

/// A switch drawn from the [`Style`], with an optional label.
///
/// The thumb scales with the track's height and travels its width, so
/// [`track`](Self::track) can resize the track of a size without redrawing
/// anything.
#[derive(Clone, Debug)]
pub struct Switch {
    label: Option<String>,
    name: Option<String>,
    mark: Mark,
    size: Size,
    track: Option<Vec2>,
    classes: Tw,
    enablement: Enablement,
}

impl Switch {
    /// A switch labelled `label`, on when `mark` is.
    pub fn new(label: impl Into<String>, mark: Mark) -> Self {
        Self {
            label: Some(label.into()),
            name: None,
            mark,
            size: Size::DEFAULT,
            track: None,
            classes: tw(),
            enablement: Enablement::Enabled,
        }
    }

    /// A switch without a label.
    pub fn unlabelled(mark: Mark) -> Self {
        Self {
            label: None,
            ..Self::new("", mark)
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

    /// Whether the switch takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the switch to `b` under `key`. `on_toggle` runs when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        on_toggle: impl Fn(&mut A) + Copy + 'static,
    ) {
        let style = Style::of(b.ctx());
        let switch = &style.switch;
        let metrics = *switch.sizes.get(self.size);
        let track = self.track.unwrap_or(metrics.track);
        let mark = self.mark;
        let on = mark == Mark::On;
        let enablement = self.enablement;
        let slide = switch.slide;
        let target = if on { Progress::END } else { Progress::START };
        let thumb_scale = track.y / metrics.track.y;
        let (thumb_off, thumb_on) = (
            metrics.thumb_off * thumb_scale,
            metrics.thumb_on * thumb_scale,
        );
        let name = self.name.unwrap_or_default();
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
            let row = choice_row(&style, &switch.text, enablement, switch.gap).merge(self.classes);
            b.div(row, |b| {
                b.div_id(
                    Part::Track,
                    classes,
                    move |app, ui| {
                        if described(ui, Role::Switch, Some(mark), &name).clicked() {
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
                if let Some(label) = self.label {
                    choice_label(b, Part::Label, control, label, enablement, on_toggle);
                }
            });
        });
    }
}
