#![allow(dead_code)]
use eggshell_ui::{Builder, tw};

fn a_span_has_no_links(b: &mut Builder<'_, '_, ()>) {
    b.span("plain", tw(), "Plain").on_link(|_, _| {});
}

fn spans_take_them(b: &mut Builder<'_, '_, ()>) {
    b.spans("rich", tw(), "Rich").on_link(|_, _| {});
}

fn main() {}
