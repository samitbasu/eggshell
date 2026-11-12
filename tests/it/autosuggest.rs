//! A text field with a suggestion list under it: an overlay that leaves focus
//! on the field (`Focus::Stay`), through the real path.

use eggshell_ui::overlay::{Anchor, Focus, Nav};
use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Role, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, focused_path, rect_of};

const FIELD: &str = r#"/"Page"/"city""#;
/// What `focused_path` reports for the text edit inside the field.
const TYPING: &str = "<widget>";
const FIRST: &str = r#"/"Page"/"suggest"/"Box"/Option(0)"#;
const CITIES: [&str; 2] = ["Paris", "Parma"];

#[derive(Default)]
struct App {
    city: String,
    zip: String,
    log: Vec<String>,
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

#[derive(Hash, Debug)]
struct Option(usize);

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.text_field("city", app.city.clone())
            .named("City")
            .classes(tw().w_px(160.0))
            .when_blurred(|app: &mut App| app.log.push("blur".into()))
            .on_change(|app: &mut App, text| app.city = text);
        s.text_field("zip", app.zip.clone())
            .named("Zip")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.zip = text);
        if s.has_focus_within("city") {
            let field = s.id_of("city");
            s.overlay("suggest")
                .anchor(Anchor::Node(field))
                .focus(Focus::Stay)
                .nav(Nav::Listbox)
                .role(Role::ListBox)
                .named("Suggestions")
                .children(|s| {
                    s.div_id(
                        "Box",
                        tw().flex().flex_col().bg(Color32::WHITE).border(1.0),
                        |s| {
                            for (index, city) in CITIES.into_iter().enumerate() {
                                s.div_id(Option(index), tw().h_px(24.0).sense_click(), |s| {
                                    s.text(tw(), city)
                                })
                                .role(Role::ListBoxOption)
                                .named(city)
                                .on_click(move |app: &mut App| {
                                    app.city = city.into();
                                    app.log.push(format!("pick {city}"));
                                });
                            }
                        },
                    );
                });
        }
    });
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(400.0, 300.0), base_text(), App::default(), view);
    harness.run_steps(4);
    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    harness
}

#[test]
fn pressing_a_suggestion_does_not_take_focus_from_the_field() {
    let mut harness = harness();
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "precondition: the field has focus"
    );
    assert!(
        rect_of(&harness, FIRST).height() > 0.0,
        "precondition: the suggestions are showing"
    );

    support::press(&mut harness, FIRST);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "the field keeps focus while the suggestion is held down"
    );
    assert!(harness.state().app.log.is_empty());

    support::release(&mut harness, FIRST);
    assert_eq!(
        harness.state().app.log,
        ["pick Paris"],
        "the pick applies, and the field's blur handler never ran before it"
    );
    assert_eq!(harness.state().app.city, "Paris");
    assert_eq!(focused_path(&harness).as_deref(), Some(TYPING));
}

#[test]
fn a_press_outside_the_suggestions_still_takes_focus_from_the_field() {
    let mut harness = harness();
    click_node(&mut harness, r#"/"Page"/"zip""#);
    harness.run_steps(6);
    assert_eq!(harness.state().app.log, ["blur"]);
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some(r#"/"Page"/"zip""#),
        "focus went where the press was"
    );
}
