//! TODO(prose): a layout's own style (#435): given to the `LayoutState` as a light and
//! dark pair and swapped through it, in place of the one derived from
//! egui's, for the build, the draw, the shell, tooltips, the base text and
//! the faces text is drawn in; and two layouts that give egui different
//! looks are reported.

use std::sync::Arc;

use eggshell_ui::contract::Themed;
use eggshell_ui::testing::reports;
use eggshell_ui::{Builder, LayoutState, NodeUi as _, Problem, Style, TextSpec, Weight, tw};
use egui::{FontFamily, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

const LIGHT: f32 = 11.0;
const DARK: f32 = 22.0;

fn padded(padding: f32) -> Arc<Style> {
    let mut style = Style::default();
    style.scroll.width = padding;
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
    *built = Some(b.style().scroll.width);
    b.span("probe", tw(), "probe").draw(|_, ui| {
        let padding = ui.node_style().scroll.width;
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
    harness.run_steps(2);
    harness
}

/// The table rule width of the style derived from egui's.
fn derived() -> f32 {
    Style::default().scroll.width
}

#[test]
fn a_layout_s_own_style_is_the_one_it_builds_and_draws_with() {
    assert_ne!(derived(), LIGHT, "precondition");
    let harness = harness(LayoutState::new(pair()));
    assert_eq!(harness.state().built, Some(LIGHT));
    assert_eq!(harness.state().drawn, Some(LIGHT));
}

#[test]
fn the_pair_follows_the_context_s_theme() {
    let mut harness = harness(LayoutState::new(pair()));
    assert_eq!(harness.state().built, Some(LIGHT), "precondition: light");
    harness.ctx.set_theme(Theme::Dark);
    harness.run_steps(2);
    assert_eq!(harness.state().built, Some(DARK));
    assert_eq!(harness.state().drawn, Some(DARK));
}

#[test]
fn set_style_swaps_the_style_from_the_next_build() {
    let mut harness = harness(LayoutState::new(pair()));
    harness.state_mut().layout.set_style(Themed {
        light: padded(33.0),
        dark: padded(33.0),
    });
    harness.run_steps(1);
    assert_eq!(harness.state().built, Some(33.0));
}

#[test]
fn the_shell_reads_the_layout_s_style() {
    let harness = harness(LayoutState::new(pair()));
    let shell = harness.state().layout.shell(&harness.ctx);
    assert_eq!(shell.style().scroll.width, LIGHT);
    let plain = LayoutState::new(crate::support::shadcn());
    assert_eq!(
        plain.shell(&harness.ctx).style().scroll.width,
        derived(),
        "without a style of its own, the derived one"
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
            LayoutState::new(Themed {
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
        let mut tooltip = style.tooltip.clone();
        tooltip.delay = delay;
        style.tooltip = tooltip;
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
            LayoutState::new(Themed {
                light: delayed(std::time::Duration::ZERO),
                dark: delayed(std::time::Duration::ZERO),
            }),
        );
    assert!(
        Style::default().tooltip.delay > std::time::Duration::from_millis(300),
        "precondition: the derived style would wait past the six steps"
    );
    harness.run_steps(2);
    harness.get_by_label("Save").hover();
    harness.run_steps(6);
    assert!(
        harness.query_by_label("Saves it").is_some(),
        "the layout's style opens it at once"
    );
}

/// A style that declares a face for Proportional at Medium.
/// The default style without the tokens' font, so it declares no weighted face.
fn fontless() -> Style {
    Style::new(&eggshell_ui::theme::Tokens {
        font: None,
        ..eggshell_ui::theme::Tokens::light()
    })
}

fn with_medium() -> Arc<Style> {
    let mut style = fontless();
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
    let (missing, _) = medium_reports(LayoutState::new(Themed::both(Arc::new(fontless()))));
    assert_eq!(
        missing, 1,
        "precondition: the style declares no medium face"
    );
    let (missing, _) = medium_reports(LayoutState::new(Themed {
        light: with_medium(),
        dark: with_medium(),
    }));
    assert_eq!(missing, 0, "the layout's style declares one");
}

#[test]
fn the_symbol_face_backs_the_families_the_layout_s_style_declares() {
    let (_, registered) = medium_reports(LayoutState::new(crate::support::shadcn()));
    assert!(!registered, "precondition: no layout declares the family");
    let (_, registered) = medium_reports(LayoutState::new(Themed {
        light: with_medium(),
        dark: with_medium(),
    }));
    assert!(registered, "the build registers the symbol face under it");
}

fn vega() -> Themed<Arc<Style>> {
    eggshell_ui::theme::Shadcn::themed(
        eggshell_ui::theme::ShadcnStyle::Vega,
        eggshell_ui::theme::Palette::new(),
    )
}

fn lyra() -> Themed<Arc<Style>> {
    eggshell_ui::theme::Shadcn::themed(
        eggshell_ui::theme::ShadcnStyle::Lyra,
        eggshell_ui::theme::Palette::new(),
    )
}

/// A new style given before the frame `at`: to the second layout, and to
/// the first too if `both` says so.
#[derive(Clone, Copy)]
struct Switch {
    at: usize,
    to: fn() -> Themed<Arc<Style>>,
    both: Both,
}

/// Two layouts built on one context for `frames` frames, with `switch`;
/// the reports of layouts giving egui different looks.
fn clash_reports(
    layouts: (LayoutState, LayoutState),
    switch: Option<Switch>,
    frames: usize,
) -> usize {
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .build_ui_state(
            |ui, (first, second): &mut (LayoutState, LayoutState)| {
                for layout in [first, second] {
                    layout
                        .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                            b.text(tw(), "x");
                        })
                        .layout(ui.max_rect())
                        .draw(&mut (), ui);
                }
            },
            layouts,
        );
    for frame in 0..frames {
        if let Some(Switch { at, to, both }) = switch
            && at == frame
        {
            let (first, second) = harness.state_mut();
            second.set_style(to());
            if both == Both::Switched {
                first.set_style(to());
            }
        }
        harness.step();
    }
    reports(&harness.ctx)
        .iter()
        .filter(|report| report.problem == Problem::EguiStyleClash)
        .count()
}

/// Whether a switch goes to both layouts or to the second alone.
#[derive(Clone, Copy, PartialEq)]
enum Both {
    Switched,
    SecondOnly,
}

#[test]
fn two_layouts_that_give_egui_different_looks_are_reported() {
    let layouts = (LayoutState::new(vega()), LayoutState::new(lyra()));
    assert_eq!(clash_reports(layouts, None, 4), 1);
}

#[test]
fn the_same_style_given_by_two_layouts_is_not_reported() {
    let shared = vega();
    let layouts = (LayoutState::new(shared.clone()), LayoutState::new(shared));
    assert_eq!(clash_reports(layouts, None, 4), 0, "one style, shared");
    let layouts = (LayoutState::new(vega()), LayoutState::new(vega()));
    assert_eq!(clash_reports(layouts, None, 4), 0, "one style, built twice");
}

#[test]
fn a_style_switched_on_every_layout_at_once_is_not_reported() {
    let layouts = (LayoutState::new(vega()), LayoutState::new(vega()));
    assert_eq!(
        clash_reports(
            layouts,
            Some(Switch {
                at: 2,
                to: lyra,
                both: Both::Switched
            }),
            5
        ),
        0
    );
}

#[test]
fn a_style_switched_on_one_layout_alone_is_reported() {
    let layouts = (LayoutState::new(vega()), LayoutState::new(vega()));
    assert_eq!(
        clash_reports(
            layouts,
            Some(Switch {
                at: 2,
                to: lyra,
                both: Both::SecondOnly
            }),
            5
        ),
        1
    );
}
