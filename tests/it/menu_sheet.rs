//! A menu's parts are drawn from the style's sheet, so a rule on a part
//! restyles every menu that builds it.

use eggshell_ui::owned::{Mark, MenuPart};
use eggshell_ui::testing::Target;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, vec2};

use crate::support;

struct App;

fn view(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().p(8.0), |b| {
        b.dropdown_menu("file", "File").items(|m| {
            m.item("copy", "Copy").on_click(|_: &mut App| {});
            m.checkbox_item("grid", "Grid", Mark::Off)
                .on_change(|_: &mut App, _| {});
        });
    });
}

const RED: Color32 = Color32::from_rgb(200, 10, 10);
const ITEM: &str = r#"/("file", Menu)/Box/"copy""#;
const CHECKBOX: &str = r#"/("file", Menu)/Box/"grid""#;

#[test]
fn a_rule_on_the_item_part_restyles_the_items_of_that_kind_only() {
    let text = TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    );
    let mut d = support::driver(vec2(400.0, 300.0), text, App, view)
        .style(support::shadcn_with(|style| {
            style.sheet.as_app().rule(MenuPart::Item, |t, _| t.bg(RED));
        }))
        .build();
    d.frames(4);
    d.click(Target::suffix(r#"/"file""#)).frames(5);
    assert_eq!(support::fill_of(&d, ITEM), Some(RED));
    assert_ne!(
        support::fill_of(&d, CHECKBOX),
        Some(RED),
        "the checkbox item is a part of its own"
    );
}
