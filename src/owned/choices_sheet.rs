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

use crate::contract::{CheckboxStyle, Paint, RadioStyle, Size, States, SwitchStyle};
use crate::sheet::{Key, Part, Sheet, When};
use crate::theme::Tokens;
use crate::{Aria, Match, Tw, Weight};
use egui::Color32;

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

/// While the control is `mark`ed and `enablement`.
fn state(mark: Mark, enablement: Enablement) -> When {
    let when = match mark {
        Mark::On => When::always().aria(Aria::Checked),
        Mark::Off => When::always().not_aria(Aria::Checked),
    };
    match enablement {
        Enablement::Enabled => when.not_aria(Aria::Disabled),
        Enablement::Disabled => when.aria(Aria::Disabled),
    }
}

/// Every combination [`state`] tells apart.
const STATES: [(Mark, Enablement); 4] = [
    (Mark::Off, Enablement::Enabled),
    (Mark::Off, Enablement::Disabled),
    (Mark::On, Enablement::Enabled),
    (Mark::On, Enablement::Disabled),
];

/// What a control's box is painted with, `mark`ed.
pub(crate) struct Painted {
    pub off: Paint,
    pub on: Paint,
    pub border_width: f32,
    pub transition: Duration,
}

impl Painted {
    fn classes(&self, mark: Mark, radius: u8, enablement: Enablement, k: &Tokens) -> Tw {
        let paint = match mark {
            Mark::Off => &self.off,
            Mark::On => &self.on,
        };
        paint.classes_in(
            self.border_width,
            (k.focus(), k.disabled),
            radius,
            enablement,
            self.transition,
        )
    }
}

/// The controls a card holds, by slot.
const CONTROLS: [&str; 3] = ["checkbox", "radio-group-item", "switch"];

/// Writes the rules that draw a field and its label, in `label`'s colours,
/// and a choice card, at the sheet's current layer.
pub(crate) fn write_field_rules(sheet: &mut Sheet, label: States) {
    let horizontal = When::always().attr(HORIZONTAL.0, HORIZONTAL.1);
    sheet.rule_when(FieldPart::Root, horizontal, |t, _| {
        t.flex()
            .items_center()
            .whitespace_nowrap()
            .select_none()
            .has_child(Match::slot(FieldPart::Content.slot()), |b| b.items_start())
    });
    write_card_rules(sheet);
    sheet.rule_when(
        FieldPart::Root,
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

/// Writes the rules of a choice card, a label that holds its field, after
/// shadcn's `cn-field-label`, `cn-field-content`, `cn-field-title` and
/// `cn-field-description`.
///
/// TODO(#446): written from the tokens until the importer generates them.
fn write_card_rules(sheet: &mut Sheet) {
    let field = Match::slot(FieldPart::Root.slot());
    sheet.rule(FieldPart::Label, move |t, k| {
        let radius = k.radius.md();
        let mut card = t
            .has_child(field, |b| {
                b.flex()
                    .flex_col()
                    .w_full()
                    .rounded_px(radius)
                    .border(1.0)
                    .border_color(k.border)
            })
            .has(Match::aria(Aria::Checked), |b| {
                b.bg(k.primary.gamma_multiply(0.05))
                    .border_color(k.primary.gamma_multiply(0.3))
            })
            .has(Match::focus_visible(), |b| {
                b.border_color(k.ring)
                    .ring(k.focus().width, k.focus().color)
            })
            .children(field, |t| t.p(3.0));
        for control in CONTROLS {
            card = card.descendants(Match::slot(control), |t| {
                t.focus(|p| p.ring(0.0, Color32::TRANSPARENT).border_color(k.input))
            });
        }
        for control in ["checkbox", "radio-group-item"] {
            card = card
                .descendants(Match::slot(control).with_aria(Aria::Checked), |t| {
                    t.focus(|p| p.border_color(k.primary))
                })
                .descendants(Match::slot(control), |t| t.mt(0.25));
        }
        card.descendants(Match::slot("switch"), |t| {
            t.focus(|p| p.border_color(Color32::TRANSPARENT))
        })
    });
    sheet.rule_when(
        FieldPart::Label,
        When::always().not_aria(Aria::Disabled),
        move |t, k| t.has_child(field, |b| b.hover(|p| p.bg(k.muted.gamma_multiply(0.5)))),
    );
    sheet.rule(FieldPart::Content, |t, _| {
        t.flex().flex_1().flex_col().gap(1.0).whitespace_normal()
    });
    sheet.rule(FieldPart::Title, |t, k| {
        let size = k.control_text_size().size;
        t.flex()
            .w_fit()
            .items_center()
            .gap(2.0)
            .text_size(size, size * 1.375)
            .font_weight(Weight::Medium)
    });
    sheet.rule(FieldPart::Description, |t, k| {
        let size = k.control_text_size().size;
        t.text_size(size, size * 1.5)
            .font_weight(Weight::Normal)
            .text(k.muted_foreground)
    });
}

/// What a style without a preset paints a checkbox, a radio button and a
/// switch with.
pub(crate) struct ChoiceDefaults {
    pub checkbox: Painted,
    pub checkbox_radius: u8,
    pub radio: Painted,
    pub switch: Painted,
}

/// Writes `defaults` as the choice controls' rules in `sheet`: each look,
/// checked or not, enabled or not.
pub(crate) fn choice_defaults(sheet: &mut Sheet, defaults: ChoiceDefaults) {
    let ChoiceDefaults {
        checkbox,
        checkbox_radius,
        radio,
        switch,
    } = defaults;
    let (checkbox, radio, switch) = (
        std::sync::Arc::new(checkbox),
        std::sync::Arc::new(radio),
        std::sync::Arc::new(switch),
    );
    sheet.rule(CheckboxPart::Indicator, |t, _| centered(t.size_full()));
    sheet.rule(RadioGroupPart::Root, |t, _| t.flex().flex_col().gap(3.0));
    sheet.rule(RadioGroupPart::Indicator, |t, _| centered(t.size_full()));
    sheet.rule(RadioGroupPart::IndicatorIcon, |t, _| t.rounded_full());
    for enablement in [Enablement::Enabled, Enablement::Disabled] {
        let when = match enablement {
            Enablement::Enabled => When::always().not_aria(Aria::Disabled),
            Enablement::Disabled => When::always().aria(Aria::Disabled),
        };
        let on = radio.on.states;
        sheet.rule_when(RadioGroupPart::IndicatorIcon, when, move |t, k| {
            t.bg(enablement.colors_in(on, k.disabled).text)
        });
    }
    for (mark, enablement) in STATES {
        let when = state(mark, enablement);
        let painted = checkbox.clone();
        sheet.rule_when(CheckboxPart::Root, when, move |t, k| {
            let look = painted.classes(mark, checkbox_radius, enablement, k);
            centered(t.merge(look)).shrink_0()
        });
        let painted = radio.clone();
        sheet.rule_when(RadioGroupPart::Item, when, move |t, k| {
            let look = painted.classes(mark, u8::MAX, enablement, k);
            centered(t.merge(look)).shrink_0()
        });
        let painted = switch.clone();
        sheet.rule_when(SwitchPart::Root, when, move |t, k| {
            t.merge(painted.classes(mark, u8::MAX, enablement, k))
                .flex()
                .items_center()
                .shrink_0()
        });
        let thumb = switch.clone();
        sheet.rule_when(SwitchPart::Thumb, when, move |t, k| {
            let paint = match mark {
                Mark::Off => &thumb.off,
                Mark::On => &thumb.on,
            };
            t.bg(enablement.colors_in(paint.states, k.disabled).text)
        });
    }
}

/// Writes the rules `checkbox`'s sizes give at the sheet's current layer.
pub(crate) fn write_checkbox_rules(sheet: &mut Sheet, checkbox: &CheckboxStyle) {
    for size in checkbox.sizes.keys() {
        let metrics = *checkbox.sizes.get(size);
        let when = When::always().size(size);
        sheet.rule_when(CheckboxPart::Indicator, when, move |t, _| {
            t.children(Match::glyph(), |g| g.size_px(metrics.glyph))
        });
        sheet.rule_when(CheckboxPart::Root, when, move |t, _| {
            t.size_px(metrics.size)
                .rounded_px(f32::from(metrics.radius))
        });
    }
}

/// Writes the rules `radio`'s sizes give at the sheet's current layer.
pub(crate) fn write_radio_rules(sheet: &mut Sheet, radio: &RadioStyle) {
    for size in radio.sizes.keys() {
        let metrics = *radio.sizes.get(size);
        let when = When::always().size(size);
        sheet.rule_when(RadioGroupPart::IndicatorIcon, when, move |t, _| {
            t.size_px(metrics.dot)
        });
        sheet.rule_when(RadioGroupPart::Item, when, move |t, _| {
            t.size_px(metrics.size)
        });
    }
}

/// Writes the rules `switch`'s sizes give at the sheet's current layer.
pub(crate) fn write_switch_rules(sheet: &mut Sheet, switch: &SwitchStyle) {
    for size in switch.sizes.keys() {
        let metrics = *switch.sizes.get(size);
        let when = When::always().size(size);
        sheet.rule_when(SwitchPart::Root, when, move |t, _| {
            t.w_px(metrics.track.x).h_px(metrics.track.y)
        });
        for (mark, diameter) in [(Mark::Off, metrics.thumb_off), (Mark::On, metrics.thumb_on)] {
            let when = match mark {
                Mark::On => when.aria(Aria::Checked),
                Mark::Off => when.not_aria(Aria::Checked),
            };
            sheet.rule_when(SwitchPart::Thumb, when, move |t, _| t.size_px(diameter));
        }
    }
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
