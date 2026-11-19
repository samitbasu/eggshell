//! The integration tests, built as one binary: each test crate links egui and
//! friends statically, so one binary per file multiplied link time and disk use.

#![allow(
    clippy::duplicate_mod,
    reason = "each example includes the shared examples/common module"
)]
#![cfg_attr(
    not(debug_assertions),
    allow(
        unused_imports,
        dead_code,
        reason = "the release step builds without the debug-only tests these serve"
    )
)]

mod accordion_example;
mod autosuggest;
mod badge;
mod blur_cause;
mod borrowed_app;
mod button_options;
mod ch;
mod chat_shell_demo;
mod children;
mod choices;
mod clipboard;
mod clipboard_example;
mod collapsible;
mod combobox_parts;
mod commands;
mod compile_fail;
mod continuity;
mod contract;
mod contract_gaps;
mod control_size;
mod controls;
mod debug;
mod dnd_gallery;
mod dnd_handle;
mod dnd_preview;
mod dnd_reorder;
mod dnd_zones;
mod docs_embed;
mod draw;
mod duplicate_key;
mod edits;
mod escape_field;
mod field_box;
mod field_focus;
mod field_focus_example;
mod field_text;
mod focus;
mod focus_on_appear;
mod focus_options;
mod focus_select;
mod font_weights;
mod form_demo;
mod fractions;
mod fuzz;
mod gallery;
mod handlers;
mod headless;
mod hello;
mod hit_route;
mod hover_anchored;
mod hover_card;
mod icons;
mod inert_look;
mod interaction;
mod invisible;
mod kaolin_sidebar;
mod known_gaps_demo;
mod layout;
mod layout_animation;
mod layout_dnd;
mod layout_focus;
mod layout_scroll;
mod line_clamp;
mod listbox_anchor;
mod long_list_example;
mod markdown;
mod menu;
mod menu_offset;
mod modal;
mod mount;
mod named_layouts;
mod nested_layouts;
mod node_controls;
mod numbers;
mod overlay;
mod overlay_input;
mod owned;
mod owned_choice;
mod owned_number;
mod pop_out;
mod popup_style;
mod ports;
mod presence;
mod press_bubble;
mod prims;
mod progress;
mod reduced_motion;
mod refused;
mod regions;
mod release;
mod reserve;
mod responsive_example;
mod roving;
#[cfg(all(feature = "screenshots", target_os = "linux"))]
mod screenshot;
mod scroll;
mod scroll_keys;
mod select;
mod selection;
mod shadow_pixels;
mod sizing;
mod skeleton;
mod slider_header;
#[cfg(feature = "snapshots")]
mod snapshots;
mod spans;
mod splitter;
mod state;
mod state_stable;
mod style;
mod style_switch;
mod styled;
mod support;
mod swatch_picker;
mod sweep;
mod table;
mod tabular_nums;
mod text_field_ring;
mod theme_toggle;
mod todo_demo;
mod toggle_group;
mod tone;
mod tooltips;
mod touch_scroll;
mod typeset;
mod underline;
mod value_widgets;
mod vlist;
mod vlist_keys_from;
mod vlist_nav;
mod vlist_ports;
mod widgets;
