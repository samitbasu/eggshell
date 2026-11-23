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
    let icon = input.icon;
    sheet.rule(InputGroupPart::Addon, move |t, _| {
        t.flex().items_center().shrink_0().text(icon)
    });
    for size in input.sizes.keys() {
        let metrics = *input.sizes.get(size);
        let radius = f32::from(metrics.radius);
        let when = When::always().size(size);
        sheet.rule_when(InputGroupPart::Addon, when, move |t, _| {
            t.children(Match::glyph(), |g| g.size_px(metrics.icon_size))
        });
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

/// Writes the input's recorded exceptions (#446) over the generated look,
/// at the sheet's current layer: a disabled group fades as a whole, and the
/// sizes other than the default keep their metrics.
pub(crate) fn write_exceptions(sheet: &mut Sheet, input: &InputStyle) {
    write_disabled_exception(sheet);
    write_size_exceptions(sheet, input);
}

/// A refused or disabled field takes exactly the disabled look (#385): its
/// whole group at half, as a disabled button is. shadcn fades only the
/// control and the addon, so those take back their own fade inside it.
fn write_disabled_exception(sheet: &mut Sheet) {
    let disabled = When::always().aria(Aria::Disabled);
    sheet.rule_when(InputGroupPart::Root, disabled, |t, _| {
        t.opacity(crate::Opacity::new(0.5))
    });
    for part in [
        InputGroupPart::Input,
        InputGroupPart::Textarea,
        InputGroupPart::Addon,
    ] {
        sheet.rule_when(part, disabled, |t, _| t.opacity(crate::Opacity::new(1.0)));
    }
}

/// Writes `input`'s sizes other than its default as rules over the generated
/// look: shadcn draws one field height per style, and eggshell's smaller
/// fields are its own (a recorded exception, #446). The group's height and
/// radius, its control's text, its addons' icons, the framed field's radius,
/// and the spacing follow each size's metrics: `padding.x` between the border
/// and what is next to it, `icon_gap` between an icon and the text.
fn write_size_exceptions(sheet: &mut Sheet, input: &InputStyle) {
    let default = input.sizes.default_key();
    // The default size keeps the generated look. Naming it says so, for the
    // check that a size its parts' rules leave out is a mistake.
    let keeps = When::always().size(default);
    sheet.rule_when(InputPart::Root, keeps, |t, _| t);
    sheet.rule_when(InputGroupPart::Root, keeps, |t, _| t);
    sheet.rule_when(InputGroupPart::Input, keeps, |t, _| t);
    sheet.rule_when(InputGroupPart::Addon, keeps, |t, _| t);
    for size in input.sizes.keys().filter(|size| *size != default) {
        let m = *input.sizes.get(size);
        let when = When::always().size(size);
        let radius = f32::from(m.radius);
        sheet.rule_when(InputPart::Root, when, move |t, _| t.rounded_px(radius));
        sheet.rule_when(InputGroupPart::Root, when, move |t, _| {
            t.h_px(m.height).rounded_px(radius)
        });
        sheet.rule_when(InputGroupPart::Input, when, move |t, _| {
            let t = t.px(m.padding.x / 4.0);
            match m.text {
                Some(text) => t.text_size(text.size, text.line_height),
                None => t,
            }
        });
        sheet.rule_when(InputGroupPart::Addon, when, move |t, _| {
            t.children(Match::glyph(), |g| g.size_px(m.icon_size))
        });
        let (edge, gap) = (m.padding.x / 4.0, m.icon_gap / 4.0);
        for (side, pad) in [
            (INLINE_START, Tw::pl as fn(Tw, f32) -> Tw),
            (INLINE_END, Tw::pr),
        ] {
            sheet.rule_when(
                InputGroupPart::Addon,
                when.attr("align", side),
                move |t, _| pad(t, edge),
            );
            sheet.rule_when(
                InputGroupPart::Input,
                when.attr("addon", side),
                move |t, _| pad(t, gap),
            );
        }
    }
}

/// Where an addon sits in its group, and so the side of the control it is
/// on, as shadcn's `data-align` says.
pub(crate) const INLINE_START: &str = "inline-start";
pub(crate) const INLINE_END: &str = "inline-end";

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
