//! `OverlayChain::hover_anchored`: an overlay hover intent opens and closes,
//! as a hover card, through the full draw path (pointer, focus, dismissal,
//! tooltips and Tab).

use core::time::Duration;

use eggshell_ui::overlay::{Distance, HoverIntent, Placement};
use eggshell_ui::testing::{Animation, Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Problem, Style, TextStyle, tw};
use egui::{Event, Key, Pos2, Rect, Theme, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

const OPEN: Duration = Duration::from_millis(700);
const CLOSE: Duration = Duration::from_millis(300);
const TOOLTIP: Duration = Duration::from_millis(500);
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

fn quick_tooltips(style: &mut Style) {
    let mut tooltip = style.tooltip.clone();
    tooltip.delay = TOOLTIP;
    style.tooltip = tooltip;
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

fn harness_with(app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .style(&eggshell_ui::SharedStyle::new(support::shadcn_with(
        quick_tooltips,
    )))
    // Motion off: a card that closes is gone in the same frame.
    .motion(Animation::Reduced)
    .build();
    d.frames(4);
    d
}

fn harness() -> Driver<App> {
    harness_with(App::default())
}

fn at(path: &str) -> Target {
    Target::suffix(path)
}

fn card_box(d: &Driver<App>) -> Option<Rect> {
    d.find(at("/\"around\"/\"card\"/\"Box\""))
}

fn is_open(d: &Driver<App>) -> bool {
    card_box(d).is_some()
}

fn tooltip_shown(d: &Driver<App>) -> bool {
    support::nodes(d)
        .iter()
        .any(|(path, _)| path.contains("Tooltip("))
}

/// Rests the pointer on the trigger until the card opens.
fn open_by_pointer(d: &mut Driver<App>) {
    d.hover(at(HANDLE))
        .advance(OPEN + Duration::from_millis(100));
    assert!(is_open(d), "precondition: the card opened");
}

/// A point on the page in neither the trigger, the area around it nor the
/// card.
fn away(d: &Driver<App>) -> Pos2 {
    let point = pos2(390.0, 390.0);
    for rect in [d.rect(at(AROUND))].into_iter().chain(card_box(d)) {
        assert!(
            !rect.contains(point),
            "precondition: {point:?} is off {rect:?}"
        );
    }
    point
}

#[test]
fn the_pointer_resting_on_the_trigger_opens_it_after_the_open_delay() {
    let mut d = harness();
    d.hover(at(HANDLE))
        .advance(OPEN - Duration::from_millis(200));
    assert!(!is_open(&d), "not before the delay");
    d.advance(Duration::from_millis(300));
    assert!(is_open(&d));
    assert!(
        card_box(&d).is_some_and(|card| card.min.y >= d.rect(at(HANDLE)).max.y),
        "below the trigger"
    );
}

#[test]
fn crossing_the_gap_into_the_card_keeps_it_open() {
    let mut d = harness();
    open_by_pointer(&mut d);
    let trigger = d.rect(at("/\"around\"/\"trigger\""));
    let card = card_box(&d).expect("open");
    assert!(card.min.y > trigger.max.y, "precondition: there is a gap");
    let gap = pos2(card.min.x + 2.0, (trigger.max.y + card.min.y) / 2.0);
    assert!(
        !trigger.contains(gap) && !card.contains(gap),
        "precondition: {gap:?} is in the gap"
    );
    d.carry_to(gap).frames(2);
    d.carry_to(card.center())
        .advance(Duration::from_millis(1000));
    assert!(is_open(&d), "the card counts as the trigger");
}

#[test]
fn leaving_closes_it_after_the_close_delay() {
    let mut d = harness();
    open_by_pointer(&mut d);
    let away = away(&d);
    d.carry_to(away).advance(CLOSE - Duration::from_millis(100));
    assert!(is_open(&d), "still open within the close delay");
    d.advance(Duration::from_millis(200));
    assert!(!is_open(&d));
}

#[test]
fn coming_back_within_the_close_delay_keeps_it_open() {
    let mut d = harness();
    open_by_pointer(&mut d);
    let away = away(&d);
    d.carry_to(away).frames(2);
    let card = card_box(&d).expect("still open");
    d.carry_to(card.center())
        .advance(Duration::from_millis(1000));
    assert!(is_open(&d));
}

#[test]
fn a_press_that_started_in_the_card_holds_it_open_until_release() {
    let mut d = harness();
    open_by_pointer(&mut d);
    let card = card_box(&d).expect("open");
    let inside = pos2(card.min.x + 6.0, card.min.y + 6.0);
    assert!(
        !d.rect(at(FOLLOW)).contains(inside),
        "precondition: the press is not on the button"
    );
    d.pointer_down(inside);
    let away = away(&d);
    d.carry_to(away).advance(Duration::from_millis(1000));
    assert!(is_open(&d), "held by the press");
    d.pointer_up(away)
        .advance(CLOSE + Duration::from_millis(100));
    assert!(!is_open(&d), "closes once released");
}

#[test]
fn the_cards_button_takes_a_click_and_the_card_stays() {
    let mut d = harness();
    open_by_pointer(&mut d);
    d.click(at(FOLLOW)).frames(2);
    assert_eq!(d.app().follows, 1);
    assert!(is_open(&d));
}

#[test]
fn a_press_on_the_trigger_closes_it_until_the_trigger_is_left_and_the_click_lands() {
    let mut d = harness();
    open_by_pointer(&mut d);
    d.click(at(HANDLE)).frames(2);
    assert_eq!(d.app().clicks, 1, "the trigger's own click");
    assert!(!is_open(&d));
    d.advance(OPEN + Duration::from_millis(300));
    assert!(!is_open(&d), "latched while the pointer stays");
    let away = away(&d);
    d.carry_to(away).frames(2);
    open_by_pointer(&mut d);
}

#[test]
fn escape_closes_it_until_the_trigger_is_left() {
    let mut d = harness();
    open_by_pointer(&mut d);
    d.press(Key::Escape).frames(1);
    assert!(!is_open(&d));
    d.advance(OPEN + Duration::from_millis(300));
    assert!(!is_open(&d), "latched while the pointer stays");
    let away = away(&d);
    d.carry_to(away).frames(2);
    open_by_pointer(&mut d);
}

#[test]
fn focus_opens_it_after_the_delay_without_moving_in_and_tab_away_closes_it() {
    let mut d = harness_with(App {
        after: true,
        ..App::default()
    });
    support::set_companion(&d, support::Companion::Alone);
    d.frames(1);
    d.press(Key::Tab).frames(1);
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some(HANDLE),
        "precondition: Tab reached the trigger"
    );
    d.advance(OPEN - Duration::from_millis(200));
    assert!(!is_open(&d), "focus waits for the delay");
    d.advance(Duration::from_millis(300));
    assert!(is_open(&d));
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some(HANDLE),
        "focus stays on the trigger"
    );
    d.press(Key::Tab).frames(1);
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some("/\"after\""),
        "Tab goes on along the page"
    );
    d.advance(CLOSE + Duration::from_millis(100));
    assert!(!is_open(&d));
}

#[test]
fn tab_never_stops_in_the_card() {
    let mut d = harness();
    support::set_companion(&d, support::Companion::Alone);
    d.frames(1);
    d.press(Key::Tab).advance(OPEN + Duration::from_millis(100));
    assert!(is_open(&d), "precondition: focus opened it");
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some(HANDLE),
        "precondition: the trigger is the page's last stop"
    );
    for _ in 0..3 {
        d.press(Key::Tab).frames(1);
        assert_ne!(support::focused_node(&d).as_deref(), Some(FOLLOW));
    }
}

#[test]
fn the_card_replaces_the_tooltips_of_and_around_its_trigger() {
    let mut d = harness();
    let around = d.rect(at(AROUND));
    let trigger = d.rect(at("/\"around\"/\"trigger\""));
    let padding = pos2(around.min.x + 2.0, around.min.y + 2.0);
    assert!(
        !trigger.contains(padding),
        "precondition: the point is off the trigger"
    );
    d.hover(padding)
        .advance(TOOLTIP + Duration::from_millis(200));
    assert!(
        tooltip_shown(&d),
        "precondition: the tooltip around the trigger shows off it"
    );
    let away = away(&d);
    d.hover(away).advance(Duration::from_millis(600));
    open_by_pointer(&mut d);
    d.advance(Duration::from_millis(600));
    assert!(!tooltip_shown(&d), "neither tooltip shows");
}

#[test]
fn a_tooltip_inside_the_card_shows_and_the_card_stays() {
    let mut d = harness();
    open_by_pointer(&mut d);
    d.carry_to(at(FOLLOW))
        .advance(TOOLTIP + Duration::from_millis(300));
    assert!(tooltip_shown(&d));
    assert!(is_open(&d));
}

#[test]
fn a_touch_never_opens_it() {
    let mut d = harness();
    let point = d.rect(at(HANDLE)).center();
    // kittest gives each event a frame of its own; a touch screen sends the
    // touch with the pointer events it makes, in one.
    d.carry_to(point);
    d.event(Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: point,
        force: None,
    });
    d.advance(OPEN + Duration::from_millis(500));
    assert!(!is_open(&d));
    d.event(Event::PointerMoved(point + vec2(1.0, 0.0)))
        .advance(OPEN + Duration::from_millis(100));
    assert!(is_open(&d), "a mouse after the touch does");
}

#[test]
fn a_card_inside_a_card_is_reported_and_never_opens() {
    let mut d = harness_with(App {
        nested: true,
        ..App::default()
    });
    open_by_pointer(&mut d);
    d.carry_to(at("/\"around\"/\"card\"/\"Box\"/\"inner-trigger\""))
        .advance(OPEN + Duration::from_millis(500));
    assert!(is_open(&d), "the outer card stays");
    assert!(
        !support::nodes(&d)
            .iter()
            .any(|(path, _)| path.contains("\"inner\"/")),
        "the inner card never opens"
    );
    d.expect_reported(Problem::NestedHoverCard);
}

#[test]
fn the_trigger_is_described_by_the_card_text() {
    let d = harness();
    let node = d.harness().get_by_label("@nextjs");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Inner tip The React Framework"),
        "its own tooltip's text, then the card's"
    );
}
