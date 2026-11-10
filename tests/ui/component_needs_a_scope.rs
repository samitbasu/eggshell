#![allow(dead_code)]
use bullseye::{Builder, Scope, tw};

fn component(s: &mut Scope<'_, '_, ()>) {
    s.span("title", tw(), "Title");
}

fn plain_builder_can_not_place_a_component(b: &mut Builder<'_, '_, ()>) {
    component(b);
}

fn a_scope_places_it(b: &mut Builder<'_, '_, ()>) {
    b.scope("title", |s| component(s));
}

fn main() {}
