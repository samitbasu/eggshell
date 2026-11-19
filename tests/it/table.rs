//! Tables through kittest's real pointer and key events: one grid whose
//! columns line up across rows, a band per row for its rule, hover and
//! selection, and the accessibility tree.

use eggshell_ui::owned::{Align, Column};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Style, TextStyle, Track, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::accesskit::Action;
use egui::epaint::Shape;
use egui::{Color32, Key, Modifiers, Rect, Role, Theme, Vec2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path};

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

fn harness(selection: Selection) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::new(selection),
        view,
    )
    .build();
    d.frames(3);
    d
}

const TABLE: &str = "/Page/Invoices";

fn band(name: &str) -> String {
    format!("{TABLE}/{name}")
}

fn cell(band: &str, column: usize) -> String {
    format!("{TABLE}/({band}, {column})")
}

fn style(d: &Driver<App>) -> eggshell_ui::contract::TableStyle {
    Style::of(d.ctx()).table.clone()
}

fn press<A>(d: &mut Driver<A>, key: Key) {
    d.harness().key_press_modifiers(Modifiers::NONE, key);
    d.frames(3);
}

/// The fill of the box last painted over the band `name`: the box, not the
/// focus ring drawn over the same rect.
fn fill_of(d: &Driver<App>, name: &str) -> Option<Color32> {
    let rect = d.rect(Target::suffix(&band(name)));
    support::shapes(d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if support::same(r.rect, rect) && r.stroke.width == 0.0 => Some(r.fill),
            _ => None,
        })
        .next_back()
}

/// Tabs into the view from past the rig's bar.
fn tab_in<A>(d: &mut Driver<A>) {
    support::drive_past_bar(d);
    press(d, Key::Tab);
}

/// Whether a strip of `color` lies along the bottom edge of `rect`.
fn rule_under(d: &Driver<App>, rect: Rect, color: Color32) -> bool {
    support::shapes(d).into_iter().any(|shape| match shape {
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
        let d = harness(Selection::Off);
        let header = d.rect(Target::suffix(&cell("Header", 0)));
        let rows = ["Row(7)", "Row(1024)", "Row(31)", "Footer"]
            .map(|band| d.rect(Target::suffix(&cell(band, 0))));
        for rect in rows {
            assert_eq!((rect.left(), rect.right()), (header.left(), header.right()));
        }
        let narrow = support::shapes(&d)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == "INV-7" => Some(t.galley.rect.width()),
                _ => None,
            })
            .expect("INV-7 is drawn");
        assert!(
            header.width() > narrow + 2.0 * style(&d).cell_padding,
            "the column is as wide as its widest cell"
        );
    }

    #[test]
    fn the_header_is_the_style_height_and_bands_span_the_table() {
        let d = harness(Selection::Off);
        let table = d.rect(Target::suffix(TABLE));
        let header = d.rect(Target::suffix(&band("Header")));
        assert!((header.height() - style(&d).header_height).abs() < 0.6);
        for name in ["Header", "Row(7)", "Row(1024)", "Row(31)", "Footer"] {
            let band = d.rect(Target::suffix(&band(name)));
            assert_eq!(
                (band.left(), band.right()),
                (table.left(), table.right()),
                "{name}"
            );
        }
        let first = d.rect(Target::suffix(&cell("Row(7)", 0)));
        let row = d.rect(Target::suffix(&band("Row(7)")));
        assert_eq!((first.top(), first.bottom()), (row.top(), row.bottom()));
    }

    #[test]
    fn an_end_column_aligns_header_and_body_text_to_its_end() {
        let d = harness(Selection::Off);
        let padding = style(&d).cell_padding;
        for (band, text) in [
            ("Header", "Amount"),
            ("Row(7)", "$250.00"),
            ("Footer", "$1,435.00"),
        ] {
            let cell = d.rect(Target::suffix(&cell(band, 2)));
            let right = support::shapes(&d)
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
        let d = harness(Selection::Off);
        let rule = style(&d).rule;
        for name in ["Header", "Row(7)", "Row(1024)"] {
            assert!(
                rule_under(&d, d.rect(Target::suffix(&band(name))), rule),
                "{name}"
            );
        }
        assert!(!rule_under(
            &d,
            d.rect(Target::suffix(&band("Row(31)"))),
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
        let mut d = support::prepared_driver(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(Selection::Off),
            view,
            striped,
        )
        .build();
        d.frames(3);
        let fills = ["Header", "Row(7)", "Row(1024)", "Row(31)", "Footer"]
            .map(|name| fill_of(&d, name) == Some(STRIPE));
        assert_eq!(fills, [false, false, true, false, false]);
    }
}

mod hover {
    use super::*;

    #[test]
    fn hovering_a_cell_fills_its_row() {
        let mut d = harness(Selection::Off);
        let hover = style(&d).hover;
        assert_ne!(fill_of(&d, "Row(1024)"), Some(hover));
        let at = d.rect(Target::suffix(&cell("Row(1024)", 0))).center();
        d.hover(at);
        d.frames(12);
        assert_eq!(fill_of(&d, "Row(1024)"), Some(hover));
        assert_ne!(fill_of(&d, "Row(7)"), Some(hover));
    }

    #[test]
    fn a_button_in_a_cell_keeps_its_row_hovered() {
        let mut d = harness(Selection::Off);
        let hover = style(&d).hover;
        let button = d.rect(Target::suffix(&format!("{TABLE}/(Row(1024), 1)/Archive")));
        assert!(
            d.rect(Target::suffix(&band("Row(1024)")))
                .contains_rect(button),
            "the button lies over the row"
        );
        d.hover(button.center());
        d.frames(12);
        assert_eq!(fill_of(&d, "Row(1024)"), Some(hover));
    }

    #[test]
    fn the_header_and_footer_do_not_hover() {
        let mut d = harness(Selection::Off);
        for name in ["Header", "Footer"] {
            let resting = fill_of(&d, name);
            let at = d.rect(Target::suffix(&band(name))).center();
            d.hover(at);
            d.frames(12);
            assert_eq!(fill_of(&d, name), resting, "{name}");
        }
    }
}

mod selection {
    use super::*;

    #[test]
    fn clicking_a_row_selects_it_and_fills_it() {
        let mut d = harness(Selection::On);
        d.click(Target::suffix(&cell("Row(31)", 0)));
        d.frames(12);
        assert_eq!(d.app().selected, Some(31));
        let selected = style(&d).selected;
        assert_eq!(fill_of(&d, "Row(31)"), Some(selected));
    }

    #[test]
    fn a_button_in_a_row_does_not_select_it() {
        let mut d = harness(Selection::On);
        d.click(Target::suffix(&format!("{TABLE}/(Row(7), 1)/Archive")));
        d.frames(12);
        assert_eq!(d.app().archived, vec![7]);
        assert_eq!(d.app().selections, Vec::<u32>::new());
    }

    #[test]
    fn tab_skips_the_rows_and_reaches_the_controls_in_their_cells() {
        let mut d = harness(Selection::On);
        d.app_mut().selected = Some(1024);
        d.frames(2);
        let archive = |row: &str| format!("{TABLE}/({row}, 1)/Archive");
        let stops = [archive("Row(7)"), archive("Row(1024)"), archive("Row(31)")];
        tab_in(&mut d);
        for (index, stop) in stops.iter().enumerate() {
            if index > 0 {
                press(&mut d, Key::Tab);
            }
            assert_eq!(focused_path(&d).as_deref(), Some(stop.as_str()));
        }
    }

    #[test]
    fn a_clicked_row_takes_no_focus_and_draws_no_ring() {
        let mut d = harness(Selection::On);
        d.click(Target::suffix(&cell("Row(1024)", 0)));
        d.frames(12);
        assert_eq!(d.app().selected, Some(1024));
        assert_eq!(focused_path(&d), None);
        let row = d.rect(Target::suffix(&band("Row(1024)")));
        let ringed = support::shapes(&d)
            .into_iter()
            .any(|shape| matches!(shape, Shape::Rect(r) if support::same(r.rect, row) && r.stroke.width > 0.0));
        assert!(!ringed, "no ring is drawn on the row");
    }

    #[test]
    fn keys_do_not_move_the_selection() {
        let mut d = harness(Selection::On);
        d.click(Target::suffix(&cell("Row(7)", 0)));
        d.frames(12);
        for key in [Key::ArrowDown, Key::End, Key::Space] {
            press(&mut d, key);
        }
        assert_eq!(d.app().selections, vec![7]);
    }
}

mod access {
    use super::*;

    #[test]
    fn the_table_rows_and_cells_are_described() {
        let d = harness(Selection::On);
        let table = d.harness().get_by_role(Role::Table);
        assert_eq!(table.accesskit_node().label().as_deref(), Some("Invoices"));
        assert_eq!(table.accesskit_node().data().row_count(), Some(5));
        assert_eq!(table.accesskit_node().data().column_count(), Some(3));
        let rows: Vec<_> = d.harness().get_all_by_role(Role::Row).collect();
        assert_eq!(rows.len(), 5);
        let indices: Vec<_> = rows
            .iter()
            .map(|row| row.accesskit_node().data().row_index())
            .collect();
        assert_eq!(indices, [0, 1, 2, 3, 4].map(Some));
        let headers: Vec<_> = d
            .harness()
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
        let amount = d
            .harness()
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
        let mut d = harness(Selection::On);
        d.app_mut().selected = Some(1024);
        d.frames(2);
        let selected: Vec<_> = d
            .harness()
            .get_all_by_role(Role::Row)
            .map(|row| row.accesskit_node().is_selected())
            .collect();
        assert_eq!(selected, [None, Some(false), Some(true), Some(false), None]);
    }

    #[test]
    fn a_selectable_row_is_not_focusable() {
        let d = harness(Selection::On);
        let focusable: Vec<_> = d
            .harness()
            .get_all_by_role(Role::Row)
            .map(|row| row.accesskit_node().data().supports_action(Action::Focus))
            .collect();
        assert_eq!(focusable, [false; 5]);
    }
}

mod misuse {
    use super::*;

    /// INV-7 has one cell and INV-1024 four, in a table of three columns.
    fn ragged(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id(Part::Page, tw().p(4.0).w_full(), |b| {
            b.table(Part::Invoices).columns(columns()).rows(
                &app.invoices,
                |invoice| invoice.id,
                |row, invoice| {
                    let cells = match invoice.id {
                        7 => 1,
                        1024 => 4,
                        _ => 3,
                    };
                    for column in 0..cells {
                        row.cell_text(format!("{}.{column}", invoice.id));
                    }
                },
            );
        });
    }

    #[test]
    fn a_row_drops_cells_past_the_columns_and_leaves_missing_ones_blank() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(Selection::Off),
            ragged,
        )
        .build();
        d.frames(3);
        let nodes = &support::nodes(&d);
        let has =
            |band: &str, column: usize| nodes.iter().any(|(path, _)| *path == cell(band, column));
        assert!(has("Row(1024)", 2), "precondition: the long row is drawn");
        assert!(!has("Row(1024)", 3), "the fourth cell is dropped");
        assert!(
            (1..3).all(|column| !has("Row(7)", column)),
            "the short row has one cell"
        );
        let table = d.rect(Target::suffix(TABLE));
        for name in ["Row(7)", "Row(1024)", "Row(31)"] {
            let band = d.rect(Target::suffix(&band(name)));
            assert_eq!(
                (band.left(), band.right()),
                (table.left(), table.right()),
                "{name}"
            );
        }
        let last = d.rect(Target::suffix(&cell("Row(1024)", 2)));
        assert_eq!(
            last.right(),
            d.rect(Target::suffix(&cell("Row(31)", 2))).right()
        );
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

    type Example = Driver<table_example::Invoices>;

    fn example() -> Example {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(760.0, 520.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            table_example::Invoices::default(),
            table_example::layout,
        )
        .build();
        d.frames(3);
        d
    }

    fn path_ending(d: &Example, end: &str) -> String {
        support::nodes(d)
            .iter()
            .find(|(path, _)| path.ends_with(end))
            .map(|(path, _)| path.clone())
            .unwrap_or_else(|| panic!("the example has a node ending {end}"))
    }

    #[test]
    fn a_click_on_a_row_selects_it() {
        let mut d = example();
        let row = path_ending(&d, "/Row(\"INV-003\")");
        d.click(Target::suffix(&row)).frames(3);
        assert_eq!(d.app().selected, Some("INV-003"));
    }

    #[test]
    fn the_checkbox_selects_its_row_by_click_and_by_keys() {
        let mut d = example();
        let pick = path_ending(&d, "(Row(\"INV-004\"), 0)/Pick/Box");
        d.click(Target::suffix(&pick)).frames(3);
        assert_eq!(d.app().selected, Some("INV-004"));
        d.click(Target::suffix(&pick)).frames(3);
        assert_eq!(d.app().selected, None);
        tab_in(&mut d);
        assert_eq!(
            focused_path(&d),
            Some(path_ending(&d, "(Row(\"INV-001\"), 0)/Pick/Box"))
        );
        press(&mut d, Key::Space);
        assert_eq!(d.app().selected, Some("INV-001"));
    }

    #[test]
    fn the_menu_button_does_not_select_its_row() {
        let mut d = example();
        let menu = path_ending(&d, "(Row(\"INV-002\"), 5)/Menu");
        d.click(Target::suffix(&menu)).frames(3);
        assert_eq!(d.app().selected, None);
    }
}

mod shape {
    use eggshell_ui::Problem;
    use eggshell_ui::testing::Headless;

    use super::*;

    #[test]
    fn a_row_of_too_few_or_too_many_cells_is_reported_and_the_extra_is_dropped() {
        let mut headless = Headless::new(vec2(400.0, 200.0));
        let rows = [1_u32, 2];
        let placed = headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _| {
            b.table("t").named("T").columns(columns()).rows(
                &rows,
                |row| *row,
                |cells, row| {
                    let texts: &[&str] = if *row == 1 {
                        &["short"]
                    } else {
                        &["a", "b", "c", "extra"]
                    };
                    for text in texts {
                        cells.cell_text(*text);
                    }
                },
            );
        });
        let messages: Vec<String> = headless
            .reports()
            .into_iter()
            .filter(|report| report.problem == Problem::TableShape)
            .map(|report| report.message)
            .collect();
        assert_eq!(messages.len(), 2, "{messages:#?}");
        assert!(
            messages[0].contains("1 cells for 3 columns"),
            "{}",
            messages[0]
        );
        assert!(
            messages[1].contains("past its 3 columns"),
            "{}",
            messages[1]
        );
        assert!(
            placed.try_at("t").is_some(),
            "precondition: the table is laid out"
        );
        let shown = |text: &str| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| placed.at("t").text(text)))
                .is_ok()
        };
        assert!(
            shown("short") && shown("c"),
            "precondition: the text query finds cells"
        );
        assert!(!shown("extra"));
    }

    #[test]
    fn a_table_of_whole_rows_reports_nothing() {
        let d = harness(Selection::Off);
        eggshell_ui::testing::assert_no_reports(d.ctx());
    }
}

/// The table fields a typeset needs (#355): its own top-bottom padding, a
/// header sized by padding, a flush first column and a rule under the last
/// row, each set in a `styled` scope.
mod spacing {
    use super::*;

    const LINE: f32 = 20.0;

    fn view<A: 'static>(b: &mut Builder<'_, '_, A>, edit: fn(&mut Style)) {
        b.styled(edit, |b| {
            b.table("t")
                .columns([Column::new(Track::auto()), Column::new(Track::auto())])
                .header(|h| {
                    h.cell_text("Name");
                    h.cell_text("Kind");
                })
                .rows(
                    &[1, 2],
                    |row| *row,
                    |cells, _| {
                        cells.cell_text("alpha");
                        cells.cell_text("beta");
                    },
                );
        });
    }

    fn harness(edit: fn(&mut Style)) -> Driver<fn(&mut Style)> {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut text = tokens.text_role();
        text.line_height = LINE;
        let mut d = support::driver(
            WINDOW,
            TextStyle::new(text, tokens.foreground),
            edit,
            |b, edit| view(b, *edit),
        )
        .build();
        d.frames(3);
        d
    }

    fn cell(d: &Driver<fn(&mut Style)>, band: &str, column: usize) -> Rect {
        d.rect(Target::suffix(&format!("/\"t\"/({band}, {column})")))
    }

    fn table(d: &Driver<fn(&mut Style)>) -> eggshell_ui::contract::TableStyle {
        Style::of(d.ctx()).table.clone()
    }

    #[test]
    fn cell_padding_y_sets_a_body_cells_top_and_bottom_only() {
        let d = harness(|style| style.table.cell_padding_y = Some(3.0));
        let padding = table(&d).cell_padding;
        assert_ne!(padding, 3.0, "precondition");
        let body = cell(&d, "Row(1)", 1);
        assert!((body.height() - (LINE + 6.0)).abs() < 0.6, "{body:?}");
        let at = d.painted().expect("beta").pos();
        let inset = at.x - body.left();
        assert!((inset - padding).abs() < 0.6, "beside: {inset}");
    }

    #[test]
    fn header_padding_y_sizes_the_header_by_its_text() {
        let d = harness(|style| style.table.header_padding_y = Some(5.0));
        assert_ne!(table(&d).header_height, LINE + 10.0, "precondition");
        let header = cell(&d, "Header", 0);
        assert!((header.height() - (LINE + 10.0)).abs() < 0.6, "{header:?}");
    }

    #[test]
    fn a_first_column_inset_moves_only_the_first_columns_text() {
        let d = harness(|style| style.table.first_column_inset = Some(0.0));
        let padding = table(&d).cell_padding;
        let (first, second) = (cell(&d, "Row(1)", 0), cell(&d, "Row(1)", 1));
        let alpha = d.painted().expect("alpha").pos();
        let beta = d.painted().expect("beta").pos();
        assert!(
            (alpha.x - first.left()).abs() < 0.6,
            "flush: {alpha:?} in {first:?}"
        );
        assert!(
            (beta.x - second.left() - padding).abs() < 0.6,
            "padded: {beta:?}"
        );
        let name = d.painted().expect("Name").pos();
        assert!(
            (name.x - cell(&d, "Header", 0).left()).abs() < 0.6,
            "the header too"
        );
    }

    #[test]
    fn rule_below_last_rules_off_the_last_row() {
        let rule = |d: &Driver<fn(&mut Style)>| {
            let last = d.rect(Target::suffix("/\"t\"/Row(2)"));
            let color = table(d).rule;
            support::shapes(d).into_iter().any(|shape| match shape {
                Shape::Rect(r) => {
                    r.fill == color
                        && (r.rect.bottom() - last.bottom()).abs() < 0.6
                        && r.rect.height() < 2.0
                }
                _ => false,
            })
        };
        assert!(!rule(&harness(|_| {})), "open by default");
        assert!(rule(&harness(|style| style.table.rule_below_last = true)));
    }
}
