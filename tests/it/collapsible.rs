//! Collapsibles through the real path: build, the two-pass layout, draw and
//! pointer events, with a fixed time step so that a motion lasts a known
//! number of frames.

use core::time::Duration;

use eggshell_ui::overlay::Shown;
use eggshell_ui::testing::{Animation, Driver, Points, Target, Timeline, record};
use eggshell_ui::{Builder, CollapseAxis, TextSpec, TextStyle, Weight, tw};
use eggshell_ui::{Easing, Progress};
use egui::{Color32, FontFamily, pos2, vec2};

use crate::support::{self, press_and_release};

/// Frames are this far apart; the default motion lasts 20 of them.
const STEP: Duration = Duration::from_millis(10);
const HEAD: f32 = 30.0;
const FILL: f32 = 100.0;
const TAIL: f32 = 20.0;
const FILL_COLOR: Color32 = Color32::from_rgb(10, 120, 40);
const TAIL_COLOR: Color32 = Color32::from_rgb(200, 30, 30);

#[derive(Default)]
struct App {
    open: [bool; 3],
    lead: f32,
    clicked: u32,
    collapsed: u32,
    duration: Option<Duration>,
}

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn section(i: usize) -> String {
    format!(r#"/"page"/("sec", {i})"#)
}

fn body(i: usize) -> String {
    format!(r#"{}/("body", {i})"#, section(i))
}

fn fill(i: usize) -> String {
    format!(r#"{}/("fill", {i})"#, body(i))
}

const TAIL_PATH: &str = r#"/"page"/"tail""#;

/// ```html
/// <div class="flex w-[200px] flex-col">
///   <div class="h-[lead]"></div>
///   <!-- three times -->
///   <div class="flex flex-col"> <div class="h-[30px]"></div> <collapsible> <div class="h-[100px]"></div> </collapsible> </div>
///   <div id="tail" class="h-[20px]"></div>
/// </div>
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div_id("page", tw().flex().flex_col().w_px(200.0), |s| {
        s.div(tw().h_px(app.lead), |_| {});
        for i in 0..3 {
            s.div_id(("sec", i), tw().flex().flex_col(), |s| {
                s.div_id(("head", i), tw().h_px(HEAD).w_px(200.0), |_| {});
                let chain = s.collapsible(("body", i), Shown::from(app.open[i]));
                let chain = match app.duration {
                    Some(duration) => chain.transition(duration),
                    None => chain,
                };
                let chain = if i == 0 {
                    chain.on_collapsed(|app: &mut App| app.collapsed += 1)
                } else {
                    chain
                };
                chain.children(|s| {
                    s.div_id(
                        ("fill", i),
                        tw().w_px(200.0).h_px(FILL).bg(FILL_COLOR).sense_click(),
                        |_| {},
                    )
                    .on_click(|app: &mut App| app.clicked += 1);
                });
            });
        }
        s.div_id(
            "tail",
            tw().w_px(200.0)
                .h_px(TAIL)
                .bg(TAIL_COLOR)
                .animate_layout(Duration::from_millis(1000))
                .ease_linear(),
            |_| {},
        );
    });
}

/// A driver for `view` of `app` whose frames are [`STEP`] apart.
fn stepped<A: 'static>(size: egui::Vec2, app: A, view: support::View<A>) -> Driver<A> {
    let mut d = support::driver(size, base_text(), app, view)
        .step(STEP)
        .settle_within(STEP * 4)
        .build();
    d.frames(4);
    d
}

fn harness_of(app: App) -> Driver<App> {
    stepped(vec2(400.0, 500.0), app, view)
}

fn harness() -> Driver<App> {
    harness_of(App::default())
}

fn height<A>(d: &Driver<A>, path: &str) -> f32 {
    d.rect(Target::suffix(path)).height()
}

fn has(d: &Driver<App>, path: &str) -> bool {
    support::nodes(d)
        .iter()
        .any(|(candidate, _)| candidate == path)
}

/// The heights of the wrapper after each of `frames` frames.
fn heights(d: &mut Driver<App>, frames: usize, path: &str) -> Vec<f32> {
    (0..frames)
        .map(|_| {
            d.frames(1);
            height(d, path)
        })
        .collect()
}

fn open(d: &mut Driver<App>, i: usize, open: bool) {
    d.app_mut().open[i] = open;
}

fn settled_open(i: usize) -> Driver<App> {
    let mut d = harness();
    open(&mut d, i, true);
    d.frames(40);
    assert_eq!(height(&d, &body(i)), FILL, "precondition: it is open");
    d
}

fn painted_top(d: &Driver<App>, fill: Color32) -> f32 {
    support::shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.rect.min.y),
            _ => None,
        })
        .unwrap_or_else(|| panic!("nothing is painted in {fill:?}"))
}

#[test]
fn a_closed_region_has_no_size_and_builds_nothing() {
    let d = harness();
    assert_eq!(height(&d, &body(0)), 0.0);
    assert!(!has(&d, &fill(0)), "its content is not built");
}

#[test]
fn a_region_first_seen_open_is_open_at_once() {
    let d = harness_of(App {
        open: [true, false, false],
        ..App::default()
    });
    assert_eq!(height(&d, &body(0)), FILL);
}

#[test]
fn opening_runs_from_nothing_to_the_content_along_the_curve() {
    let mut d = harness();
    open(&mut d, 0, true);
    let run = heights(&mut d, 40, &body(0));
    assert!(run[0] < FILL / 2.0, "it starts small: {run:?}");
    assert!(
        run.windows(2).all(|pair| pair[0] <= pair[1]),
        "it only grows: {run:?}"
    );
    assert_eq!(*run.last().expect("frames"), FILL);
    let arrived = run.iter().position(|&h| h == FILL).expect("it arrives");
    assert!(
        (15..=25).contains(&arrived),
        "200 ms of 10 ms frames: {arrived}"
    );
    let halfway = run[arrived / 2];
    let expected = FILL * f32::from(Easing::Out.ease(Progress::new(0.5)));
    assert!(
        (halfway - expected).abs() < 12.0,
        "ease-out is ahead at half time: {halfway} against {expected}: {run:?}"
    );
}

#[test]
fn what_follows_the_region_moves_with_it() {
    let mut d = harness();
    let closed_tail = d.rect(Target::suffix(TAIL_PATH)).min.y;
    open(&mut d, 0, true);
    let mut mid_motion = 0;
    for _ in 0..40 {
        d.frames(1);
        let grown = height(&d, &body(0));
        assert_eq!(
            d.rect(Target::suffix(TAIL_PATH)).min.y,
            closed_tail + grown,
            "the tail is where layout puts it"
        );
        mid_motion += usize::from(grown > 0.0 && grown < FILL);
    }
    assert!(mid_motion > 5, "precondition: it moved in between frames");
}

#[test]
fn the_content_is_clipped_at_its_natural_size_not_squeezed() {
    let mut d = harness();
    open(&mut d, 0, true);
    for _ in 0..40 {
        d.frames(1);
        let wrapper = d.rect(Target::suffix(&body(0)));
        if wrapper.height() > 0.0 && wrapper.height() < FILL {
            let content = d.rect(Target::suffix(&fill(0)));
            assert_eq!(content.size(), vec2(200.0, FILL), "{wrapper:?}");
            assert_eq!(content.min, wrapper.min);
            return;
        }
    }
    panic!("the region was never in between");
}

#[test]
fn the_part_of_the_content_below_the_clip_cannot_be_clicked() {
    let mut d = harness();
    open(&mut d, 0, true);
    let at = (0..40)
        .find_map(|_| {
            d.frames(1);
            let wrapper = d.rect(Target::suffix(&body(0)));
            (wrapper.height() > 30.0 && wrapper.height() < 60.0)
                .then(|| pos2(wrapper.center().x, wrapper.min.y + FILL - 10.0))
        })
        .expect("the region was in between");
    let wrapper = d.rect(Target::suffix(&body(0)));
    let content = d.rect(Target::suffix(&fill(0)));
    assert!(
        content.contains(at) && !wrapper.contains(at),
        "precondition: the point is in the content and below the clip edge"
    );
    press_and_release(&mut d, at);
    assert_eq!(d.app().clicked, 0);
    d.frames(40);
    let at = pos2(at.x, d.rect(Target::suffix(&fill(0))).max.y - 10.0);
    press_and_release(&mut d, at);
    assert_eq!(d.app().clicked, 1, "open, it can be clicked");
}

#[test]
fn a_region_that_is_closing_takes_no_input() {
    let mut d = settled_open(0);
    open(&mut d, 0, false);
    d.frames(3);
    let wrapper = d.rect(Target::suffix(&body(0)));
    assert!(
        wrapper.height() > 60.0 && wrapper.height() < FILL,
        "precondition: it is closing: {wrapper:?}"
    );
    press_and_release(&mut d, wrapper.min + vec2(20.0, 20.0));
    assert_eq!(d.app().clicked, 0);
}

#[test]
fn closing_unbuilds_the_content_and_collapses_once() {
    let mut d = settled_open(0);
    open(&mut d, 0, false);
    let run = heights(&mut d, 40, &body(0));
    assert!(
        run.windows(2).all(|pair| pair[0] >= pair[1]),
        "it only shrinks: {run:?}"
    );
    assert_eq!(*run.last().expect("frames"), 0.0);
    assert!(!has(&d, &fill(0)));
    assert_eq!(d.app().collapsed, 1);
    d.frames(10);
    assert_eq!(d.app().collapsed, 1, "once");
}

#[test]
fn it_does_not_collapse_before_it_has_closed() {
    let mut d = settled_open(0);
    open(&mut d, 0, false);
    d.frames(5);
    assert_eq!(d.app().collapsed, 0);
}

#[test]
fn toggling_in_the_middle_reverses_from_where_it_is() {
    let mut d = harness();
    open(&mut d, 0, true);
    d.frames(8);
    let before = height(&d, &body(0));
    assert!(
        before > 20.0 && before < 90.0,
        "precondition: in the middle: {before}"
    );
    open(&mut d, 0, false);
    let run = heights(&mut d, 40, &body(0));
    assert!(
        (run[0] - before).abs() < 25.0,
        "no jump: {before} then {}",
        run[0]
    );
    assert!(run.windows(2).all(|pair| pair[0] >= pair[1]), "{run:?}");
    assert_eq!(*run.last().expect("frames"), 0.0);
    let back = run.iter().position(|&h| h == 0.0).expect("it closes") + 1;
    assert!(back < 20, "less far to go, so quicker: {back} frames");
}

#[test]
fn siblings_open_and_close_together() {
    let mut d = settled_open(0);
    open(&mut d, 0, false);
    open(&mut d, 1, true);
    d.frames(8);
    let (closing, opening) = (height(&d, &body(0)), height(&d, &body(1)));
    assert!(
        closing > 0.0 && closing < FILL && opening > 0.0 && opening < FILL,
        "precondition: both are moving: {closing} {opening}"
    );
    d.frames(40);
    assert_eq!(height(&d, &body(0)), 0.0);
    assert_eq!(height(&d, &body(1)), FILL);
}

#[test]
fn a_position_slide_does_not_run_on_top_of_a_size_animation() {
    let mut d = harness();
    open(&mut d, 0, true);
    let mut checked = 0;
    for _ in 0..40 {
        d.frames(1);
        let grown = height(&d, &body(0));
        if grown > 0.0 && grown < FILL {
            let laid_out = d.rect(Target::suffix(TAIL_PATH)).min.y;
            assert!(
                (painted_top(&d, TAIL_COLOR) - laid_out).abs() <= 0.5,
                "the tail is painted where layout puts it: {laid_out} and {}",
                painted_top(&d, TAIL_COLOR)
            );
            checked += 1;
        }
    }
    assert!(checked > 5, "precondition: frames in between");
}

#[test]
fn a_position_slide_runs_again_once_the_size_animation_is_over() {
    let mut d = settled_open(0);
    d.app_mut().lead = 40.0;
    d.frames(1);
    let laid_out = d.rect(Target::suffix(TAIL_PATH)).min.y;
    assert!(
        painted_top(&d, TAIL_COLOR) < laid_out - 5.0,
        "the tail slides to its new place"
    );
}

#[test]
fn the_size_animation_is_continuous_and_leaves_no_gaps() {
    let mut d = harness();
    record(d.ctx());
    open(&mut d, 0, true);
    d.frames(8);
    open(&mut d, 0, false);
    open(&mut d, 2, true);
    d.frames(40);
    Timeline::recorded(d.ctx())
        .max_resize(Points::new(30.0))
        .assert_continuous();
}

#[test]
fn a_jump_is_caught_by_the_resize_check() {
    let mut d = harness_of(App {
        duration: Some(Duration::ZERO),
        ..App::default()
    });
    record(d.ctx());
    d.frames(1);
    open(&mut d, 0, true);
    d.frames(6);
    let timeline = Timeline::recorded(d.ctx());
    let found = timeline.resizes();
    assert!(!found.is_empty(), "a snap of {FILL} is more than a step");
}

#[test]
fn a_style_without_animation_time_opens_and_closes_at_once() {
    let mut d = harness();
    d.set_motion(Animation::Reduced);
    d.ctx().global_style_mut(|style| style.animation_time = 0.0);
    open(&mut d, 0, true);
    d.frames(1);
    d.frames(1);
    assert_eq!(height(&d, &body(0)), FILL);
    open(&mut d, 0, false);
    d.frames(1);
    d.frames(1);
    assert_eq!(height(&d, &body(0)), 0.0);
    assert_eq!(d.app().collapsed, 1);
}

mod sidebar {
    use super::*;

    const WIDTH: f32 = 160.0;

    #[derive(Default)]
    struct Side {
        open: bool,
    }

    /// ```html
    /// <div class="flex">
    ///   <collapsible axis=width> <div class="w-[160px] h-[40px]"> a text </div> </collapsible>
    ///   <div id="main" class="w-[50px] h-[40px]"></div>
    /// </div>
    /// ```
    fn view<'f>(b: &mut Builder<'_, 'f, Side>, app: &'f Side) {
        b.div_id("row", tw().flex(), |s| {
            s.collapsible("side", Shown::from(app.open))
                .axis(CollapseAxis::Width)
                .children(|s| {
                    s.div_id("panel", tw().w_px(WIDTH).h_px(40.0), |s| {
                        s.text(tw(), "A text that would wrap if it were squeezed");
                    });
                });
            s.div_id("main", tw().w_px(50.0).h_px(40.0), |_| {});
        });
    }

    #[test]
    fn a_sidebar_opens_by_its_width_and_its_content_keeps_its_own() {
        let mut d = stepped(vec2(400.0, 100.0), Side::default(), view);
        assert_eq!(d.rect(Target::suffix(r#"/"row"/"side""#)).width(), 0.0);
        d.app_mut().open = true;
        let mut between = 0;
        for _ in 0..40 {
            d.frames(1);
            let side = d.rect(Target::suffix(r#"/"row"/"side""#));
            assert_eq!(
                d.rect(Target::suffix(r#"/"row"/"main""#)).min.x,
                side.max.x,
                "what follows moves with it"
            );
            if side.width() > 0.0 && side.width() < WIDTH {
                between += 1;
                let panel = d.rect(Target::suffix(r#"/"row"/"side"/"panel""#));
                assert_eq!(panel.width(), WIDTH, "not squeezed");
            }
        }
        assert!(between > 5);
        assert_eq!(d.rect(Target::suffix(r#"/"row"/"side""#)).width(), WIDTH);
    }
}

mod scrolling {
    use super::*;

    #[derive(Default)]
    struct Page {
        open: bool,
    }

    fn view<'f>(b: &mut Builder<'_, 'f, Page>, app: &'f Page) {
        b.scroll("list", tw().w_px(200.0).h_px(150.0))
            .children(|s| {
                s.collapsible("body", Shown::from(app.open)).children(|s| {
                    s.div_id("fill", tw().w_px(200.0).h_px(FILL), |_| {});
                });
                s.div_id("after", tw().w_px(200.0).h_px(40.0), |_| {});
            });
    }

    #[test]
    fn a_region_in_a_scroll_container_moves_what_follows_it() {
        let mut d = stepped(vec2(400.0, 300.0), Page::default(), view);
        let after = |d: &Driver<Page>| d.rect(Target::suffix(r#"/"list"/"after""#)).min.y;
        let closed = after(&d);
        d.app_mut().open = true;
        d.frames(1);
        d.frames(1);
        let moving = after(&d);
        assert!(moving > closed && moving < closed + FILL, "{moving}");
        d.frames(40);
        assert_eq!(after(&d), closed + FILL);
    }
}

mod flight {
    use super::*;
    use eggshell_ui::{CollapseAxis, Flight, LayoutState};

    fn pass(ctx: &egui::Context, layout: &mut LayoutState, app: &App, time: f64) -> Option<Flight> {
        ctx.begin_pass(egui::RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(400.0, 500.0),
            )),
            ..egui::RawInput::default()
        });
        let placed = layout
            .build(ctx, |b| view(b, app))
            .layout(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(400.0, 500.0),
            ));
        let flight = placed
            .nodes()
            .find(|node| node.path() == body(0))
            .and_then(|node| node.size_flight());
        ctx.end_pass().textures_delta.clear();
        flight
    }

    #[test]
    fn a_region_in_motion_says_where_it_is_going_and_where_it_is() {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput::default());
        ctx.end_pass().textures_delta.clear();
        let mut layout = LayoutState::new().with_base_text(base_text());
        let mut app = App::default();
        assert_eq!(pass(&ctx, &mut layout, &app, 0.0), None, "closed");
        app.open[0] = true;
        pass(&ctx, &mut layout, &app, 0.01);
        let flight = pass(&ctx, &mut layout, &app, 0.1).expect("it is opening");
        assert_eq!(flight.axis, CollapseAxis::Height);
        assert_eq!(flight.target, FILL);
        assert!(flight.current > 0.0 && flight.current < FILL, "{flight:?}");
        assert_eq!(pass(&ctx, &mut layout, &app, 1.0), None, "open and still");
    }
}

mod header {
    use std::cell::Cell;

    use super::*;

    #[derive(Default)]
    struct Page {
        open: bool,
        progress: Cell<f32>,
    }

    /// ```html
    /// <header/> <collapsible> <div class="h-[100px]"></div> </collapsible>
    /// ```
    fn view<'f>(b: &mut Builder<'_, 'f, Page>, app: &'f Page) {
        b.collapsible("body", Shown::from(app.open))
            .children_with_header(
                |_, progress| app.progress.set(f32::from(progress)),
                |s| {
                    s.div_id("fill", tw().w_px(200.0).h_px(FILL), |_| {});
                },
            );
    }

    fn harness() -> Driver<Page> {
        stepped(vec2(400.0, 300.0), Page::default(), view)
    }

    fn lag(d: &Driver<Page>) -> f32 {
        let height = d.rect(Target::suffix(r#"/"body""#)).height();
        (d.app().progress.get() - height / FILL).abs()
    }

    #[test]
    fn the_header_is_told_how_far_open_the_region_is_by_the_value_its_size_follows() {
        let mut d = harness();
        assert_eq!(d.app().progress.get(), 0.0, "closed");
        d.app_mut().open = true;
        let mut between = 0;
        for _ in 0..40 {
            d.frames(1);
            assert!(lag(&d) < 0.005, "{}", lag(&d));
            let progress = d.app().progress.get();
            between += usize::from(progress > 0.05 && progress < 0.95);
        }
        assert!(between > 5, "precondition: it was in between");
        assert_eq!(d.app().progress.get(), 1.0);
    }

    #[test]
    fn it_reverses_with_the_size() {
        let mut d = harness();
        d.app_mut().open = true;
        d.frames(8);
        let before = d.app().progress.get();
        assert!(before > 0.2 && before < 0.95, "precondition: {before}");
        d.app_mut().open = false;
        d.frames(1);
        let mut last = d.app().progress.get();
        assert!((last - before).abs() < 0.1, "no jump: {before} then {last}");
        for _ in 0..30 {
            d.frames(1);
            let now = d.app().progress.get();
            assert!(now <= last + 1e-6, "falls: {last} then {now}");
            assert!(lag(&d) < 0.005);
            last = now;
        }
        assert_eq!(last, 0.0);
    }

    #[test]
    fn a_style_without_animation_time_snaps_it() {
        let mut d = harness();
        d.set_motion(Animation::Reduced);
        d.ctx().global_style_mut(|style| style.animation_time = 0.0);
        d.app_mut().open = true;
        d.frames(1);
        d.frames(1);
        assert_eq!(d.app().progress.get(), 1.0);
        d.app_mut().open = false;
        d.frames(1);
        d.frames(1);
        assert_eq!(d.app().progress.get(), 0.0);
    }
}
