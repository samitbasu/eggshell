//! `NodeHandle::controls` through the real path: build, draw and the
//! accessibility tree the frame hands the platform.

use eggshell_ui::overlay::Shown;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::Role as AccessRole;
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Sensing,
    Silent,
}

struct App {
    open: bool,
    target: Target,
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

/// ```html
/// <div class="flex flex-col">
///   <button aria-expanded aria-controls="details">Details</button>
///   <section id="details" class="overflow-hidden"> <p>Body</p> </section>
/// </div>
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div_id("page", tw().flex().flex_col().w_px(200.0), |b| {
        let region = b.id_of("details");
        b.div_id("head", tw().sense_click().h_px(30.0), |b| {
            b.text(tw(), "Details");
        })
        .role(egui::Role::Button)
        .named("Details")
        .expanded(app.open)
        .controls(region)
        .on_click(|app: &mut App| app.open = !app.open);
        let classes = match app.target {
            Target::Sensing => tw().sense_hover(),
            Target::Silent => tw(),
        };
        let region = b
            .collapsible("details", Shown::from(app.open))
            .classes(classes)
            .children(|b| b.text(tw(), "Body"));
        if app.target == Target::Sensing {
            region.role(egui::Role::Region).named("Details");
        }
    });
}

fn harness(open: bool, target: Target) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), App { open, target }, view);
    harness.run_steps(4);
    harness
}

fn controlled(harness: &Harness<'_, Rig<App>>) -> Vec<(AccessRole, Option<String>)> {
    harness
        .get_by_role_and_label(AccessRole::Button, "Details")
        .accesskit_node()
        .controls()
        .map(|node| (node.role(), node.label()))
        .collect()
}

#[test]
fn a_header_announces_the_region_it_controls() {
    let harness = harness(true, Target::Sensing);
    assert_eq!(
        controlled(&harness),
        vec![(AccessRole::Region, Some("Details".to_owned()))]
    );
}

#[test]
fn the_relation_names_the_node_of_the_id_it_was_given() {
    let harness = harness(true, Target::Sensing);
    let region = support::id_of(&harness, r#"/"page"/"details""#);
    let ids: Vec<_> = harness
        .get_by_role_and_label(AccessRole::Button, "Details")
        .accesskit_node()
        .data()
        .controls()
        .to_vec();
    assert_eq!(ids, vec![region.accesskit_id()]);
}

#[test]
fn a_relation_to_a_node_outside_the_tree_is_not_announced() {
    let harness = harness(true, Target::Silent);
    assert!(
        harness.query_by_role(AccessRole::Region).is_none()
            && harness.query_by_label("Body").is_some(),
        "precondition: the open region, which does not sense, has no accessibility node"
    );
    assert!(controlled(&harness).is_empty());
}

#[test]
fn the_relation_follows_the_region_open_and_closed() {
    let mut harness = harness(false, Target::Sensing);
    let closed = controlled(&harness);
    harness
        .get_by_role_and_label(AccessRole::Button, "Details")
        .click();
    harness.run_steps(40);
    assert!(
        harness.state().app.open,
        "precondition: the click opened it"
    );
    assert_eq!(
        controlled(&harness),
        vec![(AccessRole::Region, Some("Details".to_owned()))]
    );
    harness
        .get_by_role_and_label(AccessRole::Button, "Details")
        .click();
    harness.run_steps(40);
    assert!(
        !harness.state().app.open,
        "precondition: the click closed it"
    );
    assert_eq!(controlled(&harness), closed);
}
