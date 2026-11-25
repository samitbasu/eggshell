//! What a build reads at build time: egui's local state through
//! `Builder::ctx`, and the base text, which follows egui unless overridden.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, LayoutState, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Context, FontFamily, Rect, Theme, pos2, vec2};

use crate::support;

fn context(theme: Theme) -> Context {
    let ctx = Context::default();
    ctx.set_theme(theme);
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

fn body_color(layout: &mut LayoutState, ctx: &Context) -> (Color32, f32) {
    let placed = layout
        .build(ctx, |b: &mut Builder<'_, '_, ()>| b.text(tw(), "x"))
        .layout(Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0)));
    let text = placed
        .nodes()
        .find(|node| node.path().contains("text"))
        .expect("the text node")
        .text()
        .clone();
    (text.color, text.spec.size)
}

fn override_style() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 30.0,
            line_height: 36.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::RED,
    )
}

#[test]
fn switching_egui_theme_between_builds_changes_the_default_base_text() {
    let ctx = context(Theme::Light);
    let mut layout = LayoutState::new(crate::support::shadcn());
    let (light, _) = body_color(&mut layout, &ctx);
    ctx.set_theme(Theme::Dark);
    let (dark, _) = body_color(&mut layout, &ctx);
    assert_ne!(light, dark, "the default follows the theme");
    assert_eq!(light, ctx.style_of(Theme::Light).visuals.text_color());
    assert_eq!(dark, ctx.style_of(Theme::Dark).visuals.text_color());
}

#[test]
fn setting_and_clearing_the_override_takes_effect_on_the_next_build() {
    let ctx = context(Theme::Light);
    let mut layout = LayoutState::new(crate::support::shadcn());
    let (natural, natural_size) = body_color(&mut layout, &ctx);
    assert_ne!(natural, Color32::RED, "precondition");

    layout.set_base_text(Some(override_style()));
    assert_eq!(body_color(&mut layout, &ctx), (Color32::RED, 30.0));
    ctx.set_theme(Theme::Dark);
    assert_eq!(
        body_color(&mut layout, &ctx),
        (Color32::RED, 30.0),
        "an override ignores the theme"
    );

    layout.set_base_text(Some(TextStyle {
        color: Color32::GREEN,
        ..override_style()
    }));
    assert_eq!(body_color(&mut layout, &ctx).0, Color32::GREEN, "replaced");

    layout.set_base_text(None);
    ctx.set_theme(Theme::Light);
    assert_eq!(body_color(&mut layout, &ctx), (natural, natural_size));

    let mut built = LayoutState::new(crate::support::shadcn()).with_base_text(override_style());
    assert_eq!(body_color(&mut built, &ctx).0, Color32::RED, "builder form");
    built.set_base_text(None);
    assert_eq!(body_color(&mut built, &ctx).0, natural);
}

#[derive(Default)]
struct Fold;

fn fold(b: &mut Builder<'_, '_, Fold>, _: &Fold) {
    let drawn_id = b.id_of("row").with("drawn");
    let drawn = b
        .ctx()
        .data(|data| data.get_temp::<u32>(drawn_id))
        .unwrap_or(0);
    b.div_id("row", tw().w_px(50.0).h_px(20.0), |b| {
        b.rule(tw().w_px(10.0 + drawn as f32).h_px(5.0))
    })
    .draw(move |_, ui| {
        ui.data_mut(|data| data.insert_temp(drawn_id, drawn + 1));
    });
}

#[test]
fn build_reads_the_state_a_draw_kept_through_the_builders_context() {
    let mut d: Driver<Fold> =
        support::driver(vec2(100.0, 100.0), override_style(), Fold, fold).build();
    let width = |d: &Driver<Fold>| {
        support::nodes(d)
            .iter()
            .find(|(path, _)| path.contains("rule"))
            .expect("the rule")
            .1
            .width()
    };
    let before = width(&d);
    d.frames(1);
    let after = width(&d);
    assert_eq!(
        after,
        before + 1.0,
        "each build sees the previous draw's count"
    );
    assert_eq!(d.rect(Target::suffix("/\"row\"")).size(), vec2(50.0, 20.0));
}
