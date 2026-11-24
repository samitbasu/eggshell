//! Every base colour with every theme colour gives shadcn's registry values
//! (`registry/themes.ts`, a copy of the registry's file), converted from
//! oklch to sRGB, under the preset's documented departures.

use eggshell_ui::theme::{BaseColor, Palette, ThemeColor, Tokens};
use egui::{Color32, Theme};
use std::collections::BTreeMap;

const THEMES_TS: &str = include_str!("registry/themes.ts");

type Vars = BTreeMap<String, String>;

/// The registry's themes by name: the light and dark variables of each.
fn registry() -> BTreeMap<String, (Vars, Vars)> {
    let mut themes = BTreeMap::<String, (Vars, Vars)>::new();
    let (mut name, mut dark) = (String::new(), false);
    for line in THEMES_TS.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("name: ") {
            name = rest.trim_end_matches(',').trim_matches('"').to_owned();
        } else if line == "light: {" {
            dark = false;
        } else if line == "dark: {" {
            dark = true;
        } else if let Some((key, value)) = line.split_once(": ")
            && value.starts_with("\"oklch(")
        {
            let entry = themes.entry(name.clone()).or_default();
            let vars = if dark { &mut entry.1 } else { &mut entry.0 };
            vars.insert(
                key.trim_matches('"').to_owned(),
                value.trim_end_matches(',').trim_matches('"').to_owned(),
            );
        }
    }
    themes
}

/// The registry's order of names, as the file lists them.
fn registry_names() -> Vec<String> {
    THEMES_TS
        .lines()
        .filter_map(|line| line.trim().strip_prefix("name: "))
        .map(|rest| rest.trim_end_matches(',').trim_matches('"').to_owned())
        .collect()
}

/// `oklch(L C H)` or `oklch(1 0 0 / N%)` as an 8-bit sRGB colour, clipped to
/// the gamut channel by channel.
fn srgb(oklch: &str) -> Color32 {
    let inner = oklch
        .strip_prefix("oklch(")
        .and_then(|s| s.strip_suffix(')'))
        .expect("an oklch colour");
    if let Some(percent) = inner.strip_prefix("1 0 0 / ") {
        let percent: f64 = percent.trim_end_matches('%').parse().expect("a percentage");
        return Color32::from_white_alpha((percent * 2.55).round() as u8);
    }
    let [l, c, h]: [f64; 3] = inner
        .split(' ')
        .map(|n| n.parse().expect("a number"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("three components");
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    let channel = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        let v = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (v * 255.0).round() as u8
    };
    Color32::from_rgb(
        channel(4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_),
        channel(-1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_),
        channel(-0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701 * s_),
    )
}

fn registry_name(base: BaseColor, theme_color: ThemeColor) -> String {
    match theme_color {
        ThemeColor::Base => base.name(),
        other => other.name(),
    }
    .to_lowercase()
}

/// The tokens a base colour and theme colour set, by the registry's names.
fn by_name(t: &Tokens) -> BTreeMap<&'static str, Color32> {
    BTreeMap::from([
        ("background", t.background),
        ("foreground", t.foreground),
        ("card", t.card),
        ("card-foreground", t.card_foreground),
        ("popover", t.popover),
        ("popover-foreground", t.popover_foreground),
        ("primary", t.primary),
        ("primary-foreground", t.primary_foreground),
        ("secondary", t.secondary),
        ("secondary-foreground", t.secondary_foreground),
        ("muted", t.muted),
        ("muted-foreground", t.muted_foreground),
        ("accent", t.accent),
        ("accent-foreground", t.accent_foreground),
        ("destructive", t.destructive),
        ("border", t.border),
        ("input", t.input),
        ("ring", t.ring),
        ("chart-1", t.chart_1),
        ("chart-2", t.chart_2),
        ("chart-3", t.chart_3),
        ("chart-4", t.chart_4),
        ("chart-5", t.chart_5),
    ])
}

#[test]
fn the_enums_list_the_registry_s_base_and_theme_colours() {
    let names = registry_names();
    let bases: Vec<String> = BaseColor::ALL
        .iter()
        .map(|b| b.name().to_lowercase())
        .collect();
    let colours: Vec<String> = ThemeColor::ALL
        .iter()
        .filter(|c| **c != ThemeColor::Base)
        .map(|c| c.name().to_lowercase())
        .collect();
    assert_eq!(names, [bases, colours].concat());
}

#[test]
fn every_pair_is_the_registry_s_under_the_documented_departures() {
    let registry = registry();
    let mut checked = 0;
    for base in BaseColor::ALL {
        for theme_color in ThemeColor::ALL {
            for theme in [Theme::Light, Theme::Dark] {
                let tokens = Tokens::for_theme(theme)
                    .with_palette(Palette::new().base(base).theme(theme_color));
                let pick = |(light, dark): &(Vars, Vars)| match theme {
                    Theme::Light => light.clone(),
                    Theme::Dark => dark.clone(),
                };
                let mut expected = pick(&registry[&registry_name(base, ThemeColor::Base)]);
                let coloured = pick(&registry[&registry_name(base, theme_color)]);
                let charts = ["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"];
                for key in charts.iter().chain(&["primary", "primary-foreground"]) {
                    expected.insert((*key).to_owned(), coloured[*key].clone());
                }
                let mut want: BTreeMap<&str, Color32> = by_name(&tokens)
                    .keys()
                    .map(|key| (*key, srgb(&expected[*key])))
                    .collect();
                if theme_color != ThemeColor::Base && theme == Theme::Dark {
                    let five_hundred = srgb(&coloured["chart-2"]);
                    let primary = if theme_color == ThemeColor::Violet {
                        assert_eq!(five_hundred, Color32::from_rgb(0x8e, 0x51, 0xff));
                        Color32::from_rgb(0x8e, 0x52, 0xff)
                    } else {
                        five_hundred
                    };
                    want.insert("primary", primary);
                    want.insert("primary-foreground", Color32::BLACK);
                }
                assert_eq!(by_name(&tokens), want, "{base:?} {theme_color:?} {theme:?}");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 7 * 18 * 2);
}

#[test]
fn the_charts_follow_the_theme_colour() {
    for theme in [Theme::Light, Theme::Dark] {
        let teal = Tokens::for_theme(theme).with_palette(Palette::new().theme(ThemeColor::Teal));
        assert_eq!(
            [teal.chart_1, teal.chart_5],
            [
                Color32::from_rgb(0x46, 0xec, 0xd5),
                Color32::from_rgb(0x00, 0x5f, 0x5a)
            ],
        );
        let neutral = Tokens::for_theme(theme);
        assert_eq!(
            neutral.chart_2,
            Color32::from_rgb(0x73, 0x73, 0x73),
            "grey charts"
        );
    }
}
