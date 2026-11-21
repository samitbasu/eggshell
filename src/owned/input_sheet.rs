//! The input family's parts, and the sheet rules that give them today's
//! [`InputStyle`] look.
//!
//! TODO(#446): these rules are written from the values `InputStyle` holds, as
//! the step that moves the mechanism onto the sheet; the generated rules of
//! shadcn's `cn-input` and `cn-input-group` replace them.

use egui::Color32;

use crate::contract::{Colors, Disabled, InputStyle, Size, Tokens};
use crate::sheet::{Key, Part, Sheet, When};
use crate::{Aria, Match, Tw};

/// TODO(prose): the anatomy of a text field: shadcn's `input`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputPart {
    /// TODO(prose): the field.
    Root,
}

impl Part for InputPart {
    const SOURCE: &'static str = "input.tsx";

    fn slot(self) -> &'static str {
        "input"
    }
}

/// TODO(prose): the anatomy of a box that holds a field and what goes beside it: shadcn's `input-group`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputGroupPart {
    /// TODO(prose): the box.
    Root,
    /// TODO(prose): what sits beside the field.
    Addon,
    /// TODO(prose): a button in an addon.
    Button,
    /// TODO(prose): text in an addon.
    Text,
    /// TODO(prose): the field inside the box.
    Input,
    /// TODO(prose): a multi-line field inside the box.
    Textarea,
}

impl Part for InputGroupPart {
    const SOURCE: &'static str = "input-group.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "input-group",
            Self::Addon => "input-group-addon",
            Self::Button => "input-group-button",
            Self::Text => "input-group-text",
            Self::Input => "input-group-control",
            Self::Textarea => "input-group-textarea",
        }
    }
}

/// The keys of a field or group of `size`, disabled or not.
pub(crate) fn keys(size: Size, disabled: bool) -> Vec<Key> {
    let mut keys = vec![Key::Size(size)];
    if disabled {
        keys.push(Key::Aria(Aria::Disabled));
    }
    keys
}

fn painted(classes: Tw, colors: Colors) -> Tw {
    classes
        .select_none()
        .bg(colors.fill)
        .border_color(colors.border)
        .text(colors.text)
}

/// Writes the rules that draw `input`'s look at the sheet's current layer.
pub(crate) fn write_rules(sheet: &mut Sheet, input: &InputStyle) {
    let look = input.look;
    let (states, shadow, width) = (look.paint.states, look.paint.shadow, look.border_width);
    let ring = move |k: &Tokens| look.paint.focus.unwrap_or(k.focus);

    sheet.rule(InputPart::Root, move |t, k| {
        let ring = ring(k);
        painted(t.border(width), states.rest)
            .hover(|p| p.border_color(states.hover.border))
            .focus(|p| {
                p.border_color(states.focused.border)
                    .ring(ring.width, ring.color)
            })
    });
    sheet.rule(InputGroupPart::Root, move |t, _| t.flex().border(width));
    for size in input.sizes.keys() {
        let metrics = *input.sizes.get(size);
        let radius = f32::from(metrics.radius);
        let when = When::always().size(size);
        sheet.rule_when(InputPart::Root, when, move |t, _| t.rounded_px(radius));
        sheet.rule_when(InputGroupPart::Root, when, move |t, _| {
            t.rounded_px(radius)
                .h_px(metrics.height)
                .items_center()
                .gap(metrics.icon_gap / 4.0)
                .px(metrics.padding.x / 4.0)
        });
    }
    let shadowed = move |classes: Tw| match shadow {
        Some(shadow) => classes.shadow(shadow),
        None => classes,
    };
    sheet.rule_when(
        InputGroupPart::Root,
        When::always().not_aria(Aria::Disabled),
        move |t, k| {
            let ring = ring(k);
            shadowed(painted(t, states.rest).has(
                Match::slot(InputGroupPart::Input.slot()).with_focus_visible(),
                |b| {
                    b.border_color(states.focused.border)
                        .ring(ring.width, ring.color)
                },
            ))
        },
    );
    sheet.rule_when(
        InputGroupPart::Root,
        When::always().aria(Aria::Disabled),
        move |t, k| {
            let (colors, faded): (Colors, Option<crate::Opacity>) = match k.disabled {
                Disabled::Colors => (states.disabled, None),
                Disabled::Faded(opacity) => (states.rest, Some(opacity)),
            };
            let classes = shadowed(painted(t, colors).inert());
            match faded {
                Some(opacity) => classes.opacity(opacity),
                None => classes,
            }
        },
    );
}

/// What a text field's own frame is drawn with, read from the sheet.
pub(crate) struct Frame {
    pub fill: Option<Color32>,
    pub rest: Option<Color32>,
    pub hover: Option<Color32>,
    pub focused: Option<Color32>,
    pub text: Option<Color32>,
    pub border_width: f32,
    pub radius: u8,
}

impl Frame {
    pub(crate) fn of(classes: &Tw) -> Self {
        use crate::tw::State;
        let width = classes.style().border.left.into_raw();
        Self {
            fill: classes.paint.background,
            rest: classes.paint.border_colors.top,
            hover: classes.variant_border(State::Hover),
            focused: classes.variant_border(State::Focus),
            text: classes.text.color,
            border_width: width.value(),
            radius: classes.paint.radius.nw,
        }
    }
}
