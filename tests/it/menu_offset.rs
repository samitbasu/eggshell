//! The gap between a trigger and its dropdown menu or select list: one style
//! value, overridable per call.

use eggshell_ui::overlay::Distance;
use eggshell_ui::owned::Mark;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Style, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, vec2};

use crate::support;

#[derive(Default)]
struct App {
    per_call: Option<Distance>,
}

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().gap(8.0).p(8.0), |b| {
        let mut dropdown = b.dropdown_menu("file", "File");
        if let Some(offset) = app.per_call {
            dropdown = dropdown.offset(offset);
        }
        dropdown.items(|m| {
            m.item("copy", "Copy").on_click(|_: &mut App| {});
            m.checkbox_item("grid", "Grid", Mark::Off)
                .on_change(|_: &mut App, _| {});
        });
        let mut select = b
            .select("pick", 0u8, [(0u8, "Alpha"), (1, "Beta")])
            .named("Pick");
        if let Some(offset) = app.per_call {
            select = select.offset(offset);
        }
        select.on_change(|_: &mut App, _| {});
    });
}

fn offset_12(style: &mut Style) {
    let mut menu = style.menu.clone();
    menu.offset = Distance::new(12.0);
    style.menu = menu;
}

fn harness(app: App, edit: fn(&mut Style)) -> Driver<App> {
    let mut d = support::driver(vec2(400.0, 300.0), base_text(), app, view)
        .style(&eggshell_ui::SharedStyle::new(support::shadcn_with(edit)))
        .build();
    d.frames(4);
    d
}

fn gap_below(d: &mut Driver<App>, trigger: &str, list: &str) -> f32 {
    d.click(Target::suffix(trigger)).frames(5);
    let trigger = d.rect(Target::suffix(trigger));
    let list = d.rect(Target::suffix(list));
    assert!(list.min.y > trigger.min.y, "precondition: it opened below");
    list.min.y - trigger.max.y
}

const FILE: &str = r#"/"file""#;
const MENU: &str = r#"/("file", Menu)/Box"#;
const PICK: &str = r#"/"pick""#;
const LIST: &str = r#"/("pick", List)/Box"#;

#[test]
fn by_default_both_open_four_points_below_their_trigger() {
    let mut d = harness(App::default(), |_| {});
    assert_eq!(gap_below(&mut d, FILE, MENU).round(), 4.0);
    let mut d = self::harness(App::default(), |_| {});
    assert_eq!(gap_below(&mut d, PICK, LIST).round(), 4.0);
}

#[test]
fn the_style_offset_sets_the_gap_of_the_dropdown_and_the_select_list() {
    let mut d = harness(App::default(), offset_12);
    assert_eq!(gap_below(&mut d, FILE, MENU).round(), 12.0);
    let mut d = self::harness(App::default(), offset_12);
    assert_eq!(gap_below(&mut d, PICK, LIST).round(), 12.0);
}

#[test]
fn a_per_call_offset_wins_over_the_style() {
    let app = || App {
        per_call: Some(Distance::new(20.0)),
    };
    let mut d = harness(app(), offset_12);
    assert_eq!(gap_below(&mut d, FILE, MENU).round(), 20.0);
    let mut d = self::harness(app(), offset_12);
    assert_eq!(gap_below(&mut d, PICK, LIST).round(), 20.0);
}
