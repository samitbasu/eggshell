//! `testing::Driver` through real views: each action reaches the widget it
//! names along the path a user's input takes, and each query answers from
//! the frame the driver last drew.

use eggshell_ui::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use eggshell_ui::contract::Themed;
use eggshell_ui::owned::splitter::PaneSize;
use eggshell_ui::testing::{After, Animation, Driver, Focused, Reports, Target, WindowFocus};
use eggshell_ui::{Builder, Problem, Style, tw};
use egui::{Key, Role, vec2};

#[derive(Hash, Debug, Clone, Copy)]
enum Part {
    Page,
    Save,
    Name,
    Sidebar,
    Split,
    Main,
    Help,
    Menu,
    Narrow,
}

#[derive(Default)]
struct App {
    clicks: u32,
    name: String,
    sidebar: f32,
    picked: u32,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(Part::Page, tw().flex().flex_col().gap(4.0).p(4.0), |b| {
        b.button(Part::Save, format!("Saved {}", app.clicks))
            .on_click(|app: &mut App| app.clicks += 1);
        b.text_field(Part::Name, app.name.clone())
            .named("Name")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.name = text);
        b.div_id(Part::Help, tw().w_px(80.0).h_px(20.0), |_| {})
            .tooltip("More about this");
        b.dropdown_menu(Part::Menu, "Menu").items(|m| {
            m.item("pick", "Pick")
                .on_click(|app: &mut App| app.picked += 1);
        });
    });
}

fn driver() -> Driver<App> {
    Driver::new(App::default(), view)
        .size(vec2(400.0, 300.0))
        .build()
}

#[test]
fn a_click_reaches_the_button_it_names() {
    let mut d = driver();
    d.click(d.path(Part::Page).at(Part::Save));
    assert_eq!(d.app().clicks, 1);
    assert!(
        d.painted().shows("Saved 1"),
        "the settle drew the new label"
    );
}

#[test]
fn every_kind_of_target_names_the_same_button() {
    let mut d = driver();
    let rect = d.rect(d.path(Part::Page).at(Part::Save));
    assert_eq!(d.rect(Target::suffix("/Save")), rect);
    assert!(
        rect.contains_rect(d.rect(Target::text("Saved 0"))),
        "the text names the label inside the button"
    );
    let id = d
        .frame()
        .at(Part::Page)
        .at(Part::Save)
        .id()
        .expect("an identified node");
    assert_eq!(d.rect(id), rect);
    assert!(
        d.rect(Target::role(Role::Button, "Saved 0"))
            .intersects(rect),
        "kittest finds the button where the frame placed it"
    );
    d.click(Target::role(Role::Button, "Saved 0"))
        .click(Target::text("Saved 1"))
        .click(rect.center());
    assert_eq!(d.app().clicks, 3);
}

#[test]
fn find_and_exists_answer_without_failing() {
    let d = driver();
    assert!(d.exists(d.path(Part::Page).at(Part::Save)));
    assert!(d.find(d.path(Part::Page).at("nothing")).is_none());
    assert!(!d.exists(Target::suffix("/nothing")));
    assert!(!d.exists(Target::text("Never painted")));
}

#[test]
#[should_panic(expected = r#"no node /Page/"nothing"; below /Page there are: /Page/Save"#)]
fn a_missing_path_fails_with_what_is_there() {
    let mut d = driver();
    d.click(d.path(Part::Page).at("nothing"));
}

#[test]
#[should_panic(expected = "more than one node's path ends with \"e\"")]
fn a_suffix_must_name_one_node() {
    let d = driver();
    d.rect(Target::suffix("e"));
}

#[test]
fn typing_reaches_the_field_with_focus() {
    let mut d = driver();
    d.click(d.path(Part::Page).at(Part::Name));
    assert!(
        matches!(d.focused(), Some(Focused::Widget { within: Some(path) }) if path == "/Page/Name"),
        "precondition: the field has focus, {:?}",
        d.focused()
    );
    d.type_text("Mono");
    assert_eq!(d.app().name, "Mono");
    d.select_all().type_text("Sans");
    assert_eq!(d.app().name, "Sans", "select-all chose the whole text");
}

#[test]
fn tab_to_moves_focus_until_the_target_has_it() {
    let mut d = driver();
    d.tab_to(d.path(Part::Page).at(Part::Name));
    assert!(
        matches!(d.focused(), Some(Focused::Widget { within: Some(path) }) if path == "/Page/Name")
    );
    d.press(Key::Tab);
    assert!(
        !matches!(d.focused(), Some(Focused::Widget { within: Some(path) }) if path == "/Page/Name"),
        "Tab moved on"
    );
}

fn menu_open(d: &Driver<App>) -> bool {
    let id = d.frame().at(Part::Page).at(Part::Menu).id();
    id.is_some_and(|id| d.frame().menu_open_with(id))
}

#[test]
fn a_blurred_window_closes_an_open_menu() {
    let mut d = driver();
    d.click(d.path(Part::Page).at(Part::Menu));
    assert!(menu_open(&d), "precondition: the menu opened");
    d.window_focus(WindowFocus::Blurred);
    assert!(!menu_open(&d));
}

#[test]
fn a_menu_item_is_picked_by_its_role() {
    let mut d = driver();
    d.click(d.path(Part::Page).at(Part::Menu));
    d.click(Target::role(Role::MenuItem, "Pick"));
    assert_eq!(d.app().picked, 1);
}

#[test]
fn a_tooltip_opens_after_its_delay_and_not_after_a_settle() {
    let mut d = driver();
    d.hover(d.path(Part::Page).at(Part::Help));
    assert!(
        !d.painted().shows("More about this"),
        "a settle ends at the delay's repaint_after"
    );
    let style = d.shell().style();
    d.advance(style.tooltip.delay + style.motion.tooltip.enter + Duration::from_millis(100));
    assert!(d.painted().shows("More about this"));
}

/// Both halves of a style whose tooltips wait `delay`.
fn waiting(delay: Duration) -> Themed<Arc<Style>> {
    let mut style = Style::default();
    let mut tooltip = style.tooltip.clone();
    tooltip.delay = delay;
    style.tooltip = tooltip;
    Themed::both(Arc::new(style))
}

/// Whether the tooltip shows `wait` after the pointer reaches its trigger.
fn shows_after(d: &mut Driver<App>, wait: Duration) -> bool {
    d.hover(d.path(Part::Page).at(Part::Help));
    d.advance(wait);
    let shown = d.painted().shows("More about this");
    d.hover(d.path(Part::Page).at(Part::Save));
    d.advance(Duration::from_secs(1));
    shown
}

#[test]
fn a_driver_builds_with_the_style_it_is_given() {
    let mut d = Driver::new(App::default(), view)
        .size(vec2(400.0, 300.0))
        .style(&eggshell_ui::SharedStyle::new(waiting(
            Duration::from_secs(3),
        )))
        .build();
    let enter = d.shell().style().motion.tooltip.enter;
    assert!(!shows_after(&mut d, Duration::from_secs(2)));
    assert!(shows_after(
        &mut d,
        Duration::from_secs(3) + enter + Duration::from_millis(100)
    ));
}

#[test]
fn set_style_gives_the_layout_a_new_style_from_the_next_frame() {
    let mut d = Driver::new(App::default(), view)
        .size(vec2(400.0, 300.0))
        .style(&eggshell_ui::SharedStyle::new(waiting(
            Duration::from_secs(3),
        )))
        .build();
    let enter = d.shell().style().motion.tooltip.enter;
    let quick = Duration::from_millis(200) + enter + Duration::from_millis(100);
    assert!(
        !shows_after(&mut d, quick),
        "precondition: the first style waits"
    );
    d.set_style(waiting(Duration::from_millis(200)));
    assert!(shows_after(&mut d, quick));
}

#[test]
fn the_clock_moves_one_step_a_frame() {
    let mut d = Driver::new(App::default(), view)
        .step(Duration::from_millis(50))
        .build();
    let start = d.now();
    d.frames(4);
    let moved = d.now() - start;
    assert!(
        (moved.as_secs_f64() - 0.2).abs() < 1e-6,
        "four frames of 50 ms are 200 ms, got {moved:?}"
    );
    d.advance(Duration::from_millis(120));
    let moved = d.now() - start;
    assert!(
        (moved.as_secs_f64() - 0.35).abs() < 1e-6,
        "120 ms rounds up to three frames, got {moved:?}"
    );
}

fn split(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(Part::Page, tw().flex().size_full(), |b| {
        b.div_id(Part::Sidebar, tw().w_px(app.sidebar), |_| {});
        b.splitter(Part::Split)
            .before(PaneSize::new(app.sidebar))
            .range(100.0..=300.0)
            .named("Sidebar")
            .on_resize(|app: &mut App, size| app.sidebar = size.into());
        b.div_id(Part::Main, tw().flex_1(), |_| {});
    });
}

#[test]
fn a_drag_moves_a_splitter() {
    let app = App {
        sidebar: 150.0,
        ..App::default()
    };
    let mut d = Driver::new(app, split).size(vec2(400.0, 300.0)).build();
    let handle = d.rect(d.path(Part::Page).at(Part::Split)).center();
    d.drag(handle, handle + vec2(40.0, 0.0));
    assert!(
        (d.app().sidebar - 190.0).abs() < 1.0,
        "the sidebar followed the pointer, {}",
        d.app().sidebar
    );
    assert!((d.rect(d.path(Part::Page).at(Part::Sidebar)).width() - d.app().sidebar).abs() < 0.5);
}

fn spinning(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("spinner", tw().w_px(20.0).h_px(20.0), |_| {})
        .draw(|_, ui| ui.ctx().request_repaint());
}

#[test]
fn a_view_that_never_settles_names_its_repaint() {
    let mut d = Driver::new((), spinning)
        .after(After::Frames(0))
        .settle_within(Duration::from_millis(200))
        .build();
    let unsettled = d.try_settle().expect_err("the spinner repaints forever");
    assert!(
        unsettled
            .causes
            .iter()
            .any(|cause| cause.to_string().contains("driver.rs")),
        "the cause is the spinner's request: {:?}",
        unsettled.causes
    );
}

#[test]
#[should_panic(expected = "did not settle within")]
fn settle_fails_on_a_view_that_never_settles() {
    let _ = Driver::new((), spinning)
        .settle_within(Duration::from_millis(200))
        .build();
}

fn duplicated(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("twice", tw(), |_| {});
    b.div_id("twice", tw(), |_| {});
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "did not claim with expect_reported")]
fn an_unclaimed_report_fails_the_test_when_the_driver_drops() {
    let _d = Driver::new((), duplicated).build();
}

#[cfg(debug_assertions)]
#[test]
fn a_claimed_report_passes() {
    let mut d = Driver::new((), duplicated).build();
    let report = d.expect_reported(Problem::DuplicateKey);
    assert!(report.message.contains("twice"));
    d.frames(2);
}

#[cfg(debug_assertions)]
#[test]
fn an_unchecked_driver_leaves_reports_to_the_test() {
    let d = Driver::new((), duplicated)
        .reports(Reports::Unchecked)
        .build();
    assert!(!d.reports().is_empty());
}

#[test]
fn reduced_motion_jumps_to_the_end() {
    let d = Driver::new(App::default(), view)
        .motion(Animation::Reduced)
        .build();
    assert_eq!(d.ctx().global_style().animation_time, 0.0);
    let full = driver();
    assert!(full.ctx().global_style().animation_time > 0.0);
}

fn narrow(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(Part::Page, tw().flex().flex_col().gap(4.0).p(4.0), |b| {
        b.div_id(Part::Narrow, tw().w_px(60.0), |b| {
            b.span("long", tw(), "A label far too long for its box");
        });
        b.span("short", tw(), "Fits");
    });
}

#[test]
fn text_findings_name_the_text_that_wraps() {
    let d = Driver::new((), narrow).size(vec2(400.0, 300.0)).build();
    let wrapped = d.painted().expect("A label far too long for its box");
    assert!(wrapped.rows() > 1, "precondition: the long label wraps");
    assert!(d.painted().expect("Fits").fits());
    let findings = d.text_findings();
    assert_eq!(findings.len(), 1, "{findings:#?}");
    let finding = &findings[0];
    assert_eq!(finding.text, "A label far too long for its box");
    assert_eq!(finding.node.as_deref(), Some(r#"/Page/Narrow/"long""#));
    assert!(finding.wraps());
}

#[test]
fn painted_text_knows_its_parts() {
    let d = driver();
    let label = d.painted().expect("Saved 0");
    let part = label.part_rect("0").expect("a part on one row");
    assert!(label.rect().contains_rect(part));
    assert!(part.width() < label.rect().width());
    assert_eq!(d.painted().all("Saved 0").len(), 1);
}

#[test]
#[should_panic(expected = "is outside the window")]
fn a_press_outside_the_window_fails() {
    let mut d = driver();
    d.click(egui::pos2(-20.0, 10.0));
}

#[test]
fn a_held_drag_may_leave_the_window() {
    let app = App {
        sidebar: 150.0,
        ..App::default()
    };
    let mut d = Driver::new(app, split).size(vec2(400.0, 300.0)).build();
    let handle = d.rect(d.path(Part::Page).at(Part::Split)).center();
    d.drag(handle, egui::pos2(-200.0, handle.y));
    assert_eq!(d.app().sidebar, 100.0, "the sidebar stops at its least");
}
