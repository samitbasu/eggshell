//! Items from two lists dragged onto a canvas zone that takes only cards and
//! reports where the drop landed; image and document files from the OS
//! dropped on two more zones, each accepting by extension.
//!
//! A zone lights up (`drag_accepts`) while a drag it would take is in
//! progress, and fills (`drag_over`) when the drag is over it. A tag takes no
//! zone, so it lights none. A file no zone takes falls to the page, which
//! refuses it.

#[path = "common/mod.rs"]
mod common;

use std::any::Any;

use eggshell_ui::{File, Files, LayoutState, tw};
use eggshell_ui_shadcn::DragChip as _;
use egui::{Ui, Vec2};

type B<'b, 'f, A> = eggshell_ui::Builder<'b, 'f, A>;

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

    pub fn takes(self, file: &File) -> bool {
        matches!(
            (self, file.extension().as_deref()),
            (Zone::Pictures, Some("png" | "jpg" | "jpeg"))
                | (Zone::Documents, Some("pdf" | "txt" | "md"))
        )
    }

    pub fn key(self) -> &'static str {
        match self {
            Zone::Canvas => "canvas",
            Zone::Pictures => "pictures",
            Zone::Documents => "documents",
        }
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
}

impl State {
    pub fn new() -> Self {
        Self {
            cards: vec![1, 2, 3],
            tags: vec!["red".into(), "blue".into()],
            ..Self::default()
        }
    }

    fn take_files(&mut self, zone: Zone, files: &Files) {
        for file in files.iter() {
            let name = name_of(file);
            self.log.push(format!("{zone:?} took {name}"));
            self.files.push((zone, name));
        }
    }
}

fn name_of(file: &File) -> String {
    file.path()
        .map_or(String::new(), |path| path.display().to_string())
}

fn draggable<T: Any + Send + Sync>(
    b: &mut B<'_, '_, State>,
    key: (&'static str, u32),
    label: String,
    payload: T,
) {
    let t = common::tokens(b.ctx());
    b.div_id(
        key,
        tw().h(8.0)
            .px(3.0)
            .flex()
            .items_center()
            .rounded_md()
            .border(1.0)
            .border_color(t.input)
            .bg(t.muted),
        |b| b.text(tw(), label.clone()),
    )
    .drag_source(payload)
    .with_drag_chip(label);
}

fn zone(b: &mut B<'_, '_, State>, state: &State, which: Zone, title: &str) {
    let t = common::tokens(b.ctx());
    let node = b.div_id(
        which.key(),
        tw().grow()
            .h(40.0)
            .p(2.0)
            .rounded_md()
            .border(2.0)
            .border_color(t.input)
            .bg(t.muted)
            .drag_accepts(|p| p.border_color(t.ring))
            .drag_over(|p| p.bg(t.accent)),
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
    );
    match which {
        Zone::Canvas => {
            node.drop_target::<Card>(|app, drop| {
                let card = drop.payload.0;
                app.placed.push((card, drop.position.unwrap_or_default()));
                app.cards.retain(|c| *c != card);
            });
        }
        Zone::Pictures | Zone::Documents => {
            node.drop_target_if::<Files>(
                move |files| files.iter().all(|file| which.takes(file)),
                move |app, drop| app.take_files(which, &drop.payload),
            )
            .drop_fallback();
        }
    }
}

pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    b.div_id("page", tw().flex().gap(3.0).p(3.0).size_full(), |b| {
        b.div(tw().flex().flex_col().gap(1.0).w_px(120.0), |b| {
            for &card in &state.cards {
                draggable(b, ("card", card), format!("Card {card}"), Card(card));
            }
        });
        b.div(tw().flex().flex_col().gap(1.0).w_px(120.0), |b| {
            for (i, tag) in state.tags.iter().enumerate() {
                draggable(b, ("tag", i as u32), tag.clone(), Tag(tag.clone()));
            }
        });
        b.div(tw().flex().flex_col().gap(2.0).grow(), |b| {
            zone(b, state, Zone::Canvas, "Canvas (cards only)");
            zone(b, state, Zone::Pictures, "Pictures (png, jpg)");
            zone(b, state, Zone::Documents, "Documents (pdf, txt, md)");
        });
    })
    .drop_target::<Files>(|app, drop| {
        for file in drop.payload.iter() {
            app.log.push(format!("refused {}", name_of(file)));
        }
    })
    .drop_fallback();
}

/// One frame; shared by `main` and the tests.
pub fn frame(ui: &mut Ui, app: &mut State, layout_state: &mut LayoutState) {
    let area = ui.max_rect();
    layout_state
        .build(&ui.ctx().clone(), |b| layout(b, app))
        .layout(area)
        .draw(app, ui);
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
