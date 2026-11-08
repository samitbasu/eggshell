//! A segmented control: a single-choice group built from identified nodes.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Role, WidgetInfo};

use crate::{Builder, NodeUi as _, Tw, none};

/// Whether a cell is the chosen one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// The cell showing the current value: the one pressed cell.
    Pressed,
    /// Any other cell.
    Released,
}

/// A row of cells, one per `(value, label)` of `options`, of which exactly
/// the one equal to `current` is [`Cell::Pressed`]. Clicking a cell, or
/// pressing Enter or Space on it, writes its value through `lens`.
///
/// `key` identifies the group and each cell is identified by its value, so
/// values must be distinct. `cell` gives each cell's classes; the cells sense
/// clicks on top of them. Read `current` from the application when building,
/// as in the example, since the lens needs `&mut A`:
///
/// ```html
/// <div class="flex gap-1">
///   <span class="px-3 underline">Metric</span>  <!-- pressed -->
///   <span class="px-3">Imperial</span>
/// </div>
/// ```
///
/// ```
/// # use bullseye::{Builder, tw};
/// # use bullseye::prim::segmented::{Cell, segmented};
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
/// enum Unit { Metric, Imperial }
/// struct App { unit: Unit }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     segmented(
///         b,
///         "unit",
///         tw().flex().gap(1.0),
///         app.unit,
///         &[(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
///         |cell| match cell {
///             Cell::Pressed => tw().px(3.0).underline(),
///             Cell::Released => tw().px(3.0),
///         },
///         |app| &mut app.unit,
///     );
/// }
/// ```
pub fn segmented<A, K, T>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    container: Tw,
    current: T,
    options: &[(T, &str)],
    cell: impl Fn(Cell) -> Tw,
    lens: fn(&mut A) -> &mut T,
) where
    A: 'static,
    K: Hash + Debug + 'static,
    T: Copy + PartialEq + Hash + Debug + 'static,
{
    segmented_by(
        b,
        key,
        container,
        current,
        options,
        cell,
        move |app, _, value| *lens(app) = value,
    );
}

/// [`segmented`] for a value that does not live in the application, such as
/// one held by the egui context: clicking a cell calls `select` with its
/// value instead of writing through a lens.
pub fn segmented_by<A, K, T>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    container: Tw,
    current: T,
    options: &[(T, &str)],
    cell: impl Fn(Cell) -> Tw,
    select: impl Fn(&mut A, &egui::Context, T) + Copy + 'static,
) where
    A: 'static,
    K: Hash + Debug + 'static,
    T: Copy + PartialEq + Hash + Debug + 'static,
{
    b.div_id(key, container, none, |b| {
        for &(value, label) in options {
            let state = if value == current {
                Cell::Pressed
            } else {
                Cell::Released
            };
            let label_text = label.to_owned();
            b.span(value, cell(state).sense_click(), label, move |app, ui| {
                let response = ui.node_response();
                if response.clicked() {
                    select(app, ui.ctx(), value);
                }
                response.widget_info(|| {
                    WidgetInfo::selected(
                        Role::RadioButton,
                        ui.is_enabled(),
                        state == Cell::Pressed,
                        &label_text,
                    )
                });
            });
        }
    });
}
