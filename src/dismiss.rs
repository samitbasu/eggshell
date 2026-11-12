//! Dismissal: which open overlays a frame's input closes.
//!
//! [`dismissals`] is the one place the rules live. It is a pure function of
//! the open overlays, bottom first, and of what the frame's input did, so the
//! rules are tested without egui. The draw runs it once, after the overlays
//! are drawn, and calls the `on_dismiss` of each overlay it names: the
//! library never changes application state itself.
//!
//! The rules are in `docs/overlays.md`, section 4.

use egui::{Pos2, Rect};

use crate::overlay::{Band, DismissOn, Outside};

/// Where keyboard focus is, relative to an overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FocusAt {
    /// In the overlay, in an overlay nested in it, or on its anchor.
    Inside,
    /// On a widget outside all of those.
    Elsewhere,
    /// Nowhere, or the window does not have it.
    #[default]
    Nowhere,
}

/// Whether an overlay was declared for the first time this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opening {
    Opened,
    Open,
}

/// An open, attached overlay as dismissal sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Layer {
    /// The overlay this one is declared in, as an index in the slice.
    pub(crate) parent: Option<usize>,
    pub(crate) rect: Rect,
    /// The anchor, if pressing it is the trigger's own business.
    pub(crate) trigger: Option<Rect>,
    pub(crate) band: Band,
    pub(crate) dismiss_on: DismissOn,
    pub(crate) outside: Outside,
    pub(crate) opening: Opening,
    pub(crate) focus_before: FocusAt,
    pub(crate) focus_now: FocusAt,
}

/// What the frame's input did.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Input {
    /// Where a pointer button went down this frame.
    pub(crate) press: Option<Pos2>,
    pub(crate) escape: bool,
    /// A drag gesture is live.
    pub(crate) drag_live: bool,
    /// The window lost focus.
    pub(crate) window_blurred: bool,
}

/// Why an overlay is dismissed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    Outside,
    Esc,
    FocusOut,
    WindowBlur,
    /// Another overlay opened while this swallowing one was open.
    Exclusive,
}

/// An overlay to dismiss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dismissal {
    pub(crate) layer: usize,
    pub(crate) cause: Cause,
}

#[derive(Default)]
struct Found(Vec<Dismissal>);

impl Found {
    fn add(&mut self, layer: usize, cause: Cause) {
        if self.0.iter().all(|found| found.layer != layer) {
            self.0.push(Dismissal { layer, cause });
        }
    }
}

/// The overlays `input` dismisses, topmost first. `layers` are the attached
/// overlays, the one opened first at the bottom, each after the overlay it is
/// declared in.
///
/// 1. Press, Esc and focus are not judged while a drag is live.
/// 2. A press dismisses, top-down, each overlay that does not contain it,
///    and stops at the first that does. An overlay contains a press inside
///    its box, inside an overlay nested in it, or, if it passes presses
///    through, on its anchor (the trigger toggles). An overlay that swallows
///    presses ends the walk, but for the overlays it is nested in: the press
///    went to it and no further.
/// 3. Esc goes to the topmost overlay that takes it.
/// 4. Focus moving from inside an overlay to a widget outside it dismisses
///    it; focus becoming none does not.
/// 5. The window losing focus dismisses every overlay that dismisses on focus
///    leaving.
/// 6. An overlay opening while a swallowing overlay is open, that is not
///    nested in it, dismisses the swallowing one: menus are exclusive.
///    Tooltips do not count.
pub(crate) fn dismissals(layers: &[Layer], input: &Input) -> Vec<Dismissal> {
    let top_down = || (0..layers.len()).rev();
    let mut found = Found::default();
    if input.window_blurred {
        for at in top_down().filter(|&at| layers[at].dismiss_on.focus_out()) {
            found.add(at, Cause::WindowBlur);
        }
    }
    if !input.drag_live {
        if let Some(press) = input.press {
            press_outside(layers, press, &mut found);
        }
        if input.escape
            && let Some(at) = top_down().find(|&at| layers[at].dismiss_on.esc())
        {
            found.add(at, Cause::Esc);
        }
        for at in top_down().filter(|&at| leaving(&layers[at])) {
            found.add(at, Cause::FocusOut);
        }
    }
    exclusive(layers, &mut found);
    let mut found = found.0;
    found.sort_by_key(|dismissal| std::cmp::Reverse(dismissal.layer));
    found
}

fn leaving(layer: &Layer) -> bool {
    layer.dismiss_on.focus_out()
        && layer.focus_before == FocusAt::Inside
        && layer.focus_now == FocusAt::Elsewhere
}

/// Whether `ancestor` is one of the overlays `layer` is nested in.
fn nested_in(layers: &[Layer], layer: usize, ancestor: usize) -> bool {
    std::iter::successors(layers[layer].parent, |&at| layers[at].parent).any(|at| at == ancestor)
}

fn contains(layers: &[Layer], at: usize, press: Pos2) -> bool {
    let layer = &layers[at];
    layer.rect.contains(press)
        || (layer.outside == Outside::PassThrough
            && layer.trigger.is_some_and(|trigger| trigger.contains(press)))
        || (0..layers.len())
            .any(|other| nested_in(layers, other, at) && layers[other].rect.contains(press))
}

fn press_outside(layers: &[Layer], press: Pos2, found: &mut Found) {
    let mut swallowed_by = None;
    for at in (0..layers.len()).rev() {
        if swallowed_by.is_some_and(|by| !nested_in(layers, by, at)) {
            break;
        }
        if contains(layers, at, press) {
            break;
        }
        let layer = &layers[at];
        if layer.dismiss_on.outside() {
            found.add(at, Cause::Outside);
        }
        if layer.outside == Outside::Swallow && swallowed_by.is_none() {
            swallowed_by = Some(at);
        }
    }
}

fn exclusive(layers: &[Layer], found: &mut Found) {
    for opened in (0..layers.len())
        .filter(|&at| layers[at].opening == Opening::Opened && layers[at].band != Band::Tooltip)
    {
        for below in 0..opened {
            let layer = &layers[below];
            if layer.outside == Outside::Swallow
                && layer.dismiss_on == DismissOn::Default
                && !nested_in(layers, opened, below)
            {
                found.add(below, Cause::Exclusive);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::{pos2, vec2};

    use super::*;

    fn rect(x: f32, y: f32) -> Rect {
        Rect::from_min_size(pos2(x, y), vec2(100.0, 100.0))
    }

    /// A popover at `x`: passes presses through, anchored to a trigger just
    /// above it.
    fn popover(x: f32) -> Layer {
        Layer {
            parent: None,
            rect: rect(x, 100.0),
            trigger: Some(Rect::from_min_size(pos2(x, 60.0), vec2(100.0, 30.0))),
            band: Band::Popup,
            dismiss_on: DismissOn::Default,
            outside: Outside::PassThrough,
            opening: Opening::Open,
            focus_before: FocusAt::Nowhere,
            focus_now: FocusAt::Nowhere,
        }
    }

    fn menu(x: f32) -> Layer {
        Layer {
            outside: Outside::Swallow,
            ..popover(x)
        }
    }

    fn nested(layer: Layer, parent: usize) -> Layer {
        Layer {
            parent: Some(parent),
            trigger: Some(rect(0.0, 0.0)),
            ..layer
        }
    }

    fn press(at: Pos2) -> Input {
        Input {
            press: Some(at),
            ..Input::default()
        }
    }

    const ESC: Input = Input {
        press: None,
        escape: true,
        drag_live: false,
        window_blurred: false,
    };

    fn closed(layers: &[Layer], input: &Input) -> Vec<usize> {
        dismissals(layers, input)
            .into_iter()
            .map(|dismissal| dismissal.layer)
            .collect()
    }

    const INSIDE: Pos2 = pos2(50.0, 150.0);
    const ON_TRIGGER: Pos2 = pos2(50.0, 70.0);
    const PAGE: Pos2 = pos2(350.0, 350.0);

    #[test]
    fn a_press_inside_dismisses_nothing_and_one_on_the_page_dismisses() {
        let layers = [popover(0.0)];
        assert_eq!(closed(&layers, &press(INSIDE)), Vec::<usize>::new());
        assert_eq!(closed(&layers, &press(PAGE)), vec![0]);
    }

    #[test]
    fn the_trigger_of_a_popover_is_inside_so_that_its_own_click_toggles() {
        let layers = [popover(0.0)];
        assert!(!layers[0].rect.contains(ON_TRIGGER), "precondition");
        assert_eq!(closed(&layers, &press(ON_TRIGGER)), Vec::<usize>::new());
    }

    #[test]
    fn the_trigger_of_a_menu_is_outside_since_the_menu_covers_it() {
        let layers = [menu(0.0)];
        assert_eq!(closed(&layers, &press(ON_TRIGGER)), vec![0]);
    }

    #[test]
    fn a_press_walks_down_and_stops_at_the_first_overlay_that_contains_it() {
        let layers = [popover(0.0), popover(150.0), popover(300.0)];
        let in_first = pos2(50.0, 150.0);
        assert!(layers[0].rect.contains(in_first) && !layers[1].rect.contains(in_first));
        assert_eq!(closed(&layers, &press(in_first)), vec![2, 1]);
        assert_eq!(closed(&layers, &press(PAGE)), vec![2, 1, 0]);
    }

    #[test]
    fn a_press_in_a_child_is_inside_its_parent_and_one_in_the_parent_closes_only_the_child() {
        let child = Layer {
            rect: rect(200.0, 200.0),
            ..nested(popover(0.0), 0)
        };
        let layers = [popover(0.0), child];
        let in_child = pos2(250.0, 250.0);
        assert!(!layers[0].rect.contains(in_child), "precondition");
        assert_eq!(closed(&layers, &press(in_child)), Vec::<usize>::new());
        assert_eq!(closed(&layers, &press(INSIDE)), vec![1]);
        assert_eq!(closed(&layers, &press(PAGE)), vec![1, 0], "child first");
    }

    #[test]
    fn a_swallowing_overlay_ends_the_walk_but_for_those_it_is_nested_in() {
        let sub = Layer {
            rect: rect(200.0, 200.0),
            ..nested(menu(0.0), 1)
        };
        let layers = [popover(300.0), menu(0.0), sub];
        assert_eq!(
            closed(&layers, &press(PAGE)),
            vec![2, 1],
            "the popover under the menu never sees the press"
        );
        let layers = [menu(300.0), popover(0.0)];
        assert_eq!(
            closed(&layers, &press(PAGE)),
            vec![1, 0],
            "a popover over a menu lets the press through to it"
        );
    }

    #[test]
    fn a_swallowing_overlay_that_does_not_dismiss_still_ends_the_walk() {
        let held = Layer {
            dismiss_on: DismissOn::EscOnly,
            ..menu(0.0)
        };
        let layers = [popover(300.0), held];
        assert_eq!(closed(&layers, &press(PAGE)), Vec::<usize>::new());
    }

    #[test]
    fn dismiss_on_chooses_what_dismisses() {
        let only_esc = Layer {
            dismiss_on: DismissOn::EscOnly,
            ..popover(0.0)
        };
        let never = Layer {
            dismiss_on: DismissOn::Never,
            ..popover(0.0)
        };
        assert_eq!(closed(&[only_esc], &press(PAGE)), Vec::<usize>::new());
        assert_eq!(closed(&[only_esc], &ESC), vec![0]);
        assert_eq!(closed(&[never], &press(PAGE)), Vec::<usize>::new());
        assert_eq!(closed(&[never], &ESC), Vec::<usize>::new());
    }

    #[test]
    fn esc_goes_to_the_topmost_overlay_that_takes_it() {
        let never = Layer {
            dismiss_on: DismissOn::Never,
            ..popover(300.0)
        };
        assert_eq!(closed(&[popover(0.0), popover(150.0)], &ESC), vec![1]);
        assert_eq!(closed(&[popover(0.0), never], &ESC), vec![0]);
    }

    #[test]
    fn esc_closes_only_the_child_of_a_nested_pair() {
        let layers = [popover(0.0), nested(popover(150.0), 0)];
        assert_eq!(closed(&layers, &ESC), vec![1]);
    }

    #[test]
    fn nothing_is_judged_while_a_drag_is_live() {
        let layers = [popover(0.0)];
        let live = |input: Input| Input {
            drag_live: true,
            ..input
        };
        assert_eq!(closed(&layers, &live(press(PAGE))), Vec::<usize>::new());
        assert_eq!(closed(&layers, &live(ESC)), Vec::<usize>::new());
        let leaving = [Layer {
            focus_before: FocusAt::Inside,
            focus_now: FocusAt::Elsewhere,
            ..popover(0.0)
        }];
        assert_eq!(
            closed(&leaving, &live(Input::default())),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn focus_moving_out_dismisses_and_focus_becoming_none_does_not() {
        let moved = |before, now| Layer {
            focus_before: before,
            focus_now: now,
            ..popover(0.0)
        };
        let idle = Input::default();
        assert_eq!(
            closed(&[moved(FocusAt::Inside, FocusAt::Elsewhere)], &idle),
            vec![0]
        );
        for (before, now) in [
            (FocusAt::Inside, FocusAt::Nowhere),
            (FocusAt::Inside, FocusAt::Inside),
            (FocusAt::Elsewhere, FocusAt::Elsewhere),
            (FocusAt::Nowhere, FocusAt::Elsewhere),
        ] {
            assert_eq!(closed(&[moved(before, now)], &idle), Vec::<usize>::new());
        }
        let esc_only = Layer {
            dismiss_on: DismissOn::EscOnly,
            ..moved(FocusAt::Inside, FocusAt::Elsewhere)
        };
        assert_eq!(closed(&[esc_only], &idle), Vec::<usize>::new());
    }

    #[test]
    fn the_window_losing_focus_dismisses_what_focus_leaving_would() {
        let blur = Input {
            window_blurred: true,
            ..Input::default()
        };
        let esc_only = Layer {
            dismiss_on: DismissOn::EscOnly,
            ..popover(300.0)
        };
        assert_eq!(closed(&[popover(0.0), esc_only], &blur), vec![0]);
        let live = Input {
            drag_live: true,
            ..blur
        };
        assert_eq!(closed(&[popover(0.0)], &live), vec![0], "even in a drag");
    }

    #[test]
    fn a_menu_is_dismissed_when_another_overlay_opens() {
        let opened = |layer: Layer| Layer {
            opening: Opening::Opened,
            ..layer
        };
        let idle = Input::default();
        assert_eq!(
            dismissals(&[menu(0.0), opened(popover(150.0))], &idle),
            vec![Dismissal {
                layer: 0,
                cause: Cause::Exclusive
            }]
        );
        assert_eq!(
            closed(&[popover(0.0), opened(popover(150.0))], &idle),
            Vec::<usize>::new(),
            "popovers coexist"
        );
        let tip = Layer {
            band: Band::Tooltip,
            ..opened(popover(150.0))
        };
        assert_eq!(closed(&[menu(0.0), tip], &idle), Vec::<usize>::new());
        let held = Layer {
            dismiss_on: DismissOn::Never,
            ..menu(0.0)
        };
        assert_eq!(
            closed(&[held, opened(popover(150.0))], &idle),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn a_nested_overlay_opening_does_not_dismiss_its_menu() {
        let sub = Layer {
            opening: Opening::Opened,
            ..nested(menu(150.0), 0)
        };
        let deeper = Layer {
            opening: Opening::Opened,
            ..nested(menu(150.0), 1)
        };
        assert_eq!(
            closed(&[menu(0.0), sub], &Input::default()),
            Vec::<usize>::new()
        );
        assert_eq!(
            closed(
                &[menu(0.0), nested(menu(150.0), 0), deeper],
                &Input::default()
            ),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn a_menu_opened_with_another_in_one_frame_loses_to_the_later() {
        let opened = |layer: Layer| Layer {
            opening: Opening::Opened,
            ..layer
        };
        assert_eq!(
            closed(&[opened(menu(0.0)), opened(menu(150.0))], &Input::default()),
            vec![0]
        );
    }

    #[test]
    fn one_overlay_is_dismissed_once_whatever_the_causes() {
        let layer = Layer {
            focus_before: FocusAt::Inside,
            focus_now: FocusAt::Elsewhere,
            ..popover(0.0)
        };
        let input = Input {
            press: Some(PAGE),
            escape: true,
            ..Input::default()
        };
        assert_eq!(closed(&[layer], &input), vec![0]);
    }
}
