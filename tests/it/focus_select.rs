//! Focus the application asks for can select a field's text, so typing
//! replaces it, while a click or Tab places the caret as ever (#292).

use eggshell_ui::owned::number::Step;
use eggshell_ui::owned::text_field::Select;
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Event, FontFamily, Key, vec2};

use crate::support::{self, click_twice, focused_node};

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
        let name = name.focus_on_change(app.token);
        let name = match app.select {
            Some(select) => name.select_on_focus(select),
            None => name,
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
                .focus_on_appear()
                .select_on_focus(Select::All)
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
            let count = count.focus_on_appear();
            let count = match app.select {
                Some(select) => count.select_on_focus(select),
                None => count,
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
    commit(&mut d);
    assert_eq!(d.app().count, 7.0);
}

/// Enter: the stepper commits what was typed.
fn commit(d: &mut Driver<App>) {
    d.press(egui::Key::Enter).frames(3);
}

#[test]
fn a_stepper_appearing_selecting_none_keeps_its_value_and_takes_the_typing() {
    for select in [Some(Select::None), None] {
        let mut d = harness(stepper_appears, select);
        d.app_mut().count = 3.0;
        d.app_mut().shown = true;
        d.frames(4);
        d.event(Event::Text("7".into())).frames(3);
        commit(&mut d);
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
    commit(&mut d);
    assert_eq!(d.app().count, 7.0);
}

/// How the application asks a field for focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Path {
    Now,
    Appear,
    Change,
    Open,
}

const PATHS: [Path; 4] = [Path::Now, Path::Appear, Path::Change, Path::Open];

/// What takes focus: a text field or a stepper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Text,
    Stepper,
}

struct Asked {
    path: Path,
    field: Field,
    shown: bool,
    token: u32,
    name: String,
    count: f64,
}

/// The field, under `"field"`, asking for focus by `app.path` and selecting
/// all of its text when it takes focus.
fn asked_field(b: &mut Builder<'_, '_, Asked>, app: &Asked) {
    fn ask<C: eggshell_ui::owned::options::Interactive>(
        chain: C,
        app: &Asked,
        appear: impl FnOnce(C) -> C,
        change: impl FnOnce(C, u32) -> C,
    ) -> C {
        match app.path {
            Path::Now => chain.when(app.token == 1, |chain| chain.focus_now()),
            Path::Appear => appear(chain),
            Path::Change => change(chain, app.token),
            Path::Open => chain.focus_on_open(),
        }
    }
    match app.field {
        Field::Text => {
            let field = s_text(b, app);
            ask(
                field,
                app,
                |c| c.focus_on_appear(),
                |c, t| c.focus_on_change(t),
            )
            .select_on_focus(Select::All)
            .on_change(|app: &mut Asked, text| app.name = text);
        }
        Field::Stepper => {
            let field = b.stepper("field", app.count, 0.0..=10.0).named("Field");
            ask(field, app, |c| c.focus_on_appear(), |c, _| c)
                .select_on_focus(Select::All)
                .on_change(|app: &mut Asked, value| app.count = value);
        }
    }
}

fn s_text<'b, 'f>(
    b: &'b mut Builder<'_, 'f, Asked>,
    app: &Asked,
) -> eggshell_ui::owned::text_field::TextFieldChain<'b, 'f, Asked> {
    b.text_field("field", app.name.clone())
        .named("Field")
        .classes(tw().w_px(160.0))
}

fn asked(b: &mut Builder<'_, '_, Asked>, app: &Asked) {
    b.div(tw().flex().flex_col().gap(2.0).p(2.0), |b| {
        b.text_field("other", "")
            .named("Other")
            .classes(tw().w_px(160.0))
            .on_change(|_: &mut Asked, _| {});
        if !app.shown {
            return;
        }
        if app.path != Path::Open {
            asked_field(b, app);
            return;
        }
        b.div_id("anchor", tw().size_px(10.0), |_| {});
        let anchor = b.id_of("anchor");
        b.overlay("pop")
            .anchor(eggshell_ui::overlay::Anchor::Node(anchor))
            .children(|s| {
                s.div(tw().flex().flex_col().gap(4.0).p(4.0), |s| {
                    s.button("first", "First").add();
                    asked_field(s, app);
                });
            });
    });
}

/// Asks `field` for focus by `path`, as an application does after its
/// first frames, and types `typed` into it. `focus_now` asks for one
/// frame's worth, as the application drops it once the field has focus.
fn asked_and_typed(path: Path, field: Field, typed: &str) -> Driver<Asked> {
    let app = Asked {
        path,
        field,
        shown: path != Path::Appear && path != Path::Open,
        token: 0,
        name: DEFAULT.to_owned(),
        count: 3.0,
    };
    let mut d = support::driver(vec2(400.0, 300.0), base_text(), app, asked).build();
    d.frames(4);
    match path {
        Path::Now | Path::Change => d.app_mut().token += 1,
        Path::Appear | Path::Open => d.app_mut().shown = true,
    }
    d.frames(6);
    if path == Path::Now {
        d.app_mut().token += 1;
        d.frames(2);
    }
    assert!(
        focused_node(&d).is_some_and(|path| path.contains(r#""field""#)),
        "precondition: {path:?} {field:?} focuses the field, not {:?}",
        focused_node(&d)
    );
    d.event(Event::Text(typed.into())).frames(3);
    d
}

#[test]
fn select_on_focus_all_selects_the_whole_text_on_every_focus_path() {
    for path in PATHS {
        let d = asked_and_typed(path, Field::Text, "Box");
        assert_eq!(d.app().name, "Box", "{path:?}: typing replaced the text");
    }
}

#[test]
fn select_on_focus_all_selects_a_stepper_s_whole_value_on_every_focus_path() {
    for path in [Path::Now, Path::Appear, Path::Open] {
        let mut d = asked_and_typed(path, Field::Stepper, "7");
        d.press(Key::Enter).frames(3);
        assert_eq!(d.app().count, 7.0, "{path:?}: typing replaced the 3");
    }
}
