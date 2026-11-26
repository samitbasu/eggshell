//! Keyboard focus through the real path: Tab and Enter events dispatched by
//! `egui_kittest::Harness`, egui's focus traversal, the `focus` variants and
//! the focus record that answers `focus_within`.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, NodeUi, TextSpec, TextStyle, Weight, tw};
use egui::epaint::{ClippedShape, RectShape, Shape};
use egui::{Color32, FontFamily, Key, Rect, vec2};

use crate::support;

const RING: Color32 = Color32::from_rgb(10, 120, 240);
const WITHIN: Color32 = Color32::from_rgb(240, 200, 10);
const PLAIN: Color32 = Color32::from_rgb(40, 40, 40);

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
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

fn harness(view: support::View<App>) -> Driver<App> {
    support::driver(vec2(400.0, 200.0), base_text(), App::default(), view).build()
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

/// Tab with `modifiers` held, sent as a chord even with none held.
fn tab(d: &mut Driver<App>, modifiers: egui::Modifiers) {
    d.app_mut().focused = None;
    d.harness().key_press_modifiers(modifiers, Key::Tab);
    d.frames(2);
}

fn focused(d: &Driver<App>) -> Option<&'static str> {
    d.app().focused
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

fn rings(d: &Driver<App>) -> Vec<Rect> {
    let shapes: &[ClippedShape] = &d.harness().output().shapes;
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(RectShape { rect, stroke, .. }) if stroke.color == RING => Some(*rect),
            _ => None,
        })
        .collect()
}

/// The ring of the focused `ringed` is painted after the fill of `later`,
/// which comes later in paint order.
fn assert_ring_after(d: &Driver<App>, ringed: &str, later: &str) {
    let shapes: &[ClippedShape] = &d.harness().output().shapes;
    let at = |path: &str, stroked: bool| {
        let rect = d.rect(Target::suffix(&format!("/\"{path}\"")));
        shapes
            .iter()
            .position(|clipped| match &clipped.shape {
                Shape::Rect(r) if stroked => r.stroke.color == RING && r.rect == rect,
                Shape::Rect(r) => r.fill == PLAIN && r.rect == rect,
                _ => false,
            })
            .unwrap_or_else(|| panic!("nothing painted for {path}"))
    };
    assert!(
        at(later, false) < at(ringed, true),
        "the ring of {ringed} is painted over {later}"
    );
}

#[test]
fn tab_visits_nodes_in_paint_order_and_skips_the_opted_out() {
    let mut d = harness(row);
    d.harness_mut().run();
    assert!(
        d.rect(Target::suffix("/\"first\"")).min.x < d.rect(Target::suffix("/\"second\"")).min.x,
        "precondition: `first` is laid out before `second`"
    );

    // z paints `first` after its siblings, and egui registers in paint order.
    let mut order = Vec::new();
    support::drive_past_bar(&mut d);
    for _ in 0..3 {
        tab(&mut d, egui::Modifiers::NONE);
        order.push(focused(&d));
        if focused(&d) == Some("second") {
            assert_ring_after(&d, "second", "first");
        }
    }
    assert_eq!(
        order,
        [Some("second"), Some("last"), Some("first")],
        "the z-raised node is last in paint order, the opted-out one is absent"
    );
    tab(&mut d, egui::Modifiers::NONE);
    assert_eq!(
        support::focused_path(&d).as_deref(),
        Some("<widget>"),
        "the stop after the last is the bar's"
    );

    tab(&mut d, egui::Modifiers::SHIFT);
    assert_eq!(focused(&d), Some("first"));
    tab(&mut d, egui::Modifiers::SHIFT);
    assert_eq!(focused(&d), Some("last"));
}

#[test]
fn the_focused_node_paints_its_focus_variant() {
    let mut d = harness(row);
    d.harness_mut().run();
    assert!(rings(&d).is_empty(), "no focus, no ring");

    support::drive_past_bar(&mut d);
    tab(&mut d, egui::Modifiers::NONE);
    assert_eq!(focused(&d), Some("second"));
    assert_eq!(rings(&d), [d.rect(Target::suffix("/\"second\""))]);

    tab(&mut d, egui::Modifiers::NONE);
    assert_eq!(rings(&d), [d.rect(Target::suffix("/\"last\""))]);
}

#[test]
fn enter_and_space_click_the_focused_node() {
    let mut d = harness(row);
    d.harness_mut().run();
    support::drive_past_bar(&mut d);
    tab(&mut d, egui::Modifiers::NONE);
    assert_eq!(focused(&d), Some("second"));
    assert!(d.app().clicks.is_empty(), "precondition");

    d.press(Key::Enter).frames(1);
    d.press(Key::Space).frames(1);
    assert_eq!(d.app().clicks, ["second", "second"]);
}

#[test]
fn a_pointer_click_does_not_focus_the_node() {
    let mut d = harness(row);
    d.harness_mut().run();
    let at = d.rect(Target::suffix("/\"second\"")).center();
    d.click(at).frames(2);
    assert_eq!(d.app().clicks, ["second"], "precondition");
    assert_eq!(focused(&d), None);
    assert!(rings(&d).is_empty(), "this is focus-visible, not focus");
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
    let mut d = harness(with_inert);
    d.harness_mut().run();
    support::drive_past_bar(&mut d);
    tab(&mut d, egui::Modifiers::NONE);
    assert_eq!(focused(&d), Some("live"));
    tab(&mut d, egui::Modifiers::NONE);
    assert_eq!(focused(&d), Some("live"), "the only stop, wrapped");
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
    let mut d = harness(two_fields);
    d.frames(3);
    assert_eq!(support::fill_of(&d, "/\"one\""), Some(PLAIN));
    assert_eq!(support::fill_of(&d, "/\"two\""), Some(PLAIN));

    let one = d.rect(Target::suffix("/\"one\""));
    let at = one.center();
    d.click(at).frames(3);
    assert_eq!(support::fill_of(&d, "/\"one\""), Some(WITHIN));
    assert_eq!(support::fill_of(&d, "/\"two\""), Some(PLAIN));

    d.press(Key::Tab).frames(2);
    assert_eq!(support::fill_of(&d, "/\"one\""), Some(PLAIN));
    assert_eq!(support::fill_of(&d, "/\"two\""), Some(WITHIN));
}

/// A press held on a text field does not take focus back once Tab has moved
/// it on, however long it is held: egui's field claims focus in every pass
/// it is dragged. Found through #219.
#[test]
fn tab_away_from_a_held_text_field_keeps_focus_where_tab_put_it() {
    let mut d = harness(two_fields);
    d.frames(3);
    let at = d.rect(Target::suffix("/\"one\"")).center();
    d.hover(at);
    d.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    })
    .frames(1);
    assert_eq!(
        support::fill_of(&d, "/\"one\""),
        Some(WITHIN),
        "precondition: the press focused the field"
    );
    let pressed_at = d.ctx().input(|input| input.time);
    d.press(Key::Tab).frames(59);
    let held = d.ctx().input(|input| input.time) - pressed_at;
    let click = d
        .ctx()
        .options(|options| options.input_options.max_click_duration);
    assert!(
        held > click,
        "precondition: held {held} s, past a click's {click} s, so egui drags"
    );
    assert!(
        d.ctx().input(|input| input.pointer.primary_down()),
        "precondition: still held"
    );
    assert_eq!(support::fill_of(&d, "/\"one\""), Some(PLAIN));
    assert_eq!(support::fill_of(&d, "/\"two\""), Some(WITHIN));
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
    let mut d = harness(growing);
    d.frames(2);
    assert_eq!(d.rect(Target::suffix("/\"box\"")).width(), 100.0);

    support::drive_past_bar(&mut d);
    d.harness().key_down(Key::Tab);
    d.frames(1);
    assert_eq!(
        d.app().focused,
        Some("field"),
        "precondition: focus arrived in this frame"
    );
    assert_eq!(
        d.rect(Target::suffix("/\"box\"")).width(),
        100.0,
        "the build saw the focus egui held at the start of the frame"
    );
    d.frames(1);
    assert_eq!(d.rect(Target::suffix("/\"box\"")).width(), 200.0);
    d.harness().key_up(Key::Tab);
    d.frames(5);
    assert_eq!(
        d.rect(Target::suffix("/\"box\"")).width(),
        200.0,
        "no oscillation"
    );

    d.press(Key::Escape).frames(2);
    assert_eq!(d.rect(Target::suffix("/\"box\"")).width(), 100.0);
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
    let mut d = harness(view);
    d.frames(3);
    assert_eq!(d.rect(Target::suffix("/\"box\"")).width(), 100.0);
    let at = d.rect(Target::suffix("/\"form\"")).center();
    d.click(at).frames(3);
    assert_eq!(d.rect(Target::suffix("/\"box\"")).width(), 200.0);
}

fn inset_ring_cell() -> eggshell_ui::Tw {
    tw().w_px(60.0)
        .h_px(30.0)
        .bg(PLAIN)
        .sense_click()
        .focus(|p| p.ring(2.0, RING).ring_inset())
}

fn ring_shapes(d: &Driver<App>) -> Vec<RectShape> {
    d.harness()
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
    outside.harness_mut().run();
    inside.harness_mut().run();
    assert_eq!(
        outside.rect(Target::suffix("/\"cell\"")),
        inside.rect(Target::suffix("/\"cell\"")),
        "same layout"
    );
    support::drive_past_bar(&mut outside);
    support::drive_past_bar(&mut inside);
    tab(&mut outside, egui::Modifiers::NONE);
    tab(&mut inside, egui::Modifiers::NONE);
    let node = inside.rect(Target::suffix("/\"cell\""));
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
