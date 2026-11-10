//! Drag-and-drop spike (#71), scenarios 2 and 3: items from two lists dragged
//! onto a canvas zone that takes only cards and reports where the drop landed;
//! image and document files from the OS dropped on two more zones, each
//! accepting by extension.
//!
//! One resolver, `zone_at`, decides which zone the pointer is over for
//! in-app payloads and OS files alike.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;

use bullseye::{Builder, LayoutState, Opacity, tw};
use egui::{Context, DragAndDrop, Pos2, Rect, Response, Ui, Vec2};

type B<'b, 'f, A> = Builder<'b, 'f, A>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Card(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    Canvas,
    Pictures,
    Documents,
}

impl Zone {
    pub const ALL: [Zone; 3] = [Zone::Canvas, Zone::Pictures, Zone::Documents];

    pub fn takes_file(self, path: &std::path::Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        matches!(
            (self, ext.as_deref()),
            (Zone::Pictures, Some("png" | "jpg" | "jpeg"))
                | (Zone::Documents, Some("pdf" | "txt" | "md"))
        )
    }
}

#[derive(Default)]
pub struct State {
    pub cards: Vec<u32>,
    pub tags: Vec<String>,
    /// Cards dropped on the canvas: the card and where, in the zone.
    pub placed: Vec<(u32, Vec2)>,
    pub files: Vec<(Zone, String)>,
    pub log: Vec<String>,
    pub zone_rects: HashMap<Zone, Rect>,
    /// The zone a compatible payload or file hovers; drives the highlight and
    /// is one frame behind the pointer (set after draw, read at build).
    pub hover: Option<Zone>,
    /// Zones the current drag would be refused by.
    pub refusing: Vec<Zone>,
}

impl State {
    pub fn new() -> Self {
        Self {
            cards: vec![1, 2, 3],
            tags: vec!["red".into(), "blue".into()],
            ..Self::default()
        }
    }

    /// The zone under `pos`. The last in `Zone::ALL` order wins on overlap,
    /// as the paint order says.
    pub fn zone_at(&self, pos: Pos2) -> Option<Zone> {
        Zone::ALL
            .into_iter()
            .rev()
            .find(|z| self.zone_rects.get(z).is_some_and(|r| r.contains(pos)))
    }
}

fn draggable(b: &mut B<'_, '_, State>, key: &'static str, id: u32, label: String, card: bool) {
    let t = common::tokens(b.ctx());
    b.div_id(
        (key, id),
        tw().h(8.0)
            .px(3.0)
            .flex()
            .items_center()
            .rounded_md()
            .border(1.0)
            .border_color(t.input)
            .bg(t.muted)
            .sense_drag(),
        |b| b.text(tw(), label.clone()),
    )
    .on_response(move |_, r: &Response| {
        if card {
            r.dnd_set_drag_payload(Card(id));
        } else {
            r.dnd_set_drag_payload(Tag(label.clone()));
        }
    });
}

fn zone(b: &mut B<'_, '_, State>, state: &State, which: Zone, title: &str) {
    let t = common::tokens(b.ctx());
    let lit = state.hover == Some(which);
    b.div_id(
        which_key(which),
        tw().grow()
            .h(40.0)
            .p(2.0)
            .rounded_md()
            .border(2.0)
            .border_color(if lit { t.ring } else { t.input })
            .bg(if lit { t.accent } else { t.muted })
            .opacity(if state.refusing.contains(&which) {
                Opacity::new(0.4)
            } else {
                Opacity::OPAQUE
            })
            .sense_hover(),
        |b| {
            b.text(tw(), title);
            if which == Zone::Canvas {
                for (card, at) in &state.placed {
                    b.text(tw(), format!("card {card} at {:.0},{:.0}", at.x, at.y));
                }
            }
            for (z, name) in &state.files {
                if *z == which {
                    b.text(tw(), name.clone());
                }
            }
        },
    )
    .on_response(move |app, r| {
        app.zone_rects.insert(which, r.rect);
    });
}

fn which_key(z: Zone) -> &'static str {
    match z {
        Zone::Canvas => "canvas",
        Zone::Pictures => "pictures",
        Zone::Documents => "documents",
    }
}

pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    b.div(tw().flex().gap(3.0).p(3.0), |b| {
        b.div(tw().flex().flex_col().gap(1.0).w_px(120.0), |b| {
            for &c in &state.cards {
                draggable(b, "card", c, format!("Card {c}"), true);
            }
        });
        b.div(tw().flex().flex_col().gap(1.0).w_px(120.0), |b| {
            for (i, t) in state.tags.iter().enumerate() {
                draggable(b, "tag", i as u32, t.clone(), false);
            }
        });
        b.div(tw().flex().flex_col().gap(2.0).grow(), |b| {
            zone(b, state, Zone::Canvas, "Canvas (cards only)");
            zone(b, state, Zone::Pictures, "Pictures (png, jpg)");
            zone(b, state, Zone::Documents, "Documents (pdf, txt, md)");
        });
    });
}

/// After draw: resolve the hovered zone, deliver drops, set the highlight.
pub fn update(ctx: &Context, app: &mut State) {
    let pointer = ctx.pointer_latest_pos();
    let over = pointer.and_then(|p| app.zone_at(p));
    let hovered: Vec<_> = ctx.input(|i| i.raw.hovered_files.clone());
    let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());

    app.hover = None;
    app.refusing.clear();
    if DragAndDrop::has_payload_of_type::<Card>(ctx) {
        app.hover = over.filter(|z| *z == Zone::Canvas);
    } else if DragAndDrop::has_any_payload(ctx) {
        // An incompatible payload: nothing lights up, nothing is refused.
    } else if !hovered.is_empty() {
        app.hover = over.filter(|z| {
            hovered
                .iter()
                .filter_map(|f| f.path.as_deref())
                .all(|p| z.takes_file(p))
        });
        app.refusing = Zone::ALL
            .into_iter()
            .filter(|z| *z != Zone::Canvas && Some(*z) != app.hover)
            .collect();
    }

    if ctx.input(|i| i.pointer.any_released())
        && let Some(card) = DragAndDrop::payload::<Card>(ctx)
        && let (Some(Zone::Canvas), Some(p)) = (over, pointer)
    {
        let local = p - app.zone_rects[&Zone::Canvas].min;
        app.placed.push((card.0, local));
        app.cards.retain(|c| *c != card.0);
    }
    for file in dropped {
        let name = file.path().display().to_string();
        match over.filter(|z| z.takes_file(file.path())) {
            Some(z) => {
                app.log.push(format!("{z:?} took {name}"));
                app.files.push((z, name));
            }
            None => app.log.push(format!("refused {name} over {over:?}")),
        }
    }
}

pub fn frame(ui: &mut Ui, app: &mut State, layout_state: &mut LayoutState) {
    let ctx = ui.ctx().clone();
    let area = ui.max_rect();
    let placed = layout_state.build(&ctx, |b| layout(b, app)).layout(area);
    placed.draw(app, ui);
    update(&ctx, app);
    if let (Some(pos), true) = (
        ctx.pointer_interact_pos(),
        DragAndDrop::has_any_payload(&ctx),
    ) {
        common::dnd::paint_chip(&ctx, "dragging", pos);
    }
    if app.hover.is_some() || DragAndDrop::has_any_payload(&ctx) {
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let mut state = State::new();
    let mut layout_state = LayoutState::new();
    let mut theme_bar = common::ThemeBar::new();
    common::launch("dnd_zones", None, move |ui| {
        let ctx = ui.ctx().clone();
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout_state.set_base_text(Some(common::base_text_in(&ctx)));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
            .show(ui, |ui| frame(ui, &mut state, &mut layout_state));
    })
}
