//! The preset's tones (#129): each is its status tokens, in both themes.

use eggshell_ui::Tone;
use eggshell_ui::contract::ToneColors;
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::Theme;

fn expected(t: &Tokens, tone: Tone) -> ToneColors {
    let (tone, foreground) = match tone {
        Tone::Info => (t.info, t.info_foreground),
        Tone::Success => (t.success, t.success_foreground),
        Tone::Warning => (t.warning, t.warning_foreground),
    };
    ToneColors { tone, foreground }
}

#[test]
fn the_preset_s_style_takes_each_tone_from_its_tokens() {
    for theme in [Theme::Light, Theme::Dark] {
        let preset = Shadcn::for_theme(theme);
        let style = preset.style();
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
