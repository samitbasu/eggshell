//! Tooltips on the owned controls: eggshell overlays in `Band::Tooltip`, opened
//! by hover intent (a delay, a grace window between triggers, keyboard focus),
//! also on a disabled control, and in addition to the accessible name.

use core::time::Duration;

use eggshell_ui::owned::{Choice, Enablement, Face};
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

fn prepare(ctx: &egui::Context, _: &App) {
    let mut style = (*Style::of(ctx)).clone();
    style.tooltip.delay = DELAY;
    style.tooltip.grace = GRACE;
    style.install(ctx);
}

fn harness_of(view: support::View<App>) -> Harness<'static, Rig<App>> {
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
                    s.button("inside", "Inside").on_click(|_| {
                        INSIDE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    });
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
    let mut harness = harness_of(view);
    assert!(!is_shown(&harness), "precondition: nothing is shown yet");
    hover_node(&mut harness, path);
    wait(&mut harness, DELAY + GRACE);
    assert!(is_shown(&harness), "hovering {path} opens a tooltip");
    assert!(painted(&harness, tip), "hovering {path} shows {tip:?}");
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
    let inside = INSIDE.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(inside, 0, "the content took nothing");
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
    let mut harness = harness_of(chain_view);
    assert!(!is_shown(&harness), "precondition: nothing is shown yet");
    hover_node(&mut harness, path);
    harness.run_steps(1);
    assert!(!is_shown(&harness), "{path}: not before the delay");
    wait(&mut harness, DELAY + GRACE);
    let tip = tip_box(&harness).unwrap_or_else(|| panic!("hovering {path} opens a tooltip"));
    for text in texts {
        let at = support::shapes(&harness)
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

    fn prepare(ctx: &egui::Context, _: &Host) {
        super::prepare(ctx, &App::default());
    }

    pub(super) fn harness(opens: Opens) -> Harness<'static, Rig<Host>> {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness_scaled_stepped(
            vec2(400.0, 400.0),
            STEP,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            Host {
                opens,
                open: false,
                gone: false,
            },
            view,
            prepare,
        );
        harness.run_steps(12);
        harness
    }

    fn tip_painted(harness: &Harness<'_, Rig<Host>>) -> bool {
        support::shapes(harness).iter().any(|shape| match shape {
            Shape::Text(shape) => shape.galley.text() == TIP,
            _ => false,
        })
    }

    fn settle(harness: &mut Harness<'_, Rig<Host>>) {
        harness.run_steps(frames(DELAY + GRACE));
    }

    /// Tabs to the trigger, whose tooltip shows on that arrival, and opens
    /// the overlay from the keyboard, so that focus enters it.
    fn open_from_the_keyboard(harness: &mut Harness<'_, Rig<Host>>) {
        support::past_bar(harness);
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert_eq!(
            support::focused_node(harness).as_deref(),
            Some(TRIGGER),
            "precondition"
        );
        assert!(
            tip_painted(harness),
            "precondition: a keyboard arrival shows the tooltip"
        );
        harness.key_press(Key::Enter);
        settle(harness);
        assert!(harness.state().app.open, "precondition: open");
    }

    fn assert_focus_back_without_its_tooltip(harness: &mut Harness<'_, Rig<Host>>) {
        settle(harness);
        assert!(!harness.state().app.open, "precondition: dismissed");
        assert_eq!(
            support::focused_node(harness).as_deref(),
            Some(TRIGGER),
            "precondition: focus came back"
        );
        assert!(!tip_painted(harness), "focus given back is not an arrival");
    }

    #[test]
    fn escape_from_a_popover_gives_focus_back_without_the_tooltip() {
        let mut harness = harness(Opens::Popover);
        open_from_the_keyboard(&mut harness);
        harness.key_press(Key::Escape);
        assert_focus_back_without_its_tooltip(&mut harness);
    }

    #[test]
    fn escape_from_a_dialog_gives_focus_back_without_the_tooltip() {
        let mut harness = harness(Opens::Dialog);
        open_from_the_keyboard(&mut harness);
        harness.key_press(Key::Escape);
        assert_focus_back_without_its_tooltip(&mut harness);
    }

    #[test]
    fn a_key_after_a_press_dismissed_the_popover_does_not_show_the_tooltip() {
        let mut harness = harness(Opens::Popover);
        open_from_the_keyboard(&mut harness);
        let outside = pos2(390.0, 390.0);
        support::hover_over(&mut harness, outside);
        support::press_and_release(&mut harness, outside);
        settle(&mut harness);
        harness.key_press(Key::ArrowDown);
        assert_focus_back_without_its_tooltip(&mut harness);
    }

    #[test]
    fn leaving_and_coming_back_is_an_arrival_again() {
        let mut harness = harness(Opens::Popover);
        open_from_the_keyboard(&mut harness);
        harness.key_press(Key::Escape);
        assert_focus_back_without_its_tooltip(&mut harness);
        harness.key_press(Key::Tab);
        settle(&mut harness);
        harness.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
        harness.run_steps(2);
        assert_eq!(
            support::focused_node(&harness).as_deref(),
            Some(TRIGGER),
            "precondition"
        );
        assert!(tip_painted(&harness), "Tabbing back to it shows it");
    }

    #[test]
    fn a_tooltip_whose_trigger_goes_is_dropped_quietly() {
        let mut harness = harness(Opens::Popover);
        support::past_bar(&mut harness);
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert!(
            tip_painted(&harness),
            "precondition: open on the trigger's focus"
        );
        support::click_node(&mut harness, "/Remove");
        settle(&mut harness);
        assert!(
            harness.state().app.gone,
            "precondition: the trigger is gone"
        );
        assert!(!tip_painted(&harness));
        assert!(!is_shown_in(&harness));
    }

    #[test]
    fn a_tooltip_shown_on_restored_focus_whose_trigger_goes_is_dropped_quietly() {
        let mut harness = harness(Opens::Popover);
        open_from_the_keyboard(&mut harness);
        harness.key_press(Key::Escape);
        harness.run_steps(4);
        support::click_node(&mut harness, "/Remove");
        settle(&mut harness);
        assert!(
            harness.state().app.gone,
            "precondition: the trigger is gone"
        );
        assert!(!tip_painted(&harness));
        assert!(!is_shown_in(&harness));
    }

    /// Removes the trigger after each of the first frames after Tab gives it
    /// focus, so that one removal lands in the build that first declares its
    /// tooltip open.
    #[test]
    fn a_tooltip_that_opens_as_its_trigger_goes_is_dropped_quietly() {
        let mut opened_in_time = false;
        for frames in 0..6 {
            let mut harness = harness(Opens::Popover);
            support::past_bar(&mut harness);
            harness.event(egui::Event::Key {
                key: Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            harness.step();
            for _ in 0..frames {
                harness.step();
            }
            opened_in_time |= !is_shown_in(&harness);
            harness.state_mut().app.gone = true;
            settle(&mut harness);
            assert!(!tip_painted(&harness), "after {frames} frames");
            assert!(!is_shown_in(&harness), "after {frames} frames");
        }
        assert!(
            opened_in_time,
            "precondition: some removal came before the tooltip was declared"
        );
    }

    fn is_shown_in(harness: &Harness<'_, Rig<Host>>) -> bool {
        harness
            .state()
            .nodes
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
        let mut harness = harness_of(view);
        let first = rect_of(&harness, "/\"axis\"/0");
        let second = rect_of(&harness, "/\"axis\"/1");
        assert!(
            first.max.x <= second.min.x,
            "precondition: cells side by side"
        );
        stays_put(
            &mut harness,
            first.center(),
            second.center(),
            "Tip for axis",
        );
    }

    #[test]
    fn leaving_a_cell_with_its_own_tooltip_outward_never_shows_the_controls() {
        let mut harness = harness_of(view);
        let cell = rect_of(&harness, "/\"axis\"/2");
        let control = rect_of(&harness, "/\"axis\"");
        assert!(
            control.max.y > cell.max.y,
            "precondition: a band of the control below the cell"
        );
        hover_node(&mut harness, "/\"axis\"/2");
        wait(&mut harness, DELAY + GRACE);
        assert!(
            painted(&harness, "Tip for current"),
            "precondition: it opened"
        );
        let x = cell.center().x;
        let mut y = cell.center().y;
        let mut crossed = false;
        while y < control.max.y + 12.0 {
            y += 1.0;
            harness.hover_at(pos2(x, y));
            harness.step();
            crossed |= y > cell.max.y && y < control.max.y;
            assert!(!painted(&harness, "Tip for axis"), "at y = {y}");
        }
        assert!(crossed, "precondition: the pointer crossed the band");
        wait(&mut harness, DELAY + GRACE);
        assert!(!painted(&harness, "Tip for axis"), "once outside");
    }

    #[test]
    fn a_cell_with_its_own_tooltip_shows_only_that_one() {
        let mut harness = harness_of(view);
        hover_node(&mut harness, "/\"axis\"/2");
        wait(&mut harness, DELAY + GRACE);
        wait(&mut harness, Duration::from_millis(400));
        assert!(painted(&harness, "Tip for current"));
        assert!(!painted(&harness, "Tip for axis"));
        assert_eq!(boxes(&harness).len(), 1);
    }
}

/// The tooltip boxes shown.
fn boxes(harness: &Harness<'_, Rig<App>>) -> Vec<Rect> {
    tooltip_nodes(harness)
        .into_iter()
        .filter(|(path, _)| path.ends_with("/\"Box\""))
        .map(|(_, rect)| rect)
        .collect()
}

/// Opens `tip` with the pointer at `from`, then moves it to `to` in steps:
/// at each, the one tooltip shown is where it opened.
fn stays_put(harness: &mut Harness<'_, Rig<App>>, from: egui::Pos2, to: egui::Pos2, tip: &str) {
    harness.hover_at(from);
    wait(harness, DELAY + GRACE);
    assert!(painted(harness, tip), "precondition: {tip:?} opened");
    let at = boxes(harness);
    assert_eq!(at.len(), 1, "one tooltip");
    for step in 0..=10 {
        let pointer = from.lerp(to, step as f32 / 10.0);
        harness.hover_at(pointer);
        harness.run_steps(2);
        assert_eq!(boxes(harness), at, "at {pointer:?}");
    }
}

#[test]
fn a_steppers_tooltip_stays_put_from_its_field_to_its_buttons() {
    let mut harness = harness();
    let field = rect_of(&harness, "/\"stepper\"/Box/Field").center();
    let up = rect_of(&harness, "/\"stepper\"/Box/Buttons/Up").center();
    let down = rect_of(&harness, "/\"stepper\"/Box/Buttons/Down").center();
    stays_put(&mut harness, field, up, "Tip for stepper");
    stays_put(&mut harness, up, down, "Tip for stepper");
}

#[test]
fn a_sliders_tooltip_stays_put_from_its_track_to_its_thumb() {
    let mut harness = harness();
    let track = rect_of(&harness, "/\"slider\"/Track");
    let thumb = rect_of(&harness, "/\"slider\"/Track/Thumb");
    let start = pos2(track.min.x + 8.0, track.center().y);
    assert!(
        !thumb.contains(start),
        "precondition: the track, off the thumb"
    );
    stays_put(&mut harness, start, thumb.center(), "Tip for slider");
}

#[test]
fn keyboard_focus_on_a_part_shows_the_controls_tooltip_at_the_control() {
    let mut harness = harness();
    harness.state_mut().companion = support::Companion::Alone;
    let cell = "/\"segmented\"/0";
    for _ in 0..12 {
        if support::focused_node(&harness).as_deref() == Some(cell) {
            break;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some(cell),
        "precondition: Tab reached the cell"
    );
    assert!(painted(&harness, "Tip for segmented"), "no delay for focus");
    let control = rect_of(&harness, "/\"segmented\"");
    let shown = boxes(&harness);
    assert_eq!(shown.len(), 1, "one tooltip");
    assert!(
        (shown[0].center().x - control.center().x).abs() < 1.0,
        "anchored to the control: {shown:?} over {control:?}"
    );
}

#[test]
fn a_part_is_described_by_its_controls_tooltip() {
    let harness = harness();
    let described = |label: &str, tip: &str| {
        harness
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
