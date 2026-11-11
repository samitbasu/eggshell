//! The owned segmented control: a single-choice track of cells, each a Tab
//! stop.

use std::rc::Rc;

use egui::{Context, Role};

use super::Mark;
use super::choice::{self, Choice, IntoChoice};
use super::{Enablement, OnResponse, Tip, centered, described, tipped};
use crate::builder::Slot;
use crate::{Builder, NodeUi as _, Style, Tw, tw};

type Handler<A, T> = Rc<dyn Fn(&mut A, &Context, T)>;

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

/// A row of cells, one per option of `options` (anything [`IntoChoice`]; an icon
/// goes before the label), of which the one
/// equal to `current` is selected, drawn from the [`Style`]. Start one with
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
#[must_use = "a segmented control is not added until you call `.on_change(..)`"]
pub struct SegmentedChain<'a, 'f, A, T> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    current: T,
    options: Vec<Choice<T>>,
    rows: Rows,
    enablement: Enablement,
    tooltip: Tip,
    track: Tw,
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

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> SegmentedChain<'a, 'f, A, T> {
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
            enablement: Enablement::Enabled,
            tooltip: None,
            track: tw(),
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
    /// to make it fill its row; the later class wins ([`Tw::merge`]).
    pub fn track(mut self, track: Tw) -> Self {
        self.track = track;
        self
    }

    /// Classes merged over the styled ones of every cell, such as `flex_1()`
    /// to share the track equally; the later class wins ([`Tw::merge`]).
    pub fn cells(mut self, cells: Tw) -> Self {
        self.cells = cells;
        self
    }

    /// The classes of the whole control: the same as [`track`](Self::track).
    pub fn classes(self, classes: Tw) -> Self {
        self.track(classes)
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

    /// Runs `then` with the response of each cell each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Shown while the pointer hovers the control, as egui's `on_hover_text`
    /// shows it: with egui's timing and style, and also when the control is
    /// disabled, as an HTML `title` is. It adds to the control's accessible
    /// name and does not replace it. A cell's own tooltip shows instead of it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Whether the control takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
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
            b,
            slot,
            current,
            options,
            rows,
            enablement,
            tooltip,
            track,
            cells: caller,
            custom,
            on_response,
        } = self;
        let enablement = b.enablement_of(enablement);
        if let Some(Custom { track, cell }) = custom {
            cells_in(
                b,
                slot,
                (current, options, enablement),
                (track, tooltip),
                |state, enablement| {
                    let classes = cell(state);
                    match enablement {
                        Enablement::Enabled => classes,
                        Enablement::Disabled => classes.inert(),
                    }
                },
                tw(),
                (select, on_response),
            );
            return;
        }
        let style = Style::of(b.ctx());
        let seg = &style.segmented;
        let container = tw()
            .flex()
            .items_center()
            .w_fit()
            .p(seg.track_padding / 4.0)
            .rounded_px(f32::from(seg.track_radius))
            .bg(seg.track_fill);
        let container = match rows {
            Rows::One => container.h_px(seg.height).gap_x(seg.column_gap / 4.0),
            Rows::Wrapping => container.flex_wrap().h_auto().gap(seg.row_gap / 4.0),
        };
        let container = enablement.fade(container, &style).merge(track);
        let cell = |cell, enablement| {
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
            slot,
            (current, options, enablement),
            (container, tooltip),
            cell,
            icon,
            (select, on_response),
        );
    }
}

#[track_caller]
fn cells_in<A: 'static, T: Copy + PartialEq + 'static>(
    mut b: Builder<'_, '_, A>,
    slot: Slot,
    (current, options, group): (T, Vec<Choice<T>>, Enablement),
    (container, tooltip): (Tw, Tip),
    cell: impl Fn(Cell, Enablement) -> Tw,
    icon_classes: Tw,
    (select, on_response): (Handler<A, T>, OnResponse<A>),
) {
    let container = if tooltip.is_some() {
        container.sense_hover()
    } else {
        container
    };
    let control_tip = tooltip.clone();
    let handle = b.div_id_with(slot.id, &slot.label, container, |b| {
        for (
            index,
            Choice {
                value,
                label,
                icon,
                tooltip,
                enablement,
            },
        ) in options.into_iter().enumerate()
        {
            let state = if value == current {
                Cell::Pressed
            } else {
                Cell::Released
            };
            let classes = cell(state, group.and(enablement)).sense_click();
            let name = label.clone();
            let tooltip = tooltip.or_else(|| control_tip.clone());
            let on_response = on_response.clone();
            let select = select.clone();
            let clicked = move |app: &mut A, ui: &mut egui::Ui| {
                let mark = Mark::from(state == Cell::Pressed);
                let response = described(ui, Role::RadioButton, Some(mark), &name, &tooltip);
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
                    b.span(index, classes, label).draw(draw);
                }
                Some(icon) => {
                    b.div_id(index, classes, |b| {
                        b.glyph(icon_classes.clone().shrink_0(), icon);
                        b.text(tw(), label);
                    })
                    .draw(draw);
                }
            }
        }
    });
    if tooltip.is_some() {
        handle.draw(move |_, ui| {
            tipped(ui.node_response(), &tooltip);
        });
    }
}
