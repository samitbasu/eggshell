//! The extras' form section and labelled rows (#370), through the real draw
//! path and AccessKit, in the shadcn preset the rig applies; and the
//! preset's invariant that a control's track is as visible on a section as
//! on the page.

use eggshell_ui::contract::Size;
use eggshell_ui::owned::{
    Enablement, Mark, progress::ProgressPart, slider::SliderPart, switch::SwitchPart,
};
use eggshell_ui::prelude::*;
use eggshell_ui::theme::{
    BaseColor, ContrastRatio, Palette, Shadcn, ShadcnStyle, ThemeColor, Tokens,
};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Color32, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, Seen as _, rect_of};

#[derive(Default)]
struct App {
    gain: f64,
}

fn base() -> TextStyle {
    let tokens = Tokens::for_theme(Theme::Light);
    TextStyle::new(tokens.text_role(), tokens.foreground)
}

/// ```html
/// <div class="flex flex-col gap-6 p-4 w-[360px]">
///   <fieldset><legend>Audio</legend>
///     <div class="rounded-xl border bg-card py-2 divide-y">
///       <div role="group" aria-label="Gain">Gain <button>Reset</button></div>
///       <div role="group" aria-label="Sample rate">Sample rate <span>48 kHz</span></div>
///     </div>
///   </fieldset>
///   <div role="group" aria-label="Outside">Outside <span>value</span></div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(6.0).p(4.0).w_px(360.0),
        |b| {
            b.form_section("audio")
                .title("Audio")
                .divided()
                .children(|b| {
                    b.labeled("Gain", |b| {
                        b.button("reset", format!("Reset {:.0}", app.gain))
                            .on_click(|app: &mut App| app.gain = 0.0);
                    });
                    b.labeled_text("Sample rate", "48 kHz");
                });
            b.labeled_text("Outside", "value");
        },
    );
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(420.0, 300.0), base(), App::default(), view);
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(4);
    harness
}

const BOX: &str = r#"/"Page"/"audio"/"Box""#;
const GAIN: &str = r#"/"Page"/"audio"/"Box"/("eggshell-labeled", 0)"#;
const RATE: &str = r#"/"Page"/"audio"/"Box"/("eggshell-labeled", 1)"#;
const OUTSIDE: &str = r#"/"Page"/("eggshell-labeled", 0)"#;

#[test]
fn a_text_row_is_as_tall_as_a_control_row() {
    let harness = harness();
    let slider = rect_of(&harness, &format!("{GAIN}/\"Value\""));
    let text = rect_of(&harness, &format!("{RATE}/\"Value\""));
    assert!(
        slider.height() > text.height() + 4.0,
        "precondition: the values differ in height: {slider:?} {text:?}"
    );
    let (gain, rate) = (rect_of(&harness, GAIN), rect_of(&harness, RATE));
    assert!(
        (gain.height() - rate.height()).abs() < 0.5,
        "{gain:?} {rate:?}"
    );
}

#[test]
fn a_row_in_a_section_is_inset_and_one_outside_is_not() {
    let harness = harness();
    let style = harness.view_shell().style().form.clone();
    let frame = rect_of(&harness, BOX);
    let row = rect_of(&harness, RATE);
    let value = rect_of(&harness, &format!("{RATE}/\"Value\""));
    // The frame is shadcn's card, edged by a ring, which takes no room.
    assert!(
        (row.left() - frame.left()).abs() < 0.5 && (frame.right() - row.right()).abs() < 0.5,
        "the row spans the box, so the rule above it does: {row:?} in {frame:?}"
    );
    assert!(
        (row.right() - value.right() - style.row_padding_x).abs() < 0.5,
        "the value is inset: {value:?} in {row:?}"
    );
    let outside = rect_of(&harness, OUTSIDE);
    let outside_value = rect_of(&harness, &format!("{OUTSIDE}/\"Value\""));
    assert!(
        (outside.right() - outside_value.right()).abs() < 0.5,
        "no inset outside a section: {outside_value:?} in {outside:?}"
    );
}

#[test]
fn the_section_and_each_row_are_groups_named_by_their_title_and_label() {
    let harness = harness();
    let groups: Vec<_> = harness
        .query_all_by_role(Role::Group)
        .filter_map(|node| node.accesskit_node().label())
        .collect();
    assert_eq!(
        groups,
        ["Audio", "Gain", "Sample rate", "Outside"],
        "in reading order"
    );
    let rate = harness.get_by_role_and_label(Role::Group, "Sample rate");
    assert_eq!(
        rate.accesskit_node().description().map(|d| d.to_string()),
        Some("48 kHz".to_owned()),
        "a text value is read with its row: core puts no plain text in the tree"
    );
}

/// The fill a track draws, over `surface`.
fn over(surface: Color32, track: Color32) -> Color32 {
    surface.blend(track)
}

#[test]
fn every_track_shows_on_a_section_at_least_as_well_as_on_the_page() {
    let mut worse = Vec::new();
    for theme in [Theme::Light, Theme::Dark] {
        for look in ShadcnStyle::ALL {
            for base in BaseColor::ALL {
                for colour in ThemeColor::ALL {
                    let preset = Shadcn::for_theme(theme)
                        .with_style(look)
                        .with_palette(Palette::new().base(base).theme(colour));
                    let style = preset.style();
                    let section = style.form.fill;
                    let page = preset.tokens.background;
                    assert_ne!(
                        section,
                        Color32::TRANSPARENT,
                        "precondition: the section has a fill"
                    );
                    let tracks = [
                        (
                            "slider",
                            support::part_at(&style, SliderPart::Track, &[])
                                .paint_classes()
                                .background
                                .expect("a slider's track is filled"),
                        ),
                        ("segmented", style.tokens.input),
                        (
                            "switch off",
                            support::part_at(
                                &style,
                                SwitchPart::Root,
                                &SwitchPart::keys(Size::DEFAULT, Mark::Off, Enablement::Enabled),
                            )
                            .paint_classes()
                            .background
                            .expect("a switch's track is filled"),
                        ),
                        (
                            "progress",
                            support::part_at(&style, ProgressPart::Root, &[])
                                .paint_classes()
                                .background
                                .expect("a progress bar's track is filled"),
                        ),
                    ];
                    for (name, track) in tracks {
                        let on_page = f32::from(ContrastRatio::between(over(page, track), page));
                        let on_section =
                            f32::from(ContrastRatio::between(over(section, track), section));
                        if on_section + 0.005 < on_page {
                            worse.push(format!(
                                "{theme:?} {} {base:?} {colour:?} {name}: {on_section:.2} on the section, {on_page:.2} on the page",
                                look.name()
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(worse.is_empty(), "{worse:#?}");
}

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/form_sections.rs"]
mod form_sections_example;

#[test]
fn the_example_passes_kittests_accessibility_check() {
    let mut harness = support::harness(
        vec2(460.0, 560.0),
        base(),
        form_sections_example::Settings::default(),
        form_sections_example::layout,
    );
    harness.state_mut().companion = support::Companion::Alone;
    harness.run();
}

/// A choice field fills a form's line, as shadcn's `Field` does, and is as
/// wide as its content inline, beside a button in a row.
#[test]
fn a_choice_field_fills_a_form_and_is_its_contents_width_inline() {
    fn view(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div_id(
            "Page",
            tw().flex().flex_col().gap(6.0).p(4.0).w_px(360.0),
            |b| {
                b.div_id(
                    "Row",
                    tw().flex().flex_wrap().items_center().gap(2.0),
                    |b| {
                        b.button("go", "Go").add();
                        b.checkbox("inline", "Inline", false)
                            .on_change(|_: &mut App, _| {});
                    },
                );
                b.form_section("audio").children(|b| {
                    b.checkbox("formed", "Formed", false)
                        .on_change(|_: &mut App, _| {});
                });
            },
        );
    }
    let mut harness = support::harness(vec2(420.0, 300.0), base(), App::default(), view);
    harness.state_mut().companion = support::Companion::Alone;
    harness.run_steps(4);
    let field = |key: &str| {
        harness
            .placed()
            .into_iter()
            .find(|(path, _, _)| path.contains(&format!("\"{key}\"")) && path.ends_with("<div>"))
            .map(|(_, rect, _)| rect)
            .expect("the field")
    };
    let (go, inline) = (rect_of(&harness, r#"/"Page"/"Row"/"go""#), field("inline"));
    assert_eq!(
        inline.center().y.round(),
        go.center().y.round(),
        "on the button's line"
    );
    assert!(inline.width() < 120.0, "as wide as its content: {inline:?}");
    let section = rect_of(&harness, r#"/"Page"/"audio"/"Box""#);
    let formed = field("formed");
    assert!(
        (formed.width() - section.width()).abs() < 0.5,
        "the form's line: {formed:?} in {section:?}"
    );
}

/// A section's frame is shadcn's card (card.tsx: `ring-1 rounded-xl
/// bg-card`), drawn from the style's sheet; as recorded, it keeps the
/// section's own vertical padding, not the card's `py-6`.
#[test]
fn a_sections_frame_is_shadcns_card_with_the_sections_padding() {
    let harness = harness();
    let style = harness.view_shell().style().form.clone();
    let frame = rect_of(&harness, BOX);
    let drawn = support::box_at(&harness, frame);
    assert_eq!(drawn.stroke.width, 0.0, "a ring, not a border");
    assert_eq!(
        drawn.corner_radius,
        egui::CornerRadius::same(14),
        "rounded-xl"
    );
    assert_eq!(drawn.fill, Tokens::for_theme(Theme::Light).card, "bg-card");
    let first = rect_of(&harness, GAIN);
    assert_eq!(
        first.top() - frame.top(),
        style.padding_y,
        "the section's padding"
    );
}
