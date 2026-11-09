//! The integration tests, built as one binary: each test crate links egui and
//! friends statically, so one binary per file multiplied link time and disk use.

#![allow(
    clippy::duplicate_mod,
    reason = "each example includes the shared examples/common module"
)]

mod chat_shell_demo;
mod compile_fail;
mod contract;
mod contract_gaps;
mod controls;
mod debug;
mod draw;
mod focus;
mod form_demo;
mod gallery;
mod hello;
mod interaction;
mod kaolin_sidebar;
mod known_gaps_demo;
mod layout;
mod owned;
mod owned_choice;
mod owned_number;
mod popup_style;
mod ports;
mod prims;
mod selection;
mod sizing;
#[cfg(feature = "snapshots")]
mod snapshots;
mod state;
mod state_stable;
mod style;
mod style_switch;
mod support;
mod theme_toggle;
mod themed_examples;
mod todo_demo;
mod widgets;
