//! Tooltips on the owned controls: shown through egui's `on_hover_text`,
//! also when the control is disabled, and in addition to the accessible name.

use eggshell_ui::owned::{Choice, Enablement};
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, hover_node};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pick {
    A,
    B,
}

const PICKS: [(Pick, &str); 2] = [(Pick::A, "Alpha"), (Pick::B, "Beta")];

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
fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .w_px(300.0),
        |b| {
            b.button("undo", "")
                .named("Undo")
                .tooltip("Undo (Ctrl+Z)")
                .add();
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

fn harness() -> Harness<'static, Rig<()>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        view,
    );
    harness.ctx.all_styles_mut(|style| {
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
    });
    harness.run_steps(12);
    harness
}

fn hover_shows(path: &str, tip: &str) {
    let mut harness = harness();
    assert!(
        harness.query_by_label(tip).is_none(),
        "precondition: {tip:?} is not shown before hovering"
    );
    hover_node(&mut harness, path);
    harness.run_steps(6);
    assert!(
        harness.query_by_label(tip).is_some(),
        "hovering {path} shows {tip:?}"
    );
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
    harness.run_steps(6);
    assert!(harness.query_by_label("Tip for beta").is_none());
}

#[test]
fn a_plain_node_shows_the_tooltip_set_on_its_handle() {
    hover_shows("/\"crumb\"", "Tip for a span");
}

#[test]
fn a_tooltip_composes_with_another_handler_on_the_same_node() {
    hover_shows("/\"tile\"", "Tip for a div");
}
