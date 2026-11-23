//! [`BoxTw`]: the classes a `has` rule may set.

use super::Tw;

/// TODO(prose): the classes of a [`has`](Tw::has) rule: layout and box paint,
/// and nothing that is text, behavior or state, so that a parent's rule about
/// what it contains cannot reach what its children inherit.
///
/// ```
/// use eggshell_ui::{Match, tw};
/// let field = tw().has(Match::slot("icon"), |b| b.pl(1.5));
/// ```
///
/// Text classes are not on it:
///
/// ```compile_fail
/// use eggshell_ui::{Match, tw};
/// let field = tw().has(Match::slot("icon"), |b| b.uppercase());
/// ```
#[derive(Clone, Debug)]
pub struct BoxTw(pub(super) Tw);

impl BoxTw {
    pub(super) fn blank() -> Self {
        Self(Tw::blank())
    }

    pub(super) fn styled(self, set: impl FnOnce(&mut super::Layout<'_>)) -> Self {
        Self(self.0.styled(set))
    }

    pub(super) fn spaced(self, set: impl FnOnce(&mut super::Layout<'_>)) -> Self {
        Self(self.0.spaced(set))
    }
}
