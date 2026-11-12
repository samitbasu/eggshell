//! The integration tests, built as one binary: each test crate links egui and
//! friends statically, so one binary per file multiplied link time and disk use.

#![allow(
    clippy::duplicate_mod,
    reason = "each example includes the shared examples/common module"
)]

mod autosuggest;
mod button_options;
mod ch;
mod chat_shell_demo;
mod children;
mod choices;
mod compile_fail;
mod continuity;
mod contract;
mod contract_gaps;
mod controls;
mod debug;
mod dnd_gallery;
mod dnd_preview;
mod dnd_reorder;
mod dnd_zones;
mod draw;
mod duplicate_key;
mod edits;
mod escape_field;
mod focus;
mod focus_on_appear;
mod form_demo;
mod fractions;
mod fuzz;
mod gallery;
mod handlers;
mod headless;
mod hello;
mod inert_look;
mod interaction;
mod kaolin_sidebar;
mod known_gaps_demo;
mod layout;
mod layout_animation;
mod line_clamp;
mod modal;
mod named_layouts;
mod overlay;
mod overlay_input;
mod owned;
mod owned_choice;
mod owned_number;
mod popup_style;
mod ports;
mod prims;
mod progress;
mod scroll;
mod scroll_keys;
mod select;
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
mod sweep;
mod tabular_nums;
mod theme_toggle;
mod todo_demo;
mod toggle_group;
mod tooltips;
mod value_widgets;
mod widgets;
