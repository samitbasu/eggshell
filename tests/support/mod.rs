//! What the interactive example tests share: a harness that builds, lays out
//! and draws a layout every frame, and pointer helpers that go through
//! kittest's real event path.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses a subset of it"
)]

use bullseye::{Builder, LayoutState, TextStyle};
use egui::epaint::Shape;
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use egui_kittest::Harness;

pub type View<A> = for<'b, 'f> fn(&mut Builder<'b, 'f, A>, &A);

pub struct Rig<A> {
    layout: LayoutState,
    pub app: A,
    view: View<A>,
    /// Every node of the last frame: its readable path and border box.
    pub nodes: Vec<(String, Rect)>,
    ids: Vec<(String, egui::Id)>,
}

/// Whether kittest rejects widgets without an accessible name.
#[derive(Clone, Copy)]
pub enum Accessibility {
    Checked,
    Unchecked,
}

pub fn harness<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> Harness<'static, Rig<A>> {
    harness_with(size, base_text, app, view, Accessibility::Checked)
}

pub fn harness_with<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    accessibility: Accessibility,
) -> Harness<'static, Rig<A>> {
    let rig = Rig {
        layout: LayoutState::new().with_base_text(base_text),
        app,
        view,
        nodes: Vec::new(),
        ids: Vec::new(),
    };
    Harness::builder()
        .with_size(size)
        .with_step_dt(0.05)
        .with_accessibility_check(matches!(accessibility, Accessibility::Checked))
        .build_ui_state(
            |ui, rig: &mut Rig<A>| {
                let area = ui.max_rect();
                let Rig {
                    layout,
                    app,
                    view,
                    nodes,
                    ids,
                } = rig;
                let placed = layout.build(ui.ctx(), |b| view(b, &*app)).layout(area);
                *nodes = placed
                    .nodes()
                    .map(|node| (node.path().to_owned(), node.rect()))
                    .collect();
                *ids = placed
                    .nodes()
                    .filter_map(|node| Some((node.path().to_owned(), node.id()?)))
                    .collect();
                placed.draw(app, ui);
            },
            rig,
        )
}

pub fn rect_of<A>(harness: &Harness<'_, Rig<A>>, path: &str) -> Rect {
    harness
        .state()
        .nodes
        .iter()
        .find(|(candidate, _)| candidate == path)
        .unwrap_or_else(|| panic!("no node {path}: {:?}", harness.state().nodes))
        .1
}

pub fn hover_over<A>(harness: &mut Harness<'_, Rig<A>>, at: Pos2) {
    harness.hover_at(at);
    harness.run_steps(3);
}

pub fn press_and_release<A>(harness: &mut Harness<'_, Rig<A>>, at: Pos2) {
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
}

/// Presses at `from`, moves to `to` in two steps and releases there.
pub fn drag<A>(harness: &mut Harness<'_, Rig<A>>, from: Pos2, to: Pos2) {
    hover_over(harness, from);
    harness.drag_at(from);
    harness.step();
    harness.hover_at(from.lerp(to, 0.5));
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.run_steps(3);
}

/// The fill of the last rect drawn at the top-left corner of the node `path`.
pub fn fill_of<A>(harness: &Harness<'_, Rig<A>>, path: &str) -> Option<Color32> {
    let min = rect_of(harness, path).min;
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if (rect.rect.min - min).length() < 1.0 => Some(rect.fill),
            _ => None,
        })
        .next_back()
}

/// Hovers the centre of the node `path`.
pub fn hover_node<A>(harness: &mut Harness<'_, Rig<A>>, path: &str) {
    let at = rect_of(harness, path).center();
    hover_over(harness, at);
}

/// Clicks the centre of the node `path`.
pub fn click_node<A>(harness: &mut Harness<'_, Rig<A>>, path: &str) {
    let at = rect_of(harness, path).center();
    hover_over(harness, at);
    press_and_release(harness, at);
}

/// What has keyboard focus: the readable path of an identified node, or
/// `<widget>` for an egui widget that is not a node (a text edit inside one).
pub fn focused_path<A>(harness: &Harness<'_, Rig<A>>) -> Option<String> {
    let focused = harness.ctx.memory(|memory| memory.focused())?;
    Some(
        harness
            .state()
            .ids
            .iter()
            .find(|(_, id)| *id == focused)
            .map_or_else(|| "<widget>".to_owned(), |(path, _)| path.clone()),
    )
}
