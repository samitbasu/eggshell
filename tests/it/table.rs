//! Tables through kittest's real pointer and key events: one grid whose
//! columns line up across rows, a band per row for its rule, hover and
//! selection, and the accessibility tree.

use eggshell_ui::owned::{Align, Column};
use eggshell_ui::{Builder, Style, TextStyle, Track, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::epaint::Shape;
use egui::{Color32, Key, Modifiers, Rect, Role, Theme, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, focused_path, hover_over, rect_of};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/table.rs"]
mod table_example;

const WINDOW: Vec2 = vec2(640.0, 480.0);

struct Invoice {
    id: u32,
    status: &'static str,
    amount: &'static str,
}

struct App {
    invoices: Vec<Invoice>,
    selection: Selection,
    selected: Option<u32>,
    selections: Vec<u32>,
    archived: Vec<u32>,
}

#[derive(Clone, Copy, PartialEq)]
enum Selection {
    Off,
    On,
}

impl App {
    fn new(selection: Selection) -> Self {
        Self {
            invoices: vec![
                Invoice {
                    id: 7,
                    status: "Paid",
                    amount: "$250.00",
                },
                Invoice {
                    id: 1024,
                    status: "Pending",
                    amount: "$1,150.00",
                },
                Invoice {
                    id: 31,
                    status: "Unpaid",
                    amount: "$35.00",
                },
            ],
            selection,
            selected: None,
            selections: Vec::new(),
            archived: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Page,
    Invoices,
    Archive,
}

fn columns() -> [Column; 3] {
    [
        Column::new(Track::auto()),
        Column::new(Track::fr(1.0)),
        Column::new(Track::auto()).align(Align::End),
    ]
}

fn row(row: &mut eggshell_ui::owned::Cells<'_, '_, App>, invoice: &Invoice) {
    let id = invoice.id;
    row.cell_text(format!("INV-{id}"));
    row.cell(move |b| {
        b.text(tw(), invoice.status);
        b.button(Part::Archive, "Archive")
            .on_click(move |app: &mut App| app.archived.push(id));
    });
    row.cell_text(invoice.amount);
}

/// ```html
/// <div class="p-4">
///   <table> <thead>Invoice Status Amount</thead> <tbody>…</tbody> <tfoot>Total · $1,435.00</tfoot> </table>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(Part::Page, tw().p(4.0).w_full(), |b| {
        let table = b
            .table(Part::Invoices)
            .named("Invoices")
            .columns(columns())
            .header(|h| {
                h.cell_text("Invoice");
                h.cell_text("Status");
                h.cell_text("Amount");
            })
            .footer(|f| {
                f.cell_text("Total");
                f.cell_text("");
                f.cell_text("$1,435.00");
            });
        match app.selection {
            Selection::Off => table.rows(&app.invoices, |invoice| invoice.id, row),
            Selection::On => table
                .selected(app.selected)
                .on_select(|app: &mut App, id| {
                    app.selected = Some(id);
                    app.selections.push(id);
                })
                .rows(&app.invoices, |invoice| invoice.id, row),
        }
    });
}

fn harness(selection: Selection) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::new(selection),
        view,
    );
    harness.run_steps(3);
    harness
}

const TABLE: &str = "/Page/Invoices";

fn band(name: &str) -> String {
    format!("{TABLE}/{name}")
}

fn cell(band: &str, column: usize) -> String {
    format!("{TABLE}/({band}, {column})")
}

fn style(harness: &Harness<'_, Rig<App>>) -> eggshell_ui::contract::TableStyle {
    Style::of(&harness.ctx).table.clone()
}

fn settle(harness: &mut Harness<'_, Rig<App>>) {
    harness.run_steps(12);
}

fn press<A>(harness: &mut Harness<'_, Rig<A>>, key: Key) {
    harness.key_press_modifiers(Modifiers::NONE, key);
    harness.run_steps(3);
}

/// The fill of the box last painted over the band `name`: the box, not the
/// focus ring drawn over the same rect.
fn fill_of(harness: &Harness<'_, Rig<App>>, name: &str) -> Option<Color32> {
    let rect = rect_of(harness, &band(name));
    support::shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if support::same(r.rect, rect) && r.stroke.width == 0.0 => Some(r.fill),
            _ => None,
        })
        .next_back()
}

/// Tabs into the view from past the rig's bar.
fn tab_in<A>(harness: &mut Harness<'_, Rig<A>>) {
    support::past_bar(harness);
    press(harness, Key::Tab);
}

/// Whether a strip of `color` lies along the bottom edge of `rect`.
fn rule_under(harness: &Harness<'_, Rig<App>>, rect: Rect, color: Color32) -> bool {
    support::shapes(harness)
        .into_iter()
        .any(|shape| match shape {
            Shape::Rect(r) => {
                r.fill == color
                    && (r.rect.bottom() - rect.bottom()).abs() < 0.6
                    && r.rect.height() < 2.0
                    && (r.rect.width() - rect.width()).abs() < 0.6
            }
            _ => false,
        })
}

mod layout {
    use super::*;

    #[test]
    fn content_sized_columns_line_up_across_rows() {
        let harness = harness(Selection::Off);
        let header = rect_of(&harness, &cell("Header", 0));
        let rows = ["Row(7)", "Row(1024)", "Row(31)", "Footer"]
            .map(|band| rect_of(&harness, &cell(band, 0)));
        for rect in rows {
            assert_eq!((rect.left(), rect.right()), (header.left(), header.right()));
        }
        let narrow = support::shapes(&harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == "INV-7" => Some(t.galley.rect.width()),
                _ => None,
            })
            .expect("INV-7 is drawn");
        assert!(
            header.width() > narrow + 2.0 * style(&harness).cell_padding,
            "the column is as wide as its widest cell"
        );
    }

    #[test]
    fn the_header_is_the_style_height_and_bands_span_the_table() {
        let harness = harness(Selection::Off);
        let table = rect_of(&harness, TABLE);
        let header = rect_of(&harness, &band("Header"));
        assert!((header.height() - style(&harness).header_height).abs() < 0.6);
        for name in ["Header", "Row(7)", "Row(1024)", "Row(31)", "Footer"] {
            let band = rect_of(&harness, &band(name));
            assert_eq!(
                (band.left(), band.right()),
                (table.left(), table.right()),
                "{name}"
            );
        }
        let first = rect_of(&harness, &cell("Row(7)", 0));
        let row = rect_of(&harness, &band("Row(7)"));
        assert_eq!((first.top(), first.bottom()), (row.top(), row.bottom()));
    }

    #[test]
    fn an_end_column_aligns_header_and_body_text_to_its_end() {
        let harness = harness(Selection::Off);
        let padding = style(&harness).cell_padding;
        for (band, text) in [
            ("Header", "Amount"),
            ("Row(7)", "$250.00"),
            ("Footer", "$1,435.00"),
        ] {
            let cell = rect_of(&harness, &cell(band, 2));
            let right = support::shapes(&harness)
                .into_iter()
                .find_map(|shape| match shape {
                    Shape::Text(t) if t.galley.text() == text => {
                        Some(t.visual_bounding_rect().right())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{text} is drawn"));
            assert!(
                (right - (cell.right() - padding)).abs() < 1.5,
                "{text}: {right} in {cell:?}"
            );
        }
    }

    #[test]
    fn rows_are_ruled_except_the_last() {
        let harness = harness(Selection::Off);
        let rule = style(&harness).rule;
        for name in ["Header", "Row(7)", "Row(1024)"] {
            assert!(
                rule_under(&harness, rect_of(&harness, &band(name)), rule),
                "{name}"
            );
        }
        assert!(!rule_under(
            &harness,
            rect_of(&harness, &band("Row(31)")),
            rule
        ));
    }
}

mod stripe {
    use super::*;

    const STRIPE: Color32 = Color32::from_rgb(200, 220, 240);

    fn striped(ctx: &egui::Context, _: &App) {
        for theme in [Theme::Light, Theme::Dark] {
            let mut style = Shadcn::for_theme(theme).style();
            style.table.stripe = STRIPE;
            style.install_for(ctx, theme);
        }
    }

    #[test]
    fn a_stripe_fills_every_second_body_row() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness_prepared(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(Selection::Off),
            view,
            striped,
        );
        harness.run_steps(3);
        let fills = ["Header", "Row(7)", "Row(1024)", "Row(31)", "Footer"]
            .map(|name| fill_of(&harness, name) == Some(STRIPE));
        assert_eq!(fills, [false, false, true, false, false]);
    }
}

mod hover {
    use super::*;

    #[test]
    fn hovering_a_cell_fills_its_row() {
        let mut harness = harness(Selection::Off);
        let hover = style(&harness).hover;
        assert_ne!(fill_of(&harness, "Row(1024)"), Some(hover));
        let at = rect_of(&harness, &cell("Row(1024)", 0)).center();
        hover_over(&mut harness, at);
        settle(&mut harness);
        assert_eq!(fill_of(&harness, "Row(1024)"), Some(hover));
        assert_ne!(fill_of(&harness, "Row(7)"), Some(hover));
    }

    #[test]
    fn a_button_in_a_cell_keeps_its_row_hovered() {
        let mut harness = harness(Selection::Off);
        let hover = style(&harness).hover;
        let button = rect_of(&harness, &format!("{TABLE}/(Row(1024), 1)/Archive"));
        assert!(
            rect_of(&harness, &band("Row(1024)")).contains_rect(button),
            "the button lies over the row"
        );
        hover_over(&mut harness, button.center());
        settle(&mut harness);
        assert_eq!(fill_of(&harness, "Row(1024)"), Some(hover));
    }

    #[test]
    fn the_header_and_footer_do_not_hover() {
        let mut harness = harness(Selection::Off);
        for name in ["Header", "Footer"] {
            let resting = fill_of(&harness, name);
            let at = rect_of(&harness, &band(name)).center();
            hover_over(&mut harness, at);
            settle(&mut harness);
            assert_eq!(fill_of(&harness, name), resting, "{name}");
        }
    }
}

mod selection {
    use super::*;

    #[test]
    fn clicking_a_row_selects_it_and_fills_it() {
        let mut harness = harness(Selection::On);
        support::click_node(&mut harness, &cell("Row(31)", 0));
        settle(&mut harness);
        assert_eq!(harness.state().app.selected, Some(31));
        let selected = style(&harness).selected;
        assert_eq!(fill_of(&harness, "Row(31)"), Some(selected));
    }

    #[test]
    fn a_button_in_a_row_does_not_select_it() {
        let mut harness = harness(Selection::On);
        support::click_node(&mut harness, &format!("{TABLE}/(Row(7), 1)/Archive"));
        settle(&mut harness);
        assert_eq!(harness.state().app.archived, vec![7]);
        assert_eq!(harness.state().app.selections, Vec::<u32>::new());
    }

    #[test]
    fn tab_reaches_the_first_row_when_none_is_selected() {
        let mut harness = harness(Selection::On);
        tab_in(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(band("Row(7)").as_str())
        );
    }

    #[test]
    fn tab_reaches_the_selected_row_and_no_other() {
        let mut harness = harness(Selection::On);
        harness.state_mut().app.selected = Some(1024);
        harness.run_steps(2);
        let archive = |row: &str| format!("{TABLE}/({row}, 1)/Archive");
        let stops = [
            archive("Row(7)"),
            band("Row(1024)"),
            archive("Row(1024)"),
            archive("Row(31)"),
        ];
        tab_in(&mut harness);
        for (index, stop) in stops.iter().enumerate() {
            if index > 0 {
                press(&mut harness, Key::Tab);
            }
            assert_eq!(focused_path(&harness).as_deref(), Some(stop.as_str()));
        }
    }

    #[test]
    fn arrows_move_focus_and_selection_between_rows() {
        let mut harness = harness(Selection::On);
        tab_in(&mut harness);
        press(&mut harness, Key::ArrowDown);
        assert_eq!(harness.state().app.selected, Some(1024));
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(band("Row(1024)").as_str())
        );
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::ArrowDown);
        assert_eq!(
            harness.state().app.selected,
            Some(31),
            "Down stops at the last row"
        );
        press(&mut harness, Key::ArrowUp);
        assert_eq!(harness.state().app.selected, Some(1024));
        press(&mut harness, Key::Home);
        assert_eq!(harness.state().app.selected, Some(7));
        press(&mut harness, Key::End);
        assert_eq!(harness.state().app.selected, Some(31));
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(band("Row(31)").as_str())
        );
    }

    #[test]
    fn space_selects_the_focused_row() {
        let mut harness = harness(Selection::On);
        tab_in(&mut harness);
        press(&mut harness, Key::Space);
        assert_eq!(harness.state().app.selected, Some(7));
    }

    #[test]
    fn a_focused_row_shows_the_focus_ring() {
        let mut harness = harness(Selection::On);
        tab_in(&mut harness);
        settle(&mut harness);
        let row = rect_of(&harness, &band("Row(7)"));
        let ring = Style::of(&harness.ctx).focus;
        let drawn = support::shapes(&harness)
            .into_iter()
            .any(|shape| match shape {
                Shape::Rect(r) => {
                    support::same(r.rect, row)
                        && r.stroke.width == ring.width
                        && r.stroke.color == ring.color
                }
                _ => false,
            });
        assert!(drawn, "the ring is drawn on the row");
    }

    #[test]
    fn rows_without_selection_are_not_tab_stops() {
        let mut harness = harness(Selection::Off);
        tab_in(&mut harness);
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(format!("{TABLE}/(Row(7), 1)/Archive").as_str())
        );
    }
}

mod access {
    use super::*;

    #[test]
    fn the_table_rows_and_cells_are_described() {
        let harness = harness(Selection::On);
        let table = harness.get_by_role(Role::Table);
        assert_eq!(table.accesskit_node().label().as_deref(), Some("Invoices"));
        assert_eq!(table.accesskit_node().data().row_count(), Some(5));
        assert_eq!(table.accesskit_node().data().column_count(), Some(3));
        let rows: Vec<_> = harness.get_all_by_role(Role::Row).collect();
        assert_eq!(rows.len(), 5);
        let indices: Vec<_> = rows
            .iter()
            .map(|row| row.accesskit_node().data().row_index())
            .collect();
        assert_eq!(indices, [0, 1, 2, 3, 4].map(Some));
        let headers: Vec<_> = harness
            .get_all_by_role(Role::ColumnHeader)
            .map(|header| {
                (
                    header.accesskit_node().label(),
                    header.accesskit_node().data().column_index(),
                )
            })
            .collect();
        assert_eq!(
            headers,
            [("Invoice", 0), ("Status", 1), ("Amount", 2)]
                .map(|(name, index)| (Some(name.to_owned()), Some(index)))
        );
        let amount = harness
            .get_all_by_role(Role::Cell)
            .find(|cell| cell.accesskit_node().label().as_deref() == Some("$35.00"))
            .expect("the amount cell");
        assert_eq!(
            (
                amount.accesskit_node().data().row_index(),
                amount.accesskit_node().data().column_index()
            ),
            (Some(3), Some(2))
        );
    }

    #[test]
    fn a_selectable_row_reports_whether_it_is_selected() {
        let mut harness = harness(Selection::On);
        harness.state_mut().app.selected = Some(1024);
        harness.run_steps(2);
        let selected: Vec<_> = harness
            .get_all_by_role(Role::Row)
            .map(|row| row.accesskit_node().is_selected())
            .collect();
        assert_eq!(selected, [None, Some(false), Some(true), Some(false), None]);
    }
}

mod misuse {
    use super::*;

    fn short(b: &mut Builder<'_, '_, App>, app: &App) {
        b.table(Part::Invoices).columns(columns()).rows(
            &app.invoices,
            |invoice| invoice.id,
            |row, invoice| {
                row.cell_text(format!("INV-{}", invoice.id));
            },
        );
    }

    #[test]
    #[should_panic(expected = "row 7 has 1 cell, the table has 3 columns")]
    fn a_row_with_the_wrong_number_of_cells_panics() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(Selection::Off),
            short,
        );
        harness.run_steps(1);
    }
}

mod preset {
    use super::*;

    #[test]
    fn shadcn_draws_its_table() {
        let preset = Shadcn::for_theme(Theme::Light);
        let t = &preset.tokens;
        let table = preset.style().table;
        assert_eq!(table.header_fill, Color32::TRANSPARENT);
        assert_eq!(table.header_height, 40.0);
        assert_eq!(table.cell_padding, 8.0);
        assert_eq!((table.rule, table.rule_width), (t.border, 1.0));
        assert_eq!(table.stripe, Color32::TRANSPARENT);
        assert_eq!(table.hover, t.muted.gamma_multiply(0.5));
        assert_eq!(table.selected, t.muted);
        assert_eq!(table.footer_fill, t.muted.gamma_multiply(0.5));
    }
}

mod example {
    use super::*;

    #[test]
    fn the_example_selects_a_row_by_click_and_by_keys() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            vec2(720.0, 520.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            table_example::Invoices::default(),
            table_example::layout,
        );
        harness.run_steps(3);
        let row = harness
            .state()
            .nodes
            .iter()
            .find(|(path, _)| path.ends_with("/Row(\"INV-003\")"))
            .map(|(path, _)| path.clone())
            .expect("the example shows INV-003");
        support::click_node(&mut harness, &row);
        harness.run_steps(3);
        assert_eq!(harness.state().app.selected, Some("INV-003"));
        press(&mut harness, Key::ArrowDown);
        assert_eq!(harness.state().app.selected, Some("INV-004"));
    }
}
