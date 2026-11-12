//! Keyboard focus through the real path: Tab and Enter events dispatched by
//! `egui_kittest::Harness`, egui's focus traversal, the `focus` variants and
//! the focus record that answers `focus_within`.

use eggshell_ui::{Builder, NodeUi, TextSpec, TextStyle, tw};
use egui::epaint::{ClippedShape, RectShape, Shape};
use egui::{Color32, FontFamily, Key, Rect, vec2};
use egui_kittest::Harness;

use crate::support;

use support::{Rig, hover_over, press_and_release, rect_of};

const RING: Color32 = Color32::from_rgb(10, 120, 240);
const WITHIN: Color32 = Color32::from_rgb(240, 200, 10);
const PLAIN: Color32 = Color32::from_rgb(40, 40, 40);

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::WHITE,
    )
}

#[derive(Default)]
struct App {
    /// Which marked node had focus in the last draw.
    focused: Option<&'static str>,
    clicks: Vec<&'static str>,
    text: String,
    other: String,
}

fn harness(view: support::View<App>) -> Harness<'static, Rig<App>> {
    support::harness(vec2(400.0, 200.0), base_text(), App::default(), view)
}

/// Records focus and clicks of the node it is drawn for.
fn mark(name: &'static str) -> impl FnOnce(&mut App, &mut egui::Ui) {
    move |app, ui| {
        let response = ui.node_response();
        if response.has_focus() {
            app.focused = Some(name);
        }
        if response.clicked() {
            app.clicks.push(name);
        }
    }
}

fn tab(harness: &mut Harness<'_, Rig<App>>, modifiers: egui::Modifiers) {
    harness.state_mut().app.focused = None;
    harness.key_press_modifiers(modifiers, Key::Tab);
    harness.run_steps(2);
}

fn focused(harness: &Harness<'_, Rig<App>>) -> Option<&'static str> {
    harness.state().app.focused
}

fn cell() -> eggshell_ui::Tw {
    tw().w_px(60.0)
        .h_px(30.0)
        .bg(PLAIN)
        .sense_click()
        .focus(|p| p.ring(2.0, RING))
}

/// ```html
/// <div class="flex gap-2">
///   <div class="w-[60px] h-[30px] bg-[#282828] z-10 focus-visible:ring-2"></div>   <!-- first -->
///   <div class="w-[60px] h-[30px] bg-[#282828] focus-visible:ring-2"></div>        <!-- second -->
///   <div class="w-[60px] h-[30px] bg-[#282828]" tabindex="-1"></div>               <!-- skipped -->
///   <div class="w-[60px] h-[30px] bg-[#282828] focus-visible:ring-2"></div>        <!-- last -->
/// </div>
/// ```
fn row(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().gap(2.0), |b| {
        b.div_id("first", cell().z(10), |_| {}).draw(mark("first"));
        b.div_id("second", cell(), |_| {}).draw(mark("second"));
        b.div_id("skipped", cell().focusable_none(), |_| {})
            .draw(mark("skipped"));
        b.div_id("last", cell(), |_| {}).draw(mark("last"));
    });
}

fn rings(harness: &Harness<'_, Rig<App>>) -> Vec<Rect> {
    let shapes: &[ClippedShape] = &harness.output().shapes;
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(RectShape { rect, stroke, .. }) if stroke.color == RING => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn tab_visits_nodes_in_paint_order_and_skips_the_opted_out() {
    let mut harness = harness(row);
    harness.run();
    assert!(
        rect_of(&harness, "/\"first\"").min.x < rect_of(&harness, "/\"second\"").min.x,
        "precondition: `first` is laid out before `second`"
    );

    // z paints `first` after its siblings, and egui registers in paint order.
    let mut order = Vec::new();
    support::past_bar(&mut harness);
    for _ in 0..3 {
        tab(&mut harness, egui::Modifiers::NONE);
        order.push(focused(&harness));
    }
    assert_eq!(
        order,
        [Some("second"), Some("last"), Some("first")],
        "the z-raised node is last in paint order, the opted-out one is absent"
    );
    tab(&mut harness, egui::Modifiers::NONE);
    assert_eq!(
        support::focused_path(&harness).as_deref(),
        Some("<widget>"),
        "the stop after the last is the bar's"
    );

    tab(&mut harness, egui::Modifiers::SHIFT);
    assert_eq!(focused(&harness), Some("first"));
    tab(&mut harness, egui::Modifiers::SHIFT);
    assert_eq!(focused(&harness), Some("last"));
}

#[test]
fn the_focused_node_paints_its_focus_variant() {
    let mut harness = harness(row);
    harness.run();
    assert!(rings(&harness).is_empty(), "no focus, no ring");

    support::past_bar(&mut harness);
    tab(&mut harness, egui::Modifiers::NONE);
    assert_eq!(focused(&harness), Some("second"));
    assert_eq!(rings(&harness), [rect_of(&harness, "/\"second\"")]);

    tab(&mut harness, egui::Modifiers::NONE);
    assert_eq!(rings(&harness), [rect_of(&harness, "/\"last\"")]);
}

#[test]
fn enter_and_space_click_the_focused_node() {
    let mut harness = harness(row);
    harness.run();
    support::past_bar(&mut harness);
    tab(&mut harness, egui::Modifiers::NONE);
    assert_eq!(focused(&harness), Some("second"));
    assert!(harness.state().app.clicks.is_empty(), "precondition");

    harness.key_press(Key::Enter);
    harness.run_steps(2);
    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert_eq!(harness.state().app.clicks, ["second", "second"]);
}

#[test]
fn a_pointer_click_does_not_focus_the_node() {
    let mut harness = harness(row);
    harness.run();
    let at = rect_of(&harness, "/\"second\"").center();
    hover_over(&mut harness, at);
    press_and_release(&mut harness, at);
    harness.run_steps(2);
    assert_eq!(harness.state().app.clicks, ["second"], "precondition");
    assert_eq!(focused(&harness), None);
    assert!(
        rings(&harness).is_empty(),
        "this is focus-visible, not focus"
    );
}

/// ```html
/// <div class="flex flex-col gap-2">
///   <div class="inert"><div class="w-[60px] h-[30px] focus-visible:ring-2"></div></div>
///   <div class="w-[60px] h-[30px] focus-visible:ring-2"></div>
/// </div>
/// ```
fn with_inert(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().gap(2.0), |b| {
        b.div(tw().inert(), |b| {
            b.div_id("frozen", cell(), |_| {}).draw(mark("frozen"));
        });
        b.div_id("live", cell(), |_| {}).draw(mark("live"));
    });
}

#[test]
fn an_inert_subtree_is_not_a_tab_stop() {
    let mut harness = harness(with_inert);
    harness.run();
    support::past_bar(&mut harness);
    tab(&mut harness, egui::Modifiers::NONE);
    assert_eq!(focused(&harness), Some("live"));
    tab(&mut harness, egui::Modifiers::NONE);
    assert_eq!(focused(&harness), Some("live"), "the only stop, wrapped");
}

fn field_text(app: &mut App) -> &mut String {
    &mut app.text
}

/// ```html
/// <div class="flex flex-col gap-2 w-[240px]">
///   <div class="p-2 bg-[#282828] focus-within:bg-[#f0c80a]"> <input> </div>
///   <div class="p-2 bg-[#282828] focus-within:bg-[#f0c80a]"> <input> </div>
/// </div>
/// ```
fn two_fields(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().gap(2.0).w_px(240.0), |b| {
        for (key, pick) in [
            ("one", field_text as fn(&mut App) -> &mut String),
            ("two", |app: &mut App| &mut app.other),
        ] {
            b.div_id(
                key,
                tw().p(2.0).bg(PLAIN).focus_within(|p| p.bg(WITHIN)),
                |_| {},
            )
            .draw(move |app, ui| {
                ui.add(egui::TextEdit::singleline(pick(app)).hint_text(key));
            });
        }
    });
}

#[test]
fn focus_within_follows_a_text_edit_inside_a_node() {
    let mut harness = harness(two_fields);
    harness.run_steps(3);
    assert_eq!(support::fill_of(&harness, "/\"one\""), Some(PLAIN));
    assert_eq!(support::fill_of(&harness, "/\"two\""), Some(PLAIN));

    let one = rect_of(&harness, "/\"one\"");
    let at = one.center();
    hover_over(&mut harness, at);
    press_and_release(&mut harness, at);
    harness.run_steps(3);
    assert_eq!(support::fill_of(&harness, "/\"one\""), Some(WITHIN));
    assert_eq!(support::fill_of(&harness, "/\"two\""), Some(PLAIN));

    harness.key_press(Key::Tab);
    harness.run_steps(3);
    assert_eq!(support::fill_of(&harness, "/\"one\""), Some(PLAIN));
    assert_eq!(support::fill_of(&harness, "/\"two\""), Some(WITHIN));
}

/// ```html
/// <div class="w-[100px] h-[20px]"></div>
/// <div class="w-[200px]"> <!-- when the field has focus --> </div>
/// ```
fn growing(b: &mut Builder<'_, '_, App>, _: &App) {
    let wide = b.has_focus("field");
    b.div(tw().flex().gap(2.0), |b| {
        b.div_id(
            "box",
            tw().h_px(20.0).w_px(if wide { 200.0 } else { 100.0 }),
            |_| {},
        );
        b.div_id("field", cell(), |_| {}).draw(mark("field"));
    });
}

#[test]
fn has_focus_at_build_drives_a_layout_change_one_frame_later() {
    let mut harness = harness(growing);
    harness.run_steps(2);
    assert_eq!(rect_of(&harness, "/\"box\"").width(), 100.0);

    support::past_bar(&mut harness);
    harness.key_down(Key::Tab);
    harness.step();
    assert_eq!(
        harness.state().app.focused,
        Some("field"),
        "precondition: focus arrived in this frame"
    );
    assert_eq!(
        rect_of(&harness, "/\"box\"").width(),
        100.0,
        "the build saw the focus egui held at the start of the frame"
    );
    harness.step();
    assert_eq!(rect_of(&harness, "/\"box\"").width(), 200.0);
    harness.key_up(Key::Tab);
    harness.run_steps(5);
    assert_eq!(
        rect_of(&harness, "/\"box\"").width(),
        200.0,
        "no oscillation"
    );

    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert_eq!(rect_of(&harness, "/\"box\"").width(), 100.0);
}

#[test]
fn has_focus_within_sees_widgets_registered_in_the_previous_frame() {
    fn view(b: &mut Builder<'_, '_, App>, _: &App) {
        let wide = b.has_focus_within("form");
        b.div(tw().flex().gap(2.0), |b| {
            b.div_id(
                "box",
                tw().h_px(20.0).w_px(if wide { 200.0 } else { 100.0 }),
                |_| {},
            );
            b.div_id("form", tw().w_px(160.0).h_px(40.0), |_| {})
                .draw(|app, ui| {
                    ui.add(egui::TextEdit::singleline(&mut app.text).hint_text("form"));
                });
        });
    }
    let mut harness = harness(view);
    harness.run_steps(3);
    assert_eq!(rect_of(&harness, "/\"box\"").width(), 100.0);
    let at = rect_of(&harness, "/\"form\"").center();
    hover_over(&mut harness, at);
    press_and_release(&mut harness, at);
    harness.run_steps(3);
    assert_eq!(rect_of(&harness, "/\"box\"").width(), 200.0);
}

fn inset_ring_cell() -> eggshell_ui::Tw {
    tw().w_px(60.0)
        .h_px(30.0)
        .bg(PLAIN)
        .sense_click()
        .focus(|p| p.ring(2.0, RING).ring_inset())
}

fn ring_shapes(harness: &Harness<'_, Rig<App>>) -> Vec<RectShape> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.stroke.color == RING => Some(rect.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn ring_inset_paints_the_ring_inside_the_border_box_and_changes_no_layout() {
    fn plain(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div_id("cell", cell(), |_| {});
    }
    fn inset(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div_id("cell", inset_ring_cell(), |_| {});
    }
    let mut outside = harness(plain);
    let mut inside = harness(inset);
    outside.run();
    inside.run();
    assert_eq!(
        rect_of(&outside, "/\"cell\""),
        rect_of(&inside, "/\"cell\""),
        "same layout"
    );
    support::past_bar(&mut outside);
    support::past_bar(&mut inside);
    tab(&mut outside, egui::Modifiers::NONE);
    tab(&mut inside, egui::Modifiers::NONE);
    let node = rect_of(&inside, "/\"cell\"");
    let ring = ring_shapes(&inside);
    assert_eq!(ring.len(), 1, "precondition: the focused node has a ring");
    assert_eq!(ring[0].stroke_kind, egui::StrokeKind::Inside);
    assert_eq!(ring[0].rect, node);
    let ring = ring_shapes(&outside);
    assert_eq!(ring.len(), 1, "precondition");
    assert_eq!(
        ring[0].stroke_kind,
        egui::StrokeKind::Outside,
        "the default"
    );
}
