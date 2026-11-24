//! The parts of a menu, the look a style without a preset draws them with,
//! and the layout [`MenuStyle`]'s metrics give them.

use crate::Aria;
use crate::contract::Variant;
use crate::sheet::{Key, Part, Sheet, When};

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
