//! The tone table (#129): one entry per `Tone`, found by the tone.

use eggshell_ui::Style;
use eggshell_ui::Tone;
use eggshell_ui::contract::ToneColors;
use egui::Color32;

#[test]
fn each_tone_finds_its_own_entry() {
    let mut style = Style::default();
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
