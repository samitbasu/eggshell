//! The chat shell through the full path: pointer events, hit testing, drag
//! handling in the draw closures, and the geometry the layout promises.

#![allow(
    clippy::duplicate_mod,
    reason = "each test crate includes the shared examples modules"
)]

use egui::{CursorIcon, Vec2, vec2};
use egui_kittest::Harness;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/chat_shell/main.rs"]
mod chat_shell;

mod support;

use chat_shell::sidebar::Link;
use chat_shell::state::{App, Pane, SessionKey};
use chat_shell::theme::Tok;
use chat_shell::view::{Part, layout};
use support::{Rig, click_node, drag, fill_of, hover_node, hover_over, rect_of};

const WINDOW: Vec2 = vec2(1200.0, 760.0);

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    support::harness(WINDOW, app.theme.base_text(), app, layout)
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(App::default())
}

fn path(part: Part) -> String {
    format!("/{part:?}")
}

fn session_path(key: SessionKey) -> String {
    format!("/Sidebar/{key:?}")
}

fn rect(harness: &Harness<'_, Rig<App>>, part: Part) -> egui::Rect {
    rect_of(harness, &path(part))
}

fn width(harness: &Harness<'_, Rig<App>>, pane: Pane) -> f32 {
    harness.state().app.width(pane).into()
}

fn color(harness: &Harness<'_, Rig<App>>, token: Tok) -> egui::Color32 {
    harness.state().app.theme[token]
}

/// Drags the splitter of `pane` by `dx` points.
fn drag_splitter(harness: &mut Harness<'_, Rig<App>>, pane: Pane, dx: f32) {
    let from = rect(harness, Part::Splitter(pane)).center();
    drag(harness, from, from + vec2(dx, 0.0));
}

#[test]
fn the_three_panes_and_two_splitters_fill_the_window() {
    let harness = harness();
    let root = rect_of(&harness, "/<div>");
    let columns = [
        Part::Sidebar,
        Part::Splitter(Pane::Sidebar),
        Part::Centre,
        Part::Splitter(Pane::Details),
        Part::Details,
    ]
    .map(|part| rect(&harness, part));

    assert!(
        columns[2].width() > 300.0,
        "precondition: a real centre pane"
    );
    assert_eq!(columns[0].min.x, root.min.x);
    assert_eq!(columns[4].max.x, root.max.x);
    for pair in columns.windows(2) {
        assert_eq!(pair[0].max.x, pair[1].min.x, "no gap between neighbours");
    }
    for column in columns {
        assert_eq!((column.min.y, column.max.y), (root.min.y, root.max.y));
    }
    assert_eq!(columns[0].width(), width(&harness, Pane::Sidebar));
    assert_eq!(columns[4].width(), width(&harness, Pane::Details));
}

#[test]
fn the_composer_is_pinned_to_the_bottom_of_the_centre_pane() {
    let pinned = |harness: &Harness<'_, Rig<App>>| {
        rect(harness, Part::Centre).max.y - rect_of(harness, "/Centre/Composer").max.y
    };
    let mut harness = harness();
    let full = pinned(&harness);
    assert_eq!(full, 16.0, "the pane's bottom padding");
    assert!(
        rect_of(&harness, "/Centre/Conversation").max.y
            <= rect_of(&harness, "/Centre/Composer").min.y,
        "precondition: the conversation sits above the composer"
    );

    harness.state_mut().app.conversation.clear();
    harness.run_steps(2);
    assert_eq!(
        pinned(&harness),
        full,
        "an empty conversation does not lift it"
    );
}

#[test]
fn dragging_a_splitter_resizes_its_pane() {
    let mut harness = harness();
    let before = width(&harness, Pane::Sidebar);

    drag_splitter(&mut harness, Pane::Sidebar, 40.0);
    assert_eq!(width(&harness, Pane::Sidebar), before + 40.0);
    assert_eq!(rect(&harness, Part::Sidebar).width(), before + 40.0);
}

#[test]
fn the_details_splitter_grows_its_pane_leftwards() {
    let mut harness = harness();
    let before = width(&harness, Pane::Details);

    drag_splitter(&mut harness, Pane::Details, -50.0);
    assert_eq!(width(&harness, Pane::Details), before + 50.0);
    assert_eq!(rect(&harness, Part::Details).width(), before + 50.0);
}

#[test]
fn pane_widths_clamp_and_do_not_drift() {
    let mut harness = harness();
    let (min, max) = (
        f32::from(chat_shell::state::PaneWidth::new(Pane::Sidebar, 0.0)),
        f32::from(chat_shell::state::PaneWidth::new(Pane::Sidebar, 1e9)),
    );
    assert!(min < width(&harness, Pane::Sidebar) && width(&harness, Pane::Sidebar) < max);

    drag_splitter(&mut harness, Pane::Sidebar, 2000.0);
    assert_eq!(width(&harness, Pane::Sidebar), max);

    let from = rect(&harness, Part::Splitter(Pane::Sidebar)).center();
    drag(&mut harness, from, from + vec2(-40.0, 0.0));
    assert_eq!(width(&harness, Pane::Sidebar), max - 40.0);

    drag_splitter(&mut harness, Pane::Sidebar, -2000.0);
    assert_eq!(width(&harness, Pane::Sidebar), min);
}

#[test]
fn the_splitter_shows_a_resize_cursor_and_lights_its_line_under_the_pointer() {
    let mut harness = harness();
    let line = "/Splitter(Sidebar)/<rule>";
    let splitter = rect(&harness, Part::Splitter(Pane::Sidebar));
    assert_ne!(
        fill_of(&harness, line),
        Some(color(&harness, Tok::Accent)),
        "precondition: the line starts unlit"
    );

    hover_over(&mut harness, splitter.center());
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::ResizeHorizontal
    );
    assert_eq!(fill_of(&harness, line), Some(color(&harness, Tok::Accent)));

    hover_node(&mut harness, &path(Part::Centre));
    assert_ne!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::ResizeHorizontal
    );
    assert_ne!(fill_of(&harness, line), Some(color(&harness, Tok::Accent)));
}

#[test]
fn hovering_a_nav_row_repaints_it() {
    let mut harness = harness();
    let row = format!("/Sidebar/{:?}", Part::Link(Link::Projects));
    let hover = color(&harness, Tok::Hover);
    assert_ne!(fill_of(&harness, &row), Some(hover), "precondition");

    hover_node(&mut harness, &row);
    harness.run_steps(10);
    assert_eq!(fill_of(&harness, &row), Some(hover));
}

#[test]
fn clicking_a_recent_selects_it() {
    let mut harness = harness();
    let (first, third) = (SessionKey(1), SessionKey(3));
    assert_eq!(harness.state().app.selected, first);

    click_node(&mut harness, &session_path(third));
    harness.run_steps(10);
    assert_eq!(harness.state().app.selected, third);

    let selected = color(&harness, Tok::Selected);
    assert_eq!(fill_of(&harness, &session_path(third)), Some(selected));
    assert_ne!(
        fill_of(&harness, &session_path(first)),
        Some(selected),
        "the old selection let go"
    );
}

#[test]
fn recents_light_under_the_pointer_but_the_selected_one_keeps_its_fill() {
    let mut harness = harness();
    let (selected, other) = (SessionKey(1), SessionKey(3));
    let (hover, selected_fill) = (color(&harness, Tok::Hover), color(&harness, Tok::Selected));

    hover_node(&mut harness, &session_path(other));
    harness.run_steps(10);
    assert_eq!(fill_of(&harness, &session_path(other)), Some(hover));

    hover_node(&mut harness, &session_path(selected));
    harness.run_steps(10);
    assert_eq!(
        fill_of(&harness, &session_path(selected)),
        Some(selected_fill)
    );
    assert_ne!(fill_of(&harness, &session_path(other)), Some(hover));
}
