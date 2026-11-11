//! PROTOTYPE (M6 design pass, docs/overlays.md). Throwaway evidence about how
//! egui layers behave, so the overlay design rests on facts rather than on a
//! reading of the source. Nothing here is library behaviour; the build PRs
//! replace these with tests of the real overlays.

use egui::{
    Area, Context, Event, Id, Key, LayerId, Modifiers, Order, PointerButton, Pos2, Rect, Sense, Ui,
    pos2, vec2,
};
use egui_kittest::Harness;

#[derive(Default)]
struct World {
    /// Layers in the order they are shown (and moved to the top) each frame.
    layers: Vec<(Order, &'static str)>,
    base_clicks: u32,
    popup_clicks: u32,
    modal: Option<&'static str>,
    text: String,
    base_focus: Option<bool>,
    backdrop: bool,
}

fn harness<'a>(app: impl FnMut(&mut Ui, &mut World) + 'a, world: World) -> Harness<'a, World> {
    Harness::builder()
        .with_accessibility_check(false)
        .build_ui_state(app, world)
}

fn layer(order: Order, name: &'static str) -> LayerId {
    LayerId::new(order, Id::NULL.with(name))
}

fn show_boxes(ctx: &Context, world: &mut World) {
    for &(order, name) in &world.layers {
        let at = match name {
            "wide" => pos2(20.0, 20.0),
            _ => pos2(60.0, 60.0),
        };
        let id = layer(order, name);
        Area::new(id.id)
            .order(order)
            .fixed_pos(at)
            .sense(Sense::click())
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(vec2(100.0, 100.0), Sense::click());
                if response.clicked() {
                    world.popup_clicks += 1;
                }
            });
        ctx.move_to_top(id);
    }
}

fn click(harness: &mut Harness<'_, World>, at: Pos2) {
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
}

fn top_at(harness: &Harness<'_, World>, at: Pos2) -> Option<LayerId> {
    harness.ctx.layer_id_at(at)
}

fn overlap_harness(layers: Vec<(Order, &'static str)>) -> Harness<'static, World> {
    let world = World {
        layers,
        ..World::default()
    };
    let mut harness = harness(|ui: &mut Ui, w: &mut World| show_boxes(ui.ctx(), w), world);
    harness.run_steps(3);
    harness
}

const OVERLAP: Pos2 = pos2(100.0, 100.0);

#[test]
fn order_decides_before_the_stacking_within_an_order() {
    let mut harness = overlap_harness(vec![
        (Order::Foreground, "popup"),
        (Order::Middle, "wide"),
        (Order::Tooltip, "tip"),
        (Order::Foreground, "submenu"),
    ]);
    assert_eq!(
        top_at(&harness, OVERLAP),
        Some(layer(Order::Tooltip, "tip"))
    );
    harness.state_mut().layers.retain(|(_, n)| *n != "tip");
    harness.run_steps(3);
    assert_eq!(
        top_at(&harness, OVERLAP),
        Some(layer(Order::Foreground, "submenu"))
    );
}

/// FINDING: `move_to_top` sets a flag; egui stable-sorts by `(order, flag)`.
/// Flagging every overlay every frame lifts them above other layers of the
/// same order, but it cannot reorder them among themselves: the stacking is
/// the order in which the layers were first shown, whatever order they are
/// declared and flagged in. D18's "build order" is not implementable as such.
#[test]
fn flagging_every_frame_in_a_new_declaration_order_does_not_restack() {
    let mut harness = overlap_harness(vec![
        (Order::Foreground, "popup"),
        (Order::Foreground, "submenu"),
    ]);
    assert_eq!(
        top_at(&harness, OVERLAP),
        Some(layer(Order::Foreground, "submenu"))
    );
    harness.state_mut().layers.swap(0, 1);
    harness.run_steps(3);
    assert_eq!(
        top_at(&harness, OVERLAP),
        Some(layer(Order::Foreground, "submenu")),
        "declaration order did not restack"
    );
}

/// Two overlapping Foreground layers, "popup" under "submenu"; a press on
/// the exposed part of the popup. `flag_all` is the driver calling
/// `move_to_top` for every overlay every frame.
fn press_on_the_parent(flag_all: bool) -> Option<LayerId> {
    let world = World {
        layers: vec![(Order::Foreground, "popup"), (Order::Foreground, "submenu")],
        backdrop: flag_all,
        ..World::default()
    };
    let mut harness = harness(
        |ui: &mut Ui, w: &mut World| {
            let ctx = ui.ctx().clone();
            for (i, &(order, name)) in w.layers.iter().enumerate() {
                let id = layer(order, name);
                let at = if i == 0 {
                    pos2(60.0, 60.0)
                } else {
                    pos2(100.0, 100.0)
                };
                Area::new(id.id)
                    .order(order)
                    .fixed_pos(at)
                    .show(&ctx, |ui| {
                        ui.allocate_exact_size(vec2(100.0, 100.0), Sense::click());
                    });
                if w.backdrop {
                    ctx.move_to_top(id);
                }
            }
        },
        world,
    );
    harness.run_steps(3);
    let overlap = pos2(120.0, 120.0);
    assert_eq!(
        top_at(&harness, overlap),
        Some(layer(Order::Foreground, "submenu"))
    );
    click(&mut harness, pos2(70.0, 70.0));
    harness.run_steps(2);
    top_at(&harness, overlap)
}

/// Why the flag must be set every frame: egui raises an area on a press,
/// which would put a parent popup above its open submenu.
#[test]
fn a_press_on_a_parent_raises_it_over_its_submenu_unless_all_are_flagged() {
    assert_eq!(
        press_on_the_parent(false),
        Some(layer(Order::Foreground, "popup")),
        "unflagged: the press raised the parent over its submenu"
    );
    assert_eq!(
        press_on_the_parent(true),
        Some(layer(Order::Foreground, "submenu")),
        "flagged every frame: the stacking survives the press"
    );
}

#[test]
fn a_modal_layer_blocks_layers_below_it_and_not_those_above() {
    let world = World {
        modal: Some("modal"),
        ..World::default()
    };
    let mut harness = harness(
        |ui: &mut Ui, w: &mut World| {
            let ctx = ui.ctx().clone();
            let (_, base) = ui.allocate_exact_size(vec2(200.0, 200.0), Sense::click());
            if base.clicked() {
                w.base_clicks += 1;
            }
            let field = ui.text_edit_singleline(&mut w.text);
            w.base_focus = Some(field.has_focus());
            if w.modal.is_some() {
                let id = layer(Order::Middle, "modal");
                ctx.memory_mut(|m| m.set_modal_layer(id));
                Area::new(id.id)
                    .order(Order::Middle)
                    .fixed_pos(pos2(100.0, 100.0))
                    .show(&ctx, |ui| {
                        if w.backdrop {
                            let screen = ui.ctx().content_rect();
                            ui.interact(screen, id.id.with("backdrop"), Sense::click_and_drag());
                        }
                        ui.label("dialog");
                        ui.add_space(40.0);
                    });
                ctx.move_to_top(id);
                // A popup opened from inside the dialog sits above it and
                // stays interactive.
                let popup = layer(Order::Foreground, "popup");
                Area::new(popup.id)
                    .order(Order::Foreground)
                    .fixed_pos(pos2(100.0, 200.0))
                    .show(&ctx, |ui| {
                        let (_, r) = ui.allocate_exact_size(vec2(60.0, 30.0), Sense::click());
                        if r.clicked() {
                            w.popup_clicks += 1;
                        }
                    });
                ctx.move_to_top(popup);
            }
        },
        world,
    );
    harness.run_steps(3);
    // FINDING: `set_modal_layer` alone traps Tab but not the pointer; the
    // pointer is blocked by a full-screen widget in the modal layer, which is
    // what egui::Modal's backdrop is.
    click(&mut harness, pos2(20.0, 20.0));
    harness.run_steps(2);
    assert_eq!(
        harness.state().base_clicks,
        1,
        "no backdrop widget: the pointer passes"
    );
    harness.state_mut().backdrop = true;
    harness.run_steps(3);
    click(&mut harness, pos2(20.0, 20.0));
    harness.run_steps(2);
    assert_eq!(
        harness.state().base_clicks,
        1,
        "with the backdrop widget the base is blocked"
    );
    click(&mut harness, pos2(120.0, 210.0));
    harness.run_steps(2);
    assert_eq!(
        harness.state().popup_clicks,
        1,
        "a layer above the modal is live"
    );
    // Tab does not reach the base text field.
    harness.key_press(Key::Tab);
    harness.run_steps(3);
    assert_eq!(
        harness.state().base_focus,
        Some(false),
        "focus is trapped above the modal"
    );
    // Closing the modal frees the base after the same one-frame lag.
    harness.state_mut().modal = None;
    harness.run_steps(3);
    click(&mut harness, pos2(20.0, 20.0));
    harness.run_steps(2);
    assert_eq!(harness.state().base_clicks, 2);
}

#[test]
fn escape_blurs_the_focused_widget_before_any_overlay_code_runs() {
    let mut harness = harness(
        |ui: &mut Ui, w: &mut World| {
            let field = ui.text_edit_singleline(&mut w.text);
            w.base_focus = Some(field.has_focus());
        },
        World::default(),
    );
    harness.run_steps(2);
    // Focus the field by clicking it.
    click(&mut harness, pos2(30.0, 12.0));
    harness.run_steps(2);
    assert_eq!(harness.state().base_focus, Some(true));
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert_eq!(
        harness.state().base_focus,
        Some(false),
        "Esc surrenders focus in egui itself: an overlay that closes on Esc must re-request \
         focus for the trigger or field it came from"
    );
}

#[test]
fn a_press_keeps_its_origin_through_a_drag_that_ends_elsewhere() {
    let mut harness = harness(|_: &mut Ui, _: &mut World| {}, World::default());
    harness.run_steps(2);
    let inside = pos2(50.0, 50.0);
    let outside = pos2(300.0, 150.0);
    harness.event(Event::PointerButton {
        pos: inside,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let origin = |h: &Harness<'_, World>| h.ctx.input(|i| i.pointer.press_origin());
    let pressed = |h: &Harness<'_, World>| h.ctx.input(|i| i.pointer.any_pressed());
    assert!(pressed(&harness));
    assert_eq!(origin(&harness), Some(inside));
    harness.hover_at(outside);
    harness.step();
    assert!(
        !pressed(&harness),
        "any_pressed is true on the press frame only"
    );
    assert_eq!(
        origin(&harness),
        Some(inside),
        "the origin outlives the move"
    );
    harness.event(Event::PointerButton {
        pos: outside,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let released = harness.ctx.input(|i| i.pointer.any_released());
    assert!(released);
    // Dismissal judged at `any_pressed` + the press position never sees this
    // release, so a drag that began inside cannot dismiss on release outside.
    let _ = Rect::NOTHING;
}

#[derive(Default)]
struct Scrolled {
    offset: f32,
    cover: bool,
}

fn wheel_over_the_page(cover: bool) -> f32 {
    let mut harness = Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(300.0, 200.0))
        .build_ui_state(
            |ui: &mut Ui, s: &mut Scrolled| {
                let out = egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.allocate_exact_size(vec2(250.0, 1000.0), Sense::hover());
                });
                s.offset = out.state.clamped_offset().y;
                if s.cover {
                    let ctx = ui.ctx().clone();
                    let screen = ctx.content_rect();
                    let id = layer(Order::Foreground, "scrim");
                    Area::new(id.id)
                        .order(Order::Foreground)
                        .fixed_pos(screen.min)
                        .show(&ctx, |ui| {
                            ui.interact(screen, id.id.with("blocker"), Sense::click_and_drag());
                            ui.allocate_rect(screen, Sense::hover());
                        });
                    ctx.move_to_top(id);
                }
            },
            Scrolled {
                cover,
                ..Scrolled::default()
            },
        );
    harness.run_steps(3);
    harness.hover_at(pos2(100.0, 100.0));
    harness.run_steps(2);
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        source: egui::MouseWheelSource::Unknown,
        delta: vec2(0.0, -50.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(4);
    harness.state().offset
}

/// A screen-sized layer with a blocker widget also takes the wheel from the
/// page: scroll lock needs no code of its own.
#[test]
fn a_screen_sized_modal_layer_takes_the_wheel_from_the_page() {
    assert!(
        wheel_over_the_page(false) > 0.0,
        "control: the page scrolls"
    );
    assert_eq!(wheel_over_the_page(true), 0.0);
}
