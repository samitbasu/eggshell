//! `ToggleGroup`: exclusive toggles placed anywhere below the group's builder,
//! driven through the real layout, event and draw path.

use eggshell_ui::contract::{Keyed, Metrics, Size};
use eggshell_ui::owned::{ButtonOptions, Face, ToggleGroup};
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens, size};
use egui::{Rect, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, box_at, hover_over, press_and_release, rect_of};

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

fn group(app: &App) -> ToggleGroup<App, Tool> {
    ToggleGroup::new(Part::Tools, app.tool).on_change(|app: &mut App, tool| {
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

fn harness_of(view: support::View<App>) -> Harness<'static, Rig<App>> {
    harness_styled(view, |_, _| {})
}

fn harness_styled(
    view: support::View<App>,
    prepare: fn(&egui::Context, &App),
) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness_prepared(
        vec2(500.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
        prepare,
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(6);
    harness
}

fn member_rect(harness: &Harness<'_, Rig<App>>, label: &str) -> Rect {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .rect()
}

fn click(harness: &mut Harness<'_, Rig<App>>, label: &str) {
    let at = member_rect(harness, label).center();
    hover_over(harness, at);
    press_and_release(harness, at);
    harness.run_steps(4);
}

fn is_on(harness: &Harness<'_, Rig<App>>, label: &str) -> bool {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .accesskit_node()
        .toggled()
        .map(|toggled| toggled == egui::accesskit::Toggled::True)
        .unwrap_or_else(|| panic!("{label} reports no on state"))
}

fn on_labels(harness: &Harness<'_, Rig<App>>) -> Vec<&'static str> {
    LABELS
        .iter()
        .filter(|(_, label)| is_on(harness, label))
        .map(|(_, label)| *label)
        .collect()
}

#[test]
fn exactly_the_member_with_the_current_value_is_on() {
    let mut harness = harness_of(view);
    assert_eq!(on_labels(&harness), ["Select"]);
    harness.state_mut().app.tool = Tool::Zoom;
    harness.run_steps(3);
    assert_eq!(on_labels(&harness), ["Zoom"]);
}

#[test]
fn clicking_another_member_calls_on_change_once_and_moves_the_on_state() {
    let mut harness = harness_of(view);
    click(&mut harness, "Pan");
    assert_eq!(harness.state().app.changes, [Tool::Pan]);
    assert_eq!(on_labels(&harness), ["Pan"]);
}

#[test]
fn clicking_the_member_that_is_on_keeps_it_on_and_does_not_call_on_change() {
    let mut harness = harness_of(view);
    assert!(is_on(&harness, "Select"), "precondition");
    click(&mut harness, "Select");
    assert!(harness.state().app.changes.is_empty());
    assert_eq!(on_labels(&harness), ["Select"]);
}

#[test]
fn members_after_other_nodes_and_nested_two_divs_deep_work() {
    let mut harness = harness_of(view);
    let rule = harness
        .state()
        .nodes
        .iter()
        .find(|(_, rect)| rect.width() == 1.0)
        .expect("the rule")
        .1;
    let zoom = member_rect(&harness, "Zoom");
    assert!(
        rule.right() <= zoom.left(),
        "precondition: a rule sits before Zoom"
    );
    click(&mut harness, "Zoom");
    click(&mut harness, "Pen");
    assert_eq!(harness.state().app.changes, [Tool::Zoom, Tool::Pen]);
    assert_eq!(on_labels(&harness), ["Pen"]);
}

#[test]
fn members_are_radio_buttons_in_the_accessibility_tree() {
    let mut harness = harness_of(view);
    harness.state_mut().companion = support::Companion::Alone;
    harness.run();
    assert_eq!(
        harness.query_all_by_role(Role::RadioButton).count(),
        LABELS.len()
    );
}

fn options_view(b: &mut Builder<'_, '_, App>, app: &App) {
    let tools = group(app).with_options(
        ButtonOptions::new()
            .size(size::ICON)
            .classes(tw().w_px(70.0)),
    );
    b.div(tw().flex().gap(4.0), |b| {
        b.toggle_item(&tools, Tool::Select, Face::Empty)
            .named("Select")
            .add();
        b.toggle_item(&tools, Tool::Pan, Face::Empty)
            .named("Pan")
            .size(Size::DEFAULT)
            .add();
        b.toggle_item(&tools, Tool::Zoom, Face::Empty)
            .named("Zoom")
            .with_options(ButtonOptions::new().classes(tw().w_px(90.0)))
            .add();
    });
}

const TALL: f32 = 50.0;

#[test]
fn group_options_apply_to_every_member_and_a_members_own_override_it_alone() {
    let harness = harness_styled(options_view, |ctx, _| {
        let mut style = Shadcn::for_theme(Theme::Light).style();
        let tall = Metrics {
            height: TALL,
            ..*style.button.toggle_sizes.get(Size::DEFAULT)
        };
        style.button.toggle_sizes = Keyed::new(Size::DEFAULT, tall)
            .with(size::ICON, *style.button.toggle_sizes.get(size::ICON));
        style.install(ctx);
    });
    let select = member_rect(&harness, "Select");
    let pan = member_rect(&harness, "Pan");
    let zoom = member_rect(&harness, "Zoom");
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
            b.toggle_item(&tools, tool, Face::Empty)
                .named(label)
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
    let mut harness = harness_of(cards_view);
    let paths = |harness: &Harness<'_, Rig<App>>| {
        harness
            .state()
            .nodes
            .iter()
            .filter(|(path, _)| path.ends_with("/\"check\""))
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>()
    };
    let select = paths(&harness);
    assert_eq!(select.len(), 1, "one check: {select:?}");
    assert!(
        select[0].contains("Select"),
        "on the chosen card: {select:?}"
    );
    let check = rect_of(&harness, &select[0]);
    assert_eq!(box_at(&harness, check).fill, CHECK);
    click(&mut harness, "Pan");
    let pan = paths(&harness);
    assert_eq!(pan.len(), 1, "one check: {pan:?}");
    assert!(pan[0].contains("Pan"), "moved to the new card: {pan:?}");
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "duplicate key path /Tools=Pan")]
fn two_members_with_one_value_in_a_group_are_a_duplicate_path() {
    fn twice(b: &mut Builder<'_, '_, App>, app: &App) {
        let tools = group(app);
        b.toggle_item(&tools, Tool::Pan, "Pan").add();
        b.toggle_item(&tools, Tool::Pan, "Pan again").add();
    }
    let _ = harness_of(twice);
}

#[test]
fn a_group_without_a_handler_builds_and_ignores_presses() {
    fn silent(b: &mut Builder<'_, '_, App>, app: &App) {
        let tools = ToggleGroup::new(Part::Tools, app.tool);
        b.toggle_item(&tools, Tool::Select, "Select").add();
        b.toggle_item(&tools, Tool::Pan, "Pan").add();
    }
    let mut harness = harness_of(silent);
    click(&mut harness, "Pan");
    assert!(is_on(&harness, "Select") && !is_on(&harness, "Pan"));
}
