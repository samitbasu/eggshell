//! The owned segmented control: a single-choice track of cells, each a Tab
//! stop.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Context, Role};

use super::Mark;
use super::{Enablement, centered, described};
use crate::{Builder, Style, Tw, none, tw};

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
///     Segmented::new(
///         app.unit,
///         &[(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
///     )
///     .show(b, "unit", |app, _, unit| app.unit = unit);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Segmented<T> {
    current: T,
    options: Vec<(T, String)>,
    rows: Rows,
    enablement: Enablement,
}

impl<T: Copy + PartialEq + Hash + Debug + 'static> Segmented<T> {
    /// A control of `(value, label)` options of which `current` is selected.
    pub fn new(current: T, options: &[(T, &str)]) -> Self {
        Self {
            current,
            options: options
                .iter()
                .map(|&(value, label)| (value, label.to_owned()))
                .collect(),
            rows: Rows::One,
            enablement: Enablement::Enabled,
        }
    }

    /// A track that wraps its cells onto more rows when it is narrower than
    /// they are, and takes its height from the rows.
    #[must_use]
    pub fn wrapping(mut self) -> Self {
        self.rows = Rows::Wrapping;
        self
    }

    /// Whether the control takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the control to `b` under `key`. `select` gets the application and
    /// the context, for a value the context holds.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
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
            Rows::One => track.h_px(seg.height),
            Rows::Wrapping => track.flex_wrap().h_auto().gap(seg.row_gap / 4.0),
        };
        let (rows, enablement) = (self.rows, self.enablement);
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
            .px(seg.cell_padding.x / 4.0)
            .py(seg.cell_padding.y / 4.0)
            .role(style.text.clone());
            match rows {
                Rows::One => classes.h_full(),
                Rows::Wrapping => classes.h_auto(),
            }
        };
        self.show_with(b, key, track, cell, select);
    }

    /// [`show`](Self::show) with the classes of the track and of each cell
    /// given instead of read from the style, for a look the style does not
    /// describe. The cells sense clicks on top of them.
    pub fn show_with<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        container: Tw,
        cell: impl Fn(Cell) -> Tw,
        select: impl Fn(&mut A, &Context, T) + Copy + 'static,
    ) {
        b.div_id(key, container, none, |b| {
            for (value, label) in self.options {
                let state = if value == self.current {
                    Cell::Pressed
                } else {
                    Cell::Released
                };
                let name = label.clone();
                b.span(value, cell(state).sense_click(), label, move |app, ui| {
                    let mark = Mark::from(state == Cell::Pressed);
                    if described(ui, Role::RadioButton, Some(mark), &name).clicked() {
                        select(app, ui.ctx(), value);
                    }
                });
            }
        });
    }
}
