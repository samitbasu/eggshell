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

#[cfg_attr(not(test), expect(dead_code, reason = "consumed by the builder in M1"))]
mod measure;
#[cfg(test)]
mod spikes;
