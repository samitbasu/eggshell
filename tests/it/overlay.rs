//! Overlays through layout, draw and the pointer: placement, the clamp, a
//! detached anchor, the order the slots are declared in, stacking, the
//! blocker, and what a painted node does to the pointer (#102).

use eggshell_ui::overlay::{Anchor, Anchored, Band, Boundary, Flip, MatchWidth, Placement, Side};
use eggshell_ui::scroll::Axis;
use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, LayoutState, Placed, Scope, TextSpec, TextStyle, tw};
use eggshell_ui::{Distance, Sides};
use egui::{
    Area, Color32, Event, FontFamily, Id, LayerId, Modifiers, Order, PointerButton, Pos2, Rect,
    Sense, Ui, Vec2, pos2, vec2,
};
use egui_kittest::Harness;
use proptest::prelude::*;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/menu.rs"]
mod menu;

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
        (
            overlay.rect(),
            overlay.side().expect("a floating overlay"),
            overlay.anchored(),
        )
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
        assert_eq!(
            tall.side(),
            Some(Side::Top),
            "above has the room: 200 against 70"
        );
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

    #[cfg(debug_assertions)]
    #[test]
    fn an_anchor_that_is_not_in_the_frame_when_the_overlay_opens_panics_at_its_declaration() {
        let line = line!() + 3;
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            headless().layout(&(), |b, _| {
                b.overlay("pop")
                    .anchor(Anchor::Node(Id::NULL.with("nobody")))
                    .children(|s| panel(s, vec2(20.0, 20.0)));
            });
        }));
        let payload = caught.expect_err("a missing anchor is a programming error");
        let message = payload
            .downcast_ref::<String>()
            .expect("a formatted message");
        assert!(
            message.contains("anchored to a node that doesn't exist")
                && message.contains(r#"/"pop""#),
            "{message}"
        );
        assert!(
            message.contains(&format!("{}:{line}:", file!())),
            "names the declaration site {}:{line}: {message}",
            file!()
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_boundary_that_is_not_in_the_frame_when_the_overlay_opens_panics_at_its_declaration() {
        let line = line!() + 3;
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            headless().layout(&(), |b, _| {
                b.overlay("pop")
                    .anchor(Anchor::Point(pos2(20.0, 20.0)))
                    .placement(Placement::below().boundary(Boundary::Node(Id::NULL.with("nobody"))))
                    .children(|s| panel(s, vec2(20.0, 20.0)));
            });
        }));
        let payload = caught.expect_err("a missing boundary is a programming error");
        let message = payload
            .downcast_ref::<String>()
            .expect("a formatted message");
        assert!(
            message.contains("bounded by a node that doesn't exist")
                && message.contains(r#"/"pop""#),
            "{message}"
        );
        assert!(
            message.contains(&format!("{}:{line}:", file!())),
            "names the declaration site {}:{line}: {message}",
            file!()
        );
    }
}

/// What the kittest harness records of each overlay in the frame it draws.
#[derive(Clone, Debug)]
struct Seen {
    path: String,
    layer: LayerId,
    stack: usize,
    rect: Rect,
    anchored: Anchored,
}

struct World<A> {
    app: A,
    layout: LayoutState,
    view: fn(&mut Builder<'_, '_, A>, &A),
    seen: Vec<Seen>,
}

/// Builds, lays out and draws `view` every frame: the real path.
fn world<A: 'static>(app: A, view: fn(&mut Builder<'_, '_, A>, &A)) -> Harness<'static, World<A>> {
    let world = World {
        app,
        layout: LayoutState::new().with_base_text(body()),
        view,
        seen: Vec::new(),
    };
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_accessibility_check(true)
        .build_ui_state(
            |ui, world: &mut World<A>| {
                let World {
                    app,
                    layout,
                    view,
                    seen,
                } = world;
                let area = ui.max_rect();
                let placed = layout.build(ui.ctx(), |b| view(b, &*app)).layout(area);
                *seen = placed
                    .overlays()
                    .iter()
                    .map(|overlay| Seen {
                        path: overlay.path().to_owned(),
                        layer: overlay.layer(),
                        stack: overlay.stack_index(),
                        rect: overlay.rect(),
                        anchored: overlay.anchored(),
                    })
                    .collect();
                placed.draw(app, ui);
            },
            world,
        );
    harness.run_steps(3);
    harness
}

fn press_and_release<A>(harness: &mut Harness<'_, World<A>>, at: Pos2) {
    harness.hover_at(at);
    harness.run_steps(3);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run_steps(2);
}

fn drag<A>(harness: &mut Harness<'_, World<A>>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.run_steps(3);
    harness.drag_at(from);
    harness.step();
    harness.hover_at(from.lerp(to, 0.5));
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.run_steps(3);
}

mod stacking {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Pane {
        A,
        B,
        C,
    }

    impl Pane {
        /// The panes are 120 by 80 and each overlaps the one before.
        fn at(self) -> Pos2 {
            match self {
                Self::A => pos2(60.0, 60.0),
                Self::B => pos2(100.0, 100.0),
                Self::C => pos2(140.0, 140.0),
            }
        }
    }

    #[derive(Default)]
    struct App {
        open: Vec<Pane>,
        /// B is declared in A's body.
        nested: bool,
        clicked: Vec<String>,
        band: Option<Band>,
        bare: bool,
    }

    fn pane(b: &mut Builder<'_, '_, App>, app: &App, pane: Pane) {
        b.overlay(pane)
            .anchor(Anchor::Point(pane.at()))
            .band(app.band.unwrap_or_default())
            .children(|s| {
                let classes = tw().w_px(120.0).h_px(80.0);
                if app.bare {
                    s.div_id("box", classes, |_| {});
                } else {
                    s.div_id("box", classes.bg(Color32::LIGHT_BLUE).sense_click(), |_| {})
                        .on_click(move |app: &mut App| app.clicked.push(format!("{pane:?}")));
                }
                if app.nested && pane == Pane::A {
                    self::pane(s, app, Pane::B);
                }
            });
    }

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id("page", tw().w_full().h_full().sense_click(), |_| {})
            .on_click(|app: &mut App| app.clicked.push("page".to_owned()));
        for &which in &app.open {
            pane(b, app, which);
        }
    }

    const OVERLAP_AB: Pos2 = pos2(150.0, 120.0);
    const ONLY_A: Pos2 = pos2(70.0, 70.0);

    fn layer(harness: &Harness<'_, World<App>>, pane: Pane) -> LayerId {
        let path = format!("/{pane:?}");
        harness
            .state()
            .seen
            .iter()
            .find(|seen| seen.path.ends_with(&path))
            .unwrap_or_else(|| panic!("{path} is not open: {:?}", harness.state().seen))
            .layer
    }

    fn top_at(harness: &Harness<'_, World<App>>, at: Pos2) -> Option<LayerId> {
        harness.ctx.layer_id_at(at)
    }

    fn open(panes: &[Pane]) -> Harness<'static, World<App>> {
        let mut harness = world(App::default(), view);
        harness.state_mut().app.open = panes.to_vec();
        harness.run_steps(4);
        harness
    }

    /// The open overlays by their order in egui, bottom first.
    fn in_egui_order(harness: &Harness<'_, World<App>>) -> Vec<String> {
        let seen = &harness.state().seen;
        harness
            .ctx
            .memory(|memory| memory.layer_ids().collect::<Vec<_>>())
            .into_iter()
            .filter_map(|layer| Some(seen.iter().find(|s| s.layer == layer)?.path.clone()))
            .collect()
    }

    /// The open overlays by the library's stack, bottom first.
    fn in_stack_order(harness: &Harness<'_, World<App>>) -> Vec<String> {
        let mut seen = harness.state().seen.clone();
        seen.sort_by_key(|s| s.stack);
        seen.into_iter().map(|s| s.path).collect()
    }

    #[test]
    fn an_overlay_is_a_layer_of_its_own_exactly_its_box() {
        let harness = open(&[Pane::A]);
        let seen = harness.state().seen[0].clone();
        assert_eq!(
            seen.rect,
            Rect::from_min_size(pos2(60.0, 60.0), vec2(120.0, 80.0))
        );
        assert_eq!(top_at(&harness, seen.rect.center()), Some(seen.layer));
        assert_ne!(
            top_at(&harness, pos2(300.0, 250.0)),
            Some(seen.layer),
            "off the box the page is reached"
        );
    }

    #[test]
    fn scattered_overlays_leave_the_page_between_them_to_the_page() {
        let mut harness = open(&[Pane::A, Pane::C]);
        harness.run_steps(2);
        let layers = [layer(&harness, Pane::A), layer(&harness, Pane::C)];
        let between = pos2(280.0, 80.0);
        assert!(
            layers.iter().all(|l| harness
                .state()
                .seen
                .iter()
                .all(|s| s.layer != *l || !s.rect.contains(between))),
            "precondition: the point is outside both boxes"
        );
        assert!(
            top_at(&harness, between).is_none_or(|layer| !layers.contains(&layer)),
            "one shared layer would cover the page between the two"
        );
    }

    #[test]
    fn a_bare_overlay_still_blocks_the_page_under_it() {
        let mut harness = world(App::default(), view);
        harness.state_mut().app.bare = true;
        harness.run_steps(3);
        press_and_release(&mut harness, pos2(100.0, 100.0));
        assert_eq!(
            harness.state().app.clicked,
            ["page"],
            "precondition: with no overlay the page takes the click"
        );
        harness.state_mut().app.clicked.clear();
        harness.state_mut().app.open = vec![Pane::A];
        harness.run_steps(3);
        press_and_release(&mut harness, pos2(100.0, 100.0));
        assert!(
            harness.state().app.clicked.is_empty(),
            "an overlay with nothing painted or sensing in it takes the pointer: {:?}",
            harness.state().app.clicked
        );
        press_and_release(&mut harness, pos2(300.0, 250.0));
        assert_eq!(
            harness.state().app.clicked,
            ["page"],
            "off the box the page is live"
        );
    }

    #[test]
    fn a_node_inside_an_overlay_takes_its_click() {
        let mut harness = open(&[Pane::A]);
        press_and_release(&mut harness, pos2(100.0, 100.0));
        assert_eq!(harness.state().app.clicked, ["A"]);
    }

    #[test]
    fn a_tooltip_overlay_lets_the_pointer_through() {
        let mut harness = world(App::default(), view);
        harness.state_mut().app.band = Some(Band::Tooltip);
        harness.state_mut().app.bare = true;
        harness.state_mut().app.open = vec![Pane::A];
        harness.run_steps(3);
        press_and_release(&mut harness, pos2(100.0, 100.0));
        assert_eq!(harness.state().app.clicked, ["page"]);
    }

    #[test]
    fn a_detached_overlay_is_not_drawn_and_takes_no_input() {
        fn detached(b: &mut Builder<'_, '_, App>, app: &App) {
            b.div_id("page", tw().w_full().h_full().sense_click(), |_| {})
                .on_click(|app: &mut App| app.clicked.push("page".to_owned()));
            let mut trigger = None;
            b.div_id("pane", tw().w_px(40.0).h_px(20.0).overflow_hidden(), |s| {
                s.div(tw().h_px(60.0), |_| {});
                trigger = Some(s.id_of("trigger"));
                s.div_id("trigger", tw().w_px(10.0).h_px(10.0), |_| {});
            });
            if !app.open.is_empty() {
                b.overlay("pop")
                    .anchor(Anchor::Node(trigger.expect("the pane's body ran")))
                    .children(|s| {
                        s.div_id(
                            "box",
                            tw().w_px(120.0).h_px(80.0).bg(Color32::RED).sense_click(),
                            |_| {},
                        )
                        .on_click(|app: &mut App| app.clicked.push("pop".to_owned()));
                    });
            }
        }
        let mut harness = world(App::default(), detached);
        harness.state_mut().app.open = vec![Pane::A];
        harness.run_steps(4);
        let seen = harness.state().seen[0].clone();
        assert_eq!(seen.anchored, Anchored::Detached, "precondition");
        assert!(
            !harness
                .ctx
                .memory(|memory| memory.layer_ids().any(|layer| layer == seen.layer)),
            "never shown: not a layer"
        );
        press_and_release(&mut harness, seen.rect.center());
        assert_eq!(harness.state().app.clicked, ["page"]);
    }

    #[test]
    fn a_new_overlay_is_above_the_ones_open() {
        let harness = open(&[Pane::A, Pane::B]);
        assert_eq!(top_at(&harness, OVERLAP_AB), Some(layer(&harness, Pane::B)));
    }

    /// F1: egui stacks layers of one order as it first saw them, whatever
    /// order they are flagged in. The library's stack is opening order, so a
    /// frame that declares them the other way round does not restack.
    #[test]
    fn declaring_the_overlays_in_another_order_does_not_restack() {
        let mut harness = open(&[Pane::A, Pane::B]);
        harness.state_mut().app.open = vec![Pane::B, Pane::A];
        harness.run_steps(4);
        assert_eq!(top_at(&harness, OVERLAP_AB), Some(layer(&harness, Pane::B)));
        assert_eq!(
            in_stack_order(&harness),
            ["/A", "/B"],
            "the stack is opening order"
        );
    }

    /// F2: a press raises the area it lands on in egui, which would put a
    /// parent over its open submenu.
    #[test]
    fn a_press_on_a_parent_leaves_its_submenu_above_it() {
        let mut harness = world(App::default(), view);
        harness.state_mut().app.open = vec![Pane::A];
        harness.state_mut().app.nested = true;
        harness.run_steps(4);
        assert_eq!(
            harness
                .state()
                .seen
                .iter()
                .map(|s| s.path.as_str())
                .collect::<Vec<_>>(),
            ["/A", "/A/B"],
            "precondition: B is nested in A"
        );
        let nested = layer(&harness, Pane::B);
        assert_eq!(top_at(&harness, OVERLAP_AB), Some(nested));
        press_and_release(&mut harness, ONLY_A);
        harness.run_steps(3);
        assert_eq!(
            top_at(&harness, OVERLAP_AB),
            Some(nested),
            "the press on A did not bring it over B"
        );
        assert_eq!(
            harness.state().app.clicked,
            ["A"],
            "precondition: the press landed on A"
        );
    }

    #[test]
    fn a_nested_overlay_is_above_its_parent() {
        let mut harness = world(App::default(), view);
        harness.state_mut().app.open = vec![Pane::A];
        harness.state_mut().app.nested = true;
        harness.run_steps(4);
        assert_eq!(top_at(&harness, OVERLAP_AB), Some(layer(&harness, Pane::B)));
    }

    #[test]
    fn an_overlay_reopened_as_a_sibling_ends_on_top() {
        let mut harness = open(&[Pane::A]);
        harness.state_mut().app.open = vec![];
        harness.run_steps(4);
        harness.state_mut().app.open = vec![Pane::B];
        harness.run_steps(4);
        harness.state_mut().app.open = vec![Pane::B, Pane::A];
        harness.run_steps(4);
        assert_eq!(
            top_at(&harness, OVERLAP_AB),
            Some(layer(&harness, Pane::A)),
            "A was opened last, though egui ranks it under B"
        );
    }

    #[test]
    fn two_overlays_reopened_in_the_reverse_of_egui_s_order_are_repaired() {
        let mut harness = open(&[Pane::A, Pane::B]);
        harness.state_mut().app.open = vec![];
        harness.run_steps(4);
        harness.state_mut().app.open = vec![Pane::B, Pane::A];
        harness.run_steps(4);
        assert_eq!(in_stack_order(&harness), ["/B", "/A"]);
        assert_eq!(
            in_egui_order(&harness),
            ["/B", "/A"],
            "egui ranked them A, B from their first life"
        );
        assert_eq!(top_at(&harness, OVERLAP_AB), Some(layer(&harness, Pane::A)));
    }

    /// The order a band's overlays are opened in, the order they close in,
    /// and the order egui first saw them in are all independent, and the
    /// layers still end up in the library's stack within as many frames as
    /// there are overlays, and stay there.
    #[derive(Clone, Debug)]
    enum Step {
        Toggle(usize),
        Reverse,
    }

    fn steps() -> impl Strategy<Value = Vec<Step>> {
        proptest::collection::vec(
            prop_oneof![(0..3_usize).prop_map(Step::Toggle), Just(Step::Reverse)],
            1..24,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]
        #[test]
        fn the_order_converges_and_stays(steps in steps()) {
            let mut harness = world(App::default(), view);
            let mut reversed = false;
            for step in steps {
                match step {
                    Step::Toggle(which) => {
                        let pane = [Pane::A, Pane::B, Pane::C][which];
                        let open = &mut harness.state_mut().app.open;
                        match open.iter().position(|&p| p == pane) {
                            Some(at) => { open.remove(at); }
                            None => open.push(pane),
                        }
                    }
                    Step::Reverse => {
                        reversed = !reversed;
                        harness.state_mut().app.open.reverse();
                    }
                }
                let _ = reversed;
                let misplaced = harness.state().app.open.len();
                let mut frames = 0;
                loop {
                    harness.step();
                    frames += 1;
                    if in_egui_order(&harness) == in_stack_order(&harness) || frames > misplaced + 1 {
                        break;
                    }
                }
                prop_assert_eq!(
                    in_egui_order(&harness),
                    in_stack_order(&harness),
                    "after {} frames", frames
                );
                harness.run_steps(2);
                prop_assert_eq!(in_egui_order(&harness), in_stack_order(&harness), "and it stays");
            }
        }
    }
}

/// What egui does by itself, which the stacking repair is built on. If a
/// release of egui changes either, these say so.
mod egui_layers {
    use super::*;

    struct Layers {
        order: Vec<&'static str>,
        flag_all: bool,
    }

    fn layer(name: &'static str) -> LayerId {
        LayerId::new(Order::Foreground, Id::NULL.with(name))
    }

    fn harness(order: Vec<&'static str>, flag_all: bool) -> Harness<'static, Layers> {
        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .build_ui_state(
                |ui: &mut Ui, layers: &mut Layers| {
                    let ctx = ui.ctx().clone();
                    for (i, name) in layers.order.iter().enumerate() {
                        let id = layer(name);
                        Area::new(id.id)
                            .order(Order::Foreground)
                            .fixed_pos(pos2(60.0 + 40.0 * (i as f32), 60.0 + 40.0 * (i as f32)))
                            .show(&ctx, |ui| {
                                ui.allocate_exact_size(vec2(100.0, 100.0), Sense::click());
                            });
                        if layers.flag_all {
                            ctx.move_to_top(id);
                        }
                    }
                },
                Layers { order, flag_all },
            );
        harness.run_steps(3);
        harness
    }

    const OVERLAP: Pos2 = pos2(120.0, 120.0);

    #[test]
    fn flagging_every_frame_in_a_new_declaration_order_does_not_restack() {
        let mut harness = harness(vec!["popup", "submenu"], true);
        assert_eq!(harness.ctx.layer_id_at(OVERLAP), Some(layer("submenu")));
        harness.state_mut().order.swap(0, 1);
        harness.run_steps(3);
        assert_eq!(
            harness.ctx.layer_id_at(OVERLAP),
            Some(layer("submenu")),
            "egui keeps the order it first saw them in"
        );
    }

    #[test]
    fn a_press_on_a_parent_raises_it_over_its_submenu_unless_all_are_flagged() {
        let raised_by_a_press = |flag_all: bool| {
            let mut harness = harness(vec!["popup", "submenu"], flag_all);
            for pressed in [true, false] {
                harness.event(Event::PointerButton {
                    pos: pos2(70.0, 70.0),
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                });
                harness.step();
            }
            harness.run_steps(2);
            harness.ctx.layer_id_at(OVERLAP)
        };
        assert_eq!(raised_by_a_press(false), Some(layer("popup")));
        assert_eq!(raised_by_a_press(true), Some(layer("submenu")));
    }
}

/// What a painted node does to the pointer (#102).
mod pointer {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Panel {
        Painted,
        Bordered,
        Unpainted,
        PassThrough,
    }

    struct App {
        panel: Panel,
        canvas: Vec<&'static str>,
        hovered: bool,
        dragged: bool,
        button_clicks: u32,
        card_clicks: u32,
    }

    impl App {
        fn new(panel: Panel) -> Self {
            Self {
                panel,
                canvas: Vec::new(),
                hovered: false,
                dragged: false,
                button_clicks: 0,
                card_clicks: 0,
            }
        }
    }

    const PANEL: Rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(200.0, 200.0));
    const INSIDE: Pos2 = pos2(180.0, 180.0);
    const BUTTON: Pos2 = pos2(120.0, 120.0);
    const OUTSIDE: Pos2 = pos2(300.0, 250.0);

    /// ```html
    /// <div class="relative">
    ///   <div id="canvas" class="size-full">…</div>
    ///   <div class="absolute left-[100px] top-[100px] size-[100px] bg-… [pointer-events-none]">
    ///     <button class="size-[40px]"/>
    ///   </div>
    /// </div>
    /// ```
    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id("canvas", tw().w_full().h_full().sense_click_drag(), |_| {})
            .on_response(|app: &mut App, response| {
                app.hovered = response.hovered();
                app.dragged |= response.dragged();
            })
            .on_click(|app: &mut App| app.canvas.push("click"));
        let classes = tw()
            .absolute()
            .left_px(PANEL.min.x)
            .top_px(PANEL.min.y)
            .w_px(PANEL.width())
            .h_px(PANEL.height());
        let classes = match app.panel {
            Panel::Painted => classes.bg(Color32::LIGHT_GRAY),
            Panel::Bordered => classes.border(2.0).border_color(Color32::DARK_GRAY),
            Panel::Unpainted => classes,
            Panel::PassThrough => classes.bg(Color32::LIGHT_GRAY).pointer_events_none(),
        };
        b.div_id("panel", classes, |s| {
            s.div_id(
                "button",
                tw().w_px(40.0).h_px(40.0).sense_click_drag(),
                |_| {},
            )
            .on_click(|app: &mut App| app.button_clicks += 1);
        });
    }

    fn page(panel: Panel) -> Harness<'static, World<App>> {
        world(App::new(panel), view)
    }

    #[test]
    fn a_painted_panel_takes_the_pointer_from_the_canvas_beneath_it() {
        for panel in [Panel::Painted, Panel::Bordered] {
            let mut harness = page(panel);
            harness.hover_at(OUTSIDE);
            harness.run_steps(3);
            assert!(
                harness.state().app.hovered,
                "precondition: the canvas is hovered off the panel"
            );
            press_and_release(&mut harness, OUTSIDE);
            assert_eq!(harness.state().app.canvas, ["click"], "precondition");
            harness.state_mut().app.canvas.clear();

            harness.hover_at(INSIDE);
            harness.run_steps(3);
            assert!(
                !harness.state().app.hovered,
                "hover goes no further than the panel"
            );
            press_and_release(&mut harness, INSIDE);
            assert!(harness.state().app.canvas.is_empty(), "nor does a click");
        }
    }

    #[test]
    fn a_drag_that_starts_on_a_button_in_the_panel_does_not_drag_the_canvas() {
        let mut harness = page(Panel::Painted);
        drag(&mut harness, BUTTON, pos2(130.0, 130.0));
        assert!(
            !harness.state().app.dragged,
            "the button took the drag, the panel the rest"
        );
        let mut through = page(Panel::PassThrough);
        drag(&mut through, INSIDE, pos2(190.0, 190.0));
        assert!(
            through.state().app.dragged,
            "control: the panel lets it through"
        );
    }

    #[test]
    fn a_panel_with_nothing_painted_leaves_the_canvas_reachable() {
        let mut harness = page(Panel::Unpainted);
        press_and_release(&mut harness, INSIDE);
        assert_eq!(harness.state().app.canvas, ["click"]);
    }

    #[test]
    fn pointer_events_none_lets_the_pointer_through_a_painted_panel() {
        let mut harness = page(Panel::PassThrough);
        harness.hover_at(INSIDE);
        harness.run_steps(3);
        assert!(harness.state().app.hovered);
        press_and_release(&mut harness, INSIDE);
        assert_eq!(harness.state().app.canvas, ["click"]);
    }

    #[test]
    fn a_node_inside_the_panel_still_takes_its_own_click() {
        let mut harness = page(Panel::Painted);
        press_and_release(&mut harness, BUTTON);
        assert_eq!(harness.state().app.button_clicks, 1);
        assert!(harness.state().app.canvas.is_empty());
    }

    /// Descendants are not beneath their ancestors: a painted badge inside a
    /// sensing card does not take the card's click.
    #[test]
    fn a_painted_descendant_of_a_sensing_node_leaves_it_the_click() {
        fn cards(b: &mut Builder<'_, '_, App>, _: &App) {
            b.div_id("card", tw().w_px(100.0).h_px(100.0).sense_click(), |s| {
                s.div(tw().w_px(50.0).h_px(50.0).bg(Color32::GOLD), |_| {});
            })
            .on_click(|app: &mut App| app.card_clicks += 1);
        }
        let mut harness = world(App::new(Panel::Unpainted), cards);
        press_and_release(&mut harness, pos2(10.0, 10.0));
        assert_eq!(harness.state().app.card_clicks, 1);
    }
}

mod pointer_anchor {
    use super::*;

    #[derive(Default)]
    struct App {
        open: bool,
    }

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        if app.open {
            b.overlay("menu")
                .anchor(Anchor::Pointer)
                .placement(Placement::menu())
                .children(|s| {
                    s.div_id("box", tw().w_px(60.0).h_px(40.0), |_| {});
                });
        }
    }

    fn opened_at(harness: &Harness<'_, World<App>>) -> Pos2 {
        harness.state().seen[0].rect.min
    }

    #[test]
    fn it_opens_where_the_pointer_is_and_stays_when_the_pointer_moves() {
        let mut harness = world(App::default(), view);
        harness.hover_at(pos2(120.0, 90.0));
        harness.run_steps(3);
        harness.state_mut().app.open = true;
        harness.run_steps(3);
        assert_eq!(opened_at(&harness), pos2(120.0, 90.0));
        harness.hover_at(pos2(250.0, 200.0));
        harness.run_steps(3);
        assert_eq!(
            opened_at(&harness),
            pos2(120.0, 90.0),
            "it does not chase the pointer"
        );

        harness.state_mut().app.open = false;
        harness.run_steps(3);
        harness.state_mut().app.open = true;
        harness.run_steps(3);
        assert_eq!(
            opened_at(&harness),
            pos2(250.0, 200.0),
            "reopened, it is where the pointer is then"
        );
    }
}

/// The `menu` example through the full path.
mod menu_example {
    use super::*;
    use crate::support::{self, Rig, click_node, rect_of};

    use super::{common, menu};
    use menu::{ROW_HEIGHT, ROWS, State};

    const SIZE: Vec2 = vec2(600.0, 400.0);

    fn menu() -> Harness<'static, Rig<State>> {
        let mut harness = support::harness(
            SIZE,
            common::base_text(egui::Theme::Light),
            State::default(),
            menu::layout,
        );
        harness.run_steps(3);
        harness
    }

    fn has(harness: &Harness<'_, Rig<State>>, path: &str) -> bool {
        harness
            .state()
            .nodes
            .iter()
            .any(|(candidate, _)| candidate == path)
    }

    #[test]
    fn the_popover_opens_under_its_button_and_the_button_closes_it() {
        let mut harness = menu();
        assert!(
            !has(&harness, "/Page/Top/(PopoverButton, Menu)"),
            "precondition: closed"
        );
        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(3);
        let button = rect_of(&harness, "/Page/Top/PopoverButton");
        let popover = rect_of(&harness, "/Page/Top/(PopoverButton, Menu)");
        assert_eq!(popover.min.x, button.min.x);
        assert_eq!(popover.min.y, button.max.y + 4.0);
        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(8);
        assert!(!has(&harness, "/Page/Top/(PopoverButton, Menu)"));
    }

    #[test]
    fn choosing_an_item_closes_the_popover_and_reports_it() {
        let mut harness = menu();
        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(3);
        click_node(
            &mut harness,
            "/Page/Top/(PopoverButton, Menu)/Box/Item(Copy)",
        );
        harness.run_steps(8);
        assert!(!has(&harness, "/Page/Top/(PopoverButton, Menu)"));
        assert_eq!(harness.state().app.chosen.as_deref(), Some("Copy"));
    }

    #[test]
    fn the_submenu_is_nested_beside_the_item_it_opens_from() {
        let mut harness = menu();
        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(3);
        assert!(
            !has(
                &harness,
                "/Page/Top/(PopoverButton, Menu)/Box/(Item(Arrange), Submenu)"
            ),
            "precondition: closed"
        );
        click_node(
            &mut harness,
            "/Page/Top/(PopoverButton, Menu)/Box/Item(Arrange)",
        );
        harness.run_steps(3);
        let item = rect_of(
            &harness,
            "/Page/Top/(PopoverButton, Menu)/Box/Item(Arrange)",
        );
        let submenu = rect_of(
            &harness,
            "/Page/Top/(PopoverButton, Menu)/Box/(Item(Arrange), Submenu)",
        );
        assert!(
            submenu.min.x > item.center().x && submenu.min.x < item.max.x,
            "beside the item, overlapping its edge: {submenu:?} against {item:?}"
        );
        click_node(
            &mut harness,
            "/Page/Top/(PopoverButton, Menu)/Box/(Item(Arrange), Submenu)/Box/Item(Back)",
        );
        harness.run_steps(8);
        assert_eq!(harness.state().app.chosen.as_deref(), Some("Back"));
        assert!(!has(&harness, "/Page/Top/(PopoverButton, Menu)"));
    }

    #[test]
    fn a_click_away_and_escape_close_the_menu() {
        let mut harness = menu();
        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(3);
        let away = pos2(SIZE.x - 20.0, SIZE.y / 2.0);
        assert!(
            !rect_of(&harness, "/Page/Top/(PopoverButton, Menu)").contains(away),
            "precondition: away from the menu"
        );
        support::press_and_release(&mut harness, away);
        harness.run_steps(8);
        assert!(!has(&harness, "/Page/Top/(PopoverButton, Menu)"));

        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(3);
        harness.key_press(egui::Key::Escape);
        harness.run_steps(8);
        assert!(!has(&harness, "/Page/Top/(PopoverButton, Menu)"));
    }

    #[test]
    fn the_menu_is_usable_from_the_keyboard() {
        let mut harness = menu();
        click_node(&mut harness, "/Page/Top/PopoverButton");
        harness.run_steps(3);
        for _ in 0..2 {
            harness.key_press(egui::Key::ArrowDown);
            harness.run_steps(3);
        }
        harness.key_press(egui::Key::Enter);
        harness.run_steps(8);
        assert!(
            harness.state().app.grid,
            "opened with the pointer, the first Down goes to Copy and the second skips the disabled Paste to Show grid"
        );
        assert!(!has(&harness, "/Page/Top/(PopoverButton, Menu)"));
    }

    #[test]
    fn the_tall_list_flips_above_its_button_clamps_to_the_room_and_scrolls() {
        let mut harness = menu();
        click_node(&mut harness, "/Page/Bottom/ListButton");
        harness.run_steps(3);
        let button = rect_of(&harness, "/Page/Bottom/ListButton");
        let list = rect_of(&harness, "/Page/Bottom/List");
        let wanted = ROWS as f32 * ROW_HEIGHT;
        assert!(
            wanted > SIZE.y,
            "precondition: the list wants more than the window"
        );
        assert!(list.max.y <= button.min.y, "above the button: {list:?}");
        assert!(
            list.height() < wanted && list.min.y >= 0.0,
            "clamped into the room: {list:?}"
        );
        assert!(
            list.width() >= button.width(),
            "at least as wide as the button"
        );
        let content = rect_of(&harness, "/Page/Bottom/List/Menu/Rows/<scrollcontent>");
        assert_eq!(
            content.height(),
            wanted,
            "all the rows are in the scroll content"
        );

        click_node(&mut harness, "/Page/Bottom/List/Menu/Rows/Row(2)");
        harness.run_steps(3);
        assert_eq!(harness.state().app.chosen.as_deref(), Some("option 2"));
        assert!(!has(&harness, "/Page/Bottom/List"));
    }
}

mod boundary {
    use super::*;

    const CANVAS: Rect = Rect::from_min_max(pos2(50.0, 40.0), pos2(350.0, 260.0));
    const PANEL: Vec2 = vec2(120.0, 60.0);

    #[derive(Default)]
    struct App {
        anchor: Option<Rect>,
        canvas: bool,
        dismissed: u32,
        canvas_id: std::cell::Cell<Option<Id>>,
    }

    fn d(points: f32) -> Distance {
        Distance::new(points)
    }

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        if app.canvas {
            b.div_id(
                "canvas",
                tw().absolute()
                    .sense_click()
                    .left_px(CANVAS.left())
                    .top_px(CANVAS.top())
                    .w_px(CANVAS.width())
                    .h_px(CANVAS.height()),
                |_| {},
            );
        }
        let Some(anchor) = app.anchor else { return };
        let canvas = b.id_of("canvas");
        app.canvas_id.set(Some(canvas));
        b.overlay("pop")
            .anchor(Anchor::Rect(anchor))
            .placement(
                Placement::below()
                    .flip(Flip::Never)
                    .boundary(Boundary::Node(canvas).inset(Sides {
                        top: d(8.0),
                        left: d(8.0),
                        right: d(8.0),
                        bottom: d(48.0),
                    })),
            )
            .on_dismiss(|app: &mut App| {
                app.dismissed += 1;
                app.anchor = None;
            })
            .children(|s| {
                s.div_id("panel", tw().w_px(PANEL.x).h_px(PANEL.y), |_| {});
            });
    }

    fn overlay_rect(harness: &Harness<'_, World<App>>) -> Rect {
        harness.state().seen[0].rect
    }

    #[test]
    fn an_overlay_anchored_inside_a_canvas_stays_inside_the_canvas_minus_the_insets() {
        let mut harness = world(
            App {
                canvas: true,
                ..App::default()
            },
            view,
        );
        harness.state_mut().app.anchor = Some(Rect::ZERO);
        harness.run_steps(3);
        let canvas = harness
            .ctx
            .read_response(harness.state().app.canvas_id.get().expect("laid out"))
            .expect("the canvas is in the frame")
            .rect;
        assert_eq!(canvas.size(), CANVAS.size(), "precondition: the canvas");
        let inside = Rect::from_min_max(canvas.min + vec2(8.0, 8.0), canvas.max - vec2(8.0, 48.0));
        let corners = [
            canvas.left_top(),
            pos2(canvas.right() - 10.0, canvas.top()),
            pos2(canvas.left(), canvas.bottom() - 10.0),
            pos2(canvas.right() - 10.0, canvas.bottom() - 10.0),
            canvas.center(),
        ];
        for at in corners {
            harness.state_mut().app.anchor = Some(Rect::from_min_size(at, vec2(10.0, 10.0)));
            harness.run_steps(3);
            let rect = overlay_rect(&harness);
            assert_eq!(rect.size(), PANEL, "its content's size, at {at:?}");
            assert!(
                inside.contains_rect(rect),
                "{at:?}: {rect:?} not in {inside:?}"
            );
        }
    }

    #[test]
    fn a_boundary_node_that_disappears_dismisses_the_overlay() {
        let anchor = Rect::from_min_size(CANVAS.center(), vec2(10.0, 10.0));
        let mut harness = world(
            App {
                canvas: true,
                anchor: Some(anchor),
                ..App::default()
            },
            view,
        );
        assert_eq!(harness.state().seen.len(), 1, "precondition: open");
        assert_eq!(harness.state().app.dismissed, 0);
        harness.state_mut().app.canvas = false;
        harness.run_steps(3);
        assert_eq!(harness.state().app.dismissed, 1);
    }
}
