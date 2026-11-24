//! The parts of a menu, the look a style without a preset draws them with,
//! and the layout [`MenuStyle`]'s metrics give them.

use egui::Color32;

use crate::contract::{Look, MenuStyle, States, Variant};
use crate::sheet::{Key, Part, Sheet, When};
use crate::{Aria, Match, TextSpec, Tracking, Tw, Weight};

use super::menu::Tone;
use super::{Enablement, SelectPart, painted};

/// TODO(prose): the anatomy of a menu: shadcn's `dropdown-menu`, which a
/// context menu and the list a select opens share.
///
/// TODO(prose): the keys its rules see: an item's variant (shadcn's
/// `data-variant`, `default` or `destructive`) and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuPart {
    /// TODO(prose): the box that holds the items.
    Content,
    /// TODO(prose): an item that runs a handler.
    Item,
    /// TODO(prose): an item with a check mark.
    CheckboxItem,
    /// TODO(prose): an item with a dot.
    RadioItem,
    /// TODO(prose): the square an item keeps before its label for its icon, check or dot.
    ItemIndicator,
    /// TODO(prose): a heading over a group of items.
    Label,
    /// TODO(prose): a line between groups.
    Separator,
    /// TODO(prose): the keys an item shows after its label.
    Shortcut,
    /// TODO(prose): an item that opens a submenu.
    SubTrigger,
    /// TODO(prose): the box of a submenu.
    SubContent,
}

impl Part for MenuPart {
    const SOURCE: &'static str = "dropdown-menu.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Content => "dropdown-menu-content",
            Self::Item => "dropdown-menu-item",
            Self::CheckboxItem => "dropdown-menu-checkbox-item",
            Self::RadioItem => "dropdown-menu-radio-item",
            Self::ItemIndicator => "dropdown-menu-item-indicator",
            Self::Label => "dropdown-menu-label",
            Self::Separator => "dropdown-menu-separator",
            Self::Shortcut => "dropdown-menu-shortcut",
            Self::SubTrigger => "dropdown-menu-sub-trigger",
            Self::SubContent => "dropdown-menu-sub-content",
        }
    }
}

const DESTRUCTIVE: Variant = Variant("destructive");

/// The attribute of an item whose label lines up with its neighbours' past
/// their marks: shadcn's `data-inset`.
pub(crate) const INSET: (&str, &str) = ("inset", "true");

fn variant(tone: Tone) -> Variant {
    match tone {
        Tone::Plain => Variant::DEFAULT,
        Tone::Destructive => DESTRUCTIVE,
    }
}

impl MenuPart {
    /// The keys of an item that is `tone` and `enablement`.
    pub(crate) fn item_keys(tone: Tone, enablement: Enablement) -> Vec<Key> {
        let mut keys = vec![Key::Variant(variant(tone))];
        if enablement == Enablement::Disabled {
            keys.push(Key::Aria(Aria::Disabled));
        }
        keys
    }
}

/// The parts that are an item: a row that runs, checks, picks or opens.
const ITEMS: [MenuPart; 4] = [
    MenuPart::Item,
    MenuPart::CheckboxItem,
    MenuPart::RadioItem,
    MenuPart::SubTrigger,
];

fn enabled_when(when: When, enablement: Enablement) -> When {
    match enablement {
        Enablement::Enabled => when.not_aria(Aria::Disabled),
        Enablement::Disabled => when.aria(Aria::Disabled),
    }
}

/// What a style without a preset draws a menu and a select's list with:
/// the box's and the items' colours, radii and least width, and the
/// separator's, heading's and shortcut's looks.
#[derive(Clone, Copy)]
pub(crate) struct MenuDefaults {
    pub popup: Look,
    pub radius: u8,
    pub min_width: f32,
    pub item: Look,
    pub item_radius: u8,
    pub destructive: States,
    pub separator: Color32,
    pub separator_margin: f32,
    pub heading: Color32,
    pub heading_weight: Weight,
    pub shortcut: Color32,
    pub shortcut_tracking: Tracking,
}

impl MenuDefaults {
    fn box_paint(&self, t: Tw) -> Tw {
        let colors = self.popup.paint.states.rest;
        let t = t
            .border(self.popup.border_width)
            .border_color(colors.border)
            .bg(colors.fill)
            .text(colors.text)
            .rounded_px(f32::from(self.radius));
        match self.popup.paint.shadow {
            Some(shadow) => t.shadow(shadow),
            None => t,
        }
    }

    fn row_paint(&self, t: Tw, tone: Tone, enablement: Enablement) -> Tw {
        let states = match tone {
            Tone::Plain => self.item.paint.states,
            Tone::Destructive => self.destructive,
        };
        let t = t.rounded_px(f32::from(self.item_radius));
        match enablement {
            Enablement::Disabled => painted(t, self.item.paint.states.disabled),
            Enablement::Enabled => {
                let hover = states.hover;
                painted(t, states.rest).highlighted(|p| p.bg(hover.fill).text(hover.text))
            }
        }
    }
}

/// Writes `defaults` as the menu's and the select list's rules in `sheet`.
pub(crate) fn menu_defaults(sheet: &mut Sheet, defaults: MenuDefaults) {
    let d = defaults;
    for part in [MenuPart::Content, MenuPart::SubContent] {
        sheet.replace(part, move |t, _| d.box_paint(t).min_w_px(d.min_width));
    }
    sheet.replace(SelectPart::Content, move |t, _| d.box_paint(t));
    for enablement in [Enablement::Enabled, Enablement::Disabled] {
        for part in ITEMS {
            for tone in [Tone::Plain, Tone::Destructive] {
                let when = enabled_when(When::always().variant(variant(tone)), enablement);
                sheet.replace_when(part, when, move |t, _| d.row_paint(t, tone, enablement));
            }
        }
        sheet.replace_when(
            SelectPart::Item,
            enabled_when(When::always(), enablement),
            move |t, _| d.row_paint(t, Tone::Plain, enablement),
        );
    }
    sheet.replace(MenuPart::Separator, move |t, _| {
        t.my_px(d.separator_margin).bg(d.separator)
    });
    sheet.replace(MenuPart::Label, move |t, _| {
        t.text(d.heading).font_weight(d.heading_weight)
    });
    sheet.replace(MenuPart::Shortcut, move |t, _| {
        t.text(d.shortcut).tracking(d.shortcut_tracking)
    });
}

/// Writes the menu's layout, from `menu`'s metrics in the text role
/// `text`, at the sheet's current layer.
pub(crate) fn write_rules(sheet: &mut Sheet, menu: &MenuStyle, text: &TextSpec) {
    for part in [MenuPart::Content, MenuPart::SubContent] {
        let (menu, text) = (menu.clone(), text.clone());
        sheet.rule(part, move |t, _| t.merge(menu.box_layout(&text)));
    }
    for enablement in [Enablement::Enabled, Enablement::Disabled] {
        for part in ITEMS {
            let (menu, text) = (menu.clone(), text.clone());
            sheet.rule_when(
                part,
                enabled_when(When::always(), enablement),
                move |t, _| t.merge(menu.row_layout(enablement, &text)),
            );
        }
    }
    let indicator = menu.indicator;
    let inset = menu.item_padding.x + indicator + menu.item_gap;
    for part in ITEMS {
        sheet.rule(part, move |t, _| {
            t.descendants(Match::glyph(), |g| g.size_px(indicator).shrink_0())
        });
        sheet.rule_when(part, When::always().attr(INSET.0, INSET.1), move |t, _| {
            t.pl_px(inset)
        });
    }
    sheet.replace(MenuPart::ItemIndicator, move |t, _| {
        t.size_px(indicator).shrink_0()
    });
    let padding = menu.padding;
    sheet.rule(MenuPart::Separator, move |t, _| {
        t.h_px(1.0).mx_px(-padding).shrink_0()
    });
    let (padding_x, row_height) = (menu.item_padding.x, menu.row_height(text));
    sheet.rule(MenuPart::Label, move |t, _| {
        t.flex()
            .items_center()
            .px_px(padding_x)
            .h_px(row_height)
            .select_none()
            .whitespace_nowrap()
    });
    let spec = menu.shortcut_text.clone();
    sheet.rule(MenuPart::Shortcut, move |t, _| {
        t.ml_auto().role(spec.clone())
    });
}

/// Writes the menu's names over the rules at the sheet's current layer: an
/// item of the default variant keeps the generated look, which names only
/// the destructive one. Naming it says so, for the check that a variant its
/// parts' rules leave out is a mistake.
pub(crate) fn write_exceptions(sheet: &mut Sheet) {
    sheet.rule_when(
        MenuPart::Item,
        When::always().variant(Variant::DEFAULT),
        |t, _| t,
    );
}
