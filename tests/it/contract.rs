//! `bullseye::Style`: the neutral default, installing and caching, and open
//! keys that never panic a release build.

use std::sync::Arc;

use bullseye::contract::{Keyed, Size, Variant};
use bullseye::owned::Button;
use bullseye::style::Shadcn;
use bullseye::{LayoutState, Style, TextSpec, TextStyle};
use egui::{Color32, Context, FontFamily, Theme, vec2};
use egui_kittest::Harness;

#[test]
fn unknown_keys_fall_back_to_the_default_entry() {
    let keyed = Keyed::new(Variant("plain"), 1).with(Variant("loud"), 2);
    assert_eq!(*keyed.get(Variant("loud")), 2);
    assert_eq!(*keyed.get(Variant("plain")), 1);
    assert_eq!(keyed.default_key(), Variant("plain"));
    assert_eq!(
        keyed.keys().collect::<Vec<_>>(),
        [Variant("plain"), Variant("loud")]
    );

    let unknown = std::panic::catch_unwind(|| *keyed.get(Variant("missing")));
    if cfg!(debug_assertions) {
        assert!(unknown.is_err(), "a debug build flags the unknown key");
    } else {
        assert_eq!(unknown.ok(), Some(1), "a release build falls back");
    }
}

#[test]
fn a_replaced_default_entry_is_what_unknown_keys_get() {
    let mut keyed = Keyed::new(Size("md"), 1);
    keyed.insert(Size("md"), 5);
    assert_eq!(*keyed.get(Size("md")), 5);
    assert!(keyed.get_mut(Size("sm")).is_none());
}

#[test]
fn the_default_style_is_derived_from_egui_and_cached() {
    let ctx = Context::default();
    let style = Style::of(&ctx);
    assert_eq!(*style, Style::from_egui(&ctx.global_style()));
    assert_eq!(*style, Style::default());
    assert!(Arc::ptr_eq(&style, &Style::of(&ctx)), "no rebuild per call");

    let widgets = &ctx.global_style().visuals.widgets;
    let button = style.button.variants.get(Variant::DEFAULT);
    assert_eq!(button.states.rest.fill, widgets.inactive.weak_bg_fill);
    assert_eq!(button.states.hover.fill, widgets.hovered.weak_bg_fill);
    assert_eq!(button.states.pressed.fill, widgets.active.weak_bg_fill);

    ctx.all_styles_mut(|egui| egui.visuals.widgets.inactive.weak_bg_fill = Color32::RED);
    let changed = Style::of(&ctx);
    assert!(!Arc::ptr_eq(&style, &changed));
    assert_eq!(
        changed
            .button
            .variants
            .get(Variant::DEFAULT)
            .states
            .rest
            .fill,
        Color32::RED,
        "the derived style follows egui's"
    );
}

#[test]
fn an_installed_style_is_shared_and_can_differ_per_theme() {
    let ctx = Context::default();
    let mut light = Style::default();
    light.focus.width = 7.0;
    let mut dark = Style::default();
    dark.focus.width = 9.0;
    light.install_for(&ctx, Theme::Light);
    dark.install_for(&ctx, Theme::Dark);
    ctx.set_theme(Theme::Light);
    let seen = Style::of(&ctx);
    assert_eq!(seen.focus.width, 7.0);
    assert!(Arc::ptr_eq(&seen, &Style::of(&ctx)));
    ctx.set_theme(Theme::Dark);
    assert_eq!(Style::of(&ctx).focus.width, 9.0);

    let mut both = Style::default();
    both.focus.width = 4.0;
    both.install(&ctx);
    assert_eq!(Style::of(&ctx).focus.width, 4.0);
    ctx.set_theme(Theme::Light);
    assert_eq!(Style::of(&ctx).focus.width, 4.0);
}

#[test]
fn applying_the_preset_installs_its_style() {
    let ctx = Context::default();
    ctx.set_theme(Theme::Dark);
    Shadcn::dark().apply(&ctx);
    assert_eq!(*Style::of(&ctx), Shadcn::dark().style());
}

#[test]
fn core_draws_a_button_with_no_preset() {
    let ctx = Context::default();
    let expected = Style::of(&ctx);
    let mut layout = LayoutState::new().with_base_text(TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    ));
    let mut harness = Harness::builder()
        .with_size(vec2(200.0, 100.0))
        .build_ui(move |ui| {
            let area = ui.max_rect();
            layout
                .build(ui.ctx(), |b| {
                    Button::new("Plain").show(b, "plain", |_: &mut ()| {})
                })
                .layout(area)
                .draw(&mut (), ui);
        });
    harness.run_steps(3);
    let button = expected.button.variants.get(Variant::DEFAULT);
    let fills: Vec<_> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(r) => Some(r.fill),
            _ => None,
        })
        .collect();
    assert!(fills.contains(&button.states.rest.fill), "{fills:?}");
}
