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
use std::ops::RangeInclusive;
use std::time::Duration;

use egui::{
    Color32, Id, Painter, Pos2, Rect, Response, Role, Shape, Stroke, Ui, WidgetInfo, WidgetText,
    pos2,
};

pub use button::{ButtonBuilder, Face};
pub use checkbox::CheckboxBuilder;
pub use number::Step;
pub use radio::RadioGroupBuilder;
pub use segmented::{Cell, SegmentedBuilder};
pub use select::SelectBuilder;
pub use slider::SliderBuilder;
pub use stepper::StepperBuilder;
pub use switch::SwitchBuilder;

use crate::contract::{Colors, Disabled, Look, Paint, States, Style};
use crate::{Builder, NodeUi as _, TextSpec, Tw, tw};

/// Whether a checkbox is ticked, a radio button chosen or a selectable
/// button selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Unticked, unchosen or unselected.
    Off,
    /// Ticked, chosen or selected.
    On,
}

impl From<bool> for Mark {
    fn from(on: bool) -> Self {
        if on { Self::On } else { Self::Off }
    }
}

/// Whether a control takes input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Enablement {
    /// Responds to the pointer and the keyboard.
    #[default]
    Enabled,
    /// Inert, and drawn in the style's explicit disabled colours.
    Disabled,
}

impl Enablement {
    /// `classes`, inert when disabled and, if the style fades disabled
    /// controls, faded. Apply it to the root of a control: what is inside
    /// follows the root's opacity.
    fn apply(self, classes: Tw, style: &Style) -> Tw {
        match self {
            Self::Enabled => classes,
            Self::Disabled => self.fade(classes.inert(), style),
        }
    }

    /// `classes` at the opacity the style gives a disabled control, if it
    /// fades them.
    fn fade(self, classes: Tw, style: &Style) -> Tw {
        match (self, style.disabled) {
            (Self::Disabled, Disabled::Faded(opacity)) => classes.opacity(opacity),
            _ => classes,
        }
    }

    /// The colours of `states` at rest, or the disabled ones if the style
    /// has them (a style that fades has the rest colours, faded by the root).
    fn colors(self, states: States, style: &Style) -> Colors {
        match (self, style.disabled) {
            (Self::Disabled, Disabled::Colors) => states.disabled,
            _ => states.rest,
        }
    }
}

/// `classes` with their content centred in both directions.
fn centered(classes: Tw) -> Tw {
    classes.flex().items_center().justify_center()
}

/// A control's tooltip, if it has one.
type Tip = Option<WidgetText>;

/// `response` with `tip` shown on hover, also when the control is disabled.
fn tipped(response: Response, tip: &Tip) -> Response {
    match tip {
        Some(tip) => response
            .on_hover_text(tip.clone())
            .on_disabled_hover_text(tip.clone()),
        None => response,
    }
}

/// The node's response, described to assistive technology as a `role` named
/// `name`, and as on or off when it is `selected`, with the tooltip `tip`.
fn described(ui: &Ui, role: Role, selected: Option<Mark>, name: &str, tip: &Tip) -> Response {
    let response = tipped(ui.node_response(), tip);
    response.widget_info(|| match selected {
        Some(mark) => WidgetInfo::selected(role, ui.is_enabled(), mark == Mark::On, name),
        None => WidgetInfo::labeled(role, ui.is_enabled(), name),
    });
    response
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
    /// The classes that draw a box of `radius` in this look.
    pub(crate) fn classes(
        &self,
        style: &Style,
        radius: u8,
        enablement: Enablement,
        transition: Duration,
    ) -> Tw {
        self.paint
            .classes(self.border_width, style, radius, enablement, transition)
    }
}

impl Paint {
    /// The classes that draw a box of `radius` with a border `border_width`
    /// wide in this paint. A disabled box is inert and has no variants: its
    /// colours are the style's, not an opacity.
    pub(crate) fn classes(
        &self,
        border_width: f32,
        style: &Style,
        radius: u8,
        enablement: Enablement,
        transition: Duration,
    ) -> Tw {
        let base = tw()
            .border(border_width)
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
            Enablement::Disabled => {
                let fade = match style.disabled {
                    Disabled::Colors => 0.5,
                    Disabled::Faded(_) => 1.0,
                };
                shadow(
                    painted(base, enablement.colors(states, style)).inert(),
                    fade,
                )
            }
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

impl Paint {
    /// The classes of a checkbox box or radio circle: a `size` square that
    /// keeps its size and centres what it holds.
    fn indicator(
        &self,
        border_width: f32,
        style: &Style,
        radius: u8,
        enablement: Enablement,
        transition: Duration,
        size: f32,
    ) -> Tw {
        centered(self.classes(border_width, style, radius, enablement, transition))
            .shrink_0()
            .size_px(size)
    }
}

/// A box that holds a text field, drawn in `look`: it does not sense, and shows
/// the focus ring while a widget inside has focus.
fn field_classes(look: &Look, style: &Style, radius: u8, enablement: Enablement) -> Tw {
    let base = tw().border(look.border_width).rounded_px(f32::from(radius));
    let (states, shadow, focus) = (look.paint.states, look.paint.shadow, look.paint.focus);
    let classes = match enablement {
        Enablement::Disabled => painted(base, enablement.colors(states, style)).inert(),
        Enablement::Enabled => {
            let ring = focus.unwrap_or(style.focus);
            painted(base, states.rest).focus_within(|p| {
                p.border_color(states.focused.border)
                    .ring(ring.width, ring.color)
            })
        }
    };
    match shadow {
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
fn choice_row(style: &Style, text: &Option<TextSpec>, enablement: Enablement, gap: f32) -> Tw {
    let text = style.role(text);
    let row = tw()
        .flex()
        .items_center()
        .gap(gap / 4.0)
        .role(TextSpec {
            line_height: text.size,
            ..text.clone()
        })
        .whitespace_nowrap()
        .select_none();
    enablement.apply(row, style)
}

/// The visible label of the control `control`: clicking it runs `on_click`,
/// and assistive technology names the control by it.
fn choice_label<A: 'static, K: crate::Key>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    control: Id,
    text: String,
    enablement: Enablement,
    on_click: impl FnOnce(&mut A) + 'static,
) {
    let style = Style::of(b.ctx());
    let color = enablement.colors(style.label, &style).text;
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

/// The owned controls, each shown under a key like [`div_id`](Builder::div_id)
/// and [`widget`](Builder::widget).
///
/// A control starts from the builder and ends in a terminating call that adds
/// it and says what the user's input changes in the application's state:
///
/// ```text
/// b.button(Part::Save, "Save").variant(PRIMARY).on_click(|s| s.saves += 1);
/// b.checkbox(Part::Terms, "I agree", s.terms).bind(|s| &mut s.terms);
/// ```
///
/// Configuration comes before the terminating call. A control that is
/// started and dropped without one is not added, and the compiler warns.
/// The terminators are `on_click` for a button, `bind` (or `on_toggle`) for a
/// checkbox and a switch, and `on_change` for the controls that choose a
/// value.
impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// A button showing `face`: a label, or an [`Icon`](crate::Icon) alone.
    pub fn button<K: crate::Key>(
        &mut self,
        key: K,
        face: impl Into<Face>,
    ) -> ButtonBuilder<'_, 'b, 'f, A, K> {
        ButtonBuilder::new(self, key, face.into())
    }

    /// A checkbox labelled `label`, ticked when `mark` is on.
    pub fn checkbox<K: crate::Key>(
        &mut self,
        key: K,
        label: impl Into<String>,
        mark: impl Into<Mark>,
    ) -> CheckboxBuilder<'_, 'b, 'f, A, K> {
        CheckboxBuilder::new(self, key, Some(label.into()), mark.into())
    }

    /// A checkbox without a label.
    pub fn checkbox_unlabelled<K: crate::Key>(
        &mut self,
        key: K,
        mark: impl Into<Mark>,
    ) -> CheckboxBuilder<'_, 'b, 'f, A, K> {
        CheckboxBuilder::new(self, key, None, mark.into())
    }

    /// A switch labelled `label`, on when `mark` is.
    pub fn switch<K: crate::Key>(
        &mut self,
        key: K,
        label: impl Into<String>,
        mark: impl Into<Mark>,
    ) -> SwitchBuilder<'_, 'b, 'f, A, K> {
        SwitchBuilder::new(self, key, Some(label.into()), mark.into())
    }

    /// A switch without a label.
    pub fn switch_unlabelled<K: crate::Key>(
        &mut self,
        key: K,
        mark: impl Into<Mark>,
    ) -> SwitchBuilder<'_, 'b, 'f, A, K> {
        SwitchBuilder::new(self, key, None, mark.into())
    }

    /// Radio buttons for `(value, label)` options, of which `current` is
    /// chosen. Values must be distinct.
    pub fn radio_group<K: crate::Key, T: Copy + PartialEq + Hash + Debug + 'static>(
        &mut self,
        key: K,
        current: T,
        options: &[(T, &str)],
    ) -> RadioGroupBuilder<'_, 'b, 'f, A, K, T> {
        RadioGroupBuilder::new(self, key, current, options)
    }

    /// A segmented control over `(value, label)` options, of which `current`
    /// is selected. Values must be distinct.
    pub fn segmented<K: crate::Key, T: Copy + PartialEq + Hash + Debug + 'static>(
        &mut self,
        key: K,
        current: T,
        options: &[(T, &str)],
    ) -> SegmentedBuilder<'_, 'b, 'f, A, K, T> {
        SegmentedBuilder::new(self, key, current, options)
    }

    /// A slider at `value` within `range`.
    pub fn slider<K: crate::Key>(
        &mut self,
        key: K,
        value: f64,
        range: RangeInclusive<f64>,
    ) -> SliderBuilder<'_, 'b, 'f, A, K> {
        SliderBuilder::new(self, key, value, range)
    }

    /// A select over `(value, label)` options, of which `current` is chosen.
    pub fn select<K: crate::Key, T: Copy + PartialEq + Hash + Debug + 'static>(
        &mut self,
        key: K,
        current: T,
        options: &[(T, &str)],
    ) -> SelectBuilder<'_, 'b, 'f, A, K, T> {
        SelectBuilder::new(self, key, current, options)
    }

    /// A stepper at `value` within `range`.
    pub fn stepper<K: crate::Key>(
        &mut self,
        key: K,
        value: f64,
        range: RangeInclusive<f64>,
    ) -> StepperBuilder<'_, 'b, 'f, A, K> {
        StepperBuilder::new(self, key, value, range)
    }
}
