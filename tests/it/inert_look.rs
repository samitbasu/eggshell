//! Controls inside an `inert()` subtree are drawn in their disabled look, as
//! controls inside a `<fieldset disabled>` are.

use eggshell_ui::owned::Enablement;
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, shapes};

#[derive(Clone, Copy, PartialEq)]
enum Setup {
    Live,
    OwnDisabled,
    InsideInert,
}

#[derive(Default)]
struct App {
    clicks: u32,
    slider: f64,
    setup: Option<Setup>,
}

fn controls(b: &mut Builder<'_, '_, App>, app: &App) {
    let setup = app.setup.expect("a setup");
    let (wrapper, own) = match setup {
        Setup::Live => (tw(), Enablement::Enabled),
        Setup::OwnDisabled => (tw(), Enablement::Disabled),
        Setup::InsideInert => (tw().inert(), Enablement::Enabled),
    };
    b.div(
        wrapper
            .flex()
            .flex_col()
            .items_start()
            .gap(4.0)
            .p(4.0)
            .w_px(300.0),
        |b| {
            b.button("button", "Save")
                .enablement(own)
                .on_click(|app| app.clicks += 1);
            b.checkbox("check", "Check", true)
                .enablement(own)
                .on_change(|app: &mut App, _| app.clicks += 1);
            b.switch("switch", "Switch", true)
                .enablement(own)
                .on_change(|app: &mut App, _| app.clicks += 1);
            b.radio_group("radio", 0, [(0, "One"), (1, "Two")])
                .enablement(own)
                .on_change(|app: &mut App, _| app.clicks += 1);
            b.segmented("seg", 0, [(0, "One"), (1, "Two")])
                .enablement(own)
                .on_change(|app: &mut App, _| app.clicks += 1);
            b.select("select", 0, [(0, "One"), (1, "Two")])
                .named("Pick")
                .enablement(own)
                .on_change(|app: &mut App, _| app.clicks += 1);
            b.slider("slider", app.slider, 0.0..=10.0)
                .named("Level")
                .enablement(own)
                .on_change(|app: &mut App, value| app.slider = value);
            b.stepper("stepper", 1.0, 0.0..=10.0)
                .named("Count")
                .enablement(own)
                .on_change(|app: &mut App, _| app.clicks += 1);
        },
    );
}

fn harness(setup: Setup) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 700.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App {
            setup: Some(setup),
            slider: 5.0,
            ..App::default()
        },
        controls,
    );
    harness.run_steps(10);
    harness
}

#[test]
fn controls_inside_an_inert_subtree_look_disabled() {
    let live = harness(Setup::Live);
    let disabled = harness(Setup::OwnDisabled);
    let inert = harness(Setup::InsideInert);
    assert!(
        shapes(&live) != shapes(&disabled),
        "precondition: the disabled look differs from the live one"
    );
    assert!(
        shapes(&inert) == shapes(&disabled),
        "an inert subtree draws its controls as disabled ones"
    );
}

#[test]
fn and_stay_inert() {
    let mut harness = harness(Setup::InsideInert);
    for path in [
        "/\"button\"",
        "/\"check\"/Box",
        "/\"switch\"/Track",
        "/\"seg\"/1",
        "/\"radio\"/Circle(1)",
        "/\"slider\"/Track",
    ] {
        click_node(&mut harness, path);
    }
    harness.run_steps(3);
    let app = &harness.state().app;
    assert_eq!((app.clicks, app.slider), (0, 5.0));
}
