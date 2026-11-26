//! Tooltips on the owned controls: eggshell overlays in `Band::Tooltip`, opened
//! by hover intent (a delay, a grace window between triggers, keyboard focus),
//! also on a disabled control, and in addition to the accessible name.

use core::time::Duration;
use eggshell_ui::prelude::*;

use eggshell_ui::owned::{Enablement, button::Face, choice::Choice, tooltip};
use eggshell_ui::testing::{Driver, Target, Timeline, record};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Style, TextStyle, tw};
use egui::{Key, KeyboardShortcut, Rect, RichText, Shape, Theme, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

/// The widest `style` draws a tooltip's box.
fn max_width(style: &Style) -> f32 {
    let width = support::part(style, tooltip::Part::Content)
        .style_at(style.tokens.spacing)
        .max_size
        .width
        .into_raw();
    assert_eq!(width.tag(), taffy::CompactLength::LENGTH_TAG, "a length");
    width.value()
}

const DELAY: Duration = Duration::from_millis(500);
const GRACE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pick {
    A,
    B,
}

const PICKS: [(Pick, &str); 2] = [(Pick::A, "Alpha"), (Pick::B, "Beta")];

#[derive(Default)]
struct App {
    undone: u32,
    mounted: Mounted,
    /// The built-tooltip trigger is not built.
    gone: bool,
    /// A second built-tooltip trigger is built.
    second: bool,
}

/// The data a mount's view is built over, which is not the page's.
#[derive(Default)]
struct Mounted {
    clicks: u32,
}

/// Clicks on the button inside a built tooltip's content.
static INSIDE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn timed(style: &mut Style) {
    let mut tooltip = style.tooltip.clone();
    tooltip.delay = DELAY;
    tooltip.grace = GRACE;
    style.tooltip = tooltip;
}

fn harness_of(view: support::View<App>) -> Driver<App> {
    harness_styled(view, timed)
}

fn harness_styled(view: support::View<App>, edit: fn(&mut Style)) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .style(&eggshell_ui::SharedStyle::new(support::shadcn_with(edit)))
    .build();
    d.frames(12);
    d
}

fn tooltip_nodes(d: &Driver<App>) -> Vec<(String, Rect)> {
    support::nodes(d)
        .into_iter()
        .filter(|(path, _)| path.contains("Tooltip("))
        .collect()
}

fn is_shown(d: &Driver<App>) -> bool {
    !tooltip_nodes(d).is_empty()
}

/// The tooltip's box, if one is shown.
fn tip_box(d: &Driver<App>) -> Option<Rect> {
    tooltip_nodes(d)
        .into_iter()
        .find(|(path, _)| path.ends_with("/\"Box\""))
        .map(|(_, rect)| rect)
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4 w-[300px]">
///   <button title="Undo (Ctrl+Z)" aria-label="Undo"></button>
///   <button title="Cannot save" disabled>Save</button>
///   <input type="checkbox" title="…"> <button role="switch" title="…"></button>
///   <div role="radiogroup"><input type="radio" title="…"> …</div>
///   <div role="radiogroup">segmented, a title on one cell</div>
///   <input type="range" title="…"> <button role="combobox" title="…"></button>
///   <input type="number" title="…">
///   <span title="Tip for a span">Settings</span> <div title="Tip for a div" class="w-10 h-5"></div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .pt(10.0)
            .w_px(300.0),
        |b| {
            b.button("undo", Face::Empty("Undo".into()))
                .tooltip("Undo (Ctrl+Z)")
                .on_click(|app: &mut App| app.undone += 1);
            b.button("save", "Save")
                .tooltip("Cannot save")
                .enablement(Enablement::Disabled)
                .add();
            b.checkbox_unlabelled("check", false)
                .named("Check")
                .tooltip("Tip for check")
                .on_change(|_, _| {});
            b.switch_unlabelled("switch", false)
                .named("Switch")
                .tooltip("Tip for switch")
                .on_change(|_, _| {});
            b.radio_group(
                "radio",
                Pick::A,
                [
                    Choice::new(Pick::A, "Alpha"),
                    Choice::new(Pick::B, "Beta").tooltip("Tip for beta"),
                ],
            )
            .on_change(|_, _| {});
            b.segmented(
                "segmented",
                Pick::A,
                [
                    Choice::new(Pick::A, "Alpha"),
                    Choice::new(Pick::B, "Beta").tooltip("Tip for cell"),
                ],
            )
            .tooltip("Tip for segmented")
            .on_change(|_, _| {});
            b.slider("slider", 50.0, 0.0..=100.0)
                .named("Level")
                .tooltip("Tip for slider")
                .on_change(|_, _| {});
            b.select("select", Pick::A, PICKS)
                .named("Pick")
                .tooltip("Tip for select")
                .classes(tw().w_px(120.0))
                .on_change(|_, _| {});
            b.stepper("stepper", 3.0, 0.0..=10.0)
                .named("Count")
                .tooltip("Tip for stepper")
                .on_change(|_, _| {});
            b.span("crumb", tw(), "Settings").tooltip("Tip for a span");
            b.div_id("tile", tw().w_px(40.0).h_px(20.0), |_| {})
                .tooltip("Tip for a div")
                .on_click(|_| {});
        },
    );
}

fn driver() -> Driver<App> {
    harness_of(view)
}

fn hover_shows(path: &str, tip: &str) {
    let mut d = driver();
    assert!(!is_shown(&d), "precondition: nothing is shown yet");
    d.hover(Target::suffix(path));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "hovering {path} opens a tooltip");
    assert!(d.painted().shows(tip), "hovering {path} shows {tip:?}");
}

#[test]
fn a_button_shows_its_tooltip_and_keeps_its_accessible_name() {
    hover_shows("/\"undo\"", "Undo (Ctrl+Z)");
    let d = driver();
    assert!(d.harness().query_by_label("Undo").is_some());
}

#[test]
fn a_disabled_button_still_shows_its_tooltip() {
    hover_shows("/\"save\"", "Cannot save");
}

#[test]
fn checkbox_switch_slider_select_and_stepper_show_theirs() {
    hover_shows("/\"check\"/Box", "Tip for check");
    hover_shows("/\"switch\"/Track", "Tip for switch");
    hover_shows("/\"slider\"/Track", "Tip for slider");
    hover_shows("/\"select\"", "Tip for select");
    hover_shows("/\"stepper\"/Box/Field", "Tip for stepper");
    hover_shows("/\"stepper\"/Box/Buttons/Up", "Tip for stepper");
}

#[test]
fn an_option_of_a_radio_group_or_segmented_control_shows_its_own() {
    hover_shows("/\"radio\"/Circle(1)", "Tip for beta");
    hover_shows("/\"segmented\"/1", "Tip for cell");
    hover_shows("/\"segmented\"/0", "Tip for segmented");
}

#[test]
fn only_the_option_with_a_tooltip_shows_one() {
    let mut d = driver();
    d.hover(Target::suffix("/\"radio\"/Circle(0)"));
    d.advance(DELAY + GRACE);
    assert!(!is_shown(&d));
}

#[test]
fn a_plain_node_shows_the_tooltip_set_on_its_handle() {
    hover_shows("/\"crumb\"", "Tip for a span");
}

#[test]
fn a_tooltip_composes_with_another_handler_on_the_same_node() {
    hover_shows("/\"tile\"", "Tip for a div");
}

#[test]
fn a_tooltip_waits_for_the_delay() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY - Duration::from_millis(150));
    assert!(!is_shown(&d), "not before the delay");
    d.advance(Duration::from_millis(300));
    assert!(is_shown(&d), "after it");
}

#[test]
fn leaving_the_trigger_hides_the_tooltip() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "precondition");
    d.carry_to(pos2(390.0, 590.0))
        .advance(Duration::from_millis(400) - support::STEP);
    assert!(!is_shown(&d));
}

#[test]
fn a_second_trigger_within_the_grace_opens_at_once() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "precondition");
    d.carry_to(pos2(390.0, 590.0))
        .advance(Duration::from_millis(100) - support::STEP);
    d.hover(Target::suffix("/\"check\"/Box"));
    d.frames(2);
    assert!(
        d.painted().shows("Tip for check"),
        "no delay within the grace"
    );
}

#[test]
fn a_second_trigger_after_the_grace_waits_again() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    d.carry_to(pos2(390.0, 590.0))
        .advance(GRACE + Duration::from_millis(400) - support::STEP);
    assert!(!is_shown(&d), "precondition: the first is gone");
    d.hover(Target::suffix("/\"check\"/Box"));
    d.frames(2);
    assert!(!d.painted().shows("Tip for check"));
    d.advance(DELAY);
    assert!(d.painted().shows("Tip for check"));
}

#[test]
fn moving_straight_to_another_trigger_swaps_tooltips_at_once() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    d.hover(Target::suffix("/\"check\"/Box"));
    d.frames(2);
    assert!(d.painted().shows("Tip for check"));
}

#[test]
fn a_press_hides_the_tooltip_until_the_trigger_is_left() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "precondition");
    let at = d.rect(Target::suffix("/\"undo\"")).center();
    for pressed in [true, false] {
        d.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    d.advance(Duration::from_millis(400));
    assert!(!is_shown(&d), "hidden by the press");
    d.advance(DELAY + GRACE);
    assert!(!is_shown(&d), "and not shown again while still on it");
    assert_eq!(d.app().undone, 1);
}

#[test]
fn escape_hides_the_tooltip() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "precondition");
    d.press(Key::Escape)
        .advance(Duration::from_millis(400) - support::STEP);
    assert!(!is_shown(&d));
}

#[test]
fn keyboard_focus_shows_the_tooltip_at_once_and_losing_it_hides_it() {
    let mut d = driver();
    support::set_companion(&d, support::Companion::Alone);
    d.press(Key::Tab).frames(1);
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some("/\"undo\""),
        "precondition: Tab reached the first button"
    );
    assert!(d.painted().shows("Undo (Ctrl+Z)"), "no delay for focus");
    d.press(Key::Tab)
        .advance(Duration::from_millis(400) - support::STEP);
    assert!(!d.painted().shows("Undo (Ctrl+Z)"));
}

#[test]
fn a_tooltip_never_takes_the_pointer() {
    let mut d = driver();
    d.hover(Target::suffix("/\"save\""));
    d.advance(DELAY + GRACE);
    let tip = tip_box(&d).expect("the tooltip is shown");
    let undo = d.rect(Target::suffix("/\"undo\""));
    assert!(
        tip.intersects(undo),
        "precondition: the tooltip {tip:?} covers the button {undo:?}"
    );
    let at = undo.intersect(tip).center();
    d.click(at);
    assert_eq!(d.app().undone, 1, "the click reached the button");
}

#[test]
fn a_tooltip_sits_above_its_trigger_and_flips_below_at_the_top() {
    let mut d = driver();
    d.hover(Target::suffix("/\"save\""));
    d.advance(DELAY + GRACE);
    let above = tip_box(&d).expect("shown");
    assert!(
        above.max.y <= d.rect(Target::suffix("/\"save\"")).min.y,
        "above"
    );

    let mut d = harness_of(|b, _| {
        b.button("top", "Top").tooltip("Below it").add();
    });
    d.hover(Target::suffix("/\"top\""));
    d.advance(DELAY + GRACE);
    let below = tip_box(&d).expect("shown");
    assert!(
        below.min.y >= d.rect(Target::suffix("/\"top\"")).max.y,
        "flipped below the trigger at the top of the window"
    );
}

#[test]
fn long_text_wraps_at_the_maximum_width() {
    const LONG: &str = "A tooltip that says a great deal more than will fit on a single line of the maximum width the style allows it";
    let mut d = harness_of(|b, _| {
        b.div(tw().pt(10.0), |b| {
            b.button("long", "Long").tooltip(LONG).add();
        });
    });
    d.hover(Target::suffix("/\"long\""));
    d.advance(DELAY + GRACE);
    let tip = tip_box(&d).expect("shown");
    let max = max_width(&d.shell().style());
    assert!(tip.width() <= max + 0.5, "{} is within {max}", tip.width());
    assert!(
        tip.width() >= max - 40.0,
        "precondition: it filled the width"
    );
    assert!(tip.height() > 2.0 * 16.0, "it wrapped onto several lines");
}

#[test]
fn the_trigger_is_described_by_the_tooltip_text() {
    let d = driver();
    let node = d.harness().get_by_label("Undo");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Undo (Ctrl+Z)")
    );
}

#[test]
fn a_tooltip_opening_swapping_and_closing_never_teleports() {
    let mut d = driver();
    record(d.ctx());
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    d.hover(Target::suffix("/\"check\"/Box"));
    d.advance(Duration::from_millis(400));
    d.carry_to(pos2(390.0, 590.0))
        .advance(Duration::from_millis(600) - support::STEP);
    let timeline = Timeline::recorded(d.ctx());
    assert!(timeline.len() > 20, "precondition: frames were recorded");
    timeline.assert_continuous();
}

#[test]
fn the_shadcn_tooltip_is_inverted_with_an_arrow() {
    for theme in [Theme::Light, Theme::Dark] {
        let preset = eggshell_ui::theme::Shadcn::for_theme(theme);
        let style = preset.style();
        let content = support::part(&style, tooltip::Part::Content);
        assert_eq!(
            content.paint_classes().background,
            Some(preset.tokens.foreground)
        );
        assert_eq!(support::ink(&style, &content), preset.tokens.background);
        assert_eq!(style.tooltip.arrow.map(|arrow| arrow.size), Some(10.0));
        assert_eq!(
            style.motion.tooltip.entry,
            eggshell_ui::contract::Entry::Zoom
        );
    }
}

/// Steps frames only while egui asks for one, as a reactive app (eframe, the
/// browser) does when no input arrives: a resting pointer produces none.
fn run_while_repainting(d: &mut Driver<App>, limit: Duration) {
    for _ in 0..limit.as_nanos() / support::STEP.as_nanos() {
        if !d.ctx().has_requested_repaint() {
            break;
        }
        d.frames(1);
    }
}

#[test]
fn a_resting_pointer_gets_its_tooltip_without_further_input() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    assert!(!is_shown(&d), "precondition: the delay has not run out yet");
    run_while_repainting(&mut d, DELAY * 4);
    assert!(
        is_shown(&d),
        "the tooltip opens with no input after the hover, on repaints the library asks for"
    );
    run_while_repainting(&mut d, DELAY * 4);
    assert!(
        !d.ctx().has_requested_repaint(),
        "and the frames stop once it is shown"
    );
}

const RED: egui::Color32 = egui::Color32::from_rgb(200, 20, 20);

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4 pt-10">
///   <span title="Red mono">Rich</span>
///   <span title="Strong words">Strong</span>
///   <button>Target</button>
///   <button>Built</button>  <!-- tooltip: <button>Inside</button> Save <kbd>Ctrl+S</kbd> -->
///   <button disabled>Off</button>  <!-- tooltip: Cannot <kbd>Ctrl+S</kbd> -->
///   <kbd>K</kbd>
/// </div>
/// ```
fn rich_view(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .pt(10.0),
        |b| {
            b.span("rich", tw(), "Rich")
                .tooltip(RichText::new("Red mono").color(RED).monospace());
            b.span("strong", tw(), "Strong")
                .tooltip(RichText::new("Strong words").strong());
            b.button("target", "Target")
                .on_click(|app: &mut App| app.undone += 1);
            b.button("built", "Built")
                .tooltip_with(|s| {
                    s.button("inside", "Inside").on_click(|_| {
                        INSIDE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    });
                    s.text(tw(), "Save");
                    s.kbd("Ctrl+S");
                })
                .add();
            b.button("off", "Off")
                .tooltip_with(|s| {
                    s.text(tw(), "Cannot");
                    s.kbd("Ctrl+S");
                })
                .enablement(Enablement::Disabled)
                .add();
            b.kbd("K");
        },
    );
}

fn rich_harness() -> Driver<App> {
    harness_of(rich_view)
}

fn galley_of<'h>(d: &'h Driver<App>, text: &str) -> &'h egui::Galley {
    d.painted().expect(text).galley()
}

fn tooltip_ink(d: &Driver<App>) -> egui::Color32 {
    let style = d.shell().style();
    support::ink(&style, &support::part(&style, tooltip::Part::Content))
}

#[test]
fn a_rich_text_tooltip_keeps_its_colour_and_font() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"rich\""));
    d.advance(DELAY + GRACE);
    let galley = galley_of(&d, "Red mono");
    let format = &galley.job.sections[0].format;
    assert_eq!(format.color, RED);
    assert_eq!(format.font_id.family, egui::FontFamily::Monospace);
}

#[test]
fn strong_rich_text_takes_the_tooltip_ink() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"strong\""));
    d.advance(DELAY + GRACE);
    let ink = tooltip_ink(&d);
    assert_ne!(
        ink,
        d.ctx().global_style().visuals.strong_text_color(),
        "precondition: egui's strong colour is not the tooltip's ink"
    );
    let galley = galley_of(&d, "Strong words");
    assert_eq!(galley.job.sections[0].format.color, ink);
}

#[test]
fn a_plain_string_tooltip_still_takes_the_text_path() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    let galley = galley_of(&d, "Undo (Ctrl+Z)");
    assert_eq!(
        galley.job.sections[0].format.color,
        egui::Color32::PLACEHOLDER,
        "the library's text leaf, coloured by the painter"
    );
}

#[test]
fn a_built_tooltip_shows_its_content_in_the_box() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"built\""));
    d.advance(DELAY + GRACE);
    let tip = tip_box(&d).expect("the tooltip is shown");
    for text in ["Save", "Ctrl+S"] {
        let galley = support::shapes(&d)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(shape) if shape.galley.text() == text => Some(shape.pos),
                _ => None,
            });
        let at = galley.unwrap_or_else(|| panic!("{text:?} is painted"));
        assert!(
            tip.contains(at),
            "{text:?} at {at:?} is inside the box {tip:?}"
        );
    }
}

#[test]
fn a_built_tooltip_opens_on_the_frames_the_library_asks_for() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"built\""));
    assert!(!is_shown(&d), "precondition: the delay has not run out");
    run_while_repainting(&mut d, DELAY * 4);
    assert!(d.painted().shows("Ctrl+S"), "shown with no further input");
    run_while_repainting(&mut d, DELAY * 4);
    assert!(
        !d.ctx().has_requested_repaint(),
        "and the frames stop once it is shown"
    );
}

#[test]
fn a_disabled_trigger_shows_its_built_tooltip() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"off\""));
    d.advance(DELAY + GRACE);
    assert!(d.painted().shows("Cannot"));
}

/// Content shared by a control on the page and one in a mount.
fn shortcut(s: &mut Builder<'_, '_, ()>) {
    s.text(tw(), "Count");
    s.kbd("C");
}

fn mounted_view<'f>(b: &mut Builder<'_, 'f, Mounted>, _: &'f Mounted) {
    b.div(tw().flex().flex_col().items_start().gap(4.0), |b| {
        b.button("built", "Built")
            .tooltip_with(|s| s.text(tw(), "Mounted tip"))
            .on_click(|mounted: &mut Mounted| mounted.clicks += 1);
        b.button("shared", "Shared").tooltip_with(shortcut).add();
        b.button("text", "Text").tooltip("Mounted text tip").add();
    });
}

fn mount_view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .pt(10.0),
        |b| {
            b.button("page", "Page").tooltip_with(shortcut).add();
            b.mount(
                "mount",
                tw(),
                &app.mounted,
                |app: &mut App| &mut app.mounted,
                mounted_view,
            )
            .add();
        },
    );
}

/// Only a mount: no trigger of the page's own has built content.
fn mount_only_view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div(tw().p(4.0).pt(10.0), |b| {
        b.mount(
            "mount",
            tw(),
            &app.mounted,
            |app: &mut App| &mut app.mounted,
            mounted_view,
        )
        .add();
    });
}

fn mount_hover_shows(view: support::View<App>, path: &str, tip: &str) {
    let mut d = harness_of(view);
    assert!(!is_shown(&d), "precondition: nothing is shown yet");
    d.hover(Target::suffix(path));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "hovering {path} opens a tooltip");
    assert!(d.painted().shows(tip), "hovering {path} shows {tip:?}");
}

#[test]
fn a_mounted_controls_text_tooltip_shows() {
    mount_hover_shows(
        mount_only_view,
        "/\"mount\"/Frame/\"text\"",
        "Mounted text tip",
    );
}

#[test]
fn a_mounted_controls_built_tooltip_shows() {
    mount_hover_shows(mount_only_view, "/\"mount\"/Frame/\"built\"", "Mounted tip");
}

#[test]
fn one_content_helper_builds_a_tooltip_on_the_page_and_in_a_mount() {
    mount_hover_shows(mount_view, "/\"page\"", "Count");
    mount_hover_shows(mount_view, "/\"mount\"/Frame/\"shared\"", "Count");
}

#[test]
fn a_built_tooltip_never_takes_the_pointer() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"built\""));
    d.advance(DELAY + GRACE);
    let tip = tip_box(&d).expect("the tooltip is shown");
    let target = d.rect(Target::suffix("/\"target\""));
    assert!(
        tip.intersects(target),
        "precondition: the tooltip {tip:?} covers the button {target:?}"
    );
    let inside = tooltip_nodes(&d)
        .into_iter()
        .find(|(path, _)| path.ends_with("\"inside\""))
        .map(|(_, rect)| rect)
        .expect("the content's button is laid out");
    assert!(
        inside.intersects(target),
        "precondition: the content's button {inside:?} covers the target {target:?}"
    );
    let at = inside.intersect(target).intersect(tip).center();
    d.click(at);
    let app = &d.app();
    let inside = INSIDE.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(inside, 0, "the content took nothing");
    assert_eq!(app.undone, 1, "the click reached the button beneath");
}

#[test]
fn a_built_tooltip_describes_its_trigger_with_its_text_while_closed() {
    let d = rich_harness();
    assert!(!is_shown(&d), "precondition: no tooltip is open");
    let node = d.harness().get_by_label("Built");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Inside Save Ctrl+S")
    );
}

/// How often a closed built tooltip's content is built over a few frames,
/// with no pointer or focus on its trigger. Kittest needs accesskit, so this
/// drives a context of its own.
fn closed_builds(accesskit: bool) -> usize {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    let builds = Arc::new(AtomicUsize::new(0));
    let ctx = egui::Context::default();
    ctx.run_pass(Default::default(), |_| {})
        .textures_delta
        .clear();
    if accesskit {
        ctx.enable_accesskit();
    }
    let mut state =
        eggshell_ui::LayoutState::new(&eggshell_ui::SharedStyle::new(crate::support::shadcn()));
    for _ in 0..4 {
        let mut output = ctx.run_pass(Default::default(), |ctx| {
            let builds = builds.clone();
            state.build(ctx, |b: &mut Builder<'_, '_, ()>| {
                b.button("counted", "Counted")
                    .tooltip_with(move |s| {
                        builds.fetch_add(1, Relaxed);
                        s.text(tw(), "Counted");
                    })
                    .add();
            });
        });
        output.textures_delta.clear();
    }
    builds.load(Relaxed)
}

#[test]
fn a_closed_built_tooltip_is_built_only_for_accesskit() {
    assert!(
        closed_builds(true) > 0,
        "precondition: with accesskit on, the description is built"
    );
    assert_eq!(
        closed_builds(false),
        0,
        "nothing reads it, so nothing builds it"
    );
}

fn vanishing_view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .pt(10.0),
        |b| {
            if !app.gone {
                b.button("first", "First")
                    .tooltip_with(|s| s.text(tw(), "First tip"))
                    .add();
            }
            if app.second {
                b.button("second", "Second")
                    .tooltip_with(|s| s.text(tw(), "Second tip"))
                    .add();
            }
        },
    );
}

/// Whether the first trigger's tooltip is still shown on the frame after its
/// trigger stops being built.
fn shown_after_its_trigger_goes(second: bool) -> bool {
    let mut d = harness_of(vanishing_view);
    d.app_mut().second = second;
    d.frames(2);
    d.hover(Target::suffix("/\"first\""));
    d.advance(DELAY + GRACE);
    assert!(d.painted().shows("First tip"), "precondition: it is open");
    d.app_mut().gone = true;
    d.frames(1);
    is_shown(&d)
}

#[test]
fn a_built_tooltip_whose_trigger_goes_leaves_only_while_it_can_be_built() {
    assert!(
        shown_after_its_trigger_goes(true),
        "with another built trigger in the frame, it plays its exit"
    );
    assert!(
        !shown_after_its_trigger_goes(false),
        "with none, it ends at once rather than leave as an empty box"
    );
}

#[test]
fn a_built_tooltip_follows_hover_intent() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"built\""));
    d.advance(DELAY - Duration::from_millis(150));
    assert!(!is_shown(&d), "not before the delay");
    d.advance(Duration::from_millis(300));
    assert!(is_shown(&d), "after it");
    d.press(Key::Escape)
        .advance(Duration::from_millis(400) - support::STEP);
    assert!(!is_shown(&d), "Escape hides it");
    d.advance(DELAY + GRACE);
    assert!(!is_shown(&d), "and keeps it hidden while still on it");
}

#[test]
fn keyboard_focus_shows_a_built_tooltip_at_once() {
    let mut d = rich_harness();
    support::set_companion(&d, support::Companion::Alone);
    for _ in 0..2 {
        d.press(Key::Tab).frames(1);
    }
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some("/\"built\""),
        "precondition: Tab reached the button with the built tooltip"
    );
    assert!(d.painted().shows("Ctrl+S"), "no delay for focus");
    assert!(
        tooltip_nodes(&d)
            .iter()
            .any(|(path, _)| path.ends_with("\"inside\"")),
        "precondition: the tooltip holds a button"
    );
    d.press(Key::Tab).frames(1);
    let focused = support::focused_path(&d);
    assert!(
        !focused
            .as_deref()
            .is_some_and(|path| path.contains("Tooltip(")),
        "Tab never moves into the tooltip, as it did to {focused:?}"
    );
}

#[test]
fn a_kbd_takes_the_tooltip_look_inside_a_tooltip_and_its_own_on_the_page() {
    let mut d = rich_harness();
    d.hover(Target::suffix("/\"built\""));
    d.advance(DELAY + GRACE);
    let kbd = d.shell().style().kbd.clone();
    assert_ne!(
        kbd.page.text, kbd.tooltip.text,
        "precondition: the two looks differ"
    );
    assert_eq!(d.painted().expect("K").color(), kbd.page.text);
    assert_eq!(d.painted().expect("Ctrl+S").color(), kbd.tooltip.text);
}

fn red_tooltips(style: &mut Style) {
    timed(style);
    style
        .sheet
        .as_app()
        .rule(eggshell_ui::owned::tooltip::Part::Content, |t, _| {
            t.bg(egui::Color32::RED)
        });
}

#[test]
fn a_rule_on_the_content_part_restyles_the_box_and_its_arrow() {
    let mut d = harness_styled(view, red_tooltips);
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    assert!(tip_box(&d).is_some(), "precondition: the tooltip is shown");
    assert_eq!(
        diamonds(&d, egui::Color32::RED).len(),
        1,
        "the arrow takes the content's fill"
    );
}

/// The four corners of every filled four-point path painted in `fill`.
fn diamonds(d: &Driver<App>, fill: egui::Color32) -> Vec<Rect> {
    support::shapes(d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Path(path) if path.points.len() == 4 && path.fill == fill => {
                Some(Rect::from_points(&path.points))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_arrow_is_painted_between_the_box_and_its_trigger() {
    let mut d = driver();
    d.hover(Target::suffix("/\"undo\""));
    d.advance(DELAY + GRACE);
    let tip = tip_box(&d).expect("precondition: the tooltip is shown");
    let trigger = d.rect(Target::suffix("/\"undo\""));
    assert!(
        tip.bottom() <= trigger.top(),
        "precondition: the tooltip sits above its trigger"
    );
    let style = d.shell().style();
    let size = style
        .tooltip
        .arrow
        .expect("precondition: the shadcn style has an arrow")
        .size;
    let fill = support::part(&style, tooltip::Part::Content)
        .paint_classes()
        .background
        .expect("precondition: the box is filled");
    let arrows = diamonds(&d, fill);
    assert_eq!(arrows.len(), 1, "one arrow in the box's fill: {arrows:?}");
    let arrow = arrows[0];
    assert!(
        (arrow.center().x - trigger.center().x).abs() < 0.5,
        "centred on the trigger: {arrow:?} vs {trigger:?}"
    );
    assert!(
        arrow.top() < tip.bottom() && arrow.bottom() > tip.bottom(),
        "straddles the box's bottom edge: {arrow:?} vs {tip:?}"
    );
    assert!(
        arrow.bottom() <= trigger.top(),
        "and stops short of the trigger"
    );
    assert!(
        (arrow.width() - size * std::f32::consts::SQRT_2).abs() < 0.5,
        "a {size} pt square turned 45°"
    );
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4 pt-10">
///   <canvas title="Tip for a custom" class="w-[60px] h-[30px]"></canvas>  <!-- senses hover -->
///   <canvas class="w-[60px] h-[30px]"></canvas>  <!-- senses hover -->  <!-- tooltip: Zoom <kbd>Z</kbd> -->
///   <button aria-pressed="false">Pin</button>  <!-- tooltip: Pin it <kbd>Ctrl+P</kbd> -->
/// </div>
/// ```
fn chain_view(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .pt(10.0),
        |b| {
            b.custom("plot", tw().sense_hover().w_px(60.0).h_px(30.0))
                .tooltip("Tip for a custom")
                .ui(|_, _| {});
            b.custom("zoom", tw().sense_hover().w_px(60.0).h_px(30.0))
                .tooltip_with(|s| {
                    s.text(tw(), "Zoom");
                    s.kbd("Z");
                })
                .ui(|_, _| {});
            b.toggle("pin", "Pin", false)
                .tooltip_with(|s| {
                    s.text(tw(), "Pin it");
                    s.kbd("Ctrl+P");
                })
                .on_change(|_, _| {});
        },
    );
}

/// Hovers `path` in [`chain_view`]: no tooltip before the delay, and after it
/// each of `texts` is painted inside the tooltip's box.
fn chain_hover_shows(path: &str, texts: &[&str]) {
    let mut d = harness_of(chain_view);
    assert!(!is_shown(&d), "precondition: nothing is shown yet");
    d.hover(Target::suffix(path));
    d.frames(1);
    assert!(!is_shown(&d), "{path}: not before the delay");
    d.advance(DELAY + GRACE);
    let tip = tip_box(&d).unwrap_or_else(|| panic!("hovering {path} opens a tooltip"));
    for text in texts {
        let at = support::shapes(&d)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(shape) if shape.galley.text() == *text => Some(shape.pos),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{path}: {text:?} is painted"));
        assert!(tip.contains(at), "{path}: {text:?} at {at:?} is in {tip:?}");
    }
}

#[test]
fn a_custom_node_shows_its_text_tooltip() {
    chain_hover_shows("/\"plot\"", &["Tip for a custom"]);
}

#[test]
fn a_custom_node_shows_its_built_tooltip() {
    chain_hover_shows("/\"zoom\"", &["Zoom", "Z"]);
}

#[test]
fn a_toggle_shows_its_built_tooltip() {
    chain_hover_shows("/\"pin\"", &["Pin it", "Ctrl+P"]);
}

/// A trigger with a tooltip that opens a popover or a dialog, inside a
/// container the app can stop building, beside a button that does so (#272).
mod dismissed {
    use eggshell_ui::overlay::{Anchor, Focus, Modal, Placement};

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Opens {
        Popover,
        Dialog,
    }

    pub(super) struct Host {
        opens: Opens,
        open: bool,
        gone: bool,
    }

    #[derive(Clone, Copy, Debug, Hash)]
    enum Part {
        Bar,
        Trigger,
        Remove,
        Overlay,
        Box,
        Inside,
    }

    const TIP: &str = "Show the panel";
    const TRIGGER: &str = "/Bar/Trigger";

    fn view(b: &mut Builder<'_, '_, Host>, host: &Host) {
        b.div(tw().flex().gap(4.0).p(4.0).pt(10.0), |b| {
            if !host.gone {
                b.div_id(Part::Bar, tw().flex(), |s| {
                    s.button(Part::Trigger, "Panel")
                        .tooltip(TIP)
                        .on_click(|host: &mut Host| host.open = !host.open);
                    if host.open {
                        overlay(s, host.opens);
                    }
                });
            }
            b.button(Part::Remove, "Remove")
                .on_click(|host: &mut Host| host.gone = true);
        });
    }

    fn overlay(s: &mut eggshell_ui::Scope<'_, '_, Host>, opens: Opens) {
        let trigger = s.id_of(Part::Trigger);
        let dismissed = |host: &mut Host| host.open = false;
        let inside = |s: &mut eggshell_ui::Scope<'_, '_, Host>| {
            s.div_id(Part::Box, tw().p(2.0).w_px(120.0), |s| {
                s.button(Part::Inside, "Inside").add();
            });
        };
        match opens {
            Opens::Popover => s
                .overlay(Part::Overlay)
                .anchor(Anchor::Node(trigger))
                .placement(Placement::below())
                .focus(Focus::Enter)
                .on_dismiss(dismissed)
                .children(inside),
            Opens::Dialog => s
                .overlay(Part::Overlay)
                .modal(Modal::Center)
                .on_dismiss(dismissed)
                .children(inside),
        }
    }

    pub(super) fn driver(opens: Opens) -> Driver<Host> {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(400.0, 400.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            Host {
                opens,
                open: false,
                gone: false,
            },
            view,
        )
        .style(&eggshell_ui::SharedStyle::new(support::shadcn_with(
            super::timed,
        )))
        .build();
        d.frames(12);
        d
    }

    fn tip_painted(d: &Driver<Host>) -> bool {
        support::shapes(d).iter().any(|shape| match shape {
            Shape::Text(shape) => shape.galley.text() == TIP,
            _ => false,
        })
    }

    fn settle(d: &mut Driver<Host>) {
        d.advance(DELAY + GRACE);
    }

    /// Presses `key`, then waits out the tooltip's delay and grace.
    fn press_and_settle(d: &mut Driver<Host>, key: Key) {
        d.press(key).advance(DELAY + GRACE - support::STEP);
    }

    /// Tabs to the trigger, whose tooltip shows on that arrival, and opens
    /// the overlay from the keyboard, so that focus enters it.
    fn open_from_the_keyboard(d: &mut Driver<Host>) {
        support::drive_past_bar(d);
        d.press(Key::Tab).frames(1);
        assert_eq!(
            support::focused_node(d).as_deref(),
            Some(TRIGGER),
            "precondition"
        );
        assert!(
            tip_painted(d),
            "precondition: a keyboard arrival shows the tooltip"
        );
        press_and_settle(d, Key::Enter);
        assert!(d.app().open, "precondition: open");
    }

    /// Presses `key`, which gives focus back to the trigger, and checks that
    /// the tooltip does not come with it.
    fn assert_focus_back_without_its_tooltip(d: &mut Driver<Host>, key: Key) {
        press_and_settle(d, key);
        assert!(!d.app().open, "precondition: dismissed");
        assert_eq!(
            support::focused_node(d).as_deref(),
            Some(TRIGGER),
            "precondition: focus came back"
        );
        assert!(!tip_painted(d), "focus given back is not an arrival");
    }

    #[test]
    fn escape_from_a_popover_gives_focus_back_without_the_tooltip() {
        let mut d = driver(Opens::Popover);
        open_from_the_keyboard(&mut d);
        assert_focus_back_without_its_tooltip(&mut d, Key::Escape);
    }

    #[test]
    fn escape_from_a_dialog_gives_focus_back_without_the_tooltip() {
        let mut d = driver(Opens::Dialog);
        open_from_the_keyboard(&mut d);
        assert_focus_back_without_its_tooltip(&mut d, Key::Escape);
    }

    #[test]
    fn a_key_after_a_press_dismissed_the_popover_does_not_show_the_tooltip() {
        let mut d = driver(Opens::Popover);
        open_from_the_keyboard(&mut d);
        let outside = pos2(390.0, 390.0);
        d.click(outside);
        settle(&mut d);
        assert_focus_back_without_its_tooltip(&mut d, Key::ArrowDown);
    }

    #[test]
    fn leaving_and_coming_back_is_an_arrival_again() {
        let mut d = driver(Opens::Popover);
        open_from_the_keyboard(&mut d);
        assert_focus_back_without_its_tooltip(&mut d, Key::Escape);
        press_and_settle(&mut d, Key::Tab);
        d.press(KeyboardShortcut::new(egui::Modifiers::SHIFT, Key::Tab))
            .frames(1);
        assert_eq!(
            support::focused_node(&d).as_deref(),
            Some(TRIGGER),
            "precondition"
        );
        assert!(tip_painted(&d), "Tabbing back to it shows it");
    }

    #[test]
    fn a_tooltip_whose_trigger_goes_is_dropped_quietly() {
        let mut d = driver(Opens::Popover);
        support::drive_past_bar(&mut d);
        d.press(Key::Tab).frames(1);
        assert!(tip_painted(&d), "precondition: open on the trigger's focus");
        d.click(Target::suffix("/Remove"));
        settle(&mut d);
        assert!(d.app().gone, "precondition: the trigger is gone");
        assert!(!tip_painted(&d));
        assert!(!is_shown_in(&d));
    }

    #[test]
    fn a_tooltip_shown_on_restored_focus_whose_trigger_goes_is_dropped_quietly() {
        let mut d = driver(Opens::Popover);
        open_from_the_keyboard(&mut d);
        d.press(Key::Escape).frames(3);
        d.click(Target::suffix("/Remove"));
        settle(&mut d);
        assert!(d.app().gone, "precondition: the trigger is gone");
        assert!(!tip_painted(&d));
        assert!(!is_shown_in(&d));
    }

    /// Removes the trigger after each of the first frames after Tab gives it
    /// focus, so that one removal lands in the build that first declares its
    /// tooltip open.
    #[test]
    fn a_tooltip_that_opens_as_its_trigger_goes_is_dropped_quietly() {
        let mut opened_in_time = false;
        for frames in 0..6 {
            let mut d = driver(Opens::Popover);
            support::drive_past_bar(&mut d);
            d.harness_mut().event(egui::Event::Key {
                key: Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            for _ in 0..frames {
                d.frames(1);
            }
            opened_in_time |= !is_shown_in(&d);
            d.app_mut().gone = true;
            settle(&mut d);
            assert!(!tip_painted(&d), "after {frames} frames");
            assert!(!is_shown_in(&d), "after {frames} frames");
        }
        assert!(
            opened_in_time,
            "precondition: some removal came before the tooltip was declared"
        );
    }

    fn is_shown_in(d: &Driver<Host>) -> bool {
        support::nodes(d)
            .iter()
            .any(|(path, _)| path.contains("Tooltip("))
    }
}

/// A segmented control's tooltip is the control's: one, anchored to the
/// control, staying put as the pointer crosses its cells. A cell whose
/// choice has a tooltip of its own shows that one instead while hovered.
mod segmented {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Axis {
        Position,
        Velocity,
        Current,
    }

    fn view(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div(tw().p(4.0).pt(20.0), |b| {
            b.segmented(
                "axis",
                Axis::Position,
                [
                    Choice::new(Axis::Position, "Position"),
                    Choice::new(Axis::Velocity, "Velocity"),
                    Choice::new(Axis::Current, "Current").tooltip("Tip for current"),
                ],
            )
            .tooltip("Tip for axis")
            .on_change(|_, _| {});
        });
    }

    #[test]
    fn the_controls_tooltip_is_one_and_stays_put_across_its_cells() {
        let mut d = harness_of(view);
        let first = d.rect(Target::suffix("/\"axis\"/0"));
        let second = d.rect(Target::suffix("/\"axis\"/1"));
        assert!(
            first.max.x <= second.min.x,
            "precondition: cells side by side"
        );
        stays_put(&mut d, first.center(), second.center(), "Tip for axis");
    }

    #[test]
    fn leaving_a_cell_with_its_own_tooltip_outward_never_shows_the_controls() {
        let mut d = harness_of(view);
        let cell = d.rect(Target::suffix("/\"axis\"/2"));
        let control = d.rect(Target::suffix("/\"axis\""));
        assert!(
            control.max.y > cell.max.y,
            "precondition: a band of the control below the cell"
        );
        d.hover(Target::suffix("/\"axis\"/2"));
        d.advance(DELAY + GRACE);
        assert!(
            d.painted().shows("Tip for current"),
            "precondition: it opened"
        );
        let x = cell.center().x;
        let mut y = cell.center().y;
        let mut crossed = false;
        while y < control.max.y + 12.0 {
            y += 1.0;
            d.carry_to(pos2(x, y));
            crossed |= y > cell.max.y && y < control.max.y;
            assert!(!d.painted().shows("Tip for axis"), "at y = {y}");
        }
        assert!(crossed, "precondition: the pointer crossed the band");
        d.advance(DELAY + GRACE);
        assert!(!d.painted().shows("Tip for axis"), "once outside");
    }

    #[test]
    fn the_tracks_padding_beside_a_cell_shows_that_cells_tooltip() {
        let mut d = harness_of(view);
        let control = d.rect(Target::suffix("/\"axis\""));
        let cell = d.rect(Target::suffix("/\"axis\"/2"));
        let at = pos2((cell.max.x + control.max.x) / 2.0, cell.center().y);
        assert!(
            control.contains(at) && !cell.contains(at),
            "precondition: {at:?} is on the track beside the cell"
        );
        d.carry_to(at).advance(DELAY + GRACE - support::STEP);
        assert!(d.painted().shows("Tip for current"));
        assert!(!d.painted().shows("Tip for axis"));
    }

    #[test]
    fn a_cell_with_its_own_tooltip_shows_only_that_one() {
        let mut d = harness_of(view);
        d.hover(Target::suffix("/\"axis\"/2"));
        d.advance(DELAY + GRACE);
        d.advance(Duration::from_millis(400));
        assert!(d.painted().shows("Tip for current"));
        assert!(!d.painted().shows("Tip for axis"));
        assert_eq!(boxes(&d).len(), 1);
    }
}

/// The tooltip boxes shown.
fn boxes(d: &Driver<App>) -> Vec<Rect> {
    tooltip_nodes(d)
        .into_iter()
        .filter(|(path, _)| path.ends_with("/\"Box\""))
        .map(|(_, rect)| rect)
        .collect()
}

/// Opens `tip` with the pointer at `from`, then moves it to `to` in steps:
/// at each, the one tooltip shown is where it opened.
fn stays_put(d: &mut Driver<App>, from: egui::Pos2, to: egui::Pos2, tip: &str) {
    d.carry_to(from).advance(DELAY + GRACE - support::STEP);
    assert!(d.painted().shows(tip), "precondition: {tip:?} opened");
    let at = boxes(d);
    assert_eq!(at.len(), 1, "one tooltip");
    for step in 0..=10 {
        let pointer = from.lerp(to, step as f32 / 10.0);
        d.carry_to(pointer).frames(1);
        assert_eq!(boxes(d), at, "at {pointer:?}");
    }
}

#[test]
fn a_steppers_tooltip_stays_put_from_its_field_to_its_buttons() {
    let mut d = driver();
    let field = d.rect(Target::suffix("/\"stepper\"/Box/Field")).center();
    let up = d
        .rect(Target::suffix("/\"stepper\"/Box/Buttons/Up"))
        .center();
    let down = d
        .rect(Target::suffix("/\"stepper\"/Box/Buttons/Down"))
        .center();
    stays_put(&mut d, field, up, "Tip for stepper");
    stays_put(&mut d, up, down, "Tip for stepper");
}

#[test]
fn a_sliders_tooltip_stays_put_from_its_track_to_its_thumb() {
    let mut d = driver();
    let track = d.rect(Target::suffix("/\"slider\"/Track"));
    let thumb = d.rect(Target::suffix("/\"slider\"/Track/Thumb"));
    let start = pos2(track.min.x + 8.0, track.center().y);
    assert!(
        !thumb.contains(start),
        "precondition: the track, off the thumb"
    );
    stays_put(&mut d, start, thumb.center(), "Tip for slider");
}

#[test]
fn keyboard_focus_on_a_part_shows_the_controls_tooltip_at_the_control() {
    let mut d = driver();
    support::set_companion(&d, support::Companion::Alone);
    let cell = "/\"segmented\"/0";
    for _ in 0..12 {
        if support::focused_node(&d).as_deref() == Some(cell) {
            break;
        }
        d.press(Key::Tab).frames(1);
    }
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some(cell),
        "precondition: Tab reached the cell"
    );
    assert!(d.painted().shows("Tip for segmented"), "no delay for focus");
    let control = d.rect(Target::suffix("/\"segmented\""));
    let shown = boxes(&d);
    assert_eq!(shown.len(), 1, "one tooltip");
    assert!(
        (shown[0].center().x - control.center().x).abs() < 1.0,
        "anchored to the control: {shown:?} over {control:?}"
    );
}

#[test]
fn a_part_is_described_by_its_controls_tooltip() {
    let d = driver();
    let described = |label: &str, tip: &str| {
        d.harness()
            .query_all_by_label(label)
            .any(|node| node.accesskit_node().description().as_deref() == Some(tip))
    };
    assert!(
        described("Alpha", "Tip for segmented"),
        "a cell of none of its own"
    );
    assert!(described("Beta", "Tip for cell"), "a cell of its own");
    assert!(described("Level", "Tip for slider"), "the slider's thumb");
    assert!(
        described("Increase Count", "Tip for stepper"),
        "the stepper's button"
    );
}

const LONG_NOTE: &str = "A note much longer than its narrow box";
const SHORT_NOTE: &str = "Fits";

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <div class="w-[80px]"><span class="truncate">A note much longer…</span></div>
///   <div class="w-[80px]"><span class="truncate">Fits</span></div>
/// </div>
/// ```
fn notes_view(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        for (key, note) in [("long", LONG_NOTE), ("short", SHORT_NOTE)] {
            b.div(tw().w_px(80.0), |b| {
                b.span(key, tw().truncate(), note).tooltip_when_truncated();
            });
        }
    });
}

/// For each time `text` is drawn, whether it was cut short.
fn drawn_elided(d: &Driver<App>, text: &str) -> Vec<bool> {
    support::shapes(d)
        .iter()
        .filter_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => Some(shape.galley.elided),
            _ => None,
        })
        .collect()
}

#[test]
fn a_truncated_span_shows_its_whole_text_and_one_that_fits_shows_none() {
    let mut d = harness_of(notes_view);
    assert_eq!(
        drawn_elided(&d, LONG_NOTE),
        [true],
        "precondition: the long note is cut"
    );
    assert_eq!(
        drawn_elided(&d, SHORT_NOTE),
        [false],
        "precondition: the short one fits"
    );

    d.hover(Target::suffix("/\"short\""));
    d.advance(DELAY + GRACE);
    assert!(!is_shown(&d), "text that fits has no tooltip");

    d.hover(Target::suffix("/\"long\""));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "the cut note opens a tooltip");
    assert_eq!(
        drawn_elided(&d, LONG_NOTE),
        [true, false],
        "the tooltip draws the whole note over the cut one"
    );
}

#[test]
fn a_truncated_span_keeps_its_whole_text_as_its_name_and_no_description() {
    let mut d = harness_of(notes_view);
    d.hover(Target::suffix("/\"long\""));
    d.advance(DELAY + GRACE);
    assert!(is_shown(&d), "precondition: the tooltip is open");
    let node = d.harness().get_by_label(LONG_NOTE);
    assert_eq!(node.accesskit_node().description(), None);
}

/// shadcn's tooltip content (`style-vega.css` `.cn-tooltip-content`,
/// tooltip.tsx): `bg-foreground text-background px-3 py-1.5 rounded-md`, and
/// `has-data-[slot=kbd]:pr-1.5` with `**:data-[slot=kbd]:rounded-sm`.
#[test]
fn the_tooltip_is_shadcns_after_a_section_change_and_reaches_its_kbd_chip() {
    // `harness_of` sets the tooltip section's delay after the preset, which
    // keeps the preset's look.
    let mut d = harness_of(|b, _| {
        b.div(
            tw().pt(10.0).gap(4.0).flex().flex_col().items_start(),
            |b| {
                b.button("plain", "Plain").tooltip("Plain words").add();
                b.button("keyed", "Keyed")
                    .tooltip_with(|s| {
                        s.text(tw(), "Save");
                        s.kbd("Ctrl+S");
                    })
                    .add();
            },
        );
    });
    assert_eq!(
        d.shell().style().tooltip.delay,
        DELAY,
        "precondition: the delay is the one set after the preset"
    );
    let wait = DELAY + GRACE;
    let t = Tokens::for_theme(d.ctx().theme());
    d.hover(Target::suffix("/\"plain\""));
    d.advance(wait);
    let tip = tip_box(&d).expect("shown");
    let shape = support::box_at(&d, tip);
    assert_eq!(shape.fill, t.foreground, "bg-foreground");
    assert_eq!(
        shape.corner_radius,
        egui::CornerRadius::same(8),
        "rounded-md"
    );
    let text = support::text_left(&d, tip, "Plain words");
    assert_eq!(text - tip.left(), 12.0, "px-3");

    d.hover(Target::suffix("/\"keyed\""));
    d.advance(wait);
    let tip = tip_box(&d).expect("shown");
    let chip = tooltip_nodes(&d)
        .into_iter()
        .filter(|(_, rect)| rect.width() < tip.width() && rect.height() < tip.height())
        .max_by(|(_, a), (_, b)| a.right().total_cmp(&b.right()))
        .map(|(_, rect)| rect)
        .expect("the kbd chip");
    assert_eq!(tip.right() - chip.right(), 6.0, "has-[kbd]:pr-1.5");
    assert_eq!(
        support::box_at(&d, chip).corner_radius,
        egui::CornerRadius::same(6),
        "rounded-sm"
    );
}
