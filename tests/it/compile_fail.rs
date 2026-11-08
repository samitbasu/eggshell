//! `Paint` is the paint-only builder behind state variants: layout and font
//! size classes do not exist on it, so a state change never needs another
//! layout pass.

#[test]
fn paint_has_no_layout_methods() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
