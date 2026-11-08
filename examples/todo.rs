//! A to-do list: a keyed list with row hover, actions revealed by group hover, and reordering.
//!
//! Every row is a `div_id` keyed by its item's data key, never its position,
//! so a row's hover state and transition follow the item when it moves up or
//! down. The text field sits outside the bullseye tree in a plain egui panel
//! until M4 measures stock egui widgets in the same frame.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;

use bullseye::{Builder, NodeUi, Opacity, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

const ROW: Color32 = Color32::from_gray(245);
const ROW_HOVER: Color32 = Color32::from_gray(229);
const BORDER: Color32 = Color32::from_gray(208);
const MUTED: Color32 = Color32::from_gray(110);
const INK: Color32 = Color32::from_gray(20);
const ACTION_HOVER: Color32 = Color32::from_gray(208);
const ACTION_PRESSED: Color32 = Color32::from_gray(188);

const FAST: Duration = Duration::from_millis(150);

/// An item's identity, which survives reordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemKey(u32);

pub struct Item {
    pub key: ItemKey,
    pub title: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

pub struct State {
    pub items: Vec<Item>,
    pub draft: String,
    next_key: u32,
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            items: Vec::new(),
            draft: String::new(),
            next_key: 0,
        };
        for title in ["Lay it out", "Hover over it", "Reorder it", "Ship it"] {
            state.add(title);
        }
        state
    }
}

impl State {
    pub fn add(&mut self, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            return;
        }
        let key = ItemKey(self.next_key);
        self.next_key += 1;
        self.items.push(Item {
            key,
            title: title.to_owned(),
        });
    }

    pub fn remove(&mut self, key: ItemKey) {
        self.items.retain(|item| item.key != key);
    }

    pub fn shift(&mut self, key: ItemKey, direction: Direction) {
        let Some(at) = self.items.iter().position(|item| item.key == key) else {
            return;
        };
        let to = match direction {
            Direction::Up => at.checked_sub(1),
            Direction::Down => Some(at + 1).filter(|&to| to < self.items.len()),
        };
        if let Some(to) = to {
            self.items.swap(at, to);
        }
    }
}

/// A draw closure that runs `act` on the application when the node is clicked.
fn on_click(
    act: impl FnOnce(&mut State) + 'static,
) -> impl FnOnce(&mut State, &mut egui::Ui) + 'static {
    move |state, ui| {
        if ui.node_response().clicked() {
            act(state);
            ui.ctx().request_repaint();
        }
    }
}

/// A small text button.
///
/// ```html
/// <button class="px-2 py-0.5 rounded text-[#6e6e6e] hover:bg-[#d0d0d0] hover:text-[#141414]
///                active:bg-[#bcbcbc] transition duration-150">Up</button>
/// ```
fn action() -> Tw {
    tw().px(2.0)
        .py(0.5)
        .rounded()
        .text(MUTED)
        .hover(|p| p.bg(ACTION_HOVER).text(INK))
        .active(|p| p.bg(ACTION_PRESSED))
        .transition(FAST)
}

/// One row. The shift buttons that cannot move the row are inert and dimmed;
/// Remove is revealed while the row, or anything in it, is hovered.
///
/// ```html
/// <li class="group flex items-center gap-2 h-10 px-3 rounded-md border border-[#d0d0d0]
///            bg-[#f5f5f5] hover:bg-[#e5e5e5] transition duration-150">
///   <span class="grow truncate">Lay it out</span>
///   <button class="px-2 py-0.5 rounded ...">Up</button>
///   <button class="px-2 py-0.5 rounded ... inert opacity-30">Down</button>
///   <button class="px-2 py-0.5 rounded ... opacity-0 group-hover:opacity-100">Remove</button>
/// </li>
/// ```
fn row(b: &mut B<'_, '_, State>, item: &Item, position: Position) {
    let key = item.key;
    let movable = |tw: Tw, blocked_at: Position| {
        if position == blocked_at || position == Position::Only {
            tw.inert().opacity(Opacity::new(0.3))
        } else {
            tw
        }
    };
    b.div_id(
        key,
        tw().flex()
            .items_center()
            .gap(2.0)
            .h(10.0)
            .px(3.0)
            .rounded_md()
            .border(1.0)
            .border_color(BORDER)
            .bg(ROW)
            .group()
            .hover(|p| p.bg(ROW_HOVER))
            .transition(FAST),
        bullseye::none,
        |b| {
            b.text(tw().grow().truncate(), &item.title);
            b.span(
                "up",
                movable(action(), Position::First),
                "Up",
                on_click(move |state| state.shift(key, Direction::Up)),
            );
            b.span(
                "down",
                movable(action(), Position::Last),
                "Down",
                on_click(move |state| state.shift(key, Direction::Down)),
            );
            b.span(
                "remove",
                action()
                    .opacity(Opacity::TRANSPARENT)
                    .group_hover(|p| p.opacity(Opacity::OPAQUE)),
                "Remove",
                on_click(move |state| state.remove(key)),
            );
        },
    );
}

/// Where a row sits in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    First,
    Middle,
    Last,
    Only,
}

impl Position {
    fn of(index: usize, len: usize) -> Self {
        match (index, len) {
            (_, 1) => Self::Only,
            (0, _) => Self::First,
            (i, n) if i + 1 == n => Self::Last,
            _ => Self::Middle,
        }
    }
}

/// ```html
/// <ul class="flex flex-col gap-1 w-full max-w-[480px] mx-auto p-4">
///   <li>...</li>
///   <li class="py-2 text-center text-[#6e6e6e]">Nothing to do</li>
/// </ul>
/// ```
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    b.div(
        tw().flex()
            .flex_col()
            .gap(1.0)
            .w_full()
            .max_w_px(480.0)
            .mx_auto()
            .p(4.0),
        |b| {
            if state.items.is_empty() {
                b.text(tw().py(2.0).text(MUTED).text_center(), "Nothing to do");
            }
            for (index, item) in state.items.iter().enumerate() {
                row(b, item, Position::of(index, state.items.len()));
            }
        },
    );
}

fn add_bar(ui: &mut egui::Ui, state: &mut State) {
    ui.horizontal(|ui| {
        let field =
            ui.add(egui::TextEdit::singleline(&mut state.draft).hint_text("What needs doing?"));
        let submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("Add").clicked() || submitted {
            let draft = std::mem::take(&mut state.draft);
            state.add(&draft);
            field.request_focus();
        }
    });
}

fn main() -> eframe::Result {
    let mut state = State::default();
    let mut layout_state = bullseye::LayoutState::new().with_base_text(common::base_text());
    eframe::run_ui_native("todo", eframe::NativeOptions::default(), move |ui, _| {
        ui.ctx().set_theme(egui::Theme::Light);
        egui::Panel::top("add").show(ui, |ui| add_bar(ui, &mut state));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
            .show(ui, |ui| {
                let area = ui.max_rect();
                let built = layout_state.build(ui.ctx(), |b| layout(b, &state));
                built.layout(area).draw(&mut state, ui);
            });
    })
}
