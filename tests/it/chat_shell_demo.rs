//! The chat shell through the full path: pointer events, hit testing, drag
//! handling in the draw closures, and the geometry the layout promises.

use egui::{CursorIcon, Vec2, vec2};
use egui_kittest::Harness;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/chat_shell/main.rs"]
mod chat_shell;

use crate::support;

use chat_shell::sidebar::Link;
use chat_shell::state::{App, Message, Pane, Session, SessionKey};
use chat_shell::theme::{Theme, Tok};
use chat_shell::view::{Part, layout, prepare};
use support::{Rig, click_node, drag, fill_of, hover_node, hover_over, rect_of};

const WINDOW: Vec2 = vec2(1200.0, 760.0);

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    harness_in(egui::Theme::Dark, app)
}

/// A harness showing `theme`, which the tests pick so that they do not depend
/// on the machine's theme.
fn harness_in(theme: egui::Theme, app: App) -> Harness<'static, Rig<App>> {
    let harness =
        support::harness_prepared(WINDOW, Theme::new(theme).base_text(), app, layout, prepare);
    harness.ctx.set_theme(theme);
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(App::default())
}

fn path(part: Part) -> String {
    format!("/Frame/{part:?}")
}

fn session_path(key: SessionKey) -> String {
    format!("/Frame/Sidebar/Recents/{key:?}")
}

fn rect(harness: &Harness<'_, Rig<App>>, part: Part) -> egui::Rect {
    rect_of(harness, &path(part))
}

fn width(harness: &Harness<'_, Rig<App>>, pane: Pane) -> f32 {
    harness.state().app.width(pane).into()
}

fn color(harness: &Harness<'_, Rig<App>>, token: Tok) -> egui::Color32 {
    Theme::of(&harness.ctx)[token]
}

/// Drags the splitter of `pane` by `dx` points.
fn drag_splitter(harness: &mut Harness<'_, Rig<App>>, pane: Pane, dx: f32) {
    let from = rect(harness, Part::Splitter(pane)).center();
    drag(harness, from, from + vec2(dx, 0.0));
}

#[test]
fn the_three_panes_and_two_splitters_fill_the_window() {
    let harness = harness();
    let root = rect_of(&harness, "/Frame/<div>");
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
        rect(harness, Part::Centre).max.y - rect_of(harness, "/Frame/Centre/Composer").max.y
    };
    let mut harness = harness();
    let full = pinned(&harness);
    assert_eq!(full, 16.0, "the pane's bottom padding");
    assert!(
        rect_of(&harness, "/Frame/Centre/Conversation").max.y
            <= rect_of(&harness, "/Frame/Centre/Composer").min.y,
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
    let line = "/Frame/Splitter(Sidebar)/\"line\"";
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
    harness.run_steps(6);
    assert_eq!(
        fill_of(&harness, line),
        Some(color(&harness, Tok::Accent)),
        "lit once its transition has run"
    );

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
    let row = format!("/Frame/Sidebar/{:?}", Part::Link(Link::Projects));
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

// -------------------------------------------------------------------- composer

use egui::accesskit::Role as AccessRole;
use egui_kittest::kittest::{NodeT, Queryable};

fn user_messages(harness: &Harness<'_, Rig<App>>) -> Vec<String> {
    harness
        .state()
        .app
        .conversation
        .iter()
        .filter_map(|message| match message {
            chat_shell::state::Message::User(text) => Some(text.clone()),
            chat_shell::state::Message::Assistant(_) => None,
        })
        .collect()
}

/// Focuses the reply field by clicking it and types `text` into it.
fn type_reply(harness: &mut Harness<'_, Rig<App>>, text: &str) {
    click_node(harness, "/Frame/Centre/Composer/Field");
    harness.run_steps(2);
    harness.get_by_label("Message").type_text(text);
    harness.run_steps(3);
}

#[test]
fn typing_into_the_reply_field_updates_the_draft() {
    let mut harness = harness();
    assert!(harness.state().app.draft.is_empty(), "precondition");
    assert_eq!(
        harness.get_by_label("Message").accesskit_node().role(),
        AccessRole::MultilineTextInput
    );

    type_reply(&mut harness, "hello there");
    assert_eq!(harness.state().app.draft, "hello there");
}

#[test]
fn clicking_send_appends_the_trimmed_draft_and_clears_it() {
    let mut harness = harness();
    let before = user_messages(&harness);
    type_reply(&mut harness, "  a question \n");
    assert_eq!(harness.state().app.draft, "  a question \n");

    click_node(&mut harness, "/Frame/Centre/Composer/Send");
    harness.run_steps(3);
    let mut expected = before;
    expected.push("a question".into());
    assert_eq!(user_messages(&harness), expected);
    assert!(harness.state().app.draft.is_empty());
}

#[test]
fn ctrl_enter_in_the_field_sends() {
    let mut harness = harness();
    let before = user_messages(&harness).len();
    type_reply(&mut harness, "via keyboard");

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run_steps(3);
    assert_eq!(user_messages(&harness).len(), before + 1);
    assert_eq!(user_messages(&harness).last().unwrap(), "via keyboard");
    assert!(harness.state().app.draft.is_empty());
}

#[test]
fn send_is_inert_while_the_draft_is_empty() {
    let mut harness = harness();
    let before = user_messages(&harness);
    let dimmed = |harness: &Harness<'_, Rig<App>>| {
        fill_of(harness, "/Frame/Centre/Composer/Send").map(|fill| fill.a())
    };
    let empty = dimmed(&harness);

    click_node(&mut harness, "/Frame/Centre/Composer/Send");
    harness.run_steps(3);
    assert_eq!(user_messages(&harness), before, "nothing to send");

    harness.state_mut().app.draft = "   ".into();
    harness.run_steps(2);
    click_node(&mut harness, "/Frame/Centre/Composer/Send");
    harness.run_steps(3);
    assert_eq!(user_messages(&harness), before, "whitespace is empty");

    harness.state_mut().app.draft = "x".into();
    harness.run_steps(2);
    assert!(
        dimmed(&harness) > empty,
        "a draft makes the button opaque: {empty:?} -> {:?}",
        dimmed(&harness)
    );
}

#[test]
fn tab_walks_the_sidebar_in_order_and_skips_the_splitters() {
    let mut harness = harness();
    let mut order = Vec::new();
    support::past_bar(&mut harness);
    for _ in 0..40 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let Some(path) = support::focused_node(&harness) else {
            break;
        };
        if order.first() == Some(&path) {
            break;
        }
        order.push(path);
    }
    let sessions: Vec<_> = order
        .iter()
        .filter(|path| path.starts_with("/Frame/Sidebar/"))
        .collect();
    assert!(
        sessions.len() > 2,
        "precondition: the sidebar has several stops: {order:?}"
    );
    assert!(
        sessions.is_sorted_by_key(|path| rect_of(&harness, path).min.y as i32),
        "registration order is paint order, top to bottom: {sessions:?}"
    );
    assert_eq!(
        order.first().map(String::as_str),
        Some(sessions[0].as_str()),
        "the sidebar paints before the centre: {order:?}"
    );
    assert!(
        order.iter().all(|path| !path.contains("Splitter")),
        "{order:?}"
    );
}

// ------------------------------------------------- composer growth, selectors

use chat_shell::state::{MODELS, Mode};

fn composer_heights(harness: &mut Harness<'_, Rig<App>>, lines: usize) -> (f32, f32) {
    harness.state_mut().app.draft = vec!["line"; lines].join("\n");
    harness.run_steps(3);
    (
        rect_of(harness, "/Frame/Centre/Composer/Field").height(),
        rect_of(harness, "/Frame/Centre/Composer").height(),
    )
}

#[test]
fn the_composer_grows_with_its_text_up_to_a_cap_and_stays_at_the_bottom() {
    let mut harness = harness();
    let (field, composer): (Vec<f32>, Vec<f32>) = (1..=12)
        .map(|lines| composer_heights(&mut harness, lines))
        .unzip();
    assert_eq!(field[0], field[2], "three rows are the minimum: {field:?}");
    assert!(
        field[2..8].windows(2).all(|pair| pair[1] > pair[0]),
        "each row up to the cap grows it: {field:?}"
    );
    assert!(
        field[8..].iter().all(|&height| height == field[8]),
        "beyond eight rows it scrolls at the cap: {field:?}"
    );
    assert!(field[8] >= field[7], "{field:?}");
    assert_eq!(
        composer[11] - composer[0],
        field[11] - field[0],
        "the composer grows by what the field does"
    );

    let centre = rect(&harness, Part::Centre);
    let composer = rect_of(&harness, "/Frame/Centre/Composer");
    assert_eq!(centre.max.y - composer.max.y, 16.0, "still pinned");
    assert!(
        rect_of(&harness, "/Frame/Centre/Conversation").max.y <= composer.min.y,
        "the conversation gave way"
    );
}

#[test]
fn a_capped_field_still_edits_and_enter_makes_a_row() {
    let mut harness = harness();
    let (_, _) = composer_heights(&mut harness, 12);
    let capped = rect_of(&harness, "/Frame/Centre/Composer/Field").height();
    type_reply(&mut harness, "tail");
    assert!(harness.state().app.draft.contains("tail"));
    assert_eq!(
        rect_of(&harness, "/Frame/Centre/Composer/Field").height(),
        capped
    );

    harness.state_mut().app.draft.clear();
    harness.run_steps(3);
    let resting = rect_of(&harness, "/Frame/Centre/Composer/Field").height();
    click_node(&mut harness, "/Frame/Centre/Composer/Field");
    harness.run_steps(2);
    for _ in 0..4 {
        harness.key_press(egui::Key::Enter);
        harness.run_steps(2);
    }
    assert!(
        rect_of(&harness, "/Frame/Centre/Composer/Field").height() > resting,
        "four newlines make a fifth row"
    );
}

#[test]
fn the_model_select_changes_the_model_and_the_details_follow() {
    let mut harness = harness();
    let before = rect(&harness, Part::Centre);
    assert_eq!(harness.state().app.model, 0, "precondition");

    click_node(&mut harness, "/Frame/Centre/TopBar/Model");
    harness.run_steps(3);
    harness.get_by_label(MODELS[2]).click();
    harness.run_steps(12);
    assert_eq!(harness.state().app.model, 2);
    assert_eq!(rect(&harness, Part::Centre), before, "nothing moved");

    let painted = harness
        .output()
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::epaint::Shape::Text(text)
                if text.galley.text() == MODELS[2])
        })
        .count();
    assert_eq!(painted, 2, "the select's header and the details row");
}

#[test]
fn the_mode_control_has_one_pressed_cell_and_a_click_changes_it() {
    let mut harness = harness();
    assert_eq!(harness.state().app.mode, Mode::Standard);
    click_node(&mut harness, "/Frame/Centre/Composer/Mode/2");
    harness.run_steps(3);
    assert_eq!(harness.state().app.mode, Mode::Thorough);

    let selected = color(&harness, Tok::Line);
    let pressed: Vec<Mode> = Mode::ALL
        .iter()
        .enumerate()
        .filter(|(at, _)| {
            fill_of(&harness, &format!("/Frame/Centre/Composer/Mode/{at}")) == Some(selected)
        })
        .map(|(_, (mode, _))| *mode)
        .collect();
    assert_eq!(pressed, [Mode::Thorough]);
}

#[test]
fn tab_reaches_the_model_then_the_composer_in_order_and_skips_an_inert_send() {
    let mut harness = harness();
    let mut order = Vec::new();
    support::past_bar(&mut harness);
    for _ in 0..40 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let Some(stop) = support::focused_node(&harness) else {
            break;
        };
        if order.first() == Some(&stop) {
            break;
        }
        order.push(stop);
    }
    let centre: Vec<&str> = order
        .iter()
        .map(String::as_str)
        .filter(|stop| stop.starts_with("/Frame/Centre/"))
        .collect();
    assert_eq!(
        centre,
        [
            "/Frame/Centre/TopBar/Model",
            "/Frame/Centre/Conversation",
            "/Frame/Centre/Composer/Field",
            "/Frame/Centre/Composer/Mode/0",
            "/Frame/Centre/Composer/Mode/1",
            "/Frame/Centre/Composer/Mode/2",
        ],
        "the conversation is a stop, Send is inert while the draft is empty: {order:?}"
    );

    harness.state_mut().app.draft = "x".into();
    harness.run_steps(3);
    let mut with_send = Vec::new();
    support::past_bar(&mut harness);
    for _ in 0..40 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let Some(stop) = support::focused_node(&harness) else {
            break;
        };
        if with_send.first() == Some(&stop) {
            break;
        }
        with_send.push(stop);
    }
    assert!(
        with_send
            .iter()
            .any(|stop| stop == "/Frame/Centre/Composer/Send")
    );
}

mod composer_growth {
    use super::*;
    use eggshell_ui::Width;
    use eggshell_ui::widget::Stock as _;
    use egui::{Event, Key};

    const MIN_ROWS: usize = 3;
    const MAX_ROWS: usize = 8;
    const FIELD: &str = "/Frame/Centre/Composer/Field";

    fn scaled_harness(pixels_per_point: f32) -> Harness<'static, Rig<App>> {
        let mut harness = support::harness_scaled(
            WINDOW,
            pixels_per_point,
            Theme::new(egui::Theme::Dark).base_text(),
            App::default(),
            layout,
            prepare,
        );
        harness.ctx.set_theme(egui::Theme::Dark);
        harness.run();
        click_node(&mut harness, "/Frame/Centre/Composer/Field");
        harness.run();
        harness
    }

    /// The rect egui drew the text edit at: its whole content, not the part
    /// a scroll area shows.
    fn edit_rect(harness: &Harness<'_, Rig<App>>) -> egui::Rect {
        harness.get_by_label("Message").rect()
    }

    fn estimate(harness: &Harness<'_, Rig<App>>) -> Vec2 {
        let app = &harness.state().app;
        let field = eggshell_ui::prim::text_field::Field::multiline(&app.draft, MIN_ROWS)
            .max_rows(MAX_ROWS)
            .hint("Reply…")
            .bare();
        let width = rect_of(harness, FIELD).width();
        field.size(
            &harness.ctx,
            &Theme::of(&harness.ctx).base_text(),
            Some(Width::new(width)),
        )
    }

    /// A new line, one frame, and what the composer looks like in it.
    fn new_line(harness: &mut Harness<'_, Rig<App>>) {
        harness.event(Event::Text("x".into()));
        harness.key_press(Key::Enter);
        harness.step();
    }

    #[test]
    fn the_field_is_laid_out_at_the_height_egui_draws_it_in_the_frame_of_the_keystroke() {
        for pixels_per_point in [1.0, 1.5, 2.0] {
            let mut harness = scaled_harness(pixels_per_point);
            let snap = 1.0 / pixels_per_point;
            for rows in 1..=MAX_ROWS + 1 {
                new_line(&mut harness);
                let drawn_rows = rows + 1;
                let field = rect_of(&harness, FIELD);
                let edit = edit_rect(&harness);
                let estimate = estimate(&harness);
                let context = format!("ppp {pixels_per_point}, {rows} lines");
                assert!(
                    (field.height() - estimate.y).abs() <= snap,
                    "{context}: leaf {field:?} vs estimate {estimate:?}"
                );
                if drawn_rows <= MAX_ROWS {
                    assert!(
                        (edit.height() - field.height()).abs() <= snap,
                        "{context}: the field draws {edit:?} in a leaf of {field:?}"
                    );
                    assert!(
                        (edit.height() - estimate.y).abs() <= snap,
                        "{context}: estimate {estimate:?} vs drawn {edit:?}"
                    );
                } else {
                    assert!(
                        edit.height() > field.height(),
                        "{context}: precondition: the text is taller than the cap"
                    );
                }
                let scrolling = drawn_rows > MAX_ROWS;
                for _ in 0..3 {
                    harness.step();
                    assert_eq!(
                        rect_of(&harness, FIELD),
                        field,
                        "{context}: the leaf moved with no input"
                    );
                    if !scrolling {
                        assert_eq!(
                            edit_rect(&harness),
                            edit,
                            "{context}: the field moved with no input"
                        );
                    }
                }
            }
        }
    }
}

mod appearance {
    use super::*;

    const LIGHT: &str = "/Frame/Sidebar/Account/Appearance/1";
    const DARK: &str = "/Frame/Sidebar/Account/Appearance/2";
    const SYSTEM: &str = "/Frame/Sidebar/Account/Appearance/0";
    const NAV_ROW: &str = "/Frame/Sidebar/Link(Projects)";
    const MODEL: &str = "/Frame/Centre/TopBar/Model";

    fn settle(harness: &mut Harness<'_, Rig<App>>) {
        harness.run_steps(10);
    }

    /// The fill of the model select's trigger.
    fn select_fill(harness: &Harness<'_, Rig<App>>) -> Option<egui::Color32> {
        fill_of(harness, MODEL)
    }

    #[test]
    fn the_theme_the_app_shows_is_the_egui_theme() {
        let mut harness = harness();
        assert_eq!(harness.ctx.theme(), egui::Theme::Dark, "precondition");

        click_node(&mut harness, LIGHT);
        settle(&mut harness);
        assert_eq!(harness.ctx.theme(), egui::Theme::Light);

        click_node(&mut harness, DARK);
        settle(&mut harness);
        assert_eq!(harness.ctx.theme(), egui::Theme::Dark);

        click_node(&mut harness, SYSTEM);
        settle(&mut harness);
        assert_eq!(
            harness.ctx.options(|options| options.theme_preference),
            egui::ThemePreference::System
        );
    }

    #[test]
    fn switching_recolours_the_next_frame_and_back() {
        let mut harness = harness();
        let sidebar = "/Frame/Sidebar";
        let composer = "/Frame/Centre/Composer";
        let dark = Theme::new(egui::Theme::Dark);
        let light = Theme::new(egui::Theme::Light);
        assert_ne!(dark[Tok::Panel], light[Tok::Panel], "precondition");
        assert_ne!(dark[Tok::Surface], light[Tok::Surface], "precondition");
        settle(&mut harness);
        assert_eq!(fill_of(&harness, sidebar), Some(dark[Tok::Panel]));
        assert_eq!(fill_of(&harness, composer), Some(dark[Tok::Surface]));
        let dark_select = select_fill(&harness);
        assert!(
            dark_select.is_some(),
            "precondition: the select draws a fill"
        );

        click_node(&mut harness, LIGHT);
        harness.step();
        assert_eq!(
            fill_of(&harness, sidebar),
            Some(light[Tok::Panel]),
            "the frame after the click already has the new palette"
        );
        assert_eq!(fill_of(&harness, composer), Some(light[Tok::Surface]));
        settle(&mut harness);
        assert_ne!(select_fill(&harness), dark_select, "the select follows");
        assert_eq!(
            select_fill(&harness),
            Some(
                eggshell_ui_shadcn::Shadcn::light()
                    .style()
                    .select
                    .trigger
                    .paint
                    .states
                    .rest
                    .fill
            ),
            "the select is drawn with the preset's light style"
        );

        click_node(&mut harness, DARK);
        harness.step();
        assert_eq!(fill_of(&harness, sidebar), Some(dark[Tok::Panel]));
        assert_eq!(fill_of(&harness, composer), Some(dark[Tok::Surface]));
        settle(&mut harness);
        assert_eq!(select_fill(&harness), dark_select);
    }

    #[test]
    fn a_row_part_way_through_its_hover_transition_takes_the_new_palette() {
        let mut harness = harness();
        let dark = Theme::new(egui::Theme::Dark);
        let light = Theme::new(egui::Theme::Light);
        assert_ne!(dark[Tok::Hover], light[Tok::Hover], "precondition");

        harness.hover_at(rect_of(&harness, NAV_ROW).center());
        harness.run_steps(2);
        let midway = fill_of(&harness, NAV_ROW);
        assert!(
            midway.is_some_and(|fill| fill.a() > 0 && fill.a() < 255),
            "precondition: the row is part-way to its hover fill: {midway:?}"
        );

        harness.ctx.set_theme(egui::Theme::Light);
        harness.step();
        let after = fill_of(&harness, NAV_ROW);
        assert!(
            after.is_some_and(|fill| fill.a() > 0 && fill.r() > dark[Tok::Hover].r()),
            "the row keeps the old palette's fill: {after:?}"
        );
        settle(&mut harness);
        assert_eq!(fill_of(&harness, NAV_ROW), Some(light[Tok::Hover]));
    }
}

mod accents {
    use super::*;
    use eggshell_ui_shadcn::ThemeColor;
    use egui::epaint::Shape;

    fn text_colors(harness: &Harness<'_, Rig<App>>, text: &str) -> Vec<egui::Color32> {
        harness
            .output()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(shape) if shape.galley.text() == text => {
                    Some(shape.override_text_color.unwrap_or(shape.fallback_color))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_accent_colours_the_busy_marks_and_green_and_red_the_change_counts() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let mut harness = harness_in(theme, App::default());
            harness.run_steps(3);
            let k = Theme::of(&harness.ctx);
            assert_eq!(text_colors(&harness, "+24"), [k[Tok::Success]], "{theme:?}");
            assert_eq!(text_colors(&harness, "-8"), [k[Tok::Danger]]);
            assert_eq!(text_colors(&harness, "Running"), [k[Tok::Accent]]);
            assert_ne!(k[Tok::Success], k[Tok::Danger], "precondition");
            assert_eq!(
                eggshell_ui_shadcn::Shadcn::of(&harness.ctx).tokens.primary,
                eggshell_ui_shadcn::Shadcn::for_theme(theme)
                    .theme_color(ThemeColor::Blue)
                    .tokens
                    .primary,
                "the example's accent is blue"
            );
            let meter = rect_of(
                &harness,
                "/Frame/Details/\"Context\"/Fold(\"Context\")/Group/Row(\"Context window\")/Meter",
            );
            assert!(meter.is_positive(), "precondition: the meter is laid out");
            assert!(
                harness.output().shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    Shape::Rect(r) if r.fill == k[Tok::Accent]
                        && r.rect.left() >= meter.left() - 1.0
                        && r.rect.top() >= meter.top() - 1.0
                        && r.rect.bottom() <= meter.bottom() + 1.0
                )),
                "the meter's fill is the accent"
            );
        }
    }
}

mod grouped_form {
    use super::*;
    use egui::epaint::Shape;

    fn text_rect(harness: &Harness<'_, Rig<App>>, within: egui::Rect, text: &str) -> egui::Rect {
        let rects: Vec<egui::Rect> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(shape) if shape.galley.text() == text => {
                    Some(shape.galley.rect.translate(shape.pos.to_vec2()))
                }
                _ => None,
            })
            .filter(|rect| within.contains_rect(*rect))
            .collect();
        assert_eq!(rects.len(), 1, "{text} is painted once in the group");
        rects[0]
    }

    #[test]
    fn rows_in_a_group_are_divided_by_lines_and_their_values_sit_at_the_right_edge() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let mut harness = harness_in(theme, App::default());
            harness.run_steps(3);
            let line = Theme::of(&harness.ctx)[Tok::Line];
            let group = rect_of(
                &harness,
                "/Frame/Details/\"Environment\"/Fold(\"Environment\")/Group",
            );
            let rows = ["Branch", "Runtime", "Model"].map(|label| {
                rect_of(
                    &harness,
                    &format!("/Frame/Details/\"Environment\"/Fold(\"Environment\")/Group/Row(\"{label}\")"),
                )
            });
            assert!(
                rows.windows(2)
                    .all(|pair| pair[0].bottom() <= pair[1].top()),
                "precondition: the rows are stacked in order"
            );

            let rules: Vec<egui::Rect> = harness
                .output()
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Rect(r)
                        if r.fill == line
                            && r.rect.height() <= 1.0
                            && r.rect.width() > 100.0
                            && group.contains_rect(r.rect.expand(1.0)) =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .collect();
            for pair in rows.windows(2) {
                assert_eq!(
                    rules
                        .iter()
                        .filter(|rule| rule.center().y >= pair[0].bottom() - 1.0
                            && rule.center().y <= pair[1].top() + 1.0)
                        .count(),
                    1,
                    "{theme:?}: one divide line between {:?} and {:?}: {rules:?}",
                    pair[0],
                    pair[1]
                );
            }
            assert_eq!(
                rules.len(),
                2,
                "{theme:?}: none above the first or below the last"
            );

            let padding_edge = group.right() - 1.0;
            for (row, (label, value)) in rows.iter().zip([
                ("Branch", "main"),
                ("Runtime", "local"),
                ("Model", MODELS[0]),
            ]) {
                assert_eq!(
                    row.right(),
                    padding_edge,
                    "{label}: the row fills the group"
                );
                let value = text_rect(&harness, group, value);
                assert!(
                    (value.right() - (padding_edge - 12.0)).abs() <= 0.5,
                    "{label}: {value:?}"
                );
                assert!(text_rect(&harness, group, label).left() - row.left() <= 12.5);
            }
        }
    }
}

mod scrolling {
    use super::*;
    use egui::{Event, Modifiers, MouseWheelUnit};

    fn long_app() -> App {
        let mut app = App::default();
        for n in 0..24 {
            app.conversation
                .push(Message::User(format!("Another question, number {n}.")));
            app.sessions.push(Session {
                key: SessionKey(100 + n),
                title: "An older session",
                status: None,
            });
        }
        app
    }

    /// Where the part is in the tree.
    fn at(part: Part) -> &'static str {
        match part {
            Part::TopBar => "/Frame/Centre/TopBar",
            Part::Conversation => "/Frame/Centre/Conversation",
            Part::Composer => "/Frame/Centre/Composer",
            Part::Recents => "/Frame/Sidebar/Recents",
            Part::Account => "/Frame/Sidebar/Account",
            other => unreachable!("{other:?}"),
        }
    }

    fn rect(harness: &Harness<'_, Rig<App>>, part: Part) -> egui::Rect {
        rect_of(harness, at(part))
    }

    fn offset(harness: &Harness<'_, Rig<App>>, part: Part) -> Vec2 {
        let id = support::id_of(harness, at(part));
        eggshell_ui::scroll::viewport(&harness.ctx, id)
            .expect("laid out")
            .offset()
    }

    fn wheel_over(harness: &mut Harness<'_, Rig<App>>, part: Part) {
        let at = rect(harness, part).center();
        harness.hover_at(at);
        harness.step();
        harness.event(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -300.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        });
        harness.run_steps(30);
    }

    #[test]
    fn the_conversation_and_the_recents_scroll_independently_and_the_rest_stays_put() {
        let mut harness = harness_with(long_app());
        harness.run_steps(3);
        let fixed = [
            Part::TopBar,
            Part::Composer,
            Part::Account,
            Part::Conversation,
        ];
        let at_rest: Vec<egui::Rect> = fixed.iter().map(|&part| rect(&harness, part)).collect();
        let content = |harness: &Harness<'_, Rig<App>>, part: Part| {
            rect_of(harness, &format!("{}/<scrollcontent>", at(part))).height()
        };
        assert!(
            content(&harness, Part::Conversation) > rect(&harness, Part::Conversation).height(),
            "precondition: the conversation overflows"
        );
        assert!(
            content(&harness, Part::Recents) > rect(&harness, Part::Recents).height(),
            "precondition: the recents overflow"
        );

        wheel_over(&mut harness, Part::Conversation);
        assert!(
            offset(&harness, Part::Conversation).y > 100.0,
            "the conversation scrolled"
        );
        assert_eq!(
            offset(&harness, Part::Recents),
            Vec2::ZERO,
            "the recents did not"
        );

        wheel_over(&mut harness, Part::Recents);
        assert!(
            offset(&harness, Part::Recents).y > 100.0,
            "the recents scrolled"
        );

        let now: Vec<egui::Rect> = fixed.iter().map(|&part| rect(&harness, part)).collect();
        assert_eq!(
            at_rest, now,
            "the headers, the composer and the account block stay put"
        );
    }

    #[test]
    fn a_session_scrolled_out_of_the_recents_takes_no_click() {
        let mut harness = harness_with(long_app());
        harness.run_steps(3);
        let last = SessionKey(100 + 23);
        assert!(
            rect_of(&harness, &session_path(last)).min.y > rect(&harness, Part::Recents).max.y,
            "precondition: the last session is below the recents' viewport"
        );
        let before = harness.state().app.selected;
        let below = rect_of(&harness, &session_path(last)).center();
        support::press_and_release(&mut harness, below);
        harness.run_steps(2);
        assert_eq!(harness.state().app.selected, before, "nothing was selected");
    }
}

mod collapsing {
    use chat_shell::state::Panes;

    use super::*;

    fn shown(harness: &Harness<'_, Rig<App>>, part: Part) -> bool {
        let wanted = path(part);
        harness
            .state()
            .nodes
            .iter()
            .any(|(candidate, _)| *candidate == wanted)
    }

    fn panes_shown(harness: &Harness<'_, Rig<App>>) -> [bool; 4] {
        [
            Part::Sidebar,
            Part::Splitter(Pane::Sidebar),
            Part::Details,
            Part::Splitter(Pane::Details),
        ]
        .map(|part| shown(harness, part))
    }

    fn resized(size: Vec2) -> Harness<'static, Rig<App>> {
        let mut harness = harness();
        harness.set_size(size);
        harness.step();
        harness
    }

    #[test]
    fn the_thresholds_are_where_the_panes_are_given_up() {
        let at = |width: f32| Panes::fitting(vec2(width, 600.0));
        assert_eq!(at(1000.0), Panes::All);
        assert_eq!(at(999.0), Panes::WithoutDetails);
        assert_eq!(at(720.0), Panes::WithoutDetails);
        assert_eq!(at(719.0), Panes::CentreOnly);
    }

    #[test]
    fn a_wide_window_shows_all_three_panes() {
        let harness = resized(vec2(1200.0, 760.0));
        assert_eq!(panes_shown(&harness), [true; 4]);
    }

    #[test]
    fn a_narrower_window_gives_up_the_details_panel_first() {
        let wide = resized(vec2(1200.0, 760.0));
        let harness = resized(vec2(900.0, 760.0));
        assert_eq!(panes_shown(&harness), [true, true, false, false]);
        let (centre, window) = (rect(&harness, Part::Centre), rect_of(&harness, "/Frame"));
        assert_eq!(
            centre.max.x, window.max.x,
            "the conversation takes the room the panel leaves"
        );
        assert!(centre.width() > rect(&wide, Part::Centre).width() - 400.0);
    }

    #[test]
    fn a_narrow_window_leaves_only_the_conversation() {
        let harness = resized(vec2(600.0, 760.0));
        assert_eq!(panes_shown(&harness), [false; 4]);
        assert_eq!(rect(&harness, Part::Centre), rect_of(&harness, "/Frame"));
    }

    #[test]
    fn the_first_frame_after_a_resize_already_shows_the_panes_that_fit() {
        let mut harness = harness();
        assert_eq!(panes_shown(&harness), [true; 4], "precondition: all shown");
        for (size, expected) in [
            (vec2(900.0, 760.0), [true, true, false, false]),
            (vec2(600.0, 760.0), [false; 4]),
            (vec2(1200.0, 760.0), [true; 4]),
        ] {
            harness.set_size(size);
            harness.step();
            assert_eq!(panes_shown(&harness), expected, "at {size:?}");
            // A conversation whose width changed measures its messages at the
            // new width, which can take frames of its own; the panes do not.
            // At its end, each frame measures the messages it shows and can
            // bring a few more into view, so it settles in a handful.
            let mut more = 0;
            while harness.ctx.requested_repaint_last_pass() && more < 6 {
                harness.step();
                more += 1;
                assert_eq!(panes_shown(&harness), expected, "still at {size:?}");
            }
            assert!(
                !harness.ctx.requested_repaint_last_pass(),
                "the frames stop at {size:?}"
            );
        }
    }
}

mod long_conversation {
    use super::*;
    use chat_shell::state::HISTORY;

    /// The messages built in the last frame, and the last message's rect if
    /// it is built.
    fn messages(harness: &Harness<'_, Rig<App>>) -> (usize, Option<egui::Rect>) {
        let last = format!(
            "/{:?}",
            harness
                .state()
                .app
                .conversation
                .ids()
                .last()
                .expect("a conversation")
        );
        let rows: Vec<&(String, egui::Rect)> = harness
            .state()
            .nodes
            .iter()
            .filter(|(path, _)| {
                path.rsplit('/')
                    .next()
                    .is_some_and(|segment| segment.starts_with("MessageId("))
            })
            .collect();
        let last = rows
            .iter()
            .find(|(path, _)| path.ends_with(&last))
            .map(|(_, rect)| *rect);
        (rows.len(), last)
    }

    fn viewport(harness: &Harness<'_, Rig<App>>) -> egui::Rect {
        let id = support::id_of(harness, "/Frame/Centre/Conversation");
        eggshell_ui::scroll::viewport(&harness.ctx, id)
            .expect("the conversation was laid out")
            .rect
    }

    #[test]
    fn it_opens_on_the_latest_message_and_builds_only_a_screenful() {
        let mut harness = harness();
        harness.run_steps(4);
        assert!(
            harness.state().app.conversation.ids().len() > HISTORY,
            "precondition: a long conversation"
        );
        let (built, last) = messages(&harness);
        let last = last.expect("the latest message is built");
        let view = viewport(&harness);
        // Within a pixel: egui rounds the viewport it scrolls to pixels.
        assert!(
            view.expand(1.0).contains_rect(last),
            "{last:?} is in {view:?}"
        );
        assert!(
            (last.bottom() - view.bottom()).abs() < 1.0,
            "it ends the viewport: {last:?} in {view:?}"
        );
        assert!(built < 60, "{built} messages built");
    }

    #[test]
    fn a_message_sent_at_the_end_shows_at_the_bottom() {
        let mut harness = harness();
        harness.run_steps(4);
        harness.state_mut().app.draft = "One more question.".into();
        harness.state_mut().app.send();
        harness.step();
        let mut frames = 0;
        while harness.ctx.has_requested_repaint() && frames < 6 {
            harness.step();
            frames += 1;
        }
        let (_, last) = messages(&harness);
        let last = last.expect("the new message is built");
        let view = viewport(&harness);
        // Within a pixel: egui rounds the viewport it scrolls to pixels.
        assert!(
            view.expand(1.0).contains_rect(last),
            "{last:?} is in {view:?}"
        );
        assert!(
            (last.bottom() - view.bottom()).abs() < 1.0,
            "it ends the viewport: {last:?} in {view:?}"
        );
    }
}

mod folding {
    use super::*;

    const HEAD: &str = "/Frame/Details/\"Changes\"/Head(\"Changes\")";
    const FOLD: &str = "/Frame/Details/\"Changes\"/Fold(\"Changes\")";
    const NEXT: &str = "/Frame/Details/\"Background tasks\"";

    #[test]
    fn clicking_a_details_header_folds_its_section_and_the_next_one_follows() {
        let mut harness = harness();
        harness.run_steps(6);
        let open = rect_of(&harness, FOLD).height();
        let next = rect_of(&harness, NEXT).min.y;
        assert!(open > 50.0, "precondition: the section has rows: {open}");
        click_node(&mut harness, HEAD);
        harness.run_steps(30);
        assert_eq!(rect_of(&harness, FOLD).height(), 0.0);
        assert_eq!(rect_of(&harness, NEXT).min.y, next - open);
        click_node(&mut harness, HEAD);
        harness.run_steps(30);
        assert_eq!(rect_of(&harness, FOLD).height(), open);
        assert_eq!(rect_of(&harness, NEXT).min.y, next);
    }
}

mod splitter_glow {
    use super::*;

    const LINE: &str = "/Frame/Splitter(Sidebar)/\"line\"";

    #[test]
    fn hovering_a_splitter_fades_its_line_to_the_accent() {
        let mut harness = harness();
        support::full_motion(&harness.ctx);
        harness.run_steps(4);
        let (line, accent) = (color(&harness, Tok::Line), color(&harness, Tok::Accent));
        assert_eq!(fill_of(&harness, LINE), Some(line), "precondition: at rest");
        hover_node(&mut harness, &path(Part::Splitter(Pane::Sidebar)));
        let mid = fill_of(&harness, LINE).expect("the line is painted");
        let between = |channel: fn(egui::Color32) -> u8| {
            let (from, to, now) = (channel(line), channel(accent), channel(mid));
            from.min(to) < now && now < from.max(to)
        };
        assert!(
            between(|c| c.r()) || between(|c| c.g()) || between(|c| c.b()),
            "mid-transition {mid:?} is between {line:?} and {accent:?}"
        );
        harness.run_steps(6);
        assert_eq!(fill_of(&harness, LINE), Some(accent));
    }
}
