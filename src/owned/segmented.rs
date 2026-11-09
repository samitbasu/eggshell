//! The owned segmented control: a single-choice track of cells, each a Tab
//! stop.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Context, Role};

use super::Mark;
use super::{Enablement, centered, described};
use crate::{Builder, Icon, Style, Tw, none, tw};

/// Whether a cell is the chosen one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// The cell showing the current value.
    Pressed,
    /// Any other cell.
    Released,
}

/// Whether a track keeps its cells on one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rows {
    One,
    Wrapping,
}

/// A row of cells, one per `(value, label)` of `options`, of which the one
/// equal to `current` is selected, drawn from the [`Style`].
///
/// Clicking a cell, or pressing Enter or Space on it, runs `select` with its
/// value. Cells are identified by their value, so values must be distinct.
///
/// ```html
/// <div class="flex items-center w-fit h-9 p-[3px] rounded-lg bg-muted">
///   <span class="grow flex items-center justify-center px-2 py-1 rounded-md">…</span>
/// </div>
/// ```
///
/// ```
/// # use bullseye::{Builder, owned::Segmented};
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
/// enum Unit { Metric, Imperial }
/// struct App { unit: Unit }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.segmented(
///         "unit",
///         Segmented::new(
///             app.unit,
///             &[(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
///         ),
///         |app, _, unit| app.unit = unit,
///     );
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Segmented<T> {
    current: T,
    options: Vec<Option_<T>>,
    rows: Rows,
    enablement: Enablement,
    track: Tw,
    cells: Tw,
}

#[derive(Clone, Debug)]
struct Option_<T> {
    value: T,
    label: String,
    icon: Option<Icon>,
}

impl<T: Copy + PartialEq + Hash + Debug + 'static> Segmented<T> {
    /// A control of `(value, label)` options of which `current` is selected.
    pub fn new(current: T, options: &[(T, &str)]) -> Self {
        Self {
            current,
            options: options
                .iter()
                .map(|&(value, label)| Option_ {
                    value,
                    label: label.to_owned(),
                    icon: None,
                })
                .collect(),
            rows: Rows::One,
            enablement: Enablement::Enabled,
            track: tw(),
            cells: tw(),
        }
    }

    /// Puts `icon` before the label of the option `value`, tinted by the
    /// cell's text colour in each state and sized by the style.
    #[must_use]
    pub fn icon(mut self, value: T, icon: Icon) -> Self {
        let option = self.options.iter_mut().find(|option| option.value == value);
        debug_assert!(option.is_some(), "no option {value:?} to give an icon");
        if let Some(option) = option {
            option.icon = Some(icon);
        }
        self
    }

    /// A track that wraps its cells onto more rows when it is narrower than
    /// they are, and takes its height from the rows.
    #[must_use]
    pub fn wrapping(mut self) -> Self {
        self.rows = Rows::Wrapping;
        self
    }

    /// Classes merged over the styled ones of the track, such as `w_full()`
    /// to make it fill its row; the later class wins ([`Tw::merge`]).
    #[must_use]
    pub fn track(mut self, track: Tw) -> Self {
        self.track = track;
        self
    }

    /// Classes merged over the styled ones of every cell, such as `flex_1()`
    /// to share the track equally; the later class wins ([`Tw::merge`]).
    #[must_use]
    pub fn cells(mut self, cells: Tw) -> Self {
        self.cells = cells;
        self
    }

    /// The classes of the whole control: the same as [`track`](Self::track).
    #[must_use]
    pub fn classes(self, classes: Tw) -> Self {
        self.track(classes)
    }

    /// Whether the control takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the control to `b` under `key`. `select` gets the application and
    /// the context, for a value the context holds.
    pub(crate) fn show<A: 'static, K: crate::Key>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        select: impl Fn(&mut A, &Context, T) + Copy + 'static,
    ) {
        let style = Style::of(b.ctx());
        let seg = &style.segmented;
        let track = tw()
            .flex()
            .items_center()
            .w_fit()
            .p(seg.track_padding / 4.0)
            .rounded_px(f32::from(seg.track_radius))
            .bg(seg.track_fill);
        let track = match self.rows {
            Rows::One => track.h_px(seg.height).gap_x(seg.column_gap / 4.0),
            Rows::Wrapping => track.flex_wrap().h_auto().gap(seg.row_gap / 4.0),
        };
        let track = self.enablement.fade(track, &style).merge(self.track);
        let (rows, enablement, caller) = (self.rows, self.enablement, self.cells);
        let cell = |cell| {
            let look = match cell {
                Cell::Pressed => &seg.selected,
                Cell::Released => &seg.unselected,
            };
            let classes = centered(look.classes(
                seg.cell_border_width,
                &style,
                seg.cell_radius,
                enablement,
                seg.transition,
            ))
            .grow()
            .text_center()
            .px(seg.cell_padding.x / 4.0)
            .py(seg.cell_padding.y / 4.0)
            .gap(seg.icon_gap / 4.0)
            .role(style.role(&seg.text).clone());
            let classes = match rows {
                Rows::One => classes.h_full(),
                Rows::Wrapping => classes.h_auto(),
            };
            classes.merge(caller.clone())
        };
        let icon = tw().size_px(seg.icon_size);
        cells_in(
            b,
            key,
            (self.current, self.options),
            track,
            cell,
            icon,
            select,
        );
    }

    /// [`show`](Self::show) with the classes of the track and of each cell
    /// given instead of read from the style, for a look the style does not
    /// describe. The cells sense clicks on top of them, and icons are the
    /// size of the cell's text.
    pub(crate) fn show_with<A: 'static, K: crate::Key>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        container: Tw,
        cell: impl Fn(Cell) -> Tw,
        select: impl Fn(&mut A, &Context, T) + Copy + 'static,
    ) {
        cells_in(
            b,
            key,
            (self.current, self.options),
            container,
            cell,
            tw(),
            select,
        );
    }
}

fn cells_in<A: 'static, K: crate::Key, T: Copy + PartialEq + Hash + Debug + 'static>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    (current, options): (T, Vec<Option_<T>>),
    container: Tw,
    cell: impl Fn(Cell) -> Tw,
    icon_classes: Tw,
    select: impl Fn(&mut A, &Context, T) + Copy + 'static,
) {
    b.div_id(key, container, none, |b| {
        for Option_ { value, label, icon } in options {
            let state = if value == current {
                Cell::Pressed
            } else {
                Cell::Released
            };
            let classes = cell(state).sense_click();
            let name = label.clone();
            let clicked = move |ui: &mut egui::Ui| {
                let mark = Mark::from(state == Cell::Pressed);
                described(ui, Role::RadioButton, Some(mark), &name).clicked()
            };
            match icon {
                None => b.span(value, classes, label, move |app, ui| {
                    if clicked(ui) {
                        select(app, ui.ctx(), value);
                    }
                }),
                Some(icon) => b.div_id(
                    value,
                    classes,
                    move |app, ui| {
                        if clicked(ui) {
                            select(app, ui.ctx(), value);
                        }
                    },
                    |b| {
                        b.glyph(icon_classes.clone().shrink_0(), icon);
                        b.text(tw(), label);
                    },
                ),
            }
        }
    });
}
