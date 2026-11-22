//! The anatomy of a combobox, for the style's sheet.

use eggshell_ui::sheet::Part;

/// TODO(prose): the anatomy of a combobox: shadcn's `combobox`, as eggshell
/// draws it, a field with its list below. Its trigger and clear buttons
/// are the widget's own, as shadcn's are input-group buttons.
///
/// TODO(prose): the keys its rules see: an item's `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComboboxPart {
    /// TODO(prose): the box of the field and its end button: shadcn's
    /// `ComboboxInput`, an input group, whose slot it says, so that the field
    /// in it is the group's control.
    Input,
    /// TODO(prose): the chevron that opens the list.
    TriggerIcon,
    /// TODO(prose): the cross that clears the value.
    ClearIcon,
    /// TODO(prose): the box of the open list.
    Content,
    /// TODO(prose): the rows, which scroll.
    List,
    /// TODO(prose): one option.
    Item,
    /// TODO(prose): the room the chosen option keeps for its check.
    ItemIndicator,
    /// TODO(prose): the check of the chosen option.
    ItemIndicatorIcon,
    /// TODO(prose): what the list says when nothing matches.
    Empty,
    /// TODO(prose): a heading over a group of options.
    Label,
}

impl Part for ComboboxPart {
    const SOURCE: &'static str = "combobox.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Input => "input-group",
            Self::TriggerIcon => "combobox-trigger-icon",
            Self::ClearIcon => "combobox-clear-icon",
            Self::Content => "combobox-content",
            Self::List => "combobox-list",
            Self::Item => "combobox-item",
            Self::ItemIndicator => "combobox-item-indicator",
            Self::ItemIndicatorIcon => "combobox-item-indicator-icon",
            Self::Empty => "combobox-empty",
            Self::Label => "combobox-label",
        }
    }
}
