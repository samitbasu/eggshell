//! The chat shell through the full path: pointer events, hit testing, drag
//! handling in the draw closures, and the geometry the layout promises.

use eggshell_ui::testing::{Driver, PixelsPerPoint, Target};
use egui::{CursorIcon, KeyboardShortcut, Vec2, vec2};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/chat_shell/main.rs"]
mod chat_shell;

use crate::support;

use chat_shell::sidebar::Link;
use chat_shell::state::{App, Message, Pane, Session, SessionKey};
use chat_shell::theme::{Theme, Tok};
use chat_shell::view::{Part, layout, prepare};
use support::fill_of;

const WINDOW: Vec2 = vec2(1200.0, 760.0);

fn harness_with(app: App) -> Driver<App> {
    harness_in(egui::Theme::Dark, app)
}

/// A harness showing `theme`, which the tests pick so that they do not depend
/// on the machine's theme.
fn harness_in(theme: egui::Theme, app: App) -> Driver<App> {
    let d = support::prepared_driver(WINDOW, Theme::new(theme).base_text(), app, layout, prepare)
        .build();
    d.ctx().set_theme(theme);
    d
}

fn driver() -> Driver<App> {
    harness_with(App::default())
}

fn path(part: Part) -> String {
    format!("/Frame/{part:?}")
}

fn session_path(key: SessionKey) -> String {
    format!("/Frame/Sidebar/Recents/{key:?}")
}

fn rect(d: &Driver<App>, part: Part) -> egui::Rect {
    d.rect(Target::suffix(&path(part)))
}

fn width(d: &Driver<App>, pane: Pane) -> f32 {
    d.app().width(pane).into()
}

fn color(d: &Driver<App>, token: Tok) -> egui::Color32 {
    Theme::of(d.ctx())[token]
}

/// Drags the splitter of `pane` by `dx` points.
fn drag_splitter(d: &mut Driver<App>, pane: Pane, dx: f32) {
    let from = rect(d, Part::Splitter(pane)).center();
    d.drag(from, from + vec2(dx, 0.0));
}

#[test]
fn the_three_panes_and_two_splitters_fill_the_window() {
    let d = driver();
    let root = d.rect(Target::suffix("/Frame/<div>"));
    let columns = [
        Part::Sidebar,
        Part::Splitter(Pane::Sidebar),
        Part::Centre,
        Part::Splitter(Pane::Details),
        Part::Details,
    ]
    .map(|part| rect(&d, part));

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
    assert_eq!(columns[0].width(), width(&d, Pane::Sidebar));
    assert_eq!(columns[4].width(), width(&d, Pane::Details));
}

#[test]
fn the_composer_is_pinned_to_the_bottom_of_the_centre_pane() {
    let pinned = |d: &Driver<App>| {
        rect(d, Part::Centre).max.y - d.rect(Target::suffix("/Frame/Centre/Composer")).max.y
    };
    let mut d = driver();
    let full = pinned(&d);
    assert_eq!(full, 16.0, "the pane's bottom padding");
    assert!(
        d.rect(Target::suffix("/Frame/Centre/Conversation")).max.y
            <= d.rect(Target::suffix("/Frame/Centre/Composer")).min.y,
        "precondition: the conversation sits above the composer"
    );

    d.app_mut().conversation.clear();
    d.frames(2);
    assert_eq!(pinned(&d), full, "an empty conversation does not lift it");
}

#[test]
fn dragging_a_splitter_resizes_its_pane() {
    let mut d = driver();
    let before = width(&d, Pane::Sidebar);

    drag_splitter(&mut d, Pane::Sidebar, 40.0);
    assert_eq!(width(&d, Pane::Sidebar), before + 40.0);
    assert_eq!(rect(&d, Part::Sidebar).width(), before + 40.0);
}

#[test]
fn the_details_splitter_grows_its_pane_leftwards() {
    let mut d = driver();
    let before = width(&d, Pane::Details);

    drag_splitter(&mut d, Pane::Details, -50.0);
    assert_eq!(width(&d, Pane::Details), before + 50.0);
    assert_eq!(rect(&d, Part::Details).width(), before + 50.0);
}

#[test]
fn pane_widths_clamp_and_do_not_drift() {
    let mut d = driver();
    let limits = Pane::Sidebar.limits();
    let (min, max) = (*limits.start(), *limits.end());
    assert!(min < width(&d, Pane::Sidebar) && width(&d, Pane::Sidebar) < max);

    drag_splitter_off_screen(&mut d, 2000.0);
    assert_eq!(width(&d, Pane::Sidebar), max);

    let from = rect(&d, Part::Splitter(Pane::Sidebar)).center();
    d.drag(from, from + vec2(-40.0, 0.0));
    assert_eq!(width(&d, Pane::Sidebar), max - 40.0);

    drag_splitter_off_screen(&mut d, -2000.0);
    assert_eq!(width(&d, Pane::Sidebar), min);
}

/// Drags the sidebar's splitter by `dx` points, past the window's edge,
/// where the driver's pointer actions do not go: the events of `d.drag`,
/// sent through kittest.
fn drag_splitter_off_screen(d: &mut Driver<App>, dx: f32) {
    let from = rect(d, Part::Splitter(Pane::Sidebar)).center();
    let to = from + vec2(dx, 0.0);
    d.hover(from);
    d.harness_mut().drag_at(from);
    d.frames(1);
    d.harness_mut().hover_at(from.lerp(to, 0.5));
    d.frames(1);
    d.harness_mut().hover_at(to);
    d.frames(1);
    d.harness_mut().drop_at(to);
    d.frames(3);
}

#[test]
fn the_splitter_shows_a_resize_cursor_and_lights_its_line_under_the_pointer() {
    let mut d = driver();
    let splitter = rect(&d, Part::Splitter(Pane::Sidebar));
    assert_ne!(
        splitter_glow::line_color(&d),
        Some(color(&d, Tok::Accent)),
        "precondition: the line starts unlit"
    );

    d.hover(splitter.center());
    assert_eq!(
        d.harness().output().platform_output.cursor_icon,
        CursorIcon::ResizeColumn
    );
    d.frames(6);
    assert_eq!(
        splitter_glow::line_color(&d),
        Some(color(&d, Tok::Accent)),
        "lit once its transition has run"
    );

    d.hover(Target::suffix(&path(Part::Centre)));
    assert_ne!(
        d.harness().output().platform_output.cursor_icon,
        CursorIcon::ResizeColumn
    );
    assert_ne!(splitter_glow::line_color(&d), Some(color(&d, Tok::Accent)));
}

#[test]
fn hovering_a_nav_row_repaints_it() {
    let mut d = driver();
    let row = format!("/Frame/Sidebar/{:?}", Part::Link(Link::Projects));
    let hover = color(&d, Tok::Hover);
    assert_ne!(fill_of(&d, &row), Some(hover), "precondition");

    d.hover(Target::suffix(&row));
    d.frames(10);
    assert_eq!(fill_of(&d, &row), Some(hover));
}

#[test]
fn clicking_a_recent_selects_it() {
    let mut d = driver();
    let (first, third) = (SessionKey(1), SessionKey(3));
    assert_eq!(d.app().selected, first);

    d.click(Target::suffix(&session_path(third)));
    d.frames(10);
    assert_eq!(d.app().selected, third);

    let selected = color(&d, Tok::Selected);
    assert_eq!(fill_of(&d, &session_path(third)), Some(selected));
    assert_ne!(
        fill_of(&d, &session_path(first)),
        Some(selected),
        "the old selection let go"
    );
}

#[test]
fn recents_light_under_the_pointer_but_the_selected_one_keeps_its_fill() {
    let mut d = driver();
    let (selected, other) = (SessionKey(1), SessionKey(3));
    let (hover, selected_fill) = (color(&d, Tok::Hover), color(&d, Tok::Selected));

    d.hover(Target::suffix(&session_path(other)));
    d.frames(10);
    assert_eq!(fill_of(&d, &session_path(other)), Some(hover));

    d.hover(Target::suffix(&session_path(selected)));
    d.frames(10);
    assert_eq!(fill_of(&d, &session_path(selected)), Some(selected_fill));
    assert_ne!(fill_of(&d, &session_path(other)), Some(hover));
}

// -------------------------------------------------------------------- composer

use egui::accesskit::Role as AccessRole;
use egui_kittest::kittest::{NodeT, Queryable};

fn user_messages(d: &Driver<App>) -> Vec<String> {
    d.app()
        .conversation
        .iter()
        .filter_map(|message| match message {
            chat_shell::state::Message::User(text) => Some(text.clone()),
            chat_shell::state::Message::Assistant(_) => None,
        })
        .collect()
}

/// Focuses the reply field by clicking it and types `text` into it.
fn type_reply(d: &mut Driver<App>, text: &str) {
    d.click(Target::suffix("/Frame/Centre/Composer/Field"));
    d.frames(2);
    d.harness().get_by_label("Message").type_text(text);
    d.frames(3);
}

#[test]
fn typing_into_the_reply_field_updates_the_draft() {
    let mut d = driver();
    assert!(d.app().draft.is_empty(), "precondition");
    assert_eq!(
        d.harness().get_by_label("Message").accesskit_node().role(),
        AccessRole::MultilineTextInput
    );

    type_reply(&mut d, "hello there");
    assert_eq!(d.app().draft, "hello there");
}

#[test]
fn clicking_send_appends_the_trimmed_draft_and_clears_it() {
    let mut d = driver();
    let before = user_messages(&d);
    type_reply(&mut d, "  a question \n");
    assert_eq!(d.app().draft, "  a question \n");

    d.click(Target::suffix("/Frame/Centre/Composer/Send"));
    d.frames(3);
    let mut expected = before;
    expected.push("a question".into());
    assert_eq!(user_messages(&d), expected);
    assert!(d.app().draft.is_empty());
}

#[test]
fn ctrl_enter_in_the_field_sends() {
    let mut d = driver();
    let before = user_messages(&d).len();
    type_reply(&mut d, "via keyboard");

    d.press(KeyboardShortcut::new(
        egui::Modifiers::COMMAND,
        egui::Key::Enter,
    ))
    .frames(2);
    assert_eq!(user_messages(&d).len(), before + 1);
    assert_eq!(user_messages(&d).last().unwrap(), "via keyboard");
    assert!(d.app().draft.is_empty());
}

#[test]
fn send_is_inert_while_the_draft_is_empty() {
    let mut d = driver();
    let before = user_messages(&d);
    let dimmed = |d: &Driver<App>| fill_of(d, "/Frame/Centre/Composer/Send").map(|fill| fill.a());
    let empty = dimmed(&d);

    d.click(Target::suffix("/Frame/Centre/Composer/Send"));
    d.frames(3);
    assert_eq!(user_messages(&d), before, "nothing to send");

    d.app_mut().draft = "   ".into();
    d.frames(2);
    d.click(Target::suffix("/Frame/Centre/Composer/Send"));
    d.frames(3);
    assert_eq!(user_messages(&d), before, "whitespace is empty");

    d.app_mut().draft = "x".into();
    d.frames(2);
    assert!(
        dimmed(&d) > empty,
        "a draft makes the button opaque: {empty:?} -> {:?}",
        dimmed(&d)
    );
}

#[test]
fn tab_walks_the_sidebar_in_order_then_its_splitter() {
    let mut d = driver();
    let mut order = Vec::new();
    support::drive_past_bar(&mut d);
    for _ in 0..40 {
        d.press(egui::Key::Tab).frames(1);
        let Some(path) = support::focused_node(&d) else {
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
        sessions.is_sorted_by_key(|path| d.rect(Target::suffix(path)).min.y as i32),
        "registration order is paint order, top to bottom: {sessions:?}"
    );
    assert_eq!(
        order.first().map(String::as_str),
        Some(sessions[0].as_str()),
        "the sidebar paints before the centre: {order:?}"
    );
    let after_sidebar = order
        .iter()
        .position(|path| path == sessions[sessions.len() - 1])
        .map(|at| at + 1);
    assert_eq!(
        after_sidebar.and_then(|at| order.get(at)),
        Some(&path(Part::Splitter(Pane::Sidebar))),
        "{order:?}"
    );
}

// ------------------------------------------------- composer growth, selectors

use chat_shell::state::{MODELS, Mode};

fn composer_heights(d: &mut Driver<App>, lines: usize) -> (f32, f32) {
    d.app_mut().draft = vec!["line"; lines].join("\n");
    d.frames(3);
    (
        d.rect(Target::suffix("/Frame/Centre/Composer/Field"))
            .height(),
        d.rect(Target::suffix("/Frame/Centre/Composer")).height(),
    )
}

#[test]
fn the_composer_grows_with_its_text_up_to_a_cap_and_stays_at_the_bottom() {
    let mut d = driver();
    let (field, composer): (Vec<f32>, Vec<f32>) = (1..=12)
        .map(|lines| composer_heights(&mut d, lines))
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

    let centre = rect(&d, Part::Centre);
    let composer = d.rect(Target::suffix("/Frame/Centre/Composer"));
    assert_eq!(centre.max.y - composer.max.y, 16.0, "still pinned");
    assert!(
        d.rect(Target::suffix("/Frame/Centre/Conversation")).max.y <= composer.min.y,
        "the conversation gave way"
    );
}

#[test]
fn a_capped_field_still_edits_and_enter_makes_a_row() {
    let mut d = driver();
    let (_, _) = composer_heights(&mut d, 12);
    let capped = d
        .rect(Target::suffix("/Frame/Centre/Composer/Field"))
        .height();
    type_reply(&mut d, "tail");
    assert!(d.app().draft.contains("tail"));
    assert_eq!(
        d.rect(Target::suffix("/Frame/Centre/Composer/Field"))
            .height(),
        capped
    );

    d.app_mut().draft.clear();
    d.frames(3);
    let resting = d
        .rect(Target::suffix("/Frame/Centre/Composer/Field"))
        .height();
    d.click(Target::suffix("/Frame/Centre/Composer/Field"));
    d.frames(2);
    for _ in 0..4 {
        d.press(egui::Key::Enter).frames(1);
    }
    assert!(
        d.rect(Target::suffix("/Frame/Centre/Composer/Field"))
            .height()
            > resting,
        "four newlines make a fifth row"
    );
}

#[test]
fn the_model_select_changes_the_model_and_the_details_follow() {
    let mut d = driver();
    let before = rect(&d, Part::Centre);
    assert_eq!(d.app().model, 0, "precondition");

    d.click(Target::suffix("/Frame/Centre/TopBar/Model"));
    d.frames(3);
    d.harness().get_by_label(MODELS[2]).click();
    d.frames(12);
    assert_eq!(d.app().model, 2);
    assert_eq!(rect(&d, Part::Centre), before, "nothing moved");

    let painted = d
        .harness()
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
    let mut d = driver();
    assert_eq!(d.app().mode, Mode::Standard);
    d.click(Target::suffix("/Frame/Centre/Composer/Mode/2"));
    d.frames(3);
    assert_eq!(d.app().mode, Mode::Thorough);

    let selected = color(&d, Tok::Line);
    let pressed: Vec<Mode> = Mode::ALL
        .iter()
        .enumerate()
        .filter(|(at, _)| {
            fill_of(&d, &format!("/Frame/Centre/Composer/Mode/{at}")) == Some(selected)
        })
        .map(|(_, (mode, _))| *mode)
        .collect();
    assert_eq!(pressed, [Mode::Thorough]);
}

#[test]
fn tab_reaches_the_model_then_the_composer_in_order_and_skips_an_inert_send() {
    let mut d = driver();
    let mut order = Vec::new();
    support::drive_past_bar(&mut d);
    for _ in 0..40 {
        d.press(egui::Key::Tab).frames(1);
        let Some(stop) = support::focused_node(&d) else {
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

    d.app_mut().draft = "x".into();
    d.frames(3);
    let mut with_send = Vec::new();
    support::drive_past_bar(&mut d);
    for _ in 0..40 {
        d.press(egui::Key::Tab).frames(1);
        let Some(stop) = support::focused_node(&d) else {
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
    use egui::Key;

    const MIN_ROWS: usize = 3;
    const MAX_ROWS: usize = 8;
    const FIELD: &str = "/Frame/Centre/Composer/Field";

    fn scaled_harness(pixels_per_point: f32) -> Driver<App> {
        let mut d = support::prepared_driver(
            WINDOW,
            Theme::new(egui::Theme::Dark).base_text(),
            App::default(),
            layout,
            prepare,
        )
        .pixels_per_point(PixelsPerPoint::new(pixels_per_point))
        .build();
        d.ctx().set_theme(egui::Theme::Dark);
        d.harness_mut().run();
        d.click(Target::suffix("/Frame/Centre/Composer/Field"));
        d.harness_mut().run();
        d
    }

    /// The rect egui drew the text edit at: its whole content, not the part
    /// a scroll area shows.
    fn edit_rect(d: &Driver<App>) -> egui::Rect {
        d.harness().get_by_label("Message").rect()
    }

    fn estimate(d: &Driver<App>) -> Vec2 {
        let app = &d.app();
        let field = eggshell_ui::prim::text_field::Field::multiline(&app.draft, MIN_ROWS)
            .max_rows(MAX_ROWS)
            .hint("Reply…")
            .bare();
        let width = d.rect(Target::suffix(FIELD)).width();
        field.size(
            d.ctx(),
            &Theme::of(d.ctx()).base_text(),
            &d.shell().style(),
            Some(Width::new(width)),
        )
    }

    /// A new line, one frame, and what the composer looks like in it.
    fn new_line(d: &mut Driver<App>) {
        d.type_text("x");
        d.press(Key::Enter);
    }

    #[test]
    fn the_field_is_laid_out_at_the_height_egui_draws_it_in_the_frame_of_the_keystroke() {
        for pixels_per_point in [1.0, 1.5, 2.0] {
            let mut d = scaled_harness(pixels_per_point);
            let snap = 1.0 / pixels_per_point;
            for rows in 1..=MAX_ROWS + 1 {
                new_line(&mut d);
                let drawn_rows = rows + 1;
                let field = d.rect(Target::suffix(FIELD));
                let edit = edit_rect(&d);
                let estimate = estimate(&d);
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
                    d.frames(1);
                    assert_eq!(
                        d.rect(Target::suffix(FIELD)),
                        field,
                        "{context}: the leaf moved with no input"
                    );
                    if !scrolling {
                        assert_eq!(
                            edit_rect(&d),
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

    fn settle(d: &mut Driver<App>) {
        d.frames(10);
    }

    /// The fill of the model select's trigger.
    fn select_fill(d: &Driver<App>) -> Option<egui::Color32> {
        fill_of(d, MODEL)
    }

    #[test]
    fn the_theme_the_app_shows_is_the_egui_theme() {
        let mut d = driver();
        assert_eq!(d.ctx().theme(), egui::Theme::Dark, "precondition");

        d.click(Target::suffix(LIGHT));
        settle(&mut d);
        assert_eq!(d.ctx().theme(), egui::Theme::Light);

        d.click(Target::suffix(DARK));
        settle(&mut d);
        assert_eq!(d.ctx().theme(), egui::Theme::Dark);

        d.click(Target::suffix(SYSTEM));
        settle(&mut d);
        assert_eq!(
            d.ctx().options(|options| options.theme_preference),
            egui::ThemePreference::System
        );
    }

    #[test]
    fn switching_recolours_the_next_frame_and_back() {
        let mut d = driver();
        let sidebar = "/Frame/Sidebar";
        let composer = "/Frame/Centre/Composer";
        let dark = Theme::new(egui::Theme::Dark);
        let light = Theme::new(egui::Theme::Light);
        assert_ne!(dark[Tok::Panel], light[Tok::Panel], "precondition");
        assert_ne!(dark[Tok::Surface], light[Tok::Surface], "precondition");
        settle(&mut d);
        assert_eq!(fill_of(&d, sidebar), Some(dark[Tok::Panel]));
        assert_eq!(fill_of(&d, composer), Some(dark[Tok::Surface]));
        let dark_select = select_fill(&d);
        assert!(
            dark_select.is_some(),
            "precondition: the select draws a fill"
        );

        d.click(Target::suffix(LIGHT));
        d.frames(1);
        assert_eq!(
            fill_of(&d, sidebar),
            Some(light[Tok::Panel]),
            "the frame after the click already has the new palette"
        );
        assert_eq!(fill_of(&d, composer), Some(light[Tok::Surface]));
        settle(&mut d);
        assert_ne!(select_fill(&d), dark_select, "the select follows");
        assert_eq!(
            select_fill(&d),
            Some(egui::Color32::TRANSPARENT),
            "the select is drawn with the preset's light style: bg-transparent"
        );

        d.click(Target::suffix(DARK));
        d.frames(1);
        assert_eq!(fill_of(&d, sidebar), Some(dark[Tok::Panel]));
        assert_eq!(fill_of(&d, composer), Some(dark[Tok::Surface]));
        settle(&mut d);
        assert_eq!(select_fill(&d), dark_select);
    }

    #[test]
    fn a_row_part_way_through_its_hover_transition_takes_the_new_palette() {
        let mut d = driver();
        let dark = Theme::new(egui::Theme::Dark);
        let light = Theme::new(egui::Theme::Light);
        assert_ne!(dark[Tok::Hover], light[Tok::Hover], "precondition");

        d.carry_to(d.rect(Target::suffix(NAV_ROW)).center())
            .frames(1);
        let midway = fill_of(&d, NAV_ROW);
        assert!(
            midway.is_some_and(|fill| fill.a() > 0 && fill.a() < 255),
            "precondition: the row is part-way to its hover fill: {midway:?}"
        );

        d.ctx().set_theme(egui::Theme::Light);
        d.frames(1);
        let after = fill_of(&d, NAV_ROW);
        assert!(
            after.is_some_and(|fill| fill.a() > 0 && fill.r() > dark[Tok::Hover].r()),
            "the row keeps the old palette's fill: {after:?}"
        );
        settle(&mut d);
        assert_eq!(fill_of(&d, NAV_ROW), Some(light[Tok::Hover]));
    }
}

mod accents {
    use super::*;
    use eggshell_ui::theme::ThemeColor;
    use egui::epaint::Shape;

    fn text_colors(d: &Driver<App>, text: &str) -> Vec<egui::Color32> {
        d.harness()
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
            let mut d = harness_in(theme, App::default());
            d.frames(3);
            let k = Theme::of(d.ctx());
            assert_eq!(text_colors(&d, "+24"), [k[Tok::Success]], "{theme:?}");
            assert_eq!(text_colors(&d, "-8"), [k[Tok::Danger]]);
            assert_eq!(text_colors(&d, "Running"), [k[Tok::Accent]]);
            assert_ne!(k[Tok::Success], k[Tok::Danger], "precondition");
            assert_eq!(
                eggshell_ui::theme::Shadcn::of(d.ctx()).tokens.primary,
                eggshell_ui::theme::Shadcn::for_theme(theme)
                    .with_palette(eggshell_ui::theme::Palette::new().theme(ThemeColor::Blue))
                    .tokens
                    .primary,
                "the example's accent is blue"
            );
            let meter = d.rect(Target::suffix(
                "/Frame/Details/\"Context\"/Region/Group/Row(\"Context window\")/Meter",
            ));
            assert!(meter.is_positive(), "precondition: the meter is laid out");
            assert!(
                d.harness().output().shapes.iter().any(|clipped| matches!(
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

    fn text_rect(d: &Driver<App>, within: egui::Rect, text: &str) -> egui::Rect {
        let rects: Vec<egui::Rect> = d
            .harness()
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
            let mut d = harness_in(theme, App::default());
            d.frames(3);
            let line = Theme::of(d.ctx())[Tok::Line];
            let group = d.rect(Target::suffix(
                "/Frame/Details/\"Environment\"/Region/Group",
            ));
            let rows = ["Branch", "Runtime", "Model"].map(|label| {
                d.rect(Target::suffix(&format!(
                    "/Frame/Details/\"Environment\"/Region/Group/Row(\"{label}\")"
                )))
            });
            assert!(
                rows.windows(2)
                    .all(|pair| pair[0].bottom() <= pair[1].top()),
                "precondition: the rows are stacked in order"
            );

            let rules: Vec<egui::Rect> = d
                .harness()
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
                let value = text_rect(&d, group, value);
                assert!(
                    (value.right() - (padding_edge - 12.0)).abs() <= 0.5,
                    "{label}: {value:?}"
                );
                assert!(text_rect(&d, group, label).left() - row.left() <= 12.5);
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

    fn rect(d: &Driver<App>, part: Part) -> egui::Rect {
        d.rect(Target::suffix(at(part)))
    }

    fn offset(d: &Driver<App>, part: Part) -> Vec2 {
        let id = support::id_of(d, at(part));
        d.shell().viewport(id).expect("laid out").offset()
    }

    fn wheel_over(d: &mut Driver<App>, part: Part) {
        let at = rect(d, part).center();
        d.carry_to(at);
        d.event(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -300.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        })
        .frames(29);
    }

    #[test]
    fn the_conversation_and_the_recents_scroll_independently_and_the_rest_stays_put() {
        let mut d = harness_with(long_app());
        d.frames(3);
        let fixed = [
            Part::TopBar,
            Part::Composer,
            Part::Account,
            Part::Conversation,
        ];
        let at_rest: Vec<egui::Rect> = fixed.iter().map(|&part| rect(&d, part)).collect();
        let content = |d: &Driver<App>, part: Part| {
            d.rect(Target::suffix(&format!("{}/<scrollcontent>", at(part))))
                .height()
        };
        assert!(
            content(&d, Part::Conversation) > rect(&d, Part::Conversation).height(),
            "precondition: the conversation overflows"
        );
        assert!(
            content(&d, Part::Recents) > rect(&d, Part::Recents).height(),
            "precondition: the recents overflow"
        );

        wheel_over(&mut d, Part::Conversation);
        assert!(
            offset(&d, Part::Conversation).y > 100.0,
            "the conversation scrolled"
        );
        assert_eq!(offset(&d, Part::Recents), Vec2::ZERO, "the recents did not");

        wheel_over(&mut d, Part::Recents);
        assert!(offset(&d, Part::Recents).y > 100.0, "the recents scrolled");

        let now: Vec<egui::Rect> = fixed.iter().map(|&part| rect(&d, part)).collect();
        assert_eq!(
            at_rest, now,
            "the headers, the composer and the account block stay put"
        );
    }

    #[test]
    fn a_session_scrolled_out_of_the_recents_takes_no_click() {
        let mut d = harness_with(long_app());
        d.frames(3);
        let last = SessionKey(100 + 23);
        assert!(
            d.rect(Target::suffix(&session_path(last))).min.y > rect(&d, Part::Recents).max.y,
            "precondition: the last session is below the recents' viewport"
        );
        let before = d.app().selected;
        let below = d.rect(Target::suffix(&session_path(last))).center();
        for pressed in [true, false] {
            d.event(Event::PointerButton {
                pos: below,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        d.frames(2);
        assert_eq!(d.app().selected, before, "nothing was selected");
    }
}

mod collapsing {
    use chat_shell::state::Panes;

    use super::*;

    fn shown(d: &Driver<App>, part: Part) -> bool {
        let wanted = path(part);
        support::nodes(d)
            .iter()
            .any(|(candidate, _)| *candidate == wanted)
    }

    fn panes_shown(d: &Driver<App>) -> [bool; 4] {
        [
            Part::Sidebar,
            Part::Splitter(Pane::Sidebar),
            Part::Details,
            Part::Splitter(Pane::Details),
        ]
        .map(|part| shown(d, part))
    }

    fn resized(size: Vec2) -> Driver<App> {
        let mut d = driver();
        d.resize(size);
        d
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
        let d = resized(vec2(1200.0, 760.0));
        assert_eq!(panes_shown(&d), [true; 4]);
    }

    #[test]
    fn a_narrower_window_gives_up_the_details_panel_first() {
        let wide = resized(vec2(1200.0, 760.0));
        let d = resized(vec2(900.0, 760.0));
        assert_eq!(panes_shown(&d), [true, true, false, false]);
        let (centre, window) = (rect(&d, Part::Centre), d.rect(Target::suffix("/Frame")));
        assert_eq!(
            centre.max.x, window.max.x,
            "the conversation takes the room the panel leaves"
        );
        assert!(centre.width() > rect(&wide, Part::Centre).width() - 400.0);
    }

    #[test]
    fn a_narrow_window_leaves_only_the_conversation() {
        let d = resized(vec2(600.0, 760.0));
        assert_eq!(panes_shown(&d), [false; 4]);
        assert_eq!(rect(&d, Part::Centre), d.rect(Target::suffix("/Frame")));
    }

    #[test]
    fn the_first_frame_after_a_resize_already_shows_the_panes_that_fit() {
        let mut d = driver();
        assert_eq!(panes_shown(&d), [true; 4], "precondition: all shown");
        for (size, expected) in [
            (vec2(900.0, 760.0), [true, true, false, false]),
            (vec2(600.0, 760.0), [false; 4]),
            (vec2(1200.0, 760.0), [true; 4]),
        ] {
            d.resize(size);
            assert_eq!(panes_shown(&d), expected, "at {size:?}");
            // A conversation whose width changed measures its messages at the
            // new width, which can take frames of its own; the panes do not.
            // At its end, each frame measures the messages it shows and can
            // bring a few more into view, so it settles in a handful.
            let mut more = 0;
            while d.ctx().requested_repaint_last_pass() && more < 6 {
                d.frames(1);
                more += 1;
                assert_eq!(panes_shown(&d), expected, "still at {size:?}");
            }
            assert!(
                !d.ctx().requested_repaint_last_pass(),
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
    fn messages(d: &Driver<App>) -> (usize, Option<egui::Rect>) {
        let last = format!(
            "/{:?}",
            d.app().conversation.ids().last().expect("a conversation")
        );
        let nodes = support::nodes(d);
        let rows: Vec<&(String, egui::Rect)> = nodes
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

    fn viewport(d: &Driver<App>) -> egui::Rect {
        let id = support::id_of(d, "/Frame/Centre/Conversation");
        d.shell()
            .viewport(id)
            .expect("the conversation was laid out")
            .rect
    }

    #[test]
    fn it_opens_on_the_latest_message_and_builds_only_a_screenful() {
        let mut d = driver();
        d.frames(4);
        assert!(
            d.app().conversation.ids().len() > HISTORY,
            "precondition: a long conversation"
        );
        let (built, last) = messages(&d);
        let last = last.expect("the latest message is built");
        let view = viewport(&d);
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
        let mut d = driver();
        d.frames(4);
        d.app_mut().draft = "One more question.".into();
        d.app_mut().send();
        d.frames(1);
        let mut frames = 0;
        while d.ctx().has_requested_repaint() && frames < 6 {
            d.frames(1);
            frames += 1;
        }
        let (_, last) = messages(&d);
        let last = last.expect("the new message is built");
        let view = viewport(&d);
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

    const HEAD: &str = "/Frame/Details/\"Changes\"/Header";
    const FOLD: &str = "/Frame/Details/\"Changes\"/Region";
    const NEXT: &str = "/Frame/Details/\"Background tasks\"";

    #[test]
    fn clicking_a_details_header_folds_its_section_and_the_next_one_follows() {
        let mut d = driver();
        d.frames(6);
        let open = d.rect(Target::suffix(FOLD)).height();
        let next = d.rect(Target::suffix(NEXT)).min.y;
        assert!(open > 50.0, "precondition: the section has rows: {open}");
        d.click(Target::suffix(HEAD));
        d.frames(30);
        assert_eq!(d.rect(Target::suffix(FOLD)).height(), 0.0);
        assert_eq!(d.rect(Target::suffix(NEXT)).min.y, next - open);
        d.click(Target::suffix(HEAD));
        d.frames(30);
        assert_eq!(d.rect(Target::suffix(FOLD)).height(), open);
        assert_eq!(d.rect(Target::suffix(NEXT)).min.y, next);
    }
}

mod splitter_glow {
    use super::*;

    /// The colour of the line painted down the middle of the sidebar's
    /// splitter.
    pub(super) fn line_color(d: &Driver<App>) -> Option<egui::Color32> {
        let x = rect(d, Part::Splitter(Pane::Sidebar)).center().x;
        support::shapes(d)
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::LineSegment { points, stroke } if points[0].x == x => {
                    Some(stroke.color)
                }
                _ => None,
            })
    }

    #[test]
    fn hovering_a_splitter_fades_its_line_to_the_accent() {
        let mut d = driver();
        support::full_motion(d.ctx());
        d.frames(4);
        let (line, accent) = (color(&d, Tok::Line), color(&d, Tok::Accent));
        assert_eq!(line_color(&d), Some(line), "precondition: at rest");
        d.hover(Target::suffix(&path(Part::Splitter(Pane::Sidebar))));
        let mid = line_color(&d).expect("the line is painted");
        let between = |channel: fn(egui::Color32) -> u8| {
            let (from, to, now) = (channel(line), channel(accent), channel(mid));
            from.min(to) < now && now < from.max(to)
        };
        assert!(
            between(|c| c.r()) || between(|c| c.g()) || between(|c| c.b()),
            "mid-transition {mid:?} is between {line:?} and {accent:?}"
        );
        d.frames(6);
        assert_eq!(line_color(&d), Some(accent));
    }
}
