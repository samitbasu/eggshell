//! Overlays through layout: placement, the clamp, a detached anchor, the
//! order the slots are declared in and the place in the stack.

use eggshell_ui::overlay::{Anchor, Anchored, Band, MatchWidth, Placement, Side};
use eggshell_ui::scroll::Axis;
use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, LayoutState, Placed, Scope, TextSpec, TextStyle, tw};
use egui::{
    Area, Color32, Event, FontFamily, Id, LayerId, Modifiers, Order, PointerButton, Pos2, Rect,
    Sense, Ui, Vec2, pos2, vec2,
};
use egui_kittest::Harness;
use proptest::prelude::*;

fn body() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

mod layout {
    use super::*;

    const SCREEN: Vec2 = vec2(400.0, 300.0);

    fn headless() -> Headless {
        Headless::new(SCREEN).base_text(body())
    }

    fn trigger(b: &mut Builder<'_, '_, ()>, at: Pos2, size: Vec2) {
        b.div_id(
            "trigger",
            tw().absolute()
                .left_px(at.x)
                .top_px(at.y)
                .w_px(size.x)
                .h_px(size.y),
            |_| {},
        );
    }

    fn panel(s: &mut Scope<'_, '_, ()>, size: Vec2) {
        s.div_id("panel", tw().w_px(size.x).h_px(size.y), |_| {});
    }

    fn below(placed: &Placed<()>, key: &'static str) -> (Rect, Side, Anchored) {
        let overlay = placed.overlay(key);
        (overlay.rect(), overlay.side(), overlay.anchored())
    }

    #[test]
    fn an_overlay_sits_below_its_anchor() {
        let placed = headless().layout(&(), |b, _| {
            trigger(b, pos2(100.0, 50.0), vec2(80.0, 30.0));
            let anchor = b.id_of("trigger");
            b.overlay("pop")
                .anchor(Anchor::Node(anchor))
                .children(|s| panel(s, vec2(120.0, 60.0)));
        });
        let (rect, side, anchored) = below(&placed, "pop");
        assert_eq!(
            rect,
            Rect::from_min_size(pos2(100.0, 80.0), vec2(120.0, 60.0))
        );
        assert_eq!(side, Side::Bottom);
        assert_eq!(anchored, Anchored::Attached);
        assert_eq!(placed.overlay("pop").stack_index(), 0);
        assert_eq!(placed.at("pop").at("panel").rect(), rect);
    }

    #[test]
    fn the_page_is_laid_out_as_if_the_overlay_were_not_there() {
        let page = |b: &mut Builder<'_, '_, ()>| trigger(b, pos2(100.0, 50.0), vec2(80.0, 30.0));
        let without = headless().layout(&(), |b, _| page(b));
        let with = headless().layout(&(), |b, _| {
            page(b);
            let anchor = b.id_of("trigger");
            b.overlay("pop")
                .anchor(Anchor::Node(anchor))
                .children(|s| panel(s, vec2(120.0, 60.0)));
        });
        assert_eq!(
            with.len(),
            without.len() + 2,
            "the overlay's root and panel"
        );
        assert_eq!(with.at("trigger").rect(), without.at("trigger").rect());
        assert_eq!(
            with.nodes().next().map(|node| node.rect()),
            without.nodes().next().map(|node| node.rect())
        );
    }

    #[test]
    fn it_flips_to_the_side_with_room() {
        let placed = headless().layout(&(), |b, _| {
            trigger(b, pos2(100.0, 250.0), vec2(80.0, 30.0));
            let anchor = b.id_of("trigger");
            b.overlay("pop")
                .anchor(Anchor::Node(anchor))
                .children(|s| panel(s, vec2(120.0, 60.0)));
        });
        let (rect, side, _) = below(&placed, "pop");
        let trigger = placed.at("trigger").rect();
        assert!(
            trigger.max.y + 60.0 > SCREEN.y,
            "precondition: below the anchor does not fit"
        );
        assert_eq!(side, Side::Top);
        assert_eq!(rect.max.y, 250.0);
    }

    #[test]
    fn match_width_makes_the_overlay_at_least_the_anchor_wide() {
        let widths: Vec<f32> = [MatchWidth::Content, MatchWidth::Anchor]
            .into_iter()
            .map(|match_width| {
                let placed = headless().layout(&(), |b, _| {
                    trigger(b, pos2(20.0, 20.0), vec2(200.0, 30.0));
                    let anchor = b.id_of("trigger");
                    b.overlay("pop")
                        .anchor(Anchor::Node(anchor))
                        .match_width(match_width)
                        .children(|s| panel(s, vec2(100.0, 60.0)));
                });
                placed.overlay("pop").rect().width()
            })
            .collect();
        assert_eq!(widths, [100.0, 200.0]);
    }

    fn tall_list(s: &mut Scope<'_, '_, ()>) {
        s.scroll("list", tw().w_px(150.0).h_px(400.0).min_h_0())
            .axis(Axis::Y)
            .children(|s| {
                for row in 0..30_u32 {
                    s.span(row, tw().h_px(20.0), format!("row {row}"));
                }
            });
    }

    #[test]
    fn a_tall_list_is_clamped_to_the_room_and_scrolls_inside_it() {
        let roomy = Headless::new(vec2(400.0, 800.0))
            .base_text(body())
            .layout(&(), |b, _| {
                trigger(b, pos2(100.0, 200.0), vec2(80.0, 30.0));
                let anchor = b.id_of("trigger");
                b.overlay("tall")
                    .anchor(Anchor::Node(anchor))
                    .children(tall_list);
            });
        assert_eq!(
            roomy.overlay("tall").rect().height(),
            400.0,
            "precondition: the list wants 400 points"
        );

        let placed = headless().layout(&(), |b, _| {
            trigger(b, pos2(100.0, 200.0), vec2(80.0, 30.0));
            let anchor = b.id_of("trigger");
            b.overlay("tall")
                .anchor(Anchor::Node(anchor))
                .children(tall_list);
        });
        let tall = placed.overlay("tall");
        assert_eq!(tall.side(), Side::Top, "above has the room: 200 against 70");
        assert_eq!(tall.rect().max.y, 200.0);
        assert_eq!(tall.rect().height(), 200.0, "clamped to the room above");
        assert_eq!(placed.at("tall").at("list").rect().height(), 200.0);
        let content = placed
            .nodes()
            .find(|node| node.path() == "/\"tall\"/\"list\"/<scrollcontent>")
            .expect("the list's content");
        assert_eq!(
            content.rect().height(),
            600.0,
            "the content was solved after the clamp, at its own size"
        );
    }

    /// A pane that clips its content, with a trigger `at` points down it.
    fn pane_with_trigger(b: &mut Builder<'_, '_, ()>, at: f32) -> Id {
        let mut trigger = None;
        b.div_id("pane", tw().w_px(100.0).h_px(50.0).overflow_hidden(), |s| {
            s.div(tw().h_px(at), |_| {});
            trigger = Some(s.id_of("trigger"));
            s.div_id("trigger", tw().w_px(50.0).h_px(20.0), |_| {});
        });
        trigger.expect("the pane's body ran")
    }

    fn anchored_to_pane_trigger(at: f32) -> Placed<()> {
        headless().layout(&(), |b, _| {
            let trigger = pane_with_trigger(b, at);
            b.overlay("pop")
                .anchor(Anchor::Node(trigger))
                .children(|s| panel(s, vec2(60.0, 40.0)));
        })
    }

    #[test]
    fn an_anchor_clipped_away_detaches_the_overlay_and_keeps_it_open() {
        let hidden = anchored_to_pane_trigger(80.0);
        let pane = hidden.at("pane").rect();
        let trigger = hidden.at("pane").at("trigger").rect();
        assert!(
            trigger.min.y > pane.max.y,
            "precondition: the trigger lies below the pane that clips it"
        );
        let overlay = hidden.overlay("pop");
        assert_eq!(overlay.anchored(), Anchored::Detached);
        assert_eq!(hidden.overlays().len(), 1, "it stays open");
        assert_eq!(overlay.stack_index(), 0);

        let visible = anchored_to_pane_trigger(10.0);
        assert_eq!(visible.overlay("pop").anchored(), Anchored::Attached);
        let straddling = anchored_to_pane_trigger(40.0);
        let (pane, trigger) = (
            straddling.at("pane").rect(),
            straddling.at("pane").at("trigger").rect(),
        );
        assert!(
            trigger.min.y < pane.max.y && trigger.max.y > pane.max.y,
            "precondition: the trigger is half in the pane"
        );
        assert_eq!(
            straddling.overlay("pop").anchored(),
            Anchored::Attached,
            "a partly visible anchor keeps the overlay"
        );
    }

    #[test]
    fn slots_are_in_declaration_order_and_a_nested_overlay_follows_its_parent() {
        let placed = headless().layout(&(), |b, _| {
            for key in ["first", "second", "third"] {
                b.overlay(key)
                    .anchor(Anchor::Point(pos2(20.0, 20.0)))
                    .children(|s| {
                        s.div_id("item", tw().w_px(100.0).h_px(30.0), |_| {});
                        if key == "second" {
                            let item = s.id_of("item");
                            s.overlay("sub")
                                .anchor(Anchor::Node(item))
                                .placement(Placement::submenu())
                                .children(|s| panel(s, vec2(50.0, 40.0)));
                        }
                    });
            }
        });
        let order: Vec<&str> = placed
            .overlays()
            .iter()
            .map(|overlay| overlay.path())
            .collect();
        assert_eq!(
            order,
            [
                "/\"first\"",
                "/\"second\"",
                "/\"second\"/\"sub\"",
                "/\"third\""
            ]
        );
        let indices: Vec<usize> = placed.overlays().iter().map(|o| o.stack_index()).collect();
        assert_eq!(indices, [0, 1, 2, 3]);
        let second = placed.overlay("second");
        let sub = placed
            .overlay_with(
                placed
                    .at("second")
                    .at("sub")
                    .id()
                    .expect("an overlay is identified"),
            )
            .expect("the nested overlay is open");
        assert_eq!(sub.nested_in(), Some(second.id()));
        assert_eq!(second.nested_in(), None);
        let item = placed.at("second").at("item").rect();
        assert_eq!(
            sub.rect().min.x,
            item.max.x - 4.0,
            "placed against the parent's final rect, with the submenu's overlap"
        );
    }

    #[test]
    fn a_nested_overlay_is_hidden_with_its_parent() {
        let placed = anchored_with_child();
        assert_eq!(placed.overlay("pop").anchored(), Anchored::Detached);
        let child = placed
            .overlay_with(placed.at("pop").at("child").id().expect("identified"))
            .expect("open");
        assert_eq!(child.anchored(), Anchored::Detached);
    }

    fn anchored_with_child() -> Placed<()> {
        headless().layout(&(), |b, _| {
            let trigger = pane_with_trigger(b, 80.0);
            b.overlay("pop")
                .anchor(Anchor::Node(trigger))
                .children(|s| {
                    panel(s, vec2(60.0, 40.0));
                    s.overlay("child")
                        .anchor(Anchor::Point(pos2(200.0, 100.0)))
                        .children(|s| panel(s, vec2(20.0, 20.0)));
                });
        })
    }

    #[test]
    fn closing_and_reopening_changes_the_place_in_the_stack() {
        let mut headless = headless();
        let layout = |headless: &mut Headless, open: &[&'static str]| {
            let open = open.to_vec();
            headless.layout(&(), move |b, _| {
                for key in open {
                    b.overlay(key)
                        .anchor(Anchor::Point(pos2(20.0, 20.0)))
                        .children(|s| panel(s, vec2(20.0, 20.0)));
                }
            })
        };
        let stack = |placed: &Placed<()>| -> Vec<(String, usize)> {
            placed
                .overlays()
                .iter()
                .map(|o| (o.path().to_owned(), o.stack_index()))
                .collect()
        };
        let first = layout(&mut headless, &["a", "b"]);
        assert_eq!(
            stack(&first),
            [("/\"a\"".to_owned(), 0), ("/\"b\"".to_owned(), 1)]
        );
        let second = layout(&mut headless, &["b"]);
        assert_eq!(stack(&second), [("/\"b\"".to_owned(), 0)]);
        let third = layout(&mut headless, &["a", "b"]);
        assert_eq!(
            stack(&third),
            [("/\"b\"".to_owned(), 0), ("/\"a\"".to_owned(), 1)],
            "reopened, a is above b whatever the declaration order"
        );
    }

    #[test]
    fn the_anchors_resolve_to_rects() {
        let placed = headless().layout(&(), |b, _| {
            b.overlay("point")
                .anchor(Anchor::Point(pos2(50.0, 40.0)))
                .placement(Placement::menu())
                .children(|s| panel(s, vec2(30.0, 30.0)));
            b.overlay("rect")
                .anchor(Anchor::Rect(Rect::from_min_size(
                    pos2(200.0, 100.0),
                    vec2(40.0, 20.0),
                )))
                .placement(Placement::beside(Side::Right))
                .children(|s| panel(s, vec2(30.0, 30.0)));
            b.overlay("edge")
                .anchor(Anchor::Screen)
                .placement(Placement::edge(Side::Right))
                .children(|s| panel(s, vec2(60.0, 80.0)));
        });
        assert_eq!(placed.overlay("point").rect().min, pos2(50.0, 40.0));
        assert_eq!(placed.overlay("rect").rect().min, pos2(240.0, 100.0));
        assert_eq!(placed.overlay("edge").rect().max.x, SCREEN.x);
    }

    #[test]
    fn an_anchor_that_is_not_in_the_frame_hides_the_overlay() {
        let placed = headless().layout(&(), |b, _| {
            b.overlay("pop")
                .anchor(Anchor::Node(Id::NULL.with("nobody")))
                .children(|s| panel(s, vec2(20.0, 20.0)));
        });
        assert_eq!(placed.overlay("pop").anchored(), Anchored::Detached);
    }
}
