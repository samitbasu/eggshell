//! A style is data: switching the shadcn style of a running context changes
//! control sizes, radii and text size with no change to any layout code, and
//! every example still fits under both.

use eggshell_ui::contract::{Size, Style};
use eggshell_ui::widget::TextEdit;
use eggshell_ui::{Builder, LayoutState, Placed, tw};
use eggshell_ui_shadcn::{Shadcn, ShadcnStyle, ThemeColor, Tokens};
use egui::{Context, Rect, Theme, pos2};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/button.rs"]
mod button;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/chat_shell/main.rs"]
mod chat_shell;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/controls_gallery.rs"]
mod controls_gallery;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/form.rs"]
mod form;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/signup_form.rs"]
mod signup_form;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/todo.rs"]
mod todo;

const AREA: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(1280.0, 900.0),
};

fn context(theme: Theme, style: ShadcnStyle) -> Context {
    let ctx = Context::default();
    ctx.set_theme(theme);
    ctx.run_ui(Default::default(), |_| {})
        .drop_without_applying_deltas();
    common::set_style(&ctx, style);
    common::use_preset(&ctx);
    ctx
}

fn place<A>(ctx: &Context, build: impl for<'f> FnOnce(&mut Builder<'_, 'f, A>)) -> Placed<A> {
    LayoutState::new()
        .with_base_text(common::base_text_in(ctx))
        .build(ctx, build)
        .layout(AREA)
}

fn controls(b: &mut Builder<'_, '_, ()>) {
    b.div(tw().flex().flex_col().gap(2.0).w_px(400.0), |b| {
        b.button("button", "Save").add();
        b.checkbox("checkbox", "Agree", false)
            .on_change(|_: &mut (), _| {});
        b.switch("switch", "Alerts", false)
            .on_change(|_: &mut (), _| {});
        b.select("select", 0, [(0, "One"), (1, "Two")])
            .on_change(|_: &mut (), _| {});
        b.slider("slider", 0.5, 0.0..=1.0)
            .on_change(|_: &mut (), _| {});
        b.widget(
            "field",
            tw().w_full(),
            TextEdit::singleline("name"),
            |_, _, _| {},
        );
    });
}

fn height_of<A>(placed: &Placed<A>, path: &str) -> f32 {
    placed
        .nodes()
        .find(|node| node.path() == path)
        .unwrap_or_else(|| panic!("no node {path}: {}", placed.dump()))
        .rect()
        .height()
}

#[test]
fn switching_the_style_at_run_time_resizes_the_controls_per_the_style() {
    for theme in [Theme::Light, Theme::Dark] {
        let ctx = context(theme, ShadcnStyle::Vega);
        let vega = place(&ctx, controls);
        let vega_style = Style::of(&ctx);
        let vega_tokens = Tokens::for_theme(theme).in_style(ShadcnStyle::Vega);

        common::set_style(&ctx, ShadcnStyle::Lyra);
        common::use_preset(&ctx);
        let lyra = place(&ctx, controls);
        let lyra_style = Style::of(&ctx);
        let lyra_tokens = Tokens::for_theme(theme).in_style(ShadcnStyle::Lyra);

        assert_ne!(
            vega_tokens.control_height, lyra_tokens.control_height,
            "precondition"
        );
        for (name, path) in [
            ("button", "/\"button\""),
            ("select", "/\"select\""),
            ("field", "/\"field\""),
        ] {
            for (placed, tokens, style) in
                [(&vega, &vega_tokens, "vega"), (&lyra, &lyra_tokens, "lyra")]
            {
                let height = height_of(placed, path);
                assert!(
                    (height - tokens.control_height).abs() < 0.01,
                    "{theme:?} {style} {name}: {height}"
                );
            }
        }

        assert_eq!(vega_style.text.size, 14.0);
        assert_eq!(lyra_style.text.size, 12.0);
        let radius = |style: &Style| style.button.sizes.get(Size::DEFAULT).radius;
        assert_eq!(radius(&vega_style), vega_tokens.radius_md());
        assert_eq!(radius(&lyra_style), 0, "Lyra is square");
        assert!(radius(&vega_style) > 0, "precondition: Vega is rounded");
        assert_eq!(vega_style.focus.width, 3.0);
        assert_eq!(lyra_style.focus.width, 1.0);
        assert_eq!(vega_style.input.height, 36.0);
        assert_eq!(lyra_style.input.height, 32.0);
        assert_eq!(lyra_style.input.radius, 0);

        common::set_style(&ctx, ShadcnStyle::Vega);
        common::use_preset(&ctx);
        assert_eq!(*Style::of(&ctx), *vega_style, "and back again");
    }
}

#[test]
fn style_and_theme_colour_are_chosen_independently() {
    let neutral = |style| Shadcn::light().with_style(style).style();
    let blue = |style| {
        Shadcn::light()
            .with_style(style)
            .theme_color(ThemeColor::Blue)
            .style()
    };
    for style in ShadcnStyle::ALL {
        assert_ne!(
            neutral(style),
            blue(style),
            "{style:?} takes a theme colour"
        );
    }
    assert_ne!(neutral(ShadcnStyle::Vega), neutral(ShadcnStyle::Lyra));
    assert_eq!(
        blue(ShadcnStyle::Lyra).button.sizes.get(Size::DEFAULT),
        neutral(ShadcnStyle::Lyra).button.sizes.get(Size::DEFAULT),
        "the theme colour does not change sizes"
    );
}

fn overflow<A>(what: &str, placed: &Placed<A>) -> Vec<String> {
    placed
        .overflowing()
        .into_iter()
        .map(|path| format!("{what}{path}"))
        .collect()
}

fn overflows_in(style: ShadcnStyle, theme: Theme) -> Vec<String> {
    let ctx = context(theme, style);
    let mut found = Vec::new();
    found.extend(overflow(
        "button",
        &place(&ctx, |b| button::layout(b, &button::State::default())),
    ));
    found.extend(overflow(
        "todo",
        &place(&ctx, |b| todo::layout(b, &todo::State::default())),
    ));
    found.extend(overflow(
        "form",
        &place(&ctx, |b| form::layout(b, &form::State::default())),
    ));
    found.extend(overflow(
        "signup_form",
        &place(&ctx, |b| {
            signup_form::layout(b, &signup_form::State::default())
        }),
    ));
    for section in [
        controls_gallery::Section::Cells,
        controls_gallery::Section::Buttons,
        controls_gallery::Section::Choices,
        controls_gallery::Section::Values,
    ] {
        let state = controls_gallery::State {
            section,
            ..Default::default()
        };
        found.extend(overflow(
            "controls_gallery",
            &place(&ctx, |b| controls_gallery::layout(b, &state)),
        ));
    }
    let app = chat_shell::state::App::default();
    found.extend(overflow(
        "chat_shell",
        &place(&ctx, |b| chat_shell::view::layout(b, &app)),
    ));
    found
}

#[test]
fn every_example_lays_out_without_overflow_under_both_styles() {
    for theme in [Theme::Light, Theme::Dark] {
        for style in ShadcnStyle::ALL {
            let overflowing = overflows_in(style, theme);
            assert!(
                overflowing.is_empty(),
                "{theme:?} {style:?}: {overflowing:?}"
            );
        }
    }
}

#[test]
fn a_harness_switched_to_lyra_draws_lyra() {
    let mut harness = crate::support::harness_prepared(
        controls_gallery::WINDOW,
        common::base_text_styled(Theme::Light, ShadcnStyle::Lyra),
        controls_gallery::State::default(),
        controls_gallery::layout,
        controls_gallery::prepare,
    );
    common::set_style(&harness.ctx, ShadcnStyle::Lyra);
    harness.run_steps(4);
    let style = Style::of(&harness.ctx);
    assert_eq!(style.button.sizes.get(Size::DEFAULT).height, 32.0);
}
