//! The owned table: one grid holds every cell, so content-sized columns line
//! up across rows, and each row has a band spanning the grid behind its cells
//! for its rule and its fills.

use std::fmt::{self, Debug};
use std::hash::Hash;
use std::rc::Rc;

use egui::{Id, Role, Sense, Ui, UiBuilder, WidgetInfo};

use crate::builder::Slot;
use crate::contract::TableStyle;
use crate::{Builder, NodeUi as _, Style, Track, Tw, tw};

/// TODO(prose): where a column's cells place their content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    /// TODO(prose): at the left.
    #[default]
    Start,
    /// TODO(prose): centred.
    Center,
    /// TODO(prose): at the right.
    End,
}

/// TODO(prose): a column's track and alignment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    track: Track,
    align: Align,
}

impl Column {
    /// TODO(prose): a column sized by `track`, its content at the start.
    pub fn new(track: Track) -> Self {
        Self {
            track,
            align: Align::Start,
        }
    }

    /// TODO(prose): places the column's content, header and footer included.
    pub fn align(self, align: Align) -> Self {
        Self { align, ..self }
    }

    fn justified(self, classes: Tw) -> Tw {
        match self.align {
            Align::Start => classes.justify_start().text_left(),
            Align::Center => classes.justify_center().text_center(),
            Align::End => classes.justify_end().text_right(),
        }
    }
}

/// TODO(prose): a table whose rows cannot be selected.
pub struct Unselectable;

/// TODO(prose): a table told its selected row, waiting for `on_select`.
pub struct Selected<K>(Option<K>);

/// TODO(prose): a table whose rows the user selects.
pub struct Selectable<A, K> {
    current: Option<K>,
    select: Select<A, K>,
}

type Select<A, K> = Rc<dyn Fn(&mut A, K)>;

type Build<'a, 'f, A> = Box<dyn FnOnce(&mut Cells<'_, 'f, A>) + 'a>;

/// TODO(prose): a table, started with [`Builder::table`] and added by `rows`.
///
/// Selection is opt-in, and `selected` must be followed by `on_select`:
///
/// ```compile_fail
/// # use eggshell_ui::{Builder, Track};
/// # use eggshell_ui::owned::Column;
/// fn view(b: &mut Builder<'_, '_, Option<u32>>, selected: &Option<u32>) {
///     b.table("t")
///         .columns([Column::new(Track::auto())])
///         .selected(*selected)
///         .rows(&[1_u32], |n| *n, |row, n| row.cell_text(n.to_string()));
/// }
/// ```
///
/// A table that is never given its rows is not added:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// # use eggshell_ui::{Builder, Track};
/// # use eggshell_ui::owned::Column;
/// fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
///     b.table("t").columns([Column::new(Track::auto())]);
/// }
/// ```
///
/// ```
/// # use eggshell_ui::{Builder, Track};
/// # use eggshell_ui::owned::{Align, Column};
/// struct Invoice { id: u32, amount: String }
/// struct App { invoices: Vec<Invoice>, selected: Option<u32> }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.table("invoices")
///         .columns([Column::new(Track::fr(1.0)), Column::new(Track::auto()).align(Align::End)])
///         .header(|h| {
///             h.cell_text("Invoice");
///             h.cell_text("Amount");
///         })
///         .selected(app.selected)
///         .on_select(|app: &mut App, id| app.selected = Some(id))
///         .rows(&app.invoices, |invoice| invoice.id, |row, invoice| {
///             row.cell_text(format!("INV-{}", invoice.id));
///             row.cell_text(&invoice.amount);
///         });
/// }
/// ```
#[must_use = "a table is not added until you call `.rows(..)`"]
pub struct TableChain<'a, 'f, A, S = Unselectable> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    name: Option<String>,
    columns: Vec<Column>,
    header: Option<Build<'a, 'f, A>>,
    footer: Option<Build<'a, 'f, A>>,
    selection: S,
}

impl<'a, 'f, A, S> TableChain<'a, 'f, A, S> {
    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        crate::owned::options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        crate::owned::options::when_some(self, value, then)
    }
}

impl<'a, 'f, A: 'static> TableChain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            name: None,
            columns: Vec::new(),
            header: None,
            footer: None,
            selection: Unselectable,
        }
    }

    /// TODO(prose): rows the user selects, `current` among them.
    pub fn selected<K>(self, current: Option<K>) -> TableChain<'a, 'f, A, Selected<K>> {
        self.with_selection(|_| Selected(current))
    }

    /// TODO(prose): adds the table, a row per item of `items` under `key`.
    #[track_caller]
    pub fn rows<'i, T: 'i, K: crate::Key + Clone + PartialEq>(
        self,
        items: impl IntoIterator<Item = &'i T>,
        key: impl Fn(&T) -> K,
        body: impl FnMut(&mut Cells<'_, 'f, A>, &T),
    ) {
        self.with_selection(|_| None).add(items, key, body);
    }
}

impl<'a, 'f, A: 'static, S> TableChain<'a, 'f, A, S> {
    /// TODO(prose): the table's columns, in order.
    pub fn columns(mut self, columns: impl IntoIterator<Item = Column>) -> Self {
        self.columns = columns.into_iter().collect();
        self
    }

    /// TODO(prose): the header row, a cell per column.
    pub fn header(mut self, cells: impl FnOnce(&mut Cells<'_, 'f, A>) + 'a) -> Self {
        self.header = Some(Box::new(cells));
        self
    }

    /// TODO(prose): the footer row, a cell per column.
    pub fn footer(mut self, cells: impl FnOnce(&mut Cells<'_, 'f, A>) + 'a) -> Self {
        self.footer = Some(Box::new(cells));
        self
    }

    /// TODO(prose): the table's accessible name.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn with_selection<T>(self, selection: impl FnOnce(S) -> T) -> TableChain<'a, 'f, A, T> {
        let Self {
            b,
            slot,
            name,
            columns,
            header,
            footer,
            selection: was,
        } = self;
        let selection = selection(was);
        TableChain {
            b,
            slot,
            name,
            columns,
            header,
            footer,
            selection,
        }
    }
}

impl<'a, 'f, A: 'static, K> TableChain<'a, 'f, A, Selected<K>> {
    /// TODO(prose): runs `select` with the key of the row the user selects.
    pub fn on_select(
        self,
        select: impl Fn(&mut A, K) + 'static,
    ) -> TableChain<'a, 'f, A, Selectable<A, K>> {
        self.with_selection(|Selected(current)| Selectable {
            current,
            select: Rc::new(select),
        })
    }
}

impl<'a, 'f, A: 'static, K: crate::Key + Clone + PartialEq>
    TableChain<'a, 'f, A, Selectable<A, K>>
{
    /// TODO(prose): adds the table, a row per item of `items` under `key`.
    #[track_caller]
    pub fn rows<'i, T: 'i>(
        self,
        items: impl IntoIterator<Item = &'i T>,
        key: impl Fn(&T) -> K,
        body: impl FnMut(&mut Cells<'_, 'f, A>, &T),
    ) {
        self.with_selection(Some).add(items, key, body);
    }
}

/// The band behind a row of cells, keyed by what the row shows.
#[derive(Clone, Debug, Hash)]
enum Band<K> {
    Header,
    Row(K),
    Footer,
}

/// Which part of the table a row is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Header,
    Body,
    Footer,
}

impl Part {
    fn of<K>(band: &Band<K>) -> Self {
        match band {
            Band::Header => Self::Header,
            Band::Row(_) => Self::Body,
            Band::Footer => Self::Footer,
        }
    }
}

/// Where a row's band sits among the body rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Last {
    Yes,
    No,
}

/// Whether a body row is one of the second, fourth, … that the style's
/// stripe fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stripe {
    Plain,
    Striped,
}

impl Stripe {
    /// The stripe of the `nth` body row, counting from one.
    fn of(nth: usize) -> Self {
        if nth.is_multiple_of(2) {
            Self::Striped
        } else {
            Self::Plain
        }
    }
}

impl<'a, 'f, A: 'static, K: crate::Key + Clone + PartialEq>
    TableChain<'a, 'f, A, Option<Selectable<A, K>>>
{
    #[track_caller]
    fn add<'i, T: 'i>(
        self,
        items: impl IntoIterator<Item = &'i T>,
        key: impl Fn(&T) -> K,
        mut body: impl FnMut(&mut Cells<'_, 'f, A>, &T),
    ) {
        let Self {
            mut b,
            slot,
            name,
            columns,
            header,
            footer,
            selection,
        } = self;
        let style = b.style().clone();
        let rows: Vec<(&T, K)> = items.into_iter().map(|item| (item, key(item))).collect();
        let tracks: Vec<Track> = columns.iter().map(|column| column.track).collect();
        let columns: Rc<[Column]> = columns.into();
        let row_count = usize::from(header.is_some()) + rows.len() + usize::from(footer.is_some());
        let column_count = columns.len();
        let classes = tw().grid().grid_cols_tracks(&tracks).w_full().sense_hover();
        let handle = b.div_id_with(&slot, classes, |b| {
            let mut rower = Rower {
                style: &style,
                columns: &columns,
                index: 0,
                bodies: 0,
            };
            if let Some(header) = header {
                rower.row(b, Band::<K>::Header, Last::No, None, header);
            }
            let last = rows.len().saturating_sub(1);
            for (position, (item, key)) in rows.iter().enumerate() {
                let picking = selection.as_ref().map(|selection| Picking {
                    key: key.clone(),
                    selected: selection.current.as_ref() == Some(key),
                    select: selection.select.clone(),
                });
                let last = if position == last {
                    Last::Yes
                } else {
                    Last::No
                };
                rower.row(b, Band::Row(key.clone()), last, picking, |cells| {
                    body(cells, item);
                });
            }
            if let Some(footer) = footer {
                rower.row(b, Band::<K>::Footer, Last::No, None, footer);
            }
        });
        let handle = handle.role(Role::Table);
        let handle = match name {
            Some(name) => handle.named(name),
            None => handle,
        };
        handle.draw(move |_, ui| {
            let id = ui.node_response().id;
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_row_count(row_count);
                node.set_column_count(column_count);
            });
        });
    }
}

/// How a body row of a selectable table takes part in the selection.
struct Picking<A, K> {
    key: K,
    selected: bool,
    select: Select<A, K>,
}

/// Adds the rows of one table in order: a band, then its cells.
struct Rower<'r> {
    style: &'r Style,
    columns: &'r Rc<[Column]>,
    index: usize,
    bodies: usize,
}

impl Rower<'_> {
    #[track_caller]
    fn row<'f, A: 'static, K: crate::Key + Clone>(
        &mut self,
        b: &mut Builder<'_, 'f, A>,
        band: Band<K>,
        last: Last,
        picking: Option<Picking<A, K>>,
        cells: impl FnOnce(&mut Cells<'_, 'f, A>),
    ) {
        let index = self.index;
        self.index += 1;
        let Ok(line) = i16::try_from(index + 1) else {
            crate::report::report(b.ctx(), crate::Problem::TableShape, None, || {
                format!(
                    "a table of more than {} rows; the rest are dropped",
                    i16::MAX - 1
                )
            });
            return;
        };
        let part = Part::of(&band);
        let stripe = match part {
            Part::Body => {
                self.bodies += 1;
                Stripe::of(self.bodies)
            }
            Part::Header | Part::Footer => Stripe::Plain,
        };
        let table = &self.style.table;
        let label = format!("{band:?}");
        let classes = band_classes(table, (part, last, stripe), picking.as_ref())
            .col_span_full()
            .row_start(line);
        let selectable = picking.as_ref().map(|picking| picking.selected);
        let handle = b
            .div_id(band, classes, |_| {})
            .role(Role::Row)
            .hover_by_rect();
        let band_id = handle.id();
        handle.draw(move |app, ui| {
            let response = ui.node_response();
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_row_index(index);
                if let Some(selected) = selectable {
                    node.set_selected(selected);
                }
            });
            if let Some(picking) = picking
                && response.clicked()
            {
                (picking.select)(app, picking.key);
            }
        });
        let mut row = Cells {
            b: b.reborrow(),
            columns: self.columns.clone(),
            style: table.clone(),
            band: band_id,
            label,
            line,
            index,
            part,
            count: 0,
        };
        cells(&mut row);
        if row.count < self.columns.len() {
            crate::report::report(
                row.b.ctx(),
                crate::Problem::TableShape,
                Some(band_id),
                || {
                    format!(
                        "row {} has {} cells for {} columns; the rest stay blank",
                        row.label,
                        row.count,
                        self.columns.len()
                    )
                },
            );
        }
    }
}

/// The classes of a row's band: its rule, its fills and, in a selectable
/// table, the clicks that select it, which take no focus.
fn band_classes<A, K>(
    table: &TableStyle,
    (part, last, stripe): (Part, Last, Stripe),
    picking: Option<&Picking<A, K>>,
) -> Tw {
    let ruled = |classes: Tw| {
        classes
            .border_b(table.rule_width)
            .border_b_color(table.rule)
    };
    match part {
        Part::Header => ruled(tw().bg(table.header_fill)).sense_hover(),
        Part::Footer => tw()
            .bg(table.footer_fill)
            .border_t(table.rule_width)
            .border_t_color(table.rule)
            .sense_hover(),
        Part::Body => {
            let classes = match last {
                Last::Yes if !table.rule_below_last => tw(),
                Last::Yes | Last::No => ruled(tw()),
            };
            let classes = match stripe {
                Stripe::Striped => classes.bg(table.stripe),
                Stripe::Plain => classes,
            };
            let classes = match picking {
                Some(picking) if picking.selected => classes.bg(table.selected),
                _ => classes.hover(|p| p.bg(table.hover)),
            };
            let classes = classes.transition(table.transition);
            match picking {
                None => classes,
                Some(_) => classes.sense_click().focusable_none(),
            }
        }
    }
}

/// TODO(prose): the cells of one row, added in column order.
pub struct Cells<'r, 'f, A> {
    b: Builder<'r, 'f, A>,
    columns: Rc<[Column]>,
    style: TableStyle,
    band: Id,
    label: String,
    line: i16,
    index: usize,
    part: Part,
    count: usize,
}

/// A cell's key: its row's band and its column.
struct CellKey<'k> {
    band: &'k str,
    column: usize,
}

impl Debug for CellKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.band, self.column)
    }
}

impl<'f, A: 'static> Cells<'_, 'f, A> {
    /// TODO(prose): the next cell, showing `text`.
    #[track_caller]
    pub fn cell_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        let shown = text.clone();
        self.add(Some(text), |b| b.text(tw(), shown));
    }

    /// TODO(prose): the next cell, holding what `body` builds.
    #[track_caller]
    pub fn cell(&mut self, body: impl FnOnce(&mut Builder<'_, 'f, A>)) {
        self.add(None, body);
    }

    #[track_caller]
    fn add(&mut self, name: Option<String>, body: impl FnOnce(&mut Builder<'_, 'f, A>)) {
        let column = self.count;
        self.count += 1;
        let (Some(spec), Ok(start)) = (self.columns.get(column), i16::try_from(column + 1)) else {
            crate::report::report(
                self.b.ctx(),
                crate::Problem::TableShape,
                Some(self.band),
                || {
                    format!(
                        "row {} has a cell past its {} columns; it is dropped",
                        self.label,
                        self.columns.len()
                    )
                },
            );
            return;
        };
        let table = &self.style;
        let placed = spec.justified(
            tw().col_start(start)
                .row_start(self.line)
                .flex()
                .items_center()
                .whitespace_nowrap(),
        );
        let placed = placed.px(table.cell_padding / 4.0);
        let placed = match table.first_column_inset {
            Some(inset) if column == 0 => placed.pl(inset / 4.0),
            _ => placed,
        };
        let cell_y = table.cell_padding_y.unwrap_or(table.cell_padding) / 4.0;
        let (classes, role) = match self.part {
            Part::Header => {
                let placed = match table.header_padding_y {
                    Some(padding) => placed.py(padding / 4.0),
                    None => placed.h_px(table.header_height),
                };
                (placed.font_weight(table.header_weight), Role::ColumnHeader)
            }
            Part::Body => (placed.py(cell_y), Role::Cell),
            Part::Footer => (
                placed.py(cell_y).font_weight(table.footer_weight),
                Role::Cell,
            ),
        };
        let key = CellKey {
            band: &self.label,
            column,
        };
        let slot = self
            .b
            .slot_of(self.band.with(("cell", column)), format!("{key:?}"));
        let (band, index) = (self.band, self.index);
        self.b
            .div_id_with(&slot, classes, |b| body(b))
            .draw(move |_, ui| describe_cell(ui, band, role, name, (index, column)));
    }
}

/// Describes the cell drawn in `ui` to assistive technology, inside its row.
/// The cell takes no input: its widget only senses hover, so the row behind
/// it keeps the clicks.
fn describe_cell(
    ui: &mut Ui,
    band: Id,
    role: Role,
    name: Option<String>,
    (row, column): (usize, usize),
) {
    let rect = ui.max_rect();
    let Some(node) = crate::draw::node_id_or_report(ui, "a table cell's description") else {
        return;
    };
    let id = node.with("cell");
    let child = ui.new_child(UiBuilder::new().max_rect(rect).accessibility_parent(band));
    let response = child.interact(rect, id, Sense::hover());
    let name = name.unwrap_or_default();
    response.widget_info(|| WidgetInfo::labeled(role, true, &name));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_row_index(row);
        node.set_column_index(column);
    });
}
