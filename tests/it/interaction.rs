//! Interaction through the real path: pointer events dispatched by
//! `egui_kittest::Harness` into egui's hit test, the library's one
//! `ui.interact` per sensing node, variant painting and draw closures.

use core::time::Duration;

use eggshell_ui::testing::{After, Driver};
use eggshell_ui::{Builder, Easing, Opacity, Problem, Progress, TextSpec, TextStyle, Weight, tw};
use egui::epaint::{ClippedShape, RectShape, Shape};
use egui::{Color32, Event, FontFamily, Pos2, Rect, pos2, vec2};

type View<A> = for<'b, 'f> fn(&mut Builder<'b, 'f, A>, &A);

const STEP: Duration = Duration::from_millis(50);

fn harness<A: 'static>(app: A, view: View<A>) -> Driver<A> {
    harness_stepping(app, view, STEP)
}

fn harness_stepping<A: 'static>(app: A, view: View<A>, step: Duration) -> Driver<A> {
    Driver::new(app, view)
        .size(vec2(400.0, 300.0))
        .base_text(TextStyle::new(
            TextSpec {
                size: 16.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            Color32::BLACK,
        ))
        .step(step)
        .after(After::Frames(0))
        .settle_within(step * 4)
        .build()
}

/// Where the layout starts: kittest's frame sets the window in by a margin.
fn origin<A>(d: &Driver<A>) -> Pos2 {
    d.frame()
        .nodes()
        .next()
        .map_or(Pos2::ZERO, |root| root.rect().min)
}

fn point<A>(d: &Driver<A>, x: f32, y: f32) -> Pos2 {
    origin(d) + vec2(x, y)
}

fn hover<A>(d: &mut Driver<A>, x: f32, y: f32) {
    let at = point(d, x, y);
    d.hover(at);
}

/// A click whose release takes the pointer out of the window, as kittest's
/// `drop_at` does.
fn click<A>(d: &mut Driver<A>, x: f32, y: f32) {
    let at = point(d, x, y);
    d.click(at).event(Event::PointerGone).frames(1);
}

fn rects<A>(d: &Driver<A>) -> Vec<RectShape> {
    let shapes: &[ClippedShape] = &d.harness().output().shapes;
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) => Some(rect.clone()),
            _ => None,
        })
        .collect()
}

/// The fill of the rect that starts `offset` from the layout's origin.
fn fill_at<A>(d: &Driver<A>, x: f32, y: f32) -> Option<Color32> {
    let min = point(d, x, y);
    rects(d)
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
    let mut d = harness((), hoverable);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));

    hover(&mut d, 10.0, 10.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT));

    hover(&mut d, 200.0, 200.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
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
    let mut d = harness(Counts::default(), counter);
    click(&mut d, 10.0, 10.0);
    assert_eq!(d.app().first, 1);

    click(&mut d, 300.0, 200.0);
    assert_eq!(d.app().first, 1, "a click elsewhere is not ours");
}

#[test]
fn active_paints_while_the_button_is_held() {
    let mut d = harness(Counts::default(), counter);
    hover(&mut d, 10.0, 10.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));

    let at = point(&d, 10.0, 10.0);
    d.event(Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    })
    .frames(2);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT));

    d.event(Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    })
    .event(Event::PointerGone)
    .frames(1);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
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
    assert_eq!(live.app().first, 1, "precondition: live nodes click");

    let mut inert = harness(Counts::default(), inert);
    hover(&mut inert, 10.0, 10.0);
    assert_eq!(fill_at(&inert, 0.0, 0.0), Some(BASE));
    click(&mut inert, 10.0, 10.0);
    assert_eq!(inert.app().first, 0);
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
        (tied.app().first, tied.app().second),
        (0, 1),
        "equal z: pre-order, so the later sibling is on top"
    );

    let mut raised = harness(Counts::default(), first_raised);
    click(&mut raised, 40.0, 40.0);
    assert_eq!(
        (raised.app().first, raised.app().second),
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
    let mut d = harness((), row_with_button);
    hover(&mut d, 150.0, 20.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT), "precondition: row");
    hover(&mut d, 10.0, 20.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT), "over the button");
    hover(&mut d, 300.0, 200.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
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
fn action_shown(d: &Driver<()>) -> bool {
    rects(d).iter().any(|shape| shape.fill == LIT)
}

#[test]
fn group_hover_reveals_a_child_while_the_row_or_the_child_is_hovered() {
    let mut d = harness((), revealing);
    assert!(!action_shown(&d), "hidden until hovered");

    hover(&mut d, 150.0, 20.0);
    assert!(action_shown(&d), "the row is hovered");

    hover(&mut d, 10.0, 20.0);
    assert!(
        action_shown(&d),
        "the child is hovered, and the group with it"
    );

    hover(&mut d, 300.0, 200.0);
    assert!(!action_shown(&d));
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
    let mut d = harness((), colored_label);
    let color = |d: &Driver<()>| {
        d.harness()
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some(text.fallback_color),
                _ => None,
            })
    };
    assert_eq!(color(&d), Some(Color32::BLACK));
    hover(&mut d, 5.0, 5.0);
    assert_eq!(color(&d), Some(LIT));
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
    let mut d = harness((), fading);
    let at = point(&d, 10.0, 10.0);
    d.hover(at);

    let Some(mid) = fill_at(&d, 0.0, 0.0) else {
        panic!("the box is painted")
    };
    assert!(
        mid.r() > BASE.r() && mid.r() < LIT.r(),
        "mid-transition red {} is between {} and {}",
        mid.r(),
        BASE.r(),
        LIT.r()
    );

    d.frames(20);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT));

    hover(&mut d, 300.0, 200.0);
    d.frames(1);
    let Some(back) = fill_at(&d, 0.0, 0.0) else {
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
    let mut d = harness((), view);
    d.frames(2);
    let at = point(&d, 10.0, 10.0);
    d.harness().hover_at(at);
    let span = f32::from(LIT.r() - BASE.r());
    (0..10)
        .map(|_| {
            d.frames(1);
            let Some(fill) = fill_at(&d, 0.0, 0.0) else {
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
        let mut d = harness((), view);
        d.frames(2);
        let at = point(&d, 10.0, 10.0);
        d.carry_to(at).frames(13);
        assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT));
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
fn red<A>(d: &Driver<A>) -> f32 {
    let Some(fill) = fill_at(d, 0.0, 0.0) else {
        panic!("the box is painted")
    };
    f32::from(fill.r() - BASE.r()) / f32::from(LIT.r() - BASE.r())
}

#[test]
fn a_transition_lasts_its_duration_however_slow_the_frames() {
    let mut d = harness_stepping((), quick, Duration::from_millis(200));
    d.frames(2);
    let at = point(&d, 10.0, 10.0);
    d.harness().hover_at(at);
    let seen: Vec<f32> = (0..3)
        .map(|_| {
            d.frames(1);
            red(&d)
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
    let mut d = harness_stepping((), quick, Duration::from_millis(16));
    d.frames(2);
    let at = point(&d, 10.0, 10.0);
    d.harness().hover_at(at);
    let seen: Vec<f32> = (0..16)
        .map(|_| {
            d.frames(1);
            red(&d)
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
    let mut d = harness((), fading_linearly);
    d.frames(2);
    let at = point(&d, 10.0, 10.0);
    d.carry_to(at).frames(5);
    let halfway = red(&d);
    assert!(
        (0.3..0.7).contains(&halfway),
        "precondition: about halfway through the 500ms transition: {halfway}"
    );

    hover(&mut d, 300.0, 200.0);
    let mut back = vec![halfway];
    while red(&d) > 0.0 && back.len() < 20 {
        back.push(red(&d));
        d.frames(1);
    }
    assert!(
        back.windows(2).all(|pair| pair[0] >= pair[1]),
        "monotonic from the current value: {back:?}"
    );
    assert!(
        back.iter().all(|r| *r <= halfway + 0.2),
        "it did not jump to the end first: {back:?}"
    );
    let full = 500.0 / 1000.0 / STEP.as_secs_f32();
    assert!(
        (back.len() as f32) < full * 0.8,
        "shortened in proportion to the distance left: {} frames of {full}",
        back.len()
    );
}

#[test]
fn repaints_are_requested_only_while_a_transition_is_in_flight() {
    let mut d = harness((), fading_linearly);
    d.frames(4);
    d.frames(1);
    assert!(!d.ctx().has_requested_repaint(), "settled");

    let at = point(&d, 10.0, 10.0);
    d.hover(at);
    assert!(red(&d) > 0.0 && red(&d) < 1.0, "precondition: in flight");
    assert!(d.ctx().has_requested_repaint(), "in flight");

    d.frames(14);
    assert_eq!(red(&d), 1.0, "precondition: arrived");
    d.frames(1);
    assert!(!d.ctx().has_requested_repaint(), "settled again");
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
    let mut d = harness(Order(vec![1, 2]), keyed_rows);
    d.frames(2);
    hover(&mut d, 10.0, 10.0);
    d.frames(40);
    assert_eq!(
        fill_at(&d, 0.0, 0.0),
        Some(LIT),
        "precondition: row 1 is fully hovered at the top"
    );
    assert_eq!(fill_at(&d, 0.0, 30.0), Some(BASE));

    d.app_mut().0.reverse();
    d.frames(1);

    let moved = fill_at(&d, 0.0, 30.0).map(|c| c.r());
    let arrived = fill_at(&d, 0.0, 0.0).map(|c| c.r());
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
    let mut d = harness(Order(vec![1, 2]), keyed_rows);
    d.frames(2);
    let at = point(&d, 10.0, 10.0);
    d.carry_to(at).frames(5);
    let before = red_of(&d, 0.0);
    assert!(
        (0.1..0.9).contains(&before),
        "precondition: row 1 is mid-transition: {before}"
    );
    assert_eq!(red_of(&d, 30.0), 0.0, "precondition: row 2 is at rest");

    d.app_mut().0.reverse();
    d.frames(1);

    let moved = red_of(&d, 30.0);
    let arrived = red_of(&d, 0.0);
    assert!(
        (moved - before).abs() < 0.25,
        "row 1 took its progress to its new place: {moved} from {before}"
    );
    assert!(arrived < 0.25, "row 2 did not inherit it: {arrived}");
}

fn red_of<A>(d: &Driver<A>, y: f32) -> f32 {
    let Some(fill) = fill_at(d, 0.0, y) else {
        panic!("the row is painted")
    };
    f32::from(fill.r() - BASE.r()) / f32::from(LIT.r() - BASE.r())
}

// ---------------------------------------------------------------- build errors

/// ```html
/// <div class="w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414]"></div>
/// ```
/// on a node with no key, which cannot carry a hover look.
fn anonymous_hoverable(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(
        tw().w_px(100.0)
            .h_px(40.0)
            .bg(BASE)
            .hover(|p| p.bg(LIT))
            .sense_click()
            .transition(Duration::from_millis(100)),
        |_| {},
    );
}

#[test]
fn variants_on_an_anonymous_node_are_ignored_and_reported() {
    let mut d = harness((), anonymous_hoverable);
    let report = d.expect_reported(Problem::NeedsIdentity);
    assert!(
        report.message.contains("only identified nodes"),
        "{}",
        report.message
    );
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
    hover(&mut d, 10.0, 10.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
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
    let mut d = harness((), anonymous_child);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
    hover(&mut d, 80.0, 30.0);
    assert_eq!(
        fill_at(&d, 0.0, 0.0),
        Some(LIT),
        "the child lights while the pointer is elsewhere in the group"
    );
    hover(&mut d, 300.0, 200.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
}

/// An anonymous child that keeps its `group_hover` once its transition is
/// dropped.
fn anonymous_transition(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "parent",
        tw().w_px(100.0).h_px(40.0).bg(BASE).group().sense_hover(),
        |b| {
            b.div(
                tw().w_px(10.0)
                    .h_px(10.0)
                    .bg(BASE)
                    .group_hover(|p| p.bg(LIT))
                    .transition(Duration::from_millis(100)),
                |_| {},
            );
        },
    );
}

#[test]
fn a_transition_on_an_anonymous_node_is_dropped_and_reported() {
    let mut d = harness((), anonymous_transition);
    d.expect_reported(Problem::NeedsIdentity);
    hover(&mut d, 80.0, 30.0);
    assert_eq!(
        fill_at(&d, 0.0, 0.0),
        Some(LIT),
        "the group_hover look is kept and shows at once"
    );
}

#[test]
fn a_sense_on_anonymous_text_is_dropped_and_reported() {
    let mut d = harness((), |b, _| b.text(tw().sense_click(), "nope"));
    d.expect_reported(Problem::NeedsIdentity);
    hover(&mut d, 5.0, 5.0);
}

/// ```html
/// <div class="group w-[100px] h-[40px] bg-[#141414]">
///   <div class="w-[10px] h-[10px] bg-[#141414] group-hover:bg-[#dc1414]"></div>
/// </div>
/// ```
/// with a group that is not told to sense hover.
fn unsensing_group(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "parent",
        tw().w_px(100.0).h_px(40.0).bg(BASE).group(),
        |b| {
            b.div_id(
                "child",
                tw().w_px(10.0)
                    .h_px(10.0)
                    .bg(BASE)
                    .group_hover(|p| p.bg(LIT)),
                |_| {},
            );
        },
    );
}

#[test]
fn a_group_that_senses_nothing_senses_hover_and_is_reported() {
    let mut d = harness((), unsensing_group);
    d.expect_reported(Problem::GroupSensesNothing);
    hover(&mut d, 80.0, 30.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(LIT));
}

/// A `group_hover` with no group above it.
fn orphan_group_hover(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "orphan",
        tw().w_px(100.0)
            .h_px(40.0)
            .bg(BASE)
            .group_hover(|p| p.bg(LIT))
            .sense_hover(),
        |_| {},
    );
}

#[test]
fn a_group_hover_without_a_group_never_shows_and_is_reported() {
    let mut d = harness((), orphan_group_hover);
    d.expect_reported(Problem::GroupHoverWithoutGroup);
    hover(&mut d, 10.0, 10.0);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
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
    let mut d = harness((), hoverable);
    d.frames(2);
    assert_eq!(fill_at(&d, 0.0, 0.0), Some(BASE));
}

#[test]
fn on_click_on_a_span_fires_once_per_click() {
    let mut d = harness(Handled::default(), span_with_click);
    click(&mut d, 10.0, 10.0);
    assert_eq!(d.app().clicks, 1);

    click(&mut d, 10.0, 10.0);
    assert_eq!(d.app().clicks, 2);

    click(&mut d, 300.0, 200.0);
    assert_eq!(d.app().clicks, 2, "a click elsewhere is not ours");
}

#[test]
fn on_response_sees_hover() {
    let mut d = harness(Handled::default(), |b, _| {
        b.div_id("box", tw().sense_hover().w_px(100.0).h_px(40.0), |_| {})
            .on_response(|app: &mut Handled, response| app.hovered = response.hovered());
    });
    assert!(!d.app().hovered);

    hover(&mut d, 10.0, 10.0);
    assert!(d.app().hovered);

    hover(&mut d, 300.0, 200.0);
    assert!(!d.app().hovered);
}

#[test]
fn a_custom_nodes_on_response_sees_its_hover_and_click_before_its_ui_runs() {
    let mut d = harness(Handled::default(), |b, _| {
        b.custom("plot", tw().sense_click().w_px(100.0).h_px(40.0))
            .on_response(|app: &mut Handled, response| {
                app.hovered = response.hovered();
                app.clicks += u32::from(response.clicked());
                app.order.push("response");
            })
            .ui(|app: &mut Handled, _| app.order.push("ui"));
    });
    d.frames(2);
    let order = &d.app().order;
    assert!(!order.is_empty(), "precondition: frames were drawn");
    assert!(
        order.chunks(2).all(|frame| frame == ["response", "ui"]),
        "{order:?}"
    );
    assert!(!d.app().hovered);

    hover(&mut d, 10.0, 10.0);
    assert!(d.app().hovered);
    assert_eq!(d.app().clicks, 0);

    click(&mut d, 10.0, 10.0);
    assert_eq!(d.app().clicks, 1);

    click(&mut d, 300.0, 200.0);
    assert_eq!(d.app().clicks, 1, "a click elsewhere is not its");
    assert!(!d.app().hovered);
}

#[test]
fn draw_gets_the_nodes_own_ui_limited_to_its_content_rect() {
    let mut d = harness(Handled::default(), |b, _| {
        b.div_id("box", tw().p(2.0).w_px(100.0).h_px(40.0), |_| {})
            .draw(|app: &mut Handled, ui| app.content = Some(ui.max_rect()));
    });
    d.frames(2);
    let origin = origin(&d);
    assert_eq!(
        d.app().content,
        Some(Rect::from_min_size(
            origin + vec2(8.0, 8.0),
            vec2(84.0, 24.0)
        ))
    );
}

#[test]
fn handlers_compose_in_call_order() {
    let mut d = harness(Handled::default(), |b, _| {
        b.span("up", tw().sense_click().w_px(100.0).h_px(40.0), "Up")
            .draw(|app: &mut Handled, _| app.order.push("draw"))
            .on_response(|app: &mut Handled, _| app.order.push("response"))
            .on_click(|app: &mut Handled| app.order.push("click"));
    });
    d.frames(2);
    let order = &d.app().order;
    assert!(!order.is_empty());
    assert!(
        order.chunks(2).all(|frame| frame == ["draw", "response"]),
        "{order:?}"
    );

    click(&mut d, 10.0, 10.0);
    assert!(d.app().order.contains(&"click"));
    let order = &d.app().order;
    let at = order.iter().position(|&step| step == "click").unwrap();
    assert_eq!(order[at - 1], "response", "{order:?}");
    assert_eq!(order[at - 2], "draw", "{order:?}");
}

#[test]
fn a_live_node_is_announced_as_a_live_region() {
    use egui::accesskit::{Live, Role};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    let harness = crate::support::harness(
        vec2(200.0, 100.0),
        TextStyle::new(
            TextSpec {
                size: 16.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            Color32::BLACK,
        ),
        (),
        |b, _| {
            b.div_id("status", tw().sense_hover().w_px(100.0).h_px(40.0), |s| {
                s.text(tw(), "Saved");
            })
            .role(egui::Role::Status)
            .named("Status")
            .live(Live::Polite);
            b.div_id("quiet", tw().sense_hover().w_px(100.0).h_px(40.0), |_| {})
                .role(egui::Role::Status)
                .named("Quiet");
        },
    );
    let live = |name: &str| {
        harness
            .get_by_role_and_label(Role::Status, name)
            .accesskit_node()
            .data()
            .live()
    };
    assert_eq!(live("Status"), Some(Live::Polite));
    assert_eq!(live("Quiet"), None);
}
