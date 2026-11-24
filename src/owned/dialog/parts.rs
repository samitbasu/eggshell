//! The anatomies of the dialog surface, for the style's sheet: shadcn's
//! `dialog`, `alert-dialog`, `sheet` and `popover`, each drawn by the
//! surface it names.

use eggshell_ui::sheet::Part;

/// TODO(prose): a centred dialog's anatomy: shadcn's `dialog`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DialogPart {
    /// TODO(prose): the box.
    Content,
    /// TODO(prose): the close button (×).
    Close,
    /// TODO(prose): the title and the description.
    Header,
    /// TODO(prose): the footer's buttons.
    Footer,
    /// TODO(prose): the title.
    Title,
    /// TODO(prose): the description.
    Description,
}

impl Part for DialogPart {
    const SOURCE: &'static str = "dialog.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Content => "dialog-content",
            Self::Close => "dialog-close",
            Self::Header => "dialog-header",
            Self::Footer => "dialog-footer",
            Self::Title => "dialog-title",
            Self::Description => "dialog-description",
        }
    }
}

/// TODO(prose): an alert dialog's anatomy: shadcn's `alert-dialog`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertDialogPart {
    /// TODO(prose): the box.
    Content,
    /// TODO(prose): the title and the description.
    Header,
    /// TODO(prose): the footer's buttons.
    Footer,
    /// TODO(prose): the title.
    Title,
    /// TODO(prose): the description.
    Description,
}

impl Part for AlertDialogPart {
    const SOURCE: &'static str = "alert-dialog.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Content => "alert-dialog-content",
            Self::Header => "alert-dialog-header",
            Self::Footer => "alert-dialog-footer",
            Self::Title => "alert-dialog-title",
            Self::Description => "alert-dialog-description",
        }
    }
}

/// TODO(prose): a sheet's anatomy: shadcn's `sheet`, docked to a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SheetPart {
    /// TODO(prose): the box.
    Content,
    /// TODO(prose): the close button (×).
    Close,
    /// TODO(prose): the title and the description.
    Header,
    /// TODO(prose): the footer's buttons.
    Footer,
    /// TODO(prose): the title.
    Title,
    /// TODO(prose): the description.
    Description,
}

impl Part for SheetPart {
    const SOURCE: &'static str = "sheet.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Content => "sheet-content",
            Self::Close => "sheet-close",
            Self::Header => "sheet-header",
            Self::Footer => "sheet-footer",
            Self::Title => "sheet-title",
            Self::Description => "sheet-description",
        }
    }
}

/// TODO(prose): a popover's anatomy: shadcn's `popover`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PopoverPart {
    /// TODO(prose): the box.
    Content,
}

impl Part for PopoverPart {
    const SOURCE: &'static str = "popover.tsx";

    fn slot(self) -> &'static str {
        "popover-content"
    }
}
