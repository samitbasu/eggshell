//! A list that reorders live under a drag. Siblings slide with
//! `animate_layout`; the drag image is a chip or the row itself (G toggles);
//! Esc restores the order; releasing outside the list cancels; Alt+Up and
//! Alt+Down move the focused row without a pointer.
//!
//! The order in `State` does not change until the drop: the view previews it
//! from where the drag is over the list, so cancelling needs no undo.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;

use eggshell_ui::{LayoutState, Opacity, tw};
use eggshell_ui_shadcn::DragChip as _;
use egui::{Context, Key, Modifiers, Response, Ui};

type B<'b, 'f, A> = eggshell_ui::Builder<'b, 'f, A>;

const SLIDE: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemKey(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    Chip,
    Live,
}

pub struct State {
    pub items: Vec<(ItemKey, String)>,
    pub image: Image,
    pub status: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            items: ["Lay it out", "Hover over it", "Reorder it", "Ship it"]
                .iter()
                .enumerate()
                .map(|(i, t)| (ItemKey(i as u32), (*t).to_owned()))
                .collect(),
            image: Image::Chip,
            status: String::new(),
        }
    }
}

impl State {
    pub fn index_of(&self, key: ItemKey) -> Option<usize> {
        self.items.iter().position(|item| item.0 == key)
    }

    pub fn move_to(&mut self, key: ItemKey, to: Option<usize>) {
        if let (Some(from), Some(to)) = (self.index_of(key), to) {
            let last = self.items.len() - 1;
            common::dnd::move_to(&mut self.items, from, to.min(last));
        }
    }
}

pub const LIST: &str = "/\"list\"";

/// The row in its slot. The slot is outlined while a live image carries the
/// row away; with a chip the row stays in it, dimmed.
fn row(b: &mut B<'_, '_, State>, key: ItemKey, title: &str, state: &State, lifted: bool) {
    let t = common::tokens(b.ctx());
    let slot = tw().h(10.0).animate_layout(SLIDE);
    let slot = match (lifted, state.image) {
        (true, Image::Live) => slot.rounded_md().border(1.0).border_color(t.ring),
        _ => slot,
    };
    let row = tw()
        .flex()
        .items_center()
        .size_full()
        .px(3.0)
        .rounded_md()
        .border(1.0)
        .border_color(t.input)
        .bg(t.muted);
    // With a chip the lifted row stays in its slot, dimmed; the live image is
    // the row itself and keeps its look.
    let row = match state.image {
        Image::Chip => row.drag_lifted(|p| p.opacity(Opacity::new(0.35))),
        Image::Live => row,
    };
    b.div_id(key, slot, |b| {
        let source = b
            .div_id("row", row, |b| b.text(tw().grow().truncate(), title))
            .drag_source(key)
            .on_response(move |app, response| move_with_keys(app, response, key));
        if state.image == Image::Chip {
            source.with_drag_chip(title);
        }
    });
}

/// Alt+Up and Alt+Down move the focused row.
fn move_with_keys(app: &mut State, response: &Response, key: ItemKey) {
    if !response.has_focus() {
        return;
    }
    let consume = |arrow| {
        response
            .ctx
            .input_mut(|i| i.consume_key(Modifiers::ALT, arrow))
    };
    let (up, down) = (consume(Key::ArrowUp), consume(Key::ArrowDown));
    if let Some(at) = app.index_of(key) {
        if up && at > 0 {
            app.move_to(key, Some(at - 1));
        }
        if down && at + 1 < app.items.len() {
            app.move_to(key, Some(at + 1));
        }
    }
}

pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    let carried = b.dragging::<ItemKey>();
    b.on_drag_end::<ItemKey>(|app, key, end| app.status = format!("{key:?}: {end:?}"));
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
            let mut order: Vec<&(ItemKey, String)> = state.items.iter().collect();
            if let Some(drag) = &carried
                && let Some(to) = drag.index_hint()
                && let Some(from) = order.iter().position(|item| item.0 == *drag.payload)
            {
                common::dnd::move_to(&mut order, from, to);
            }
            let lifted = carried.as_ref().map(|drag| *drag.payload);
            for (key, title) in order {
                row(b, *key, title, state, lifted == Some(*key));
            }
        },
    )
    .drop_target::<ItemKey>(|app, drop| app.move_to(*drop.payload, drop.index_hint));
}

/// One frame; shared by `main` and the tests.
pub fn frame(ui: &mut Ui, app: &mut State, layout_state: &mut LayoutState) {
    let area = ui.max_rect();
    layout_state
        .build(&ui.ctx().clone(), |b| layout(b, app))
        .layout(area)
        .draw(app, ui);
}

fn shortcuts(ctx: &Context, state: &mut State) {
    if ctx.input(|i| i.key_pressed(Key::G)) {
        state.image = match state.image {
            Image::Chip => Image::Live,
            Image::Live => Image::Chip,
        };
    }
}

fn main() -> eframe::Result {
    let mut state = State::default();
    let mut layout_state = LayoutState::new();
    let mut theme_bar = common::ThemeBar::new();
    common::launch("dnd_reorder", None, move |ui| {
        let ctx = ui.ctx().clone();
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout_state.set_base_text(Some(common::base_text_in(&ctx)));
        shortcuts(&ctx, &mut state);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
            .show(ui, |ui| frame(ui, &mut state, &mut layout_state));
    })
}
