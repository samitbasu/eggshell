//! The gap between a trigger and its dropdown menu or select list: one style
//! value, overridable per call.

use eggshell_ui::overlay::Distance;
use eggshell_ui::owned::Mark;
use eggshell_ui::{Builder, Style, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, rect_of};

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

fn offset_12(ctx: &egui::Context, _: &App) {
    let mut style = (*Style::of(ctx)).clone();
    style.menu.offset = Distance::new(12.0);
    style.install(ctx);
}

fn harness(app: App, prepare: fn(&egui::Context, &App)) -> Harness<'static, Rig<App>> {
    let mut harness =
        support::harness_prepared(vec2(400.0, 300.0), base_text(), app, view, prepare);
    harness.run_steps(4);
    harness
}

fn gap_below(harness: &mut Harness<'_, Rig<App>>, trigger: &str, list: &str) -> f32 {
    click_node(harness, trigger);
    harness.run_steps(5);
    let trigger = rect_of(harness, trigger);
    let list = rect_of(harness, list);
    assert!(list.min.y > trigger.min.y, "precondition: it opened below");
    list.min.y - trigger.max.y
}

const FILE: &str = r#"/"file""#;
const MENU: &str = r#"/("file", Menu)/Box"#;
const PICK: &str = r#"/"pick""#;
const LIST: &str = r#"/("pick", List)/Box"#;

#[test]
fn by_default_both_open_four_points_below_their_trigger() {
    let mut harness = harness(App::default(), |_, _| {});
    assert_eq!(gap_below(&mut harness, FILE, MENU).round(), 4.0);
    let mut harness = self::harness(App::default(), |_, _| {});
    assert_eq!(gap_below(&mut harness, PICK, LIST).round(), 4.0);
}

#[test]
fn the_style_offset_sets_the_gap_of_the_dropdown_and_the_select_list() {
    let mut harness = harness(App::default(), offset_12);
    assert_eq!(gap_below(&mut harness, FILE, MENU).round(), 12.0);
    let mut harness = self::harness(App::default(), offset_12);
    assert_eq!(gap_below(&mut harness, PICK, LIST).round(), 12.0);
}

#[test]
fn a_per_call_offset_wins_over_the_style() {
    let app = || App {
        per_call: Some(Distance::new(20.0)),
    };
    let mut harness = harness(app(), offset_12);
    assert_eq!(gap_below(&mut harness, FILE, MENU).round(), 20.0);
    let mut harness = self::harness(app(), offset_12);
    assert_eq!(gap_below(&mut harness, PICK, LIST).round(), 20.0);
}
