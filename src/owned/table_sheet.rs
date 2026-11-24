//! The parts of a table, the look a style without a preset draws them with,
//! and the rules written from [`TableStyle`].

use std::time::Duration;

use egui::Color32;

use crate::contract::TableStyle;
use crate::sheet::{Key, Part, Sheet, When};
use crate::{Aria, Weight};

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
    /// TODO(prose): the head and cells of a table's first column, which a table's `first_column_inset` pads.
    pub fn first_column() -> When {
        When::always().attr(COLUMN.0, COLUMN.1)
    }

    /// TODO(prose): a table's last body row, which the table leaves unruled.
    pub fn last_row() -> When {
        When::always().attr(POSITION.0, POSITION.1)
    }

    /// TODO(prose): every second body row of a table with
    /// [`Stripes::Alternate`](crate::owned::Stripes::Alternate), the ones a stripe fills.
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

/// What a style without a preset draws a table with: its bands' fills, its
/// cells' padding and weights, and its rows' hover and selection.
pub(crate) struct TableDefaults {
    pub header_fill: Color32,
    pub header_weight: Weight,
    pub header_height: f32,
    pub cell_padding: f32,
    pub group_fill: Color32,
    pub group_text: Option<Color32>,
    pub hover: Color32,
    pub selected: Color32,
    pub footer_fill: Color32,
    pub footer_weight: Weight,
    pub transition: Duration,
}

impl TableDefaults {
    /// Rows hovered and selected in egui's widget and selection fills.
    pub(crate) fn from_egui(egui: &egui::Style) -> Self {
        let (v, s) = (&egui.visuals, &egui.spacing);
        Self {
            header_fill: Color32::TRANSPARENT,
            header_weight: Weight::Normal,
            header_height: s.interact_size.y + 2.0 * s.button_padding.y,
            cell_padding: s.button_padding.x,
            group_fill: v.faint_bg_color,
            group_text: None,
            hover: v.widgets.hovered.weak_bg_fill,
            selected: v.selection.bg_fill,
            footer_fill: v.faint_bg_color,
            footer_weight: Weight::Normal,
            transition: Duration::from_millis(100),
        }
    }
}

/// Writes `table` as the table's rules in `sheet`.
pub(crate) fn table_defaults(sheet: &mut Sheet, table: TableDefaults) {
    sheet.replace(TablePart::Root, |t, _| t.grid().w_full());
    let fill = table.header_fill;
    sheet.replace(TablePart::Header, move |b, _| b.bg(fill));
    let fill = table.group_fill;
    sheet.replace(TablePart::Group, move |b, _| b.bg(fill));
    let fill = table.footer_fill;
    sheet.replace(TablePart::Footer, move |b, _| b.bg(fill));

    let transition = table.transition;
    sheet.replace(TablePart::Row, move |b, _| b.transition(transition));
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

    let padding = table.cell_padding;
    let cell = move |b: crate::Tw| b.flex().items_center().px_px(padding);
    sheet.replace(TablePart::Cell, move |b, _| cell(b).py_px(padding));
    let color = table.group_text;
    sheet.replace(TablePart::GroupCell, move |b, _| {
        let b = cell(b).py_px(padding).whitespace_nowrap();
        match color {
            Some(color) => b.text(color),
            None => b,
        }
    });
    let weight = table.footer_weight;
    sheet.replace(TablePart::FooterCell, move |b, _| {
        cell(b).py_px(padding).font_weight(weight)
    });
    let (height, weight) = (table.header_height, table.header_weight);
    sheet.replace(TablePart::Head, move |b, _| {
        cell(b).font_weight(weight).h_px(height)
    });
}

/// Writes the rules `table` gives, its rules between bands and rows, its
/// stripes and its first column's inset, at the sheet's current layer.
pub(crate) fn write_rules(sheet: &mut Sheet, table: &TableStyle) {
    let (rule, width) = (table.rule, table.rule_width);
    sheet.rule(TablePart::Header, move |b, _| {
        b.border_b(width).border_b_color(rule)
    });
    sheet.rule(TablePart::Group, move |b, _| {
        b.border_b(width).border_b_color(rule)
    });
    sheet.rule(TablePart::Footer, move |b, _| {
        b.border_t(width).border_t_color(rule)
    });
    sheet.rule_when(
        TablePart::Row,
        When::always().not_attr(POSITION.0, POSITION.1),
        move |b, _| b.border_b(width).border_b_color(rule),
    );
    let last = if table.rule_below_last { width } else { 0.0 };
    sheet.rule_when(
        TablePart::Row,
        When::always().attr(POSITION.0, POSITION.1),
        move |b, _| b.border_b(last).border_b_color(rule),
    );
    let stripe = table.stripe;
    sheet.rule_when(
        TablePart::Row,
        When::always()
            .attr(STRIPE.0, STRIPE.1)
            .not_aria(Aria::Selected),
        move |b, _| b.bg(stripe),
    );
    if let Some(inset) = table.first_column_inset {
        for part in [
            TablePart::Head,
            TablePart::Cell,
            TablePart::GroupCell,
            TablePart::FooterCell,
        ] {
            sheet.rule_when(part, TablePart::first_column(), move |b, _| b.pl_px(inset));
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
        for part in [TablePart::Head, TablePart::Cell, TablePart::FooterCell] {
            sheet.rule_when(part, TablePart::first_column(), move |t, _| t.pl_px(inset));
        }
    }
}
