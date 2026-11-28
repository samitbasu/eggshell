//! The interactive ports (button, custom_background, sidebar) through real
//! pointer events: clicks reach the draw closures and variants repaint.

use eggshell_ui::testing::{Driver, Target};
use egui::vec2;

use eggshell_examples::common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/button.rs"]
mod button;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/custom_background.rs"]
mod custom_background;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/sidebar.rs"]
mod sidebar;

use crate::support;

use support::fill_of;

mod buttons {
    use super::*;
    use button::{Fills, State};

    fn harness() -> Driver<State> {
        support::driver(
            vec2(400.0, 520.0),
            common::base_text(egui::Theme::Light),
            State::default(),
            button::view,
        )
        .build()
    }

    fn fills(d: &Driver<State>) -> Fills {
        Fills::of(&common::tokens(d.ctx()))
    }

    #[test]
    fn a_button_counts_clicks_and_hover_and_press_repaint_it() {
        let mut d = harness();
        let node = "/\"button\"";
        assert_eq!(fill_of(&d, node), Some(fills(&d).rest));

        d.hover(Target::suffix(node));
        assert_eq!(fill_of(&d, node), Some(fills(&d).hover));

        let at = d.rect(Target::suffix(node)).center();
        d.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        })
        .frames(2);
        assert_eq!(fill_of(&d, node), Some(fills(&d).pressed));
        d.pointer_up(at).event(egui::Event::PointerGone).frames(2);

        assert_eq!(d.app().clicks, 1);
        d.click(Target::suffix(node));
        assert_eq!(d.app().clicks, 2);
    }

    #[test]
    fn a_selectable_button_toggles_and_keeps_its_selected_fill_under_the_pointer() {
        let mut d = harness();
        let node = "/\"selectable\"";
        let t = common::tokens(d.ctx());
        assert!(
            fill_of(&d, node).is_none_or(|fill| fill == egui::Color32::TRANSPARENT),
            "a toggle is transparent at rest"
        );

        d.hover(Target::suffix(node));
        assert_eq!(fill_of(&d, node), Some(t.muted));

        d.click(Target::suffix(node)).frames(3);
        assert!(d.app().selected);
        assert_eq!(fill_of(&d, node), Some(t.accent));
        assert_eq!(
            d.app().clicks,
            0,
            "precondition: it did not press the counting button"
        );

        d.click(Target::suffix(node)).frames(3);
        assert!(!d.app().selected);
    }
}

mod backgrounds {
    use super::*;
    use custom_background::{BUTTON_HOVER, DEFAULT_FILL, State};

    fn harness() -> Driver<State> {
        support::driver(
            vec2(1400.0, 700.0),
            common::base_text(egui::Theme::Light),
            State::default(),
            custom_background::view,
        )
        .build()
    }

    #[test]
    fn the_buttons_row_senses_clicks_and_hover() {
        let mut d = harness();
        let node = "/\"button\"";
        assert_eq!(fill_of(&d, node), Some(DEFAULT_FILL));
        d.hover(Target::suffix(node));
        assert_eq!(fill_of(&d, node), Some(BUTTON_HOVER));

        d.click(Target::suffix(node));
        d.click(Target::suffix("/\"clickable\""));
        d.click(Target::suffix("/\"clickable transparent\""));
        assert_eq!(d.app().clicks, 3);
    }

    #[test]
    fn selectable_buttons_share_one_toggle() {
        let mut d = harness();
        d.click(Target::suffix("/\"selectable\""));
        assert!(d.app().selected);
        d.click(Target::suffix("/\"ext selectable\""));
        assert!(!d.app().selected);
        assert_eq!(d.app().clicks, 0);
    }

    #[test]
    fn the_ext_clickable_button_gains_a_ring_on_hover() {
        let mut d = harness();
        let ring_around = |d: &Driver<State>| {
            let node = d.rect(Target::suffix("/\"clickable\""));
            d.harness().output().shapes.iter().any(|clipped| {
                matches!(&clipped.shape, egui::epaint::Shape::Rect(rect)
                    if rect.rect == node && rect.stroke.width == 8.0
                        && rect.stroke_kind == egui::StrokeKind::Outside)
            })
        };
        assert!(!ring_around(&d), "precondition: no ring at rest");
        d.hover(Target::suffix("/\"clickable\""));
        assert!(ring_around(&d));
    }
}

mod navigation {
    use super::*;
    use sidebar::{ENTRY_HOVER, ENTRY_SELECTED, PILL, PILL_HOVER, Route, State};

    fn harness() -> Driver<State> {
        support::driver(
            vec2(800.0, 800.0),
            common::base_text(egui::Theme::Light),
            State::default(),
            sidebar::view,
        )
        .build()
    }

    const SELECTED: &str = "/Example(\"Flex\")";
    const OTHER: &str = "/Example(\"Grid\")";

    #[test]
    fn the_selected_entry_is_base_paint_and_hover_lights_the_others() {
        let mut d = harness();
        assert_eq!(fill_of(&d, SELECTED), Some(ENTRY_SELECTED));

        d.hover(Target::suffix(OTHER)).frames(10);
        assert_eq!(fill_of(&d, OTHER), Some(ENTRY_HOVER));

        d.hover(Target::suffix(SELECTED)).frames(10);
        assert_eq!(fill_of(&d, SELECTED), Some(ENTRY_SELECTED));
    }

    #[test]
    fn clicking_an_entry_or_a_pill_navigates() {
        let mut d = harness();
        d.click(Target::suffix(OTHER)).frames(2);
        assert_eq!(d.app().route, Route::Example("Grid"));
        assert_eq!(fill_of(&d, OTHER), Some(ENTRY_SELECTED));
        assert_ne!(fill_of(&d, SELECTED), Some(ENTRY_SELECTED));

        let pill = "/Crate(\"dnd\")";
        assert_eq!(fill_of(&d, pill), Some(PILL));
        d.hover(Target::suffix(pill));
        assert_eq!(fill_of(&d, pill), Some(PILL_HOVER));
        d.click(Target::suffix(pill)).frames(2);
        assert_eq!(d.app().route, Route::Crate("dnd"));
        assert_eq!(fill_of(&d, pill), Some(ENTRY_SELECTED));
    }

    #[test]
    fn the_link_is_pinned_to_the_bottom_of_the_sidebar() {
        let d = harness();
        let nav = d.rect(Target::suffix("/\"github\""));
        let last_pill = d.rect(Target::suffix("/Crate(\"pull_to_refresh\")"));
        assert!(nav.min.y > last_pill.max.y, "below the pills");
        let sidebar = support::nodes(&d)
            .into_iter()
            .find(|(path, _)| path == "/<div>")
            .expect("the sidebar, the first anonymous div")
            .1;
        assert_eq!(
            nav.max.y + 8.0,
            sidebar.max.y,
            "pinned by the sidebar's padding"
        );
    }
}
