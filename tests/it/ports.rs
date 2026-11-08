//! The interactive ports (button, custom_background, sidebar) through real
//! pointer events: clicks reach the draw closures and variants repaint.

use egui::vec2;
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

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

use support::{Rig, click_node, fill_of, hover_node, rect_of};

fn settle<A>(harness: &mut Harness<'_, Rig<A>>) {
    harness.run_steps(3);
}

mod buttons {
    use super::*;
    use button::{Fills, State};

    fn harness() -> Harness<'static, Rig<State>> {
        support::harness(
            vec2(400.0, 520.0),
            common::base_text(egui::Theme::Light),
            State::default(),
            button::layout,
        )
    }

    fn fills(harness: &Harness<'static, Rig<State>>) -> Fills {
        Fills::of(&common::tokens(&harness.ctx))
    }

    #[test]
    fn a_button_counts_clicks_and_hover_and_press_repaint_it() {
        let mut harness = harness();
        let node = "/\"button\"";
        assert_eq!(fill_of(&harness, node), Some(fills(&harness).rest));

        hover_node(&mut harness, node);
        assert_eq!(fill_of(&harness, node), Some(fills(&harness).hover));

        let at = rect_of(&harness, node).center();
        harness.drag_at(at);
        settle(&mut harness);
        assert_eq!(fill_of(&harness, node), Some(fills(&harness).pressed));
        harness.drop_at(at);
        settle(&mut harness);

        assert_eq!(harness.state().app.clicks, 1);
        click_node(&mut harness, node);
        assert_eq!(harness.state().app.clicks, 2);
    }

    #[test]
    fn a_selectable_button_toggles_and_keeps_its_selected_fill_under_the_pointer() {
        let mut harness = harness();
        let node = "/\"selectable\"";
        let t = common::tokens(&harness.ctx);
        assert!(
            fill_of(&harness, node).is_none_or(|fill| fill == egui::Color32::TRANSPARENT),
            "a toggle is transparent at rest"
        );

        hover_node(&mut harness, node);
        assert_eq!(fill_of(&harness, node), Some(t.muted));

        click_node(&mut harness, node);
        settle(&mut harness);
        assert!(harness.state().app.selected);
        assert_eq!(fill_of(&harness, node), Some(t.accent));
        assert_eq!(
            harness.state().app.clicks,
            0,
            "precondition: it did not press the counting button"
        );

        click_node(&mut harness, node);
        settle(&mut harness);
        assert!(!harness.state().app.selected);
    }
}

mod backgrounds {
    use super::*;
    use custom_background::{BUTTON_HOVER, DEFAULT_FILL, State};

    fn harness() -> Harness<'static, Rig<State>> {
        support::harness(
            vec2(1400.0, 700.0),
            common::base_text(egui::Theme::Light),
            State::default(),
            custom_background::layout,
        )
    }

    #[test]
    fn the_buttons_row_senses_clicks_and_hover() {
        let mut harness = harness();
        let node = "/\"button\"";
        assert_eq!(fill_of(&harness, node), Some(DEFAULT_FILL));
        hover_node(&mut harness, node);
        assert_eq!(fill_of(&harness, node), Some(BUTTON_HOVER));

        click_node(&mut harness, node);
        click_node(&mut harness, "/\"clickable\"");
        click_node(&mut harness, "/\"clickable transparent\"");
        assert_eq!(harness.state().app.clicks, 3);
    }

    #[test]
    fn selectable_buttons_share_one_toggle() {
        let mut harness = harness();
        click_node(&mut harness, "/\"selectable\"");
        assert!(harness.state().app.selected);
        click_node(&mut harness, "/\"ext selectable\"");
        assert!(!harness.state().app.selected);
        assert_eq!(harness.state().app.clicks, 0);
    }

    #[test]
    fn the_ext_clickable_button_gains_a_ring_on_hover() {
        let mut harness = harness();
        let ring_around = |harness: &Harness<'_, Rig<State>>| {
            let node = rect_of(harness, "/\"clickable\"");
            harness.output().shapes.iter().any(|clipped| {
                matches!(&clipped.shape, egui::epaint::Shape::Rect(rect)
                    if rect.rect == node && rect.stroke.width == 8.0
                        && rect.stroke_kind == egui::StrokeKind::Outside)
            })
        };
        assert!(!ring_around(&harness), "precondition: no ring at rest");
        hover_node(&mut harness, "/\"clickable\"");
        assert!(ring_around(&harness));
    }
}

mod navigation {
    use super::*;
    use sidebar::{ENTRY_HOVER, ENTRY_SELECTED, PILL, PILL_HOVER, Route, State};

    fn harness() -> Harness<'static, Rig<State>> {
        support::harness(
            vec2(800.0, 800.0),
            common::base_text(egui::Theme::Light),
            State::default(),
            sidebar::layout,
        )
    }

    const SELECTED: &str = "/Example(\"Flex\")";
    const OTHER: &str = "/Example(\"Grid\")";

    #[test]
    fn the_selected_entry_is_base_paint_and_hover_lights_the_others() {
        let mut harness = harness();
        assert_eq!(fill_of(&harness, SELECTED), Some(ENTRY_SELECTED));

        hover_node(&mut harness, OTHER);
        harness.run_steps(10);
        assert_eq!(fill_of(&harness, OTHER), Some(ENTRY_HOVER));

        hover_node(&mut harness, SELECTED);
        harness.run_steps(10);
        assert_eq!(fill_of(&harness, SELECTED), Some(ENTRY_SELECTED));
    }

    #[test]
    fn clicking_an_entry_or_a_pill_navigates() {
        let mut harness = harness();
        click_node(&mut harness, OTHER);
        harness.run_steps(2);
        assert_eq!(harness.state().app.route, Route::Example("Grid"));
        assert_eq!(fill_of(&harness, OTHER), Some(ENTRY_SELECTED));
        assert_ne!(fill_of(&harness, SELECTED), Some(ENTRY_SELECTED));

        let pill = "/Crate(\"dnd\")";
        assert_eq!(fill_of(&harness, pill), Some(PILL));
        hover_node(&mut harness, pill);
        assert_eq!(fill_of(&harness, pill), Some(PILL_HOVER));
        click_node(&mut harness, pill);
        harness.run_steps(2);
        assert_eq!(harness.state().app.route, Route::Crate("dnd"));
        assert_eq!(fill_of(&harness, pill), Some(ENTRY_SELECTED));
    }

    #[test]
    fn the_link_is_pinned_to_the_bottom_of_the_sidebar() {
        let harness = harness();
        let nav = rect_of(&harness, "/\"github\"");
        let last_pill = rect_of(&harness, "/Crate(\"pull_to_refresh\")");
        assert!(nav.min.y > last_pill.max.y, "below the pills");
        assert_eq!(
            nav.max.y + 8.0,
            rect_of(&harness, "/<div>").max.y,
            "pinned by the sidebar's padding"
        );
    }
}
