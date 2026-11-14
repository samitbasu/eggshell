//! Regions: subtrees built after the layout around them, at the size layout
//! gave them, in the same frame; and choosing what fits with
//! `max_content_width`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use eggshell_ui::overlay::Anchor;
use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, LayoutState, Placed, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Pos2, Rect, Vec2, pos2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Companion, Rig, rect_of};

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

fn headless(size: Vec2) -> Headless {
    Headless::new(size).base_text(base())
}

#[test]
fn the_closure_gets_the_content_box_of_its_leaf() {
    let seen = Cell::new(None);
    let placed = headless(vec2(400.0, 300.0)).layout(&seen, |b, seen| {
        b.region(
            "host",
            tw().w_px(200.0).h_px(120.0).p(2.5).border(2.0),
            move |_, size| seen.set(Some(size)),
        );
    });
    assert_eq!(
        placed.at("host").rect().size(),
        vec2(200.0, 120.0),
        "precondition: the leaf is 200 by 120 with padding and border"
    );
    assert_eq!(seen.get(), Some(vec2(176.0, 96.0)));
}

#[test]
fn the_subtree_is_laid_out_at_exactly_the_content_box() {
    let placed = headless(vec2(400.0, 300.0)).layout(&(), |b, _| {
        b.div(tw().w_px(30.0).h_px(40.0), |_| {});
        b.region(
            "host",
            tw().w_px(200.0).h_px(120.0).p(2.5).border(2.0),
            |b, _| {
                b.div_id("fill", tw().size_full(), |_| {});
            },
        );
    });
    let host = placed.at("host").rect();
    assert_eq!(
        placed.at("host").at("fill").rect(),
        Rect::from_min_size(host.min + vec2(12.0, 12.0), vec2(176.0, 96.0))
    );
}

#[test]
fn a_regions_size_does_not_depend_on_its_contents() {
    let host_in = |filled: bool| {
        headless(vec2(400.0, 300.0)).layout(&(), move |b, _| {
            b.div(tw().flex(), |b| {
                b.region("host", tw().w_px(100.0).h_px(50.0), move |b, size| {
                    assert_eq!(size, vec2(100.0, 50.0));
                    if filled {
                        b.div_id("wide", tw().w_px(1000.0).h_px(1000.0), |_| {});
                    }
                });
                b.div_id("next", tw().w_px(10.0).h_px(10.0), |_| {});
            });
        })
    };
    let empty = host_in(false);
    let overflowing = host_in(true);
    assert!(
        overflowing.at("host").at("wide").rect().width() > 900.0,
        "precondition: the contents overflow the region"
    );
    assert_eq!(overflowing.at("host").rect(), empty.at("host").rect());
    assert_eq!(overflowing.at("next").rect(), empty.at("next").rect());
}

/// A sidebar of 100 points beside a region that takes the rest of the width
/// and shows one form while it is at least 300 points wide and another
/// below.
fn wide_or_narrow(placed_width: f32) -> Placed<()> {
    headless(vec2(placed_width, 100.0)).layout(&(), |b, _| {
        b.div(tw().flex().h_px(100.0), |b| {
            b.div_id("side", tw().w_px(100.0).flex_none(), |_| {});
            b.region("host", tw().flex_1().min_w_0(), |b, size| {
                if size.x >= 300.0 {
                    b.div_id("wide", tw().size_full(), |_| {});
                } else {
                    b.div_id("narrow", tw().size_full(), |_| {});
                }
            });
        });
    })
}

#[test]
fn a_build_that_branches_on_size_crosses_its_threshold_at_the_expected_width() {
    let narrow = wide_or_narrow(399.0);
    assert_eq!(narrow.at("host").rect().width(), 299.0);
    assert!(narrow.at("host").try_at("narrow").is_some());
    assert!(narrow.at("host").try_at("wide").is_none());

    let wide = wide_or_narrow(400.0);
    assert_eq!(wide.at("host").rect().width(), 300.0);
    assert!(wide.at("host").try_at("wide").is_some());
    assert!(wide.at("host").try_at("narrow").is_none());
}

#[test]
fn nested_regions_run_outermost_first() {
    let order = RefCell::new(Vec::new());
    let placed = headless(vec2(400.0, 300.0)).layout(&order, |b, order| {
        b.region(
            "first",
            tw().w_px(200.0).h_px(100.0).p(2.5),
            move |b, size| {
                order.borrow_mut().push(("first", size));
                b.region("inner", tw().size_full(), move |b, size| {
                    order.borrow_mut().push(("inner", size));
                    b.div_id("leaf", tw().size_full(), |_| {});
                });
            },
        );
        b.region("second", tw().w_px(50.0).h_px(50.0), move |_, size| {
            order.borrow_mut().push(("second", size));
        });
    });
    assert_eq!(
        order.take(),
        [
            ("first", vec2(180.0, 80.0)),
            ("inner", vec2(180.0, 80.0)),
            ("second", vec2(50.0, 50.0)),
        ],
        "an enclosing region runs before the one inside it, and siblings in order"
    );
    let inner = placed.at("first").at("inner");
    assert_eq!(
        inner.rect(),
        Rect::from_min_size(
            placed.at("first").rect().min + vec2(10.0, 10.0),
            vec2(180.0, 80.0)
        )
    );
    assert_eq!(inner.at("leaf").rect(), inner.rect());
}

#[test]
fn a_scroll_area_inside_a_region_scrolls_the_regions_content() {
    let placed = headless(vec2(400.0, 300.0)).layout(&(), |b, _| {
        b.region("host", tw().w_px(200.0).h_px(100.0), |b, _| {
            b.scroll("list", tw().size_full()).children(|s| {
                for row in 0..10_u32 {
                    s.div_id(row, tw().h_px(30.0), |_| {});
                }
            });
        });
    });
    let list = placed.at("host").at("list").rect();
    assert_eq!(list, placed.at("host").rect());
    assert!(
        placed.at("host").at("list").at(9_u32).rect().max.y > list.max.y,
        "precondition: the rows are taller than the viewport"
    );
    assert_eq!(
        placed.at("host").at("list").at(0_u32).rect().min.y,
        list.min.y
    );
}

#[test]
fn an_overlay_in_a_region_follows_its_anchor_outside_the_region() {
    let placed = headless(vec2(400.0, 300.0)).layout(&(), |b, _| {
        b.div_id(
            "trigger",
            tw().w_px(80.0).h_px(30.0).bg(Color32::GRAY),
            |_| {},
        );
        let trigger = b.id_of("trigger");
        b.region("host", tw().w_px(200.0).h_px(100.0), move |b, _| {
            b.overlay("pop")
                .anchor(Anchor::Node(trigger))
                .children(|s| {
                    s.div_id("panel", tw().w_px(120.0).h_px(60.0), |_| {});
                });
        });
    });
    let pop = placed.at("host").at("pop");
    let overlay = placed
        .overlay_with(pop.id().expect("the overlay root is identified"))
        .expect("the region declared an overlay");
    let trigger = placed.at("trigger").rect();
    assert_eq!(
        overlay.rect(),
        Rect::from_min_size(pos2(trigger.min.x, trigger.max.y), vec2(120.0, 60.0))
    );
    assert_eq!(overlay.anchor(), trigger);
}

#[test]
fn a_region_in_an_overlay_is_laid_out_at_the_overlays_size() {
    let seen = Cell::new(None);
    let placed = headless(vec2(400.0, 300.0)).layout(&seen, |b, seen| {
        b.overlay("pop")
            .anchor(Anchor::Point(pos2(20.0, 20.0)))
            .children(|s| {
                s.div_id("box", tw().w_px(140.0).h_px(90.0), |b| {
                    b.region("host", tw().size_full(), move |_, size| {
                        seen.set(Some(size))
                    });
                });
            });
    });
    assert_eq!(seen.get(), Some(vec2(140.0, 90.0)));
    assert_eq!(
        placed.overlay("pop").rect().min,
        pos2(20.0, 20.0),
        "precondition: the overlay is where it was asked to be"
    );
}

#[test]
fn text_inheritance_continues_into_a_region() {
    let placed = headless(vec2(400.0, 300.0)).layout(&(), |b, _| {
        let role = TextSpec {
            size: 20.0,
            line_height: 24.0,
            family: FontFamily::Monospace,
        };
        b.div(tw().text(Color32::RED).role(role), |b| {
            b.region("host", tw().w_px(100.0).h_px(50.0), |b, _| {
                b.span("label", tw(), "hello");
            });
        });
    });
    let label = placed
        .node(placed.at("host").at("label").id().expect("identified"))
        .expect("the span is a node");
    assert_eq!(label.text().color, Color32::RED);
    assert_eq!(label.text().spec.size, 20.0);
    assert_eq!(label.text().spec.family, FontFamily::Monospace);
}

#[test]
fn a_region_closure_may_borrow_application_data() {
    struct App {
        title: String,
    }
    let app = App {
        title: "Quarterly".to_owned(),
    };
    let placed = headless(vec2(400.0, 300.0)).layout(&app, |b, app| {
        b.region("host", tw().w_px(200.0).h_px(50.0), move |b, _| {
            b.span("title", tw(), app.title.as_str());
        });
    });
    assert!(placed.at("host").try_at("title").is_some());
}

fn candidate<A>(b: &mut Builder<'_, '_, A>) {
    b.div_id("cand", tw().flex().gap(4.0), |b| {
        b.text(tw(), "Alpha beta gamma");
        b.text(tw(), "delta");
    });
}

#[test]
fn max_content_width_is_the_width_the_subtree_has_at_max_content() {
    let measured = Cell::new(0.0);
    let placed = headless(vec2(600.0, 200.0)).layout(&measured, |b, measured| {
        measured.set(b.max_content_width(candidate).into());
        b.div(tw().flex(), candidate);
    });
    let laid = placed.at("cand").rect().width();
    assert!(
        laid > 0.0 && laid < 600.0,
        "precondition: it fits the window"
    );
    assert_eq!(measured.get(), laid);
}

#[test]
fn max_content_width_does_not_depend_on_the_room_the_region_has() {
    let measure_in = |width: f32| {
        let measured = Cell::new(0.0);
        let placed = headless(vec2(600.0, 200.0)).layout(&measured, |b, measured| {
            b.region("host", tw().w_px(width).h_px(10.0), move |b, _| {
                measured.set(b.max_content_width(candidate).into());
                b.div(tw().flex(), candidate);
            });
        });
        (measured.get(), placed.at("host").at("cand").rect().width())
    };
    let (narrow, narrow_laid) = measure_in(10.0);
    let (wide, wide_laid) = measure_in(400.0);
    assert!(
        narrow_laid < narrow,
        "precondition: the candidate really is squeezed in the narrow region"
    );
    assert_eq!(narrow, wide);
    assert_eq!(
        wide, wide_laid,
        "and it is as measured where it has the room"
    );
}

#[test]
fn a_scratch_build_leaves_no_trace() {
    let plain = headless(vec2(600.0, 200.0)).layout(&(), |b, _| {
        b.div(tw().flex(), candidate);
    });
    let measured_first = headless(vec2(600.0, 200.0)).layout(&(), |b, _| {
        let width = f32::from(b.max_content_width(candidate));
        assert!(width > 0.0);
        b.div(tw().flex(), candidate);
    });
    assert_eq!(measured_first.len(), plain.len());
    assert_eq!(
        measured_first.at("cand").rect(),
        plain.at("cand").rect(),
        "the candidate chosen after measuring is built for real, with no duplicate-key panic"
    );
    assert!(measured_first.diff(&plain).is_empty());
}

#[test]
fn a_scratch_build_discards_its_closures() {
    let held = Rc::new(());
    headless(vec2(600.0, 200.0)).layout(&(), |b, _| {
        let width = f32::from(b.max_content_width(|b| {
            let held = held.clone();
            b.span("probe", tw(), "hi")
                .draw(move |(): &mut (), _| drop(held.clone()));
        }));
        assert!(width > 0.0);
        assert_eq!(Rc::strong_count(&held), 1, "the scratch build dropped it");
    });
}

#[test]
fn a_scratch_build_ignores_what_it_declares_besides_nodes() {
    let placed = headless(vec2(600.0, 200.0)).layout(&(), |b, _| {
        b.max_content_width(|b| {
            b.span("anchor", tw(), "hi");
            b.overlay("pop")
                .anchor(Anchor::Point(Pos2::ZERO))
                .children(|s| {
                    s.span("inside", tw(), "x");
                });
            b.region("host", tw().w_px(5.0).h_px(5.0), |b, _| {
                b.span("never", tw(), "x");
            });
        });
    });
    assert!(placed.try_overlay(&"pop").is_none());
    assert!(placed.overlays().is_empty());
    assert!(placed.try_at("anchor").is_none());
}

#[derive(Default)]
struct Canvas {
    panel_open: bool,
}

const MARKER: Vec2 = vec2(50.0, 30.0);
const PANEL: f32 = 200.0;

/// A canvas the application places things on from its size: a region over a
/// relative, clipping area, with a marker at its bottom right corner.
fn canvas_view(b: &mut Builder<'_, '_, Canvas>, app: &Canvas) {
    b.div(tw().flex().w_full().h_full(), |b| {
        if app.panel_open {
            b.div_id("panel", tw().w_px(PANEL).flex_none(), |_| {});
        }
        b.region(
            "canvas",
            tw().relative().flex_1().min_w_0().overflow_hidden(),
            |b, size| {
                b.div_id(
                    "marker",
                    tw().absolute()
                        .left_px(size.x - MARKER.x)
                        .top_px(size.y - MARKER.y)
                        .w_px(MARKER.x)
                        .h_px(MARKER.y),
                    |_| {},
                );
            },
        );
    });
}

fn canvas_harness(companion: Companion) -> Harness<'static, Rig<Canvas>> {
    let mut harness = support::harness(vec2(500.0, 300.0), base(), Canvas::default(), canvas_view);
    harness.state_mut().companion = companion;
    harness.run_steps(3);
    harness
}

fn marker_corner(harness: &Harness<'_, Rig<Canvas>>) -> Pos2 {
    rect_of(harness, "/\"canvas\"/\"marker\"").max
}

fn canvas_rect(harness: &Harness<'_, Rig<Canvas>>) -> Rect {
    rect_of(harness, "/\"canvas\"")
}

#[test]
fn the_first_frame_after_a_resize_places_children_against_the_new_size() {
    let mut harness = canvas_harness(Companion::Alone);
    let before = canvas_rect(&harness);
    assert_eq!(marker_corner(&harness), before.max);

    harness.set_size(vec2(640.0, 360.0));
    harness.step();

    let canvas = canvas_rect(&harness);
    assert_eq!(
        canvas.size(),
        before.size() + vec2(140.0, 60.0),
        "precondition: the canvas follows the window"
    );
    assert_eq!(marker_corner(&harness), canvas.max);
    assert!(
        !harness.ctx.requested_repaint_last_pass(),
        "no second frame is needed"
    );
}

#[test]
fn the_first_frame_after_a_sibling_opens_places_children_against_the_new_size() {
    let mut harness = canvas_harness(Companion::Alone);
    let before = canvas_rect(&harness);

    harness.state_mut().app.panel_open = true;
    harness.step();

    let canvas = canvas_rect(&harness);
    assert_eq!(
        canvas.width(),
        before.width() - PANEL,
        "precondition: the panel took its width from the canvas"
    );
    assert_eq!(marker_corner(&harness), canvas.max);
    assert!(
        !harness.ctx.requested_repaint_last_pass(),
        "no second frame is needed"
    );
}

#[test]
fn regions_lay_out_beside_a_second_layout_state_in_the_context() {
    assert_eq!(
        Companion::default(),
        Companion::Bar,
        "precondition: the rig lays out a second LayoutState by default"
    );
    let mut harness = canvas_harness(Companion::default());
    let before = canvas_rect(&harness);
    harness.state_mut().app.panel_open = true;
    harness.set_size(vec2(600.0, 320.0));
    harness.step();
    let canvas = canvas_rect(&harness);
    assert_eq!(canvas.width(), before.width() + 100.0 - PANEL);
    assert_eq!(marker_corner(&harness), canvas.max);
}

#[test]
fn two_layout_states_keep_their_regions_apart() {
    struct Pair {
        left: LayoutState,
        right: LayoutState,
        seen: Vec<(Vec2, Vec2)>,
    }
    let pair = Pair {
        left: LayoutState::named("left").with_base_text(base()),
        right: LayoutState::named("right").with_base_text(base()),
        seen: Vec::new(),
    };
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, 200.0))
        .build_ui_state(
            |ui, pair: &mut Pair| {
                let window = ui.max_rect();
                let halves = [
                    Rect::from_min_max(window.min, pos2(window.center().x, window.max.y)),
                    Rect::from_min_max(pos2(window.center().x, window.min.y), window.max),
                ];
                let Pair { left, right, seen } = pair;
                seen.clear();
                for (state, area) in [(left, halves[0]), (right, halves[1])] {
                    let sink = RefCell::new(Vec::new());
                    let placed = state
                        .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                            b.region("r", tw().size_full(), |b, size| {
                                sink.borrow_mut().push(size);
                                b.div_id("x", tw().size_full(), |_| {});
                            });
                        })
                        .layout(area);
                    assert_eq!(placed.at("r").at("x").rect(), area);
                    seen.extend(sink.take().into_iter().map(|size| (size, area.size())));
                }
            },
            pair,
        );
    harness.run_steps(2);
    let seen = &harness.state().seen;
    assert_eq!(seen.len(), 2);
    for &(size, area) in seen {
        assert_eq!(size, area, "each state's region got its own state's room");
    }
    assert!(seen[0].1.x > 0.0, "precondition: the areas are not empty");
}
