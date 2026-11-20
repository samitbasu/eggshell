//! The owned select on overlays, through the real path: kittest's pointer and
//! key events, build, layout, draw and the judgement of the frame's input.

use core::time::Duration;

use eggshell_ui::overlay::Modal;
use eggshell_ui::owned::{Choice, Enablement};
use eggshell_ui::testing::{After, Driver, Target};
use eggshell_ui::{Builder, Style, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, Role, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path};

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
            weight: Weight::Normal,
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

fn harness() -> Driver<App> {
    let mut d = support::driver(SCREEN, base_text(), App::default(), view).build();
    d.frames(4);
    d
}

fn has(d: &Driver<App>, path: &str) -> bool {
    support::nodes(d)
        .iter()
        .any(|(candidate, _)| candidate == path)
}

fn open(d: &Driver<App>) -> bool {
    has(d, &format!("{LIST}/Box"))
}

fn row(list: &str, index: usize) -> String {
    format!("{list}/Box/Option({index})")
}

fn tall_row(index: usize) -> String {
    format!("{MANY_LIST}/Box/Rows/Option({index})")
}

fn tab_to(d: &mut Driver<App>, path: &str) {
    for _ in 0..8 {
        if focused_path(d).as_deref() == Some(path) {
            return;
        }
        d.press(Key::Tab).frames(4);
    }
    panic!("could not tab to {path}");
}

/// The rows, among `0..count` of `list`, drawn in the item's highlight colours.
fn highlighted(d: &Driver<App>, list: &str, count: usize) -> Vec<usize> {
    let hover = Style::of(d.ctx()).menu.item.paint.states.hover.fill;
    (0..count)
        .filter(|&index| support::fill_of(d, &row(list, index)) == Some(hover))
        .collect()
}

#[test]
fn the_rows_are_eggshell_nodes_with_the_option_role() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    assert!(open(&d), "precondition: a click opens the list");
    for (index, (_, label)) in CONTINENTS.into_iter().enumerate() {
        let node = d
            .harness()
            .get_by_role_and_label(Role::ListBoxOption, label);
        assert!(
            d.rect(Target::suffix(&row(LIST, index)))
                .contains_rect(node.rect()),
            "{label} is the node at its path"
        );
    }
    let list = d.harness().get_by_role_and_label(Role::ListBox, "Pick");
    assert!(
        list.rect()
            .contains_rect(d.rect(Target::suffix(&row(LIST, 0))))
    );
    assert_eq!(
        d.harness()
            .get_by_role_and_label(Role::ListBoxOption, "Europe")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True),
        "the chosen option is selected"
    );
}

#[test]
fn the_trigger_says_whether_its_list_is_open() {
    let mut d = harness();
    let trigger = |d: &Driver<App>| {
        let node = d.harness().get_by_role_and_label(Role::ComboBox, "Pick");
        let data = node.accesskit_node().data().clone();
        (data.is_expanded(), data.has_popup())
    };
    assert_eq!(
        trigger(&d),
        (Some(false), Some(egui::accesskit::HasPopup::Listbox))
    );
    d.click(Target::suffix(PICK)).frames(5);
    assert_eq!(
        trigger(&d),
        (Some(true), Some(egui::accesskit::HasPopup::Listbox))
    );
}

#[test]
fn the_list_opens_with_the_chosen_option_highlighted() {
    let mut d = harness();
    d.app_mut().pick = 2;
    d.frames(5);
    d.click(Target::suffix(PICK)).frames(5);
    assert!(open(&d), "precondition");
    assert_eq!(highlighted(&d, LIST, 4), [2]);
}

#[test]
fn arrow_up_opens_the_list_as_down_does() {
    let mut d = harness();
    tab_to(&mut d, PICK);
    d.press(Key::ArrowUp).frames(4);
    assert!(open(&d));
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(PICK),
        "focus stays on the trigger"
    );
}

#[test]
fn a_list_opened_with_the_pointer_is_chosen_with_the_keyboard() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    assert!(open(&d), "precondition");
    assert_eq!(focused_path(&d), None, "precondition: nothing focused");
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(
        focused_path(&d),
        None,
        "the arrow moved the highlight, not egui's focus"
    );
    d.press(Key::Enter).frames(4);
    assert_eq!(d.app().changes, [2], "the disabled option is skipped");
    assert!(!open(&d));
}

#[test]
fn only_one_option_looks_highlighted_whether_the_pointer_or_the_keys_moved_it() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    let target = d.rect(Target::suffix(&row(LIST, 3))).center();
    d.hover(target).frames(5);
    assert_eq!(highlighted(&d, LIST, 4), [3], "the pointer moved it");
    d.press(Key::ArrowUp).frames(4);
    assert_eq!(
        highlighted(&d, LIST, 4),
        [2],
        "the keys moved it, though the pointer is still over option 3"
    );
}

#[test]
fn choosing_with_the_pointer_leaves_focus_on_the_trigger() {
    let mut d = harness();
    tab_to(&mut d, PICK);
    d.press(Key::Enter).frames(4);
    assert!(open(&d), "precondition");
    d.pointer_down(Target::suffix(&row(LIST, 2))).frames(11);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(PICK),
        "held through the press"
    );
    d.pointer_up(Target::suffix(&row(LIST, 2))).frames(11);
    assert_eq!(d.app().changes, [2]);
    assert_eq!(focused_path(&d).as_deref(), Some(PICK));
}

#[test]
fn the_arrows_stop_at_the_ends_of_the_list() {
    let mut d = harness();
    tab_to(&mut d, PICK);
    d.press(Key::Enter).frames(4);
    d.press(Key::End).frames(4);
    d.press(Key::ArrowDown).frames(4);
    assert_eq!(highlighted(&d, LIST, 4), [3]);
    d.press(Key::Home).frames(4);
    d.press(Key::ArrowUp).frames(4);
    assert_eq!(highlighted(&d, LIST, 4), [0]);
}

#[test]
fn a_disabled_option_cannot_be_clicked() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    d.click(Target::suffix(&row(LIST, 1))).frames(5);
    assert!(d.app().changes.is_empty());
    assert!(open(&d), "the list stays open");
}

#[test]
fn the_press_that_closes_the_list_is_swallowed() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    let other = d.rect(Target::suffix(OTHER));
    assert!(
        !d.rect(Target::suffix(LIST)).intersects(other),
        "precondition: the button is clear of the list"
    );
    d.click(Target::suffix(OTHER)).frames(5);
    assert!(!open(&d), "the press closed the list");
    assert_eq!(d.app().clicks, 0, "and the button never saw it");
    d.click(Target::suffix(OTHER)).frames(5);
    assert_eq!(d.app().clicks, 1, "control: a press now reaches it");
}

#[test]
fn the_list_is_as_wide_as_its_options_and_has_no_scroll_gutter_when_it_fits() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    assert!(
        !has(&d, &format!("{LIST}/Box/Rows")),
        "a list that fits is not a scroll container"
    );
    let trigger = d.rect(Target::suffix(PICK));
    let list = d.rect(Target::suffix(LIST));
    assert_eq!(list.width(), trigger.width(), "the trigger is the wider");
    assert_eq!(list.min.y, trigger.max.y + 4.0, "four points below it");
}

mod tall {
    use super::*;

    fn opened() -> Driver<App> {
        let mut d = harness();
        d.click(Target::suffix(MANY)).frames(5);
        d
    }

    fn visible(d: &Driver<App>, index: usize) -> bool {
        let rows = d.rect(Target::suffix(&format!("{MANY_LIST}/Box/Rows")));
        rows.contains_rect(d.rect(Target::suffix(&tall_row(index))))
    }

    #[test]
    fn a_long_list_near_the_window_bottom_flips_clamps_and_scrolls() {
        let d = opened();
        let (trigger, list) = (
            d.rect(Target::suffix(MANY)),
            d.rect(Target::suffix(MANY_LIST)),
        );
        let wanted = 50.0 * d.rect(Target::suffix(&tall_row(1))).height();
        assert!(
            wanted > SCREEN.y,
            "precondition: the options want more than the window"
        );
        assert!(list.max.y <= trigger.min.y, "above the trigger: {list:?}");
        assert!(
            list.min.y >= 0.0 && list.height() < wanted,
            "clamped into the room: {list:?}"
        );
        assert!(visible(&d, 0), "the chosen option is in view");
        assert!(!visible(&d, 49), "the end of the list is scrolled off");
    }

    #[test]
    fn the_highlight_scrolls_into_view_as_the_keys_move_it() {
        let mut d = opened();
        d.press(Key::Escape).frames(4);
        super::tab_to(&mut d, MANY);
        d.press(Key::Enter).frames(4);
        assert!(has(&d, MANY_LIST), "precondition: open");
        d.press(Key::End).frames(4);
        assert!(visible(&d, 49), "End shows the last option");
        assert!(!visible(&d, 0));
        let hover = Style::of(d.ctx()).menu.item.paint.states.hover.fill;
        assert_eq!(
            support::fill_of(&d, &tall_row(49)),
            Some(hover),
            "and it is the highlighted one"
        );
        d.press(Key::Home).frames(4);
        assert!(visible(&d, 0));
        for _ in 0..40 {
            d.press(Key::ArrowDown);
        }
        d.frames(4);
        assert!(visible(&d, 40), "the highlight stays in view");
        d.press(Key::Enter).frames(4);
        assert_eq!(d.app().many, 40);
        assert!(!has(&d, MANY_LIST));
    }

    #[test]
    fn the_chosen_option_far_down_the_list_is_in_view_when_it_opens() {
        let mut d = harness();
        d.app_mut().many = 45;
        d.frames(5);
        d.click(Target::suffix(MANY)).frames(5);
        assert!(visible(&d, 45));
    }
}

mod dialog {
    use super::*;

    fn opened() -> Driver<App> {
        let mut d = harness();
        d.click(Target::suffix(DIALOG)).frames(5);
        assert!(d.app().dialog, "precondition");
        d
    }

    const INNER: &str = r#"/"dialog"/"Box"/"inner""#;
    const INNER_LIST: &str = r#"/"dialog"/"Box"/("inner", List)"#;

    #[test]
    fn a_select_in_a_dialog_opens_above_the_trap_and_chooses() {
        let mut d = opened();
        d.click(Target::suffix(INNER)).frames(5);
        assert!(has(&d, &row(INNER_LIST, 0)), "the list is open");
        d.click(Target::suffix(&row(INNER_LIST, 2))).frames(5);
        assert_eq!(d.app().inner, 2);
        assert!(d.app().dialog, "choosing leaves the dialog open");
        assert!(!has(&d, &row(INNER_LIST, 0)), "the list closed");
    }

    #[test]
    fn escape_closes_the_list_first_and_then_the_dialog() {
        let mut d = opened();
        d.click(Target::suffix(INNER)).frames(5);
        assert!(has(&d, &row(INNER_LIST, 0)), "precondition: open");
        d.press(Key::Escape).frames(4);
        assert!(!has(&d, &row(INNER_LIST, 0)));
        assert!(d.app().dialog, "the dialog stays");
        d.press(Key::Escape).frames(4);
        assert!(!d.app().dialog);
    }

    #[test]
    fn the_keyboard_drives_a_select_in_the_dialog() {
        let mut d = opened();
        tab_to(&mut d, INNER);
        d.press(Key::Enter).frames(4);
        assert!(has(&d, &row(INNER_LIST, 0)), "precondition: open");
        d.press(Key::ArrowDown).frames(4);
        d.press(Key::Enter).frames(4);
        assert_eq!(d.app().inner, 2);
        assert_eq!(focused_path(&d).as_deref(), Some(INNER));
    }
}

#[test]
fn clicking_the_trigger_again_closes_the_list() {
    let mut d = harness();
    d.click(Target::suffix(PICK)).frames(5);
    assert!(open(&d), "precondition");
    d.click(Target::suffix(PICK)).frames(5);
    assert!(!open(&d), "the second click closes it");
    d.click(Target::suffix(PICK)).frames(5);
    assert!(open(&d), "and a third opens it again");
}

/// Two frames laid out in one context each pass, as the examples' shell does
/// with its theme bar and its page: the bar declares no overlay.
mod beside_another_frame {
    use super::*;
    use eggshell_ui::LayoutState;

    /// kittest's own step, a quarter of a second, and its 4 settling steps.
    const STEP: Duration = Duration::from_millis(250);

    fn harness() -> Driver<App> {
        let mut bar = LayoutState::new().with_base_text(base_text());
        let mut d = Driver::whole_frame(App::default(), move |ui, app, page| {
            if eggshell_ui_shadcn::tokens(ui.ctx()).is_none() {
                eggshell_ui_shadcn::Shadcn::apply_both(ui.ctx());
            }
            let area = ui.max_rect();
            bar.build(ui.ctx(), |b| b.div(tw().h_px(10.0), |_| {}))
                .layout(egui::Rect::from_min_size(
                    area.min,
                    vec2(area.width(), 10.0),
                ))
                .draw(app, ui);
            page.build(ui.ctx(), |b| view(b, &*app))
                .layout(area)
                .draw(app, ui)
        })
        .size(SCREEN)
        .base_text(base_text())
        .step(STEP)
        .after(After::Frames(0))
        .settle_within(STEP * 4)
        .build();
        d.frames(4);
        d
    }

    #[test]
    fn the_highlight_survives_the_other_frames_judgement() {
        let mut d = harness();
        d.harness()
            .get_by_role_and_label(Role::ComboBox, "Pick")
            .click();
        d.frames(6);
        assert!(
            d.harness()
                .query_by_role_and_label(Role::ListBoxOption, "Asia Pacific")
                .is_some(),
            "precondition: the list is open"
        );
        d.press(Key::ArrowDown).frames(5);
        d.press(Key::Enter).frames(5);
        assert_eq!(d.app().changes, [2]);
    }
}

#[test]
fn the_list_pops_in_with_the_styles_menu_motion_and_leaves_the_same_way() {
    use eggshell_ui::testing::{Motion, record};

    let step = Duration::from_millis(20);
    let mut d = support::driver(SCREEN, base_text(), App::default(), view)
        .step(step)
        .settle_within(step * 4)
        .build();
    d.frames(4);
    record(d.ctx());
    let motion = Style::of(d.ctx()).motion.menu;
    assert!(
        !motion.enter.is_zero(),
        "precondition: the shadcn preset moves menus"
    );
    let moved = |d: &Driver<App>| {
        let node = support::last_drawn(d.ctx(), LIST)?;
        Some((node.painted_rect != node.layout_rect, node.motion))
    };

    d.click(Target::suffix(PICK)).frames(1);
    assert_eq!(
        moved(&d),
        Some((true, Motion::Presents)),
        "scaled and nudged while it enters"
    );
    d.frames(20);
    assert_eq!(moved(&d).map(|moved| moved.0), Some(false), "settled");

    d.press(Key::Escape).frames(2);
    assert_eq!(
        moved(&d).map(|moved| moved.0),
        Some(true),
        "scaled and nudged while it leaves"
    );
    d.frames(20);
    assert!(
        support::nodes(&d)
            .iter()
            .all(|(path, _)| !path.starts_with(LIST)),
        "gone once the exit has run"
    );
}
