//! The parts of the choice controls (a checkbox, a radio group, a switch) and
//! of the field row that holds one beside its label, and the sheet rules that
//! give them today's [`CheckboxStyle`], [`RadioStyle`] and [`SwitchStyle`]
//! look.
//!
//! TODO(#446): these rules are written from the values the style sections
//! hold, as the step that moves the mechanism onto the sheet; the generated
//! rules of shadcn's `cn-checkbox*`, `cn-radio-group*`, `cn-switch*` and
//! `cn-field*` replace them.

use core::time::Duration;

use crate::contract::{CheckboxStyle, Paint, RadioStyle, Size, States, SwitchStyle, Tokens};
use crate::sheet::{Key, Part, Sheet, When};
use crate::{Aria, Tw};

use super::{Enablement, Mark, centered};

/// TODO(prose): the anatomy of a field: shadcn's `field`. A choice control
/// is drawn in a horizontal field, beside its label.
///
/// TODO(prose): the keys its rules see: `data-[orientation=horizontal]` on a
/// field, and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldPart {
    /// TODO(prose): the field, holding a control and its label.
    Field,
    /// TODO(prose): the label, which names the control and toggles it.
    Label,
}

impl Part for FieldPart {
    const SOURCE: &'static str = "field.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Field => "field",
            Self::Label => "field-label",
        }
    }
}

/// The attribute of a field that lays its control and label out in a row.
const HORIZONTAL: (&str, &str) = ("orientation", "horizontal");

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
    /// TODO(prose): the dot in the chosen circle.
    Indicator,
}

impl Part for RadioGroupPart {
    const SOURCE: &'static str = "radio-group.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "radio-group",
            Self::Item => "radio-group-item",
            Self::Indicator => "radio-group-indicator",
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

/// The condition of a control of `size`, `mark`ed and `enablement`.
fn when(size: Size, mark: Mark, enablement: Enablement) -> When {
    let when = When::always().size(size);
    let when = match mark {
        Mark::On => when.aria(Aria::Checked),
        Mark::Off => when.not_aria(Aria::Checked),
    };
    match enablement {
        Enablement::Enabled => when.not_aria(Aria::Disabled),
        Enablement::Disabled => when.aria(Aria::Disabled),
    }
}

/// Every combination [`when`] tells apart, but the size.
const STATES: [(Mark, Enablement); 4] = [
    (Mark::Off, Enablement::Enabled),
    (Mark::Off, Enablement::Disabled),
    (Mark::On, Enablement::Enabled),
    (Mark::On, Enablement::Disabled),
];

/// What a control's box is painted with, `mark`ed.
struct Painted {
    off: Paint,
    on: Paint,
    border_width: f32,
    transition: Duration,
}

impl Painted {
    fn classes(&self, mark: Mark, radius: u8, enablement: Enablement, k: &Tokens) -> Tw {
        let paint = match mark {
            Mark::Off => &self.off,
            Mark::On => &self.on,
        };
        paint.classes_in(
            self.border_width,
            (k.focus, k.disabled),
            radius,
            enablement,
            self.transition,
        )
    }
}

/// Writes the rules that draw a field and its label, in `label`'s colours,
/// at the sheet's current layer.
pub(crate) fn write_field_rules(sheet: &mut Sheet, label: States) {
    let horizontal = When::always().attr(HORIZONTAL.0, HORIZONTAL.1);
    sheet.rule_when(FieldPart::Field, horizontal, |t, _| {
        t.flex().items_center().whitespace_nowrap().select_none()
    });
    sheet.rule_when(
        FieldPart::Field,
        When::always().aria(Aria::Disabled),
        |t, k| Enablement::Disabled.apply_in(t, k.disabled),
    );
    for enablement in [Enablement::Enabled, Enablement::Disabled] {
        let when = match enablement {
            Enablement::Enabled => When::always().not_aria(Aria::Disabled),
            Enablement::Disabled => When::always().aria(Aria::Disabled),
        };
        sheet.rule_when(FieldPart::Label, when, move |t, k| {
            t.text(enablement.colors_in(label, k.disabled).text)
        });
    }
}

/// Writes the rules that draw `checkbox`'s look at the sheet's current layer.
pub(crate) fn write_checkbox_rules(sheet: &mut Sheet, checkbox: &CheckboxStyle) {
    let painted = Painted {
        off: checkbox.off,
        on: checkbox.on,
        border_width: checkbox.border_width,
        transition: checkbox.transition,
    };
    let painted = std::sync::Arc::new(painted);
    for size in checkbox.sizes.keys() {
        let metrics = *checkbox.sizes.get(size);
        sheet.rule_when(
            CheckboxPart::Indicator,
            When::always().size(size),
            move |t, _| t.size_px(metrics.glyph),
        );
        for (mark, enablement) in STATES {
            let painted = painted.clone();
            sheet.rule_when(
                CheckboxPart::Root,
                when(size, mark, enablement),
                move |t, k| {
                    let look = painted.classes(mark, metrics.radius, enablement, k);
                    centered(t.merge(look)).shrink_0().size_px(metrics.size)
                },
            );
        }
    }
}

/// Writes the rules that draw `radio`'s look at the sheet's current layer.
pub(crate) fn write_radio_rules(sheet: &mut Sheet, radio: &RadioStyle) {
    let painted = std::sync::Arc::new(Painted {
        off: radio.off,
        on: radio.on,
        border_width: radio.border_width,
        transition: radio.transition,
    });
    sheet.rule(RadioGroupPart::Root, |t, _| t.flex().flex_col().gap(3.0));
    for size in radio.sizes.keys() {
        let metrics = *radio.sizes.get(size);
        sheet.rule_when(
            RadioGroupPart::Indicator,
            When::always().size(size),
            move |t, _| t.size_px(metrics.dot),
        );
        for (mark, enablement) in STATES {
            let painted = painted.clone();
            sheet.rule_when(
                RadioGroupPart::Item,
                when(size, mark, enablement),
                move |t, k| {
                    let look = painted.classes(mark, u8::MAX, enablement, k);
                    centered(t.merge(look)).shrink_0().size_px(metrics.size)
                },
            );
        }
    }
}

/// Writes the rules that draw `switch`'s look at the sheet's current layer.
pub(crate) fn write_switch_rules(sheet: &mut Sheet, switch: &SwitchStyle) {
    let painted = std::sync::Arc::new(Painted {
        off: switch.off,
        on: switch.on,
        border_width: switch.border_width,
        transition: switch.transition,
    });
    for size in switch.sizes.keys() {
        let metrics = *switch.sizes.get(size);
        for (mark, enablement) in STATES {
            let (painted, thumb) = (painted.clone(), painted.clone());
            let when = when(size, mark, enablement);
            sheet.rule_when(SwitchPart::Root, when, move |t, k| {
                t.merge(painted.classes(mark, u8::MAX, enablement, k))
                    .flex()
                    .items_center()
                    .shrink_0()
                    .w_px(metrics.track.x)
                    .h_px(metrics.track.y)
            });
            let diameter = match mark {
                Mark::Off => metrics.thumb_off,
                Mark::On => metrics.thumb_on,
            };
            sheet.rule_when(SwitchPart::Thumb, when, move |t, k| {
                let paint = match mark {
                    Mark::Off => &thumb.off,
                    Mark::On => &thumb.on,
                };
                t.size_px(diameter)
                    .bg(enablement.colors_in(paint.states, k.disabled).text)
            });
        }
    }
}
