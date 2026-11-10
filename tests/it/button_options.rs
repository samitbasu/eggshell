//! `ButtonOptions`: shared button settings, and the helper-function pattern
//! that returns a chain for the caller to finish.

use bullseye::contract::Size;
use bullseye::owned::{ButtonChain, ButtonOptions};
use bullseye::{Builder, LayoutState, Scope, tw};
use bullseye_shadcn::{Shadcn, size, variant};
use egui::epaint::ClippedShape;
use egui::{RawInput, Rect, pos2};

const SCREEN: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(400.0, 200.0),
};

#[derive(Default)]
struct App {
    clicks: u32,
}

#[derive(Hash, Debug, Clone, Copy)]
enum Part {
    Save,
    Send,
}

fn shapes(view: impl for<'f> FnOnce(&mut Builder<'_, 'f, App>)) -> Vec<ClippedShape> {
    let ctx = egui::Context::default();
    Shadcn::apply_both(&ctx);
    let input = RawInput {
        screen_rect: Some(SCREEN),
        ..RawInput::default()
    };
    ctx.run_ui(input.clone(), |_| {})
        .drop_without_applying_deltas();
    let mut view = Some(view);
    let mut app = App::default();
    let mut output = ctx.run_ui(input, |ui| {
        let view = view.take().expect("one pass");
        LayoutState::new()
            .build(ui.ctx(), view)
            .layout(SCREEN)
            .draw(&mut app, ui);
    });
    output.textures_delta.clear();
    output.shapes
}

fn one_button(
    options: impl for<'a, 'f> FnOnce(ButtonChain<'a, 'f, App>) -> ButtonChain<'a, 'f, App>,
) -> Vec<ClippedShape> {
    let shapes = shapes(|b| {
        b.div(tw().flex().items_start(), |b| {
            options(b.button(Part::Save, "Save")).add();
        });
    });
    assert!(!shapes.is_empty(), "the button draws something");
    shapes
}

fn verb() -> ButtonOptions {
    ButtonOptions::new()
        .variant(variant::OUTLINE)
        .size(size::ICON)
        .classes(tw().w_px(120.0))
        .named("Do it")
        .tooltip("Does it")
}

fn toggle_verb() -> ButtonOptions {
    ButtonOptions::new()
        .size(size::ICON)
        .classes(tw().w_px(120.0))
        .named("Do it")
        .tooltip("Does it")
}

fn same(left: &[ClippedShape], right: &[ClippedShape], what: &str) {
    assert!(left == right, "{what}: the shapes differ");
}

fn differ(left: &[ClippedShape], right: &[ClippedShape], what: &str) {
    assert!(left != right, "{what}: the shapes are the same");
}

#[test]
fn shared_options_equal_the_same_settings_inline() {
    let shared = shapes(|b| {
        b.button(Part::Save, "Save").with_options(verb()).add();
        b.button(Part::Send, "Send").with_options(verb()).add();
    });
    let inline = shapes(|b| {
        for (key, label) in [(Part::Save, "Save"), (Part::Send, "Send")] {
            b.button(key, label)
                .variant(variant::OUTLINE)
                .size(size::ICON)
                .classes(tw().w_px(120.0))
                .named("Do it")
                .tooltip("Does it")
                .add();
        }
    });
    let plain = shapes(|b| {
        b.button(Part::Save, "Save").add();
        b.button(Part::Send, "Send").add();
    });
    differ(&shared, &plain, "the options change how the buttons look");
    same(&shared, &inline, "shared options match inline settings");
}

#[test]
fn a_size_set_after_the_options_wins_and_one_set_before_them_loses_to_them() {
    let icon = one_button(|c| c.size(size::ICON));
    let default = one_button(|c| c.size(Size::DEFAULT));
    differ(&icon, &default, "the two sizes differ");
    let without_size = ButtonOptions::new().variant(variant::OUTLINE);
    let with_size = ButtonOptions::new()
        .variant(variant::OUTLINE)
        .size(size::ICON);
    let outline_icon = one_button(|c| c.variant(variant::OUTLINE).size(size::ICON));

    same(
        &one_button(|c| c.with_options(without_size.clone()).size(size::ICON)),
        &outline_icon,
        "a size after options without one",
    );
    same(
        &one_button(|c| c.size(size::ICON).with_options(without_size.clone())),
        &outline_icon,
        "a size before options without one",
    );
    same(
        &one_button(|c| c.size(Size::DEFAULT).with_options(with_size.clone())),
        &outline_icon,
        "options applied after a size override it",
    );
    same(
        &one_button(|c| c.with_options(with_size).size(Size::DEFAULT)),
        &one_button(|c| c.variant(variant::OUTLINE)),
        "a size set after the options overrides them",
    );
}

#[test]
fn merging_options_lets_the_later_ones_win_and_merges_classes() {
    let merged = ButtonOptions::new()
        .size(size::ICON)
        .classes(tw().w_px(120.0))
        .merge(
            ButtonOptions::new()
                .size(Size::DEFAULT)
                .classes(tw().h_px(40.0)),
        );
    let by_hand = ButtonOptions::new()
        .size(Size::DEFAULT)
        .classes(tw().w_px(120.0).h_px(40.0));
    same(
        &one_button(|c| c.with_options(merged)),
        &one_button(|c| c.with_options(by_hand)),
        "merged options",
    );
}

#[test]
fn a_toggle_takes_options_too() {
    let shared = shapes(|b| {
        b.toggle("t", "Bold", false)
            .with_options(toggle_verb())
            .on_change(|_, _| {});
    });
    let inline = shapes(|b| {
        b.toggle("t", "Bold", false)
            .size(size::ICON)
            .classes(tw().w_px(120.0))
            .named("Do it")
            .tooltip("Does it")
            .on_change(|_, _| {});
    });
    assert!(!shared.is_empty());
    same(&shared, &inline, "shared options match inline settings");
}

fn labelled<'a, 'f>(s: &'a mut Scope<'_, 'f, App>, key: Part) -> ButtonChain<'a, 'f, App> {
    s.button(key, "Go")
        .size(Size::DEFAULT)
        .named("Go now")
        .tooltip("Goes")
}

#[test]
fn a_helper_returns_a_chain_for_its_caller_to_finish() {
    let shapes = shapes(|b| {
        b.scope("row", |s| {
            labelled(s, Part::Save).on_click(|app| app.clicks += 1);
        });
        let make =
            |s: &mut Scope<'_, '_, App>| labelled(s, Part::Send).variant(variant::GHOST).add();
        b.scope("again", make);
    });
    assert!(!shapes.is_empty());
}
