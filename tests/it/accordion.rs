//! The extras' accordion through the real path: its three modes, each
//! asked through clicks on real headers, and what they announce.

use std::collections::BTreeSet;

use eggshell_ui::overlay::Shown;
use eggshell_ui::owned::Enablement;
use eggshell_ui::{Builder, tw};
use eggshell_ui_extras::accordion::{AccordionExt, Items, Mode, Multiple, Single};
use egui::accesskit::Role as AccessRole;
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig};

#[path = "../../examples/common/mod.rs"]
mod common;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Faq {
    Access,
    Motion,
    Text,
}

const ITEMS: [(Faq, &str); 3] = [
    (Faq::Access, "Is it accessible?"),
    (Faq::Motion, "Is it animated?"),
    (Faq::Text, "Does it stretch the text?"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Collapsible,
    Always,
    Multiple,
}

struct App {
    kind: Kind,
    one: Option<Faq>,
    always: Faq,
    many: BTreeSet<Faq>,
    disabled: Option<Faq>,
    asked: u32,
}

impl App {
    fn of(kind: Kind) -> Self {
        Self {
            kind,
            one: None,
            always: Faq::Access,
            many: BTreeSet::new(),
            disabled: None,
            asked: 0,
        }
    }
}

fn items<'f, M: Mode<Faq, Change = C>, C: Clone + 'static>(
    acc: &mut Items<'_, '_, 'f, App, M, Faq, C>,
    app: &'f App,
) {
    for (faq, title) in ITEMS {
        let enablement = Enablement::from(app.disabled != Some(faq));
        acc.item(faq, title)
            .enablement(enablement)
            .children(move |b| b.button(("inside", faq), "Inside").on_click(|_| {}));
    }
}

/// ```html
/// <Accordion type="single|multiple" collapsible?> three items </Accordion>
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div(tw().flex().flex_col().w_px(320.0), |b| match app.kind {
        Kind::Collapsible => b
            .accordion("faq", Single::collapsible(app.one))
            .on_change(|app: &mut App, open| {
                app.asked += 1;
                app.one = open;
            })
            .items(move |acc| items(acc, app)),
        Kind::Always => b
            .accordion("faq", Single::always(app.always))
            .on_change(|app: &mut App, open| {
                app.asked += 1;
                app.always = open;
            })
            .items(move |acc| items(acc, app)),
        Kind::Multiple => b
            .accordion(
                "faq",
                Multiple::new(move |faq: Faq| Shown::from(app.many.contains(&faq))),
            )
            .on_change(|app: &mut App, (faq, shown): (Faq, Shown)| {
                app.asked += 1;
                if shown.is_open() {
                    app.many.insert(faq);
                } else {
                    app.many.remove(&faq);
                }
            })
            .items(move |acc| items(acc, app)),
    });
}

fn harness_of(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness_stepped(
        vec2(400.0, 500.0),
        0.01,
        common::base_text(egui::Theme::Light),
        app,
        view,
    );
    harness.run_steps(4);
    harness
}

fn click(harness: &mut Harness<'_, Rig<App>>, faq: Faq) {
    let title = ITEMS
        .iter()
        .find(|(item, _)| *item == faq)
        .map_or("", |(_, title)| title);
    harness
        .get_by_role_and_label(AccessRole::Button, title)
        .click();
    harness.run_steps(40);
}

fn expanded(harness: &Harness<'_, Rig<App>>) -> Vec<Option<bool>> {
    ITEMS
        .iter()
        .map(|(_, title)| {
            harness
                .get_by_role_and_label(AccessRole::Button, title)
                .accesskit_node()
                .data()
                .is_expanded()
        })
        .collect()
}

fn height(harness: &Harness<'_, Rig<App>>, faq: Faq) -> f32 {
    support::rect_of(harness, &format!(r#"/"faq"/{faq:?}/Region"#)).height()
}

#[test]
fn single_collapsible_opens_one_closes_the_other_and_closes_it_again() {
    let mut harness = harness_of(App::of(Kind::Collapsible));
    click(&mut harness, Faq::Access);
    assert_eq!(harness.state().app.one, Some(Faq::Access));
    click(&mut harness, Faq::Motion);
    assert_eq!(harness.state().app.one, Some(Faq::Motion));
    assert_eq!(height(&harness, Faq::Access), 0.0, "the other closed");
    assert!(height(&harness, Faq::Motion) > 20.0, "it opened");
    assert_eq!(
        expanded(&harness),
        vec![Some(false), Some(true), Some(false)]
    );
    click(&mut harness, Faq::Motion);
    assert_eq!(harness.state().app.one, None, "the open one closes");
}

#[test]
fn single_always_cannot_close_the_open_item() {
    let mut harness = harness_of(App::of(Kind::Always));
    assert!(height(&harness, Faq::Access) > 20.0, "precondition: open");
    click(&mut harness, Faq::Access);
    assert_eq!(harness.state().app.always, Faq::Access);
    assert_eq!(harness.state().app.asked, 0, "the open header asks nothing");
    assert!(height(&harness, Faq::Access) > 20.0, "it stays open");
    click(&mut harness, Faq::Text);
    assert_eq!(harness.state().app.always, Faq::Text, "another replaces it");
}

#[test]
fn single_always_announces_the_open_header_as_disabled() {
    let harness = harness_of(App::of(Kind::Always));
    let disabled = |title: &str| {
        harness
            .get_by_role_and_label(AccessRole::Button, title)
            .accesskit_node()
            .is_disabled()
    };
    assert!(disabled("Is it accessible?"), "the open one");
    assert!(!disabled("Is it animated?"), "a closed one");
}

#[test]
fn multiple_opens_and_closes_each_on_its_own() {
    let mut harness = harness_of(App::of(Kind::Multiple));
    click(&mut harness, Faq::Access);
    click(&mut harness, Faq::Text);
    assert_eq!(
        harness.state().app.many,
        BTreeSet::from([Faq::Access, Faq::Text])
    );
    assert_eq!(
        expanded(&harness),
        vec![Some(true), Some(false), Some(true)]
    );
    click(&mut harness, Faq::Access);
    assert_eq!(harness.state().app.many, BTreeSet::from([Faq::Text]));
}

#[test]
fn a_disabled_item_is_not_toggled() {
    let mut harness = harness_of(App {
        disabled: Some(Faq::Motion),
        ..App::of(Kind::Collapsible)
    });
    click(&mut harness, Faq::Motion);
    assert_eq!(harness.state().app.asked, 0);
    assert_eq!(harness.state().app.one, None);
}

#[test]
fn each_open_header_controls_its_own_region() {
    let mut harness = harness_of(App::of(Kind::Multiple));
    click(&mut harness, Faq::Motion);
    let controlled: Vec<Vec<Option<String>>> = ITEMS
        .iter()
        .map(|(_, title)| {
            harness
                .get_by_role_and_label(AccessRole::Button, title)
                .accesskit_node()
                .controls()
                .map(|node| node.label())
                .collect()
        })
        .collect();
    assert_eq!(
        controlled,
        vec![vec![], vec![Some("Is it animated?".to_owned())], vec![]]
    );
}
