//! Virtual lists: only the rows in and near the viewport are built, their
//! heights are measured and remembered, the list stays put when rows above
//! it change height, and a row that holds focus or a popup stays built.

use std::cell::Cell;
use std::collections::HashMap;

use eggshell_ui::overlay::{Anchor, Shown};
use eggshell_ui::{Builder, TextStyle, scroll, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Align, Id, Key, Rect, Theme, vec2};

use eggshell_ui::testing::{Driver, Target};

use crate::support::{self, focused_node};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Row(u32);

/// How the list is told about its rows.
#[derive(Clone, Copy, Debug)]
enum VList {
    Estimated { estimate: f32, overscan: f32 },
    Fixed { height: f32, overscan: f32 },
}

impl VList {
    fn estimated(estimate: f32) -> Self {
        Self::Estimated {
            estimate,
            overscan: 500.0,
        }
    }

    fn fixed(height: f32) -> Self {
        Self::Fixed {
            height,
            overscan: 500.0,
        }
    }

    fn overscan(self, points: f32) -> Self {
        match self {
            Self::Estimated { estimate, .. } => Self::Estimated {
                estimate,
                overscan: points,
            },
            Self::Fixed { height, .. } => Self::Fixed {
                height,
                overscan: points,
            },
        }
    }
}

/// How a row's height is decided.
#[derive(Clone, Copy, Debug)]
enum Rows {
    /// Every row this tall, unless `tall` says otherwise.
    Tall(f32),
    /// A tenth of the list's width.
    TenthOfWidth,
}

struct App {
    rows: Vec<Row>,
    list: VList,
    sizing: Rows,
    tall: HashMap<Row, f32>,
    width: f32,
    built: Cell<usize>,
    texts: HashMap<Row, String>,
    fields: bool,
    menu_on: Option<Row>,
    menu_in: Option<Row>,
    start_at_end: bool,
    /// The row whose 200-point collapsible is open.
    opened: Option<Row>,
}

impl App {
    fn new(rows: u32, list: VList, sizing: Rows) -> Self {
        Self {
            rows: (0..rows).map(Row).collect(),
            list,
            sizing,
            tall: HashMap::new(),
            width: 200.0,
            built: Cell::new(0),
            texts: HashMap::new(),
            fields: false,
            menu_on: None,
            menu_in: None,
            start_at_end: false,
            opened: None,
        }
    }

    fn height_of(&self, row: Row, width: f32) -> f32 {
        match self.sizing {
            Rows::Tall(height) => self.tall.get(&row).copied().unwrap_or(height),
            Rows::TenthOfWidth => width / 10.0,
        }
    }
}

const VIEWPORT: f32 = 300.0;

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    let list = b.virtual_list("list", tw().w_px(app.width).h_px(VIEWPORT), &app.rows);
    let list = match app.list {
        VList::Estimated { estimate, overscan } => list.estimate_px(estimate).overscan_px(overscan),
        VList::Fixed { height, overscan } => list.fixed_px(height).overscan_px(overscan),
    };
    let list = if app.start_at_end {
        list.start_at_end()
    } else {
        list
    };
    list.items(move |s, row| {
        app.built.set(app.built.get() + 1);
        if app.fields {
            s.text_field("field", app.texts.get(&row).cloned().unwrap_or_default())
                .named(format!("Row {}", row.0))
                .on_change(move |app: &mut App, text| {
                    app.texts.insert(row, text);
                });
            return;
        }
        if app.menu_in == Some(row) {
            let cell = s.id_of("cell");
            s.overlay("inner")
                .anchor(Anchor::Node(cell))
                .children(|s| s.text(tw(), "Inner"));
        }
        if row == Row(3) {
            s.collapsible("more", Shown::from(app.opened == Some(row)))
                .children(|s| s.div(tw().h_px(200.0), |_| {}));
        }
        s.custom("cell", tw().w_full())
            .measure(move |m| {
                let width = m.width().unwrap_or(0.0);
                taffy::Size {
                    width,
                    height: app.height_of(row, width),
                }
            })
            .ui(|_, _| {});
    });
    if let Some(row) = app.menu_on {
        let cell = row_cell(b, row);
        b.overlay("menu")
            .anchor(Anchor::Node(cell))
            .children(|s| s.text(tw(), "Menu"));
    }
}

/// The id of the `cell` of `row`: rows are keyed under the list.
fn row_cell(b: &mut Builder<'_, '_, App>, row: Row) -> Id {
    let mut id = None;
    b.scope("list", |s| s.scope(row, |s| id = Some(s.id_of("cell"))));
    id.expect("the scope ran")
}

fn harness_of(app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .build();
    d.frames(4);
    d
}

fn list_id(d: &Driver<App>) -> Id {
    support::id_of(d, "/\"list\"")
}

fn viewport(d: &Driver<App>) -> scroll::Viewport {
    d.shell()
        .viewport(list_id(d))
        .expect("the list was laid out")
}

/// The rows built in the last frame, in order.
fn built_rows(d: &Driver<App>) -> Vec<u32> {
    let mut rows: Vec<u32> = support::nodes(d)
        .iter()
        .filter_map(|(path, _)| {
            let last = path.rsplit('/').next()?;
            last.strip_prefix("Row(")?.strip_suffix(')')?.parse().ok()
        })
        .collect();
    rows.sort_unstable();
    rows
}

fn row_rect(d: &Driver<App>, row: u32) -> Rect {
    let suffix = format!("/Row({row})");
    support::nodes(d)
        .iter()
        .find(|(path, _)| path.ends_with(&suffix))
        .unwrap_or_else(|| panic!("row {row} is built: {:?}", built_rows(d)))
        .1
}

/// Steps one frame and returns how many rows `item` built in it.
fn step_counting(d: &mut Driver<App>) -> usize {
    d.app().built.set(0);
    d.frames(1);
    d.app().built.get()
}

fn scroll_by(d: &mut Driver<App>, y: f32) {
    d.shell().scroll_by(list_id(d), vec2(0.0, y));
    d.frames(3);
}

fn run_while_repainting(d: &mut Driver<App>, limit: usize) {
    for _ in 0..limit {
        if !d.ctx().has_requested_repaint() {
            break;
        }
        d.frames(1);
    }
}

#[test]
fn only_the_rows_in_the_viewport_and_the_overscan_are_built() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    ));
    let rows = 0..18;
    assert_eq!(
        built_rows(&d),
        rows.clone().collect::<Vec<_>>(),
        "−50 to 350 at the top"
    );
    assert_eq!(step_counting(&mut d), rows.len(), "and only they are built");
    scroll_by(&mut d, 2000.0);
    assert_eq!(viewport(&d).offset().y, 2000.0, "precondition: scrolled");
    let rows = 97..118;
    assert_eq!(
        built_rows(&d),
        rows.clone().collect::<Vec<_>>(),
        "1950 to 2350"
    );
    assert_eq!(step_counting(&mut d), rows.len());
}

#[test]
fn fixed_rows_build_the_same_rows_as_measured_ones() {
    let at = |list: VList, offset: f32| {
        let mut d = harness_of(App::new(1000, list, Rows::Tall(20.0)));
        scroll_by(&mut d, offset);
        (viewport(&d).offset().y, built_rows(&d))
    };
    for offset in [0.0, 2000.0, 7013.0, 19_700.0, 50_000.0] {
        let measured = at(VList::estimated(20.0).overscan(50.0), offset);
        assert_eq!(
            at(VList::fixed(20.0).overscan(50.0), offset),
            measured,
            "at {offset}"
        );
    }
}

#[test]
fn rows_not_yet_built_count_at_the_estimate_and_built_ones_as_measured() {
    let d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    ));
    let measured = 18;
    assert_eq!(
        viewport(&d).content.height(),
        measured as f32 * 40.0 + (1000 - measured) as f32 * 20.0,
        "the first frame built 18 rows at the estimate's offsets, and they measured 40"
    );
}

#[test]
fn a_change_of_width_forgets_the_measured_heights() {
    let mut app = App::new(
        1000,
        VList::estimated(5.0).overscan(50.0),
        Rows::TenthOfWidth,
    );
    app.width = 300.0;
    let mut d = harness_of(app);
    let wide = viewport(&d).rect.width() / 10.0;
    assert_eq!(
        viewport(&d).content.height(),
        70.0 * wide + 930.0 * 5.0,
        "precondition: the 70 rows the estimate reaches to 350 are measured"
    );
    scroll_by(&mut d, 3000.0);
    assert!(
        built_rows(&d).iter().all(|&row| row > 70),
        "precondition: rows further down are measured too"
    );
    scroll_by(&mut d, -100_000.0);
    assert_eq!(
        viewport(&d).offset().y,
        0.0,
        "precondition: back at the top"
    );
    d.app_mut().width = 200.0;
    d.frames(4);
    let row = viewport(&d).rect.width() / 10.0;
    assert_eq!(
        viewport(&d).content.height(),
        70.0 * row + 930.0 * 5.0,
        "the same 70 rows are measured anew; the rest are estimates again"
    );
}

#[test]
fn a_row_that_leaves_the_list_loses_its_height() {
    let mut d = harness_of(App::new(
        10,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    ));
    assert_eq!(
        viewport(&d).content.height(),
        400.0,
        "precondition: all measured"
    );
    d.app_mut().rows.retain(|&row| row != Row(5));
    d.frames(2);
    d.app_mut().rows.push(Row(5));
    d.frames(1);
    assert_eq!(
        viewport(&d).content.height(),
        9.0 * 40.0 + 20.0,
        "back, it is at the estimate until it is measured again"
    );
}

#[test]
fn a_row_growing_above_the_viewport_leaves_the_first_visible_row_in_place() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(100.0),
        Rows::Tall(20.0),
    ));
    scroll_by(&mut d, 1000.0);
    let top = viewport(&d).rect.top();
    let before = row_rect(&d, 50);
    assert_eq!(
        before.top(),
        top,
        "precondition: row 50 is the first visible row"
    );
    assert!(
        row_rect(&d, 48).bottom() <= top,
        "precondition: row 48 is above the viewport, and built"
    );
    d.app_mut().tall.insert(Row(48), 60.0);
    d.frames(1);
    assert_eq!(row_rect(&d, 50).top(), top, "the frame it grew in");
    run_while_repainting(&mut d, 10);
    assert_eq!(row_rect(&d, 50).top(), top, "and after");
    assert!(
        !d.ctx().has_requested_repaint(),
        "the frames stop once it is settled"
    );
}

#[test]
fn a_field_keeps_its_focus_and_cursor_while_scrolled_away_and_back() {
    let mut app = App::new(
        1000,
        VList::estimated(36.0).overscan(50.0),
        Rows::Tall(36.0),
    );
    app.fields = true;
    let mut d = harness_of(app);
    let path = support::nodes(&d)
        .iter()
        .map(|(path, _)| path.clone())
        .find(|path| path.ends_with("Row(2)/\"field\""))
        .expect("row 2's field is built");
    d.click(Target::suffix(&path));
    d.type_text("abcd").frames(1);
    d.press(Key::ArrowLeft);
    d.press(Key::ArrowLeft).frames(1);
    scroll_by(&mut d, 5000.0);
    let view = viewport(&d).rect;
    assert!(
        !built_rows(&d)
            .iter()
            .filter(|&&row| row != 2)
            .any(|&row| row < 10),
        "precondition: the rows around row 2 are no longer built"
    );
    assert!(
        !row_rect(&d, 2).intersects(view),
        "precondition: row 2 is out of view"
    );
    scroll_by(&mut d, -5000.0);
    assert_eq!(focused_node(&d).as_deref(), Some(path.as_str()));
    d.type_text("X").frames(1);
    assert_eq!(
        d.app().texts.get(&Row(2)).map(String::as_str),
        Some("abXcd")
    );
}

#[test]
fn a_row_a_popup_is_anchored_to_stays_built_while_scrolled_away() {
    let mut app = App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    );
    app.menu_on = Some(Row(3));
    let mut d = harness_of(app);
    assert!(
        support::nodes(&d)
            .iter()
            .any(|(path, _)| path.contains("menu")),
        "precondition: the menu is open"
    );
    scroll_by(&mut d, 5000.0);
    assert!(built_rows(&d).contains(&3), "row 3 is still built");
    assert!(
        support::nodes(&d)
            .iter()
            .any(|(path, _)| path.contains("menu")),
        "and the menu still open"
    );
}

#[test]
fn scroll_to_lands_the_row_in_view_after_one_correction() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    ));
    assert!(
        !built_rows(&d).contains(&500),
        "precondition: row 500 is far away"
    );
    d.shell()
        .scroll_to_row(list_id(&d), Row(500), Some(Align::Min));
    d.frames(1);
    let view = viewport(&d).rect;
    assert!(
        row_rect(&d, 500).intersects(view),
        "the frame of the request shows it"
    );
    d.frames(1);
    assert_eq!(
        row_rect(&d, 500).top(),
        view.top(),
        "and the next aligns it"
    );
}

#[test]
fn rows_are_built_under_the_list_by_their_own_key() {
    let d = harness_of(App::new(5, VList::estimated(20.0), Rows::Tall(20.0)));
    d.rect(Target::suffix("/\"list\""));
    assert_eq!(built_rows(&d), vec![0, 1, 2, 3, 4]);
}

#[test]
fn an_overlay_declared_in_a_row_stays_open_while_the_row_is_scrolled_away() {
    let mut app = App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    );
    app.menu_in = Some(Row(3));
    let mut d = harness_of(app);
    let open = |d: &Driver<App>| {
        support::nodes(d)
            .iter()
            .any(|(path, _)| path.contains("\"inner\""))
    };
    assert!(open(&d), "precondition: the overlay is open");
    scroll_by(&mut d, 5000.0);
    assert!(built_rows(&d).contains(&3), "row 3 is still built");
    assert!(open(&d), "and its overlay still open");
}

#[test]
fn rows_inserted_above_the_viewport_leave_the_first_visible_row_in_place() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    ));
    scroll_by(&mut d, 1000.0);
    let top = viewport(&d).rect.top();
    assert_eq!(row_rect(&d, 50).top(), top, "precondition: row 50 is first");
    let new: Vec<Row> = (2000..2010).map(Row).collect();
    d.app_mut().rows.splice(10..10, new);
    d.frames(1);
    assert_eq!(row_rect(&d, 50).top(), top, "the frame they arrive in");
    run_while_repainting(&mut d, 10);
    assert_eq!(row_rect(&d, 50).top(), top, "and after");
    d.app_mut().rows.drain(5..15);
    d.frames(1);
    assert_eq!(row_rect(&d, 50).top(), top, "and when rows above go");
}

#[test]
fn at_the_top_rows_inserted_above_show() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    ));
    assert_eq!(viewport(&d).offset().y, 0.0, "precondition: at the top");
    d.app_mut().rows.insert(0, Row(2000));
    d.frames(2);
    assert_eq!(viewport(&d).offset().y, 0.0);
    assert_eq!(row_rect(&d, 2000).top(), viewport(&d).rect.top());
}

#[test]
fn tab_goes_to_the_first_row_s_control_not_to_the_list() {
    let mut app = App::new(1000, VList::estimated(36.0), Rows::Tall(36.0));
    app.fields = true;
    let mut d = harness_of(app);
    support::set_companion(&d, support::Companion::Alone);
    d.press(Key::Tab).frames(1);
    let focused = focused_node(&d).expect("Tab focused something");
    assert!(
        focused.ends_with("Row(0)/\"field\""),
        "a list of controls is reached through them: {focused}"
    );
}

#[test]
fn a_zero_or_unknown_estimate_still_builds_only_a_viewport_and_overscan() {
    for estimate in [0.0, f32::NAN] {
        let tokens = Tokens::for_theme(Theme::Light);
        // Building the d runs the first frame.
        let mut d = support::driver(
            vec2(400.0, 600.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(
                100_000,
                VList::estimated(estimate).overscan(50.0),
                Rows::Tall(20.0),
            ),
            view,
        )
        .build();
        let built = d.app().built.get();
        assert!(
            built > 0 && built <= (VIEWPORT + 2.0 * 50.0) as usize,
            "estimate {estimate}: {built} rows built in the first frame"
        );
        d.frames(1);
    }
}

#[test]
fn a_scroll_by_shows_rows_in_the_frame_it_is_applied() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    ));
    d.shell().scroll_by(list_id(&d), vec2(0.0, 2000.0));
    d.frames(1);
    let view = viewport(&d);
    assert_eq!(
        view.offset().y,
        2000.0,
        "precondition: applied in this frame"
    );
    assert!(
        row_rect(&d, 100).top() == view.rect.top(),
        "the row at the new offset is built and shown: {:?}",
        built_rows(&d)
    );
}

fn last_row_ends_the_viewport(d: &Driver<App>) -> bool {
    let last = d.app().rows.last().expect("rows").0;
    let view = viewport(d).rect;
    built_rows(d).contains(&last) && (row_rect(d, last).bottom() - view.bottom()).abs() < 0.5
}

#[test]
fn a_list_that_starts_at_its_end_opens_on_its_last_row() {
    let mut app = App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    );
    app.start_at_end = true;
    let d = harness_of(app);
    assert!(
        last_row_ends_the_viewport(&d),
        "the last row ends the viewport: built {:?}",
        built_rows(&d)
    );
}

#[test]
fn a_row_added_at_the_end_shows_while_the_list_is_at_its_end() {
    let mut app = App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    );
    app.start_at_end = true;
    let mut d = harness_of(app);
    assert!(last_row_ends_the_viewport(&d), "precondition: at the end");
    d.app_mut().rows.push(Row(5000));
    d.frames(1);
    run_while_repainting(&mut d, 10);
    assert!(
        last_row_ends_the_viewport(&d),
        "the new row ends the viewport"
    );
}

#[test]
fn a_row_added_at_the_end_leaves_a_list_scrolled_away_where_it_is() {
    let mut app = App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    );
    app.start_at_end = true;
    let mut d = harness_of(app);
    scroll_by(&mut d, -2000.0);
    let offset = viewport(&d).offset().y;
    assert!(
        !last_row_ends_the_viewport(&d),
        "precondition: scrolled away"
    );
    d.app_mut().rows.push(Row(5000));
    d.frames(1);
    run_while_repainting(&mut d, 10);
    assert_eq!(viewport(&d).offset().y, offset);
}

#[test]
fn scroll_to_the_bottom_of_the_viewport_lands_there_as_rows_are_measured() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(40.0),
    ));
    d.shell()
        .scroll_to_row(list_id(&d), Row(500), Some(Align::Max));
    d.frames(4);
    let view = viewport(&d).rect;
    let row = row_rect(&d, 500);
    assert!(
        (row.bottom() - view.bottom()).abs() < 1.0,
        "row 500 {row:?} ends the viewport {view:?}"
    );
}

#[test]
fn a_row_whose_size_is_in_flight_counts_at_its_target_and_stays_built() {
    let mut d = harness_of(App::new(
        1000,
        VList::estimated(20.0).overscan(50.0),
        Rows::Tall(20.0),
    ));
    let rest = viewport(&d).content.height();
    d.app_mut().opened = Some(Row(3));
    d.frames(2);
    assert!(
        d.ctx().has_requested_repaint(),
        "precondition: the collapsible is still opening"
    );
    assert_eq!(
        viewport(&d).content.height(),
        rest + 200.0,
        "mid-flight, the content counts the row at where it is going"
    );
    scroll_by(&mut d, 5000.0);
    assert!(
        built_rows(&d).contains(&3),
        "the row stays built while it moves, scrolled away"
    );
    d.frames(40);
    assert!(
        !built_rows(&d).contains(&3),
        "and is let go once it has landed"
    );
    assert_eq!(viewport(&d).content.height(), rest + 200.0);
}

mod query {
    use eggshell_ui::testing::Headless;

    use super::*;

    fn placed() -> eggshell_ui::testing::Placed<Vec<Row>> {
        let rows: Vec<Row> = (0..50).map(Row).collect();
        Headless::new(vec2(200.0, 100.0)).layout(&rows, |b, rows| {
            b.virtual_list("list", tw().w_px(200.0).h_px(100.0), rows)
                .fixed_px(20.0)
                .items(|s, _| {
                    s.div_id("cell", tw().w_full().h_px(20.0), |_| {});
                });
        })
    }

    #[test]
    fn a_row_is_addressed_as_the_app_wrote_it() {
        let placed = placed();
        let list = placed.at("list");
        let row = list.at(Row(2));
        assert_eq!(row.at("cell").rect().height(), 20.0);
        assert!(
            row.rect().min.y > list.at(Row(1)).rect().min.y,
            "the rows follow one another"
        );
        assert!(list.try_at(Row(49)).is_none(), "a row that is not built");
    }

    #[test]
    fn a_row_node_keyed_by_the_row_is_addressed_by_it() {
        let rows: Vec<Row> = (0..50).map(Row).collect();
        let placed = Headless::new(vec2(200.0, 100.0)).layout(&rows, |b, rows| {
            b.virtual_list("list", tw().w_px(200.0).h_px(100.0), rows)
                .fixed_px(20.0)
                .items(|s, row| {
                    s.div_id(row, tw().w_full().h_px(20.0), |_| {});
                });
        });
        let row = placed.at("list").at(Row(2));
        assert_eq!(row.rect().height(), 20.0);
    }

    #[test]
    fn measured_rows_are_addressed_too() {
        let rows: Vec<Row> = (0..300).map(Row).collect();
        let placed = Headless::new(vec2(200.0, 100.0)).layout(&rows, |b, rows| {
            b.div_id("panel", tw().w_px(200.0).h_px(100.0), |b| {
                b.virtual_list("list", tw().w_px(200.0).h_px(100.0), rows)
                    .estimate_px(20.0)
                    .items(|s, row| {
                        s.div_id(row, tw().w_full().h_px(20.0), |_| {});
                    });
            });
        });
        let row = placed.at("panel").at("list").at(Row(2));
        assert_eq!(row.rect().height(), 20.0);
    }

    #[test]
    #[should_panic(expected = "a virtual list builds only the rows in and near its viewport")]
    fn a_row_that_is_not_built_says_why() {
        placed().at("list").at(Row(49));
    }
}
