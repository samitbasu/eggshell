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
//!
//! # Differences from CSS
//!
//! egui paints shapes one at a time into rectangular clip rects, so three
//! things CSS does are not available:
//!
//! - **No backdrop blur.** What is behind a layer cannot be blurred.
//! - **No rounded clipping.** `overflow_hidden` clips a subtree to the node's
//!   rectangle, not to its rounded shape. A child that reaches a rounded corner
//!   must be rounded itself: the owned slider's range is `rounded_full`.
//! - **No group opacity.** `opacity` and a colour's alpha fade each shape on its
//!   own, so overlapping shapes show through one another where CSS would fade
//!   the composited subtree. Disabled looks therefore pre-blend each colour
//!   against the opaque surface it sits on
//!   ([`Colors::faded_over`](contract::Colors::faded_over)) and are never
//!   translucent.

mod builder;
pub mod contract;
mod debug;
mod draw;
mod focus;
mod key;
mod measure;
pub mod owned;
mod pipeline;
pub mod prim;
mod sizing;
pub mod style;
mod text;
mod tw;
pub mod widget;

pub use builder::{Builder, none};
pub use contract::Style;
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
