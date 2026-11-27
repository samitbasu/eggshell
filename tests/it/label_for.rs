//! `NodeUi::label_for` through the real path: build, draw and the
//! accessibility tree the frame hands the platform.

use eggshell_ui::{Builder, NodeUi as _, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::Role as AccessRole;
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _, by};

use crate::support::{self, Rig};

/// Where the control the label names is built.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    BeforeLabel,
    AfterLabel,
    Missing,
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

fn control(b: &mut Builder<'_, '_, Control>) {
    b.div_id("control", tw().sense_click().h_px(20.0), |_| {})
        .role(egui::Role::CheckBox)
        .named("Own name");
}

/// ```html
/// <div class="flex flex-col">
///   <input type="checkbox" id="control" aria-label="Own name">
///   <label for="control">Shown name</label>
/// </div>
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, Control>, app: &'f Control) {
    let app = *app;
    b.div_id("page", tw().flex().flex_col().w_px(200.0), |b| {
        let id = b.id_of("control");
        if app == Control::BeforeLabel {
            control(b);
        }
        b.span("label", tw().sense_click().focusable_none(), "Shown name")
            .draw(move |_, ui| {
                ui.label_for(id, "Shown name");
            });
        if app == Control::AfterLabel {
            control(b);
        }
    });
}

fn harness(control: Control) -> Harness<'static, Rig<Control>> {
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), control, view);
    harness.run_steps(4);
    harness
}

/// The labels that name the control, by the text they show.
fn labels(harness: &Harness<'_, Rig<Control>>) -> Vec<Option<String>> {
    harness
        .get_by_role(AccessRole::CheckBox)
        .accesskit_node()
        .labelled_by()
        .map(|node| node.value())
        .collect()
}

#[test]
fn a_label_names_a_control_built_before_it() {
    let harness = harness(Control::BeforeLabel);
    assert_eq!(labels(&harness), [Some("Shown name".to_owned())]);
    assert!(harness.query_by_label("Own name").is_none());
}

#[test]
fn a_label_names_a_control_built_after_it() {
    let harness = harness(Control::AfterLabel);
    assert_eq!(labels(&harness), [Some("Shown name".to_owned())]);
    assert!(harness.query_by_label("Own name").is_none());
}

#[test]
fn a_label_for_a_control_not_built_names_nothing() {
    let harness = harness(Control::Missing);
    assert!(
        harness.query_by_role(AccessRole::CheckBox).is_none()
            && harness.query_by_role(AccessRole::Label).is_some(),
        "precondition: the label is in the tree and the control is not"
    );
    let labelled = harness
        .query_all(by().predicate(|node| !node.data().labelled_by().is_empty()))
        .count();
    assert_eq!(labelled, 0, "no node, phantom or not, is labelled by it");
}
