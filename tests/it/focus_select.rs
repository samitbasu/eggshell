//! Focus the application asks for can select a field's text, so typing
//! replaces it, while a click or Tab places the caret as ever (#292).

use eggshell_ui::owned::Step;
use eggshell_ui::prim::text_field::Select;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Event, FontFamily, Key, PointerButton, vec2};

use crate::support::{self, focused_node};

const NAME: &str = r#"/"Page"/"name""#;
const OTHER: &str = r#"/"Page"/"other""#;
const DEFAULT: &str = "Item 1";

#[derive(Default)]
struct App {
    name: String,
    token: u32,
    shown: bool,
    select: Option<Select>,
    count: f64,
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

/// The name field takes focus when the token changes, selecting as
/// `app.select` says, or as `focus_on_change` does with none.
fn on_change(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.text_field("other", "")
            .named("Other")
            .classes(tw().w_px(160.0))
            .on_change(|_: &mut App, _| {});
        let name = s
            .text_field("name", app.name.clone())
            .named("Name")
            .classes(tw().w_px(160.0));
        let name = match app.select {
            Some(select) => name.focus_on_change_selecting(app.token, select),
            None => name.focus_on_change(app.token),
        };
        name.on_change(|app: &mut App, text| app.name = text);
    });
}

/// The name field takes focus, selecting all, when it appears.
fn on_appear(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.text_field("other", "")
            .named("Other")
            .classes(tw().w_px(160.0))
            .on_change(|_: &mut App, _| {});
        if app.shown {
            s.text_field("name", app.name.clone())
                .named("Name")
                .classes(tw().w_px(160.0))
                .focus_on_appear_selecting(Select::All)
                .on_change(|app: &mut App, text| app.name = text);
        }
    });
}

/// A stepper that takes focus when it appears, selecting as `app.select`
/// says, or as `focus_on_appear` does with none.
fn stepper_appears(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        if app.shown {
            let count = s.stepper("count", app.count, 0.0..=10.0).named("Count");
            let count = match app.select {
                Some(select) => count.focus_on_appear_selecting(select),
                None => count.focus_on_appear(),
            };
            count.on_change(|app: &mut App, value| app.count = value);
        }
    });
}

fn harness(view: fn(&mut Builder<'_, '_, App>, &App), select: Option<Select>) -> Driver<App> {
    let app = App {
        name: DEFAULT.to_owned(),
        select,
        ..App::default()
    };
    let mut d = support::driver(vec2(400.0, 200.0), base_text(), app, view).build();
    d.frames(4);
    d
}

fn type_text(d: &mut Driver<App>, text: &str) {
    assert_eq!(
        focused_node(d).as_deref(),
        Some(NAME),
        "precondition: the name field has focus"
    );
    d.event(Event::Text(text.into())).frames(3);
}

fn app_focuses(d: &mut Driver<App>) {
    d.app_mut().token += 1;
    d.frames(4);
}

#[test]
fn app_focus_selecting_all_lets_typing_replace_the_text() {
    let mut d = harness(on_change, Some(Select::All));
    app_focuses(&mut d);
    type_text(&mut d, "Box");
    assert_eq!(d.app().name, "Box");
}

#[test]
fn app_focus_selecting_none_appends_as_focus_on_change_does() {
    for select in [Some(Select::None), None] {
        let mut d = harness(on_change, select);
        app_focuses(&mut d);
        type_text(&mut d, "x");
        assert_eq!(d.app().name, "Item 1x", "{select:?}");
    }
}

#[test]
fn appearing_selecting_all_lets_typing_replace_the_text() {
    let mut d = harness(on_appear, None);
    d.app_mut().shown = true;
    d.frames(4);
    type_text(&mut d, "Box");
    assert_eq!(d.app().name, "Box");
}

#[test]
fn app_focus_selects_all_again_in_a_field_that_already_has_focus() {
    let mut d = harness(on_change, Some(Select::All));
    app_focuses(&mut d);
    type_text(&mut d, "Box");
    app_focuses(&mut d);
    type_text(&mut d, "Crate");
    assert_eq!(d.app().name, "Crate");
}

#[test]
fn a_click_into_the_field_places_the_caret_and_selects_nothing() {
    let mut d = harness(on_change, Some(Select::All));
    d.click(Target::suffix(NAME)).frames(6);
    type_text(&mut d, "x");
    let name = &d.app().name;
    assert_eq!(name.chars().count(), DEFAULT.len() + 1, "{name:?}");
}

#[test]
fn tab_into_the_field_selects_nothing() {
    let mut d = harness(on_change, Some(Select::All));
    d.click(Target::suffix(OTHER)).frames(6);
    assert_eq!(
        focused_node(&d).as_deref(),
        Some(OTHER),
        "precondition: Tab starts from the other field"
    );
    d.press(Key::Tab).frames(5);
    type_text(&mut d, "x");
    let name = &d.app().name;
    assert_eq!(name.chars().count(), DEFAULT.len() + 1, "{name:?}");
}

#[test]
fn a_click_after_app_focus_selected_all_places_the_caret() {
    let mut d = harness(on_change, Some(Select::All));
    app_focuses(&mut d);
    d.click(Target::suffix(OTHER)).frames(6);
    d.click(Target::suffix(NAME)).frames(6);
    type_text(&mut d, "x");
    let name = &d.app().name;
    assert_eq!(name.chars().count(), DEFAULT.len() + 1, "{name:?}");
}

#[test]
fn a_stepper_appearing_selecting_all_lets_typing_replace_its_value() {
    let mut d = harness(stepper_appears, Some(Select::All));
    d.app_mut().count = 3.0;
    d.app_mut().shown = true;
    d.frames(4);
    d.event(Event::Text("7".into())).frames(3);
    assert_eq!(d.app().count, 7.0);
}

#[test]
fn a_stepper_appearing_selecting_none_keeps_its_value_and_takes_the_typing() {
    for select in [Some(Select::None), None] {
        let mut d = harness(stepper_appears, select);
        d.app_mut().count = 3.0;
        d.app_mut().shown = true;
        d.frames(4);
        d.event(Event::Text("7".into())).frames(3);
        assert_eq!(
            d.app().count,
            10.0,
            "{select:?}: the 7 joins the 3, and 37 clamps to 10"
        );
    }
}

/// A stepper in halves, so its value has two words.
fn halves(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.stepper("count", app.count, 0.0..=10.0)
            .step(Step::new(0.5))
            .named("Count")
            .on_change(|app: &mut App, value| app.count = value);
    });
}

#[test]
fn a_double_click_in_a_stepper_field_selects_the_whole_value_not_the_word_under_it() {
    let mut d = harness(halves, None);
    d.app_mut().count = 2.5;
    d.frames(4);
    let fraction = d
        .painted()
        .expect("2.5")
        .part_rect("5")
        .expect("precondition: \"5\" is on one row");
    click_twice(
        &mut d,
        fraction.center(),
        std::time::Duration::from_millis(200),
    );
    d.event(Event::Text("7".into())).frames(3);
    assert_eq!(d.app().count, 7.0);
}

/// Clicks twice at `at`, one event a frame as a browser sends them, the
/// releases `apart` apart.
fn click_twice(d: &mut Driver<App>, at: egui::Pos2, apart: std::time::Duration) {
    let frame = d.ctx().input(|input| input.predicted_dt);
    let frames = (apart.as_secs_f32() / frame).round() as usize;
    assert!(
        frames >= 2,
        "precondition: a press and a release fit in {apart:?}"
    );
    press_and_release(d, at);
    d.frames(frames - 2);
    press_and_release(d, at);
    d.frames(6);
}

fn press_and_release(d: &mut Driver<App>, at: egui::Pos2) {
    for pressed in [true, false] {
        d.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}
