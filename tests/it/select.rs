//! The owned select on overlays, through the real path: kittest's pointer and
//! key events, build, layout, draw and the judgement of the frame's input.

use eggshell_ui::overlay::Modal;
use eggshell_ui::owned::{Choice, Enablement};
use eggshell_ui::{Builder, Style, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Key, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, rect_of};

const SCREEN: egui::Vec2 = vec2(400.0, 400.0);

const PICK: &str = r#"/"Page"/"Top"/"pick""#;
const LIST: &str = r#"/"Page"/"Top"/("pick", List)"#;
const OTHER: &str = r#"/"Page"/"Top"/"other""#;
const DIALOG: &str = r#"/"Page"/"dialog""#;
const MANY: &str = r#"/"Page"/"many""#;
const MANY_LIST: &str = r#"/"Page"/("many", List)"#;

const CONTINENTS: [(usize, &str); 4] = [
    (0, "Europe"),
    (1, "North America"),
    (2, "Asia Pacific"),
    (3, "Africa"),
];

#[derive(Default)]
struct App {
    pick: usize,
    many: usize,
    clicks: u32,
    dialog: bool,
    inner: usize,
    changes: Vec<usize>,
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

fn continents() -> impl Iterator<Item = Choice<usize>> {
    CONTINENTS.into_iter().map(|(value, label)| {
        let choice = Choice::new(value, label);
        if value == 1 {
            choice.enablement(Enablement::Disabled)
        } else {
            choice
        }
    })
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        "Page",
        tw().flex()
            .flex_col()
            .items_start()
            .gap(2.0)
            .p(2.0)
            .w_full()
            .h_full(),
        |s| {
            s.div_id("Top", tw().flex().gap(2.0), |s| {
                s.select("pick", app.pick, continents())
                    .named("Pick")
                    .classes(tw().w_px(160.0))
                    .on_change(|app: &mut App, value| {
                        app.pick = value;
                        app.changes.push(value);
                    });
                s.button("other", "Other")
                    .on_click(|app: &mut App| app.clicks += 1);
            });
            s.button("dialog", "Dialog")
                .on_click(|app: &mut App| app.dialog = true);
            s.div(tw().grow(), |_| {});
            s.select(
                "many",
                app.many,
                (0..50).map(|n| (n, format!("Option {n}"))),
            )
            .named("Many")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, value| app.many = value);
        },
    );
    if app.dialog {
        b.overlay("dialog")
            .modal(Modal::Center)
            .role(Role::Dialog)
            .named("Dialog")
            .on_dismiss(|app: &mut App| app.dialog = false)
            .children(|s| {
                s.div_id(
                    "Box",
                    tw().flex()
                        .flex_col()
                        .gap(2.0)
                        .p(4.0)
                        .w_px(240.0)
                        .bg(Color32::WHITE)
                        .border(1.0)
                        .border_color(Color32::GRAY),
                    |s| {
                        s.select("inner", app.inner, continents())
                            .named("Inner")
                            .on_change(|app: &mut App, value| app.inner = value);
                    },
                );
            });
    }
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(SCREEN, base_text(), App::default(), view);
    harness.run_steps(4);
    harness
}

fn settle(harness: &mut Harness<'_, Rig<App>>) {
    harness.run_steps(5);
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    settle(harness);
}

fn has(harness: &Harness<'_, Rig<App>>, path: &str) -> bool {
    harness
        .state()
        .nodes
        .iter()
        .any(|(candidate, _)| candidate == path)
}

fn open(harness: &Harness<'_, Rig<App>>) -> bool {
    has(harness, &format!("{LIST}/Box"))
}

fn row(list: &str, index: usize) -> String {
    format!("{list}/Box/Option({index})")
}

fn tall_row(index: usize) -> String {
    format!("{MANY_LIST}/Box/Rows/Option({index})")
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..8 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        harness.key_press(Key::Tab);
        settle(harness);
    }
    panic!("could not tab to {path}");
}

/// The rows, among `0..count` of `list`, drawn in the item's highlight colours.
fn highlighted(harness: &Harness<'_, Rig<App>>, list: &str, count: usize) -> Vec<usize> {
    let hover = Style::of(&harness.ctx).menu.item.paint.states.hover.fill;
    (0..count)
        .filter(|&index| support::fill_of(harness, &row(list, index)) == Some(hover))
        .collect()
}

#[test]
fn the_rows_are_eggshell_nodes_with_the_option_role() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(open(&harness), "precondition: a click opens the list");
    for (index, (_, label)) in CONTINENTS.into_iter().enumerate() {
        let node = harness.get_by_role_and_label(Role::ListBoxOption, label);
        assert!(
            rect_of(&harness, &row(LIST, index)).contains_rect(node.rect()),
            "{label} is the node at its path"
        );
    }
    let list = harness.get_by_role_and_label(Role::ListBox, "Pick");
    assert!(list.rect().contains_rect(rect_of(&harness, &row(LIST, 0))));
    assert_eq!(
        harness
            .get_by_role_and_label(Role::ListBoxOption, "Europe")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True),
        "the chosen option is selected"
    );
}

#[test]
fn the_trigger_says_whether_its_list_is_open() {
    let mut harness = harness();
    let trigger = |harness: &Harness<'_, Rig<App>>| {
        let node = harness.get_by_role_and_label(Role::ComboBox, "Pick");
        let data = node.accesskit_node().data().clone();
        (data.is_expanded(), data.has_popup())
    };
    assert_eq!(
        trigger(&harness),
        (Some(false), Some(egui::accesskit::HasPopup::Listbox))
    );
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert_eq!(
        trigger(&harness),
        (Some(true), Some(egui::accesskit::HasPopup::Listbox))
    );
}

#[test]
fn the_list_opens_with_the_chosen_option_highlighted() {
    let mut harness = harness();
    harness.state_mut().app.pick = 2;
    settle(&mut harness);
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(open(&harness), "precondition");
    assert_eq!(highlighted(&harness, LIST, 4), [2]);
}

#[test]
fn arrow_up_opens_the_list_as_down_does() {
    let mut harness = harness();
    tab_to(&mut harness, PICK);
    press(&mut harness, Key::ArrowUp);
    assert!(open(&harness));
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(PICK),
        "focus stays on the trigger"
    );
}

#[test]
fn a_list_opened_with_the_pointer_is_chosen_with_the_keyboard() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(open(&harness), "precondition");
    assert_eq!(
        focused_path(&harness),
        None,
        "precondition: nothing focused"
    );
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        focused_path(&harness),
        None,
        "the arrow moved the highlight, not egui's focus"
    );
    press(&mut harness, Key::Enter);
    assert_eq!(
        harness.state().app.changes,
        [2],
        "the disabled option is skipped"
    );
    assert!(!open(&harness));
}

#[test]
fn only_one_option_looks_highlighted_whether_the_pointer_or_the_keys_moved_it() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    let target = rect_of(&harness, &row(LIST, 3)).center();
    support::hover_over(&mut harness, target);
    settle(&mut harness);
    assert_eq!(highlighted(&harness, LIST, 4), [3], "the pointer moved it");
    press(&mut harness, Key::ArrowUp);
    assert_eq!(
        highlighted(&harness, LIST, 4),
        [2],
        "the keys moved it, though the pointer is still over option 3"
    );
}

#[test]
fn choosing_with_the_pointer_leaves_focus_on_the_trigger() {
    let mut harness = harness();
    tab_to(&mut harness, PICK);
    press(&mut harness, Key::Enter);
    assert!(open(&harness), "precondition");
    support::press(&mut harness, &row(LIST, 2));
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some(PICK),
        "held through the press"
    );
    support::release(&mut harness, &row(LIST, 2));
    assert_eq!(harness.state().app.changes, [2]);
    assert_eq!(focused_path(&harness).as_deref(), Some(PICK));
}

#[test]
fn the_arrows_stop_at_the_ends_of_the_list() {
    let mut harness = harness();
    tab_to(&mut harness, PICK);
    press(&mut harness, Key::Enter);
    press(&mut harness, Key::End);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(highlighted(&harness, LIST, 4), [3]);
    press(&mut harness, Key::Home);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(highlighted(&harness, LIST, 4), [0]);
}

#[test]
fn a_disabled_option_cannot_be_clicked() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    click_node(&mut harness, &row(LIST, 1));
    settle(&mut harness);
    assert!(harness.state().app.changes.is_empty());
    assert!(open(&harness), "the list stays open");
}

#[test]
fn the_press_that_closes_the_list_is_swallowed() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    let other = rect_of(&harness, OTHER);
    assert!(
        !rect_of(&harness, LIST).intersects(other),
        "precondition: the button is clear of the list"
    );
    click_node(&mut harness, OTHER);
    settle(&mut harness);
    assert!(!open(&harness), "the press closed the list");
    assert_eq!(harness.state().app.clicks, 0, "and the button never saw it");
    click_node(&mut harness, OTHER);
    settle(&mut harness);
    assert_eq!(
        harness.state().app.clicks,
        1,
        "control: a press now reaches it"
    );
}

#[test]
fn the_list_is_as_wide_as_its_options_and_has_no_scroll_gutter_when_it_fits() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(
        !has(&harness, &format!("{LIST}/Box/Rows")),
        "a list that fits is not a scroll container"
    );
    let trigger = rect_of(&harness, PICK);
    let list = rect_of(&harness, LIST);
    assert_eq!(list.width(), trigger.width(), "the trigger is the wider");
    assert_eq!(list.min.y, trigger.max.y + 4.0, "four points below it");
}

mod tall {
    use super::*;

    fn opened() -> Harness<'static, Rig<App>> {
        let mut harness = harness();
        click_node(&mut harness, MANY);
        settle(&mut harness);
        harness
    }

    fn visible(harness: &Harness<'_, Rig<App>>, index: usize) -> bool {
        let rows = rect_of(harness, &format!("{MANY_LIST}/Box/Rows"));
        rows.contains_rect(rect_of(harness, &tall_row(index)))
    }

    #[test]
    fn a_long_list_near_the_window_bottom_flips_clamps_and_scrolls() {
        let harness = opened();
        let (trigger, list) = (rect_of(&harness, MANY), rect_of(&harness, MANY_LIST));
        let wanted = 50.0 * rect_of(&harness, &tall_row(1)).height();
        assert!(
            wanted > SCREEN.y,
            "precondition: the options want more than the window"
        );
        assert!(list.max.y <= trigger.min.y, "above the trigger: {list:?}");
        assert!(
            list.min.y >= 0.0 && list.height() < wanted,
            "clamped into the room: {list:?}"
        );
        assert!(visible(&harness, 0), "the chosen option is in view");
        assert!(
            !visible(&harness, 49),
            "the end of the list is scrolled off"
        );
    }

    #[test]
    fn the_highlight_scrolls_into_view_as_the_keys_move_it() {
        let mut harness = opened();
        harness.key_press(Key::Escape);
        settle(&mut harness);
        super::tab_to(&mut harness, MANY);
        press(&mut harness, Key::Enter);
        assert!(has(&harness, MANY_LIST), "precondition: open");
        press(&mut harness, Key::End);
        assert!(visible(&harness, 49), "End shows the last option");
        assert!(!visible(&harness, 0));
        let hover = Style::of(&harness.ctx).menu.item.paint.states.hover.fill;
        assert_eq!(
            support::fill_of(&harness, &tall_row(49)),
            Some(hover),
            "and it is the highlighted one"
        );
        press(&mut harness, Key::Home);
        assert!(visible(&harness, 0));
        for _ in 0..40 {
            harness.key_press(Key::ArrowDown);
        }
        settle(&mut harness);
        assert!(visible(&harness, 40), "the highlight stays in view");
        press(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.many, 40);
        assert!(!has(&harness, MANY_LIST));
    }

    #[test]
    fn the_chosen_option_far_down_the_list_is_in_view_when_it_opens() {
        let mut harness = harness();
        harness.state_mut().app.many = 45;
        settle(&mut harness);
        click_node(&mut harness, MANY);
        settle(&mut harness);
        assert!(visible(&harness, 45));
    }
}

mod dialog {
    use super::*;

    fn opened() -> Harness<'static, Rig<App>> {
        let mut harness = harness();
        click_node(&mut harness, DIALOG);
        settle(&mut harness);
        assert!(harness.state().app.dialog, "precondition");
        harness
    }

    const INNER: &str = r#"/"dialog"/"Box"/"inner""#;
    const INNER_LIST: &str = r#"/"dialog"/"Box"/("inner", List)"#;

    #[test]
    fn a_select_in_a_dialog_opens_above_the_trap_and_chooses() {
        let mut harness = opened();
        click_node(&mut harness, INNER);
        settle(&mut harness);
        assert!(has(&harness, &row(INNER_LIST, 0)), "the list is open");
        click_node(&mut harness, &row(INNER_LIST, 2));
        settle(&mut harness);
        assert_eq!(harness.state().app.inner, 2);
        assert!(
            harness.state().app.dialog,
            "choosing leaves the dialog open"
        );
        assert!(!has(&harness, &row(INNER_LIST, 0)), "the list closed");
    }

    #[test]
    fn escape_closes_the_list_first_and_then_the_dialog() {
        let mut harness = opened();
        click_node(&mut harness, INNER);
        settle(&mut harness);
        assert!(has(&harness, &row(INNER_LIST, 0)), "precondition: open");
        press(&mut harness, Key::Escape);
        assert!(!has(&harness, &row(INNER_LIST, 0)));
        assert!(harness.state().app.dialog, "the dialog stays");
        press(&mut harness, Key::Escape);
        assert!(!harness.state().app.dialog);
    }

    #[test]
    fn the_keyboard_drives_a_select_in_the_dialog() {
        let mut harness = opened();
        tab_to(&mut harness, INNER);
        press(&mut harness, Key::Enter);
        assert!(has(&harness, &row(INNER_LIST, 0)), "precondition: open");
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.inner, 2);
        assert_eq!(focused_path(&harness).as_deref(), Some(INNER));
    }
}

#[test]
fn clicking_the_trigger_again_closes_the_list() {
    let mut harness = harness();
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(open(&harness), "precondition");
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(!open(&harness), "the second click closes it");
    click_node(&mut harness, PICK);
    settle(&mut harness);
    assert!(open(&harness), "and a third opens it again");
}

/// Two frames laid out in one context each pass, as the examples' shell does
/// with its theme bar and its page: the bar declares no overlay.
mod beside_another_frame {
    use super::*;
    use eggshell_ui::LayoutState;

    struct Shell {
        bar: LayoutState,
        page: LayoutState,
        app: App,
    }

    fn harness() -> Harness<'static, Shell> {
        let mut harness = Harness::builder().with_size(SCREEN).build_ui_state(
            |ui, shell: &mut Shell| {
                if eggshell_ui_shadcn::tokens(ui.ctx()).is_none() {
                    eggshell_ui_shadcn::Shadcn::apply_both(ui.ctx());
                }
                let area = ui.max_rect();
                let Shell { bar, page, app } = shell;
                bar.build(ui.ctx(), |b| b.div(tw().h_px(10.0), |_| {}))
                    .layout(egui::Rect::from_min_size(
                        area.min,
                        vec2(area.width(), 10.0),
                    ))
                    .draw(app, ui);
                page.build(ui.ctx(), |b| view(b, &*app))
                    .layout(area)
                    .draw(app, ui);
            },
            Shell {
                bar: LayoutState::new().with_base_text(base_text()),
                page: LayoutState::new().with_base_text(base_text()),
                app: App::default(),
            },
        );
        harness.run_steps(4);
        harness
    }

    #[test]
    fn the_highlight_survives_the_other_frames_judgement() {
        let mut harness = harness();
        harness
            .get_by_role_and_label(Role::ComboBox, "Pick")
            .click();
        harness.run_steps(6);
        assert!(
            harness
                .query_by_role_and_label(Role::ListBoxOption, "Asia Pacific")
                .is_some(),
            "precondition: the list is open"
        );
        harness.key_press(Key::ArrowDown);
        harness.run_steps(6);
        harness.key_press(Key::Enter);
        harness.run_steps(6);
        assert_eq!(harness.state().app.changes, [2]);
    }
}

#[test]
fn the_list_pops_in_with_the_styles_menu_motion_and_leaves_the_same_way() {
    use eggshell_ui::testing::{Motion, record};

    let mut harness = support::harness_stepped(SCREEN, 0.02, base_text(), App::default(), view);
    harness.run_steps(4);
    record(&harness.ctx);
    let motion = Style::of(&harness.ctx).motion.menu;
    assert!(
        !motion.enter.is_zero(),
        "precondition: the shadcn preset moves menus"
    );
    let moved = |harness: &Harness<'_, Rig<App>>| {
        let node = support::last_drawn(&harness.ctx, LIST)?;
        Some((node.painted_rect != node.layout_rect, node.motion))
    };

    click_node(&mut harness, PICK);
    harness.step();
    assert_eq!(
        moved(&harness),
        Some((true, Motion::Presents)),
        "scaled and nudged while it enters"
    );
    harness.run_steps(20);
    assert_eq!(moved(&harness).map(|moved| moved.0), Some(false), "settled");

    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert_eq!(
        moved(&harness).map(|moved| moved.0),
        Some(true),
        "scaled and nudged while it leaves"
    );
    harness.run_steps(20);
    assert!(
        harness
            .state()
            .nodes
            .iter()
            .all(|(path, _)| !path.starts_with(LIST)),
        "gone once the exit has run"
    );
}
