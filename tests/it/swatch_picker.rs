//! The swatch picker example through the real path: kittest's keys and
//! pointer, build, layout, draw and `Nav::Grid`'s judgement of the arrows by
//! the swatches' placed rects.

use eggshell_ui::overlay::{Anchor, Nav, Wrap};
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Key, Role, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, focused_path};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/swatch_picker.rs"]
mod example;

use example::{COLOURS, Choice, State, layout};

const TRIGGER: &str = "/Page/Trigger";

fn harness(choice: Choice) -> Harness<'static, Rig<State>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        State::with(choice),
        layout,
    );
    harness.state_mut().companion = support::Companion::Alone;
    harness.run_steps(4);
    harness
}

fn press(harness: &mut Harness<'_, Rig<State>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(5);
}

fn tab_to_trigger(harness: &mut Harness<'_, Rig<State>>) {
    for _ in 0..4 {
        if focused_path(harness).as_deref() == Some(TRIGGER) {
            return;
        }
        press(harness, Key::Tab);
    }
    panic!("could not tab to {TRIGGER}");
}

/// The swatch that has focus.
fn focused(harness: &Harness<'_, Rig<State>>) -> Option<Choice> {
    let path = focused_path(harness)?;
    let swatch = path.rsplit('/').next()?;
    let all = std::iter::once(Choice::NoColour).chain((0..COLOURS.len()).map(Choice::Colour));
    all.into_iter()
        .find(|choice| swatch == format!("Swatch({choice:?})"))
}

fn open_from_the_keyboard(choice: Choice) -> Harness<'static, Rig<State>> {
    let mut harness = harness(choice);
    tab_to_trigger(&mut harness);
    press(&mut harness, Key::Enter);
    assert!(harness.state().app.open, "precondition: Enter opened it");
    harness
}

#[test]
fn it_opens_with_focus_on_the_current_colour() {
    for choice in [Choice::Colour(8), Choice::NoColour, Choice::Colour(11)] {
        let harness = open_from_the_keyboard(choice);
        assert_eq!(focused(&harness), Some(choice));
    }
}

#[test]
fn a_click_opens_it_with_focus_on_the_current_colour_too() {
    let mut harness = harness(Choice::Colour(5));
    click_node(&mut harness, TRIGGER);
    harness.run_steps(5);
    assert!(
        harness.state().app.open,
        "precondition: the click opened it"
    );
    assert_eq!(focused(&harness), Some(Choice::Colour(5)));
}

#[test]
fn the_arrows_follow_the_layout_and_left_crosses_to_no_colour() {
    let mut harness = open_from_the_keyboard(Choice::Colour(5));
    for (key, to) in [
        (Key::ArrowRight, Choice::Colour(6)),
        (Key::ArrowDown, Choice::Colour(10)),
        (Key::ArrowLeft, Choice::Colour(9)),
        (Key::ArrowLeft, Choice::Colour(8)),
        (Key::ArrowLeft, Choice::NoColour),
        (Key::ArrowRight, Choice::Colour(0)),
        (Key::ArrowUp, Choice::Colour(0)),
        (Key::End, Choice::Colour(11)),
        (Key::ArrowRight, Choice::Colour(11)),
        (Key::ArrowDown, Choice::Colour(11)),
        (Key::Home, Choice::NoColour),
    ] {
        press(&mut harness, key);
        assert_eq!(focused(&harness), Some(to), "{key:?}");
    }
    assert!(harness.state().app.open, "the arrows do not close it");
}

#[test]
fn enter_picks_the_focused_colour_and_closes() {
    let mut harness = open_from_the_keyboard(Choice::Colour(8));
    press(&mut harness, Key::ArrowUp);
    assert_eq!(focused(&harness), Some(Choice::Colour(4)), "precondition");
    press(&mut harness, Key::Enter);
    harness.run_steps(10);
    let app = &harness.state().app;
    assert_eq!(app.choice, Choice::Colour(4));
    assert!(!app.open);
    assert_eq!(focused_path(&harness).as_deref(), Some(TRIGGER));
}

#[test]
fn space_picks_too() {
    let mut harness = open_from_the_keyboard(Choice::NoColour);
    press(&mut harness, Key::ArrowRight);
    press(&mut harness, Key::Space);
    assert_eq!(harness.state().app.choice, Choice::Colour(0));
}

#[derive(Default)]
struct Plain;

fn grid_overlay(b: &mut Builder<'_, '_, Plain>, wrap: Option<Wrap>, marked: &[u32]) {
    let marked = marked.to_vec();
    b.div_id("anchor", tw().size_px(10.0), |_| {});
    let anchor = b.id_of("anchor");
    let chain = b
        .overlay("grid")
        .anchor(Anchor::Node(anchor))
        .nav(Nav::Grid)
        .role(Role::Grid)
        .named("Grid");
    let chain = match wrap {
        Some(wrap) => chain.wrap(wrap),
        None => chain,
    };
    chain.children(|s| {
        for cell in 0..3u32 {
            let node = s
                .div_id(("cell", cell), tw().size_px(20.0).sense_click(), |_| {})
                .role(Role::GridCell)
                .named(format!("Cell {cell}"));
            if marked.contains(&cell) {
                node.focus_on_open();
            }
        }
    });
}

fn plain_harness(view: support::View<Plain>) -> Harness<'static, Rig<Plain>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(200.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        Plain,
        view,
    );
    harness.run_steps(4);
    harness
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "a grid overlay stops at its edges and cannot wrap around")]
fn a_grid_told_to_wrap_around_is_a_build_error() {
    plain_harness(|b, _| grid_overlay(b, Some(Wrap::Around), &[]));
}

#[test]
fn a_grid_told_to_stop_builds() {
    plain_harness(|b, _| grid_overlay(b, Some(Wrap::Stop), &[1]));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "are marked `focus_on_open`")]
fn two_nodes_marked_focus_on_open_in_one_overlay_panic() {
    plain_harness(|b, _| grid_overlay(b, None, &[0, 2]));
}
