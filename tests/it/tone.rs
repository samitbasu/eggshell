//! The tone table (#129): one entry per `Tone`, found by the tone.

use eggshell_ui::Style;
use eggshell_ui::Tone;
use eggshell_ui::contract::ToneColors;
use eggshell_ui_shadcn::ContrastRatio;
use egui::Color32;

#[test]
fn each_tone_finds_its_own_entry() {
    let mut style = Style::from_egui(&egui::Style::default());
    for (index, tone) in Tone::ALL.into_iter().enumerate() {
        let marker = ToneColors {
            tone: Color32::from_rgb(index as u8 + 1, 0, 0),
            foreground: Color32::from_rgb(0, index as u8 + 1, 0),
        };
        style.tones.set(tone, marker);
    }
    for (index, tone) in Tone::ALL.into_iter().enumerate() {
        assert_eq!(
            style.tones.get(tone).tone,
            Color32::from_rgb(index as u8 + 1, 0, 0),
            "{tone:?}"
        );
        assert_eq!(
            style.tones.get(tone).foreground,
            Color32::from_rgb(0, index as u8 + 1, 0),
            "{tone:?}"
        );
    }
}

#[test]
fn the_egui_style_takes_egui_s_warning_colour() {
    for visuals in [egui::Visuals::light(), egui::Visuals::dark()] {
        let egui = egui::Style {
            visuals: visuals.clone(),
            ..egui::Style::default()
        };
        let tones = Style::from_egui(&egui).tones;
        assert_eq!(tones.get(Tone::Warning).tone, visuals.warn_fg_color);
    }
}

/// The egui-derived tones' pairs under AA, which the user accepted: egui's
/// own warning colour, kept as egui has it.
const EGUI_MISSES: [(egui::Theme, Tone, Pair, f32); 1] =
    [(egui::Theme::Light, Tone::Warning, Pair::TextOnPanel, 2.79)];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Pair {
    TextOnPanel,
    ForegroundOnFill,
}

#[test]
fn the_egui_style_s_tones_pass_aa_but_for_the_listed_misses() {
    let mut found = Vec::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let visuals = theme.default_visuals();
        let egui = egui::Style {
            visuals: visuals.clone(),
            ..egui::Style::default()
        };
        let tones = Style::from_egui(&egui).tones;
        for tone in Tone::ALL {
            let colours = tones.get(tone);
            for (pair, text, surface) in [
                (Pair::TextOnPanel, colours.tone, visuals.panel_fill),
                (Pair::ForegroundOnFill, colours.foreground, colours.tone),
            ] {
                let ratio = f32::from(ContrastRatio::between(text, surface));
                if ratio < 4.5 {
                    found.push((theme, tone, pair, (ratio * 100.0).round() / 100.0));
                }
            }
        }
    }
    assert_eq!(found, EGUI_MISSES, "the misses, each with its ratio");
}
