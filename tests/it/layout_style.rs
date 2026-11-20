//! A layout's own style (#435): given to the `LayoutState` as a light and
//! dark pair and swapped through it, in place of the style installed in the
//! context, for the build, the draw, the shell, tooltips, the base text and
//! the faces text is drawn in.

use std::sync::Arc;

use eggshell_ui::contract::Themed;
use eggshell_ui::testing::reports;
use eggshell_ui::{Builder, LayoutState, NodeUi as _, Problem, Style, TextSpec, Weight, tw};
use egui::{FontFamily, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

const INSTALLED: f32 = 4.0;
const LIGHT: f32 = 11.0;
const DARK: f32 = 22.0;

fn padded(padding: f32) -> Arc<Style> {
    let mut style = Style::default();
    style.table.cell_padding = padding;
    Arc::new(style)
}

fn pair() -> Themed<Arc<Style>> {
    Themed {
        light: padded(LIGHT),
        dark: padded(DARK),
    }
}

struct Scene {
    layout: LayoutState,
    /// What the build's `Builder::style` gave.
    built: Option<f32>,
    /// What a draw closure's `node_style` gave.
    drawn: Option<f32>,
}

fn view(b: &mut Builder<'_, '_, ()>, built: &mut Option<f32>) {
    *built = Some(b.style().table.cell_padding);
    b.span("probe", tw(), "probe").draw(|_, ui| {
        let padding = ui.node_style().table.cell_padding;
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::NULL.with("drawn"), padding));
    });
}

fn harness(layout: LayoutState) -> Harness<'static, Scene> {
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .with_theme(Theme::Light)
        .build_ui_state(
            |ui, scene: &mut Scene| {
                let mut built = None;
                scene
                    .layout
                    .build(ui.ctx(), |b| view(b, &mut built))
                    .layout(ui.max_rect())
                    .draw(&mut (), ui);
                scene.built = built;
                scene.drawn = ui
                    .ctx()
                    .data(|data| data.get_temp(egui::Id::NULL.with("drawn")));
            },
            Scene {
                layout,
                built: None,
                drawn: None,
            },
        );
    padded(INSTALLED).as_ref().clone().install(&harness.ctx);
    harness.run_steps(2);
    harness
}

#[test]
fn without_a_style_of_its_own_a_layout_builds_with_the_installed_one() {
    let harness = harness(LayoutState::new());
    assert_eq!(harness.state().built, Some(INSTALLED));
    assert_eq!(harness.state().drawn, Some(INSTALLED));
}

#[test]
fn a_layout_s_own_style_wins_over_the_installed_one_at_build_and_draw() {
    let harness = harness(LayoutState::new().with_style(pair()));
    assert_eq!(harness.state().built, Some(LIGHT));
    assert_eq!(harness.state().drawn, Some(LIGHT));
}

#[test]
fn the_pair_follows_the_context_s_theme() {
    let mut harness = harness(LayoutState::new().with_style(pair()));
    assert_eq!(harness.state().built, Some(LIGHT), "precondition: light");
    harness.ctx.set_theme(Theme::Dark);
    harness.run_steps(2);
    assert_eq!(harness.state().built, Some(DARK));
    assert_eq!(harness.state().drawn, Some(DARK));
}

#[test]
fn set_style_swaps_the_style_from_the_next_build() {
    let mut harness = harness(LayoutState::new().with_style(pair()));
    harness.state_mut().layout.set_style(Themed {
        light: padded(33.0),
        dark: padded(33.0),
    });
    harness.run_steps(1);
    assert_eq!(harness.state().built, Some(33.0));
}

#[test]
fn the_shell_reads_the_layout_s_style() {
    let harness = harness(LayoutState::new().with_style(pair()));
    let shell = harness.state().layout.shell(&harness.ctx);
    assert_eq!(shell.style().table.cell_padding, LIGHT);
    let plain = LayoutState::new();
    assert_eq!(
        plain.shell(&harness.ctx).style().table.cell_padding,
        INSTALLED,
        "without a style of its own, the installed one"
    );
}

#[test]
fn without_base_text_the_root_text_is_the_layout_s_style_text() {
    fn tall() -> Arc<Style> {
        let mut style = Style::default();
        style.text.line_height = 40.0;
        Arc::new(style)
    }
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .build_ui_state(
            |ui, layout: &mut LayoutState| {
                let placed = layout
                    .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                        b.span("line", tw(), "Line");
                    })
                    .layout(ui.max_rect());
                let height = placed.at("line").rect().height();
                ui.ctx()
                    .data_mut(|data| data.insert_temp(egui::Id::NULL.with("height"), height));
                placed.draw(&mut (), ui);
            },
            LayoutState::new().with_style(Themed {
                light: tall(),
                dark: tall(),
            }),
        );
    harness.run_steps(2);
    let height: Option<f32> = harness
        .ctx
        .data(|data| data.get_temp(egui::Id::NULL.with("height")));
    assert_eq!(height, Some(40.0));
}

#[test]
fn a_tooltip_waits_as_the_layout_s_style_says() {
    fn delayed(delay: std::time::Duration) -> Arc<Style> {
        let mut style = Style::default();
        style.tooltip.delay = delay;
        Arc::new(style)
    }
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .with_step_dt(0.05)
        .build_ui_state(
            |ui, layout: &mut LayoutState| {
                layout
                    .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                        b.div(tw().p(20.0), |b| {
                            b.button("save", "Save").tooltip("Saves it").add();
                        });
                    })
                    .layout(ui.max_rect())
                    .draw(&mut (), ui);
            },
            LayoutState::new().with_style(Themed {
                light: delayed(std::time::Duration::ZERO),
                dark: delayed(std::time::Duration::ZERO),
            }),
        );
    delayed(std::time::Duration::from_secs(60))
        .as_ref()
        .clone()
        .install(&harness.ctx);
    harness.run_steps(2);
    harness.get_by_label("Save").hover();
    harness.run_steps(6);
    assert!(
        harness.query_by_label("Saves it").is_some(),
        "the layout's style opens it at once; the installed one would wait a minute"
    );
}

/// A style that declares a face for Proportional at Medium.
fn with_medium() -> Arc<Style> {
    let mut style = Style::default();
    style.faces = style.faces.clone().with(
        FontFamily::Proportional,
        Weight::Medium,
        FontFamily::Name("bespoke-medium".into()),
    );
    Arc::new(style)
}

fn medium_reports(layout: LayoutState) -> (usize, bool) {
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .build_ui_state(
            |ui, layout: &mut LayoutState| {
                layout
                    .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                        b.span("line", tw(), "Line");
                    })
                    .layout(ui.max_rect())
                    .draw(&mut (), ui);
            },
            layout,
        );
    harness.run_steps(3);
    let spec = TextSpec {
        size: 14.0,
        line_height: 20.0,
        family: FontFamily::Proportional,
        weight: Weight::Medium,
    };
    spec.font_id(&harness.ctx);
    let missing = reports(&harness.ctx)
        .iter()
        .filter(|report| report.problem == Problem::MissingFace)
        .count();
    let registered = harness
        .ctx
        .fonts(|fonts| fonts.families())
        .contains(&FontFamily::Name("bespoke-medium".into()));
    (missing, registered)
}

#[test]
fn text_is_drawn_in_the_faces_the_layout_s_style_declares() {
    let (missing, _) = medium_reports(LayoutState::new());
    assert_eq!(
        missing, 1,
        "precondition: the default style declares no medium face"
    );
    let (missing, _) = medium_reports(LayoutState::new().with_style(Themed {
        light: with_medium(),
        dark: with_medium(),
    }));
    assert_eq!(missing, 0, "the layout's style declares one");
}

#[test]
fn the_symbol_face_backs_the_families_the_layout_s_style_declares() {
    let (_, registered) = medium_reports(LayoutState::new());
    assert!(!registered, "precondition: no layout declares the family");
    let (_, registered) = medium_reports(LayoutState::new().with_style(Themed {
        light: with_medium(),
        dark: with_medium(),
    }));
    assert!(registered, "the build registers the symbol face under it");
}
