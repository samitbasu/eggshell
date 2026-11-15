//! The layout debug overlay through the real frame path, asserted on the
//! shapes egui is handed, so no GPU is needed.

use eggshell_ui::{Builder, Debug, TextSpec, TextStyle, Weight, tw};
use egui::epaint::{ClippedShape, Shape};
use egui::{Color32, FontFamily, Pos2, Rect, Vec2};
use egui_kittest::Harness;

use crate::support;
use support::Rig;

const SIZE: Vec2 = Vec2::new(400.0, 300.0);

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn rig(view: support::View<()>) -> Harness<'static, Rig<()>> {
    let mut harness = support::harness(SIZE, base(), (), view);
    harness.state_mut().companion = support::Companion::Alone;
    harness
}

fn run(harness: &mut Harness<'_, Rig<()>>, mode: Debug) {
    Debug::set(&harness.ctx, mode);
    harness.run_steps(2);
}

/// Every outline of the last frame, with its colour, in paint order.
fn outlines(harness: &Harness<'_, Rig<()>>) -> Vec<(Rect, Color32)> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect)
                if rect.fill == Color32::TRANSPARENT
                    && [Debug::OUTLINE, Debug::OVERFLOW].contains(&rect.stroke.color) =>
            {
                Some((rect.rect, rect.stroke.color))
            }
            _ => None,
        })
        .collect()
}

fn texts(shape: &Shape, into: &mut Vec<String>) {
    match shape {
        Shape::Text(text) => into.push(text.galley.text().to_owned()),
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| texts(shape, into)),
        _ => {}
    }
}

fn labels(harness: &Harness<'_, Rig<()>>) -> Vec<String> {
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        texts(&clipped.shape, &mut found);
    }
    found
}

fn paths(harness: &Harness<'_, Rig<()>>) -> Vec<String> {
    harness
        .state()
        .nodes
        .iter()
        .map(|(path, _)| path.clone())
        .collect()
}

fn nested(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("outer", tw().w_px(300.0).h_px(200.0).p(10.0), |b| {
        b.div_id("inner", tw().flex().w_px(100.0).h_px(50.0), |b| {
            b.span("label", tw(), "label");
        });
        b.rule(tw().h_px(1.0));
        b.paint(tw().w_px(10.0).h_px(10.0), |_, _, _| {});
    });
    b.div_id(
        "overlay",
        tw().absolute().top_px(250.0).left_px(10.0),
        |b| {
            b.text(tw(), "floating");
        },
    );
}

fn snapped(rect: Rect) -> Rect {
    Rect::from_min_max(rect.min.round(), rect.max.round())
}

fn same(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 0.01 && (a.max - b.max).length() < 0.01
}

#[test]
fn off_emits_no_debug_shapes() {
    let mut harness = rig(nested);
    run(&mut harness, Debug::Off);
    let off: Vec<ClippedShape> = harness.output().shapes.clone();
    assert!(outlines(&harness).is_empty());

    run(&mut harness, Debug::Outlines);
    assert!(
        !outlines(&harness).is_empty(),
        "precondition: the overlay draws when on"
    );
    assert_eq!(
        harness.output().shapes[..off.len()],
        off,
        "the frame itself is unchanged by the overlay"
    );

    run(&mut harness, Debug::Off);
    assert_eq!(harness.output().shapes, off);
}

#[test]
fn outlines_one_hairline_per_node_at_its_rect() {
    let mut harness = rig(nested);
    run(&mut harness, Debug::Outlines);
    let nodes = &harness.state().nodes;
    assert!(nodes.len() >= 8, "precondition: a tree of many kinds");
    let drawn = outlines(&harness);
    assert_eq!(drawn.len(), nodes.len());
    for ((path, rect), (outline, _)) in nodes.iter().zip(&drawn) {
        assert!(
            same(snapped(*rect), *outline),
            "{path}: {rect:?} vs {outline:?}"
        );
    }
    let hairline = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.stroke.color == Debug::OUTLINE => Some(rect.stroke.width),
            _ => None,
        });
    assert_eq!(hairline, Some(1.0), "one physical pixel at 1 ppp");
}

fn overflowing(b: &mut Builder<'_, '_, ()>, _: &()) {
    let long = "a line of text much too long for the box it is in";
    b.div_id("narrow", tw().w_px(40.0), |b| {
        b.span("nowrap", tw().whitespace_nowrap(), long);
    });
    b.div_id("roomy", tw().w_px(300.0), |b| {
        b.span("fits", tw().whitespace_nowrap(), "short");
    });
    b.div_id("short box", tw().w_px(200.0), |b| {
        b.span("too tall", tw().h_px(10.0), "one line");
    });
    b.div_id("spills", tw().w_px(50.0).h_px(20.0), |b| {
        b.div_id("wide child", tw().w_px(80.0).h_px(10.0).shrink_0(), |_| {});
    });
    b.div_id("clips", tw().w_px(50.0).h_px(20.0).overflow_hidden(), |b| {
        b.div_id(
            "clipped child",
            tw().w_px(80.0).h_px(10.0).shrink_0(),
            |_| {},
        );
    });
    b.div_id("wraps", tw().w_px(60.0), |b| {
        b.span("wrapping", tw(), "several short words");
    });
}

fn colour_of(harness: &Harness<'_, Rig<()>>, key: &str) -> Color32 {
    let paths = paths(harness);
    let index = paths
        .iter()
        .position(|path| path.ends_with(&format!("\"{key}\"")))
        .unwrap_or_else(|| panic!("no node {key}: {paths:?}"));
    outlines(harness)[index].1
}

#[test]
fn overflowing_needs_no_overlay() {
    let mut headless = eggshell_ui::testing::Headless::new(SIZE).base_text(base());
    let placed = headless.layout(&(), overflowing);
    assert_eq!(
        Debug::of(headless.ctx()),
        Debug::Off,
        "precondition: the overlay is off"
    );
    let found = placed.overflowing();
    for red in ["nowrap", "too tall", "spills"] {
        assert!(
            found
                .iter()
                .any(|path| path.ends_with(&format!("\"{red}\""))),
            "{red} in {found:?}"
        );
    }
    for fine in [
        "narrow",
        "roomy",
        "fits",
        "short box",
        "wraps",
        "wrapping",
        "clips",
    ] {
        assert!(
            !found
                .iter()
                .any(|path| path.ends_with(&format!("\"{fine}\""))),
            "{fine} in {found:?}"
        );
    }
}

#[test]
fn nodes_whose_content_did_not_fit_are_red() {
    let mut harness = rig(overflowing);
    run(&mut harness, Debug::Outlines);
    let node_rect = |key: &str| support::rect_of(&harness, &format!("/\"{key}\"")).size();
    assert!(
        node_rect("narrow").x <= 40.0 && node_rect("roomy").x >= 300.0,
        "precondition: the boxes are as declared"
    );

    for red in ["nowrap", "too tall", "spills"] {
        assert_eq!(colour_of(&harness, red), Debug::OVERFLOW, "{red}");
    }
    for fine in [
        "narrow",
        "roomy",
        "fits",
        "short box",
        "wide child",
        "clips",
        "clipped child",
        "wraps",
        "wrapping",
    ] {
        assert_eq!(colour_of(&harness, fine), Debug::OUTLINE, "{fine}");
    }
}

#[test]
fn hovering_labels_the_innermost_node() {
    let mut harness = rig(nested);
    run(&mut harness, Debug::Outlines);
    let (inner, label) = ("/\"outer\"/\"inner\"", "/\"outer\"/\"inner\"/\"label\"");
    assert!(
        !labels(&harness).iter().any(|text| text.contains(inner)),
        "precondition: no label without a pointer"
    );

    let label_rect = support::rect_of(&harness, label);
    let inner_rect = support::rect_of(&harness, inner);
    assert!(
        inner_rect.contains(label_rect.center()) && inner_rect.max.x > label_rect.max.x + 4.0,
        "precondition: the label is inside the inner box, which has room beside it"
    );

    support::hover_over(&mut harness, label_rect.center());
    let shown = labels(&harness);
    assert!(
        shown.iter().any(|text| text.starts_with(label)),
        "{shown:?}"
    );

    let beside: Pos2 = inner_rect.max - Vec2::splat(2.0);
    support::hover_over(&mut harness, beside);
    let shown = labels(&harness);
    assert!(
        shown
            .iter()
            .any(|text| text.starts_with(inner) && !text.starts_with(label)),
        "{shown:?}"
    );
    assert!(
        shown.iter().any(|text| text.contains("100.0 x 50.0")),
        "{shown:?}"
    );

    run(&mut harness, Debug::Off);
    assert!(!labels(&harness).iter().any(|text| text.contains(inner)));
}

#[test]
fn toggling_flips_the_mode_in_the_context() {
    let ctx = egui::Context::default();
    assert_eq!(Debug::of(&ctx), Debug::Off);
    Debug::toggle(&ctx);
    assert_eq!(Debug::of(&ctx), Debug::Outlines);
    Debug::toggle(&ctx);
    assert_eq!(Debug::of(&ctx), Debug::Off);
}

fn press(key: egui::Key) -> egui::RawInput {
    egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..egui::RawInput::default()
    }
}

#[test]
fn the_toggle_key_flips_the_mode_and_other_keys_do_not() {
    let ctx = egui::Context::default();
    let pass = |input| {
        ctx.run_ui(input, |ui| Debug::toggle_on(ui.ctx(), egui::Key::F12))
            .drop_without_applying_deltas();
        Debug::of(&ctx)
    };
    assert_eq!(pass(press(egui::Key::F11)), Debug::Off);
    assert_eq!(pass(press(egui::Key::F12)), Debug::Outlines);
    assert_eq!(pass(egui::RawInput::default()), Debug::Outlines);
    assert_eq!(pass(press(egui::Key::F12)), Debug::Off);
}

fn readouts(harness: &Harness<'_, Rig<()>>) -> Vec<String> {
    labels(harness)
        .into_iter()
        .filter(|text| text.contains("fps"))
        .collect()
}

#[test]
fn the_readout_is_drawn_only_while_the_overlay_is_on() {
    let mut harness = rig(nested);
    run(&mut harness, Debug::Off);
    assert!(readouts(&harness).is_empty());

    run(&mut harness, Debug::Outlines);
    let shown = readouts(&harness);
    assert_eq!(shown.len(), 1, "one readout per frame: {shown:?}");
    assert!(shown[0].contains("ms"), "{}", shown[0]);

    run(&mut harness, Debug::Off);
    assert!(readouts(&harness).is_empty());
}

#[test]
fn the_smoothed_fps_follows_the_frame_time() {
    let dt = 0.05;
    let mut harness = rig(nested);
    Debug::set(&harness.ctx, Debug::Outlines);
    harness.run_steps(40);
    let text = readouts(&harness).remove(0);
    let fps: f32 = text
        .split_whitespace()
        .rev()
        .skip_while(|word| *word != "fps")
        .nth(1)
        .and_then(|word| word.parse().ok())
        .unwrap_or_else(|| panic!("no fps in {text}"));
    assert!(
        (fps - 1.0 / dt).abs() < 1.0,
        "expected about {}, read {fps} from {text}",
        1.0 / dt
    );
}

#[test]
fn the_readout_does_not_take_part_in_layout_or_hit_testing() {
    let mut harness = rig(nested);
    run(&mut harness, Debug::Off);
    let off = paths(&harness);
    run(&mut harness, Debug::Outlines);
    assert_eq!(paths(&harness), off);
}

mod scrolling {
    use super::*;
    use eggshell_ui::scroll::Axis;

    const GUTTER: f32 = 10.0;

    fn scroller(axis: Axis) -> support::View<()> {
        match axis {
            Axis::Y => |b, _| scroll_box(b, Axis::Y),
            Axis::X => |b, _| scroll_box(b, Axis::X),
            Axis::Both => |b, _| scroll_box(b, Axis::Both),
        }
    }

    fn scroll_box(b: &mut Builder<'_, '_, ()>, axis: Axis) {
        b.scroll("pane", tw().w_px(200.0).h_px(100.0).focusable_none())
            .axis(axis)
            .children(|s| s.div(tw().w_px(400.0).h_px(300.0), |_| {}));
    }

    /// Filled shapes of the overlay's gutter tint, by rect.
    fn gutters(harness: &Harness<'_, Rig<()>>) -> Vec<Rect> {
        support::shapes(harness)
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::Rect(r) if r.fill == Debug::GUTTER => Some(r.rect),
                _ => None,
            })
            .collect()
    }

    fn viewport_outlines(harness: &Harness<'_, Rig<()>>) -> Vec<Rect> {
        support::shapes(harness)
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::Rect(r) if r.stroke.color == Debug::VIEWPORT => Some(r.rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_overlay_shows_the_viewport_and_the_gutters_of_scroll_containers() {
        for axis in [Axis::Y, Axis::X, Axis::Both] {
            let mut h = rig(scroller(axis));
            run(&mut h, Debug::Outlines);
            let pane = support::rect_of(&h, "/\"pane\"");
            let tint = gutters(&h);
            let (right, bottom) = (
                Rect::from_min_max(
                    Pos2::new(pane.max.x - GUTTER, pane.min.y),
                    Pos2::new(
                        pane.max.x,
                        pane.max.y - if axis == Axis::Both { GUTTER } else { 0.0 },
                    ),
                ),
                Rect::from_min_max(
                    Pos2::new(pane.min.x, pane.max.y - GUTTER),
                    Pos2::new(
                        pane.max.x - if axis == Axis::Both { GUTTER } else { 0.0 },
                        pane.max.y,
                    ),
                ),
            );
            let corner = Rect::from_min_max(pane.max - Vec2::splat(GUTTER), pane.max);
            let expected: Vec<Rect> = match axis {
                Axis::Y => vec![right],
                Axis::X => vec![bottom],
                Axis::Both => vec![right, bottom, corner],
            };
            assert_eq!(tint.len(), expected.len(), "{axis:?}: {tint:?}");
            for want in expected {
                assert!(
                    tint.iter().any(|got| support::same(*got, want)),
                    "{axis:?}: {want:?} in {tint:?}"
                );
            }
            let view = Rect::from_min_size(
                pane.min,
                pane.size()
                    - match axis {
                        Axis::Y => Vec2::new(GUTTER, 0.0),
                        Axis::X => Vec2::new(0.0, GUTTER),
                        Axis::Both => Vec2::splat(GUTTER),
                    },
            );
            assert!(
                viewport_outlines(&h)
                    .iter()
                    .any(|got| support::same(*got, view)),
                "{axis:?}: the viewport {view:?} is outlined"
            );
        }
    }

    #[test]
    fn without_the_overlay_nothing_of_it_is_drawn() {
        let mut h = rig(scroller(Axis::Both));
        run(&mut h, Debug::Off);
        assert!(gutters(&h).is_empty());
        assert!(viewport_outlines(&h).is_empty());
    }
}

/// Where the readout and the hover label land: kept inside the window, the
/// label above the readout, the readout away from the pointer (#232).
mod placement {
    use super::*;

    /// One node the size of the window, so the pointer is always over one.
    fn page(size: Vec2) -> Harness<'static, Rig<()>> {
        fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
            let size = b.ctx().content_rect().size();
            b.div_id("page", tw().w_px(size.x).h_px(size.y), |_| {});
        }
        let mut harness = support::harness(size, base(), (), view);
        harness.state_mut().companion = support::Companion::Alone;
        run(&mut harness, Debug::Outlines);
        harness
    }

    /// Every shape of the last frame, flattened, in paint order.
    fn flat(harness: &Harness<'_, Rig<()>>) -> Vec<Shape> {
        fn into(shape: &Shape, all: &mut Vec<Shape>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|shape| into(shape, all)),
                other => all.push(other.clone()),
            }
        }
        let mut all = Vec::new();
        for clipped in &harness.output().shapes {
            into(&clipped.shape, &mut all);
        }
        all
    }

    /// The paint position and box of the text containing `needle`, with the
    /// background drawn just before it.
    fn boxed(harness: &Harness<'_, Rig<()>>, needle: &str) -> Option<(usize, Rect)> {
        let all = flat(harness);
        let at = all.iter().position(
            |shape| matches!(shape, Shape::Text(text) if text.galley.text().contains(needle)),
        )?;
        match &all[at.checked_sub(1)?] {
            Shape::Rect(background) => Some((at, background.rect)),
            other => panic!("the text {needle:?} has no background: {other:?}"),
        }
    }

    fn readout(harness: &Harness<'_, Rig<()>>) -> (usize, Rect) {
        boxed(harness, "fps").expect("the readout is drawn")
    }

    fn screen(harness: &Harness<'_, Rig<()>>) -> Rect {
        harness.ctx.content_rect()
    }

    #[test]
    fn in_a_narrow_window_the_readout_stays_inside_it() {
        let harness = page(Vec2::new(240.0, 300.0));
        let (_, rect) = readout(&harness);
        let screen = screen(&harness);
        assert!(
            screen.width() < 300.0,
            "precondition: narrower than the readout's longest line"
        );
        assert!(screen.contains_rect(rect), "{rect:?} in {screen:?}");
    }

    #[test]
    fn near_the_top_right_the_label_is_drawn_above_the_readout() {
        let mut harness = page(SIZE);
        let page = support::rect_of(&harness, "/\"page\"");
        let at = page.right_top() + Vec2::new(-6.0, 6.0);
        assert!(
            (at - screen(&harness).right_top()).length() < 30.0,
            "precondition: the pointer is near the top right"
        );
        support::hover_over(&mut harness, at);
        let (readout_at, _) = readout(&harness);
        let (label_at, label) = boxed(&harness, "\"page\"").expect("the page is labelled");
        assert!(screen(&harness).contains_rect(label), "{label:?}");
        let all = flat(&harness);
        assert!(
            label_at > readout_at,
            "the label is painted after the readout"
        );
        assert!(
            all[label_at + 1..]
                .iter()
                .all(|shape| !matches!(shape, Shape::Rect(_))),
            "nothing is painted over the label"
        );
    }

    #[test]
    fn the_readout_takes_the_corner_farthest_from_the_pointer() {
        let mut harness = page(SIZE);
        let screen = screen(&harness);
        let inset = Vec2::splat(20.0);
        let quadrants = [
            (screen.left_top() + inset, screen.right_bottom()),
            (
                screen.right_top() + Vec2::new(-20.0, 20.0),
                screen.left_bottom(),
            ),
            (
                screen.left_bottom() + Vec2::new(20.0, -20.0),
                screen.right_top(),
            ),
            (screen.right_bottom() - inset, screen.left_top()),
        ];
        for (pointer, corner) in quadrants {
            support::hover_over(&mut harness, pointer);
            let (_, rect) = readout(&harness);
            let nearest = [
                rect.left_top(),
                rect.right_top(),
                rect.left_bottom(),
                rect.right_bottom(),
            ]
            .into_iter()
            .map(|point| point.distance(corner))
            .fold(f32::INFINITY, f32::min);
            assert!(
                nearest <= 8.0,
                "pointer at {pointer:?}: the readout {rect:?} is in the corner at {corner:?}"
            );
        }
    }

    #[test]
    fn without_a_pointer_the_readout_stays_where_it_was() {
        let mut harness = page(SIZE);
        let screen = screen(&harness);
        let (_, untouched) = readout(&harness);
        assert!(
            (untouched.right_top() - screen.right_top()).length() <= 8.0,
            "the readout starts in the top right: {untouched:?}"
        );
        support::hover_over(&mut harness, screen.right_top() + Vec2::new(-20.0, 20.0));
        let (_, moved) = readout(&harness);
        assert!(
            (moved.left_bottom() - screen.left_bottom()).length() <= 8.0,
            "precondition: the pointer moved it to the bottom left: {moved:?}"
        );
        harness.event(egui::Event::PointerGone);
        harness.run_steps(3);
        assert!(
            harness.ctx.pointer_hover_pos().is_none(),
            "precondition: no pointer"
        );
        let (_, gone) = readout(&harness);
        assert!(support::same(gone, moved), "{gone:?} stayed at {moved:?}");
    }
}
