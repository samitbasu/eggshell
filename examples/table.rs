//! TODO(prose): an invoices table with a checkbox column that selects a row,
//! a status badge, a right-aligned amount, a footer total and a menu on each
//! row; the same invoices grouped by status, striped; and a summary whose
//! labels name its rows, whose counts keep a six-digit column, and whose
//! long notes are cut with a tooltip.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

use eggshell_examples::common;

use common::glyphs::{self, Glyph};
use eggshell_ui::owned::table::{Align, Column, Stripes};
use eggshell_ui::prelude::*;
use eggshell_ui::theme::variant;
use eggshell_ui::{Builder, Track, tw};

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Paid,
    Pending,
    Unpaid,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Paid => "Paid",
            Self::Pending => "Pending",
            Self::Unpaid => "Unpaid",
        }
    }
}

pub struct Invoice {
    pub id: &'static str,
    pub status: Status,
    pub method: &'static str,
    pub cents: u64,
}

/// The invoices, the one the user selected and the last menu action.
pub struct Invoices {
    pub invoices: Vec<Invoice>,
    pub selected: Option<&'static str>,
    pub last_action: Option<String>,
}

impl Default for Invoices {
    fn default() -> Self {
        let invoice = |id, status, method, cents| Invoice {
            id,
            status,
            method,
            cents,
        };
        Self {
            invoices: vec![
                invoice("INV-001", Status::Paid, "Credit card", 25_000),
                invoice("INV-002", Status::Pending, "PayPal", 15_000),
                invoice("INV-003", Status::Unpaid, "Bank transfer", 35_000),
                invoice("INV-004", Status::Paid, "Credit card", 45_000),
                invoice("INV-005", Status::Paid, "PayPal", 55_000),
                invoice("INV-006", Status::Pending, "Bank transfer", 20_000),
                invoice("INV-007", Status::Unpaid, "Credit card", 30_000),
            ],
            selected: None,
            last_action: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Invoices,
    ByStatus,
    Summary,
    Pick,
    Menu,
    Copy,
    Void,
}

fn dollars(cents: u64) -> String {
    let (whole, part) = (cents / 100, cents % 100);
    let thousands = whole / 1000;
    if thousands > 0 {
        format!("${thousands},{:03}.{part:02}", whole % 1000)
    } else {
        format!("${whole}.{part:02}")
    }
}

/// ```html
/// <div class="p-6 flex flex-col gap-4">
///   <table> <thead>…</thead> <tbody> <tr><input type="checkbox"> INV-001 <span class="badge">Paid</span> … $250.00 ⋯</tr> … </tbody> <tfoot>Total $2,250.00</tfoot> </table>
///   <p class="text-muted-foreground">Selected: INV-003</p>
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, Invoices>, app: &Invoices) {
    let t = common::tokens(b.ctx());
    let total: u64 = app.invoices.iter().map(|invoice| invoice.cents).sum();
    b.div_id(
        Part::Page,
        tw().flex()
            .flex_col()
            .gap(4.0)
            .p(6.0)
            .size_full()
            .bg(t.background)
            .text(t.foreground),
        |b| {
            b.table(Part::Invoices)
                .named("Invoices")
                .columns([
                    Column::new(Track::auto()),
                    Column::new(Track::auto()),
                    Column::new(Track::auto()),
                    Column::new(Track::fr(1.0)),
                    Column::new(Track::auto()).align(Align::End),
                    Column::new(Track::auto()).align(Align::Center),
                ])
                .header(|h| {
                    for heading in ["", "Invoice", "Status", "Method", "Amount", ""] {
                        h.cell_text(heading);
                    }
                })
                .footer(|f| {
                    f.cell_text("");
                    f.cell_text("Total");
                    f.cell_text("");
                    f.cell_text("");
                    f.cell_text(dollars(total));
                    f.cell_text("");
                })
                .selected(app.selected)
                .on_select(|app: &mut Invoices, id| app.selected = Some(id))
                .rows(
                    &app.invoices,
                    |invoice| invoice.id,
                    |row, invoice| {
                        let id = invoice.id;
                        row.cell(|b| {
                            b.checkbox_unlabelled(Part::Pick, app.selected == Some(id))
                                .named(format!("Select {id}"))
                                .on_change(move |app: &mut Invoices, on| {
                                    app.selected = on.then_some(id);
                                });
                        });
                        row.cell(|b| b.text(tw().font_medium(), id));
                        row.cell(|b| {
                            let badge = tw()
                                .px(2.0)
                                .rounded_md()
                                .border(1.0)
                                .border_color(t.border)
                                .font_medium();
                            b.div(badge, |b| b.text(tw(), invoice.status.label()));
                        });
                        row.cell_text(invoice.method);
                        row.cell_text(dollars(invoice.cents));
                        row.cell(|b| {
                            let more = glyphs::icon(b.ctx(), Glyph::Ellipsis);
                            b.dropdown_menu(Part::Menu, (more, format!("Actions for {id}")))
                                .variant(variant::GHOST)
                                .items(|m| {
                                    m.item(Part::Copy, "Copy invoice ID").on_click(
                                        move |app: &mut Invoices| {
                                            app.last_action = Some(format!("Copied {id}"));
                                        },
                                    );
                                    m.item(Part::Void, "Void invoice").destructive().on_click(
                                        move |app: &mut Invoices| {
                                            app.last_action = Some(format!("Voided {id}"));
                                        },
                                    );
                                });
                        });
                    },
                )
                .add();
            let selected = app.selected.unwrap_or("none");
            b.text(
                tw().text(t.muted_foreground),
                format!("Selected: {selected}"),
            );
            if let Some(action) = &app.last_action {
                b.text(tw().text(t.muted_foreground), action.clone());
            }
            by_status(b, app);
            summary(b, app);
        },
    );
}

/// ```html
/// <table class="striped">
///   <tbody><tr><th scope="rowgroup" colspan="3">Pending</th></tr> <tr>INV-002 PayPal $150.00</tr> … </tbody>
///   <tbody><tr><th scope="rowgroup" colspan="3">Unpaid</th></tr> … </tbody>
///   <tbody class="mt-3"><tr>INV-001 Credit card $250.00</tr> … </tbody>
/// </table>
/// ```
fn by_status(b: &mut Builder<'_, '_, Invoices>, app: &Invoices) {
    b.table(Part::ByStatus)
        .named("Invoices by status")
        .columns([
            Column::new(Track::auto()),
            Column::new(Track::fr(1.0)),
            Column::new(Track::auto()).align(Align::End),
        ])
        .stripes(Stripes::Alternate)
        .header(|h| {
            for heading in ["Invoice", "Method", "Amount"] {
                h.cell_text(heading);
            }
        })
        .rows(
            &by_status_order(app),
            |invoice| invoice.id,
            |row, invoice| {
                row.cell_text(invoice.id);
                row.cell_text(invoice.method);
                row.cell_text(dollars(invoice.cents));
            },
        )
        .grouped_by(|invoice| (invoice.status != Status::Paid).then(|| invoice.status.label()))
        .add();
}

/// The open invoices by status, then the paid ones.
fn by_status_order(app: &Invoices) -> Vec<&Invoice> {
    [Status::Pending, Status::Unpaid, Status::Paid]
        .into_iter()
        .flat_map(|status| {
            app.invoices
                .iter()
                .filter(move |invoice| invoice.status == status)
        })
        .collect()
}

/// ```html
/// <table>
///   <tr><th scope="row">Paid</th><td class="text-right tabular-nums min-w-[6ch]">3</td>
///       <td class="truncate" title="…">Settled by card…</td></tr>
/// </table>
/// ```
fn summary(b: &mut Builder<'_, '_, Invoices>, app: &Invoices) {
    b.div(tw().w_px(440.0), |b| summary_table(b, app));
}

fn summary_table(b: &mut Builder<'_, '_, Invoices>, app: &Invoices) {
    let rows = [
        (
            Status::Paid,
            "Settled by card or bank transfer, receipts sent to each customer",
        ),
        (Status::Pending, "Awaiting transfer"),
        (
            Status::Unpaid,
            "Reminders go out on the first of the month, then every week",
        ),
    ];
    b.table(Part::Summary)
        .named("Summary")
        .columns([
            Column::new(Track::auto()).labels(),
            Column::digits(6),
            Column::new(Track::fr(1.0)).truncate(),
        ])
        .header(|h| {
            for heading in ["Status", "Count", "Note"] {
                h.cell_text(heading);
            }
        })
        .rows(
            &rows,
            |(status, _)| status.label(),
            |row, (status, note)| {
                let count = app
                    .invoices
                    .iter()
                    .filter(|invoice| invoice.status == *status)
                    .count();
                row.cell_text(status.label());
                row.cell_text(count.to_string());
                row.cell_text(*note);
            },
        )
        .add();
}

fn main() -> eframe::Result {
    common::run_app_sized(
        "table",
        egui::vec2(820.0, 1160.0),
        Invoices::default(),
        |_, _| {},
        layout,
    )
}
