//! The integration tests, built as one binary: each test crate links egui and
//! friends statically, so one binary per file multiplied link time and disk use.

#![allow(
    clippy::duplicate_mod,
    reason = "each example includes the shared examples/common module"
)]

mod button_options;
mod chat_shell_demo;
mod children;
mod choices;
mod compile_fail;
mod contract;
mod contract_gaps;
mod controls;
mod debug;
mod draw;
mod focus;
mod form_demo;
mod gallery;
mod headless;
mod hello;
mod interaction;
mod kaolin_sidebar;
mod known_gaps_demo;
mod layout;
mod layout_animation;
mod owned;
mod owned_choice;
mod owned_number;
mod popup_style;
mod ports;
mod prims;
mod selection;
mod shadow_pixels;
mod sizing;
#[cfg(feature = "snapshots")]
mod snapshots;
mod state;
mod state_stable;
mod style;
mod style_switch;
mod support;
mod theme_toggle;
mod todo_demo;
mod toggle_group;
mod tooltips;
mod value_widgets;
mod widgets;
