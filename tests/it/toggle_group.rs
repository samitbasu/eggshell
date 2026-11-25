//! `toggle_group::Group`: exclusive toggles placed anywhere below the group's builder,
//! driven through the real layout, event and draw path.

use std::sync::Arc;

use eggshell_ui::contract::{Keyed, Metrics, Size, Themed};
use eggshell_ui::owned::{
    button::{self, Face},
    toggle_group,
};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, Tokens, size};
use eggshell_ui::{Builder, Problem, Style, TextStyle, tw};
use egui::{Rect, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, box_at};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Part {
    Tools,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Tool {
    Select,
    Pan,
    Zoom,
    Pen,
}

const LABELS: [(Tool, &str); 4] = [
    (Tool::Select, "Select"),
    (Tool::Pan, "Pan"),
    (Tool::Zoom, "Zoom"),
    (Tool::Pen, "Pen"),
];

struct App {
    tool: Tool,
    changes: Vec<Tool>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            tool: Tool::Select,
            changes: Vec::new(),
        }
    }
}

fn group(app: &App) -> toggle_group::Group<App, Tool> {
    toggle_group::Group::new(Part::Tools, app.tool).on_change(|app: &mut App, tool| {
        app.changes.push(tool);
        app.tool = tool;
    })
}

/// ```html
/// <div class="flex">
///   <button>Select</button> <button>Pan</button>
///   <div class="rule"/>
///   <button>Zoom</button>
///   <div><div><button>Pen</button></div></div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let tools = group(app);
    b.div(tw().flex().gap(4.0), |b| {
        b.toggle_item(&tools, Tool::Select, "Select").add();
        b.toggle_item(&tools, Tool::Pan, "Pan").add();
        b.div(tw().w_px(1.0).h_px(20.0), |_| {});
        b.toggle_item(&tools, Tool::Zoom, "Zoom").add();
        b.div(tw().flex(), |b| {
            b.div(tw().flex(), |b| {
                b.toggle_item(&tools, Tool::Pen, "Pen").add();
            });
        });
    });
}

fn harness_of(view: support::View<App>) -> Driver<App> {
    harness_styled(view, support::shadcn())
}

fn harness_styled(view: support::View<App>, style: Themed<Arc<Style>>) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(500.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .style(style)
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(6);
    d
}

fn member_rect(d: &Driver<App>, label: &str) -> Rect {
    d.rect(Target::role(Role::RadioButton, label))
}

fn click(d: &mut Driver<App>, label: &str) {
    d.click(Target::role(Role::RadioButton, label)).frames(4);
}

fn is_on(d: &Driver<App>, label: &str) -> bool {
    d.harness()
        .get_by_role_and_label(Role::RadioButton, label)
        .accesskit_node()
        .toggled()
        .map(|toggled| toggled == egui::accesskit::Toggled::True)
        .unwrap_or_else(|| panic!("{label} reports no on state"))
}

fn on_labels(d: &Driver<App>) -> Vec<&'static str> {
    LABELS
        .iter()
        .filter(|(_, label)| is_on(d, label))
        .map(|(_, label)| *label)
        .collect()
}

#[test]
fn exactly_the_member_with_the_current_value_is_on() {
    let mut d = harness_of(view);
    assert_eq!(on_labels(&d), ["Select"]);
    d.app_mut().tool = Tool::Zoom;
    d.frames(3);
    assert_eq!(on_labels(&d), ["Zoom"]);
}

#[test]
fn clicking_another_member_calls_on_change_once_and_moves_the_on_state() {
    let mut d = harness_of(view);
    click(&mut d, "Pan");
    assert_eq!(d.app().changes, [Tool::Pan]);
    assert_eq!(on_labels(&d), ["Pan"]);
}

#[test]
fn clicking_the_member_that_is_on_keeps_it_on_and_does_not_call_on_change() {
    let mut d = harness_of(view);
    assert!(is_on(&d, "Select"), "precondition");
    click(&mut d, "Select");
    assert!(d.app().changes.is_empty());
    assert_eq!(on_labels(&d), ["Select"]);
}

#[test]
fn members_after_other_nodes_and_nested_two_divs_deep_work() {
    let mut d = harness_of(view);
    let rule = support::nodes(&d)
        .into_iter()
        .find(|(_, rect)| rect.width() == 1.0)
        .expect("the rule")
        .1;
    let zoom = member_rect(&d, "Zoom");
    assert!(
        rule.right() <= zoom.left(),
        "precondition: a rule sits before Zoom"
    );
    click(&mut d, "Zoom");
    click(&mut d, "Pen");
    assert_eq!(d.app().changes, [Tool::Zoom, Tool::Pen]);
    assert_eq!(on_labels(&d), ["Pen"]);
}

#[test]
fn members_are_radio_buttons_in_the_accessibility_tree() {
    let mut d = harness_of(view);
    support::set_companion(&d, support::Companion::Alone);
    d.harness_mut().run();
    assert_eq!(
        d.harness().query_all_by_role(Role::RadioButton).count(),
        LABELS.len()
    );
}

fn options_view(b: &mut Builder<'_, '_, App>, app: &App) {
    let tools = group(app).with_options(
        button::Options::new()
            .size(size::ICON)
            .classes(tw().w_px(70.0)),
    );
    b.div(tw().flex().gap(4.0), |b| {
        b.toggle_item(&tools, Tool::Select, Face::Empty("Select".into()))
            .add();
        b.toggle_item(&tools, Tool::Pan, Face::Empty("Pan".into()))
            .size(Size::DEFAULT)
            .add();
        b.toggle_item(&tools, Tool::Zoom, Face::Empty("Zoom".into()))
            .with_options(button::Options::new().classes(tw().w_px(90.0)))
            .add();
    });
}

const TALL: f32 = 50.0;

#[test]
fn group_options_apply_to_every_member_and_a_members_own_override_it_alone() {
    let mut style = Shadcn::for_theme(Theme::Light).style();
    let tall = Metrics {
        height: TALL,
        ..*style.toggle.sizes.get(Size::DEFAULT)
    };
    style.toggle.sizes =
        Keyed::new(Size::DEFAULT, tall).with(size::ICON, *style.toggle.sizes.get(size::ICON));
    let d = harness_styled(options_view, Themed::both(Arc::new(style)));
    let select = member_rect(&d, "Select");
    let pan = member_rect(&d, "Pan");
    let zoom = member_rect(&d, "Zoom");
    assert_eq!(select.height(), 36.0, "the group's size");
    assert_eq!(zoom.height(), 36.0, "the group's size");
    assert_eq!(pan.height(), TALL, "the member's own size wins");
    assert_eq!(zoom.width(), 90.0, "the member's class over the group's");
    assert_eq!(select.width(), 70.0, "the group's class");
    assert_eq!(pan.width(), 70.0, "the group's class");
}

const CHECK: egui::Color32 = egui::Color32::from_rgb(20, 160, 60);

fn cards_view(b: &mut Builder<'_, '_, App>, app: &App) {
    let tools = group(app);
    b.div(tw().flex().gap(4.0), |b| {
        for (tool, label) in LABELS.into_iter().take(2) {
            let chosen = app.tool == tool;
            b.toggle_item(&tools, tool, Face::Empty(label.into()))
                .classes(tw().flex_col().h_px(60.0).w_px(100.0))
                .children(|s| {
                    s.text(tw(), label);
                    if chosen {
                        s.div_id("check", tw().w_px(8.0).h_px(8.0).bg(CHECK), |_| {});
                    }
                })
                .add();
        }
    });
}

#[test]
fn a_card_member_has_its_check_only_while_chosen() {
    let mut d = harness_of(cards_view);
    let paths = |d: &Driver<App>| {
        support::nodes(d)
            .into_iter()
            .filter(|(path, _)| path.ends_with("/\"check\""))
            .map(|(path, _)| path)
            .collect::<Vec<_>>()
    };
    let select = paths(&d);
    assert_eq!(select.len(), 1, "one check: {select:?}");
    assert!(
        select[0].contains("Select"),
        "on the chosen card: {select:?}"
    );
    let check = d.rect(Target::suffix(&select[0]));
    assert_eq!(box_at(&d, check).fill, CHECK);
    click(&mut d, "Pan");
    let pan = paths(&d);
    assert_eq!(pan.len(), 1, "one check: {pan:?}");
    assert!(pan[0].contains("Pan"), "moved to the new card: {pan:?}");
}

#[cfg(debug_assertions)]
#[test]
fn two_members_with_one_value_in_a_group_are_a_duplicate_path() {
    fn twice(b: &mut Builder<'_, '_, App>, app: &App) {
        let tools = group(app);
        b.toggle_item(&tools, Tool::Pan, "Pan").add();
        b.toggle_item(&tools, Tool::Pan, "Pan again").add();
    }
    let mut d = harness_of(twice);
    let report = d.expect_reported(Problem::DuplicateKey);
    assert!(
        report.message.contains("duplicate key path /Tools=Pan"),
        "{}",
        report.message
    );
}

#[test]
fn a_group_without_a_handler_builds_and_ignores_presses() {
    fn silent(b: &mut Builder<'_, '_, App>, app: &App) {
        let tools = toggle_group::Group::new(Part::Tools, app.tool);
        b.toggle_item(&tools, Tool::Select, "Select").add();
        b.toggle_item(&tools, Tool::Pan, "Pan").add();
    }
    let mut d = harness_of(silent);
    click(&mut d, "Pan");
    assert!(is_on(&d, "Select") && !is_on(&d, "Pan"));
}
