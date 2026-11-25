//! What the interactive example tests share: a harness that builds, lays out
//! and draws a layout every frame, and pointer helpers that go through
//! kittest's real event path.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses a subset of it"
)]

pub mod button;

use core::time::Duration;
use std::sync::Arc;

#[path = "../../../examples/common/mod.rs"]
mod common;

use eggshell_ui::contract::Themed;
use eggshell_ui::testing::{After, Driver, DriverBuilder, PlacedFrame};
use eggshell_ui::{Builder, LayoutState, Shell, TextStyle};
use egui::epaint::{RectShape, Shape, StrokeKind};
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use egui_kittest::Harness;

/// What else shares the egui context with the layout under test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Companion {
    /// A second `LayoutState` laid out before the view every frame, as the
    /// examples' theme bar is: library state kept per context instead of per
    /// layout shows up as a failure.
    #[default]
    Bar,
    /// The view alone, for pixel-exact goldens.
    Alone,
}

/// A small layout of focusable controls, laid out and drawn just above the
/// window so that it takes part in focus, Tab order and every per-frame pass
/// without painting over or catching the pointer of the view under test.
pub struct Bar {
    layout: LayoutState,
    pub last_stop: Option<egui::Id>,
}

/// A rig that lays a view out with a [`LayoutState`] of its own.
pub trait Shelled {
    fn layout_state(&self) -> &LayoutState;
}

impl<A> Shelled for Rig<A> {
    fn layout_state(&self) -> &LayoutState {
        &self.layout
    }
}

impl<A> Shelled for FrameRig<A> {
    fn layout_state(&self) -> &LayoutState {
        &self.layout
    }
}

/// The state of the view under test, for requests and reads made of it.
pub fn shell<S: Shelled>(harness: &Harness<'_, S>) -> Shell {
    harness.state().layout_state().shell(&harness.ctx)
}

/// What a test reads of the last frame, from a rig's harness or a driver.
pub trait Seen {
    fn context(&self) -> &egui::Context;
    fn last_output(&self) -> &egui::FullOutput;
    /// Every node of the last frame: its readable path, border box and id.
    fn placed(&self) -> Vec<(String, Rect, Option<egui::Id>)>;
    fn view_shell(&self) -> Shell;
}

impl<A> Seen for Harness<'_, Rig<A>> {
    fn context(&self) -> &egui::Context {
        &self.ctx
    }

    fn last_output(&self) -> &egui::FullOutput {
        self.output()
    }

    fn placed(&self) -> Vec<(String, Rect, Option<egui::Id>)> {
        let ids = &self.state().ids;
        self.state()
            .nodes
            .iter()
            .map(|(path, rect)| {
                let id = ids.iter().find(|(p, _)| p == path).map(|(_, id)| *id);
                (path.clone(), *rect, id)
            })
            .collect()
    }

    fn view_shell(&self) -> Shell {
        shell(self)
    }
}

impl<A> Seen for Driver<A> {
    fn context(&self) -> &egui::Context {
        self.ctx()
    }

    fn last_output(&self) -> &egui::FullOutput {
        self.harness().output()
    }

    fn placed(&self) -> Vec<(String, Rect, Option<egui::Id>)> {
        self.frame()
            .nodes()
            .map(|node| (node.path().to_owned(), node.rect(), node.id()))
            .collect()
    }

    fn view_shell(&self) -> Shell {
        self.shell()
    }
}

impl<S: Seen> Seen for &S {
    fn context(&self) -> &egui::Context {
        (**self).context()
    }

    fn last_output(&self) -> &egui::FullOutput {
        (**self).last_output()
    }

    fn placed(&self) -> Vec<(String, Rect, Option<egui::Id>)> {
        (**self).placed()
    }

    fn view_shell(&self) -> Shell {
        (**self).view_shell()
    }
}

/// Every node of the last frame: its readable path and border box.
pub fn nodes<S: Seen>(seen: &S) -> Vec<(String, Rect)> {
    seen.placed()
        .into_iter()
        .map(|(path, rect, _)| (path, rect))
        .collect()
}

/// A rig that carries a [`Bar`].
pub trait Barred {
    fn bar(&self) -> &Bar;
}

impl<A> Barred for Rig<A> {
    fn bar(&self) -> &Bar {
        &self.bar
    }
}

impl<A> Barred for FrameRig<A> {
    fn bar(&self) -> &Bar {
        &self.bar
    }
}

/// Puts focus on the bar's last Tab stop, so that the next Tab enters the
/// view's first: a test that Tabs "from nothing" starts here, as a user who
/// has Tabbed past the app's bar does. Does nothing when the rig has no bar.
pub fn past_bar<S: Barred>(harness: &mut Harness<'_, S>) {
    if let Some(stop) = harness.state().bar().last_stop {
        harness.ctx.memory_mut(|memory| memory.request_focus(stop));
        harness.run_steps(2);
    }
}

/// [`past_bar`] for a driver.
pub fn drive_past_bar<A>(d: &mut Driver<A>) {
    if let Some(stop) = bar_last_stop(d) {
        d.ctx().memory_mut(|memory| memory.request_focus(stop));
        d.frames(2);
    }
}

/// The bar's last Tab stop, once a frame has drawn the bar.
pub fn bar_last_stop<A>(d: &Driver<A>) -> Option<egui::Id> {
    d.ctx()
        .data(|data| data.get_temp::<egui::Id>(last_stop_id()))
}

fn last_stop_id() -> egui::Id {
    egui::Id::NULL.with("support-bar-last-stop")
}

impl Bar {
    pub fn new(base_text: TextStyle) -> Self {
        Self {
            layout: LayoutState::named("rig-bar").with_base_text(base_text),
            last_stop: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.layout.set_style(common::style(ui.ctx()));
        let window = ui.max_rect();
        let area = Rect::from_min_size(
            Pos2::new(window.min.x, window.min.y - BAR_HEIGHT),
            Vec2::new(window.width(), BAR_HEIGHT),
        );
        let placed = self
            .layout
            .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                b.div(eggshell_ui::tw().flex().items_center().gap(3.0), |b| {
                    b.segmented("bar-segmented", 0u8, [(0u8, "One"), (1, "Two")])
                        .on_change(|_: &mut (), _| {});
                    b.button("bar-button", "Bar").on_click(|_: &mut ()| {});
                });
            })
            .layout(area);
        self.last_stop = placed.nodes().filter_map(|node| node.id()).last();
        if let Some(stop) = self.last_stop {
            ui.ctx()
                .data_mut(|data| data.insert_temp(last_stop_id(), stop));
        }
        placed.draw(&mut (), ui);
    }
}

const BAR_HEIGHT: f32 = 44.0;

/// The style a rig's layout builds with unless it `prepare`s its own: shadcn's
/// default, light and dark.
pub fn shadcn() -> Themed<Arc<eggshell_ui::Style>> {
    eggshell_ui::theme::Shadcn::themed(
        eggshell_ui::theme::ShadcnStyle::default(),
        eggshell_ui::theme::Palette::new(),
    )
}

/// [`shadcn`] with `edit` made to each half.
pub fn shadcn_with(edit: impl Fn(&mut eggshell_ui::Style)) -> Themed<Arc<eggshell_ui::Style>> {
    shadcn().map(|half| {
        let mut style = (*half).clone();
        edit(&mut style);
        Arc::new(style)
    })
}

/// `style` without its preset's generated looks: only the widgets' own rules
/// its tokens give.
pub fn sections_of(style: eggshell_ui::Style) -> eggshell_ui::Style {
    eggshell_ui::Style::new(&style.tokens)
}

pub type View<A> = for<'b, 'f> fn(&mut Builder<'b, 'f, A>, &'f A);

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

/// Puts egui's animation time back where kittest zeroes it: with a zero one
/// every eggshell motion is switched off, which is for the tests of that.
pub fn full_motion(ctx: &egui::Context) {
    if ctx.global_style().animation_time == 0.0 {
        ctx.global_style_mut(|style| style.animation_time = 1.0 / 12.0);
    }
}

pub struct Rig<A> {
    pub page: Page,
    pub companion: Companion,
    pub bar: Bar,
    pub layout: LayoutState,
    pub app: A,
    view: View<A>,
    /// Sets egui's style before each frame is built, and with it the layout
    /// takes the examples' chosen style; without one the layout keeps the
    /// style it was given.
    prepare: Option<fn(&egui::Context, &A)>,
    /// Every node of the last frame: its readable path and border box.
    pub nodes: Vec<(String, Rect)>,
    ids: Vec<(String, egui::Id)>,
}

pub fn harness<A>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> Harness<'static, Rig<A>> {
    harness_timed(size, 1.0, 0.05, base_text, app, view, None)
}

/// [`harness`] for an app that sets egui's style: `prepare` runs before
/// every frame is built, and the layout takes the examples' chosen style.
pub fn harness_prepared<A>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
) -> Harness<'static, Rig<A>> {
    harness_scaled(size, 1.0, base_text, app, view, prepare)
}

/// [`harness`] whose frames are `step_dt` seconds apart.
pub fn harness_stepped<A>(
    size: Vec2,
    step_dt: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> Harness<'static, Rig<A>> {
    harness_timed(size, 1.0, step_dt, base_text, app, view, None)
}

/// [`harness`] on a display of `pixels_per_point`; `size` is in points.
pub fn harness_at<A>(
    size: Vec2,
    pixels_per_point: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> Harness<'static, Rig<A>> {
    harness_timed(size, pixels_per_point, 0.05, base_text, app, view, None)
}

/// [`harness_prepared`] on a display of `pixels_per_point`; `size` is in
/// points.
pub fn harness_scaled<A>(
    size: Vec2,
    pixels_per_point: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
) -> Harness<'static, Rig<A>> {
    harness_timed(
        size,
        pixels_per_point,
        0.05,
        base_text,
        app,
        view,
        Some(prepare),
    )
}

fn harness_timed<A>(
    size: Vec2,
    pixels_per_point: f32,
    step_dt: f32,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: Option<fn(&egui::Context, &A)>,
) -> Harness<'static, Rig<A>> {
    let rig = Rig {
        page: Page::default(),
        companion: Companion::default(),
        bar: Bar::new(base_text.clone()),
        layout: LayoutState::new()
            .with_base_text(base_text)
            .with_style(shadcn()),
        app,
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
                    companion,
                    bar,
                    layout,
                    app,
                    view,
                    prepare,
                    nodes,
                    ids,
                } = rig;
                full_motion(ui.ctx());
                if eggshell_ui::theme::tokens(ui.ctx()).is_none() {
                    eggshell_ui::theme::Shadcn::apply_both(ui.ctx());
                }
                if let Some(prepare) = prepare {
                    prepare(ui.ctx(), &*app);
                    layout.set_style(common::style(ui.ctx()));
                }
                if *page == Page::Themed {
                    let background = eggshell_ui::theme::Shadcn::of(ui.ctx()).tokens.background;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, background);
                }
                if *companion == Companion::Bar {
                    bar.show(ui);
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

/// The border box of the first node whose readable path is `path`: an
/// anonymous path may repeat, which a driver's unique `Target::suffix`
/// refuses.
pub fn rect_of<S: Seen>(seen: &S, path: &str) -> Rect {
    let nodes = nodes(seen);
    nodes
        .iter()
        .find(|(candidate, _)| candidate == path)
        .unwrap_or_else(|| panic!("no node {path}: {nodes:?}"))
        .1
}

/// The egui id of the identified node `path`.
pub fn id_of<S: Seen>(seen: &S, path: &str) -> egui::Id {
    seen.placed()
        .into_iter()
        .find_map(|(candidate, _, id)| (candidate == path).then_some(id).flatten())
        .unwrap_or_else(|| panic!("no identified node {path}"))
}

pub fn hover_over<A>(harness: &mut Harness<'_, Rig<A>>, at: Pos2) {
    harness.hover_at(at);
    harness.run_steps(3);
}

/// What a test steps, a rig's harness or a driver: one event in a frame
/// of its own, or frames alone.
pub trait Drives {
    fn send(&mut self, event: Event);
    fn run(&mut self, frames: usize);
    fn frame_dt(&self) -> f32;
}

impl<A> Drives for Harness<'_, Rig<A>> {
    fn send(&mut self, event: Event) {
        self.event(event);
        self.step();
    }

    fn run(&mut self, frames: usize) {
        self.run_steps(frames);
    }

    fn frame_dt(&self) -> f32 {
        self.ctx.input(|input| input.predicted_dt)
    }
}

impl<A> Drives for Driver<A> {
    fn send(&mut self, event: Event) {
        self.event(event);
    }

    fn run(&mut self, frames: usize) {
        self.frames(frames);
    }

    fn frame_dt(&self) -> f32 {
        self.ctx().input(|input| input.predicted_dt)
    }
}

/// A primary press and release at `at`, a frame each, without resting there
/// first.
pub fn press_and_release<D: Drives>(d: &mut D, at: Pos2) {
    for pressed in [true, false] {
        d.send(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
}

/// The fill of the last rect drawn at the top-left corner of the node `path`.
pub fn fill_of<S: Seen>(seen: &S, path: &str) -> Option<Color32> {
    let min = seen
        .placed()
        .into_iter()
        .find(|(candidate, _, _)| candidate == path)
        .unwrap_or_else(|| panic!("no node {path}"))
        .1
        .min;
    seen.last_output()
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

/// Leaves the pointer resting until a tooltip it opens has entered: the
/// style's delay and entry motion as frame time, then a few frames for the
/// overlay to be built and placed.
pub fn rest_for_tooltip<D: Drives + Seen>(harness: &mut D) {
    let style = harness.view_shell().style();
    let wait = (style.tooltip.delay + style.motion.tooltip.enter).as_secs_f64();
    let start = harness.context().input(|input| input.time);
    for _ in 0..10_000 {
        if harness.context().input(|input| input.time) - start >= wait {
            break;
        }
        harness.run(1);
    }
    harness.run(4);
}

/// Clicks twice at `at`, one event a frame as a browser sends them, the
/// releases `apart` apart.
pub fn click_twice<D: Drives>(harness: &mut D, at: egui::Pos2, apart: std::time::Duration) {
    let frame = harness.frame_dt();
    let frames = (apart.as_secs_f32() / frame).round() as usize;
    assert!(
        frames >= 2,
        "precondition: a press and a release fit in {apart:?}"
    );
    press_and_release(harness, at);
    harness.run(frames - 2);
    press_and_release(harness, at);
    harness.run(6);
}

/// Clicks the centre of the node `path`.
pub fn click_node<A>(harness: &mut Harness<'_, Rig<A>>, path: &str) {
    let at = rect_of(harness, path).center();
    hover_over(harness, at);
    press_and_release(harness, at);
}

/// The deepest node that has keyboard focus or holds a widget that does: the
/// leaf a `text_field` or `select` sits in, or a node that is itself focused.
pub fn focused_node<S: Seen>(seen: &S) -> Option<String> {
    let shell = seen.view_shell();
    seen.placed()
        .into_iter()
        .filter_map(|(path, _, id)| Some((path, id?)))
        .rfind(|(_, id)| shell.focus_within(*id))
        .map(|(path, _)| path)
}

/// What has keyboard focus: the readable path of an identified node, or
/// `<widget>` for an egui widget that is not a node (a text edit inside one).
pub fn focused_path<S: Seen>(seen: &S) -> Option<String> {
    let focused = seen.context().memory(|memory| memory.focused())?;
    Some(
        seen.placed()
            .into_iter()
            .find(|(_, _, id)| *id == Some(focused))
            .map_or_else(|| "<widget>".to_owned(), |(path, _, _)| path),
    )
}

pub fn flatten<'a>(shape: &'a Shape, into: &mut Vec<&'a Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| flatten(shape, into)),
        shape => into.push(shape),
    }
}

pub fn shapes<S: Seen>(seen: &S) -> Vec<&Shape> {
    let mut all = Vec::new();
    for clipped in &seen.last_output().shapes {
        flatten(&clipped.shape, &mut all);
    }
    all
}

/// The meshes painted with a texture of their own, not the font atlas: the
/// images and icons.
pub fn textured<S: Seen>(seen: &S) -> Vec<&egui::epaint::Mesh> {
    shapes(seen)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => Some(&**mesh),
            _ => None,
        })
        .collect()
}

/// How far `mesh`, an image's quad, is turned, in degrees clockwise: the
/// direction of its top edge, from its first corner to its second.
pub fn turn(mesh: &egui::epaint::Mesh) -> f32 {
    let edge = mesh.vertices[1].pos - mesh.vertices[0].pos;
    edge.y.atan2(edge.x).to_degrees().rem_euclid(360.0)
}

/// Whether `a` and `b` are one rect, each edge within the half point a
/// drawn rect is rounded by.
pub fn same(a: Rect, b: Rect) -> bool {
    let near = |a: f32, b: f32| (a - b).abs() < 0.55;
    near(a.min.x, b.min.x)
        && near(a.min.y, b.min.y)
        && near(a.max.x, b.max.x)
        && near(a.max.y, b.max.y)
}

/// The colour of the glyph drawn inside `rect`.
pub fn glyph_color<S: Seen>(seen: &S, rect: Rect) -> Color32 {
    shapes(seen)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Mesh(mesh) if rect.expand(0.5).contains_rect(shape.visual_bounding_rect()) => {
                mesh.vertices.first().map(|vertex| vertex.color)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no glyph in {rect:?}"))
}

/// The last box painted over exactly `rect`, ring excluded.
pub fn box_at<S: Seen>(seen: &S, rect: Rect) -> RectShape {
    try_box_at(seen, rect).unwrap_or_else(|| panic!("no box painted at {rect:?}"))
}

/// [`box_at`], or `None` where nothing is painted.
pub fn try_box_at<S: Seen>(seen: &S, rect: Rect) -> Option<RectShape> {
    shapes(seen)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if same(r.rect, rect) && r.stroke_kind == StrokeKind::Inside => {
                Some(r.clone())
            }
            _ => None,
        })
        .next_back()
}

pub fn ring_at<S: Seen>(seen: &S, rect: Rect) -> Option<RectShape> {
    shapes(seen)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if same(r.rect, rect) && r.stroke_kind == StrokeKind::Outside => {
                Some(r.clone())
            }
            _ => None,
        })
        .next_back()
}

/// Where the text `text` painted with its origin inside `inside` starts.
pub fn text_left<S: Seen>(seen: &S, inside: Rect, text: &str) -> f32 {
    shapes(seen)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == text && inside.contains(t.pos) => Some(t.pos.x),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text:?} in {inside:?}"))
}

/// The colour of the text `text` painted with its origin inside `inside`.
pub fn text_color<S: Seen>(seen: &S, inside: Rect, text: &str) -> Color32 {
    shapes(seen)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == text && inside.contains(t.pos) => {
                // As epaint paints it: the override, else the glyphs' own
                // colour (egui's hint text has one, faded with the shape),
                // else the fallback.
                let glyph = t.galley.rows.iter().find_map(|placed| {
                    let visuals = &placed.row.visuals;
                    visuals.mesh.vertices.get(visuals.glyph_vertex_range.start)
                });
                let own = glyph
                    .map(|vertex| vertex.color)
                    .filter(|color| *color != Color32::PLACEHOLDER);
                Some(t.override_text_color.or(own).unwrap_or(t.fallback_color))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text:?} in {inside:?}"))
}

/// Records `harness` while a parameter steps over `values`: `apply` sets the
/// parameter, one frame is run for it, and `idle` frames at the end let
/// animations and the layout settle. `apply` is told the index of the frame
/// its value is drawn in. The caller declares the jumps it allows
/// and asserts on the returned timeline.
///
/// Test-only: the harness is a dev-dependency, and `eggshell_ui::testing` already
/// offers the harness-free [`Timeline`](eggshell_ui::testing::Timeline).
pub fn sweep<A, T>(
    harness: &mut Harness<'_, Rig<A>>,
    values: impl IntoIterator<Item = T>,
    mut apply: impl FnMut(&mut Harness<'_, Rig<A>>, usize, T),
    idle: usize,
) -> eggshell_ui::testing::Timeline {
    eggshell_ui::testing::record(&harness.ctx);
    for (frame, value) in values.into_iter().enumerate() {
        apply(harness, frame, value);
        harness.step();
    }
    harness.run_steps(idle);
    eggshell_ui::testing::Timeline::recorded(&harness.ctx)
}

/// A rig for examples that draw a whole frame themselves (scroll areas, work
/// after the draw): `frame` runs inside the harness's root `Ui` each step.
pub struct FrameRig<A> {
    pub app: A,
    pub bar: Bar,
    pub layout: LayoutState,
}

pub fn frame_harness<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    frame: fn(&mut egui::Ui, &mut A, &mut LayoutState),
) -> Harness<'static, FrameRig<A>> {
    frame_harness_scaled(size, 1.0, base_text, app, frame)
}

/// [`frame_harness`] on a display of `pixels_per_point`; `size` is in points.
pub fn frame_harness_scaled<A: 'static>(
    size: Vec2,
    pixels_per_point: f32,
    base_text: TextStyle,
    app: A,
    frame: fn(&mut egui::Ui, &mut A, &mut LayoutState),
) -> Harness<'static, FrameRig<A>> {
    let rig = FrameRig {
        app,
        bar: Bar::new(base_text.clone()),
        layout: LayoutState::new()
            .with_base_text(base_text)
            .with_style(shadcn()),
    };
    let harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(pixels_per_point)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, rig: &mut FrameRig<A>| {
                full_motion(ui.ctx());
                if eggshell_ui::theme::tokens(ui.ctx()).is_none() {
                    eggshell_ui::theme::Shadcn::apply_both(ui.ctx());
                }
                let background = eggshell_ui::theme::Shadcn::of(ui.ctx()).tokens.background;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, background);
                rig.bar.show(ui);
                frame(ui, &mut rig.app, &mut rig.layout);
            },
            rig,
        );
    eggshell_ui::testing::record(&harness.ctx);
    harness
}

/// The node `path` as the last recorded frame drew it.
pub fn last_drawn(ctx: &egui::Context, path: &str) -> Option<eggshell_ui::testing::NodeFrame> {
    let timeline = eggshell_ui::testing::Timeline::recorded(ctx);
    let last = timeline.len().checked_sub(1)?;
    timeline
        .frame(last)?
        .iter()
        .find(|n| n.path == path)
        .cloned()
}

/// Its layout rect, whether or not the last frame painted it.
pub fn layout_rect(ctx: &egui::Context, path: &str) -> Rect {
    eggshell_ui::testing::laid_out_rect(ctx, path)
        .unwrap_or_else(|| panic!("no node {path} was laid out"))
}

/// The shapes inside `shape`, groups flattened.
pub fn flatten_all_of(shape: &Shape) -> Vec<&Shape> {
    let mut all = Vec::new();
    flatten(shape, &mut all);
    all
}

/// The format of the section of `galley` that `part` starts in.
pub fn section_of<'g>(galley: &'g egui::Galley, part: &str) -> &'g egui::TextFormat {
    let start = galley.job.text.find(part).expect("part of the text");
    &galley
        .job
        .sections
        .iter()
        .find(|section| (section.byte_range.start.0..section.byte_range.end.0).contains(&start))
        .expect("a section covers the part")
        .format
}

/// The URLs the last frame asked the platform to open.
pub fn opened_urls<S: Seen>(seen: &S) -> Vec<String> {
    seen.last_output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::OutputCommand::OpenUrl(open) => Some(open.url.clone()),
            _ => None,
        })
        .collect()
}

// ------------------------------------------------------------- the driver

/// What the support's frame draws around the view. It is kept in the
/// context, not in the frame closure, so a test changes it between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Surround {
    page: Page,
    companion: Companion,
}

fn surround_id() -> egui::Id {
    egui::Id::NULL.with("support-surround")
}

fn surround(ctx: &egui::Context) -> Surround {
    ctx.data(|data| data.get_temp(surround_id()))
        .unwrap_or_default()
}

/// What the view under test is drawn beside from the next frame.
pub fn set_companion<A>(d: &Driver<A>, companion: Companion) {
    let ctx = d.ctx();
    let changed = Surround {
        companion,
        ..surround(ctx)
    };
    ctx.data_mut(|data| data.insert_temp(surround_id(), changed));
}

/// What is painted under the view from the next frame.
pub fn set_page<A>(d: &Driver<A>, page: Page) {
    let ctx = d.ctx();
    let changed = Surround {
        page,
        ..surround(ctx)
    };
    ctx.data_mut(|data| data.insert_temp(surround_id(), changed));
}

/// The step every support driver runs at unless told otherwise.
pub const STEP: Duration = Duration::from_millis(50);

/// A driver for `view` of `app` with what eggshell's own tests put around
/// it: the shadcn style, a [`Bar`] above the window (see [`Companion`]), the
/// [`Page`], and a style `prepare`d before each frame. It runs no frames
/// after an action ([`After::Frames(0)`]), and fails a test whose view
/// reports a problem it does not claim.
pub fn driver<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
) -> DriverBuilder<A> {
    rig_driver(size, base_text, app, view, None).style(shadcn())
}

/// [`driver`] for an app that sets egui's style: `prepare` runs before every
/// frame is built, and the layout takes the examples' chosen style.
pub fn prepared_driver<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: fn(&egui::Context, &A),
) -> DriverBuilder<A> {
    rig_driver(size, base_text, app, view, Some(prepare))
}

/// A support driver; `prepare` as [`Rig`]'s.
fn rig_driver<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    view: View<A>,
    prepare: Option<fn(&egui::Context, &A)>,
) -> DriverBuilder<A> {
    let mut bar = Bar::new(base_text.clone());
    Driver::whole_frame(app, move |ui, app, layout| {
        let area = ui.max_rect();
        let Surround { page, companion } = surround(ui.ctx());
        if eggshell_ui::theme::tokens(ui.ctx()).is_none() {
            eggshell_ui::theme::Shadcn::apply_both(ui.ctx());
        }
        if let Some(prepare) = prepare {
            prepare(ui.ctx(), &*app);
            layout.set_style(common::style(ui.ctx()));
        }
        if page == Page::Themed {
            let background = eggshell_ui::theme::Shadcn::of(ui.ctx()).tokens.background;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, background);
        }
        if companion == Companion::Bar {
            bar.show(ui);
        }
        layout
            .build(ui.ctx(), |b| view(b, &*app))
            .layout(area)
            .draw(app, ui)
    })
    .size(size)
    .base_text(base_text)
    .step(STEP)
    .after(After::Frames(0))
    .settle_within(STEP * 4)
}

/// A driver for a view that draws the whole frame itself (scroll areas,
/// work after the draw), with the shadcn style, the themed page and a
/// [`Bar`]; recording starts after the first frames.
pub fn frame_driver<A: 'static>(
    size: Vec2,
    base_text: TextStyle,
    app: A,
    frame: fn(&mut egui::Ui, &mut A, &mut LayoutState) -> PlacedFrame,
) -> DriverBuilder<A> {
    let mut bar = Bar::new(base_text.clone());
    let step = Duration::from_nanos(1_000_000_000 / 60);
    Driver::whole_frame(app, move |ui, app, layout| {
        if eggshell_ui::theme::tokens(ui.ctx()).is_none() {
            eggshell_ui::theme::Shadcn::apply_both(ui.ctx());
        }
        let background = eggshell_ui::theme::Shadcn::of(ui.ctx()).tokens.background;
        ui.painter()
            .rect_filled(ui.ctx().content_rect(), 0.0, background);
        bar.show(ui);
        frame(ui, app, layout)
    })
    .size(size)
    .base_text(base_text)
    .style(shadcn())
    .step(step)
    .after(After::Frames(0))
    .settle_within(step * 4)
}

/// Builds a [`frame_driver`] and starts recording, as the old frame rig did:
/// after its first frames.
pub fn recorded<A: 'static>(builder: DriverBuilder<A>) -> Driver<A> {
    let d = builder.build();
    eggshell_ui::testing::record(d.ctx());
    d
}

/// The classes `style`'s sheet gives `part` with no keys.
pub fn part<P: eggshell_ui::sheet::Part>(style: &eggshell_ui::Style, part: P) -> eggshell_ui::Tw {
    style.sheet.resolve(part, &[], &style.tokens)
}

/// The text colour `classes` give text in `style`.
pub fn ink(style: &eggshell_ui::Style, classes: &eggshell_ui::Tw) -> Color32 {
    classes
        .text_classes()
        .over(&TextStyle::new(style.text.clone(), Color32::PLACEHOLDER))
        .color
}

/// The classes `style`'s sheet gives `part` under `keys`.
pub fn part_at<P: eggshell_ui::sheet::Part>(
    style: &eggshell_ui::Style,
    part: P,
    keys: &[eggshell_ui::sheet::Key],
) -> eggshell_ui::Tw {
    style.sheet.resolve(part, keys, &style.tokens)
}

/// The border width and rest colour `style`'s sheet gives a text field.
pub fn input_border(style: &eggshell_ui::Style) -> (f32, Color32) {
    let classes = part(style, eggshell_ui::owned::input::Part::Root);
    let width = classes.style_at(style.tokens.spacing).border.left;
    let color = classes.paint_classes().border_colors.top;
    (
        width.into_raw().value(),
        color.unwrap_or(Color32::TRANSPARENT),
    )
}

/// What `style`'s sheet gives a text field of `size`: its height, padding
/// and radius.
pub struct FieldBox {
    pub height: f32,
    pub padding: Vec2,
    pub radius: u8,
}

/// The [`FieldBox`] `style`'s sheet gives a text field of `size`.
pub fn field_box(style: &eggshell_ui::Style, size: eggshell_ui::contract::Size) -> FieldBox {
    let classes = part_at(
        style,
        eggshell_ui::owned::input::Part::Root,
        &[eggshell_ui::sheet::Key::Size(size)],
    );
    let layout = classes.style_at(style.tokens.spacing);
    FieldBox {
        height: layout.size.height.into_raw().value(),
        padding: Vec2::new(
            layout.padding.left.into_raw().value(),
            layout.padding.top.into_raw().value(),
        ),
        radius: classes.paint_classes().radius.nw,
    }
}
