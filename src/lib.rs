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
mod debug;
mod draw;
mod focus;
mod key;
mod measure;
mod pipeline;
pub mod prim;
mod sizing;
pub mod style;
mod text;
mod tw;
pub mod widget;

pub use builder::{Builder, none};
pub use debug::Debug;
pub use draw::NodeUi;
pub use focus::focus_within;
pub use measure::{Measure, fill};
pub use pipeline::{Built, LayoutState, NodeKind, Placed, PlacedNode};
pub use prim::Icon;
pub use sizing::Width;
pub use text::{
    FontStyle, TextAlign, TextCase, TextClasses, TextDecoration, TextSpec, TextStyle, Tracking,
    WhiteSpace,
};
pub use tw::{Angle, BoxPaint, Divide, Flip, Opacity, Paint, Progress, Ring, Sides, Track, Tw, tw};
