#![deny(unused_must_use)]
use bullseye::Builder;

fn dropped_without_a_terminator(b: &mut Builder<'_, '_, ()>) {
    b.button("button", "Save");
    b.checkbox("checkbox", "Agree", false);
    b.switch("switch", "Alerts", false);
    b.radio_group("radio", 0, [(0, "Zero")]);
    b.segmented("segmented", 0, [(0, "Zero")]);
    b.slider("slider", 0.5, 0.0..=1.0);
    b.select("select", 0, [(0, "Zero")]);
    b.stepper("stepper", 1.0, 0.0..=9.0);
    b.text_field("field", "");
    b.toggle("toggle", "Bold", false);
    b.custom("custom", bullseye::tw());
}

fn terminated(b: &mut Builder<'_, '_, ()>) {
    b.button("button", "Save").add();
    b.checkbox("checkbox", "Agree", false).on_change(|_, _| {});
}

fn main() {}
