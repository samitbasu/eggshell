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
use chat_shell::state::{App, Appearance, Pane, SessionKey};
use chat_shell::theme::{Theme, Tok};
use chat_shell::view::{Part, layout, prepare};
use support::{Rig, click_node, drag, fill_of, hover_node, hover_over, rect_of};

const WINDOW: Vec2 = vec2(1200.0, 760.0);

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    support::harness_prepared(
        WINDOW,
        Theme::new(egui::Theme::Dark).base_text(),
        app,
        layout,
        prepare,
    )
}

/// The app showing `appearance`, which the tests pick so that they do not
/// depend on the machine's theme.
#[allow(
    clippy::field_reassign_with_default,
    reason = "App has private fields, so a struct update is not available here"
)]
fn app_showing(appearance: Appearance) -> App {
    let mut app = App::default();
    app.appearance = appearance;
    app
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(app_showing(Appearance::Dark))
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
    click_node(harness, "/Centre/Composer/Field");
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

    click_node(&mut harness, "/Centre/Composer/Send");
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
        fill_of(harness, "/Centre/Composer/Send").map(|fill| fill.a())
    };
    let empty = dimmed(&harness);

    click_node(&mut harness, "/Centre/Composer/Send");
    harness.run_steps(3);
    assert_eq!(user_messages(&harness), before, "nothing to send");

    harness.state_mut().app.draft = "   ".into();
    harness.run_steps(2);
    click_node(&mut harness, "/Centre/Composer/Send");
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
    for _ in 0..40 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let path = support::focused_path(&harness).expect("Tab always lands on a stop");
        if order.first() == Some(&path) {
            break;
        }
        order.push(path);
    }
    let sessions: Vec<_> = order
        .iter()
        .filter(|path| path.starts_with("/Sidebar/"))
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
        rect_of(harness, "/Centre/Composer/Field").height(),
        rect_of(harness, "/Centre/Composer").height(),
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
    let composer = rect_of(&harness, "/Centre/Composer");
    assert_eq!(centre.max.y - composer.max.y, 16.0, "still pinned");
    assert!(
        rect_of(&harness, "/Centre/Conversation").max.y <= composer.min.y,
        "the conversation gave way"
    );
}

#[test]
fn a_capped_field_still_edits_and_enter_makes_a_row() {
    let mut harness = harness();
    let (_, _) = composer_heights(&mut harness, 12);
    let capped = rect_of(&harness, "/Centre/Composer/Field").height();
    type_reply(&mut harness, "tail");
    assert!(harness.state().app.draft.contains("tail"));
    assert_eq!(rect_of(&harness, "/Centre/Composer/Field").height(), capped);

    harness.state_mut().app.draft.clear();
    harness.run_steps(3);
    let resting = rect_of(&harness, "/Centre/Composer/Field").height();
    click_node(&mut harness, "/Centre/Composer/Field");
    harness.run_steps(2);
    for _ in 0..4 {
        harness.key_press(egui::Key::Enter);
        harness.run_steps(2);
    }
    assert!(
        rect_of(&harness, "/Centre/Composer/Field").height() > resting,
        "four newlines make a fifth row"
    );
}

#[test]
fn the_model_select_changes_the_model_and_the_details_follow() {
    let mut harness = harness();
    let before = rect(&harness, Part::Centre);
    assert_eq!(harness.state().app.model, 0, "precondition");

    click_node(&mut harness, "/Centre/TopBar/Model");
    harness.run_steps(3);
    harness.get_by_label(MODELS[2]).click();
    harness.run_steps(3);
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
    click_node(&mut harness, "/Centre/Composer/Mode/Thorough");
    harness.run_steps(3);
    assert_eq!(harness.state().app.mode, Mode::Thorough);

    let selected = color(&harness, Tok::Selected);
    let pressed: Vec<Mode> = Mode::ALL
        .iter()
        .map(|(mode, _)| *mode)
        .filter(|mode| {
            fill_of(&harness, &format!("/Centre/Composer/Mode/{mode:?}")) == Some(selected)
        })
        .collect();
    assert_eq!(pressed, [Mode::Thorough]);
}

#[test]
fn tab_reaches_the_model_then_the_composer_in_order_and_skips_an_inert_send() {
    let mut harness = harness();
    let mut order = Vec::new();
    for _ in 0..40 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let stop = support::focused_node(&harness).expect("a stop");
        if order.first() == Some(&stop) {
            break;
        }
        order.push(stop);
    }
    let centre: Vec<&str> = order
        .iter()
        .map(String::as_str)
        .filter(|stop| stop.starts_with("/Centre/"))
        .collect();
    assert_eq!(
        centre,
        [
            "/Centre/TopBar/Model",
            "/Centre/Composer/Field",
            "/Centre/Composer/Mode/Quick",
            "/Centre/Composer/Mode/Standard",
            "/Centre/Composer/Mode/Thorough",
        ],
        "Send is inert while the draft is empty: {order:?}"
    );

    harness.state_mut().app.draft = "x".into();
    harness.run_steps(3);
    let mut with_send = Vec::new();
    for _ in 0..40 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let stop = support::focused_node(&harness).expect("a stop");
        if with_send.first() == Some(&stop) {
            break;
        }
        with_send.push(stop);
    }
    assert!(with_send.iter().any(|stop| stop == "/Centre/Composer/Send"));
}

mod composer_growth {
    use super::*;
    use bullseye::Width;
    use bullseye::widget::Stock as _;
    use egui::{Event, Key};

    const MIN_ROWS: usize = 3;
    const MAX_ROWS: usize = 8;
    const FIELD: &str = "/Centre/Composer/Field";

    fn scaled_harness(pixels_per_point: f32) -> Harness<'static, Rig<App>> {
        let app = app_showing(Appearance::Dark);
        let mut harness = support::harness_scaled(
            WINDOW,
            pixels_per_point,
            Theme::new(egui::Theme::Dark).base_text(),
            app,
            layout,
            prepare,
        );
        harness.run();
        click_node(&mut harness, "/Centre/Composer/Field");
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
        let field = bullseye::prim::text_field::Field::multiline(&app.draft, MIN_ROWS)
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

    const LIGHT: &str = "/Sidebar/Account/Appearance/Light";
    const DARK: &str = "/Sidebar/Account/Appearance/Dark";
    const SYSTEM: &str = "/Sidebar/Account/Appearance/System";
    const NAV_ROW: &str = "/Sidebar/Link(Projects)";
    const MODEL: &str = "/Centre/TopBar/Model";

    fn settle(harness: &mut Harness<'_, Rig<App>>) {
        harness.run_steps(10);
    }

    /// The fill of the model select's header, which egui draws from its
    /// widget visuals.
    fn select_fill(harness: &Harness<'_, Rig<App>>) -> Option<egui::Color32> {
        fill_of(harness, MODEL)
    }

    #[test]
    fn the_theme_the_app_shows_is_the_egui_theme() {
        let mut harness = harness();
        assert_eq!(harness.ctx.theme(), egui::Theme::Dark, "precondition");

        click_node(&mut harness, LIGHT);
        settle(&mut harness);
        assert_eq!(harness.state().app.appearance, Appearance::Light);
        assert_eq!(harness.ctx.theme(), egui::Theme::Light);

        click_node(&mut harness, DARK);
        settle(&mut harness);
        assert_eq!(harness.ctx.theme(), egui::Theme::Dark);

        click_node(&mut harness, SYSTEM);
        settle(&mut harness);
        assert_eq!(harness.state().app.appearance, Appearance::System);
        assert_eq!(
            harness.ctx.options(|options| options.theme_preference),
            egui::ThemePreference::System
        );
    }

    #[test]
    fn switching_recolours_the_next_frame_and_back() {
        let mut harness = harness();
        let sidebar = "/Sidebar";
        let composer = "/Centre/Composer";
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
        assert_ne!(select_fill(&harness), dark_select, "egui's widgets follow");
        let light_widgets = harness
            .ctx
            .global_style()
            .visuals
            .widgets
            .inactive
            .weak_bg_fill;
        assert_eq!(select_fill(&harness), Some(light_widgets));
        assert_eq!(
            light_widgets,
            bullseye::style::Tokens::light().secondary,
            "the select is drawn with the preset's light widgets"
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

        harness.state_mut().app.appearance = Appearance::Light;
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
