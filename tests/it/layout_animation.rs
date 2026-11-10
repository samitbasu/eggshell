//! `animate_layout` through build, layout, draw and pointer events: a node
//! whose layout position changes is painted where it was and slides to where
//! layout put it, while its layout and hit rects are final at once.

use core::time::Duration;

use bullseye::{Builder, LayoutState, TextSpec, TextStyle, tw};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, press_and_release, rect_of};

const SLIDE: Duration = Duration::from_millis(1000);
const ROW: f32 = 30.0;

struct App {
    order: Vec<u32>,
    animated: Vec<u32>,
    lead: f32,
    clicked: Option<u32>,
}

impl App {
    fn new(order: &[u32], animated: &[u32]) -> Self {
        Self {
            order: order.to_vec(),
            animated: animated.to_vec(),
            lead: 10.0,
            clicked: None,
        }
    }
}

fn color(key: u32) -> Color32 {
    Color32::from_rgb(20 + 40 * key as u8, 100, 200)
}

const PARENT: Color32 = Color32::from_rgb(200, 10, 10);
const CHILD: Color32 = Color32::from_rgb(10, 200, 10);

/// ```html
/// <div class="w-[100px] h-[30px]" style="background: ..."></div>
/// ```
fn rows(b: &mut Builder<'_, '_, App>, app: &App) {
    for &key in &app.order {
        let row = tw().w_px(100.0).h_px(ROW).bg(color(key)).sense_click();
        let row = if app.animated.contains(&key) {
            row.animate_layout(SLIDE).ease_linear()
        } else {
            row
        };
        b.div_id(key, row, |_| {}).on_click(move |app: &mut App| {
            app.clicked = Some(key);
        });
    }
}

/// ```html
/// <div class="h-[lead]"></div>
/// <div class="w-[100px] h-[60px]"><div class="w-[50px] h-[20px]"></div></div>
/// ```
fn nested(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().w_px(100.0).h_px(app.lead), |_| {});
    b.div_id(
        "parent",
        tw().w_px(100.0)
            .h_px(60.0)
            .bg(PARENT)
            .animate_layout(SLIDE)
            .ease_linear(),
        |b| {
            b.div_id(
                "child",
                tw().w_px(50.0)
                    .h_px(20.0)
                    .bg(CHILD)
                    .animate_layout(SLIDE)
                    .ease_linear(),
                |_| {},
            );
        },
    );
}

fn harness(app: App, view: support::View<App>) -> Harness<'static, Rig<App>> {
    let text = TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    );
    let mut harness = support::harness(vec2(400.0, 300.0), text, app, view);
    harness.run_steps(3);
    harness
}

fn painted_top<A>(harness: &Harness<'_, Rig<A>>, fill: Color32) -> f32 {
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == fill => Some(rect.rect.min.y),
            _ => None,
        })
        .unwrap_or_else(|| panic!("nothing is painted in {fill:?}"))
}

fn painted_offset(harness: &Harness<'_, Rig<App>>, fill: Color32, path: &str) -> f32 {
    painted_top(harness, fill) - rect_of(harness, path).min.y
}

fn reverse(harness: &mut Harness<'_, Rig<App>>) {
    harness.state_mut().app.order.reverse();
}

fn requested_repaint<A>(harness: &Harness<'_, Rig<A>>) -> bool {
    harness.ctx.has_requested_repaint()
}

#[test]
fn a_reordered_row_slides_from_its_old_place_to_its_new_one() {
    let mut harness = harness(App::new(&[1, 2], &[1, 2]), rows);
    let old = painted_top(&harness, color(1));
    reverse(&mut harness);
    harness.step();
    let new = rect_of(&harness, "/1").min.y;
    assert_ne!(new, old, "precondition: the reorder moved row 1");

    let first = painted_top(&harness, color(1));
    assert!((first - old).abs() < 1.0, "painted at {first}, was {old}");

    let mut seen = vec![first];
    for _ in 0..9 {
        harness.step();
        seen.push(painted_top(&harness, color(1)));
    }
    let midway = *seen.last().expect("steps ran");
    assert!(
        (old..new).contains(&midway) || (new..old).contains(&midway),
        "part-way between {old} and {new}: {seen:?}"
    );
    assert!(
        seen.windows(2)
            .all(|pair| (pair[1] - new).abs() < (pair[0] - new).abs()),
        "closer to the new place every frame: {seen:?}"
    );

    harness.run_steps(12);
    assert_eq!(painted_top(&harness, color(1)), new);
    harness.step();
    assert!(!requested_repaint(&harness), "settled");
}

#[test]
fn layout_and_hit_rects_are_final_on_the_first_frame() {
    let mut harness = harness(App::new(&[1, 2], &[1, 2]), rows);
    let old = rect_of(&harness, "/1");
    reverse(&mut harness);
    harness.step();
    let new = rect_of(&harness, "/1");
    assert_ne!(new, old, "precondition: the reorder moved row 1");
    assert!(
        (painted_top(&harness, color(1)) - old.min.y).abs() < 1.0,
        "precondition: it is still painted at the old place"
    );

    let at = new.center();
    press_and_release(&mut harness, at);
    assert_eq!(harness.state().app.clicked, Some(1));
}

#[test]
fn nodes_that_did_not_opt_in_move_at_once() {
    let mut harness = harness(App::new(&[1, 2], &[1]), rows);
    reverse(&mut harness);
    harness.step();
    assert_eq!(
        painted_top(&harness, color(2)),
        rect_of(&harness, "/2").min.y
    );
    assert_ne!(
        painted_top(&harness, color(1)),
        rect_of(&harness, "/1").min.y,
        "precondition: the animated row is still on its way"
    );
}

#[test]
fn nothing_animates_or_repaints_when_nothing_moved() {
    let mut harness = harness(App::new(&[1, 2], &[1, 2]), rows);
    for _ in 0..3 {
        harness.step();
        assert_eq!(painted_offset(&harness, color(1), "/1"), 0.0);
        assert_eq!(painted_offset(&harness, color(2), "/2"), 0.0);
        assert!(!requested_repaint(&harness));
    }
}

#[test]
fn a_child_does_not_animate_again_when_only_its_parent_moved() {
    let mut harness = harness(App::new(&[], &[]), nested);
    harness.state_mut().app.lead = 50.0;
    harness.run_steps(8);
    let parent = painted_offset(&harness, PARENT, "/\"parent\"");
    let child = painted_offset(&harness, CHILD, "/\"parent\"/\"child\"");
    assert!(
        parent.abs() > 1.0,
        "precondition: the parent is part-way: {parent}"
    );
    assert!(
        (child - parent).abs() < 0.01,
        "the child rides on the parent's offset: {child} vs {parent}"
    );
}

#[test]
fn a_resize_moves_everything_at_once() {
    let mut harness = harness(App::new(&[1, 2], &[1, 2]), rows);
    reverse(&mut harness);
    harness.set_size(vec2(500.0, 300.0));
    harness.step();
    assert_eq!(
        harness.state().nodes.len(),
        3,
        "precondition: the row layout is built"
    );
    for key in [1, 2] {
        assert_eq!(
            painted_offset(&harness, color(key), &format!("/{key}")),
            0.0
        );
    }
    assert!(!requested_repaint(&harness));
}

#[test]
fn a_move_interrupted_mid_flight_starts_from_where_the_row_is_painted() {
    let mut harness = harness(App::new(&[1, 2], &[1, 2]), rows);
    let home = painted_top(&harness, color(1));
    reverse(&mut harness);
    harness.run_steps(8);
    let before = painted_top(&harness, color(1));
    assert!(
        before > home + 5.0 && before < home + ROW - 5.0,
        "precondition: row 1 is part-way down from {home}: {before}"
    );

    reverse(&mut harness);
    harness.step();
    let after = painted_top(&harness, color(1));
    let per_frame = ROW * 0.05 / SLIDE.as_secs_f32();
    assert!(
        (after - before).abs() <= per_frame * 1.5 + 0.5,
        "no jump: {before} to {after}"
    );
    let mut seen = vec![after];
    for _ in 0..30 {
        harness.step();
        seen.push(painted_top(&harness, color(1)));
    }
    assert!(
        seen.windows(2).all(|pair| pair[1] <= pair[0]),
        "heads back home monotonically: {seen:?}"
    );
    assert_eq!(*seen.last().expect("steps ran"), home);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only identified nodes")]
fn animating_an_anonymous_node_is_a_build_error() {
    let mut layout = LayoutState::new();
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    layout.build::<()>(&ctx, |b| {
        b.div(tw().animate_layout(SLIDE), |_| {});
    });
}

#[test]
fn nodes_that_are_no_longer_built_are_forgotten() {
    let mut harness = harness(App::new(&[1, 2, 3], &[1, 2, 3]), rows);
    let tracked = |harness: &Harness<'_, Rig<App>>| {
        bullseye::tracked_layout_nodes(&harness.ctx, harness.state().scope)
    };
    assert_eq!(tracked(&harness), 3);
    harness.state_mut().app.order.truncate(1);
    harness.step();
    assert_eq!(tracked(&harness), 1);
}
