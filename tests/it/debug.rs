//! The layout debug overlay through the real frame path, asserted on the
//! shapes egui is handed, so no GPU is needed.

use eggshell_ui::{Builder, Debug, TextSpec, TextStyle, tw};
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
        },
        Color32::BLACK,
    )
}

fn rig(view: support::View<()>) -> Harness<'static, Rig<()>> {
    support::harness(SIZE, base(), (), view)
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
