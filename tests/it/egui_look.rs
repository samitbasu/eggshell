//! egui's own look from a layout's style (#435): a layout given a style of
//! its own sets egui's visuals, spacing and fonts from it, exactly as the
//! preset's `apply` does, so raw egui widgets look as they always have.

use eggshell_ui::contract::Themed;
use eggshell_ui::{Builder, LayoutState, tw};
use eggshell_ui_shadcn::{Palette, Shadcn, ShadcnStyle};
use egui::{Context, Theme, vec2};
use egui_kittest::Harness;

/// egui's styles for both themes, in their debug form (their number
/// formatters compare by pointer, and each context makes its own), and its
/// fonts' names, after `setup`.
fn egui_after(
    setup: impl FnOnce(&Context),
    layout: Option<LayoutState>,
) -> (String, String, Vec<String>) {
    let mut harness = Harness::builder()
        .with_size(vec2(200.0, 100.0))
        .build_ui_state(
            |ui, layout: &mut Option<LayoutState>| {
                if let Some(layout) = layout {
                    layout
                        .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                            b.div(tw(), |_| {});
                        })
                        .layout(ui.max_rect())
                        .draw(&mut (), ui);
                }
            },
            layout,
        );
    setup(&harness.ctx);
    harness.run_steps(3);
    let ctx = &harness.ctx;
    let mut faces: Vec<String> =
        ctx.fonts(|fonts| fonts.definitions().font_data.keys().cloned().collect());
    faces.sort();
    (
        format!("{:#?}", ctx.style_of(Theme::Light)),
        format!("{:#?}", ctx.style_of(Theme::Dark)),
        faces,
    )
}

fn applied(style: ShadcnStyle) -> (String, String, Vec<String>) {
    egui_after(
        |ctx| Shadcn::apply_both_with(ctx, style, Palette::new()),
        Some(LayoutState::new()),
    )
}

fn themed(style: ShadcnStyle) -> (String, String, Vec<String>) {
    egui_after(
        |_| {},
        Some(LayoutState::new().with_style(Shadcn::themed(style, Palette::new()))),
    )
}

#[test]
fn a_layout_s_style_gives_egui_the_preset_s_look_in_both_themes() {
    for style in ShadcnStyle::ALL {
        let (light, dark, _) = applied(style);
        let (themed_light, themed_dark, _) = themed(style);
        assert!(light == themed_light, "{style:?} light");
        assert!(dark == themed_dark, "{style:?} dark");
    }
}

#[test]
fn a_layout_s_style_gives_egui_the_preset_s_fonts() {
    let (_, _, preset) = applied(ShadcnStyle::Vega);
    let (_, _, layout) = themed(ShadcnStyle::Vega);
    let plain = egui_after(|_| {}, Some(LayoutState::new())).2;
    assert_ne!(preset, plain, "precondition: the preset adds fonts");
    assert_eq!(layout, preset);
}

#[test]
fn a_layout_without_a_style_of_its_own_leaves_egui_s_look_alone() {
    let plain = egui_after(|_| {}, None);
    let unstyled = egui_after(|_| {}, Some(LayoutState::new()));
    assert!(plain.0 == unstyled.0 && plain.1 == unstyled.1);
}

#[test]
fn set_style_gives_egui_the_new_look_on_the_next_build() {
    let (lyra_light, _, _) = applied(ShadcnStyle::Lyra);
    let (vega_light, _, _) = applied(ShadcnStyle::Vega);
    assert!(lyra_light != vega_light, "precondition: the styles differ");
    let mut harness = Harness::builder()
        .with_size(vec2(200.0, 100.0))
        .with_theme(Theme::Light)
        .build_ui_state(
            |ui, layout: &mut LayoutState| {
                layout
                    .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                        b.div(tw(), |_| {});
                    })
                    .layout(ui.max_rect())
                    .draw(&mut (), ui);
            },
            LayoutState::new().with_style(Shadcn::themed(ShadcnStyle::Vega, Palette::new())),
        );
    harness.run_steps(2);
    assert!(
        format!("{:#?}", harness.ctx.style_of(Theme::Light)) == vega_light,
        "precondition: Vega"
    );
    harness
        .state_mut()
        .set_style(Shadcn::themed(ShadcnStyle::Lyra, Palette::new()));
    harness.run_steps(2);
    assert!(format!("{:#?}", harness.ctx.style_of(Theme::Light)) == lyra_light);
}

#[test]
fn the_core_tokens_are_the_preset_s() {
    for theme in [Theme::Light, Theme::Dark] {
        let preset = Shadcn::for_theme(theme);
        let tokens = preset.style().tokens;
        let t = preset.tokens;
        assert_eq!(
            (
                tokens.background,
                tokens.foreground,
                tokens.popover,
                tokens.primary
            ),
            (t.background, t.foreground, t.popover, t.primary),
            "{theme:?}"
        );
        assert_eq!(
            (
                tokens.muted_foreground,
                tokens.border,
                tokens.input,
                tokens.ring
            ),
            (t.muted_foreground, t.border, t.input, t.ring),
            "{theme:?}"
        );
    }
    let _: Themed<std::sync::Arc<eggshell_ui::Style>> =
        Shadcn::themed(ShadcnStyle::Vega, Palette::new());
}

#[test]
fn each_token_lands_where_egui_reads_it() {
    let mut style = eggshell_ui::Style::default();
    let mut shade = 0u8;
    let mut next = || {
        shade += 9;
        egui::Color32::from_rgb(shade, 255 - shade, shade / 2)
    };
    let t = &mut style.tokens;
    for colour in [
        &mut t.background,
        &mut t.foreground,
        &mut t.popover,
        &mut t.primary,
        &mut t.secondary,
        &mut t.secondary_foreground,
        &mut t.muted,
        &mut t.accent,
        &mut t.accent_foreground,
        &mut t.destructive,
        &mut t.border,
        &mut t.input,
        &mut t.ring,
    ] {
        *colour = next();
    }
    t.radius = 9;
    t.control_height = 41.0;
    t.text_size = 17.0;
    let t = style.tokens;
    let mut egui = egui::Style::default();
    style.egui_style(Theme::Light, &mut egui);
    let v = &egui.visuals;
    let w = &v.widgets;
    assert_eq!(
        (v.panel_fill, v.window_fill, v.window_stroke.color),
        (t.background, t.popover, t.border)
    );
    assert_eq!(
        (v.hyperlink_color, v.faint_bg_color, v.error_fg_color),
        (t.primary, t.muted, t.destructive)
    );
    assert_eq!(
        (w.noninteractive.fg_stroke.color, w.inactive.weak_bg_fill),
        (t.foreground, t.secondary)
    );
    assert_eq!(
        (w.inactive.fg_stroke.color, w.inactive.bg_stroke.color),
        (t.secondary_foreground, t.input)
    );
    assert_eq!(
        (
            w.active.weak_bg_fill,
            w.active.fg_stroke.color,
            w.active.bg_stroke.color
        ),
        (t.accent, t.accent_foreground, t.ring)
    );
    assert_eq!(v.window_corner_radius.nw, 9);
    assert_eq!(w.inactive.corner_radius.nw, 7, "rounded-md");
    assert_eq!(egui.spacing.interact_size.y, 41.0);
    assert_eq!(egui.text_styles[&egui::TextStyle::Body].size, 17.0);
}
