//! Interaction through the real path: pointer events dispatched by
//! `egui_kittest::Harness` into egui's hit test, the library's one
//! `ui.interact` per sensing node, variant painting and draw closures.

use core::time::Duration;

use bullseye::{
    Builder, Easing, LayoutState, NodeUi, Opacity, Progress, TextSpec, TextStyle, none, tw,
};
use egui::epaint::{ClippedShape, RectShape, Shape};
use egui::{Color32, FontFamily, Pos2, Rect, pos2, vec2};
use egui_kittest::Harness;

type View<A> = for<'b, 'f> fn(&mut Builder<'b, 'f, A>, &A);

struct Rig<A> {
    layout: LayoutState,
    app: A,
    view: View<A>,
    origin: Pos2,
}

const STEP: f32 = 0.05;

fn harness<A: 'static>(app: A, view: View<A>) -> Harness<'static, Rig<A>> {
    let layout = LayoutState::new().with_base_text(TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    ));
    let rig = Rig {
        layout,
        app,
        view,
        origin: Pos2::ZERO,
    };
    Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(STEP)
        .build_ui_state(
            |ui, rig: &mut Rig<A>| {
                let area = ui.max_rect();
                rig.origin = area.min;
                let Rig {
                    layout, app, view, ..
                } = rig;
                layout
                    .build(ui.ctx(), |b| view(b, &*app))
                    .layout(area)
                    .draw(app, ui);
            },
            rig,
        )
}

impl<A> Rig<A> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y)
    }
}

fn point<A>(harness: &Harness<'_, Rig<A>>, x: f32, y: f32) -> Pos2 {
    harness.state().at(x, y)
}

fn hover<A>(harness: &mut Harness<'_, Rig<A>>, x: f32, y: f32) {
    let at = point(harness, x, y);
    harness.hover_at(at);
    harness.run_steps(3);
}

fn click<A>(harness: &mut Harness<'_, Rig<A>>, x: f32, y: f32) {
    hover(harness, x, y);
    let at = point(harness, x, y);
    harness.drag_at(at);
    harness.step();
    harness.drop_at(at);
    harness.run_steps(2);
}

fn rects<A>(harness: &Harness<'_, Rig<A>>) -> Vec<RectShape> {
    let shapes: &[ClippedShape] = &harness.output().shapes;
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) => Some(rect.clone()),
            _ => None,
        })
        .collect()
}

/// The fill of the rect that starts `offset` from the layout's origin.
fn fill_at<A>(harness: &Harness<'_, Rig<A>>, x: f32, y: f32) -> Option<Color32> {
    let min = point(harness, x, y);
    rects(harness)
        .into_iter()
        .rfind(|shape| (shape.rect.min - min).length() < 0.5)
        .map(|shape| shape.fill)
}

const BASE: Color32 = Color32::from_rgb(20, 20, 20);
const LIT: Color32 = Color32::from_rgb(220, 20, 20);

// ---------------------------------------------------------------------- hover

/// ```html
/// <div class="group w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414]"></div>
/// ```
fn hoverable(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "box",
        tw().w_px(100.0).h_px(40.0).bg(BASE).hover(|p| p.bg(LIT)),
        none,
        |_| {},
    );
}

#[test]
fn hover_repaints_the_background() {
    let mut harness = harness((), hoverable);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));

    hover(&mut harness, 10.0, 10.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(LIT));

    hover(&mut harness, 200.0, 200.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));
}

// ---------------------------------------------------------------------- click

#[derive(Default)]
struct Counts {
    first: u32,
    second: u32,
}

/// ```html
/// <div class="w-[100px] h-[40px] bg-[#141414] active:bg-[#dc1414]"></div>
/// ```
fn counter(b: &mut Builder<'_, '_, Counts>, _: &Counts) {
    b.div_id(
        "button",
        tw().w_px(100.0).h_px(40.0).bg(BASE).active(|p| p.bg(LIT)),
        |counts: &mut Counts, ui| {
            if ui.node_response().clicked() {
                counts.first += 1;
            }
        },
        |_| {},
    );
}

#[test]
fn a_click_runs_the_draw_closures_mutation_of_the_application() {
    let mut harness = harness(Counts::default(), counter);
    click(&mut harness, 10.0, 10.0);
    assert_eq!(harness.state().app.first, 1);

    click(&mut harness, 300.0, 200.0);
    assert_eq!(
        harness.state().app.first,
        1,
        "a click elsewhere is not ours"
    );
}

#[test]
fn active_paints_while_the_button_is_held() {
    let mut harness = harness(Counts::default(), counter);
    hover(&mut harness, 10.0, 10.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));

    let at = point(&harness, 10.0, 10.0);
    harness.drag_at(at);
    harness.run_steps(3);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(LIT));

    harness.drop_at(at);
    harness.run_steps(3);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));
}

// ---------------------------------------------------------------------- inert

/// ```html
/// <div inert class="opacity-50">
///   <div class="w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414]"></div>
/// </div>
/// ```
fn guarded(b: &mut Builder<'_, '_, Counts>, wrapper: bullseye::Tw) {
    b.div(wrapper, |b| {
        b.div_id(
            "button",
            tw().w_px(100.0)
                .h_px(40.0)
                .bg(BASE)
                .sense_click()
                .hover(|p| p.bg(LIT)),
            |counts: &mut Counts, ui| {
                if ui.node_response().clicked() {
                    counts.first += 1;
                }
            },
            |_| {},
        );
    });
}

fn live(b: &mut Builder<'_, '_, Counts>, _: &Counts) {
    guarded(b, tw());
}

fn inert(b: &mut Builder<'_, '_, Counts>, _: &Counts) {
    guarded(b, tw().inert());
}

#[test]
fn inert_blocks_clicks_and_hover_below_it() {
    let mut live = harness(Counts::default(), live);
    hover(&mut live, 10.0, 10.0);
    assert_eq!(
        fill_at(&live, 0.0, 0.0),
        Some(LIT),
        "precondition: live nodes hover"
    );
    click(&mut live, 10.0, 10.0);
    assert_eq!(live.state().app.first, 1, "precondition: live nodes click");

    let mut inert = harness(Counts::default(), inert);
    hover(&mut inert, 10.0, 10.0);
    assert_eq!(fill_at(&inert, 0.0, 0.0), Some(BASE));
    click(&mut inert, 10.0, 10.0);
    assert_eq!(inert.state().app.first, 0);
}

// ------------------------------------------------------------------------ z

/// ```html
/// <div class="relative">
///   <div class="absolute left-0 top-0 size-[60px] z-[5]"></div>
///   <div class="absolute left-[20px] top-[20px] size-[60px]"></div>
/// </div>
/// ```
fn overlapping(b: &mut Builder<'_, '_, Counts>, first_z: i16) {
    b.div(tw().relative().size_px(200.0), |b| {
        b.div_id(
            "first",
            tw().absolute()
                .left_px(0.0)
                .top_px(0.0)
                .size_px(60.0)
                .z(first_z)
                .sense_click(),
            |counts: &mut Counts, ui| {
                if ui.node_response().clicked() {
                    counts.first += 1;
                }
            },
            |_| {},
        );
        b.div_id(
            "second",
            tw().absolute()
                .left_px(20.0)
                .top_px(20.0)
                .size_px(60.0)
                .z(0)
                .sense_click(),
            |counts: &mut Counts, ui| {
                if ui.node_response().clicked() {
                    counts.second += 1;
                }
            },
            |_| {},
        );
    });
}

fn tied(b: &mut Builder<'_, '_, Counts>, _: &Counts) {
    overlapping(b, 0);
}

fn first_raised(b: &mut Builder<'_, '_, Counts>, _: &Counts) {
    overlapping(b, 5);
}

#[test]
fn higher_z_wins_among_overlapping_siblings_and_ties_go_to_the_later_one() {
    let first = Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 60.0));
    let second = Rect::from_min_size(pos2(20.0, 20.0), vec2(60.0, 60.0));
    let overlap = first.intersect(second);
    assert!(
        overlap.is_positive() && overlap.contains(pos2(40.0, 40.0)),
        "precondition: the siblings overlap where we click"
    );

    let mut tied = harness(Counts::default(), tied);
    click(&mut tied, 40.0, 40.0);
    assert_eq!(
        (tied.state().app.first, tied.state().app.second),
        (0, 1),
        "equal z: pre-order, so the later sibling is on top"
    );

    let mut raised = harness(Counts::default(), first_raised);
    click(&mut raised, 40.0, 40.0);
    assert_eq!(
        (raised.state().app.first, raised.state().app.second),
        (1, 0),
        "z(5) on the earlier sibling puts it on top"
    );
}

/// ```html
/// <div class="w-[200px] h-[40px] bg-[#141414] hover:bg-[#dc1414]">
///   <button class="w-[50px] h-[40px]"></button>
/// </div>
/// ```
fn row_with_button(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "row",
        tw().w_px(200.0).h_px(40.0).bg(BASE).hover(|p| p.bg(LIT)),
        none,
        |b| {
            b.div_id(
                "button",
                tw().w_px(50.0).h_px(40.0).sense_click(),
                none,
                |_| {},
            );
        },
    );
}

#[test]
fn a_row_stays_hovered_while_the_pointer_is_over_its_button() {
    let mut harness = harness((), row_with_button);
    hover(&mut harness, 150.0, 20.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(LIT), "precondition: row");
    hover(&mut harness, 10.0, 20.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(LIT), "over the button");
    hover(&mut harness, 300.0, 200.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));
}

// ---------------------------------------------------------------- group hover

/// ```html
/// <div class="group flex w-[200px] h-[40px] bg-[#141414]">
///   <button class="w-[50px] h-[40px] opacity-0 group-hover:opacity-100 bg-[#dc1414]"></button>
/// </div>
/// ```
fn revealing(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "row",
        tw().flex()
            .w_px(200.0)
            .h_px(40.0)
            .bg(BASE)
            .group()
            .sense_hover(),
        none,
        |b| {
            b.div_id(
                "action",
                tw().w_px(50.0)
                    .h_px(40.0)
                    .bg(LIT)
                    .opacity(Opacity::TRANSPARENT)
                    .group_hover(|p| p.opacity(Opacity::OPAQUE))
                    .sense_click(),
                none,
                |_| {},
            );
        },
    );
}

/// The action paints only while it is not fully transparent.
fn action_shown(harness: &Harness<'_, Rig<()>>) -> bool {
    rects(harness).iter().any(|shape| shape.fill == LIT)
}

#[test]
fn group_hover_reveals_a_child_while_the_row_or_the_child_is_hovered() {
    let mut harness = harness((), revealing);
    assert!(!action_shown(&harness), "hidden until hovered");

    hover(&mut harness, 150.0, 20.0);
    assert!(action_shown(&harness), "the row is hovered");

    hover(&mut harness, 10.0, 20.0);
    assert!(
        action_shown(&harness),
        "the child is hovered, and the group with it"
    );

    hover(&mut harness, 300.0, 200.0);
    assert!(!action_shown(&harness));
}

// ----------------------------------------------------------------- text colour

/// ```html
/// <div class="group hover:text-[#dc1414]"><p>label</p></div>
/// ```
fn colored_label(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "row",
        tw().w_px(100.0).h_px(40.0).hover(|p| p.text(LIT)),
        none,
        |b| b.text(tw(), "label"),
    );
}

#[test]
fn a_text_colour_variant_reaches_inheriting_descendants() {
    let mut harness = harness((), colored_label);
    let color = |harness: &Harness<'_, Rig<()>>| {
        harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some(text.fallback_color),
                _ => None,
            })
    };
    assert_eq!(color(&harness), Some(Color32::BLACK));
    hover(&mut harness, 5.0, 5.0);
    assert_eq!(color(&harness), Some(LIT));
}

// ------------------------------------------------------------------ transition

/// ```html
/// <div class="w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414] transition duration-500"></div>
/// ```
fn fading(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "box",
        tw().w_px(100.0)
            .h_px(40.0)
            .bg(BASE)
            .hover(|p| p.bg(LIT))
            .transition(Duration::from_millis(500)),
        none,
        |_| {},
    );
}

#[test]
fn a_transition_passes_through_colours_between_base_and_variant() {
    let mut harness = harness((), fading);
    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    harness.step();
    harness.step();
    harness.step();

    let Some(mid) = fill_at(&harness, 0.0, 0.0) else {
        panic!("the box is painted")
    };
    assert!(
        mid.r() > BASE.r() && mid.r() < LIT.r(),
        "mid-transition red {} is between {} and {}",
        mid.r(),
        BASE.r(),
        LIT.r()
    );

    harness.run_steps(20);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(LIT));

    hover(&mut harness, 300.0, 200.0);
    harness.step();
    let Some(back) = fill_at(&harness, 0.0, 0.0) else {
        panic!("the box is painted")
    };
    assert!(back.r() < LIT.r() && back.r() > BASE.r(), "fading back");
}

/// ```html
/// <div class="w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414] transition duration-500 ease-linear"></div>
/// ```
fn fading_linearly(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "box",
        tw().w_px(100.0)
            .h_px(40.0)
            .bg(BASE)
            .hover(|p| p.bg(LIT))
            .transition(Duration::from_millis(500))
            .ease_linear(),
        none,
        |_| {},
    );
}

/// The red channel's progress from `BASE` to `LIT`, sampled after each step
/// of the 500ms transition, starting with the step that sees the hover.
fn red_progress(view: View<()>) -> Vec<f32> {
    let mut harness = harness((), view);
    harness.run_steps(2);
    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    let span = f32::from(LIT.r() - BASE.r());
    (0..10)
        .map(|_| {
            harness.step();
            let Some(fill) = fill_at(&harness, 0.0, 0.0) else {
                panic!("the box is painted")
            };
            f32::from(fill.r() - BASE.r()) / span
        })
        .collect()
}

/// Whether `samples` are `easing` of the `clock`, to a byte of colour.
fn follows(samples: &[f32], clock: &[f32], easing: Easing) -> bool {
    samples.iter().zip(clock).all(|(got, linear)| {
        let want: f32 = easing.ease(Progress::new(*linear)).into();
        (got - want).abs() < 0.01
    })
}

#[test]
fn a_transition_follows_the_default_curve_not_the_linear_clock() {
    let clock = red_progress(fading_linearly);
    let samples = red_progress(fading);
    assert!(
        follows(&clock, &clock, Easing::Linear)
            && clock.iter().any(|t| (0.4..0.6).contains(t))
            && clock.windows(2).all(|pair| pair[0] < pair[1]),
        "precondition: the linear run is a clock passing the midpoint: {clock:?}"
    );
    assert!(
        !follows(&samples, &clock, Easing::Linear),
        "not linear: {samples:?}"
    );
    assert!(
        follows(&samples, &clock, Easing::Default),
        "{samples:?} vs {clock:?}"
    );
}

#[test]
fn a_transition_arrives_exactly_at_the_end() {
    for view in [fading as View<()>, fading_linearly] {
        let mut harness = harness((), view);
        harness.run_steps(2);
        let at = point(&harness, 10.0, 10.0);
        harness.hover_at(at);
        harness.run_steps(14);
        assert_eq!(fill_at(&harness, 0.0, 0.0), Some(LIT));
    }
}

// ---------------------------------------------------------------------- reorder

struct Order(Vec<u32>);

/// ```html
/// <ul>
///   <li class="w-[100px] h-[30px] bg-[#141414] hover:bg-[#dc1414] transition duration-1000"></li>
/// </ul>
/// ```
fn keyed_rows(b: &mut Builder<'_, '_, Order>, order: &Order) {
    for &key in &order.0 {
        b.div_id(
            ("row", key),
            tw().w_px(100.0)
                .h_px(30.0)
                .bg(BASE)
                .hover(|p| p.bg(LIT))
                .transition(Duration::from_millis(1000)),
            none,
            |_| {},
        );
    }
}

#[test]
fn hover_state_follows_a_keyed_row_through_a_reorder() {
    let mut harness = harness(Order(vec![1, 2]), keyed_rows);
    harness.run_steps(2);
    hover(&mut harness, 10.0, 10.0);
    harness.run_steps(40);
    assert_eq!(
        fill_at(&harness, 0.0, 0.0),
        Some(LIT),
        "precondition: row 1 is fully hovered at the top"
    );
    assert_eq!(fill_at(&harness, 0.0, 30.0), Some(BASE));

    harness.state_mut().app.0.reverse();
    harness.step();

    let moved = fill_at(&harness, 0.0, 30.0).map(|c| c.r());
    let arrived = fill_at(&harness, 0.0, 0.0).map(|c| c.r());
    assert!(
        moved.is_some_and(|r| r > 200),
        "row 1 took its red to its new place: {moved:?}"
    );
    assert!(
        arrived.is_some_and(|r| r < 100),
        "row 2 did not inherit the red by position: {arrived:?}"
    );
}

// ---------------------------------------------------------------- build errors

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only identified nodes")]
fn variants_on_an_anonymous_node_are_a_build_error() {
    let mut layout = LayoutState::new();
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    layout.build::<()>(&ctx, |b| {
        b.div(tw().hover(|p| p.bg(Color32::RED)), |_| {});
    });
}

/// ```html
/// <div class="group w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414]">
///   <div class="w-[10px] h-[10px] bg-[#141414] group-hover:bg-[#dc1414]"></div>
/// </div>
/// ```
fn anonymous_child(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "parent",
        tw().w_px(100.0).h_px(40.0).bg(BASE).group().sense_hover(),
        none,
        |b| {
            b.rule(
                tw().w_px(10.0)
                    .h_px(10.0)
                    .bg(BASE)
                    .group_hover(|p| p.bg(LIT)),
            );
        },
    );
}

#[test]
fn an_anonymous_child_follows_its_groups_hover() {
    let mut harness = harness((), anonymous_child);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));
    hover(&mut harness, 80.0, 30.0);
    assert_eq!(
        fill_at(&harness, 0.0, 0.0),
        Some(LIT),
        "the child lights while the pointer is elsewhere in the group"
    );
    hover(&mut harness, 300.0, 200.0);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only identified nodes")]
fn a_transition_on_an_anonymous_node_is_a_build_error() {
    let mut layout = LayoutState::new();
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    layout.build::<()>(&ctx, |b| {
        b.div(
            tw().group_hover(|p| p.bg(Color32::RED))
                .transition(Duration::from_millis(100)),
            |_| {},
        );
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only identified nodes")]
fn a_sense_on_anonymous_text_is_a_build_error() {
    let mut layout = LayoutState::new();
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    layout.build::<()>(&ctx, |b| b.text(tw().sense_click(), "nope"));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "group must sense")]
fn a_group_that_senses_nothing_is_a_build_error() {
    let mut layout = LayoutState::new();
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    layout.build::<()>(&ctx, |b| b.div_id("g", tw().group(), none, |_| {}));
}
