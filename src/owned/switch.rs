//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Role, Vec2, WidgetInfo};

use super::{Enablement, choice_label, choice_row};
use crate::contract::Size;
use crate::style::Mark;
use crate::{Builder, NodeUi as _, Progress, Style, tw};

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
        on_toggle: fn(&mut A),
    ) {
        let style = Style::of(b.ctx());
        let switch = &style.switch;
        let metrics = *switch.sizes.get(self.size);
        let track = self.track.unwrap_or(metrics.track);
        let on = self.mark == Mark::On;
        let enablement = self.enablement;
        let slide = switch.slide.as_secs_f32();
        let thumb_scale = track.y / metrics.track.y;
        let (thumb_off, thumb_on) = (
            metrics.thumb_off * thumb_scale,
            metrics.thumb_on * thumb_scale,
        );
        let name = self.name.unwrap_or_default();
        b.scope(key, |b| {
            let control = b.id_of(Part::Track);
            let tint = b
                .ctx()
                .animate_bool_with_time(control.with("slide"), on, slide);
            let look = switch.off.towards(switch.on, Progress::new(tint));
            let thumb_color = match enablement {
                Enablement::Enabled => look.states.rest.text,
                Enablement::Disabled => look.states.disabled.text,
            };
            let classes = look
                .classes(&style, u8::MAX, enablement, switch.transition)
                .flex()
                .items_center()
                .shrink_0()
                .w_px(track.x)
                .h_px(track.y);
            b.div(choice_row(&style, enablement, switch.gap), |b| {
                b.div_id(
                    Part::Track,
                    classes,
                    move |app, ui| {
                        let response = ui.node_response();
                        if response.clicked() {
                            on_toggle(app);
                        }
                        response.widget_info(|| {
                            WidgetInfo::selected(Role::Switch, ui.is_enabled(), on, &name)
                        });
                    },
                    |b| {
                        b.paint(tw().w_full().h_full(), move |painter, rect, _| {
                            let t = painter.ctx().animate_bool_with_time(
                                control.with("slide"),
                                on,
                                slide,
                            );
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
