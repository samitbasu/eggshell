//! `OverlayChain::hover_anchored`: an overlay hover intent opens and closes,
//! as a hover card, through the full draw path (pointer, focus, dismissal,
//! tooltips and Tab).

use core::time::Duration;

use eggshell_ui::overlay::{Distance, HoverIntent, Placement};
use eggshell_ui::{Builder, Problem, Style, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Event, Key, PointerButton, Pos2, Rect, Theme, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, hover_over, rect_of};

const OPEN: Duration = Duration::from_millis(700);
const CLOSE: Duration = Duration::from_millis(300);
const TOOLTIP: Duration = Duration::from_millis(500);
/// Frames are this far apart.
const STEP: f32 = 0.05;

const HANDLE: &str = "/\"around\"/\"trigger\"/\"handle\"";
const AROUND: &str = "/\"around\"";
const FOLLOW: &str = "/\"around\"/\"card\"/\"Box\"/\"follow\"";

#[derive(Default)]
struct App {
    follows: u32,
    clicks: u32,
    /// A second card is declared inside the first's content.
    nested: bool,
    /// The page has a button after the trigger.
    after: bool,
}

/// Motion off: a card that closes is gone in the same frame.
fn prepare(ctx: &egui::Context, _: &App) {
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    let mut style = (*Style::of(ctx)).clone();
    style.tooltip.delay = TOOLTIP;
    style.install(ctx);
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <div title="Around tip" class="flex p-6">
///     <span class="flex"><button title="Inner tip">@nextjs</button></span>
///   </div>
///   <button>After</button>
/// </div>
/// <!-- the card: 4 px below the trigger -->
/// <div class="w-40 p-4 border"><p>The React Framework</p><button title="Follow tip">Follow</button></div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.div_id("around", tw().flex().p(6.0), |b| {
            b.div_id("trigger", tw().flex(), |t| {
                t.button("handle", "@nextjs")
                    .tooltip("Inner tip")
                    .on_click(|app: &mut App| app.clicks += 1);
            });
            let trigger = b.id_of("trigger");
            let intent = HoverIntent::new(OPEN, CLOSE).description("The React Framework");
            b.overlay("card")
                .hover_anchored(trigger, intent)
                .placement(Placement::below().gap(Distance::new(4.0)))
                .children(|c| {
                    c.div_id("Box", tw().w_px(160.0).p(4.0).border(1.0), |c| {
                        c.text(tw(), "The React Framework");
                        c.button("follow", "Follow")
                            .tooltip("Follow tip")
                            .on_click(|app: &mut App| app.follows += 1);
                        if app.nested {
                            c.div_id("inner-trigger", tw().flex(), |t| {
                                t.text(tw(), "@vercel");
                            });
                            let inner = c.id_of("inner-trigger");
                            c.overlay("inner")
                                .hover_anchored(inner, HoverIntent::new(OPEN, CLOSE))
                                .children(|c| c.text(tw(), "Inner card"));
                        }
                    });
                });
        })
        .tooltip("Around tip");
        if app.after {
            b.button("after", "After").on_click(|_: &mut App| {});
        }
    });
}

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness_scaled_stepped(
        vec2(400.0, 400.0),
        STEP,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
        prepare,
    );
    harness.run_steps(4);
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(App::default())
}

fn wait(harness: &mut Harness<'_, Rig<App>>, time: Duration) {
    harness.run_steps((time.as_secs_f32() / STEP).ceil() as usize);
}

fn card_box(harness: &Harness<'_, Rig<App>>) -> Option<Rect> {
    harness
        .state()
        .nodes
        .iter()
        .find(|(path, _)| path == "/\"around\"/\"card\"/\"Box\"")
        .map(|(_, rect)| *rect)
}

fn is_open(harness: &Harness<'_, Rig<App>>) -> bool {
    card_box(harness).is_some()
}

fn tooltip_shown(harness: &Harness<'_, Rig<App>>) -> bool {
    harness
        .state()
        .nodes
        .iter()
        .any(|(path, _)| path.contains("Tooltip("))
}

/// Rests the pointer on the trigger until the card opens.
fn open_by_pointer(harness: &mut Harness<'_, Rig<App>>) {
    let at = rect_of(harness, HANDLE).center();
    hover_over(harness, at);
    wait(harness, OPEN + Duration::from_millis(100));
    assert!(is_open(harness), "precondition: the card opened");
}

/// A point on the page in neither the trigger, the area around it nor the
/// card.
fn away(harness: &Harness<'_, Rig<App>>) -> Pos2 {
    let at = pos2(390.0, 390.0);
    for rect in [rect_of(harness, AROUND)]
        .into_iter()
        .chain(card_box(harness))
    {
        assert!(!rect.contains(at), "precondition: {at:?} is off {rect:?}");
    }
    at
}

fn button(harness: &mut Harness<'_, Rig<App>>, at: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();
}

#[test]
fn the_pointer_resting_on_the_trigger_opens_it_after_the_open_delay() {
    let mut harness = harness();
    let at = rect_of(&harness, HANDLE).center();
    hover_over(&mut harness, at);
    wait(&mut harness, OPEN - Duration::from_millis(200));
    assert!(!is_open(&harness), "not before the delay");
    wait(&mut harness, Duration::from_millis(300));
    assert!(is_open(&harness));
    assert!(
        card_box(&harness).is_some_and(|card| card.min.y >= rect_of(&harness, HANDLE).max.y),
        "below the trigger"
    );
}

#[test]
fn crossing_the_gap_into_the_card_keeps_it_open() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let trigger = rect_of(&harness, "/\"around\"/\"trigger\"");
    let card = card_box(&harness).expect("open");
    assert!(card.min.y > trigger.max.y, "precondition: there is a gap");
    let gap = pos2(card.min.x + 2.0, (trigger.max.y + card.min.y) / 2.0);
    assert!(
        !trigger.contains(gap) && !card.contains(gap),
        "precondition: {gap:?} is in the gap"
    );
    harness.hover_at(gap);
    harness.run_steps(2);
    harness.hover_at(card.center());
    wait(&mut harness, Duration::from_millis(1000));
    assert!(is_open(&harness), "the card counts as the trigger");
}

#[test]
fn leaving_closes_it_after_the_close_delay() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let away = away(&harness);
    harness.hover_at(away);
    wait(&mut harness, CLOSE - Duration::from_millis(100));
    assert!(is_open(&harness), "still open within the close delay");
    wait(&mut harness, Duration::from_millis(200));
    assert!(!is_open(&harness));
}

#[test]
fn coming_back_within_the_close_delay_keeps_it_open() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let away = away(&harness);
    harness.hover_at(away);
    harness.run_steps(2);
    let card = card_box(&harness).expect("still open");
    harness.hover_at(card.center());
    wait(&mut harness, Duration::from_millis(1000));
    assert!(is_open(&harness));
}

#[test]
fn a_press_that_started_in_the_card_holds_it_open_until_release() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let card = card_box(&harness).expect("open");
    let inside = pos2(card.min.x + 6.0, card.min.y + 6.0);
    assert!(
        !rect_of(&harness, FOLLOW).contains(inside),
        "precondition: the press is not on the button"
    );
    harness.hover_at(inside);
    harness.step();
    button(&mut harness, inside, true);
    let away = away(&harness);
    harness.hover_at(away);
    wait(&mut harness, Duration::from_millis(1000));
    assert!(is_open(&harness), "held by the press");
    button(&mut harness, away, false);
    wait(&mut harness, CLOSE + Duration::from_millis(100));
    assert!(!is_open(&harness), "closes once released");
}

#[test]
fn the_cards_button_takes_a_click_and_the_card_stays() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let follow = rect_of(&harness, FOLLOW).center();
    harness.hover_at(follow);
    harness.run_steps(2);
    support::press_and_release(&mut harness, follow);
    harness.run_steps(2);
    assert_eq!(harness.state().app.follows, 1);
    assert!(is_open(&harness));
}

#[test]
fn a_press_on_the_trigger_closes_it_until_the_trigger_is_left_and_the_click_lands() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let at = rect_of(&harness, HANDLE).center();
    support::press_and_release(&mut harness, at);
    harness.run_steps(2);
    assert_eq!(harness.state().app.clicks, 1, "the trigger's own click");
    assert!(!is_open(&harness));
    wait(&mut harness, OPEN + Duration::from_millis(300));
    assert!(!is_open(&harness), "latched while the pointer stays");
    let away = away(&harness);
    harness.hover_at(away);
    harness.run_steps(2);
    open_by_pointer(&mut harness);
}

#[test]
fn escape_closes_it_until_the_trigger_is_left() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert!(!is_open(&harness));
    wait(&mut harness, OPEN + Duration::from_millis(300));
    assert!(!is_open(&harness), "latched while the pointer stays");
    let away = away(&harness);
    harness.hover_at(away);
    harness.run_steps(2);
    open_by_pointer(&mut harness);
}

#[test]
fn focus_opens_it_after_the_delay_without_moving_in_and_tab_away_closes_it() {
    let mut harness = harness_with(App {
        after: true,
        ..App::default()
    });
    harness.state_mut().companion = support::Companion::Alone;
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some(HANDLE),
        "precondition: Tab reached the trigger"
    );
    wait(&mut harness, OPEN - Duration::from_millis(200));
    assert!(!is_open(&harness), "focus waits for the delay");
    wait(&mut harness, Duration::from_millis(300));
    assert!(is_open(&harness));
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some(HANDLE),
        "focus stays on the trigger"
    );
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some("/\"after\""),
        "Tab goes on along the page"
    );
    wait(&mut harness, CLOSE + Duration::from_millis(100));
    assert!(!is_open(&harness));
}

#[test]
fn tab_never_stops_in_the_card() {
    let mut harness = harness();
    harness.state_mut().companion = support::Companion::Alone;
    harness.key_press(Key::Tab);
    wait(&mut harness, OPEN + Duration::from_millis(100));
    assert!(is_open(&harness), "precondition: focus opened it");
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some(HANDLE),
        "precondition: the trigger is the page's last stop"
    );
    for _ in 0..3 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert_ne!(support::focused_node(&harness).as_deref(), Some(FOLLOW));
    }
}

#[test]
fn the_card_replaces_the_tooltips_of_and_around_its_trigger() {
    let mut harness = harness();
    let around = rect_of(&harness, AROUND);
    let trigger = rect_of(&harness, "/\"around\"/\"trigger\"");
    let padding = pos2(around.min.x + 2.0, around.min.y + 2.0);
    assert!(
        !trigger.contains(padding),
        "precondition: the point is off the trigger"
    );
    hover_over(&mut harness, padding);
    wait(&mut harness, TOOLTIP + Duration::from_millis(200));
    assert!(
        tooltip_shown(&harness),
        "precondition: the tooltip around the trigger shows off it"
    );
    let away = away(&harness);
    hover_over(&mut harness, away);
    wait(&mut harness, Duration::from_millis(600));
    open_by_pointer(&mut harness);
    wait(&mut harness, Duration::from_millis(600));
    assert!(!tooltip_shown(&harness), "neither tooltip shows");
}

#[test]
fn a_tooltip_inside_the_card_shows_and_the_card_stays() {
    let mut harness = harness();
    open_by_pointer(&mut harness);
    let follow = rect_of(&harness, FOLLOW).center();
    harness.hover_at(follow);
    wait(&mut harness, TOOLTIP + Duration::from_millis(300));
    assert!(tooltip_shown(&harness));
    assert!(is_open(&harness));
}

#[test]
fn a_touch_never_opens_it() {
    let mut harness = harness();
    let at = rect_of(&harness, HANDLE).center();
    // kittest gives each event a frame of its own; a touch screen sends the
    // touch with the pointer events it makes, in one.
    harness.hover_at(at);
    harness.event(Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: at,
        force: None,
    });
    harness.step();
    wait(&mut harness, OPEN + Duration::from_millis(500));
    assert!(!is_open(&harness));
    harness.event(Event::PointerMoved(at + vec2(1.0, 0.0)));
    harness.step();
    wait(&mut harness, OPEN + Duration::from_millis(100));
    assert!(is_open(&harness), "a mouse after the touch does");
}

#[test]
fn a_card_inside_a_card_is_reported_and_never_opens() {
    let mut harness = harness_with(App {
        nested: true,
        ..App::default()
    });
    open_by_pointer(&mut harness);
    let inner = rect_of(&harness, "/\"around\"/\"card\"/\"Box\"/\"inner-trigger\"").center();
    harness.hover_at(inner);
    wait(&mut harness, OPEN + Duration::from_millis(500));
    assert!(is_open(&harness), "the outer card stays");
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.contains("\"inner\"/")),
        "the inner card never opens"
    );
    assert!(
        eggshell_ui::testing::reports(&harness.ctx)
            .iter()
            .any(|report| report.problem == Problem::NestedHoverCard)
    );
}

#[test]
fn the_trigger_is_described_by_the_card_text() {
    let harness = harness();
    let node = harness.get_by_label("@nextjs");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Inner tip The React Framework"),
        "its own tooltip's text, then the card's"
    );
}
