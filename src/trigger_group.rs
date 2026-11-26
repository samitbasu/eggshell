//! A row of triggers that each open their own floating content, one at a
//! time: a navigation menu's triggers and their panels, a menubar's and
//! their menus.
//!
//! Both share one rule of moving along the row, [`switch`]: while one is
//! open, arriving at a neighbouring trigger, by the pointer or by the arrow
//! keys, opens the neighbour's instead. How each is opened in the first place
//! (a hover's delay, a click) and how it closes are the row's own.

use egui::Id;

/// What arriving at `trigger` opens, while `open` is the row's trigger whose
/// content is open: `trigger` itself if another one's is, so the open one
/// moves along the row; nothing if none is, or if `trigger` is the open one.
pub(crate) fn switch(open: Option<Id>, trigger: Id) -> Option<Id> {
    open.filter(|&open| open != trigger).map(|_| trigger)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    #[test]
    fn arriving_at_a_neighbour_while_one_is_open_opens_the_neighbour() {
        assert_eq!(switch(Some(id("a")), id("b")), Some(id("b")));
    }

    #[test]
    fn arriving_with_none_open_or_at_the_open_one_opens_nothing() {
        assert_eq!(switch(None, id("b")), None);
        assert_eq!(switch(Some(id("b")), id("b")), None);
    }
}
