//! The parts of a table, and the sheet rules that give them today's
//! [`TableStyle`] look.
//!
//! TODO(#446): these rules are written from the values `TableStyle` holds, as
//! the step that moves the mechanism onto the sheet; the generated rules of
//! shadcn's `cn-table-*` replace them.

use crate::Aria;
use crate::contract::TableStyle;
use crate::sheet::{Key, Part, Sheet, When};

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
    pub(crate) fn cell_keys(first: bool) -> Vec<Key> {
        match first {
            true => vec![Key::Attr(COLUMN.0, COLUMN.1)],
            false => Vec::new(),
        }
    }
}

/// Writes the rules that draw `table`'s look at the sheet's current layer.
pub(crate) fn write_rules(sheet: &mut Sheet, table: &TableStyle) {
    sheet.replace(TablePart::Root, |t, _| t.grid().w_full());
    let t = table.clone();
    sheet.replace(TablePart::Header, move |b, _| {
        b.bg(t.header_fill)
            .border_b(t.rule_width)
            .border_b_color(t.rule)
    });
    let t = table.clone();
    sheet.replace(TablePart::Group, move |b, _| {
        b.bg(t.group_fill)
            .border_b(t.rule_width)
            .border_b_color(t.rule)
    });
    let t = table.clone();
    sheet.replace(TablePart::Footer, move |b, _| {
        b.bg(t.footer_fill)
            .border_t(t.rule_width)
            .border_t_color(t.rule)
    });

    let t = table.clone();
    sheet.replace(TablePart::Row, move |b, _| b.transition(t.transition));
    let t = table.clone();
    sheet.rule_when(
        TablePart::Row,
        When::always().not_attr(POSITION.0, POSITION.1),
        move |b, _| b.border_b(t.rule_width).border_b_color(t.rule),
    );
    let t = table.clone();
    if table.rule_below_last {
        sheet.rule_when(
            TablePart::Row,
            When::always().attr(POSITION.0, POSITION.1),
            move |b, _| b.border_b(t.rule_width).border_b_color(t.rule),
        );
    }
    let stripe = table.stripe;
    sheet.rule_when(
        TablePart::Row,
        When::always().attr(STRIPE.0, STRIPE.1),
        move |b, _| b.bg(stripe),
    );
    let selected = table.selected;
    sheet.rule_when(
        TablePart::Row,
        When::always().aria(Aria::Selected),
        move |b, _| b.bg(selected),
    );
    let hover = table.hover;
    sheet.rule_when(
        TablePart::Row,
        When::always().not_aria(Aria::Selected),
        move |b, _| b.hover(|p| p.bg(hover)),
    );

    let t = table.clone();
    let cell_y = table.cell_padding_y.unwrap_or(table.cell_padding);
    let cell = move |b: crate::Tw| b.flex().items_center().px_px(t.cell_padding);
    let cell_of = cell;
    sheet.replace(TablePart::Cell, move |b, _| cell_of(b).py_px(cell_y));
    let cell_of = cell;
    let weight = table.group_weight;
    let color = table.group_text;
    sheet.replace(TablePart::GroupCell, move |b, _| {
        let b = cell_of(b)
            .py_px(cell_y)
            .font_weight(weight)
            .whitespace_nowrap();
        match color {
            Some(color) => b.text(color),
            None => b,
        }
    });
    let cell_of = cell;
    let weight = table.footer_weight;
    sheet.replace(TablePart::FooterCell, move |b, _| {
        cell_of(b).py_px(cell_y).font_weight(weight)
    });
    let (padding_y, height, weight) = (
        table.header_padding_y,
        table.header_height,
        table.header_weight,
    );
    sheet.replace(TablePart::Head, move |b, _| {
        let b = cell(b).font_weight(weight);
        match padding_y {
            Some(padding) => b.py_px(padding),
            None => b.h_px(height),
        }
    });
    if let Some(inset) = table.first_column_inset {
        let first = When::always().attr(COLUMN.0, COLUMN.1);
        for part in [
            TablePart::Head,
            TablePart::Cell,
            TablePart::GroupCell,
            TablePart::FooterCell,
        ] {
            sheet.rule_when(part, first, move |b, _| b.pl_px(inset));
        }
    }
}

/// Writes the table's recorded rules over a preset's generated look, at the
/// sheet's current layer.
///
/// eggshell's header is a band, its own row, drawn from the section, which
/// rules it as table.tsx's `[&_tr]:border-b` does; its body rows say which
/// is last, for the body's `[&_tr:last-child]:border-0`, unless the section
/// keeps a rule below it. A selected row keeps its fill under the pointer, as
/// `data-[state=selected]:` follows `hover:` in Tailwind's order. Stripes
/// and the first column's inset are eggshell's own, from the section.
pub(crate) fn write_exceptions(sheet: &mut Sheet, table: &TableStyle) {
    let last = if table.rule_below_last { 1.0 } else { 0.0 };
    sheet.rule_when(
        TablePart::Row,
        When::always().attr(POSITION.0, POSITION.1),
        move |t, _| t.border_b(last),
    );
    sheet.rule_when(
        TablePart::Row,
        When::always().aria(Aria::Selected),
        |t, k| t.hover(|p| p.bg(k.muted)),
    );
    let stripe = table.stripe;
    sheet.rule_when(
        TablePart::Row,
        When::always().attr(STRIPE.0, STRIPE.1),
        move |t, _| t.bg(stripe),
    );
    if let Some(inset) = table.first_column_inset {
        let first = When::always().attr(COLUMN.0, COLUMN.1);
        for part in [TablePart::Head, TablePart::Cell, TablePart::FooterCell] {
            sheet.rule_when(part, first, move |t, _| t.pl_px(inset));
        }
    }
}
