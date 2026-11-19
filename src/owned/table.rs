//! The owned table: one grid holds every cell, so content-sized columns line
//! up across rows, and each row has a band spanning the grid behind its cells
//! for its rule and its fills.

use std::fmt::{self, Debug};
use std::hash::Hash;
use std::rc::Rc;

use egui::{Id, Role, Sense, Ui, UiBuilder, WidgetInfo};

use crate::builder::Slot;
use crate::contract::TableStyle;
use crate::{Builder, NodeUi as _, Track, Tw, tw};

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

/// TODO(prose): whether a table fills every second body row with the
/// style's stripe, counting afresh after each group row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Stripes {
    /// TODO(prose): no stripes.
    #[default]
    None,
    /// TODO(prose): the second, fourth, … body row of each run is striped.
    Alternate,
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
    stripes: Stripes,
    selection: S,
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
            stripes: Stripes::None,
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
        self.rows_with(each_item(items, key, body));
    }

    /// TODO(prose): adds the table, with the rows and group rows `rows` adds
    /// in order.
    ///
    /// ```
    /// # use eggshell_ui::{Builder, Track};
    /// # use eggshell_ui::owned::{Align, Column};
    /// struct Entry { key: u32, label: String, value: f64 }
    /// struct Section { title: String, entries: Vec<Entry> }
    ///
    /// fn view(b: &mut Builder<'_, '_, ()>, sections: &[Section]) {
    ///     b.table("state")
    ///         .columns([Column::new(Track::fr(1.0)), Column::new(Track::auto()).align(Align::End)])
    ///         .rows_with(|rows| {
    ///             for section in sections {
    ///                 rows.group(&section.title);
    ///                 for entry in &section.entries {
    ///                     rows.row(entry.key, |cells| {
    ///                         cells.cell_text(&entry.label);
    ///                         cells.cell_text(entry.value.to_string());
    ///                     });
    ///                 }
    ///             }
    ///         });
    /// }
    /// ```
    #[track_caller]
    pub fn rows_with<K: crate::Key + Clone + PartialEq>(
        self,
        rows: impl FnOnce(&mut Rows<'_, 'f, A, K>),
    ) {
        self.with_selection(|_| None).add(rows);
    }
}

/// A row per item of `items` under `key`, each filled by `body`.
fn each_item<'i, 'f, A: 'static, T: 'i, K: crate::Key + Clone + PartialEq>(
    items: impl IntoIterator<Item = &'i T>,
    key: impl Fn(&T) -> K,
    mut body: impl FnMut(&mut Cells<'_, 'f, A>, &T),
) -> impl FnOnce(&mut Rows<'_, 'f, A, K>) {
    move |rows| {
        for item in items {
            rows.row(key(item), |cells| body(cells, item));
        }
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

    /// TODO(prose): stripes every second body row, `Stripes::None` by default.
    pub fn stripes(mut self, stripes: Stripes) -> Self {
        self.stripes = stripes;
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
            stripes,
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
            stripes,
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
        self.rows_with(each_item(items, key, body));
    }

    /// TODO(prose): adds the table, with the rows and group rows `rows` adds
    /// in order. Group rows are never selected.
    #[track_caller]
    pub fn rows_with(self, rows: impl FnOnce(&mut Rows<'_, 'f, A, K>)) {
        self.with_selection(Some).add(rows);
    }
}

/// The band behind a row of cells, keyed by what the row shows; a group row
/// by its ordinal among the groups, so two may share a title.
#[derive(Clone, Debug, Hash)]
enum Band<K> {
    Header,
    Row(K),
    Group(usize),
    Footer,
}

/// Which part of the table a row is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Header,
    Body,
    Group,
    Footer,
}

impl Part {
    fn of<K>(band: &Band<K>) -> Self {
        match band {
            Band::Header => Self::Header,
            Band::Row(_) => Self::Body,
            Band::Group(_) => Self::Group,
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
    /// The stripe of the `nth` body row of its run, counting from one.
    fn of(nth: usize, stripes: Stripes) -> Self {
        match stripes {
            Stripes::Alternate if nth.is_multiple_of(2) => Self::Striped,
            Stripes::Alternate | Stripes::None => Self::Plain,
        }
    }
}

impl<'a, 'f, A: 'static, K: crate::Key + Clone + PartialEq>
    TableChain<'a, 'f, A, Option<Selectable<A, K>>>
{
    #[track_caller]
    fn add(self, body: impl FnOnce(&mut Rows<'_, 'f, A, K>)) {
        let Self {
            mut b,
            slot,
            name,
            columns,
            header,
            footer,
            stripes,
            selection,
        } = self;
        let table = b.style().table.clone();
        let tracks: Vec<Track> = columns.iter().map(|column| column.track).collect();
        let columns: Rc<[Column]> = columns.into();
        let column_count = columns.len();
        let classes = tw().grid().grid_cols_tracks(&tracks).w_full().sense_hover();
        let table_id = slot.id;
        let mut built = Built::default();
        let handle = b.div_id_with(&slot, classes, |b| {
            let mut rows = Rows {
                b: b.reborrow(),
                table,
                columns,
                stripes,
                selection,
                table_id,
                parent: table_id,
                index: 0,
                bodies: 0,
                groups: Vec::new(),
                pending: None,
            };
            if let Some(header) = header {
                rows.start(Band::Header, None, header);
            }
            body(&mut rows);
            rows.settle(Last::Yes);
            if let Some(footer) = footer {
                rows.parent = table_id;
                rows.start(Band::Footer, None, footer);
                rows.settle(Last::No);
            }
            built = Built {
                row_count: rows.index,
                groups: rows.groups,
            };
        });
        let handle = handle.role(Role::Table);
        let handle = match name {
            Some(name) => handle.named(name),
            None => handle,
        };
        handle.draw(move |_, ui| {
            let id = ui.node_response().id;
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_row_count(built.row_count);
                node.set_column_count(column_count);
            });
            for (ordinal, title) in built.groups.into_iter().enumerate() {
                ui.new_child(
                    UiBuilder::new()
                        .scope_id(group_id(table_id, ordinal))
                        .max_rect(ui.max_rect())
                        .accessibility_parent(id)
                        .accessibility_role(Role::RowGroup)
                        .accessibility_label(title),
                );
            }
        });
    }
}

/// What adding a table's rows found: for its accessibility node.
#[derive(Default)]
struct Built {
    row_count: usize,
    groups: Vec<String>,
}

/// The id of the accessibility node of the table's `ordinal`th row group.
fn group_id(table: Id, ordinal: usize) -> Id {
    table.with(("row group", ordinal))
}

/// How a body row of a selectable table takes part in the selection.
struct Picking<A, K> {
    key: K,
    selected: bool,
    select: Select<A, K>,
}

/// TODO(prose): the rows of a table, added in order by
/// [`TableChain::rows_with`]: body rows, and group rows that head the body
/// rows after them.
pub struct Rows<'r, 'f, A, K> {
    b: Builder<'r, 'f, A>,
    table: TableStyle,
    columns: Rc<[Column]>,
    stripes: Stripes,
    selection: Option<Selectable<A, K>>,
    table_id: Id,
    /// The table, or the row group the next rows are in.
    parent: Id,
    index: usize,
    /// The body rows since the header or the last group row.
    bodies: usize,
    groups: Vec<String>,
    /// The row whose cells are added and whose band is not: whether it is
    /// the last body row, which decides its rule, is known only once the
    /// next row starts or the rows end.
    pending: Option<Pending<A, K>>,
}

/// A row's band, waiting to be added behind the cells that are.
struct Pending<A, K> {
    /// Where the band goes among the table's children: before its cells.
    at: usize,
    slot: Slot,
    part: Part,
    stripe: Stripe,
    index: usize,
    line: i16,
    picking: Option<Picking<A, K>>,
    parent: Id,
}

impl<'f, A: 'static, K: crate::Key + Clone + PartialEq> Rows<'_, 'f, A, K> {
    /// TODO(prose): the next body row, under `key`, its cells added by
    /// `cells`.
    #[track_caller]
    pub fn row(&mut self, key: K, cells: impl FnOnce(&mut Cells<'_, 'f, A>)) {
        let picking = self.selection.as_ref().map(|selection| Picking {
            selected: selection.current.as_ref() == Some(&key),
            key: key.clone(),
            select: selection.select.clone(),
        });
        self.start(Band::Row(key), picking, cells);
    }

    /// TODO(prose): a group row titled `title`, one cell spanning the table,
    /// which heads the body rows after it up to the next group row.
    /// Assistive technology finds them in a `Role::RowGroup` named `title`.
    #[track_caller]
    pub fn group(&mut self, title: impl Into<String>) {
        let title = title.into();
        let ordinal = self.groups.len();
        self.settle(Last::No);
        self.parent = group_id(self.table_id, ordinal);
        self.bodies = 0;
        self.groups.push(title.clone());
        self.start(Band::Group(ordinal), None, |cells| cells.group_cell(title));
    }

    /// Adds the band left waiting, then starts the row `band`: its cells now,
    /// its band once the next row starts.
    #[track_caller]
    fn start(
        &mut self,
        band: Band<K>,
        picking: Option<Picking<A, K>>,
        cells: impl FnOnce(&mut Cells<'_, 'f, A>),
    ) {
        self.settle(Last::No);
        let index = self.index;
        self.index += 1;
        let Ok(line) = i16::try_from(index + 1) else {
            crate::report::report(self.b.ctx(), crate::Problem::TableShape, None, || {
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
                Stripe::of(self.bodies, self.stripes)
            }
            Part::Header | Part::Group | Part::Footer => Stripe::Plain,
        };
        let label = format!("{band:?}");
        let slot = self.b.slot(&band);
        let at = self.b.added();
        let mut row = Cells {
            b: self.b.reborrow(),
            columns: self.columns.clone(),
            style: self.table.clone(),
            band: slot.id,
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
                Some(slot.id),
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
        self.pending = Some(Pending {
            at,
            slot,
            part,
            stripe,
            index,
            line,
            picking,
            parent: self.parent,
        });
    }

    /// Adds the band left waiting, if any, before its cells.
    #[track_caller]
    fn settle(&mut self, last: Last) {
        let Some(Pending {
            at,
            slot,
            part,
            stripe,
            index,
            line,
            picking,
            parent,
        }) = self.pending.take()
        else {
            return;
        };
        let classes = band_classes(&self.table, (part, last, stripe), picking.as_ref())
            .col_span_full()
            .row_start(line);
        let selectable = picking.as_ref().map(|picking| picking.selected);
        self.b
            .div_id_with(&slot, classes, |_| {})
            .role(Role::Row)
            .hover_by_rect()
            .accessibility_parent(parent)
            .draw(move |app, ui| {
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
        self.b.move_last_to(at);
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
        Part::Group => ruled(tw().bg(table.group_fill)).sense_hover(),
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

    /// The one cell of a group row, spanning every column.
    #[track_caller]
    fn group_cell(&mut self, title: String) {
        self.count = self.columns.len();
        let table = &self.style;
        let classes = crate::owned::text_sized(&self.b, table.group_text_size).col_span_full();
        let classes = match table.group_text {
            Some(color) => classes.text(color),
            None => classes,
        };
        let shown = title.clone();
        let span = Span::Row(self.columns.len());
        self.place(classes, (0, span), Some(title), |b| b.text(tw(), shown));
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
        let classes = spec.justified(tw().col_start(start));
        self.place(classes, (column, Span::Column), name, body);
    }

    /// Adds a cell from `column` on, over `classes` that place it across the
    /// grid's columns.
    #[track_caller]
    fn place(
        &mut self,
        classes: Tw,
        (column, span): (usize, Span),
        name: Option<String>,
        body: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) {
        let table = &self.style;
        let placed = classes
            .row_start(self.line)
            .flex()
            .items_center()
            .whitespace_nowrap();
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
            Part::Group => (
                placed.py(cell_y).font_weight(table.group_weight),
                Role::RowHeader,
            ),
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
            .draw(move |_, ui| describe_cell(ui, band, (role, name), (index, column, span)));
    }
}

/// How many columns a cell covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    Column,
    /// The whole row, of this many columns.
    Row(usize),
}

/// Describes the cell drawn in `ui` to assistive technology, inside its row.
/// The cell takes no input: its widget only senses hover, so the row behind
/// it keeps the clicks.
fn describe_cell(
    ui: &mut Ui,
    band: Id,
    (role, name): (Role, Option<String>),
    (row, column, span): (usize, usize, Span),
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
        if let Span::Row(columns) = span {
            node.set_column_span(columns);
        }
    });
}
