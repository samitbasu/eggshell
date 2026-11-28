//! Overlays: a stock dropdown menu with a submenu, check and radio items, and
//! a tall list that flips to the side with room, clamps to it and scrolls.
//! The example is `eggshell_examples::menu`.

use eggshell_examples::{common, menu};

fn main() -> eframe::Result {
    common::run_app("menu", menu::State::default(), menu::view)
}
