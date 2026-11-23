//! The combobox, an extension crate's widget, drawn from the style's sheet:
//! its own look is the defaults of its parts (`Builder::part_over`), the
//! preset's generated look replaces them, and an App rule restyles them.

use eggshell_ui::Style;
use eggshell_ui::contract::Themed;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use eggshell_ui_extras::command::{ComboboxPart, CommandExt};
use eggshell_ui_shadcn::Tokens;
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Rect, vec2};

use crate::support;

#[derive(Default)]
struct App {
    framework: Option<u8>,
}

const FRAMEWORKS: [(u8, &str); 3] = [(1, "Next.js"), (2, "SvelteKit"), (3, "Nuxt")];

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().p(4.0).w_px(260.0), |b| {
        b.combobox(
            "framework",
            app.framework,
            FRAMEWORKS.map(|(value, label)| (value, label.to_owned())),
        )
        .named("Framework")
        .on_change(|app: &mut App, value| app.framework = Some(value));
    });
}

fn opened(style: Option<Style>) -> Driver<App> {
    let text = TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    );
    let mut d = support::driver(vec2(320.0, 320.0), text, App::default(), view).build();
    if let Some(style) = style {
        d.set_style(Themed::both(std::sync::Arc::new(style)));
    }
    d.frames(3);
    d.click(Target::suffix("Chevron)")).frames(8);
    d.hover(egui::pos2(310.0, 310.0)).frames(4);
    d
}

/// The box of the open list.
fn content(d: &Driver<App>) -> Rect {
    support::nodes(d)
        .into_iter()
        .find(|(path, _)| path.contains("Popover") && path.ends_with("/Box"))
        .map(|(_, rect)| rect)
        .expect("the list's box")
}

#[test]
fn the_presets_look_replaces_the_combobox_defaults() {
    // The widget's own box is the menu's, bordered; shadcn's is ringed.
    let bare = opened(Some(Style::default()));
    let boxed = content(&bare);
    assert_eq!(
        support::box_at(&bare, boxed).stroke.width,
        1.0,
        "the defaults' border"
    );
    let preset = opened(None);
    let boxed = content(&preset);
    assert_eq!(
        support::box_at(&preset, boxed).stroke.width,
        0.0,
        "shadcn's content has no border: the defaults are cut"
    );
}

#[test]
fn the_highlighted_option_takes_shadcns_accent() {
    let d = opened(None);
    let t = Tokens::for_theme(d.ctx().theme());
    let list = content(&d);
    let filled: Vec<Color32> = support::shapes(&d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(r) if r.fill == t.accent && list.contains_rect(r.rect) => Some(r.fill),
            _ => None,
        })
        .collect();
    assert_eq!(
        filled.len(),
        1,
        "one highlighted option, data-highlighted:bg-accent"
    );
}

#[test]
fn an_app_rule_restyles_an_option() {
    let mut style = eggshell_ui_shadcn::Shadcn::light().style();
    style
        .sheet
        .as_app()
        .rule(ComboboxPart::Item, |t, _| t.bg(Color32::RED));
    let d = opened(Some(style));
    let red = support::shapes(&d)
        .into_iter()
        .filter(|shape| matches!(shape, Shape::Rect(r) if r.fill == Color32::RED))
        .count();
    assert_eq!(
        red,
        FRAMEWORKS.len() - 1,
        "every option but the highlighted one"
    );
}
