//! TODO(prose): the parts of the choice controls (a checkbox, a radio group, a switch) and
//! of the field row that holds one beside its label, and the sheet rules that
//! give them today's [`CheckboxStyle`], [`RadioStyle`] and [`SwitchStyle`]
//! look.
//!
//! TODO(#446): these rules are written from the values the style sections
//! hold, as the step that moves the mechanism onto the sheet; the generated
//! rules of shadcn's `cn-checkbox*`, `cn-radio-group*`, `cn-switch*` and
//! `cn-field*` replace them.

use crate::contract::{CheckboxStyle, RadioStyle, Size, SwitchStyle};
use crate::sheet::{Key, Part, Sheet, When};
use crate::theme::{Tokens, size};
use crate::{Aria, Match};

use super::checkbox::CheckboxPart;
use super::radio::RadioGroupPart;
use super::switch::SwitchPart;
use super::{Enablement, Mark, centered};

/// TODO(prose): the anatomy of a field: shadcn's `field`. A choice control
/// is drawn in a horizontal field, beside its label; a choice card is a label
/// that holds the field, the control after the content's title and
/// description.
///
/// TODO(prose): the keys its rules see: `data-[orientation=horizontal]` on a
/// field, and `aria-disabled`. shadcn's title shares the label's slot; here it
/// has its own, as the sheet keeps a part's rules by its slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldPart {
    /// TODO(prose): the field, holding a control and its label or content.
    Root,
    /// TODO(prose): the label, which names the control and toggles it; a card when it holds the field.
    Label,
    /// TODO(prose): a card's title and description.
    Content,
    /// TODO(prose): a card's title, which names the control.
    Title,
    /// TODO(prose): a card's description.
    Description,
}

impl Part for FieldPart {
    const SOURCE: &'static str = "field.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "field",
            Self::Label => "field-label",
            Self::Content => "field-content",
            Self::Title => "field-title",
            Self::Description => "field-description",
        }
    }
}

/// The attribute of a field that lays its control and label out in a row.
const HORIZONTAL: (&str, &str) = ("orientation", "horizontal");

/// The attribute of a label that holds its field, a choice card, and of the
/// control in it: shadcn asks `has-[>[data-slot=field]]` of the label and
/// `group-has-[…]/field-label` of the control, which eggshell knows as it
/// builds the card.
pub(crate) const CARD: (&str, &str) = ("card", "true");

/// The attribute of a card whose control is disabled: shadcn asks
/// `not-has-[:disabled,[data-disabled]]` of the card's label, which holds
/// the field and is not inside it, so the field's own disabled state does
/// not fade it.
const DISABLED_CONTROL: (&str, &str) = ("disabled-control", "true");

impl FieldPart {
    /// TODO(prose): the keys of a horizontal field and its label, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::owned::{Enablement, FieldPart};
    ///
    /// assert_eq!(FieldPart::keys(Enablement::Disabled).len(), 2);
    /// ```
    pub fn keys(enablement: Enablement) -> Vec<Key> {
        let mut keys = vec![Key::Attr(HORIZONTAL.0, HORIZONTAL.1)];
        if enablement == Enablement::Disabled {
            keys.push(Key::Aria(Aria::Disabled));
        }
        keys
    }

    /// The keys of a choice card's label, which holds a field of
    /// `enablement`.
    pub(crate) fn card_keys(enablement: Enablement) -> Vec<Key> {
        let mut keys = vec![Key::Attr(CARD.0, CARD.1)];
        if enablement == Enablement::Disabled {
            keys.push(Key::Attr(DISABLED_CONTROL.0, DISABLED_CONTROL.1));
        }
        keys
    }
}

pub(crate) fn choice_keys(size: Size, mark: Mark, enablement: Enablement) -> Vec<Key> {
    let mut keys = vec![Key::Size(size)];
    if mark == Mark::On {
        keys.push(Key::Aria(Aria::Checked));
    }
    if enablement == Enablement::Disabled {
        keys.push(Key::Aria(Aria::Disabled));
    }
    keys
}

/// Writes the choice controls' recorded exceptions over the rules at the
/// sheet's current layer.
///
/// A radio's dot is centred in its circle, as shadcn's `absolute left-1/2
/// top-1/2 -translate-1/2` places it: the indicator fills the circle and
/// centres the dot in its flow. A checkbox's indicator fills its box and
/// centres the check, as its `grid place-content-center` does, and is drawn
/// unchecked too, so checking moves no node. A card's checkbox or radio sits a point
/// down beside the card's content, field.tsx's
/// `has-[>[data-slot=field-content]]:[&>[role=checkbox],[role=radio]]:mt-px`,
/// whose selector list the importer does not split. A field is as wide as
/// its content, not shadcn's `w-full`: a checkbox, radio or switch is often
/// inline in a toolbar or a row. In a card, and in a form (whose section
/// says so of its fields), it fills the line, as shadcn's `Field` does. A size the controls' rules leave out keeps
/// its section's metrics: shadcn's checkbox and radio have one size, and
/// its switch two.
pub(crate) fn write_exceptions(
    sheet: &mut Sheet,
    (checkbox, radio, switch): (&CheckboxStyle, &RadioStyle, &SwitchStyle),
) {
    sheet.rule(RadioGroupPart::Indicator, |t, _| t.size_full());
    sheet.rule(CheckboxPart::Indicator, |t, _| centered(t.size_full()));
    sheet.rule(FieldPart::Root, |t, _| t.w_auto());
    let card = When::always().attr(CARD.0, CARD.1);
    sheet.rule_when(FieldPart::Label, card, |t, _| {
        ["checkbox", "radio-group-item"]
            .into_iter()
            .fold(t, |t, control| {
                t.descendants(Match::slot(control), |c| c.mt(0.25))
            })
            .children(Match::slot(FieldPart::Root.slot()), |f| f.w_full())
    });
    sheet.rule(RadioGroupPart::IndicatorIcon, |t, _| t.relative());
    let default = checkbox.sizes.default_key();
    // The default size keeps the generated look; naming it says so.
    for part in [CheckboxPart::Root, CheckboxPart::Indicator] {
        sheet.rule_when(part, When::always().size(default), |t, _| t);
    }
    for size in checkbox.sizes.keys().filter(|size| *size != default) {
        let when = When::always().size(size);
        sheet.rule_when(CheckboxPart::Root, when, move |t, k| {
            let m = checkbox_box(k, size);
            t.size_px(m.side).rounded_px(f32::from(m.radius))
        });
        sheet.rule_when(CheckboxPart::Indicator, when, move |t, k| {
            let m = checkbox_box(k, size);
            t.children(Match::glyph(), |g| g.size_px(m.glyph))
        });
    }
    let default = radio.sizes.default_key();
    for part in [RadioGroupPart::Item, RadioGroupPart::IndicatorIcon] {
        sheet.rule_when(part, When::always().size(default), |t, _| t);
    }
    for size in radio.sizes.keys().filter(|size| *size != default) {
        let when = When::always().size(size);
        sheet.rule_when(RadioGroupPart::Item, when, move |t, k| {
            t.size_px(radio_circle(k, size).0)
        });
        sheet.rule_when(RadioGroupPart::IndicatorIcon, when, move |t, k| {
            t.size_px(radio_circle(k, size).1)
        });
    }
    for size in switch.sizes.keys() {
        if sheet
            .unnamed(SwitchPart::Root, &[Key::Size(size)])
            .is_none()
        {
            continue;
        }
        let when = When::always().size(size);
        sheet.rule_when(SwitchPart::Root, when, move |t, k| {
            let track = switch_track(k, size);
            t.w_px(track.x).h_px(track.y)
        });
        for mark in [Mark::Off, Mark::On] {
            let when = match mark {
                Mark::Off => when.not_aria(Aria::Checked),
                Mark::On => when.aria(Aria::Checked),
            };
            sheet.rule_when(SwitchPart::Thumb, when, move |t, k| {
                t.size_px(switch_thumb(k, size))
            });
        }
    }
}

/// A checkbox's box at a size: eggshell's own smaller ones, which shadcn,
/// with one size, does not draw.
#[derive(Clone, Copy)]
struct CheckboxBox {
    side: f32,
    radius: u8,
    glyph: f32,
}

fn checkbox_box(k: &Tokens, size: Size) -> CheckboxBox {
    let at = |side, glyph| CheckboxBox {
        side: k.points(side),
        radius: k.checkbox_radius,
        glyph: k.points(glyph),
    };
    match size {
        s if s == size::XS => at(3.0, 2.5),
        s if s == size::SM => at(3.5, 3.0),
        _ => CheckboxBox {
            side: 16.0,
            radius: k.checkbox_radius,
            glyph: 14.0,
        },
    }
}

/// A radio button's circle and dot diameters at a size.
fn radio_circle(k: &Tokens, size: Size) -> (f32, f32) {
    match size {
        s if s == size::XS => (k.points(3.0), k.points(1.5)),
        s if s == size::SM => (k.points(3.5), k.points(1.75)),
        _ => (16.0, 8.0),
    }
}

/// The share of the default switch a switch at `size` is.
fn switch_scale(size: Size) -> f32 {
    match size {
        s if s == size::XS => 0.625,
        s if s == size::SM => 0.75,
        _ => 1.0,
    }
}

/// A switch's track at `size`.
pub(crate) fn switch_track(k: &Tokens, size: Size) -> egui::Vec2 {
    k.switch_track * switch_scale(size)
}

/// A switch's thumb diameter at `size`, on or off.
fn switch_thumb(k: &Tokens, size: Size) -> f32 {
    k.points(k.switch_thumb) * switch_scale(size)
}

/// field.tsx, a choice card's label in the dark theme:
/// `dark:has-data-checked:bg-primary/10 dark:has-data-checked:border-primary/20`.
pub(crate) fn write_dark_card_label(sheet: &mut Sheet) {
    sheet.rule(FieldPart::Label, |t, k| {
        t.has(Match::aria(Aria::Checked), |b| {
            b.bg(k.primary.gamma_multiply(0.1))
                .border_color(k.primary.gamma_multiply(0.2))
        })
    });
}
