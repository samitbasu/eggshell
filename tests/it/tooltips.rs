//! Tooltips on the owned controls: eggshell overlays in `Band::Tooltip`, opened
//! by hover intent (a delay, a grace window between triggers, keyboard focus),
//! also on a disabled control, and in addition to the accessible name.

use core::time::Duration;

use eggshell_ui::owned::{Choice, Enablement};
use eggshell_ui::testing::{Timeline, record};
use eggshell_ui::{Builder, Style, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Key, Rect, RichText, Shape, Theme, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, hover_node, rect_of};

const DELAY: Duration = Duration::from_millis(500);
const GRACE: Duration = Duration::from_millis(300);
/// Frames are this far apart.
const STEP: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pick {
    A,
    B,
}

const PICKS: [(Pick, &str); 2] = [(Pick::A, "Alpha"), (Pick::B, "Beta")];

#[derive(Default)]
struct App {
    undone: u32,
    inside: u32,
    /// The built-tooltip trigger is not built.
    gone: bool,
    /// A second built-tooltip trigger is built.
    second: bool,
}

fn prepare(ctx: &egui::Context, _: &App) {
    let mut style = (*Style::of(ctx)).clone();
    style.tooltip.delay = DELAY;
    style.tooltip.grace = GRACE;
    style.install(ctx);
}

fn harness_of(view: fn(&mut Builder<'_, '_, App>, &App)) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness_scaled_stepped(
        vec2(400.0, 600.0),
        STEP,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
        prepare,
    );
    harness.run_steps(12);
    harness
}

fn frames(time: Duration) -> usize {
    (time.as_secs_f32() / STEP).ceil() as usize
}

fn wait(harness: &mut Harness<'_, Rig<App>>, time: Duration) {
    harness.run_steps(frames(time));
}

fn tooltip_nodes(harness: &Harness<'_, Rig<App>>) -> Vec<(String, Rect)> {
    harness
        .state()
        .nodes
        .iter()
        .filter(|(path, _)| path.contains("Tooltip("))
        .cloned()
        .collect()
}

fn is_shown(harness: &Harness<'_, Rig<App>>) -> bool {
    !tooltip_nodes(harness).is_empty()
}

/// The tooltip's box, if one is shown.
fn tip_box(harness: &Harness<'_, Rig<App>>) -> Option<Rect> {
    tooltip_nodes(harness)
        .into_iter()
        .find(|(path, _)| path.ends_with("/\"Box\""))
        .map(|(_, rect)| rect)
}

fn painted(harness: &Harness<'_, Rig<App>>, text: &str) -> bool {
    support::shapes(harness).iter().any(|shape| match shape {
        Shape::Text(shape) => shape.galley.text() == text,
        _ => false,
    })
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
            b.button("undo", "")
                .named("Undo")
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

fn harness() -> Harness<'static, Rig<App>> {
    harness_of(view)
}

fn hover_shows(path: &str, tip: &str) {
    let mut harness = harness();
    assert!(!is_shown(&harness), "precondition: nothing is shown yet");
    hover_node(&mut harness, path);
    wait(&mut harness, DELAY + GRACE);
    assert!(is_shown(&harness), "hovering {path} opens a tooltip");
    assert!(painted(&harness, tip), "hovering {path} shows {tip:?}");
}

#[test]
fn a_button_shows_its_tooltip_and_keeps_its_accessible_name() {
    hover_shows("/\"undo\"", "Undo (Ctrl+Z)");
    let harness = harness();
    assert!(harness.query_by_label("Undo").is_some());
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
    let mut harness = harness();
    hover_node(&mut harness, "/\"radio\"/Circle(0)");
    wait(&mut harness, DELAY + GRACE);
    assert!(!is_shown(&harness));
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
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY - Duration::from_millis(150));
    assert!(!is_shown(&harness), "not before the delay");
    wait(&mut harness, Duration::from_millis(300));
    assert!(is_shown(&harness), "after it");
}

#[test]
fn leaving_the_trigger_hides_the_tooltip() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    assert!(is_shown(&harness), "precondition");
    harness.hover_at(pos2(390.0, 590.0));
    wait(&mut harness, Duration::from_millis(400));
    assert!(!is_shown(&harness));
}

#[test]
fn a_second_trigger_within_the_grace_opens_at_once() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    assert!(is_shown(&harness), "precondition");
    harness.hover_at(pos2(390.0, 590.0));
    wait(&mut harness, Duration::from_millis(100));
    hover_node(&mut harness, "/\"check\"/Box");
    harness.run_steps(2);
    assert!(
        painted(&harness, "Tip for check"),
        "no delay within the grace"
    );
}

#[test]
fn a_second_trigger_after_the_grace_waits_again() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    harness.hover_at(pos2(390.0, 590.0));
    wait(&mut harness, GRACE + Duration::from_millis(400));
    assert!(!is_shown(&harness), "precondition: the first is gone");
    hover_node(&mut harness, "/\"check\"/Box");
    harness.run_steps(2);
    assert!(!painted(&harness, "Tip for check"));
    wait(&mut harness, DELAY);
    assert!(painted(&harness, "Tip for check"));
}

#[test]
fn moving_straight_to_another_trigger_swaps_tooltips_at_once() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    hover_node(&mut harness, "/\"check\"/Box");
    harness.run_steps(2);
    assert!(painted(&harness, "Tip for check"));
}

#[test]
fn a_press_hides_the_tooltip_until_the_trigger_is_left() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    assert!(is_shown(&harness), "precondition");
    let at = rect_of(&harness, "/\"undo\"").center();
    support::press_and_release(&mut harness, at);
    wait(&mut harness, Duration::from_millis(400));
    assert!(!is_shown(&harness), "hidden by the press");
    wait(&mut harness, DELAY + GRACE);
    assert!(!is_shown(&harness), "and not shown again while still on it");
    assert_eq!(harness.state().app.undone, 1);
}

#[test]
fn escape_hides_the_tooltip() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    assert!(is_shown(&harness), "precondition");
    harness.key_press(Key::Escape);
    wait(&mut harness, Duration::from_millis(400));
    assert!(!is_shown(&harness));
}

#[test]
fn keyboard_focus_shows_the_tooltip_at_once_and_losing_it_hides_it() {
    let mut harness = harness();
    harness.state_mut().companion = support::Companion::Alone;
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some("/\"undo\""),
        "precondition: Tab reached the first button"
    );
    assert!(painted(&harness, "Undo (Ctrl+Z)"), "no delay for focus");
    harness.key_press(Key::Tab);
    wait(&mut harness, Duration::from_millis(400));
    assert!(!painted(&harness, "Undo (Ctrl+Z)"));
}

#[test]
fn a_tooltip_never_takes_the_pointer() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"save\"");
    wait(&mut harness, DELAY + GRACE);
    let tip = tip_box(&harness).expect("the tooltip is shown");
    let undo = rect_of(&harness, "/\"undo\"");
    assert!(
        tip.intersects(undo),
        "precondition: the tooltip {tip:?} covers the button {undo:?}"
    );
    let at = undo.intersect(tip).center();
    support::hover_over(&mut harness, at);
    support::press_and_release(&mut harness, at);
    assert_eq!(
        harness.state().app.undone,
        1,
        "the click reached the button"
    );
}

#[test]
fn a_tooltip_sits_above_its_trigger_and_flips_below_at_the_top() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"save\"");
    wait(&mut harness, DELAY + GRACE);
    let above = tip_box(&harness).expect("shown");
    assert!(above.max.y <= rect_of(&harness, "/\"save\"").min.y, "above");

    let mut harness = harness_of(|b, _| {
        b.button("top", "Top").tooltip("Below it").add();
    });
    hover_node(&mut harness, "/\"top\"");
    wait(&mut harness, DELAY + GRACE);
    let below = tip_box(&harness).expect("shown");
    assert!(
        below.min.y >= rect_of(&harness, "/\"top\"").max.y,
        "flipped below the trigger at the top of the window"
    );
}

#[test]
fn long_text_wraps_at_the_maximum_width() {
    const LONG: &str = "A tooltip that says a great deal more than will fit on a single line of the maximum width the style allows it";
    let mut harness = harness_of(|b, _| {
        b.div(tw().pt(10.0), |b| {
            b.button("long", "Long").tooltip(LONG).add();
        });
    });
    hover_node(&mut harness, "/\"long\"");
    wait(&mut harness, DELAY + GRACE);
    let tip = tip_box(&harness).expect("shown");
    let max = Style::of(&harness.ctx).tooltip.max_width;
    assert!(tip.width() <= max + 0.5, "{} is within {max}", tip.width());
    assert!(
        tip.width() >= max - 40.0,
        "precondition: it filled the width"
    );
    assert!(tip.height() > 2.0 * 16.0, "it wrapped onto several lines");
}

#[test]
fn the_trigger_is_described_by_the_tooltip_text() {
    let harness = harness();
    let node = harness.get_by_label("Undo");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Undo (Ctrl+Z)")
    );
}

#[test]
fn a_tooltip_opening_swapping_and_closing_never_teleports() {
    let mut harness = harness();
    record(&harness.ctx);
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    hover_node(&mut harness, "/\"check\"/Box");
    wait(&mut harness, Duration::from_millis(400));
    harness.hover_at(pos2(390.0, 590.0));
    wait(&mut harness, Duration::from_millis(600));
    let timeline = Timeline::recorded(&harness.ctx);
    assert!(timeline.len() > 20, "precondition: frames were recorded");
    timeline.assert_continuous();
}

#[test]
fn the_neutral_style_takes_its_timing_and_width_from_egui() {
    let mut egui_style = egui::Style::default();
    egui_style.interaction.tooltip_delay = 0.25;
    egui_style.interaction.tooltip_grace_time = 0.75;
    let style = Style::from_egui(&egui_style);
    assert_eq!(style.tooltip.delay, Duration::from_millis(250));
    assert_eq!(style.tooltip.grace, Duration::from_millis(750));
    assert!((style.tooltip.max_width - egui_style.spacing.tooltip_width).abs() < f32::EPSILON);
    assert!(style.tooltip.arrow.is_none());
}

#[test]
fn the_shadcn_tooltip_is_inverted_with_an_arrow() {
    for theme in [Theme::Light, Theme::Dark] {
        let preset = eggshell_ui_shadcn::Shadcn::for_theme(theme);
        let style = preset.style();
        let rest = style.tooltip.popup.paint.states.rest;
        assert_eq!(rest.fill, preset.tokens.foreground);
        assert_eq!(rest.text, preset.tokens.background);
        assert_eq!(style.tooltip.arrow.map(|arrow| arrow.size), Some(10.0));
        assert_eq!(
            style.motion.tooltip.entry,
            eggshell_ui::contract::Entry::Zoom
        );
    }
}

/// Steps frames only while egui asks for one, as a reactive app (eframe, the
/// browser) does when no input arrives: a resting pointer produces none.
fn run_while_repainting(harness: &mut Harness<'_, Rig<App>>, limit: Duration) {
    for _ in 0..frames(limit) {
        if !harness.ctx.has_requested_repaint() {
            break;
        }
        harness.step();
    }
}

#[test]
fn a_resting_pointer_gets_its_tooltip_without_further_input() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    assert!(
        !is_shown(&harness),
        "precondition: the delay has not run out yet"
    );
    run_while_repainting(&mut harness, DELAY * 4);
    assert!(
        is_shown(&harness),
        "the tooltip opens with no input after the hover, on repaints the library asks for"
    );
    run_while_repainting(&mut harness, DELAY * 4);
    assert!(
        !harness.ctx.has_requested_repaint(),
        "and the frames stop once it is shown"
    );
}

const RED: egui::Color32 = egui::Color32::from_rgb(200, 20, 20);

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4 pt-10">
///   <span title="Red mono">Rich</span>
///   <span title="Strong words">Strong</span>
///   <button>Target</button>
///   <button>Built</button>  <!-- tooltip: Save <kbd>Ctrl+S</kbd> -->
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
                    s.text(tw(), "Save");
                    s.kbd("Ctrl+S");
                    s.button("inside", "Inside")
                        .on_click(|app: &mut App| app.inside += 1);
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

fn rich_harness() -> Harness<'static, Rig<App>> {
    harness_of(rich_view)
}

fn galley_of<'h>(harness: &'h Harness<'_, Rig<App>>, text: &str) -> &'h egui::Galley {
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => Some(&*shape.galley),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text:?} is painted"))
}

fn tooltip_ink(harness: &Harness<'_, Rig<App>>) -> egui::Color32 {
    Style::of(&harness.ctx).tooltip.popup.paint.states.rest.text
}

#[test]
fn a_rich_text_tooltip_keeps_its_colour_and_font() {
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"rich\"");
    wait(&mut harness, DELAY + GRACE);
    let galley = galley_of(&harness, "Red mono");
    let format = &galley.job.sections[0].format;
    assert_eq!(format.color, RED);
    assert_eq!(format.font_id.family, egui::FontFamily::Monospace);
}

#[test]
fn strong_rich_text_takes_the_tooltip_ink() {
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"strong\"");
    wait(&mut harness, DELAY + GRACE);
    let ink = tooltip_ink(&harness);
    assert_ne!(
        ink,
        harness.ctx.global_style().visuals.strong_text_color(),
        "precondition: egui's strong colour is not the tooltip's ink"
    );
    let galley = galley_of(&harness, "Strong words");
    assert_eq!(galley.job.sections[0].format.color, ink);
}

#[test]
fn a_plain_string_tooltip_still_takes_the_text_path() {
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    let galley = galley_of(&harness, "Undo (Ctrl+Z)");
    assert_eq!(
        galley.job.sections[0].format.color,
        egui::Color32::PLACEHOLDER,
        "the library's text leaf, coloured by the painter"
    );
}

#[test]
fn a_built_tooltip_shows_its_content_in_the_box() {
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"built\"");
    wait(&mut harness, DELAY + GRACE);
    let tip = tip_box(&harness).expect("the tooltip is shown");
    for text in ["Save", "Ctrl+S"] {
        let galley = support::shapes(&harness)
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
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"built\"");
    assert!(
        !is_shown(&harness),
        "precondition: the delay has not run out"
    );
    run_while_repainting(&mut harness, DELAY * 4);
    assert!(painted(&harness, "Ctrl+S"), "shown with no further input");
    run_while_repainting(&mut harness, DELAY * 4);
    assert!(
        !harness.ctx.has_requested_repaint(),
        "and the frames stop once it is shown"
    );
}

#[test]
fn a_disabled_trigger_shows_its_built_tooltip() {
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"off\"");
    wait(&mut harness, DELAY + GRACE);
    assert!(painted(&harness, "Cannot"));
}

#[test]
fn a_built_tooltip_never_takes_the_pointer() {
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"built\"");
    wait(&mut harness, DELAY + GRACE);
    let tip = tip_box(&harness).expect("the tooltip is shown");
    let target = rect_of(&harness, "/\"target\"");
    assert!(
        tip.intersects(target),
        "precondition: the tooltip {tip:?} covers the button {target:?}"
    );
    let inside = tooltip_nodes(&harness)
        .into_iter()
        .find(|(path, _)| path.ends_with("\"inside\""))
        .map(|(_, rect)| rect)
        .expect("the content's button is laid out");
    assert!(
        inside.intersects(target),
        "precondition: the content's button {inside:?} covers the target {target:?}"
    );
    let at = inside.intersect(target).intersect(tip).center();
    support::hover_over(&mut harness, at);
    support::press_and_release(&mut harness, at);
    let app = &harness.state().app;
    assert_eq!(app.inside, 0, "the content took nothing");
    assert_eq!(app.undone, 1, "the click reached the button beneath");
}

#[test]
fn a_built_tooltip_describes_its_trigger_with_its_text_while_closed() {
    let harness = rich_harness();
    assert!(!is_shown(&harness), "precondition: no tooltip is open");
    let node = harness.get_by_label("Built");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Save Ctrl+S Inside")
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
    let mut state = eggshell_ui::LayoutState::new();
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
    let mut harness = harness_of(vanishing_view);
    harness.state_mut().app.second = second;
    harness.run_steps(2);
    hover_node(&mut harness, "/\"first\"");
    wait(&mut harness, DELAY + GRACE);
    assert!(painted(&harness, "First tip"), "precondition: it is open");
    harness.state_mut().app.gone = true;
    harness.run_steps(1);
    is_shown(&harness)
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
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"built\"");
    wait(&mut harness, DELAY - Duration::from_millis(150));
    assert!(!is_shown(&harness), "not before the delay");
    wait(&mut harness, Duration::from_millis(300));
    assert!(is_shown(&harness), "after it");
    harness.key_press(Key::Escape);
    wait(&mut harness, Duration::from_millis(400));
    assert!(!is_shown(&harness), "Escape hides it");
    wait(&mut harness, DELAY + GRACE);
    assert!(!is_shown(&harness), "and keeps it hidden while still on it");
}

#[test]
fn keyboard_focus_shows_a_built_tooltip_at_once() {
    let mut harness = rich_harness();
    harness.state_mut().companion = support::Companion::Alone;
    for _ in 0..2 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some("/\"built\""),
        "precondition: Tab reached the button with the built tooltip"
    );
    assert!(painted(&harness, "Ctrl+S"), "no delay for focus");
    assert!(
        tooltip_nodes(&harness)
            .iter()
            .any(|(path, _)| path.ends_with("\"inside\"")),
        "precondition: the tooltip holds a button"
    );
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    let focused = support::focused_path(&harness);
    assert!(
        !focused
            .as_deref()
            .is_some_and(|path| path.contains("Tooltip(")),
        "Tab never moves into the tooltip, as it did to {focused:?}"
    );
}

#[test]
fn a_kbd_takes_the_tooltip_look_inside_a_tooltip_and_its_own_on_the_page() {
    let mut harness = rich_harness();
    hover_node(&mut harness, "/\"built\"");
    wait(&mut harness, DELAY + GRACE);
    let kbd = Style::of(&harness.ctx).kbd.clone();
    assert_ne!(
        kbd.page.text, kbd.tooltip.text,
        "precondition: the two looks differ"
    );
    let everywhere = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 600.0));
    assert_eq!(
        support::text_color(&harness, everywhere, "K"),
        kbd.page.text
    );
    assert_eq!(
        support::text_color(&harness, everywhere, "Ctrl+S"),
        kbd.tooltip.text
    );
}

/// The four corners of every filled four-point path painted in `fill`.
fn diamonds(harness: &Harness<'_, Rig<App>>, fill: egui::Color32) -> Vec<Rect> {
    support::shapes(harness)
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
    let mut harness = harness();
    hover_node(&mut harness, "/\"undo\"");
    wait(&mut harness, DELAY + GRACE);
    let tip = tip_box(&harness).expect("precondition: the tooltip is shown");
    let trigger = rect_of(&harness, "/\"undo\"");
    assert!(
        tip.bottom() <= trigger.top(),
        "precondition: the tooltip sits above its trigger"
    );
    let style = Style::of(&harness.ctx);
    let size = style
        .tooltip
        .arrow
        .expect("precondition: the shadcn style has an arrow")
        .size;
    let fill = style.tooltip.popup.paint.states.rest.fill;
    let arrows = diamonds(&harness, fill);
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
