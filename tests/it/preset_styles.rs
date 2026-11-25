//! The shadcn styles as parameter sets: Vega is today's look, Lyra is
//! `style-lyra.css`, and both fill the same contract.

use eggshell_ui::Tw;
use eggshell_ui::contract::{Size, Style, TextScale, TextSize, TextStep, Variant};
use eggshell_ui::owned::{Enablement, button, input, segmented, slider, tabs};
use eggshell_ui::sheet::Key;
use eggshell_ui::theme::{Palette, Shadcn, ShadcnStyle, ThemeColor, Tokens, variant};
use egui::{Theme, vec2};

fn style_of(theme: Theme, style: ShadcnStyle) -> Style {
    Shadcn::for_theme(theme).with_style(style).style()
}

/// The classes `style`'s sheet gives a button of `variant` and `size`.
fn button(style: &Style, variant: Variant, size: Size) -> Tw {
    style.sheet.resolve(
        button::Part::Root,
        &[Key::Variant(variant), Key::Size(size)],
        &style.tokens,
    )
}

/// A button's height and corner radius.
fn box_of(style: &Style, size: Size) -> (Option<f32>, u8) {
    let classes = button(style, Variant::DEFAULT, size);
    (
        classes
            .style_at(style.tokens.spacing)
            .size
            .height
            .into_option(),
        classes.paint_classes().radius.nw,
    )
}

/// The height, padding and radius the sheet gives a text field of `size`.
fn field_of(style: &Style, size: Size) -> (f32, egui::Vec2, u8) {
    let classes = style
        .sheet
        .resolve(input::Part::Root, &[Key::Size(size)], &style.tokens);
    let layout = classes.style_at(style.tokens.spacing);
    (
        layout.size.height.into_raw().value(),
        vec2(
            layout.padding.left.into_raw().value(),
            layout.padding.top.into_raw().value(),
        ),
        classes.paint_classes().radius.nw,
    )
}

#[test]
fn vega_is_the_default_and_the_tokens_of_before() {
    assert_eq!(ShadcnStyle::default(), ShadcnStyle::Vega);
    let t = Tokens::light();
    assert_eq!(t.style, ShadcnStyle::Vega);
    assert_eq!((t.radius.lg(), t.control_height()), (10.0, 36.0));
    assert_eq!(t.control_text, TextStep::Sm, "text-sm");
    assert_eq!(
        t.points_vec(t.input_padding),
        vec2(10.0, 4.0),
        "px-2.5 py-1, style-vega.css:678"
    );
    assert_eq!(t.ring_width, 3.0);
}

#[test]
fn lyra_values_are_style_lyra_css() {
    for theme in [Theme::Light, Theme::Dark] {
        let t = Tokens::for_theme(theme).in_style(ShadcnStyle::Lyra);
        assert_eq!(t.radius_md(), 0, "rounded-none");
        assert_eq!(t.radius_sm(), 0);
        assert_eq!(t.control_text, TextStep::Xs, "text-xs");
        assert_eq!(t.control_height(), 32.0, "h-8");
        assert_eq!(t.button_padding.x, 2.5, "px-2.5");
        assert_eq!(t.control_gap, 1.5, "gap-1.5");
        assert_eq!(t.ring_width, 1.0, "ring-1");
        assert_eq!((t.slider_track, t.slider_thumb), (1.0, 3.0), "h-1, size-3");

        let style = style_of(theme, ShadcnStyle::Lyra);
        assert_eq!(
            box_of(&style, Size::DEFAULT),
            (Some(32.0), 0),
            "h-8 rounded-none"
        );
        assert_eq!(style.tokens.focus().width, 1.0);
        assert_eq!(style.text.size, 12.0);
        assert_eq!(style.tokens.checkbox_radius, 0);
        assert_eq!(style.slider.sizes.get(Size::DEFAULT).radius, 0);
        assert_eq!(
            field_of(&style, Size::DEFAULT),
            (32.0, vec2(10.0, 4.0), 0),
            "h-8 px-2.5 py-1 rounded-none"
        );
        assert!(
            button(&style, variant::OUTLINE, Size::DEFAULT)
                .paint_classes()
                .shadow
                .is_none(),
            "Lyra's outline has no shadow-xs"
        );
    }
}

#[test]
fn both_styles_tint_the_destructive_button() {
    let t = Tokens::light();
    for style in ShadcnStyle::ALL {
        let classes = button(
            &style_of(Theme::Light, style),
            variant::DESTRUCTIVE,
            Size::DEFAULT,
        );
        assert_eq!(
            classes.paint_classes().background,
            Some(t.destructive.gamma_multiply(0.1)),
            "{style:?}: bg-destructive/10"
        );
    }
}

#[test]
fn every_style_names_the_same_variants_and_sizes() {
    let keys = |style: &Style| -> Vec<Key> { style.sheet.keys::<button::Part>().collect() };
    let vega = style_of(Theme::Light, ShadcnStyle::Vega);
    let lyra = style_of(Theme::Light, ShadcnStyle::Lyra);
    assert_eq!(keys(&vega), keys(&lyra));
    assert!(keys(&vega).contains(&Key::Variant(variant::LINK)), "link");
    assert!(
        keys(&vega).contains(&Key::Size(eggshell_ui::theme::size::ICON_LG)),
        "icon-lg"
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
                box_of(&s, Size::DEFAULT),
                field_of(&s, Size::DEFAULT).0,
                s.tokens.focus().width,
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
            let ring = preset.style().tokens.focus().color;
            assert_eq!(
                ring,
                neutral.style().tokens.focus().color,
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
            let track = s
                .sheet
                .resolve(slider::Part::Track, &[], &s.tokens)
                .paint_classes()
                .background;
            assert_eq!(track, Some(tokens.input), "{theme:?} {style:?}");
            let list = s
                .sheet
                .resolve(
                    tabs::Part::List,
                    &tabs::Part::list_keys(
                        Size::DEFAULT,
                        segmented::Rows::One,
                        Enablement::Enabled,
                    ),
                    &s.tokens,
                )
                .paint_classes()
                .background;
            assert_eq!(list, Some(tokens.input), "{theme:?} {style:?}");
            assert_ne!(track, Some(tokens.muted), "{theme:?} {style:?}");
            assert_ne!(list, Some(tokens.muted), "{theme:?} {style:?}");
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
            assert_eq!(half.toggle, core.toggle, "{at}");
            assert_eq!(half.menu, core.menu, "{at}");
            let fonts = &half.fonts;
            assert!(preset.tokens.font.is_some(), "precondition: {at} has fonts");
            assert!(!fonts.is_empty(), "{at}");
            assert!(fonts.iter().all(|font| !font.families.is_empty()), "{at}");
        }
    }
}

#[test]
fn a_presets_control_text_is_the_step_its_generated_classes_name() {
    let at = |size, line_height| TextSize { size, line_height };
    let distinct = TextScale {
        xs: at(11.0, 15.0),
        sm: at(13.0, 17.0),
        base: at(17.0, 23.0),
        lg: at(19.0, 27.0),
    };
    for shadcn in [ShadcnStyle::Vega, ShadcnStyle::Lyra] {
        let mut preset = Shadcn::light().with_style(shadcn);
        preset.tokens.text = distinct;
        let style = preset.style();
        let want = preset.tokens.control_text_size().size;
        let page = eggshell_ui::TextStyle::new(
            eggshell_ui::TextSpec {
                size: 99.0,
                ..preset.tokens.text_role()
            },
            egui::Color32::BLACK,
        );
        let input = style.sheet.resolve(
            input::Part::Root,
            &[Key::Size(Size::DEFAULT)],
            &style.tokens,
        );
        let button = button(&style, Variant::DEFAULT, Size::DEFAULT);
        for (part, classes) in [("input", input), ("button", button)] {
            assert_eq!(
                classes.text_classes().over(&page).spec.size,
                want,
                "{shadcn:?} {part}: control_text {:?}",
                preset.tokens.control_text
            );
        }
    }
}
