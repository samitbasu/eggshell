//! A window with a label; the smallest thing `cargo xtask example` can run.
//! The example is `eggshell_examples::hello`.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

use eggshell_examples::{common, hello};

fn main() -> eframe::Result {
    common::run("hello", (), hello::view)
}
