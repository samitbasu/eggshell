//! Mounts: a view over part of the application data, built into the page's
//! layout. What a mount declares reaches its data through the lens; two
//! mounts over one field share it; a mount's commands are live while focus is
//! within it, ahead of the page's.

use eggshell_ui::prim::text_field::Field;
use eggshell_ui::{Builder, Command, Commands, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, KeyboardShortcut, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, click_node, focused_path, rect_of};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Page,
    Before,
    After,
    Left,
    Right,
    Other,
    Fixed,
    Notes,
    Note,
    Rows,
    List,
    Row(u32),
    Nest,
    Inner,
    Add,
    More,
    Reset,
}

#[derive(Default)]
struct Counter {
    count: u32,
}

#[derive(Default)]
struct Notes {
    text: String,
    log: Vec<&'static str>,
}

#[derive(Default)]
struct Rows {
    rows: Vec<u32>,
    picked: Option<u32>,
}

#[derive(Default)]
struct Nest {
    inner: Counter,
}

#[derive(Default)]
struct App {
    shared: Counter,
    other: Counter,
    notes: Notes,
    rows: Rows,
    nest: Nest,
    log: Vec<&'static str>,
}

const fn chord(modifiers: Modifiers, key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(modifiers, key)
}

const SHARED: KeyboardShortcut = chord(Modifiers::COMMAND, Key::K);
const PAGE_ONLY: KeyboardShortcut = chord(Modifiers::COMMAND, Key::J);

fn page_commands() -> Commands<App> {
    Commands::new()
        .add(
            "page k",
            Command::new("Page K", |app: &mut App| app.log.push("page k")).shortcut(SHARED),
        )
        .add(
            "page j",
            Command::new("Page J", |app: &mut App| app.log.push("page j")).shortcut(PAGE_ONLY),
        )
}

fn notes_commands() -> Commands<Notes> {
    Commands::new().add(
        "notes k",
        Command::new("Notes K", |notes: &mut Notes| notes.log.push("notes k")).shortcut(SHARED),
    )
}

fn counter<'f>(b: &mut Builder<'_, 'f, Counter>, counter: &'f Counter) {
    b.div(tw().flex().flex_row().gap(2.0), |b| {
        b.button(Part::Add, "Add")
            .on_click(|counter: &mut Counter| counter.count += 1);
        b.text(tw(), format!("Count {}", counter.count));
        b.dropdown_menu(Part::More, "More").items(|m| {
            m.item(Part::Reset, "Reset")
                .on_click(|counter: &mut Counter| counter.count = 0);
        });
    });
}

fn notes<'f>(b: &mut Builder<'_, 'f, Notes>, notes: &'f Notes) {
    b.commands(&notes_commands(), notes);
    b.text_field_with(Part::Note, Field::singleline(&notes.text).named("Note"))
        .classes(tw().w_px(160.0))
        .on_change(|notes: &mut Notes, text| notes.text = text);
}

fn rows<'f>(b: &mut Builder<'_, 'f, Rows>, rows: &'f Rows) {
    b.virtual_list(Part::List, tw().w_px(160.0).h_px(60.0), &rows.rows)
        .fixed_px(24.0)
        .items(|s, row| {
            s.button(Part::Row(row), format!("Row {row}"))
                .on_click(move |rows: &mut Rows| rows.picked = Some(row));
        });
}

fn nest<'f>(b: &mut Builder<'_, 'f, Nest>, nest: &'f Nest) {
    b.mount(
        Part::Inner,
        tw(),
        &nest.inner,
        |nest: &mut Nest| &mut nest.inner,
        counter,
    );
}

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.commands(&page_commands(), app);
    b.div_id(Part::Page, tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.button(Part::Before, "Before").on_click(|_: &mut App| {});
        s.mount(
            Part::Left,
            tw(),
            &app.shared,
            |app: &mut App| &mut app.shared,
            counter,
        );
        s.mount(
            Part::Right,
            tw(),
            &app.shared,
            |app: &mut App| &mut app.shared,
            counter,
        );
        s.mount(
            Part::Other,
            tw(),
            &app.other,
            |app: &mut App| &mut app.other,
            counter,
        );
        s.mount(
            Part::Fixed,
            tw().h_px(48.0),
            &app.other,
            |app: &mut App| &mut app.other,
            counter,
        );
        s.mount(
            Part::Notes,
            tw(),
            &app.notes,
            |app: &mut App| &mut app.notes,
            notes,
        );
        s.mount(
            Part::Rows,
            tw(),
            &app.rows,
            |app: &mut App| &mut app.rows,
            rows,
        );
        s.mount(
            Part::Nest,
            tw(),
            &app.nest,
            |app: &mut App| &mut app.nest,
            nest,
        );
        s.button(Part::After, "After").on_click(|_: &mut App| {});
    });
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

fn harness() -> Harness<'static, Rig<App>> {
    let app = App {
        rows: Rows {
            rows: (0..200).collect(),
            picked: None,
        },
        ..App::default()
    };
    let mut harness = support::harness(vec2(480.0, 720.0), base_text(), app, view);
    harness.run_steps(4);
    harness
}

fn path(parts: &[&str]) -> String {
    parts.iter().map(|part| format!("/{part}")).collect()
}

fn press(harness: &mut Harness<'_, Rig<App>>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run_steps(4);
}

#[test]
fn a_mounted_control_changes_the_data_its_lens_reaches() {
    let mut harness = harness();
    click_node(&mut harness, &path(&["Page", "Other", "Add"]));
    harness.run_steps(2);
    let app = &harness.state().app;
    assert_eq!((app.other.count, app.shared.count), (1, 0));
}

#[test]
fn two_mounts_over_one_field_share_it() {
    let mut harness = harness();
    click_node(&mut harness, &path(&["Page", "Left", "Add"]));
    click_node(&mut harness, &path(&["Page", "Right", "Add"]));
    harness.run_steps(2);
    assert_eq!(harness.state().app.shared.count, 2);
    assert_eq!(
        harness.query_all_by_label("Count 2").count(),
        2,
        "both mounts show the shared count"
    );
}

#[test]
fn a_mount_is_as_tall_as_its_content_unless_its_classes_say() {
    let harness = harness();
    let mount = rect_of(&harness, &path(&["Page", "Other"]));
    let add = rect_of(&harness, &path(&["Page", "Other", "Add"]));
    assert!(add.height() > 0.0, "precondition: the button is laid out");
    assert_eq!(mount.height(), add.height());
    assert_eq!(rect_of(&harness, &path(&["Page", "Fixed"])).height(), 48.0);
}

#[test]
fn a_mounted_menu_item_changes_the_mounted_data() {
    let mut harness = harness();
    click_node(&mut harness, &path(&["Page", "Other", "Add"]));
    click_node(&mut harness, &path(&["Page", "Other", "More"]));
    harness.run_steps(6);
    harness.get_by_label("Reset").click();
    harness.run_steps(6);
    assert_eq!(harness.state().app.other.count, 0);
    assert!(
        harness.query_by_label("Reset").is_none(),
        "the menu closed on its item"
    );
}

#[test]
fn escape_closes_a_mounted_menu() {
    let mut harness = harness();
    click_node(&mut harness, &path(&["Page", "Other", "More"]));
    harness.run_steps(6);
    assert!(
        harness.query_by_label("Reset").is_some(),
        "precondition: open"
    );
    press(&mut harness, Modifiers::NONE, Key::Escape);
    harness.run_steps(6);
    assert!(harness.query_by_label("Reset").is_none());
}

#[test]
fn a_mounted_virtual_list_builds_its_rows_under_the_mount() {
    let mut harness = harness();
    harness.get_by_label("Row 1").click();
    harness.run_steps(4);
    assert_eq!(harness.state().app.rows.picked, Some(1));
    assert!(
        harness.query_by_label("Row 199").is_none(),
        "precondition: rows out of view are not built"
    );
}

/// The documentation page's shape: a virtual list of blocks, some of which
/// mount an app, built only near the viewport.
mod in_a_list {
    use super::*;

    #[derive(Default)]
    struct Page {
        blocks: Vec<u32>,
        counters: Vec<Counter>,
    }

    fn view<'f>(b: &mut Builder<'_, 'f, Page>, page: &'f Page) {
        b.virtual_list(Part::List, tw().w_px(200.0).h_px(120.0), &page.blocks)
            .fixed_px(40.0)
            .overscan_px(0.0)
            .items(move |s, block| {
                let at = block as usize;
                if let Some(counter) = page.counters.get(at) {
                    s.mount(
                        Part::Row(block),
                        tw(),
                        counter,
                        move |page: &mut Page| &mut page.counters[at],
                        super::counter,
                    );
                }
            });
    }

    #[test]
    fn a_mount_in_a_row_is_built_near_the_viewport_and_keeps_its_data() {
        let page = Page {
            blocks: (0..50).collect(),
            counters: (0..50).map(|_| Counter::default()).collect(),
        };
        let mut harness = support::harness(vec2(300.0, 300.0), base_text(), page, view);
        harness.run_steps(4);
        assert!(
            harness.query_all_by_label("Add").count() < 10,
            "precondition: only the rows near the viewport are built"
        );
        click_node(&mut harness, "/List/<rows>/0/Row(0)/Add");
        harness.run_steps(2);
        assert_eq!(harness.state().app.counters[0].count, 1);
        assert_eq!(harness.query_all_by_label("Count 1").count(), 1);
    }
}

#[test]
fn a_mount_inside_a_mount_reaches_its_data_through_both_lenses() {
    let mut harness = harness();
    click_node(&mut harness, &path(&["Page", "Nest", "Inner", "Add"]));
    harness.run_steps(2);
    assert_eq!(harness.state().app.nest.inner.count, 1);
}

#[test]
fn typing_in_a_mounted_field_changes_the_mounted_data() {
    let mut harness = harness();
    click_node(&mut harness, &path(&["Page", "Notes", "Note"]));
    harness.run_steps(4);
    harness.get_by_label("Note").type_text("hi");
    harness.run_steps(4);
    assert_eq!(harness.state().app.notes.text, "hi");
}

mod commands {
    use super::*;

    fn focus_the_note(harness: &mut Harness<'_, Rig<App>>) {
        click_node(harness, &path(&["Page", "Notes", "Note"]));
        harness.run_steps(6);
        assert_eq!(
            focused_path(harness).as_deref(),
            Some("<widget>"),
            "precondition: the field has focus"
        );
    }

    #[test]
    fn a_chord_both_have_goes_to_the_mount_while_focus_is_within_it() {
        let mut harness = harness();
        focus_the_note(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::K);
        let app = &harness.state().app;
        assert_eq!(
            (app.notes.log.as_slice(), app.log.as_slice()),
            (&["notes k"][..], &[][..])
        );
    }

    #[test]
    fn a_chord_only_the_page_has_falls_through_from_the_mount() {
        let mut harness = harness();
        focus_the_note(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::J);
        assert_eq!(harness.state().app.log, ["page j"]);
    }

    #[test]
    fn a_chord_both_have_goes_to_the_page_while_focus_is_outside_the_mount() {
        let mut harness = harness();
        click_node(&mut harness, &path(&["Page", "Before"]));
        harness.run_steps(4);
        press(&mut harness, Modifiers::COMMAND, Key::K);
        let app = &harness.state().app;
        assert_eq!(
            (app.notes.log.as_slice(), app.log.as_slice()),
            (&[][..], &["page k"][..])
        );
    }
}

#[test]
fn tab_moves_into_a_mount_and_out_of_it() {
    let mut harness = harness();
    support::past_bar(&mut harness);
    let mut seen = Vec::new();
    for _ in 0..12 {
        press(&mut harness, Modifiers::NONE, Key::Tab);
        seen.extend(focused_path(&harness));
    }
    let at = |wanted: &str| seen.iter().position(|path| path == wanted);
    let before = at(&path(&["Page", "Before"])).expect("Tab reaches Before");
    let add = at(&path(&["Page", "Left", "Add"])).expect("Tab reaches the mount");
    let more = at(&path(&["Page", "Left", "More"])).expect("Tab moves on in the mount");
    let next = at(&path(&["Page", "Right", "Add"])).expect("Tab leaves the mount");
    assert!(before < add && add < more && more < next, "{seen:?}");
}

/// Misuse inside and around a mount is reported, and the frame goes on.
mod misuse {
    use eggshell_ui::Problem;
    use eggshell_ui::testing::{Headless, assert_reported, reports};

    use super::*;

    #[derive(Default)]
    struct Two {
        first: Notes,
        second: Notes,
    }

    fn twice<'f>(b: &mut Builder<'_, 'f, Two>, two: &'f Two) {
        b.mount("m", tw(), &two.first, |two: &mut Two| &mut two.first, notes);
        b.mount(
            "m",
            tw(),
            &two.second,
            |two: &mut Two| &mut two.second,
            notes,
        );
    }

    #[test]
    fn a_mount_key_used_twice_is_reported_and_each_mount_keeps_its_own_commands() {
        let mut harness = support::harness(vec2(400.0, 300.0), base_text(), Two::default(), twice);
        harness.run_steps(4);
        assert_reported(&harness.ctx, Problem::DuplicateKey);
        let fields: Vec<_> = harness.query_all_by_label("Note").collect();
        assert_eq!(fields.len(), 2, "precondition: both mounts are built");
        fields[1].click();
        harness.run_steps(6);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
        harness.run_steps(4);
        let two = &harness.state().app;
        assert_eq!(
            (two.first.log.as_slice(), two.second.log.as_slice()),
            (&[][..], &["notes k"][..])
        );
    }

    fn repeats<'f>(b: &mut Builder<'_, 'f, Counter>, _: &'f Counter) {
        b.div_id("x", tw(), |_| {});
        b.div_id("x", tw(), |_| {});
    }

    #[test]
    fn a_duplicate_key_inside_a_mount_is_reported_under_the_mount_key() {
        let mut headless = Headless::new(vec2(200.0, 100.0));
        let page = Nest::default();
        headless.layout(&page, |b, page| {
            b.mount(
                "m",
                tw(),
                &page.inner,
                |page: &mut Nest| &mut page.inner,
                repeats,
            );
        });
        let report = assert_reported(headless.ctx(), Problem::DuplicateKey);
        assert!(report.message.contains(r#"/"m"/"x""#), "{}", report.message);
    }

    fn notes_twice<'f>(b: &mut Builder<'_, 'f, Notes>, notes: &'f Notes) {
        let second = Commands::new().add(
            "second k",
            Command::new("Second K", |notes: &mut Notes| notes.log.push("second k"))
                .shortcut(SHARED),
        );
        b.commands(&notes_commands(), notes);
        b.commands(&second, notes);
        b.text_field_with(Part::Note, Field::singleline(&notes.text).named("Note"))
            .on_change(|notes: &mut Notes, text| notes.text = text);
    }

    #[test]
    fn commands_declared_twice_in_a_mount_are_reported_and_the_first_runs() {
        fn view<'f>(b: &mut Builder<'_, 'f, Notes>, notes: &'f Notes) {
            b.mount("m", tw(), notes, |notes: &mut Notes| notes, notes_twice);
        }
        let mut harness = support::harness(vec2(400.0, 300.0), base_text(), Notes::default(), view);
        harness.run_steps(4);
        assert!(
            reports(&harness.ctx)
                .iter()
                .any(|report| report.problem == Problem::CommandsTwice),
            "reported"
        );
        harness.get_by_label("Note").click();
        harness.run_steps(6);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
        harness.run_steps(4);
        assert_eq!(harness.state().app.log, ["notes k"]);
    }
}
