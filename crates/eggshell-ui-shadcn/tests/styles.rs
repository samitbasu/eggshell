//! The shadcn styles as parameter sets: Vega is today's look, Lyra is
//! `style-lyra.css`, and both fill the same contract.

use eggshell_ui::contract::{Fonts, Size, Style, Variant};
use eggshell_ui_shadcn::{Palette, Shadcn, ShadcnStyle, ThemeColor, Tokens, variant};
use egui::{Color32, Theme, vec2};

fn style_of(theme: Theme, style: ShadcnStyle) -> Style {
    Shadcn::for_theme(theme).with_style(style).style()
}

#[test]
fn vega_is_the_default_and_the_tokens_of_before() {
    assert_eq!(ShadcnStyle::default(), ShadcnStyle::Vega);
    let t = Tokens::light();
    assert_eq!(t.style, ShadcnStyle::Vega);
    assert_eq!((t.radius, t.text_size, t.control_height), (10, 14.0, 36.0));
    assert_eq!(t.input_padding, vec2(12.0, 4.0));
    assert_eq!(t.ring_width, 3.0);
}

#[test]
fn lyra_values_are_style_lyra_css() {
    for theme in [Theme::Light, Theme::Dark] {
        let t = Tokens::for_theme(theme).in_style(ShadcnStyle::Lyra);
        assert_eq!(t.radius_md(), 0, "rounded-none");
        assert_eq!(t.radius_sm(), 0);
        assert_eq!((t.text_size, t.text_line_height), (12.0, 16.0), "text-xs");
        assert_eq!(t.control_height, 32.0, "h-8");
        assert_eq!(t.button_padding.x, 10.0, "px-2.5");
        assert_eq!(t.control_gap, 6.0, "gap-1.5");
        assert_eq!(t.ring_width, 1.0, "ring-1");
        assert_eq!((t.slider_track, t.slider_thumb), (4.0, 12.0), "h-1, size-3");

        let style = style_of(theme, ShadcnStyle::Lyra);
        let button = style.button.sizes.get(Size::DEFAULT);
        assert_eq!((button.height, button.radius, button.gap), (32.0, 0, 6.0));
        assert_eq!(style.tokens.focus.width, 1.0);
        assert_eq!(style.text.size, 12.0);
        assert_eq!(style.checkbox.sizes.get(Size::DEFAULT).radius, 0);
        assert_eq!(style.slider.sizes.get(Size::DEFAULT).radius, 0);
        assert_eq!(style.input().sizes.get(Size::DEFAULT).height, 32.0);
        assert_eq!(style.input().sizes.get(Size::DEFAULT).radius, 0);
        assert_eq!(
            style.input().sizes.get(Size::DEFAULT).padding,
            vec2(10.0, 4.0)
        );
        assert!(
            style
                .button
                .variants
                .get(variant::OUTLINE)
                .paint
                .shadow
                .is_none()
        );
    }
}

#[test]
fn lyra_tints_the_destructive_button_and_vega_fills_it() {
    let fill = |style, theme| {
        style_of(theme, style)
            .button
            .variants
            .get(variant::DESTRUCTIVE)
            .paint
            .states
            .rest
    };
    let t = Tokens::light();
    assert_eq!(
        fill(ShadcnStyle::Lyra, Theme::Light).fill,
        t.destructive.gamma_multiply(0.1),
        "bg-destructive/10"
    );
    assert_eq!(fill(ShadcnStyle::Lyra, Theme::Light).text, t.destructive);
    assert_eq!(fill(ShadcnStyle::Vega, Theme::Light).fill, t.destructive);
    assert_eq!(fill(ShadcnStyle::Vega, Theme::Light).text, Color32::WHITE);
}

#[test]
fn every_style_defines_the_same_variants_and_sizes() {
    let keys = |style: &Style| -> Vec<Variant> { style.button.variants.keys().collect() };
    let vega = style_of(Theme::Light, ShadcnStyle::Vega);
    let lyra = style_of(Theme::Light, ShadcnStyle::Lyra);
    assert_eq!(keys(&vega), keys(&lyra));
    assert_eq!(
        vega.button.sizes.keys().collect::<Vec<_>>(),
        lyra.button.sizes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn style_light_dark_and_theme_colour_vary_independently() {
    for style in ShadcnStyle::ALL {
        let sizes = |theme, color| {
            let preset = Shadcn::for_theme(theme)
                .with_style(style)
                .with_palette(Palette::new().theme(color));
            let s = preset.style();
            (
                *s.button.sizes.get(Size::DEFAULT),
                s.input().sizes.get(Size::DEFAULT).height,
                s.tokens.focus.width,
            )
        };
        let reference = sizes(Theme::Light, ThemeColor::Base);
        for theme in [Theme::Light, Theme::Dark] {
            for color in ThemeColor::ALL {
                assert_eq!(
                    sizes(theme, color),
                    reference,
                    "{style:?} {theme:?} {color:?}"
                );
            }
        }
    }
}

#[test]
fn vega_with_a_style_applied_is_the_plain_preset() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(
            Shadcn::for_theme(theme).with_style(ShadcnStyle::Vega),
            Shadcn::for_theme(theme)
        );
    }
}

#[test]
fn the_ring_is_the_neutral_ring_under_every_theme_colour() {
    for theme in [Theme::Light, Theme::Dark] {
        let neutral = Shadcn::for_theme(theme);
        for color in ThemeColor::ALL {
            let preset = Shadcn::for_theme(theme).with_palette(Palette::new().theme(color));
            assert_eq!(
                preset.tokens.ring, neutral.tokens.ring,
                "{theme:?} {color:?}: the registry's coloured themes leave `ring` to the base colour"
            );
            let ring = preset.style().tokens.focus.color;
            assert_eq!(
                ring,
                neutral.style().tokens.focus.color,
                "{theme:?} {color:?}"
            );
        }
    }
}

#[test]
fn slider_and_segmented_tracks_read_on_a_muted_panel() {
    for theme in [Theme::Light, Theme::Dark] {
        for style in [ShadcnStyle::Vega, ShadcnStyle::Lyra] {
            let tokens = Tokens::for_theme(theme).in_style(style);
            let s = style_of(theme, style);
            assert_ne!(
                tokens.input, tokens.muted,
                "{theme:?} {style:?}: precondition"
            );
            assert_eq!(
                s.slider.track.rest.fill, tokens.input,
                "{theme:?} {style:?}"
            );
            assert_eq!(s.segmented.track_fill, tokens.input, "{theme:?} {style:?}");
            assert_ne!(
                s.slider.track.rest.fill, tokens.muted,
                "{theme:?} {style:?}"
            );
            assert_ne!(s.segmented.track_fill, tokens.muted, "{theme:?} {style:?}");
        }
    }
}

#[test]
fn each_themed_half_is_the_preset_s_style_with_its_fonts() {
    for style in ShadcnStyle::ALL {
        let pair = Shadcn::themed(style, Palette::new());
        for (theme, half) in [(Theme::Light, &pair.light), (Theme::Dark, &pair.dark)] {
            let preset = Shadcn::for_theme(theme)
                .with_style(style)
                .with_palette(Palette::new());
            let core = preset.style();
            let at = format!("{theme:?} {style:?}");
            assert_eq!(half.tokens, core.tokens, "{at}");
            assert_eq!(half.text, core.text, "{at}");
            assert_eq!(half.button, core.button, "{at}");
            assert_eq!(half.segmented, core.segmented, "{at}");
            assert_eq!(half.menu, core.menu, "{at}");
            let fonts = half.extension::<Fonts>();
            assert!(preset.tokens.font.is_some(), "precondition: {at} has fonts");
            assert!(!fonts.0.is_empty(), "{at}");
            assert!(fonts.0.iter().all(|font| !font.families.is_empty()), "{at}");
        }
    }
}
