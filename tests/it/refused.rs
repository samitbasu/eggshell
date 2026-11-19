//! `refused(reason)` (#315): a control is disabled while it has a reason, the
//! reason replaces its tooltip and is its accessible description, and `None`
//! clears it. With the gap fills, `when`/`when_some`, and `classes` merging.

use core::time::Duration;

use eggshell_ui::owned::{Choice, Enablement};
use eggshell_ui::{Builder, Command, Commands, Style, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Role, Shape, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, hover_node, rect_of};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pick {
    A,
    B,
}

#[derive(Hash, Debug, Clone, Copy)]
enum Cmd {
    Run,
}

#[derive(Default)]
struct App {
    /// Why "save" is refused, if it is.
    save: Option<&'static str>,
    saves: u32,
    picked: Vec<Pick>,
    /// Why the segmented control is refused, if it is.
    group: Option<&'static str>,
    custom_enabled: Option<bool>,
    runs: u32,
    note: bool,
}

fn prepare(ctx: &egui::Context, _: &App) {
    let mut style = (*Style::of(ctx)).clone();
    style.tooltip.delay = Duration::from_millis(100);
    style.install(ctx);
}

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    let table = Commands::new().add(Cmd::Run, Command::new("Run", |app: &mut App| app.runs += 1));
    b.commands(&table, app);
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.button("save", "Save")
            .tooltip("Saves the file")
            .refused(app.save)
            .on_click(|app| app.saves += 1);
        b.button("run", "Run").refused("Not now").command(Cmd::Run);
        b.text_field("name", "Ada")
            .named("Name")
            .refused("Locked by an admin")
            .on_change(|_, _| {});
        b.text_field("plain", "Grace")
            .named("Plain")
            .enablement(Enablement::Disabled)
            .on_change(|_, _| {});
        b.segmented(
            "group",
            Pick::A,
            [
                Choice::new(Pick::A, "Alpha").tooltip("Alpha's own"),
                Choice::new(Pick::B, "Beta"),
            ],
        )
        .named("Group")
        .refused(app.group)
        .on_change(|app, pick| app.picked.push(pick));
        b.segmented(
            "one",
            Pick::A,
            [
                Choice::new(Pick::A, "Gamma"),
                Choice::new(Pick::B, "Delta").refused("Delta is retired"),
            ],
        )
        .on_change(|app, pick| app.picked.push(pick));
        b.radio_group("size", Pick::A, [(Pick::A, "Small"), (Pick::B, "Large")])
            .named("Size")
            .tooltip("Pick a size")
            .on_change(|_, _| {});
        b.custom("custom", tw().w_px(40.0).h_px(20.0))
            .refused("Busy")
            .ui(|app, ui| app.custom_enabled = Some(ui.is_enabled()));
        b.checkbox("merged", "Merged", false)
            .classes(tw().w_px(200.0))
            .classes(tw().h_px(40.0))
            .on_change(|_, _| {});
        b.button("maybe", "Maybe")
            .when(app.note, |chain| chain.tooltip("Noted"))
            .when_some(app.save, |chain, why| chain.refused(why))
            .add();
        b.stepper("off", 3.0, 0.0..=9.0)
            .named("Off")
            .enablement(Enablement::Disabled)
            .on_change(|_, _| {});
        b.text_field("open", "Ok")
            .named("Open")
            .on_change(|_, _| {});
        b.div_id("blank", tw().w_px(200.0).h_px(40.0), |_| {});
        b.dropdown_menu("menu", "More")
            .refused("Nothing to show")
            .items(|m| m.item("one", "One").on_click(|_| {}));
    });
}

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness_scaled_stepped(
        vec2(400.0, 900.0),
        0.05,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
        prepare,
    );
    harness.run_steps(8);
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_with(App::default())
}

fn painted(harness: &Harness<'_, Rig<App>>, text: &str) -> bool {
    support::shapes(harness).iter().any(|shape| match shape {
        Shape::Text(shape) => shape.galley.text() == text,
        _ => false,
    })
}

fn tooltip_boxes(harness: &Harness<'_, Rig<App>>) -> usize {
    harness
        .state()
        .nodes
        .iter()
        .filter(|(path, _)| path.contains("Tooltip(") && path.ends_with("/\"Box\""))
        .count()
}

fn rest_on(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    hover_node(harness, path);
    support::rest_for_tooltip(harness);
}

fn description(harness: &Harness<'_, Rig<App>>, label: &str) -> Option<String> {
    harness
        .get_by_label(label)
        .accesskit_node()
        .description()
        .map(|text| text.to_string())
}

fn disabled(harness: &Harness<'_, Rig<App>>, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_disabled()
}

mod button {
    use super::*;

    #[test]
    fn a_refused_button_is_disabled_and_its_reason_replaces_the_tooltip() {
        let mut harness = harness_with(App {
            save: Some("Nothing to save"),
            ..App::default()
        });
        rest_on(&mut harness, "/\"save\"");
        assert!(painted(&harness, "Nothing to save"));
        assert!(
            !painted(&harness, "Saves the file"),
            "the reason replaces it"
        );
        click_node(&mut harness, "/\"save\"");
        harness.run_steps(2);
        assert_eq!(
            harness.state().app.saves,
            0,
            "a refused button does nothing"
        );
        assert!(disabled(&harness, "Save"));
        assert_eq!(
            description(&harness, "Save").as_deref(),
            Some("Nothing to save")
        );
    }

    #[test]
    fn none_clears_the_refusal() {
        let mut harness = harness();
        click_node(&mut harness, "/\"save\"");
        harness.run_steps(2);
        assert_eq!(harness.state().app.saves, 1);
        assert!(!disabled(&harness, "Save"));
        assert_eq!(
            description(&harness, "Save").as_deref(),
            Some("Saves the file")
        );
    }

    #[test]
    fn a_refusal_that_goes_away_gives_the_button_back() {
        let mut harness = harness_with(App {
            save: Some("Nothing to save"),
            ..App::default()
        });
        harness.state_mut().app.save = None;
        harness.run_steps(4);
        click_node(&mut harness, "/\"save\"");
        harness.run_steps(2);
        assert_eq!(harness.state().app.saves, 1);
    }

    #[test]
    fn a_refused_commands_button_is_disabled_with_its_reason() {
        let mut harness = harness();
        click_node(&mut harness, "/\"run\"");
        harness.run_steps(2);
        assert_eq!(
            harness.state().app.runs,
            0,
            "the command is enabled; the refusal is not"
        );
        assert!(disabled(&harness, "Run"));
        assert_eq!(description(&harness, "Run").as_deref(), Some("Not now"));
    }

    #[test]
    fn when_and_when_some_apply_only_when_they_hold() {
        let harness = harness();
        assert!(!disabled(&harness, "Maybe"));
        assert_eq!(description(&harness, "Maybe"), None);
        let harness = harness_with(App {
            note: true,
            save: Some("Later"),
            ..App::default()
        });
        assert!(disabled(&harness, "Maybe"));
        assert_eq!(
            description(&harness, "Maybe").as_deref(),
            Some("Later"),
            "the refusal replaces the tooltip `when` added"
        );
    }
}

mod text_field {
    use super::*;

    #[test]
    fn a_disabled_field_takes_no_focus_from_a_click_a_drag_or_tab() {
        let mut harness = harness();
        for field in ["/\"name\"", "/\"plain\""] {
            click_node(&mut harness, field);
            harness.run_steps(2);
            assert_eq!(focused_path(&harness), None, "{field}: a click");
            let rect = rect_of(&harness, field);
            support::drag(&mut harness, rect.left_center(), rect.right_center());
            harness.run_steps(2);
            assert_eq!(focused_path(&harness), None, "{field}: a drag");
        }
        click_node(&mut harness, "/\"run\"");
        harness.key_press(egui::Key::Tab);
        harness.run_steps(2);
        let landed = focused_path(&harness).unwrap_or_default();
        assert!(
            !landed.contains("\"name\"") && !landed.contains("\"plain\""),
            "Tab passes the disabled fields: {landed}"
        );
        assert!(disabled(&harness, "Name") && disabled(&harness, "Plain"));
    }

    #[test]
    fn a_refused_fields_reason_is_its_tooltip_and_description() {
        let mut harness = harness();
        rest_on(&mut harness, "/\"name\"");
        assert!(painted(&harness, "Locked by an admin"));
        assert_eq!(
            description(&harness, "Name").as_deref(),
            Some("Locked by an admin")
        );
    }
}

mod choices {
    use super::*;

    #[test]
    fn a_refused_group_disables_every_choice_and_shows_its_reason_once() {
        let mut harness = harness_with(App {
            group: Some("Locked"),
            ..App::default()
        });
        rest_on(&mut harness, "/\"group\"/0");
        assert!(
            painted(&harness, "Locked"),
            "over a choice with its own tooltip"
        );
        assert!(!painted(&harness, "Alpha's own"));
        assert_eq!(tooltip_boxes(&harness), 1);
        click_node(&mut harness, "/\"group\"/1");
        harness.run_steps(2);
        assert!(harness.state().app.picked.is_empty());
        assert!(disabled(&harness, "Alpha") && disabled(&harness, "Beta"));
        assert_eq!(description(&harness, "Alpha").as_deref(), Some("Locked"));
    }

    #[test]
    fn an_unrefused_group_shows_a_choices_own_tooltip() {
        let mut harness = harness();
        rest_on(&mut harness, "/\"group\"/0");
        assert!(painted(&harness, "Alpha's own"));
    }

    #[test]
    fn a_refused_choice_is_disabled_with_its_reason_and_the_others_are_not() {
        let mut harness = harness();
        rest_on(&mut harness, "/\"one\"/1");
        assert!(painted(&harness, "Delta is retired"));
        click_node(&mut harness, "/\"one\"/1");
        harness.run_steps(2);
        assert!(harness.state().app.picked.is_empty());
        assert!(disabled(&harness, "Delta"));
        assert!(!disabled(&harness, "Gamma"));
        assert_eq!(
            description(&harness, "Delta").as_deref(),
            Some("Delta is retired")
        );
    }

    #[test]
    fn a_segmented_control_and_a_radio_group_are_named_groups() {
        let harness = harness();
        for name in ["Group", "Size"] {
            let node = harness.get_by_label(name);
            assert_eq!(node.accesskit_node().role(), Role::RadioGroup, "{name}");
        }
    }

    #[test]
    fn a_radio_groups_tooltip_is_one_trigger_on_the_group() {
        let mut harness = harness();
        rest_on(&mut harness, "/\"size\"/Circle(1)");
        assert!(painted(&harness, "Pick a size"));
        assert_eq!(tooltip_boxes(&harness), 1);
    }
}

#[test]
fn a_refused_custom_nodes_draw_sees_a_disabled_ui() {
    let harness = harness();
    assert_eq!(harness.state().app.custom_enabled, Some(false));
}

#[test]
fn a_refused_dropdown_does_not_open() {
    let mut harness = harness();
    click_node(&mut harness, "/\"menu\"");
    harness.run_steps(4);
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.contains("Menu)")),
        "no menu"
    );
    assert!(disabled(&harness, "More"));
}

#[test]
fn classes_set_twice_merge() {
    let harness = harness();
    let row = rect_of(&harness, "/\"merged\"/<div>");
    assert_eq!(row.width(), 200.0, "the first call's width stays");
    assert_eq!(row.height(), 40.0, "the second call's height is added");
}

mod press {
    use super::*;

    fn press_away(harness: &mut Harness<'_, Rig<App>>) {
        click_node(harness, "/\"blank\"");
        harness.run_steps(6);
    }

    #[test]
    fn a_click_on_a_refused_button_shows_its_reason_at_once_and_fires_nothing() {
        let mut harness = harness_with(App {
            save: Some("Nothing to save"),
            ..App::default()
        });
        click_node(&mut harness, "/\"save\"");
        harness.run_steps(3);
        assert!(
            painted(&harness, "Nothing to save"),
            "shown without the delay"
        );
        assert_eq!(harness.state().app.saves, 0);
        hover_node(&mut harness, "/\"blank\"");
        harness.run_steps(12);
        assert!(
            painted(&harness, "Nothing to save"),
            "it stays as the pointer leaves"
        );
        press_away(&mut harness);
        assert!(
            !painted(&harness, "Nothing to save"),
            "a press elsewhere closes it"
        );
    }

    #[test]
    fn a_tap_on_a_refused_field_shows_its_reason_and_a_press_elsewhere_closes_it() {
        let mut harness = harness();
        click_node(&mut harness, "/\"name\"");
        harness.run_steps(3);
        assert!(painted(&harness, "Locked by an admin"));
        assert_eq!(focused_path(&harness), None);
        press_away(&mut harness);
        assert!(!painted(&harness, "Locked by an admin"));
    }

    #[test]
    fn a_tap_on_a_refused_choice_shows_its_reason() {
        let mut harness = harness();
        click_node(&mut harness, "/\"one\"/1");
        harness.run_steps(3);
        assert!(painted(&harness, "Delta is retired"));
        assert!(harness.state().app.picked.is_empty());
    }

    #[test]
    fn a_click_on_an_enabled_control_with_a_tooltip_shows_nothing() {
        let mut harness = harness();
        click_node(&mut harness, "/\"save\"");
        harness.run_steps(3);
        assert!(
            !painted(&harness, "Saves the file"),
            "a press still closes a tooltip"
        );
    }
}

#[test]
fn a_refused_field_takes_the_disabled_look() {
    let harness = harness();
    let refused = support::text_color(&harness, rect_of(&harness, "/\"name\""), "Ada");
    let enabled = support::text_color(&harness, rect_of(&harness, "/\"open\""), "Ok");
    let disabled = support::text_color(&harness, rect_of(&harness, "/\"off\"/Box"), "3");
    assert_ne!(refused, enabled, "not the enabled look");
    assert_eq!(refused, disabled, "the disabled look the stepper has");
}
