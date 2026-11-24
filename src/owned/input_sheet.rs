//! The input family's parts, and the sheet rules that give them today's
//! [`InputStyle`] look.
//!
//! TODO(#446): these rules are written from the values `InputStyle` holds, as
//! the step that moves the mechanism onto the sheet; the generated rules of
//! shadcn's `cn-input` and `cn-input-group` replace them.

use egui::{Color32, Vec2};

use crate::contract::{InputStyle, Size, Spacing};
use crate::sheet::{Key, Part, Sheet, When};
use crate::theme::Tokens;
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
/// look: shadcn draws one field per style, and eggshell's smaller fields are
/// its own (a recorded exception, #446). Their control's text, their addons'
/// icons and the `icon_gap` between an icon and the text follow each size's
/// metrics; their height and padding are the preset's.
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
        if let Some(text) = m.text {
            sheet.rule_when(InputGroupPart::Input, when, move |t, _| {
                t.text_size(text.size, text.line_height)
            });
        }
        sheet.rule_when(InputGroupPart::Addon, when, move |t, k| {
            t.children(Match::glyph(), |g| g.size_px(addon_icon(k, size).0))
        });
        for (side, pad) in [
            (INLINE_START, Tw::pl_px as fn(Tw, f32) -> Tw),
            (INLINE_END, Tw::pr_px),
        ] {
            sheet.rule_when(
                InputGroupPart::Input,
                when.attr("addon", side),
                move |t, k| pad(t, addon_icon(k, size).1),
            );
        }
    }
}

/// The side of an addon's icon and its gap to the control's text at a size
/// other than the default: eggshell's own smaller fields.
fn addon_icon(k: &Tokens, size: Size) -> (f32, f32) {
    use crate::theme::size;
    match size {
        s if s == size::XS => (k.points(3.0), k.points(1.5)),
        s if s == size::SM => (k.points(3.5), k.points(1.5)),
        _ => (k.points(4.0), k.points(2.0)),
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
    /// A one-line field's height; a taller field's text sets its height.
    pub height: f32,
    pub padding: Vec2,
}

impl Frame {
    pub(crate) fn of(classes: &Tw, spacing: Spacing) -> Self {
        use crate::builder::length_px;
        use crate::tw::State;
        let width = classes.style().border.left.into_raw();
        let layout = classes.style_at(spacing);
        Self {
            height: length_px(layout.size.height.into_raw()),
            padding: Vec2::new(
                length_px(layout.padding.left.into_raw()),
                length_px(layout.padding.top.into_raw()),
            ),
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

/// TODO(prose): eggshell's smaller fields, which shadcn does not draw: a `sm`
/// field a unit shorter than the preset's controls and a `xs` one three, each
/// with its own inline padding, over the generated look.
pub(crate) fn write_field_sizes(sheet: &mut Sheet, t: &Tokens) {
    use crate::theme::size;
    let own = t.own_input_padding_x;
    for (size, height, padding_x) in [
        (size::SM, t.control_units - 1.0, own),
        (size::XS, t.control_units - 3.0, (own - 1.0).max(0.0)),
    ] {
        let when = When::always().size(size);
        sheet.rule_when(InputPart::Root, when, move |c, _| c.h(height).px(padding_x));
        sheet.rule_when(InputGroupPart::Root, when, move |c, _| c.h(height));
        sheet.rule_when(InputGroupPart::Input, when, move |c, _| c.px(padding_x));
        sheet.rule_when(
            InputGroupPart::Addon,
            when.attr("align", "inline-start"),
            move |c, _| c.pl(padding_x),
        );
        sheet.rule_when(
            InputGroupPart::Addon,
            when.attr("align", "inline-end"),
            move |c, _| c.pr(padding_x),
        );
    }
    sheet.rule_when(InputPart::Root, When::always().size(size::XS), |c, _| {
        c.py(0.5)
    });
}
