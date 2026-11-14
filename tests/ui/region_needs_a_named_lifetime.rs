#![allow(dead_code)]
use eggshell_ui::{Builder, tw};

struct App {
    title: String,
}

fn declares_a_region_without_naming_the_lifetime(app: &App, b: &mut Builder<'_, '_, App>) {
    b.region("host", tw(), move |b, _| {
        b.text(tw(), app.title.clone());
    });
}

fn names_it<'f>(app: &'f App, b: &mut Builder<'_, 'f, App>) {
    b.region("host", tw(), move |b, _| {
        b.text(tw(), app.title.clone());
    });
}

fn main() {}
