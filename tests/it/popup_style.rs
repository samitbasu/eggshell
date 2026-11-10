//! Risk 1 of the style contract: an egui `Popup` opened from inside a
//! bullseye node does not take the node `Ui`'s scoped style, it takes the
//! context's global style. The owned select therefore paints its list from
//! `style.select` alone, which the second test pins.

use bullseye::{LayoutState, NodeUi as _, TextSpec, TextStyle, tw};
use bullseye_shadcn::{Shadcn, ShadcnStyle, ThemeColor};
use egui::{Color32, FontFamily, Popup, SetOpenCommand, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

#[derive(Default)]
struct Probe {
    scoped: Option<Vec2>,
    in_popup: Option<Vec2>,
    global: Option<Vec2>,
}

const MARKER: Vec2 = vec2(77.0, 77.0);

#[test]
fn a_popup_opened_in_a_node_takes_the_global_style_not_the_nodes() {
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                let area = ui.max_rect();
                let base = TextStyle::new(
                    TextSpec {
                        size: 14.0,
                        line_height: 20.0,
                        family: FontFamily::Proportional,
                    },
                    Color32::BLACK,
                );
                probe.global = Some(ui.ctx().global_style().spacing.item_spacing);
                LayoutState::new()
                    .with_base_text(base)
                    .build(ui.ctx(), |b| {
                        b.div_id("host", tw().w_px(100.0).h_px(30.0).sense_click(), |_| {})
                            .draw(|probe: &mut Probe, ui| {
                                ui.style_mut().spacing.item_spacing = MARKER;
                                probe.scoped = Some(ui.spacing().item_spacing);
                                let response = ui.node_response();
                                Popup::from_response(&response)
                                    .open_memory(Some(SetOpenCommand::Bool(true)))
                                    .show(|popup_ui| {
                                        probe.in_popup = Some(popup_ui.spacing().item_spacing);
                                    });
                            });
                    })
                    .layout(area)
                    .draw(probe, ui);
            },
            Probe::default(),
        );
    harness.run_steps(3);
    let probe = harness.state();
    assert_eq!(
        probe.scoped,
        Some(MARKER),
        "precondition: the node is scoped"
    );
    assert_ne!(
        probe.global,
        Some(MARKER),
        "precondition: the marker is not global"
    );
    assert_eq!(
        probe.in_popup, probe.global,
        "the popup follows the context's style"
    );
    assert_ne!(probe.in_popup, probe.scoped, "and not the node's");
}

#[derive(Default)]
struct Pick(usize);

fn open_select_shapes(restyle_global: bool) -> Vec<String> {
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 250.0))
        .build_ui_state(
            |ui, pick: &mut Pick| {
                let ctx = ui.ctx().clone();
                Shadcn::apply_both_with(&ctx, ShadcnStyle::Lyra, ThemeColor::Neutral);
                if restyle_global {
                    ctx.global_style_mut(|style| {
                        let v = &mut style.visuals;
                        v.window_fill = Color32::RED;
                        v.window_stroke = egui::Stroke::new(3.0, Color32::GREEN);
                        v.window_shadow = egui::Shadow::NONE;
                        v.popup_shadow = egui::Shadow::NONE;
                        v.panel_fill = Color32::BLUE;
                        v.widgets.noninteractive.fg_stroke.color = Color32::YELLOW;
                        v.widgets.hovered.weak_bg_fill = Color32::BROWN;
                        v.selection.bg_fill = Color32::GOLD;
                        v.window_corner_radius = egui::CornerRadius::same(20);
                        v.menu_corner_radius = egui::CornerRadius::same(20);
                    });
                }
                let area = ui.max_rect();
                let base = TextStyle::new(
                    TextSpec {
                        size: 12.0,
                        line_height: 16.0,
                        family: FontFamily::Proportional,
                    },
                    Color32::BLACK,
                );
                LayoutState::new()
                    .with_base_text(base)
                    .build(&ctx, |b| {
                        b.select("select", pick.0, [(0, "One"), (1, "Two"), (2, "Three")])
                            .named("pick")
                            .on_change(|pick: &mut Pick, value| pick.0 = value);
                    })
                    .layout(area)
                    .draw(pick, ui);
            },
            Pick::default(),
        );
    harness.run_steps(2);
    harness.get_by_label("pick").click();
    harness.run_steps(4);
    harness
        .output()
        .shapes
        .iter()
        .map(|clipped| format!("{:?}", clipped.shape))
        .collect()
}

#[test]
fn the_select_list_does_not_depend_on_the_global_visuals() {
    let calm = open_select_shapes(false);
    let garish = open_select_shapes(true);
    let list = |shapes: &[String]| -> Vec<String> {
        let start = shapes
            .iter()
            .position(|shape| shape.starts_with("Vec"))
            .expect("the list casts its own shadow, and nothing else here does");
        shapes[start..].to_vec()
    };
    let (calm, garish) = (list(&calm), list(&garish));
    assert!(
        calm.iter().any(|shape| shape.contains("\"Three\"")),
        "precondition: the list holds the options"
    );
    assert_eq!(calm, garish, "the list is drawn from the contract alone");
}
