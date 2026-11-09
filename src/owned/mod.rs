//! Controls built from bullseye nodes instead of wrapped egui widgets
//! (issue #19). A control is an identified node with a sense, `hover`,
//! `active` and `focus` variants and text or paint children, styled from
//! [`Style`] and measured by taffy, so there is no size
//! estimate to drift.
//!
//! Every control sets its accessibility role, label and state, is a Tab stop
//! (Space and Enter activate it), and is drawn in the style's explicit
//! disabled colours when disabled.

mod button;
mod checkbox;
mod number;
mod radio;
mod segmented;
mod select;
mod slider;
mod stepper;
mod switch;

use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

use egui::{Color32, Id, Painter, Pos2, Rect, Shape, Stroke, pos2};

pub use button::Button;
pub use checkbox::Checkbox;
pub use number::Step;
pub use radio::RadioGroup;
pub use segmented::{Cell, Segmented};
pub use select::Select;
pub use slider::Slider;
pub use stepper::Stepper;
pub use switch::Switch;

use crate::contract::{Colors, Look, Style};
use crate::{Builder, NodeUi as _, Tw, tw};

/// Whether a control takes input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Enablement {
    /// Responds to the pointer and the keyboard.
    #[default]
    Enabled,
    /// Inert, and drawn in the style's explicit disabled colours.
    Disabled,
}

/// The classes that paint a box with `colors`. A control's text is not
/// selectable: pressing it operates the control.
fn painted(classes: Tw, colors: Colors) -> Tw {
    classes
        .select_none()
        .bg(colors.fill)
        .border_color(colors.border)
        .text(colors.text)
}

impl Look {
    /// The classes that draw a box of `radius` in this look. A disabled box is
    /// inert and has no variants: its colours are the style's, not an opacity.
    pub(crate) fn classes(
        &self,
        style: &Style,
        radius: u8,
        enablement: Enablement,
        transition: Duration,
    ) -> Tw {
        let base = tw()
            .border(self.border_width)
            .rounded_px(f32::from(radius))
            .sense_click();
        let shadow = |classes: Tw, fade: f32| match self.shadow {
            Some(shadow) => classes.shadow(egui::Shadow {
                color: shadow.color.gamma_multiply(fade),
                ..shadow
            }),
            None => classes,
        };
        let states = self.states;
        match enablement {
            Enablement::Disabled => shadow(painted(base, states.disabled).inert(), 0.5),
            Enablement::Enabled => {
                let ring = self.focus.unwrap_or(style.focus);
                let (rest, focused) = (states.rest, states.focused);
                let classes = painted(base, rest)
                    .hover(|p| {
                        p.bg(states.hover.fill)
                            .border_color(states.hover.border)
                            .text(states.hover.text)
                    })
                    .active(|p| {
                        p.bg(states.pressed.fill)
                            .border_color(states.pressed.border)
                            .text(states.pressed.text)
                    })
                    .focus(|p| {
                        let p = if focused.fill == rest.fill {
                            p
                        } else {
                            p.bg(focused.fill)
                        };
                        let p = if focused.border == rest.border {
                            p
                        } else {
                            p.border_color(focused.border)
                        };
                        let p = if focused.text == rest.text {
                            p
                        } else {
                            p.text(focused.text)
                        };
                        p.ring(ring.width, ring.color)
                    })
                    .transition(transition);
                shadow(classes, 1.0)
            }
        }
    }
}

/// A box that holds a text field, drawn in `look`: it does not sense, and shows
/// the focus ring while a widget inside has focus.
fn field_classes(look: &Look, style: &Style, radius: u8, enablement: Enablement) -> Tw {
    let base = tw().border(look.border_width).rounded_px(f32::from(radius));
    let states = look.states;
    let classes = match enablement {
        Enablement::Disabled => painted(base, states.disabled).inert(),
        Enablement::Enabled => {
            let ring = look.focus.unwrap_or(style.focus);
            painted(base, states.rest).focus_within(|p| {
                p.border_color(states.focused.border)
                    .ring(ring.width, ring.color)
            })
        }
    };
    match look.shadow {
        Some(shadow) => classes.shadow(shadow),
        None => classes,
    }
}

/// lucide's `check`, on its 24 point grid.
const CHECK: [(f32, f32); 3] = [(4.0, 12.0), (9.0, 17.0), (20.0, 6.0)];

/// lucide's `chevron-down`, on its 24 point grid.
const CHEVRON_DOWN: [(f32, f32); 3] = [(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)];

/// A 2-point stroke of `points` on a 24 point grid fitted to `rect`, with
/// round joins, as lucide draws its icons.
fn paint_strokes(painter: &Painter, rect: Rect, color: Color32, points: [(f32, f32); 3]) {
    let scale = rect.width() / 24.0;
    let at = |(x, y): (f32, f32)| -> Pos2 { rect.min + pos2(x, y).to_vec2() * scale };
    let width = 2.0 * scale;
    let points = points.map(at);
    painter.add(Shape::line(points.to_vec(), Stroke::new(width, color)));
    for point in points {
        painter.circle_filled(point, width / 2.0, color);
    }
}

fn paint_check(painter: &Painter, rect: Rect, color: Color32) {
    paint_strokes(painter, rect, color, CHECK);
}

/// Which way a chevron points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chevron {
    Up,
    Down,
}

fn paint_chevron(chevron: Chevron) -> impl Fn(&Painter, Rect, Color32) + 'static {
    move |painter, rect, color| {
        let points = match chevron {
            Chevron::Down => CHEVRON_DOWN,
            Chevron::Up => CHEVRON_DOWN.map(|(x, y)| (x, 24.0 - y)),
        };
        paint_strokes(painter, rect, color, points);
    }
}

/// The classes of the row that holds a checkbox, radio button or switch and
/// its label.
fn choice_row(style: &Style, enablement: Enablement, gap: f32) -> Tw {
    let row = tw()
        .flex()
        .items_center()
        .gap(gap / 4.0)
        .role(crate::TextSpec {
            line_height: style.text.size,
            ..style.text.clone()
        })
        .whitespace_nowrap()
        .select_none();
    match enablement {
        Enablement::Enabled => row,
        Enablement::Disabled => row.inert(),
    }
}

/// The visible label of the control `control`: clicking it runs `on_click`,
/// and assistive technology names the control by it.
fn choice_label<A: 'static, K: Hash + Debug + 'static>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    control: Id,
    text: String,
    enablement: Enablement,
    on_click: impl FnOnce(&mut A) + 'static,
) {
    let label = Style::of(b.ctx()).label;
    let color = match enablement {
        Enablement::Enabled => label.rest,
        Enablement::Disabled => label.disabled,
    }
    .text;
    let name = text.clone();
    b.span(
        key,
        tw().text(color).sense_click().focusable_none(),
        text,
        move |app, ui| {
            if ui.label_for(control, &name) {
                on_click(app);
            }
        },
    );
}
