//! The preset's tones (#129): Critical is the destructive tokens, Serious
//! the new serious tokens, the others the status tokens, in both themes.
//! Critical's text is `destructive_text`; the others' text is the tone.

use eggshell_ui::contract::ToneColors;
use eggshell_ui::{Style, Tone};
use eggshell_ui_shadcn::{ContrastPair, Shadcn, Tokens};
use egui::{Context, Theme};

fn expected(t: &Tokens, tone: Tone) -> ToneColors {
    let (tone, text, foreground) = match tone {
        Tone::Info => (t.info, t.info, t.info_foreground),
        Tone::Success => (t.success, t.success, t.success_foreground),
        Tone::Warning => (t.warning, t.warning, t.warning_foreground),
        Tone::Serious => (t.serious, t.serious, t.serious_foreground),
        Tone::Critical => (t.destructive, t.destructive_text, t.destructive_foreground),
    };
    ToneColors {
        tone,
        text,
        foreground,
    }
}

#[test]
fn the_preset_installs_each_tone_from_its_tokens() {
    for theme in [Theme::Light, Theme::Dark] {
        let preset = Shadcn::for_theme(theme);
        let ctx = Context::default();
        ctx.set_theme(theme);
        preset.apply(&ctx);
        let style = Style::of(&ctx);
        for tone in Tone::ALL {
            assert_eq!(
                style.tones.get(tone),
                expected(&preset.tokens, tone),
                "{theme:?} {tone:?}"
            );
        }
    }
}

#[test]
fn every_tone_has_its_own_colour() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = Shadcn::for_theme(theme).style();
        for (i, a) in Tone::ALL.into_iter().enumerate() {
            for b in Tone::ALL.into_iter().skip(i + 1) {
                assert_ne!(
                    style.tones.get(a).tone,
                    style.tones.get(b).tone,
                    "{theme:?}: {a:?} and {b:?}"
                );
            }
        }
    }
}

#[test]
fn a_tone_on_the_page_is_measured_by_its_text_colour() {
    let mut tokens = Tokens::light();
    tokens.destructive_text = tokens.background;
    assert_ne!(
        tokens.destructive, tokens.background,
        "the fill alone would pass"
    );
    for pair in [
        ContrastPair::ToneOnBackground(Tone::Critical),
        ContrastPair::ToneOnCard(Tone::Critical),
    ] {
        assert_eq!(f32::from(pair.ratio(&tokens)), 1.0, "{pair:?}");
    }
}
