//! TODO(prose): an invoices table with a status badge, a right-aligned
//! amount, a footer total, selectable rows and a menu on each row.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use common::glyphs::{self, Glyph};
use eggshell_ui::owned::{Align, Column};
use eggshell_ui::{Builder, Track, tw};
use eggshell_ui_shadcn::variant;

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
///   <table> <thead>…</thead> <tbody> <tr>INV-001 <span class="badge">Paid</span> … $250.00 ⋯</tr> … </tbody> <tfoot>Total $2,250.00</tfoot> </table>
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
                    Column::new(Track::fr(1.0)),
                    Column::new(Track::auto()).align(Align::End),
                    Column::new(Track::auto()).align(Align::Center),
                ])
                .header(|h| {
                    for heading in ["Invoice", "Status", "Method", "Amount", ""] {
                        h.cell_text(heading);
                    }
                })
                .footer(|f| {
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
                            b.dropdown_menu(Part::Menu, more)
                                .variant(variant::GHOST)
                                .named(format!("Actions for {id}"))
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
                );
            let selected = app.selected.unwrap_or("none");
            b.text(
                tw().text(t.muted_foreground),
                format!("Selected: {selected}"),
            );
            if let Some(action) = &app.last_action {
                b.text(tw().text(t.muted_foreground), action.clone());
            }
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("table", Invoices::default(), layout)
}
