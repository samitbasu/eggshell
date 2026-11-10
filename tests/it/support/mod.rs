//! What the interactive example tests share: a harness that builds, lays out
//! and draws a layout every frame, and pointer helpers that go through
//! kittest's real event path.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses a subset of it"
)]

use bullseye::{Builder, LayoutState, TextStyle};
use egui::epaint::{RectShape, Shape, StrokeKind};
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use egui_kittest::Harness;

pub type View<A> = for<'b, 'f> fn(&mut Builder<'b, 'f, A>, &A);

/// What the rig paints under the layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    /// Nothing: the harness's own clear colour shows.
    #[default]
    Bare,
    /// The theme's `background` over the whole window, as the examples' central
    /// panel does.
    Themed,
}

pub struct Rig<A> {
    pub page: Page,
    layout: LayoutState,
    pub app: A,
    /// The `scope_id` of the `Ui` the layout is drawn into.
    pub scope: egui::Id,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
    /// Every node of the last frame: its readable path and border box.
    pub nodes: Vec<(String, Rect)>,
    ids: Vec<(String, egui::Id)>,
}

pub fn harness<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> Harness<'static, Rig<A>> {
    harness_prepared(size, base_text, app, view, |_, _| {})
}

/// [`harness`] for an app that sets egui's style: `prepare` runs before
/// every frame is built.
pub fn harness_prepared<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
) -> Harness<'static, Rig<A>> {
    harness_scaled(size, 1.0, base_text, app, view, prepare)
}

/// [`harness`] whose frames are `step_dt` seconds apart.
pub fn harness_stepped<A: 'static>(
    size: Vec2,
    step_dt: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> Harness<'static, Rig<A>> {
    harness_timed(size, 1.0, step_dt, base_text, app, view, |_, _| {})
}

/// [`harness_prepared`] on a display of `pixels_per_point`; `size` is in
/// points.
pub fn harness_scaled<A: 'static>(
    size: Vec2,
    pixels_per_point: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
) -> Harness<'static, Rig<A>> {
    harness_timed(size, pixels_per_point, 0.05, base_text, app, view, prepare)
}

fn harness_timed<A: 'static>(
    size: Vec2,
    pixels_per_point: f32,
    step_dt: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
) -> Harness<'static, Rig<A>> {
    let rig = Rig {
        page: Page::default(),
        layout: LayoutState::new().with_base_text(base_text),
        app,
        scope: egui::Id::NULL,
        view,
        prepare,
        nodes: Vec::new(),
        ids: Vec::new(),
    };
    Harness::builder()
        .with_size(size)
        .with_pixels_per_point(pixels_per_point)
        .with_step_dt(step_dt)
        .with_accessibility_check(true)
        .build_ui_state(
            |ui, rig: &mut Rig<A>| {
                let area = ui.max_rect();
                let Rig {
                    page,
                    layout,
                    app,
                    scope,
                    view,
                    prepare,
                    nodes,
                    ids,
                } = rig;
                *scope = ui.scope_id();
                if bullseye_shadcn::tokens(ui.ctx()).is_none() {
                    bullseye_shadcn::Shadcn::apply_both(ui.ctx());
                }
                prepare(ui.ctx(), &*app);
                if *page == Page::Themed {
                    let background = bullseye_shadcn::Shadcn::of(ui.ctx()).tokens.background;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, background);
                }
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

/// The deepest node that has keyboard focus or holds a widget that does: the
/// leaf a `text_field` or `select` sits in, or a node that is itself focused.
pub fn focused_node<A>(harness: &Harness<'_, Rig<A>>) -> Option<String> {
    harness
        .state()
        .ids
        .iter()
        .rfind(|(_, id)| bullseye::focus_within(&harness.ctx, *id))
        .map(|(path, _)| path.clone())
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

pub fn flatten<'a>(shape: &'a Shape, into: &mut Vec<&'a Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| flatten(shape, into)),
        shape => into.push(shape),
    }
}

pub fn shapes<'a, A>(harness: &'a Harness<'_, Rig<A>>) -> Vec<&'a Shape> {
    let mut all = Vec::new();
    for clipped in &harness.output().shapes {
        flatten(&clipped.shape, &mut all);
    }
    all
}

pub fn same(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 0.6 && (a.max - b.max).length() < 0.6
}

/// The last box painted over exactly `rect`, ring excluded.
pub fn box_at<A>(harness: &Harness<'_, Rig<A>>, rect: Rect) -> RectShape {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if same(r.rect, rect) && r.stroke_kind == StrokeKind::Inside => {
                Some(r.clone())
            }
            _ => None,
        })
        .next_back()
        .unwrap_or_else(|| panic!("no box painted at {rect:?}"))
}

pub fn ring_at<A>(harness: &Harness<'_, Rig<A>>, rect: Rect) -> Option<RectShape> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if same(r.rect, rect) && r.stroke_kind == StrokeKind::Outside => {
                Some(r.clone())
            }
            _ => None,
        })
        .next_back()
}

pub fn text_color<A>(harness: &Harness<'_, Rig<A>>, inside: Rect, text: &str) -> Color32 {
    shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == text && inside.contains(t.pos) => {
                Some(t.override_text_color.unwrap_or(t.fallback_color))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text:?} in {inside:?}"))
}

pub fn press<A>(harness: &mut Harness<'_, Rig<A>>, path: &str) {
    let at = rect_of(harness, path).center();
    harness.hover_at(at);
    harness.run_steps(2);
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(12);
}

pub fn release<A>(harness: &mut Harness<'_, Rig<A>>, path: &str) {
    let at = rect_of(harness, path).center();
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(12);
}
