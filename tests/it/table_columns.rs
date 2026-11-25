//! A table's column kinds (#371 T3): a reserved digit column that does not
//! move as its values change, a truncating note column that shows a cut
//! note's text in a tooltip, and a label column of row headers. Through the
//! real draw path and AccessKit, in the shadcn preset the rig applies.

use eggshell_ui::owned::table::Column;
use eggshell_ui::testing::assert_reported;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Problem, TextStyle, Track, tw};
use egui::epaint::Shape;
use egui::{Rect, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, hover_over};

const LONG: &str = "A note far longer than the column it sits in, which is cut short";

struct App {
    rows: Vec<(&'static str, String, &'static str)>,
}

impl App {
    fn with_values(gain: &str, trim: &str) -> Self {
        Self {
            rows: vec![
                ("Gain", gain.to_owned(), LONG),
                ("Trim", trim.to_owned(), "Fits"),
            ],
        }
    }
}

/// ```html
/// <table class="w-[320px]">
///   <thead><tr><th>Name</th><th class="text-right">Value</th><th>Note</th></tr></thead>
///   <tbody>
///     <tr><th scope="row">Gain</th><td class="text-right tabular-nums min-w-[6ch]">12</td>
///         <td class="truncate" title="…">A note far longer…</td></tr>
///   </tbody>
/// </table>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().w_px(320.0).p(2.0), |b| {
        b.table("levels")
            .columns([
                Column::new(Track::auto()).labels(),
                Column::digits(6),
                Column::new(Track::auto()).truncate(),
            ])
            .header(|h| {
                h.cell_text("Name");
                h.cell_text("Value");
                h.cell_text("Note");
            })
            .rows(
                &app.rows,
                |row| row.0,
                |cells, row| {
                    cells.cell_text(row.0);
                    cells.cell_text(row.1.clone());
                    cells.cell_text(row.2);
                },
            )
            .add();
    });
}

fn harness(app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 240.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.state_mut().companion = support::Companion::Alone;
    harness.run_steps(4);
    harness
}

fn cell(harness: &Harness<'_, Rig<App>>, role: Role, name: &str) -> Rect {
    harness.get_by_role_and_label(role, name).rect()
}

/// The drawn text `text`, if it was drawn.
fn drawn<'h>(harness: &'h Harness<'_, Rig<App>>, text: &str) -> Option<&'h egui::Galley> {
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.job.text == text => Some(&*shape.galley),
            _ => None,
        })
}

fn tooltip_open(harness: &Harness<'_, Rig<App>>) -> bool {
    harness
        .state()
        .nodes
        .iter()
        .any(|(path, _)| path.contains("Tooltip("))
}

#[test]
fn a_digit_column_keeps_the_next_column_still_as_its_values_change() {
    let narrow = harness(App::with_values("7", "1"));
    let wide = harness(App::with_values("123456", "654321"));
    let (seven, many) = (
        drawn(&narrow, "7").map(|g| g.rect.width()),
        drawn(&wide, "123456").map(|g| g.rect.width()),
    );
    assert!(
        matches!((seven, many), (Some(a), Some(b)) if b > a + 10.0),
        "precondition: the values' widths differ: {seven:?} {many:?}"
    );
    let at = |h: &Harness<'_, Rig<App>>| cell(h, Role::ColumnHeader, "Note").left();
    assert!(
        (at(&narrow) - at(&wide)).abs() < 0.5,
        "{} {}",
        at(&narrow),
        at(&wide)
    );
}

#[test]
fn a_digit_column_is_end_aligned_header_included() {
    let harness = harness(App::with_values("7", "1"));
    for (role, name) in [(Role::ColumnHeader, "Value"), (Role::Cell, "7")] {
        let cell = cell(&harness, role, name);
        let galley_right = support::shapes(&harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.job.text == name => {
                    Some(t.pos.x + t.galley.rect.right())
                }
                _ => None,
            });
        let right = galley_right.unwrap_or(f32::NAN);
        assert!(
            (cell.right() - right).abs() < 1.0,
            "{name}: text ends at {right}, cell at {}",
            cell.right()
        );
    }
}

#[test]
fn a_cut_note_shows_its_text_in_a_tooltip_and_one_that_fits_shows_none() {
    let mut harness = harness(App::with_values("7", "1"));
    let long = drawn(&harness, LONG).map(|g| g.elided);
    assert_eq!(long, Some(true), "precondition: the long note is cut");
    assert_eq!(drawn(&harness, "Fits").map(|g| g.elided), Some(false));
    let fits = cell(&harness, Role::Cell, "Fits").center();
    hover_over(&mut harness, fits);
    support::rest_for_tooltip(&mut harness);
    assert!(!tooltip_open(&harness), "a note that fits shows no tooltip");
    let long = cell(&harness, Role::Cell, LONG).center();
    hover_over(&mut harness, long);
    support::rest_for_tooltip(&mut harness);
    assert!(tooltip_open(&harness), "a cut note shows one");
    let whole = support::shapes(&harness).into_iter().any(
        |shape| matches!(shape, Shape::Text(t) if t.galley.job.text == LONG && !t.galley.elided),
    );
    assert!(whole, "the tooltip holds the whole note");
}

#[test]
fn a_label_column_names_its_rows_as_row_headers() {
    let harness = harness(App::with_values("7", "1"));
    for name in ["Gain", "Trim"] {
        harness.get_by_role_and_label(Role::RowHeader, name);
    }
    harness.get_by_role_and_label(Role::ColumnHeader, "Name");
    harness.get_by_role_and_label(Role::Cell, "7");
    assert!(
        harness
            .query_by_role_and_label(Role::Cell, "Gain")
            .is_none()
    );
}

#[test]
fn a_value_wider_than_its_reservation_is_reported() {
    let harness = harness(App::with_values("12345678", "1"));
    assert_reported(&harness.ctx, Problem::OutgrewReserve);
}
