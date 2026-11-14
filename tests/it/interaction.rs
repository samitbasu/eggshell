//! Interaction through the real path: pointer events dispatched by
//! `egui_kittest::Harness` into egui's hit test, the library's one
//! `ui.interact` per sensing node, variant painting and draw closures.

use core::time::Duration;

use eggshell_ui::{Builder, Easing, LayoutState, Opacity, Progress, TextSpec, TextStyle, tw};
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

fn harness<A>(app: A, view: View<A>) -> Harness<'static, Rig<A>> {
    harness_stepping(app, view, STEP)
}

fn harness_stepping<A>(app: A, view: View<A>, step_dt: f32) -> Harness<'static, Rig<A>> {
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
        .with_step_dt(step_dt)
        .build_ui_state(
            |ui, rig: &mut Rig<A>| {
                crate::support::full_motion(ui.ctx());
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
        |_| {},
    )
    .on_click(|counts: &mut Counts| {
        counts.first += 1;
    });
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
fn guarded(b: &mut Builder<'_, '_, Counts>, wrapper: eggshell_ui::Tw) {
    b.div(wrapper, |b| {
        b.div_id(
            "button",
            tw().w_px(100.0)
                .h_px(40.0)
                .bg(BASE)
                .sense_click()
                .hover(|p| p.bg(LIT)),
            |_| {},
        )
        .on_click(|counts: &mut Counts| {
            counts.first += 1;
        });
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
            |_| {},
        )
        .on_click(|counts: &mut Counts| {
            counts.first += 1;
        });
        b.div_id(
            "second",
            tw().absolute()
                .left_px(20.0)
                .top_px(20.0)
                .size_px(60.0)
                .z(0)
                .sense_click(),
            |_| {},
        )
        .on_click(|counts: &mut Counts| {
            counts.second += 1;
        });
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
        |b| {
            b.div_id("button", tw().w_px(50.0).h_px(40.0).sense_click(), |_| {});
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
        |b| {
            b.div_id(
                "action",
                tw().w_px(50.0)
                    .h_px(40.0)
                    .bg(LIT)
                    .opacity(Opacity::TRANSPARENT)
                    .group_hover(|p| p.opacity(Opacity::OPAQUE))
                    .sense_click(),
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

// ------------------------------------------------------------ transition clock

/// ```html
/// <div class="w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414] transition duration-150"></div>
/// ```
fn quick(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "box",
        tw().w_px(100.0)
            .h_px(40.0)
            .bg(BASE)
            .hover(|p| p.bg(LIT))
            .transition(Duration::from_millis(150)),
        |_| {},
    );
}

/// The red channel's progress from `BASE` to `LIT` as drawn.
fn red<A>(harness: &Harness<'_, Rig<A>>) -> f32 {
    let Some(fill) = fill_at(harness, 0.0, 0.0) else {
        panic!("the box is painted")
    };
    f32::from(fill.r() - BASE.r()) / f32::from(LIT.r() - BASE.r())
}

#[test]
fn a_transition_lasts_its_duration_however_slow_the_frames() {
    let mut harness = harness_stepping((), quick, 0.2);
    harness.run_steps(2);
    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    let seen: Vec<f32> = (0..3)
        .map(|_| {
            harness.step();
            red(&harness)
        })
        .collect();
    assert!(
        seen.iter().all(|r| *r == 0.0 || *r == 1.0),
        "200ms frames skip the middle of a 150ms transition: {seen:?}"
    );
    assert_eq!(
        seen.iter().position(|r| *r == 1.0),
        seen.iter().position(|r| *r == 0.0).map(|p| p + 1),
        "it arrives on the first frame 200ms after the flip: {seen:?}"
    );
}

#[test]
fn fast_frames_pass_through_intermediate_values_to_the_end_on_time() {
    let mut harness = harness_stepping((), quick, 0.016);
    harness.run_steps(2);
    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    let seen: Vec<f32> = (0..16)
        .map(|_| {
            harness.step();
            red(&harness)
        })
        .collect();
    assert!(seen.iter().filter(|r| **r > 0.0 && **r < 1.0).count() >= 5);
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{seen:?}");
    assert_eq!(seen[0], 0.0, "precondition: the first frame sees the flip");
    assert!(seen[8] < 1.0, "not early, 128ms in: {seen:?}");
    assert_eq!(seen[10], 1.0, "arrived by 160ms in: {seen:?}");
}

#[test]
fn a_reversal_mid_flight_starts_from_the_current_value_and_is_shortened() {
    let mut harness = harness((), fading_linearly);
    harness.run_steps(2);
    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    harness.run_steps(6);
    let halfway = red(&harness);
    assert!(
        (0.3..0.7).contains(&halfway),
        "precondition: about halfway through the 500ms transition: {halfway}"
    );

    hover(&mut harness, 300.0, 200.0);
    let mut back = vec![halfway];
    while red(&harness) > 0.0 && back.len() < 20 {
        back.push(red(&harness));
        harness.step();
    }
    assert!(
        back.windows(2).all(|pair| pair[0] >= pair[1]),
        "monotonic from the current value: {back:?}"
    );
    assert!(
        back.iter().all(|r| *r <= halfway + 0.2),
        "it did not jump to the end first: {back:?}"
    );
    let full = 500.0 / 1000.0 / STEP;
    assert!(
        (back.len() as f32) < full * 0.8,
        "shortened in proportion to the distance left: {} frames of {full}",
        back.len()
    );
}

#[test]
fn repaints_are_requested_only_while_a_transition_is_in_flight() {
    let mut harness = harness((), fading_linearly);
    harness.run_steps(4);
    harness.step();
    assert!(!harness.ctx.has_requested_repaint(), "settled");

    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    harness.run_steps(3);
    assert!(
        red(&harness) > 0.0 && red(&harness) < 1.0,
        "precondition: in flight"
    );
    assert!(harness.ctx.has_requested_repaint(), "in flight");

    harness.run_steps(14);
    assert_eq!(red(&harness), 1.0, "precondition: arrived");
    harness.step();
    assert!(!harness.ctx.has_requested_repaint(), "settled again");
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

#[test]
fn a_transition_in_flight_follows_a_keyed_row_through_a_reorder() {
    let mut harness = harness(Order(vec![1, 2]), keyed_rows);
    harness.run_steps(2);
    let at = point(&harness, 10.0, 10.0);
    harness.hover_at(at);
    harness.run_steps(6);
    let before = red_of(&harness, 0.0);
    assert!(
        (0.1..0.9).contains(&before),
        "precondition: row 1 is mid-transition: {before}"
    );
    assert_eq!(
        red_of(&harness, 30.0),
        0.0,
        "precondition: row 2 is at rest"
    );

    harness.state_mut().app.0.reverse();
    harness.step();

    let moved = red_of(&harness, 30.0);
    let arrived = red_of(&harness, 0.0);
    assert!(
        (moved - before).abs() < 0.25,
        "row 1 took its progress to its new place: {moved} from {before}"
    );
    assert!(arrived < 0.25, "row 2 did not inherit it: {arrived}");
}

fn red_of<A>(harness: &Harness<'_, Rig<A>>, y: f32) -> f32 {
    let Some(fill) = fill_at(harness, 0.0, y) else {
        panic!("the row is painted")
    };
    f32::from(fill.r() - BASE.r()) / f32::from(LIT.r() - BASE.r())
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
    layout.build::<()>(&ctx, |b| {
        b.div_id("g", tw().group(), |_| {});
    });
}

// -------------------------------------------------------------- node handles

#[derive(Default)]
struct Handled {
    clicks: u32,
    hovered: bool,
    content: Option<Rect>,
    order: Vec<&'static str>,
}

fn span_with_click(b: &mut Builder<'_, '_, Handled>, _: &Handled) {
    b.span("up", tw().sense_click().w_px(100.0).h_px(40.0), "Up")
        .on_click(|app: &mut Handled| app.clicks += 1);
}

#[test]
fn a_div_id_without_a_handler_builds_and_draws() {
    let mut harness = harness((), hoverable);
    harness.run_steps(2);
    assert_eq!(fill_at(&harness, 0.0, 0.0), Some(BASE));
}

#[test]
fn on_click_on_a_span_fires_once_per_click() {
    let mut harness = harness(Handled::default(), span_with_click);
    click(&mut harness, 10.0, 10.0);
    assert_eq!(harness.state().app.clicks, 1);

    click(&mut harness, 10.0, 10.0);
    assert_eq!(harness.state().app.clicks, 2);

    click(&mut harness, 300.0, 200.0);
    assert_eq!(
        harness.state().app.clicks,
        2,
        "a click elsewhere is not ours"
    );
}

#[test]
fn on_response_sees_hover() {
    let mut harness = harness(Handled::default(), |b, _| {
        b.div_id("box", tw().sense_hover().w_px(100.0).h_px(40.0), |_| {})
            .on_response(|app: &mut Handled, response| app.hovered = response.hovered());
    });
    assert!(!harness.state().app.hovered);

    hover(&mut harness, 10.0, 10.0);
    assert!(harness.state().app.hovered);

    hover(&mut harness, 300.0, 200.0);
    assert!(!harness.state().app.hovered);
}

#[test]
fn draw_gets_the_nodes_own_ui_limited_to_its_content_rect() {
    let mut harness = harness(Handled::default(), |b, _| {
        b.div_id("box", tw().p(2.0).w_px(100.0).h_px(40.0), |_| {})
            .draw(|app: &mut Handled, ui| app.content = Some(ui.max_rect()));
    });
    harness.run_steps(2);
    let origin = harness.state().origin;
    assert_eq!(
        harness.state().app.content,
        Some(Rect::from_min_size(
            origin + vec2(8.0, 8.0),
            vec2(84.0, 24.0)
        ))
    );
}

#[test]
fn handlers_compose_in_call_order() {
    let mut harness = harness(Handled::default(), |b, _| {
        b.span("up", tw().sense_click().w_px(100.0).h_px(40.0), "Up")
            .draw(|app: &mut Handled, _| app.order.push("draw"))
            .on_response(|app: &mut Handled, _| app.order.push("response"))
            .on_click(|app: &mut Handled| app.order.push("click"));
    });
    harness.run_steps(2);
    let order = &harness.state().app.order;
    assert!(!order.is_empty());
    assert!(
        order.chunks(2).all(|frame| frame == ["draw", "response"]),
        "{order:?}"
    );

    click(&mut harness, 10.0, 10.0);
    assert!(harness.state().app.order.contains(&"click"));
    let order = &harness.state().app.order;
    let at = order.iter().position(|&step| step == "click").unwrap();
    assert_eq!(order[at - 1], "response", "{order:?}");
    assert_eq!(order[at - 2], "draw", "{order:?}");
}
