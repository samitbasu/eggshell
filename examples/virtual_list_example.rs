//! A virtual list of 100,000 items of differing heights.
//!
//! A port of egui_virtual_list's `virtual_list_example`
//! (github.com/lucasmerlin/hello_egui, `crates/egui_virtual_list`, MIT),
//! rebuilt with `virtual_list`. Each item is a framed label padded by a
//! height that follows from its index, as the original's seeded random
//! height does; the list measures each item when it is first built and
//! counts it at the estimate until then.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Scope, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How many items the list has.
pub const ITEMS: u32 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    List,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Item(pub u32);

pub struct State {
    pub items: Vec<Item>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            items: (0..ITEMS).map(Item).collect(),
        }
    }
}

/// The extra height of an item, 0 to 100 points, from its index alone: the
/// same item is always as tall.
pub fn extra(Item(index): Item) -> f32 {
    let mut seed = u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    seed ^= seed >> 29;
    seed = seed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    seed ^= seed >> 32;
    (seed % 101) as f32
}

/// ```html
/// <div class="mb-1 px-4 rounded-md border bg-card" style="padding-block: calc(8px + extra / 2)">Item 12</div>
/// ```
fn item(s: &mut Scope<'_, '_, State>, item: Item) {
    let t = common::tokens(s.ctx());
    s.div(
        tw().mb(1.0)
            .px(4.0)
            .py((8.0 + extra(item) / 2.0) / 4.0)
            .rounded_md()
            .border(1.0)
            .border_color(t.border)
            .bg(t.card),
        |s| s.text(tw(), format!("Item {}", item.0)),
    );
}

/// ```html
/// <div class="size-full p-2 overflow-y-auto">items</div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, state: &'f State) {
    b.virtual_list(Part::List, tw().size_full().p(2.0), &state.items)
        .estimate_px(70.0)
        .items(item);
}

fn main() -> eframe::Result {
    common::run_app("virtual_list_example", State::default(), layout)
}
