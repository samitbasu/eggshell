//! The parts of a table, the look a style without a preset draws them with,
//! and the rules written from [`TableStyle`].

use crate::Aria;
use crate::sheet::{Key, Part, Sheet, When};
use egui::Color32;

/// TODO(prose): the anatomy of a table: shadcn's `table`. A band is a
/// row of the grid: the header, a body row, a group row or the footer.
/// A cell is named for the band it is in.
///
/// TODO(prose): the keys its rules see: a body row's `aria-selected`, its
/// `data-[stripe=odd]` when striped and `data-[position=last]` when last, and
/// a cell's `data-[column=first]`. The group row and its cell are eggshell's,
/// not shadcn's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TablePart {
    /// TODO(prose): the grid that holds the bands.
    Root,
    /// TODO(prose): the band of column headings.
    Header,
    /// TODO(prose): a body row.
    Row,
    /// TODO(prose): a row that titles the rows after it.
    Group,
    /// TODO(prose): the band of totals under the body.
    Footer,
    /// TODO(prose): a cell of the header.
    Head,
    /// TODO(prose): a cell of a body row.
    Cell,
    /// TODO(prose): the one cell of a group row.
    GroupCell,
    /// TODO(prose): a cell of the footer.
    FooterCell,
}

impl Part for TablePart {
    const SOURCE: &'static str = "table.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "table",
            Self::Header => "table-header",
            Self::Row => "table-row",
            Self::Group => "table-group",
            Self::Footer => "table-footer",
            Self::Head => "table-head",
            Self::Cell => "table-cell",
            Self::GroupCell => "table-group-cell",
            Self::FooterCell => "table-footer-cell",
        }
    }
}

const STRIPE: (&str, &str) = ("stripe", "odd");
const POSITION: (&str, &str) = ("position", "last");
const COLUMN: (&str, &str) = ("column", "first");

/// What the keys of a body row say of it.
#[derive(Clone, Copy)]
pub(crate) struct RowState {
    pub(crate) striped: bool,
    pub(crate) last: bool,
    pub(crate) selected: bool,
}

impl TablePart {
    /// The keys of a body row.
    pub(crate) fn row_keys(state: RowState) -> Vec<Key> {
        let mut keys = Vec::new();
        if state.striped {
            keys.push(Key::Attr(STRIPE.0, STRIPE.1));
        }
        if state.last {
            keys.push(Key::Attr(POSITION.0, POSITION.1));
        }
        if state.selected {
            keys.push(Key::Aria(Aria::Selected));
        }
        keys
    }

    /// The keys of a cell, the first of its row if `first`.
    /// TODO(prose): the head and cells of a table's first column.
    pub fn first_column() -> When {
        When::always().attr(COLUMN.0, COLUMN.1)
    }

    /// TODO(prose): a table's last body row, which the table leaves unruled.
    pub fn last_row() -> When {
        When::always().attr(POSITION.0, POSITION.1)
    }

    /// TODO(prose): every second body row of a table with
    /// [`Stripes::Alternate`](crate::owned::table::Stripes::Alternate), the ones a stripe fills.
    pub fn striped_row() -> When {
        When::always().attr(STRIPE.0, STRIPE.1)
    }

    pub(crate) fn cell_keys(first: bool) -> Vec<Key> {
        match first {
            true => vec![Key::Attr(COLUMN.0, COLUMN.1)],
            false => Vec::new(),
        }
    }
}

/// TODO(prose): writes the table's recorded rules over the generated look.
///
/// eggshell's header and group rows are bands, rows of their own, ruled as
/// table.tsx's `[&_tr]:border-b` rules its rows; a group's cells are laid
/// out as body cells are. Its body rows say which is last, for the body's
/// `[&_tr:last-child]:border-0`. A selected row keeps its fill under the
/// pointer, as `data-[state=selected]:` follows `hover:` in Tailwind's
/// order. Stripes are eggshell's own: `muted/30`.
pub(crate) fn write_exceptions(sheet: &mut Sheet) {
    sheet.rule(TablePart::Header, |t, k| {
        t.bg(Color32::TRANSPARENT)
            .border_b(1.0)
            .border_b_color(k.border)
    });
    sheet.rule(TablePart::Group, |t, k| {
        t.border_b(1.0).border_b_color(k.border)
    });
    sheet.rule(TablePart::GroupCell, |t, _| {
        t.flex()
            .items_center()
            .px_px(GROUP_CELL_PADDING)
            .py_px(GROUP_CELL_PADDING)
            .whitespace_nowrap()
    });
    sheet.rule_when(TablePart::Row, TablePart::last_row(), |t, _| {
        t.border_b(0.0)
    });
    sheet.rule_when(
        TablePart::Row,
        When::always().aria(Aria::Selected),
        |t, k| t.hover(|p| p.bg(k.muted)),
    );
    sheet.rule_when(TablePart::Row, TablePart::striped_row(), |t, k| {
        t.bg(k.muted.gamma_multiply(0.3))
    });
}

/// The padding of a group row's cells, which shadcn does not draw: the one
/// egui's buttons had, which the cells have always kept.
const GROUP_CELL_PADDING: f32 = 8.0;

/// A table's group rows, which are eggshell's own: shadcn's footer band.
pub(crate) fn write_group_band(sheet: &mut Sheet) {
    sheet
        .rule(TablePart::Group, |t, k| t.bg(k.muted.gamma_multiply(0.5)))
        .rule(TablePart::GroupCell, |t, k| t.text(k.muted_foreground));
}
