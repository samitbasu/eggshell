//! The parts of a menu, and the sheet rules that give them today's
//! [`MenuStyle`] look.
//!
//! TODO(#446): these rules are written from the values `MenuStyle` holds, as
//! the step that moves the mechanism onto the sheet; the generated rules of
//! shadcn's `cn-dropdown-menu-*` replace them.

use crate::contract::{MenuStyle, Variant};
use crate::sheet::{Key, Part, Sheet, When};
use crate::{Aria, TextSpec};

use super::Enablement;
use super::menu::Tone;

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

/// Writes the rules that draw `menu`'s look, in the text role `text`, at the
/// sheet's current layer.
pub(crate) fn write_rules(sheet: &mut Sheet, menu: &MenuStyle, text: &TextSpec) {
    for part in [MenuPart::Content, MenuPart::SubContent] {
        let (menu, text) = (menu.clone(), text.clone());
        sheet.replace(part, move |t, _| {
            t.merge(menu.box_classes(&text).min_w_px(menu.min_width))
        });
    }
    for part in [
        MenuPart::Item,
        MenuPart::CheckboxItem,
        MenuPart::RadioItem,
        MenuPart::SubTrigger,
    ] {
        for tone in [Tone::Plain, Tone::Destructive] {
            for enablement in [Enablement::Enabled, Enablement::Disabled] {
                let when = When::always().variant(variant(tone));
                let when = match enablement {
                    Enablement::Enabled => when.not_aria(Aria::Disabled),
                    Enablement::Disabled => when.aria(Aria::Disabled),
                };
                let (menu, text) = (menu.clone(), text.clone());
                sheet.replace_when(part, when, move |t, _| {
                    t.merge(menu.row_classes(tone, enablement, &text))
                });
            }
        }
    }
    let indicator = menu.indicator;
    sheet.replace(MenuPart::ItemIndicator, move |t, _| {
        t.size_px(indicator).shrink_0()
    });
    let (padding, margin, separator) = (menu.padding, menu.separator_margin, menu.separator);
    sheet.replace(MenuPart::Separator, move |t, _| {
        t.h_px(1.0)
            .mx(-padding / 4.0)
            .my(margin / 4.0)
            .shrink_0()
            .bg(separator)
    });
    let (menu_label, text_label) = (menu.clone(), text.clone());
    sheet.replace(MenuPart::Label, move |t, _| {
        t.flex()
            .items_center()
            .px(menu_label.item_padding.x / 4.0)
            .h_px(menu_label.row_height(&text_label))
            .text(menu_label.heading)
            .font_weight(menu_label.heading_weight)
            .select_none()
            .whitespace_nowrap()
    });
    let (spec, color, tracking) = (
        menu.shortcut_text.clone(),
        menu.shortcut,
        menu.shortcut_tracking,
    );
    sheet.replace(MenuPart::Shortcut, move |t, _| {
        t.ml_auto()
            .role(spec.clone())
            .text(color)
            .tracking(tracking)
    });
}
