//! WCAG 2 contrast of the token pairs the components draw, for every base
//! colour with every theme colour in light and dark. A text pair under 4.5:1
//! fails unless the preset's `REGISTRY_MISSES` lists it; borders are
//! reported, not checked. Then the warning `Shadcn::apply` gives in a debug
//! build for other colours.

use eggshell_ui::Problem;
use eggshell_ui::testing::{assert_no_reports, assert_reported, reports};
use eggshell_ui::theme::{
    BaseColor, ContrastPair, ContrastRatio, Palette, REGISTRY_MISSES, Shadcn, ThemeColor, Tokens,
};
use egui::{Color32, Context, Theme};

fn tokens(theme: Theme, base: BaseColor, colour: ThemeColor) -> Tokens {
    Tokens::for_theme(theme).with_palette(Palette::new().base(base).theme(colour))
}

fn every_combination() -> impl Iterator<Item = (Theme, BaseColor, ThemeColor)> {
    [Theme::Light, Theme::Dark].into_iter().flat_map(|theme| {
        BaseColor::ALL.into_iter().flat_map(move |base| {
            ThemeColor::ALL
                .into_iter()
                .map(move |colour| (theme, base, colour))
        })
    })
}

#[test]
fn text_pairs_reach_4_5_to_1_or_are_listed_misses() {
    let mut used = [false; REGISTRY_MISSES.len()];
    let mut unlisted = Vec::new();
    let mut worst: Vec<(ContrastPair, f32, String)> = ContrastPair::ALL
        .iter()
        .map(|pair| (*pair, f32::INFINITY, String::new()))
        .collect();
    for (theme, base, colour) in every_combination() {
        let t = tokens(theme, base, colour);
        for (pair, lowest, at) in &mut worst {
            let (_, surface) = pair.colours(&t);
            assert_eq!(
                surface.a(),
                255,
                "{pair:?}: precondition: an opaque surface"
            );
            let ratio = f32::from(pair.ratio(&t));
            if ratio < *lowest {
                *lowest = ratio;
                *at = format!("{theme:?} {base:?} {colour:?}");
            }
            if pair.ratio(&t) >= ContrastRatio::AA_TEXT {
                continue;
            }
            let listed = REGISTRY_MISSES.iter().position(|m| {
                m.theme == theme
                    && m.pair == *pair
                    && m.base.is_none_or(|b| b == base)
                    && m.colour.is_none_or(|c| c == colour)
            });
            match listed {
                Some(i) => {
                    used[i] = true;
                    assert!(
                        (REGISTRY_MISSES[i].ratio - ratio).abs() < 0.006,
                        "{theme:?} {base:?} {colour:?} {pair:?}: {ratio:.3}, listed {}",
                        REGISTRY_MISSES[i].ratio
                    );
                }
                None => unlisted.push(format!(
                    "{theme:?} {base:?} {colour:?} {pair:?}: {ratio:.2}"
                )),
            }
        }
    }
    println!("worst ratio per pair:");
    for (pair, lowest, at) in &worst {
        println!("  {}: {lowest:.2}:1 ({at})", pair.name());
    }
    assert!(
        unlisted.is_empty(),
        "under 4.5:1 and not listed: {unlisted:#?}"
    );
    let stale: Vec<usize> = (0..REGISTRY_MISSES.len()).filter(|i| !used[*i]).collect();
    assert!(
        stale.is_empty(),
        "listed misses that no longer occur: {stale:?}"
    );
}

#[test]
fn borders_and_inputs_against_the_page_are_reported() {
    let mut worst = [
        (f32::INFINITY, String::new()),
        (f32::INFINITY, String::new()),
    ];
    for (theme, base, colour) in every_combination() {
        let t = tokens(theme, base, colour);
        for ((lowest, at), line) in worst.iter_mut().zip([t.border, t.input]) {
            let ratio = f32::from(ContrastRatio::between(
                t.background.blend(line),
                t.background,
            ));
            if ratio < *lowest {
                *lowest = ratio;
                *at = format!("{theme:?} {base:?} {colour:?}");
            }
        }
    }
    let [(border, at_border), (input, at_input)] = worst;
    println!("border on background, lowest: {border:.2}:1 ({at_border}); 3:1 is not checked");
    println!("input on background, lowest: {input:.2}:1 ({at_input}); 3:1 is not checked");
}

fn applied(preset: &Shadcn) -> Context {
    let ctx = Context::default();
    preset.apply(&ctx);
    preset.apply(&ctx);
    ctx
}

/// Light tokens whose primary button label is grey on grey.
fn low_contrast() -> Shadcn {
    let mut preset = Shadcn::light();
    preset.tokens.primary = Color32::from_rgb(0x33, 0x33, 0x33);
    preset.tokens.primary_foreground = Color32::from_rgb(0x55, 0x55, 0x55);
    preset
}

#[test]
fn the_default_themes_and_every_palette_warn_of_nothing() {
    for (theme, base, colour) in every_combination() {
        let preset = Shadcn::for_theme(theme).with_palette(Palette::new().base(base).theme(colour));
        assert_no_reports(&applied(&preset));
    }
    assert_no_reports(&applied(&Shadcn::light()));
    assert_no_reports(&applied(&Shadcn::dark()));
}

#[test]
fn a_low_contrast_token_set_is_reported_once_in_a_debug_build() {
    let preset = low_contrast();
    let ratio = ContrastPair::PrimaryForegroundOnPrimary.ratio(&preset.tokens);
    assert!(ratio < ContrastRatio::AA_TEXT, "precondition: {ratio}");
    let ctx = applied(&preset);
    if cfg!(debug_assertions) {
        let report = assert_reported(&ctx, Problem::LowContrast);
        assert_eq!(
            report.count, 2,
            "one report, counted again on the second apply"
        );
        assert_eq!(reports(&ctx).len(), 1);
        assert_eq!(
            report.message,
            format!("primary-foreground on primary is {ratio} in Light (AA text needs 4.50:1)")
        );
        assert_eq!(preset.tokens.primary, Color32::from_rgb(0x33, 0x33, 0x33));
    } else {
        assert_no_reports(&ctx);
    }
}

#[test]
fn an_accepted_pair_is_not_reported() {
    let preset = low_contrast().accept_contrast(ContrastPair::PrimaryForegroundOnPrimary);
    assert_no_reports(&applied(&preset));
}
