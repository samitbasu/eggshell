//! The owned segmented control: a single-choice track of cells, each a Tab
//! stop.

use std::rc::Rc;

use egui::{Context, Role};

use super::Mark;
use super::choice::{self, Choice, IntoChoice};
use super::options::Refused;
use super::options::{Configure, Interactive, Options, Resolved, Respond};
use super::{
    Enablement, OnResponse, Tip, announce_group, described, tabs, tip_describes, tipped_whole,
};
use crate::builder::Slot;
use crate::contract::Size;
use crate::focus::Autofocus;
use crate::{Builder, Tw, tw};

type Handler<A, T> = Rc<dyn Fn(&mut A, &Context, T)>;

/// Whether a cell is the chosen one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// The cell showing the current value.
    Pressed,
    /// Any other cell.
    Released,
}

/// TODO(prose): whether a segmented track keeps its cells on one row or wraps them onto more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rows {
    /// TODO(prose): one row, at the size's height.
    One,
    /// TODO(prose): as many rows as the width needs, the track as tall as they are.
    Wrapping,
}

/// A row of cells, one per option of `options` (anything [`IntoChoice`]; an icon
/// goes before the label), of which the one
/// equal to `current` is selected, drawn from the [`Style`](crate::Style). Start one with
/// [`Builder::segmented`] and finish it with [`on_change`](Self::on_change).
///
/// Clicking a cell, or pressing Enter or Space on it, runs the handler with
/// its value. Cells are identified by their position, so the options keep their order
/// for as long as a cell's state (focus, hover) should follow it.
///
/// ```html
/// <div class="flex items-center w-fit h-9 p-[3px] rounded-lg bg-muted">
///   <span class="grow flex items-center justify-center px-2 py-1 rounded-md">…</span>
/// </div>
/// ```
///
/// ```
/// # use eggshell_ui::Builder;
/// #[derive(Clone, Copy, PartialEq)]
/// enum Unit { Metric, Imperial }
/// struct App { unit: Unit }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.segmented(
///         "unit",
///         app.unit,
///         [(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
///     )
///     .on_change(|app, unit| app.unit = unit);
/// }
/// ```
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its classes are the
/// [`track`](Chain::track)'s, and its response is each cell's. A cell's own
/// tooltip shows instead of the control's.
#[must_use = "a segmented control is not added until you call `.on_change(..)`"]
pub struct Chain<'a, 'f, A, T> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    current: T,
    options: Vec<Choice<T>>,
    rows: Rows,
    opts: Options,
    size: Option<Size>,
    cells: Tw,
    custom: Option<Custom<'a>>,
    on_response: OnResponse<A>,
}

/// The classes of the track and of each cell, given instead of read from the
/// style.
struct Custom<'a> {
    track: Tw,
    cell: Box<dyn Fn(Cell) -> Tw + 'a>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T> {
    /// A control of `options` of which `current` is selected.
    pub(super) fn new<K: crate::Key, O: IntoChoice<T>>(
        b: Builder<'a, 'f, A>,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            current,
            options: choice::collect(options),
            rows: Rows::One,
            opts: Options::new(),
            size: None,
            cells: tw(),
            custom: None,
            on_response: OnResponse::default(),
        }
    }

    /// A track that wraps its cells onto more rows when it is narrower than
    /// they are, and takes its height from the rows.
    pub fn wrapping(mut self) -> Self {
        self.rows = Rows::Wrapping;
        self
    }

    /// Classes merged over the styled ones of the track, such as `w_full()`
    /// to make it fill its row; the later class wins ([`Tw::merge`]). The
    /// track is the whole control, so these are its
    /// [`classes`](Configure::classes).
    pub fn track(self, track: Tw) -> Self {
        self.classes(track)
    }

    /// Classes merged over the styled ones of every cell, such as `flex_1()`
    /// to share the track equally; the later class wins ([`Tw::merge`]).
    pub fn cells(mut self, cells: Tw) -> Self {
        self.cells = self.cells.merge(cells);
        self
    }

    /// Takes the classes of the track and of each cell from `track` and `cell`
    /// instead of the style, for a look the style does not describe. The cells
    /// sense clicks on top of them, and icons are the size of the cell's text.
    pub fn custom_classes(mut self, track: Tw, cell: impl Fn(Cell) -> Tw + 'a) -> Self {
        self.custom = Some(Custom {
            track,
            cell: Box::new(cell),
        });
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Adds the control. `select` runs with the value of the chosen cell.
    #[track_caller]
    pub fn on_change(self, select: impl Fn(&mut A, T) + 'static) {
        self.on_change_with(move |app, _, value| select(app, value));
    }

    /// Adds the control. `select` gets the application and the egui context,
    /// for a value the context holds.
    #[track_caller]
    pub fn on_change_with(self, select: impl Fn(&mut A, &Context, T) + 'static) {
        let select: Handler<A, T> = Rc::new(select);
        let Self {
            mut b,
            slot,
            current,
            options,
            rows,
            opts,
            size,
            cells: caller,
            custom,
            on_response,
        } = self;
        let Resolved {
            name,
            classes: track,
            enablement,
            tooltip,
            refused,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let group = Group {
            current,
            options,
            enablement,
            refused,
            name,
            autofocus,
        };
        if let Some(Custom { track, cell }) = custom {
            cells_in(
                b,
                slot,
                group,
                (track, tooltip),
                |state, enablement| {
                    let classes = cell(state);
                    match enablement {
                        Enablement::Enabled => classes,
                        Enablement::Disabled => classes.inert(),
                    }
                },
                tw().shrink_0(),
                (select, on_response),
            );
            return;
        }
        let size = tabs::Part::size_key(&b, b.size_of(size));
        let container = b.part(
            tabs::Part::List,
            &tabs::Part::list_keys(size, rows, enablement),
            track,
        );
        let mut trigger = |cell: Cell, enablement| {
            let keys = tabs::Part::trigger_keys(size, rows, cell, enablement);
            let own = match enablement {
                Enablement::Enabled => tw().text_center(),
                Enablement::Disabled => tw().text_center().inert(),
            };
            b.part(tabs::Part::Trigger, &keys, own.merge(caller.clone()))
        };
        let (released, pressed) = (
            trigger(Cell::Released, Enablement::Enabled),
            trigger(Cell::Pressed, Enablement::Enabled),
        );
        let (released_off, pressed_off) = (
            trigger(Cell::Released, Enablement::Disabled),
            trigger(Cell::Pressed, Enablement::Disabled),
        );
        let cell = |cell, enablement| match (cell, enablement) {
            (Cell::Released, Enablement::Enabled) => released.clone(),
            (Cell::Pressed, Enablement::Enabled) => pressed.clone(),
            (Cell::Released, Enablement::Disabled) => released_off.clone(),
            (Cell::Pressed, Enablement::Disabled) => pressed_off.clone(),
        };
        cells_in(
            b,
            slot,
            group,
            (container, tooltip),
            cell,
            tw(),
            (select, on_response),
        );
    }
}

#[track_caller]
fn cells_in<A: 'static, T: Copy + PartialEq + 'static>(
    mut b: Builder<'_, '_, A>,
    slot: Slot,
    group: Group<T>,
    (container, tooltip): (Tw, Tip),
    cell: impl Fn(Cell, Enablement) -> Tw,
    icon_classes: Tw,
    (select, on_response): (Handler<A, T>, OnResponse<A>),
) {
    let Group {
        current,
        options,
        enablement,
        refused,
        name,
        autofocus,
    } = group;
    let target = choice::focus_target(&options, &current, enablement);
    let container = if tooltip.is_some() {
        container.sense_hover()
    } else {
        container
    };
    let control_tip = tooltip.clone();
    let handle = b.div_id_with(&slot, container, |b| {
        for (index, option) in options.into_iter().enumerate() {
            let (enablement, tooltip) = option.gated(enablement, refused);
            let Choice {
                value, label, icon, ..
            } = option;
            let state = if value == current {
                Cell::Pressed
            } else {
                Cell::Released
            };
            let classes = cell(state, enablement).sense_click();
            let name = label.clone();
            let autofocus = if Some(index) == target {
                autofocus
            } else {
                Autofocus::Never
            };

            let control_tip = control_tip.clone();
            let on_response = on_response.clone();
            let select = select.clone();
            let clicked = move |app: &mut A, ui: &mut egui::Ui| {
                let mark = Mark::from(state == Cell::Pressed);
                let response = described(ui, Role::RadioButton, Some(mark), &name, &tooltip);
                autofocus.apply(ui.ctx(), &response);
                if tooltip.is_none() {
                    tip_describes(ui, &response, &control_tip);
                }
                on_response.run(app, &response);
                response.clicked()
            };
            let draw = move |app: &mut A, ui: &mut egui::Ui| {
                if clicked(app, ui) {
                    select(app, ui.ctx(), value);
                }
            };
            match icon {
                None => {
                    b.span(index, classes, label)
                        .open_focus(autofocus.open_focus())
                        .draw(draw);
                }
                Some(icon) => {
                    b.div_id(index, classes, |b| {
                        b.glyph(icon_classes.clone(), icon);
                        b.text(tw(), label);
                    })
                    .open_focus(autofocus.open_focus())
                    .draw(draw);
                }
            }
        }
    });
    handle.tile_hits().draw(move |_, ui| {
        tipped_whole(ui, &tooltip);
        announce_group(ui, Role::RadioGroup, name.as_deref());
    });
}

/// The options of a segmented control, as its chain left them.
struct Group<T> {
    current: T,
    options: Vec<Choice<T>>,
    enablement: Enablement,
    refused: Refused,
    name: Option<String>,
    autofocus: Autofocus,
}

impl<A, T> Configure for Chain<'_, '_, A, T> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, T> Interactive for Chain<'_, '_, A, T> {}

impl<A, T> Respond for Chain<'_, '_, A, T> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A segmented control over `options` (anything [`IntoChoice`]), of which
    /// `current` is selected. Values must be distinct. Assistive technology
    /// sees each cell as a `Role::RadioButton`.
    pub fn segmented<K: crate::Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, current, options)
    }
}
