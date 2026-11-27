//! A button that counts its clicks: the smallest example with state. The
//! example is `eggshell_examples::counter`.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

use eggshell_examples::{common, counter};

fn main() -> eframe::Result {
    common::run_app("counter", counter::State::default(), counter::view)
}
