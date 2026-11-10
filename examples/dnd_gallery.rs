//! Drag-and-drop spike (#71), scenario 4: a gallery of square panels in 1, 2
//! or 3 columns inside a scroll area. Dragging a panel reorders in 2D with the
//! panels reflowing across rows; the drag image is the panel's own painted
//! shapes. Image files dropped from the OS insert a panel at the pointer. Near
//! the top or bottom of the viewport a drag scrolls. 1/2/3 set the columns,
//! G swaps the ghost for a chip, Esc cancels.
//!
//! The scroll area is egui's `ScrollArea` hosting the bullseye subtree; see
//! docs/dnd.md for what a native container has to offer instead.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;
use std::collections::HashMap;

use bullseye::{Builder, LayoutState, Opacity, tw};
use common::dnd::{Captured, capture, insertion_index, paint_chip, paint_ghost};
use egui::{Context, DragAndDrop, Key, Pos2, Rect, Response, Ui, vec2};

type B<'b, 'f, A> = Builder<'b, 'f, A>;

const SLIDE: Duration = Duration::from_millis(200);
/// How close to a viewport edge a drag starts to scroll, in points.
pub const EDGE: f32 = 48.0;
/// Scroll speed at the very edge, in points per second.
pub const SPEED: f32 = 600.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PanelKey(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Image {
    Chip,
    Ghost,
}

pub struct Gesture {
    pub original: Vec<PanelKey>,
    pub cancelled: bool,
    pub captured: Option<Captured>,
    /// The pointer's offset from the panel's centre when it was grabbed.
    pub grab: egui::Vec2,
}

pub struct State {
    pub panels: Vec<(PanelKey, String)>,
    pub cols: usize,
    pub image: Image,
    pub gesture: Option<Gesture>,
    /// Final layout rects of the panels in the last frame, screen space.
    pub rects: HashMap<PanelKey, Rect>,
    /// The scrolled content's origin and the visible viewport, screen space.
    pub origin: Pos2,
    pub viewport: Rect,
    /// Where an outside drop would insert, shown as a placeholder panel.
    pub preview: Option<(usize, String)>,
    pub content_height: f32,
    pub dropped: Vec<String>,
    next: u32,
}

impl Default for State {
    fn default() -> Self {
        let mut s = Self {
            panels: Vec::new(),
            cols: 3,
            image: Image::Ghost,
            gesture: None,
            rects: HashMap::new(),
            origin: Pos2::ZERO,
            viewport: Rect::NOTHING,
            preview: None,
            content_height: 0.0,
            dropped: Vec::new(),
            next: 0,
        };
        for _ in 0..12 {
            s.add_panel(None, format!("w{}", s.next));
        }
        s
    }
}

impl State {
    pub fn add_panel(&mut self, at: Option<usize>, label: String) {
        let key = PanelKey(self.next);
        self.next += 1;
        let at = at.unwrap_or(self.panels.len()).min(self.panels.len());
        self.panels.insert(at, (key, label));
    }

    pub fn keys(&self) -> Vec<PanelKey> {
        self.panels.iter().map(|p| p.0).collect()
    }

    pub fn restore(&mut self) {
        if let Some(g) = &self.gesture {
            let order = g.original.clone();
            self.panels
                .sort_by_key(|p| order.iter().position(|k| *k == p.0));
        }
    }

    /// The insertion index for a pointer at `pos` among the panels other than
    /// `skip`, worked out in content coordinates so scrolling cannot matter.
    pub fn index_at(&self, pos: Pos2, skip: Option<PanelKey>) -> usize {
        let rects: Vec<Rect> = self
            .panels
            .iter()
            .filter(|p| Some(p.0) != skip)
            .filter_map(|p| self.rects.get(&p.0))
            .map(|r| r.translate(-self.origin.to_vec2()))
            .collect();
        insertion_index(&rects, pos - self.origin.to_vec2())
    }
}

pub fn dragged(ctx: &Context) -> Option<PanelKey> {
    DragAndDrop::payload::<PanelKey>(ctx).map(|k| *k)
}

/// Whether the gallery takes a file: images only.
pub fn accepts(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
}

fn cell(b: &mut B<'_, '_, State>, key: PanelKey, label: &str, cols: usize, lifted: bool) {
    let t = common::tokens(b.ctx());
    b.div_id(
        key,
        tw().w_frac(1.0, cols as f32)
            .aspect_square()
            .p(1.0)
            .opacity(if lifted {
                Opacity::new(0.35)
            } else {
                Opacity::OPAQUE
            })
            .sense_drag()
            .merge(if lifted {
                tw()
            } else {
                tw().animate_layout(SLIDE)
            }),
        |b| {
            b.div(
                tw().flex()
                    .items_center()
                    .justify_center()
                    .w_full()
                    .h_full()
                    .rounded_md()
                    .border(1.0)
                    .border_color(t.input)
                    .bg(t.muted)
                    .text(t.foreground),
                |b| {
                    b.div(
                        tw().flex().flex_col().gap(1.0).p(2.0).w_full().h_full(),
                        |b| {
                            let now = b.ctx().input(|i| i.time);
                            b.div(tw().flex().justify_between().items_center(), |b| {
                                b.text(tw().role(common::sized(12.0)), label);
                                b.text(
                                    tw().role(common::sized(12.0)).text(t.muted_foreground),
                                    format!("{:+.2}", channel(key, now, 1.0)),
                                );
                            });
                            trace(b, key, t.primary, t.border);
                        },
                    );
                },
            );
        },
    )
    .on_response(move |app, r| source(app, r, key));
}

/// A telemetry channel's value at `time` seconds, `x` from 0 (oldest) to 1
/// (now): a few sines whose frequencies differ per panel.
fn channel(key: PanelKey, time: f64, x: f32) -> f32 {
    let seed = key.0 as f32;
    let t = time as f32 - (1.0 - x) * 2.0;
    0.55 * (t * (1.3 + 0.37 * seed)).sin()
        + 0.30 * (t * (3.1 + 0.53 * seed) + seed).sin()
        + 0.15 * (t * (7.7 + 0.11 * seed) + 2.0 * seed).sin()
}

/// The last two seconds of the channel, scrolling left, drawn as a 2 pt line
/// over a hairline zero baseline.
fn trace(b: &mut B<'_, '_, State>, key: PanelKey, line: egui::Color32, baseline: egui::Color32) {
    const POINTS: usize = 64;
    b.custom("trace", tw().w_full().grow())
        .ui(move |_: &mut State, ui| {
            let rect = ui.max_rect();
            let now = ui.input(|i| i.time);
            let painter = ui.painter();
            painter.hline(
                rect.x_range(),
                rect.center().y,
                egui::Stroke::new(1.0, baseline),
            );
            let points: Vec<Pos2> = (0..POINTS)
                .map(|i| {
                    let x = i as f32 / (POINTS - 1) as f32;
                    let y = channel(key, now, x);
                    egui::pos2(
                        rect.lerp_inside(vec2(x, 0.5)).x,
                        rect.center().y - y * rect.height() * 0.45,
                    )
                })
                .collect();
            painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, line)));
            ui.ctx().request_repaint();
        });
}

fn placeholder(b: &mut B<'_, '_, State>, cols: usize) {
    let t = common::tokens(b.ctx());
    b.div_id(
        "insert",
        tw().w_frac(1.0, cols as f32)
            .aspect_square()
            .p(1.0)
            .animate_layout(SLIDE),
        |b| {
            b.div(
                tw().w_full()
                    .h_full()
                    .rounded_md()
                    .border(2.0)
                    .border_color(t.ring),
                |_| {},
            )
        },
    );
}

pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    let released = b.ctx().input(|i| i.pointer.any_released());
    let held = dragged(b.ctx()).filter(|_| !released);
    b.div_id(
        "grid",
        tw().flex().flex_row().flex_wrap().w_full().p(2.0),
        |b| {
            let mut slot = 0;
            for (key, label) in &state.panels {
                if state.preview.as_ref().is_some_and(|p| p.0 == slot) {
                    placeholder(b, state.cols);
                }
                slot += 1;
                cell(b, *key, label, state.cols, held == Some(*key));
            }
            if state.preview.as_ref().is_some_and(|p| p.0 >= slot) {
                placeholder(b, state.cols);
            }
        },
    );
}

fn source(app: &mut State, r: &Response, key: PanelKey) {
    app.rects.insert(key, r.rect);
    if r.drag_started() {
        r.dnd_set_drag_payload(key);
        app.gesture = Some(Gesture {
            original: app.keys(),
            cancelled: false,
            captured: None,
            grab: r
                .ctx
                .pointer_interact_pos()
                .map_or(egui::Vec2::ZERO, |p| p - r.rect.center()),
        });
        r.ctx.request_repaint();
    }
    if r.dragged()
        && dragged(&r.ctx).is_none()
        && let Some(g) = &mut app.gesture
        && !g.cancelled
    {
        g.cancelled = true;
        app.restore();
        r.ctx.request_repaint();
    }
    if r.drag_stopped() {
        let inside = dragged(&r.ctx).is_some()
            && r.ctx
                .pointer_interact_pos()
                .is_some_and(|p| app.viewport.contains(p));
        if !inside {
            app.restore();
        }
        app.gesture = None;
        r.ctx.request_repaint();
    }
}

/// Points per second to scroll: negative near the top, positive near the
/// bottom, proportional to how deep into the edge band the pointer is.
pub fn edge_speed(viewport: Rect, y: f32) -> f32 {
    let top = (viewport.min.y + EDGE - y).max(0.0) / EDGE;
    let bottom = (y - (viewport.max.y - EDGE)).max(0.0) / EDGE;
    SPEED * (bottom.min(1.0) - top.min(1.0))
}

/// Everything that follows the draw, once per frame, inside the scroll area:
/// the one place that turns pointer and payload into an insertion index.
pub fn update(ui: &mut Ui, app: &mut State) {
    let ctx = ui.ctx().clone();
    app.viewport = ui.clip_rect();
    let pointer = ctx.pointer_latest_pos();
    let hovered: Vec<_> = ctx.input(|i| i.raw.hovered_files.clone());
    let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
    let wanted: Vec<String> = hovered
        .iter()
        .filter_map(|f| f.path.as_deref())
        .filter(|p| accepts(p))
        .filter_map(|p| p.file_name()?.to_str().map(str::to_owned))
        .collect();

    let live = app.gesture.as_ref().is_some_and(|g| !g.cancelled);
    let held = dragged(&ctx).filter(|_| live);
    if let (Some(held), Some(pos)) = (held, pointer) {
        let to = app.index_at(pos, Some(held));
        if let Some(at) = app.panels.iter().position(|p| p.0 == held)
            && at != to
        {
            let item = app.panels.remove(at);
            app.panels.insert(to, item);
            ctx.request_repaint();
        }
    }
    app.preview = match (pointer, wanted.first()) {
        (Some(pos), Some(label)) if app.viewport.contains(pos) => {
            Some((app.index_at(pos, None), label.clone()))
        }
        _ => None,
    };
    if !dropped.is_empty() {
        let mut at = pointer
            .filter(|p| app.viewport.contains(*p))
            .map(|p| app.index_at(p, None));
        for file in &dropped {
            let path = file.path();
            if accepts(path) {
                let name = path
                    .file_name()
                    .map_or(String::new(), |n| n.to_string_lossy().into());
                app.add_panel(at, name.clone());
                at = at.map(|i| i + 1);
                app.dropped.push(format!("accepted {name}"));
            } else {
                app.dropped.push(format!("refused {}", path.display()));
            }
        }
        app.preview = None;
        ctx.request_repaint();
    }

    if app.gesture.as_ref().is_some_and(|g| g.captured.is_none())
        && let Some(key) = dragged(&ctx)
        && let Some(rect) = app.rects.get(&key)
    {
        let captured = capture(&ctx, ui.layer_id(), *rect);
        if let Some(g) = &mut app.gesture {
            g.captured = Some(captured);
        }
    }

    if (held.is_some() || !hovered.is_empty())
        && let Some(pos) = pointer
    {
        let dy = edge_speed(app.viewport, pos.y);
        if dy != 0.0 {
            let dt = ctx.input(|i| i.stable_dt);
            ui.scroll_with_delta(vec2(0.0, -dy * dt));
            ctx.request_repaint();
        }
    }
}

pub fn drag_image(ctx: &Context, app: &State) {
    let (Some(pos), Some(key), Some(g)) = (ctx.pointer_interact_pos(), dragged(ctx), &app.gesture)
    else {
        return;
    };
    match (app.image, &g.captured) {
        (Image::Ghost, Some(captured)) => paint_ghost(ctx, captured, pos - g.grab),
        _ => {
            let label = app.panels.iter().find(|p| p.0 == key).map_or("", |p| &p.1);
            paint_chip(ctx, label, pos);
        }
    }
}

/// One frame: the scroll area, the build, layout and draw inside it, then
/// `update`. Shared by `main` and the tests.
pub fn frame(ui: &mut Ui, app: &mut State, layout_state: &mut LayoutState) {
    let ctx = ui.ctx().clone();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let origin = ui.max_rect().min;
            let area = Rect::from_min_size(
                origin,
                vec2(
                    ui.available_width(),
                    app.content_height.max(ui.clip_rect().height()),
                ),
            );
            let built = layout_state.build(&ctx, |b| layout(b, app));
            let placed = built.layout(area);
            if let Some(grid) = placed.nodes().find(|n| n.path() == GRID) {
                app.origin = grid.rect().min;
                app.content_height = grid.rect().max.y - origin.y;
            }
            placed.draw(app, ui);
            ui.advance_cursor_after_rect(Rect::from_min_size(
                origin,
                vec2(area.width(), app.content_height),
            ));
            update(ui, app);
        });
    drag_image(&ctx, app);
}

pub const GRID: &str = "/\"grid\"";

/// The strip above the gallery: the column count, and the drag image.
///
/// ```html
/// <div class="flex items-center gap-3 px-3 h-11 border-b">
///   <span>Columns</span> <div role="radiogroup">1 2 3</div>
///   <div role="radiogroup">Ghost Chip</div>
/// </div>
/// ```
pub fn toolbar(b: &mut B<'_, '_, State>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .items_center()
            .gap(3.0)
            .px(3.0)
            .size_full()
            .border_b(1.0)
            .border_color(t.border)
            .text(t.foreground),
        |b| {
            b.text(tw().text(t.muted_foreground), "Columns");
            b.segmented("columns", state.cols, [(1, "1"), (2, "2"), (3, "3")])
                .on_change(|state: &mut State, cols| state.cols = cols);
            b.text(tw().text(t.muted_foreground), "Drag image");
            b.segmented(
                "image",
                state.image,
                [(Image::Ghost, "Ghost"), (Image::Chip, "Chip")],
            )
            .on_change(|state: &mut State, image| state.image = image);
        },
    );
}

fn main() -> eframe::Result {
    let mut state = State::default();
    let mut layout_state = LayoutState::new();
    let mut toolbar_state = LayoutState::new();
    let mut theme_bar = common::ThemeBar::new();
    common::launch("dnd_gallery", None, move |ui| {
        let ctx = ui.ctx().clone();
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout_state.set_base_text(Some(common::base_text_in(&ctx)));
        for (key, cols) in [(Key::Num1, 1), (Key::Num2, 2), (Key::Num3, 3)] {
            if ctx.input(|i| i.key_pressed(key)) {
                state.cols = cols;
            }
        }
        if ctx.input(|i| i.key_pressed(Key::G)) {
            state.image = match state.image {
                Image::Chip => Image::Ghost,
                Image::Ghost => Image::Chip,
            };
        }
        egui::Panel::top("gallery_toolbar")
            .exact_size(44.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
            .show(ui, |ui| {
                let area = ui.max_rect();
                toolbar_state.set_base_text(Some(common::base_text_in(&ctx)));
                toolbar_state
                    .build(&ctx, |b| toolbar(b, &state))
                    .layout(area)
                    .draw(&mut state, ui);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
            .show(ui, |ui| frame(ui, &mut state, &mut layout_state));
    })
}
