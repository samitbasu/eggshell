#![deny(missing_docs)]
//! Bullseye: taffy lays out, egui draws and interacts.
//!
//! This crate is under construction. Each frame is planned as three phases:
//!
//! 1. **Build**: walk application state with a nested builder and emit a
//!    taffy tree whose identified nodes carry key paths, measures and draw
//!    closures.
//! 2. **Layout**: taffy solves every root to absolute rectangles.
//! 3. **Draw**: a pre-order walk paints boxes and content, then opens a child
//!    [`egui::Ui`] per identified node and runs its draw closure.
//!
//! The design brief lives in `docs/design.md`.

mod builder;
mod draw;
mod key;
mod measure;
mod pipeline;
mod prim;
#[cfg(test)]
mod spikes;
mod text;
mod tw;

pub use builder::{Builder, none};
pub use measure::{Measure, fill};
pub use pipeline::{Built, LayoutState, NodeKind, Placed, PlacedNode};
pub use prim::Icon;
pub use text::{
    FontStyle, TextAlign, TextCase, TextClasses, TextDecoration, TextSpec, TextStyle, Tracking,
    WhiteSpace,
};
pub use tw::{Angle, BoxPaint, Divide, Flip, Opacity, Ring, Sides, Track, Tw, tw};
