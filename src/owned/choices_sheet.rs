//! The parts of the choice controls (a checkbox, a radio group, a switch) and
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
use crate::{Aria, Match};

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

/// TODO(prose): the anatomy of a checkbox: shadcn's `checkbox`.
///
/// TODO(prose): the keys its rules see: the size, `aria-checked` (shadcn's
/// `data-checked`) and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckboxPart {
    /// TODO(prose): the box.
    Root,
    /// TODO(prose): the check glyph in a ticked box.
    Indicator,
}

impl Part for CheckboxPart {
    const SOURCE: &'static str = "checkbox.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "checkbox",
            Self::Indicator => "checkbox-indicator",
        }
    }
}

impl CheckboxPart {
    /// TODO(prose): the keys of a checkbox of `size`, ticked if `mark` is on, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::Aria;
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{CheckboxPart, Enablement, Mark};
    /// use eggshell_ui::sheet::Key;
    ///
    /// let keys = CheckboxPart::keys(Size::DEFAULT, Mark::On, Enablement::Enabled);
    /// assert!(keys.contains(&Key::Aria(Aria::Checked)));
    /// ```
    pub fn keys(size: Size, mark: Mark, enablement: Enablement) -> Vec<Key> {
        choice_keys(size, mark, enablement)
    }
}

/// TODO(prose): the anatomy of a radio group: shadcn's `radio-group`.
///
/// TODO(prose): the keys its rules see: an item's size, `aria-checked`
/// (shadcn's `data-checked`) and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RadioGroupPart {
    /// TODO(prose): the group, holding a field per option.
    Root,
    /// TODO(prose): one option's circle.
    Item,
    /// TODO(prose): what the chosen circle holds: the room its dot is
    /// centred in.
    Indicator,
    /// TODO(prose): the dot in the chosen circle.
    IndicatorIcon,
}

impl Part for RadioGroupPart {
    const SOURCE: &'static str = "radio-group.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "radio-group",
            Self::Item => "radio-group-item",
            Self::Indicator => "radio-group-indicator",
            Self::IndicatorIcon => "radio-group-indicator-icon",
        }
    }
}

impl RadioGroupPart {
    /// TODO(prose): the keys of an item of `size` and its dot, chosen if `mark` is on, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::Aria;
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, Mark, RadioGroupPart};
    /// use eggshell_ui::sheet::Key;
    ///
    /// let keys = RadioGroupPart::item_keys(Size::DEFAULT, Mark::Off, Enablement::Disabled);
    /// assert!(keys.contains(&Key::Aria(Aria::Disabled)));
    /// ```
    pub fn item_keys(size: Size, mark: Mark, enablement: Enablement) -> Vec<Key> {
        choice_keys(size, mark, enablement)
    }
}

/// TODO(prose): the anatomy of a switch: shadcn's `switch`.
///
/// TODO(prose): the keys its rules see: the size (shadcn's `data-size`),
/// `aria-checked` (shadcn's `data-checked`) and `aria-disabled`. The thumb is
/// painted, sliding between its two places: its rules give the size and fill
/// it is painted at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwitchPart {
    /// TODO(prose): the track.
    Root,
    /// TODO(prose): the thumb.
    Thumb,
}

impl Part for SwitchPart {
    const SOURCE: &'static str = "switch.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "switch",
            Self::Thumb => "switch-thumb",
        }
    }
}

impl SwitchPart {
    /// TODO(prose): the keys of a switch of `size` and its thumb, on if `mark` is, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, Mark, SwitchPart};
    ///
    /// let keys = SwitchPart::keys(Size::DEFAULT, Mark::Off, Enablement::Enabled);
    /// assert_eq!(keys.len(), 1);
    /// ```
    pub fn keys(size: Size, mark: Mark, enablement: Enablement) -> Vec<Key> {
        choice_keys(size, mark, enablement)
    }
}

fn choice_keys(size: Size, mark: Mark, enablement: Enablement) -> Vec<Key> {
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
        let m = *checkbox.sizes.get(size);
        let when = When::always().size(size);
        sheet.rule_when(CheckboxPart::Root, when, move |t, _| {
            t.size_px(m.size).rounded_px(f32::from(m.radius))
        });
        sheet.rule_when(CheckboxPart::Indicator, when, move |t, _| {
            t.children(Match::glyph(), |g| g.size_px(m.glyph))
        });
    }
    let default = radio.sizes.default_key();
    for part in [RadioGroupPart::Item, RadioGroupPart::IndicatorIcon] {
        sheet.rule_when(part, When::always().size(default), |t, _| t);
    }
    for size in radio.sizes.keys().filter(|size| *size != default) {
        let m = *radio.sizes.get(size);
        let when = When::always().size(size);
        sheet.rule_when(RadioGroupPart::Item, when, move |t, _| t.size_px(m.size));
        sheet.rule_when(RadioGroupPart::IndicatorIcon, when, move |t, _| {
            t.size_px(m.dot)
        });
    }
    for size in switch.sizes.keys() {
        if sheet
            .unnamed(SwitchPart::Root, &[Key::Size(size)])
            .is_none()
        {
            continue;
        }
        let m = *switch.sizes.get(size);
        let when = When::always().size(size);
        sheet.rule_when(SwitchPart::Root, when, move |t, _| {
            t.w_px(m.track.x).h_px(m.track.y)
        });
        for (mark, diameter) in [(Mark::Off, m.thumb_off), (Mark::On, m.thumb_on)] {
            let when = match mark {
                Mark::Off => when.not_aria(Aria::Checked),
                Mark::On => when.aria(Aria::Checked),
            };
            sheet.rule_when(SwitchPart::Thumb, when, move |t, _| t.size_px(diameter));
        }
    }
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
