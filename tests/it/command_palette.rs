//! The extras `command` feature through the real path: the example's
//! palette, combobox and autosuggest, driven by kittest's keys and pointer,
//! read back through the accessibility tree.

#[path = "../../examples/command_palette.rs"]
#[allow(dead_code)]
mod example;

use eggshell_ui::{TextSpec, TextStyle, Weight};
use egui::accesskit::{Role, Toggled};
use egui::{Color32, FontFamily, Key, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, Seen as _};
use example::State;

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

fn harness() -> Harness<'static, Rig<State>> {
    let mut harness = support::harness(
        vec2(800.0, 700.0),
        base_text(),
        State::default(),
        example::layout,
    );
    harness.run_steps(4);
    harness
}

fn step(harness: &mut Harness<'_, Rig<State>>) {
    harness.run_steps(6);
}

fn press(harness: &mut Harness<'_, Rig<State>>, key: Key) {
    harness.key_press(key);
    step(harness);
}

fn open_palette(harness: &mut Harness<'_, Rig<State>>) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run_steps(20);
    assert!(
        harness.state().app.palette,
        "precondition: Ctrl+K opened it"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Dialog, "Command palette")
            .is_some(),
        "precondition: the palette is showing"
    );
}

/// The label of the item the field of the combobox named `name` announces
/// as its active descendant.
fn active(harness: &Harness<'_, Rig<State>>, name: &str) -> Option<String> {
    harness
        .get_by_role_and_label(Role::ComboBox, name)
        .accesskit_node()
        .active_descendant()
        .and_then(|node| node.label())
}

fn options(harness: &Harness<'_, Rig<State>>) -> Vec<String> {
    harness
        .query_all_by_role(Role::ListBoxOption)
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

#[test]
fn ctrl_k_opens_the_palette_with_its_field_focused_and_the_first_item_highlighted() {
    let mut harness = harness();
    open_palette(&mut harness);
    assert_eq!(
        support::focused_path(&harness).as_deref(),
        Some("<widget>"),
        "the search field's text edit has the focus"
    );
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Getting started")
    );
}

#[test]
fn typing_filters_by_word_prefix_and_keywords() {
    let mut harness = harness();
    open_palette(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("lay");
    step(&mut harness);
    assert_eq!(options(&harness), ["Layout"]);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("x");
    step(&mut harness);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("dialog");
    step(&mut harness);
    assert_eq!(options(&harness), ["Overlays"], "a keyword finds its item");
}

#[test]
fn nothing_matching_shows_the_empty_state() {
    let mut harness = harness();
    open_palette(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("zzz");
    step(&mut harness);
    assert!(options(&harness).is_empty());
    assert!(harness.query_by_label("No results found.").is_some());
}

#[test]
fn enter_picks_the_highlighted_item_and_closes_the_palette() {
    let mut harness = harness();
    open_palette(&mut harness);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Layout")
    );
    press(&mut harness, Key::Enter);
    harness.run_steps(20);
    assert_eq!(harness.state().app.said, "Opened Layout");
    assert!(!harness.state().app.palette, "a pick closes the palette");
}

#[test]
fn a_picked_command_runs() {
    let mut harness = harness();
    open_palette(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("show");
    step(&mut harness);
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Show grid")
    );
    press(&mut harness, Key::Enter);
    assert!(harness.state().app.grid);
    assert!(!harness.state().app.palette);
}

#[test]
fn a_checked_command_says_so_and_a_disabled_one_is_passed_by() {
    let mut harness = harness();
    harness.state_mut().app.grid = true;
    open_palette(&mut harness);
    let grid = harness.get_by_role_and_label(Role::ListBoxOption, "Show grid");
    assert_eq!(grid.accesskit_node().toggled(), Some(Toggled::True));
    let delete = harness.get_by_role_and_label(Role::ListBoxOption, "Delete");
    assert!(delete.accesskit_node().is_disabled());
    harness
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .type_text("show");
    step(&mut harness);
    assert_eq!(
        active(&harness, "Command palette").as_deref(),
        Some("Show grid")
    );
    harness.key_press(Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state().app.palette, "Escape closes the palette");
}

fn combobox<'a>(harness: &'a Harness<'_, Rig<State>>) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::ComboBox, "Framework")
}

#[test]
fn the_combobox_is_a_field_that_opens_on_a_click_filters_as_typed_and_picks() {
    let mut harness = harness();
    combobox(&harness).click();
    harness.run_steps(20);
    assert_eq!(
        support::focused_path(&harness).as_deref(),
        Some("<widget>"),
        "the field itself has the focus"
    );
    assert_eq!(
        options(&harness).len(),
        example::FRAMEWORKS.len(),
        "every option shows before anything is typed"
    );
    combobox(&harness).type_text("sv");
    step(&mut harness);
    assert_eq!(options(&harness), ["SvelteKit"]);
    assert_eq!(
        active(&harness, "Framework").as_deref(),
        Some("SvelteKit"),
        "the first match is highlighted"
    );
    press(&mut harness, Key::Enter);
    harness.run_steps(20);
    assert_eq!(
        harness.state().app.framework,
        Some(example_framework("SvelteKit"))
    );
    assert!(options(&harness).is_empty(), "a pick closes the list");
    assert_eq!(
        combobox(&harness).accesskit_node().value().as_deref(),
        Some("SvelteKit"),
        "the field shows the chosen label"
    );
}

/// The rects painted with a stroke in the focus ring's color.
fn ring_rects(harness: &Harness<'_, Rig<State>>) -> Vec<egui::Rect> {
    let ring = harness.view_shell().style().tokens.focus().color;
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Rect(shape) if shape.stroke.color == ring => Some(shape.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn the_combobox_box_wears_the_focus_ring_while_its_field_has_focus() {
    let mut harness = harness();
    assert!(
        ring_rects(&harness).is_empty(),
        "precondition: no focus, no ring"
    );
    combobox(&harness).focus();
    harness.run_steps(6);
    assert!(
        support::focused_path(&harness).is_some(),
        "precondition: the field has the focus"
    );
    assert!(
        !ring_rects(&harness).is_empty(),
        "the group's ring is drawn once the field has focus"
    );
}

#[test]
fn the_chevron_opens_the_list_and_gives_the_field_the_focus() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::Button, "Show options")
        .click();
    harness.run_steps(20);
    assert_eq!(options(&harness).len(), example::FRAMEWORKS.len());
    assert_eq!(support::focused_path(&harness).as_deref(), Some("<widget>"));
}

#[test]
fn escape_closes_the_list_and_puts_back_the_chosen_label() {
    let mut harness = harness();
    harness.state_mut().app.framework = Some(example_framework("SvelteKit"));
    harness.run_steps(4);
    combobox(&harness).click();
    harness.run_steps(20);
    combobox(&harness).type_text("x");
    step(&mut harness);
    harness.key_press(Key::Escape);
    harness.run_steps(20);
    assert!(options(&harness).is_empty(), "Escape closes the list");
    assert_eq!(
        combobox(&harness).accesskit_node().value().as_deref(),
        Some("SvelteKit"),
        "what was typed is dropped"
    );
}

#[test]
fn the_down_arrow_opens_a_closed_list() {
    let mut harness = harness();
    combobox(&harness).focus();
    step(&mut harness);
    assert!(options(&harness).is_empty(), "precondition: closed");
    press(&mut harness, Key::ArrowDown);
    harness.run_steps(10);
    assert_eq!(options(&harness).len(), example::FRAMEWORKS.len());
}

#[test]
fn a_chosen_value_can_be_cleared_with_the_clear_button() {
    let mut harness = harness();
    harness.state_mut().app.framework = Some(example_framework("SvelteKit"));
    harness.run_steps(4);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Show options")
            .is_none(),
        "the clear button takes the chevron's place"
    );
    harness.get_by_role_and_label(Role::Button, "Clear").click();
    harness.run_steps(20);
    assert_eq!(harness.state().app.framework, None);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Show options")
            .is_some(),
        "the chevron is back"
    );
}

fn example_framework(label: &str) -> example::Framework {
    example::FRAMEWORKS
        .iter()
        .find(|(_, name)| *name == label)
        .map(|(value, _)| *value)
        .expect("a framework of that name")
}

#[test]
fn the_autosuggest_suggests_without_a_highlight_and_takes_a_pick() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("b");
    step(&mut harness);
    assert_eq!(options(&harness), ["Berlin", "Bern"]);
    assert_eq!(active(&harness, "City"), None, "nothing is highlighted");
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness, "City").as_deref(), Some("Bern"));
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.city, "Bern");
    assert!(
        options(&harness).is_empty(),
        "a pick puts the suggestions away"
    );
}

#[test]
fn the_autosuggest_shows_nothing_when_nothing_matches() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("q");
    step(&mut harness);
    assert_eq!(harness.state().app.city, "q", "precondition: it typed");
    assert!(options(&harness).is_empty());
    assert!(harness.query_by_label("No results found.").is_none());
}

#[test]
fn escape_puts_the_suggestions_away_until_the_next_edit() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("b");
    step(&mut harness);
    assert!(!options(&harness).is_empty(), "precondition: suggesting");
    harness.key_press(Key::Escape);
    harness.run_steps(20);
    assert!(
        options(&harness).is_empty(),
        "focus after Escape: {:?}",
        support::focused_path(&harness)
    );
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .click();
    step(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "City")
        .type_text("e");
    step(&mut harness);
    assert_eq!(options(&harness), ["Berlin", "Bern"]);
}

fn results_height(harness: &Harness<'_, Rig<State>>) -> f32 {
    harness
        .state()
        .nodes
        .iter()
        .find(|(path, _)| path.ends_with("/Results"))
        .map_or(0.0, |(_, rect)| rect.height())
}

#[test]
fn the_list_is_as_tall_as_its_rows_up_to_the_style_limit() {
    let mut harness = harness();
    combobox(&harness).click();
    harness.run_steps(20);
    let few = results_height(&harness);
    let rows = example::FRAMEWORKS.len() as f32;
    assert!(
        (few - (rows * 32.0 + 8.0)).abs() < 1.0,
        "{rows} rows of 32 and the list's padding: {few}"
    );
    combobox(&harness).type_text("sv");
    step(&mut harness);
    assert!(
        results_height(&harness) < few,
        "a narrower query shortens the list"
    );
    harness.key_press(Key::Escape);
    harness.run_steps(10);
    open_palette(&mut harness);
    let height = results_height(&harness);
    assert!(
        (height - 288.0).abs() < 1.0,
        "a long list stops at max-h-72: {height}"
    );
}

#[test]
fn tabbing_away_from_the_field_closes_the_list() {
    let mut harness = harness();
    combobox(&harness).click();
    harness.run_steps(20);
    assert!(
        !options(&harness).is_empty(),
        "precondition: the list is open"
    );
    press(&mut harness, Key::Tab);
    harness.run_steps(10);
    assert!(options(&harness).is_empty(), "Tab closes the list");
    assert!(
        !combobox(&harness).accesskit_node().is_focused(),
        "precondition: the focus left the field"
    );
}

#[test]
fn the_lists_have_no_scrollbar_so_their_rows_are_as_wide_as_the_box() {
    let mut harness = harness();
    open_palette(&mut harness);
    let results = harness
        .state()
        .nodes
        .iter()
        .find(|(path, _)| path.ends_with("/Results"))
        .map(|(_, rect)| *rect)
        .expect("the palette's list");
    let rows = harness
        .state()
        .nodes
        .iter()
        .find(|(path, _)| path.ends_with("/Results/<rows>"))
        .map(|(_, rect)| *rect)
        .expect("the rows");
    let padding = 8.0;
    assert!(
        (rows.width() - (results.width() - padding)).abs() < 1.0,
        "the rows span the list less its padding, with no gutter: {} of {}",
        rows.width(),
        results.width()
    );
}

#[test]
fn the_highlight_keeps_four_points_from_the_lists_edges_as_the_arrows_scroll_it() {
    let mut harness = harness();
    open_palette(&mut harness);
    let results = |harness: &Harness<'_, Rig<State>>| {
        harness
            .state()
            .nodes
            .iter()
            .find(|(path, _)| path.ends_with("/Results"))
            .map(|(_, rect)| *rect)
            .expect("the palette's list")
    };
    let highlighted = |harness: &Harness<'_, Rig<State>>| {
        let label = active(harness, "Command palette").expect("a highlight");
        harness
            .get_by_role_and_label(Role::ListBoxOption, &label)
            .rect()
    };
    let rows = options(&harness).len();
    let mut nearest_below = f32::MAX;
    let mut nearest_above = f32::MAX;
    for _ in 0..rows + 3 {
        press(&mut harness, Key::ArrowDown);
        let (list, row) = (results(&harness), highlighted(&harness));
        nearest_below = nearest_below.min(list.max.y - row.max.y);
    }
    for _ in 0..rows + 3 {
        press(&mut harness, Key::ArrowUp);
        let (list, row) = (results(&harness), highlighted(&harness));
        nearest_above = nearest_above.min(row.min.y - list.min.y);
    }
    assert!(
        nearest_below >= 3.5,
        "going down, no highlighted row comes nearer the bottom than 4 pt: {nearest_below}"
    );
    assert!(
        nearest_above >= 3.5,
        "going up, no highlighted row comes nearer the top than 4 pt: {nearest_above}"
    );
}

/// A driver over the example with the palette open, its style edited by
/// `edit`.
fn palette_with(edit: impl Fn(&mut eggshell_ui::Style)) -> eggshell_ui::testing::Driver<State> {
    let mut d = support::driver(
        vec2(800.0, 700.0),
        base_text(),
        State::default(),
        example::layout,
    )
    .style(support::shadcn_with(edit))
    .theme(egui::Theme::Light)
    .build();
    d.frames(4);
    d.app_mut().palette = true;
    d.frames(20);
    d
}

/// The rect of the palette's node whose path ends with `suffix`.
fn palette_node(d: &eggshell_ui::testing::Driver<State>, suffix: &str) -> egui::Rect {
    support::nodes(d)
        .iter()
        .find(|(path, _)| path.contains("Dialog)/Box/") && path.ends_with(suffix))
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("no palette node ending {suffix}"))
}

/// The palette's highlighted command is shadcn's `data-selected:bg-muted`
/// (command.tsx; the audit's selection on #345).
#[test]
fn the_palettes_current_command_is_muted() {
    let d = palette_with(|_| {});
    let label = d
        .harness()
        .get_by_role_and_label(Role::ComboBox, "Command palette")
        .accesskit_node()
        .active_descendant()
        .and_then(|node| node.label())
        .expect("precondition: a current command");
    let row = d
        .harness()
        .get_by_role_and_label(Role::ListBoxOption, &label)
        .rect();
    let tokens = eggshell_ui_shadcn::Tokens::for_theme(egui::Theme::Light);
    assert_eq!(support::box_at(&d, row).fill, tokens.muted, "bg-muted");
}

/// The palette draws from the style's sheet: an App rule restyles its panel.
#[test]
fn an_app_rule_restyles_the_palette() {
    let d = palette_with(|style| {
        style
            .sheet
            .as_app()
            .rule(eggshell_ui_extras::command::CommandPart::Root, |t, _| {
                t.bg(Color32::RED)
            });
    });
    let results = palette_node(&d, "/Results");
    let root = support::nodes(&d)
        .iter()
        .filter(|(path, rect)| path.contains("Dialog)/Box/") && rect.contains_rect(results))
        .map(|(_, rect)| *rect)
        .find(|rect| *rect != results)
        .expect("the palette's root, around its list");
    assert_eq!(support::box_at(&d, root).fill, Color32::RED);
}

/// A group's heading is a row of its own, not one of the options; a
/// separator comes before every group but the first (look 4a, shadcn's
/// `CommandSeparator`).
#[test]
fn the_palettes_headings_are_not_options_and_separators_part_the_groups() {
    let d = palette_with(|_| {});
    let mut headings: Vec<(String, egui::Rect)> = d
        .harness()
        .query_all_by_role(Role::Heading)
        .filter_map(|node| Some((node.accesskit_node().label()?, node.rect())))
        .collect();
    headings.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    assert!(
        headings.len() >= 2,
        "precondition: two groups show: {headings:?}"
    );
    let options: Vec<String> = d
        .harness()
        .query_all_by_role(Role::ListBoxOption)
        .filter_map(|node| node.accesskit_node().label())
        .collect();
    for (heading, _) in &headings {
        assert!(!options.contains(heading), "{heading} is no option");
    }
    let rules: Vec<egui::Rect> = support::nodes(&d)
        .iter()
        .filter(|(path, rect)| path.contains("/Results/") && rect.height() == 1.0)
        .map(|(_, rect)| *rect)
        .collect();
    for (at, (heading, rect)) in headings.iter().enumerate() {
        // Within the row's height: shadcn's `-mx-1` runs it past the sides.
        let above = rules
            .iter()
            .filter(|rule| rule.min.y >= rect.min.y && rule.max.y <= rect.max.y)
            .count();
        let expected = usize::from(at > 0);
        assert_eq!(above, expected, "separators in {heading}'s row");
    }
}
