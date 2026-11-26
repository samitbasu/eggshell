//! An open listbox overlay announces itself on the node it is anchored to:
//! expanded, the listbox it controls and the item the arrows reached, the
//! `aria-activedescendant` of the web. Through the real path: build, draw,
//! keys, and the accessibility tree the frame hands the platform.

use eggshell_ui::overlay::{Anchor, Focus, Nav, Wrap};
use eggshell_ui::owned::choice::Choice;
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::{HasPopup, Role as AccessRole};
use egui::{Color32, FontFamily, Key, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path};

const FIELD: &str = r#"/"Page"/"city""#;
const TYPING: &str = "<widget>";
const CITIES: [&str; 3] = ["Paris", "Parma", "Porto"];

#[derive(Default)]
struct App {
    city: String,
    pick: usize,
    dismissed: bool,
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

/// A text field with its suggestions in a listbox overlay anchored to it.
fn suggest(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().p(2.0), |s| {
        s.text_field("city", app.city.clone())
            .named("City")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.city = text);
        if s.has_focus_within("city") && !app.dismissed {
            let field = s.id_of("city");
            s.overlay("suggest")
                .anchor(Anchor::Node(field))
                .focus(Focus::Stay)
                .nav(Nav::Listbox(Wrap::Around))
                .role(Role::ListBox)
                .named("Suggestions")
                .on_dismiss(|app: &mut App| app.dismissed = true)
                .children(|s| {
                    s.div_id("Box", tw().flex().flex_col().bg(Color32::WHITE), |s| {
                        for city in CITIES {
                            s.div_id(city, tw().h_px(24.0).sense_click(), |s| s.text(tw(), city))
                                .role(Role::ListBoxOption)
                                .named(city)
                                .on_click(move |app: &mut App| app.city = city.into());
                        }
                    });
                });
        }
    });
}

fn suggesting() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(400.0, 300.0), base_text(), App::default(), suggest);
    harness.run_steps(4);
    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(TYPING),
        "precondition: the field's text edit has the focus"
    );
    assert!(
        harness
            .query_by_role_and_label(AccessRole::ListBox, "Suggestions")
            .is_some(),
        "precondition: the suggestions are open"
    );
    harness
}

fn down(harness: &mut Harness<'_, Rig<App>>) {
    harness.key_press(Key::ArrowDown);
    harness.run_steps(3);
}

/// The label of the active descendant of the node labelled `label`.
fn active(harness: &Harness<'_, Rig<App>>, role: AccessRole, label: &str) -> Option<String> {
    harness
        .get_by_role_and_label(role, label)
        .accesskit_node()
        .active_descendant()
        .and_then(|node| node.label())
}

fn controlled(harness: &Harness<'_, Rig<App>>, role: AccessRole, label: &str) -> Vec<String> {
    harness
        .get_by_role_and_label(role, label)
        .accesskit_node()
        .controls()
        .filter_map(|node| node.label())
        .collect()
}

#[test]
fn the_field_announces_the_suggestion_the_arrows_reached() {
    let mut harness = suggesting();
    assert_eq!(active(&harness, AccessRole::TextInput, "City"), None);
    down(&mut harness);
    assert_eq!(
        active(&harness, AccessRole::TextInput, "City").as_deref(),
        Some("Paris")
    );
    down(&mut harness);
    assert_eq!(
        active(&harness, AccessRole::TextInput, "City").as_deref(),
        Some("Parma")
    );
}

#[test]
fn the_field_is_expanded_and_controls_its_suggestions() {
    let harness = suggesting();
    let field = harness.get_by_role_and_label(AccessRole::TextInput, "City");
    let data = field.accesskit_node().data().clone();
    assert_eq!(
        (data.is_expanded(), data.has_popup()),
        (Some(true), Some(HasPopup::Listbox))
    );
    assert_eq!(
        controlled(&harness, AccessRole::TextInput, "City"),
        ["Suggestions"]
    );
}

#[test]
fn a_closed_field_controls_nothing() {
    let mut harness = suggesting();
    down(&mut harness);
    harness.state_mut().app.dismissed = true;
    harness.run_steps(40);
    assert!(
        harness
            .query_by_role_and_label(AccessRole::ListBox, "Suggestions")
            .is_none(),
        "precondition: the suggestions closed"
    );
    assert!(controlled(&harness, AccessRole::TextInput, "City").is_empty());
    assert_eq!(active(&harness, AccessRole::TextInput, "City"), None);
}

fn select(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().p(2.0), |s| {
        s.select(
            "pick",
            app.pick,
            CITIES
                .into_iter()
                .enumerate()
                .map(|(at, city)| Choice::new(at, city)),
        )
        .named("Pick")
        .classes(tw().w_px(160.0))
        .on_change(|app: &mut App, value| app.pick = value);
    });
}

#[test]
fn the_select_trigger_announces_its_highlight_and_its_list() {
    let mut harness = support::harness(vec2(400.0, 300.0), base_text(), App::default(), select);
    harness.run_steps(4);
    harness
        .get_by_role_and_label(AccessRole::ComboBox, "Pick")
        .click();
    harness.run_steps(6);
    assert!(
        harness
            .query_by_role_and_label(AccessRole::ListBoxOption, "Parma")
            .is_some(),
        "precondition: the list is open"
    );
    assert_eq!(
        active(&harness, AccessRole::ComboBox, "Pick").as_deref(),
        Some("Paris"),
        "the chosen option is highlighted as the list opens"
    );
    down(&mut harness);
    assert_eq!(
        active(&harness, AccessRole::ComboBox, "Pick").as_deref(),
        Some("Parma")
    );
    let lists: Vec<AccessRole> = harness
        .get_by_role_and_label(AccessRole::ComboBox, "Pick")
        .accesskit_node()
        .controls()
        .map(|node| node.role())
        .collect();
    assert_eq!(lists, [AccessRole::ListBox]);
}
