//! A label: the smallest example.

use eggshell_ui::{Builder, tw};

/// Hello has no state.
pub type State = ();

pub fn view(b: &mut Builder<'_, '_, ()>, _: &State) {
    b.text(tw(), "Hello, eggshell");
}
