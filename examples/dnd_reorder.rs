//! Drag-and-drop spike (#71), scenario 1: a list that reorders live under a
//! drag. Siblings slide with `animate_layout`; the drag image is either a
//! chip or the source's own painted shapes (toggle with G); Esc restores the
//! order; releasing outside the list cancels; Alt+Up / Alt+Down move the
//! focused row without a pointer.
//!
//! The spike keeps the gesture in egui's `DragAndDrop` payload (readable at
//! build time) and the order snapshot in the app.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;

use bullseye::{Builder, Opacity, tw};
use common::dnd::{Captured, capture, insertion_index, paint_chip, paint_ghost};
use egui::{Context, DragAndDrop, Key, Modifiers, Rect, Response, Ui};

type B<'b, 'f, A> = Builder<'b, 'f, A>;

const SLIDE: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemKey(pub u32);

/// How the drag image is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    Chip,
    Ghost,
}

pub const LIST: &str = "/\"list\"";

pub struct Gesture {
    pub original: Vec<ItemKey>,
    pub cancelled: bool,
    pub captured: Option<Captured>,
}

pub struct State {
    pub items: Vec<(ItemKey, String)>,
    pub gesture: Option<Gesture>,
    pub image: Image,
    /// The list's rect in the last frame; releasing outside it cancels.
    pub list_rect: Rect,
    pub source_rect: Option<Rect>,
    pub moves: usize,
    pub rects: std::collections::HashMap<ItemKey, Rect>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            items: ["Lay it out", "Hover over it", "Reorder it", "Ship it"]
                .iter()
                .enumerate()
                .map(|(i, t)| (ItemKey(i as u32), (*t).to_owned()))
                .collect(),
            gesture: None,
            image: Image::Chip,
            list_rect: Rect::NOTHING,
            source_rect: None,
            moves: 0,
            rects: Default::default(),
        }
    }
}

impl State {
    pub fn keys(&self) -> Vec<ItemKey> {
        self.items.iter().map(|i| i.0).collect()
    }

    pub fn index_of(&self, key: ItemKey) -> Option<usize> {
        self.items.iter().position(|i| i.0 == key)
    }

    pub fn move_to(&mut self, key: ItemKey, to: usize) {
        if let Some(at) = self.index_of(key)
            && at != to
        {
            let item = self.items.remove(at);
            self.items.insert(to, item);
            self.moves += 1;
        }
    }

    pub fn restore(&mut self) {
        if let Some(gesture) = &self.gesture {
            let order = gesture.original.clone();
            self.items
                .sort_by_key(|item| order.iter().position(|k| *k == item.0));
        }
    }
}

/// Who is being dragged, readable at build time with no app state.
pub fn dragged(ctx: &Context) -> Option<ItemKey> {
    DragAndDrop::payload::<ItemKey>(ctx).map(|k| *k)
}

fn row(b: &mut B<'_, '_, State>, key: ItemKey, title: &str) {
    let t = common::tokens(b.ctx());
    let lifted = dragged(b.ctx()) == Some(key);
    b.div_id(
        key,
        tw().flex()
            .items_center()
            .h(10.0)
            .px(3.0)
            .rounded_md()
            .border(1.0)
            .border_color(t.input)
            .bg(t.muted)
            .opacity(if lifted {
                Opacity::new(0.35)
            } else {
                Opacity::OPAQUE
            })
            .sense_drag()
            .animate_layout(SLIDE),
        |b| b.text(tw().grow().truncate(), title),
    )
    .on_response(move |app, r| source(app, r, key));
}

/// The drag source side of a row.
fn source(app: &mut State, r: &Response, key: ItemKey) {
    app.rects.insert(key, r.rect);
    if r.drag_started() {
        r.dnd_set_drag_payload(key);
        app.gesture = Some(Gesture {
            original: app.keys(),
            cancelled: false,
            captured: None,
        });
        app.source_rect = Some(r.rect);
        r.ctx.request_repaint();
    }
    if r.dragged() {
        app.source_rect = Some(r.rect);
        if dragged(&r.ctx).is_none()
            && let Some(g) = &mut app.gesture
            && !g.cancelled
        {
            g.cancelled = true;
            app.restore();
            r.ctx.request_repaint();
        }
    }
    if r.drag_stopped() {
        // Esc clears the payload at the start of the frame and egui then ends
        // the drag, so a stop with no payload is a cancel, not a release.
        let inside = dragged(&r.ctx).is_some()
            && r.ctx
                .pointer_interact_pos()
                .is_some_and(|p| app.list_rect.contains(p));
        if !inside {
            app.restore();
        }
        app.gesture = None;
        app.source_rect = None;
        r.ctx.request_repaint();
    }
    if r.has_focus() {
        let up = r
            .ctx
            .input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowUp));
        let down = r
            .ctx
            .input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowDown));
        if let Some(at) = app.index_of(key) {
            if up && at > 0 {
                app.move_to(key, at - 1);
            }
            if down && at + 1 < app.items.len() {
                app.move_to(key, at + 1);
            }
        }
    }
}

/// The one resolver: where the dragged row goes, from this frame's final
/// layout rects and the pointer. A per-row `contains_pointer` is not usable:
/// egui answers it from the previous frame's rects, so a row that has just
/// moved still claims the pointer and the order thrashes (docs/dnd.md).
fn resolve(app: &mut State, ctx: &Context) {
    let live = app.gesture.as_ref().is_some_and(|g| !g.cancelled);
    let (Some(held), Some(pos), true) = (dragged(ctx), ctx.pointer_latest_pos(), live) else {
        return;
    };
    let rects: Vec<Rect> = app
        .items
        .iter()
        .filter(|i| i.0 != held)
        .filter_map(|i| app.rects.get(&i.0).copied())
        .collect();
    let to = insertion_index(&rects, pos);
    app.move_to(held, to);
    ctx.request_repaint();
}

/// Paints the drag image on the tooltip layer, which is above every other
/// layer.
pub fn drag_image(ctx: &Context, app: &State) {
    let Some(pos) = ctx.pointer_interact_pos() else {
        return;
    };
    let (Some(key), Some(gesture)) = (dragged(ctx), &app.gesture) else {
        return;
    };
    match (app.image, &gesture.captured) {
        (Image::Ghost, Some(captured)) => paint_ghost(ctx, captured, pos),
        _ => {
            let title = app
                .items
                .iter()
                .find(|i| i.0 == key)
                .map_or("", |i| i.1.as_str());
            paint_chip(ctx, title, pos);
        }
    }
}

pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    b.div_id(
        "list",
        tw().flex()
            .flex_col()
            .gap(1.0)
            .w_full()
            .max_w_px(480.0)
            .mx_auto()
            .p(4.0),
        |b| {
            for (key, title) in &state.items {
                row(b, *key, title);
            }
        },
    );
}

/// After `draw`: record the list rect, and capture the ghost on the first
/// frame of a drag, when the source is still undimmed.
pub fn after_draw(ctx: &Context, ui: &Ui, app: &mut State, list: Rect) {
    app.list_rect = list;
    resolve(app, ctx);
    if let (Some(rect), Some(g)) = (app.source_rect, &mut app.gesture)
        && g.captured.is_none()
    {
        g.captured = Some(capture(ctx, ui.layer_id(), rect));
    }
}

/// One frame; shared by `main` and the tests.
pub fn frame(ui: &mut Ui, app: &mut State, layout_state: &mut bullseye::LayoutState) {
    let ctx = ui.ctx().clone();
    let area = ui.max_rect();
    let built = layout_state.build(&ctx, |b| layout(b, app));
    let placed = built.layout(area);
    let list = placed
        .nodes()
        .find(|n| n.path() == LIST)
        .map_or(area, |n| n.rect());
    placed.draw(app, ui);
    after_draw(&ctx, ui, app, list);
    drag_image(&ctx, app);
}

fn main() -> eframe::Result {
    let mut state = State::default();
    let mut layout_state = bullseye::LayoutState::new();
    let mut theme_bar = common::ThemeBar::new();
    common::launch("dnd_reorder", None, move |ui| {
        let ctx = ui.ctx().clone();
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout_state.set_base_text(Some(common::base_text_in(&ctx)));
        if ctx.input(|i| i.key_pressed(Key::G)) {
            state.image = match state.image {
                Image::Chip => Image::Ghost,
                Image::Ghost => Image::Chip,
            };
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
            .show(ui, |ui| frame(ui, &mut state, &mut layout_state));
    })
}
