//! Mounts: a view over part of the application data, built into the page's
//! layout. What a mount declares reaches its data through the lens; two
//! mounts over one field share it; a mount's commands are live while focus is
//! within it, ahead of the page's.

use eggshell_ui::prim::text_field::Field;
use eggshell_ui::{Builder, Chord, Command, Commands, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use eggshell_ui::testing::{Driver, Target};
use egui::KeyboardShortcut;

use crate::support::{self, focused_path};

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

const fn chord(modifiers: Modifiers, key: Key) -> Chord {
    Chord::new(modifiers, key).expect("not a chord the library keeps")
}

const SHARED: Chord = chord(Modifiers::COMMAND, Key::K);
const PAGE_ONLY: Chord = chord(Modifiers::COMMAND, Key::J);

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
    )
    .add();
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
        )
        .add();
        s.mount(
            Part::Right,
            tw(),
            &app.shared,
            |app: &mut App| &mut app.shared,
            counter,
        )
        .add();
        s.mount(
            Part::Other,
            tw(),
            &app.other,
            |app: &mut App| &mut app.other,
            counter,
        )
        .add();
        s.mount(
            Part::Fixed,
            tw().h_px(48.0),
            &app.other,
            |app: &mut App| &mut app.other,
            counter,
        )
        .add();
        s.mount(
            Part::Notes,
            tw(),
            &app.notes,
            |app: &mut App| &mut app.notes,
            notes,
        )
        .add();
        s.mount(
            Part::Rows,
            tw(),
            &app.rows,
            |app: &mut App| &mut app.rows,
            rows,
        )
        .add();
        s.mount(
            Part::Nest,
            tw(),
            &app.nest,
            |app: &mut App| &mut app.nest,
            nest,
        )
        .add();
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

fn harness() -> Driver<App> {
    let app = App {
        rows: Rows {
            rows: (0..200).collect(),
            picked: None,
        },
        ..App::default()
    };
    let mut d = support::driver(vec2(480.0, 720.0), base_text(), app, view).build();
    d.frames(4);
    d
}

fn path(parts: &[&str]) -> String {
    parts.iter().map(|part| format!("/{part}")).collect()
}

/// Sends the modifiers' change events even for none, as kittest's
/// `key_press_modifiers` does.
fn press(d: &mut Driver<App>, modifiers: Modifiers, key: Key) {
    d.harness().key_press_modifiers(modifiers, key);
    d.frames(4);
}

#[test]
fn a_mounted_control_changes_the_data_its_lens_reaches() {
    let mut d = harness();
    d.click(Target::suffix(&path(&["Page", "Other", "Frame", "Add"])));
    d.frames(2);
    let app = &d.app();
    assert_eq!((app.other.count, app.shared.count), (1, 0));
}

#[test]
fn two_mounts_over_one_field_share_it() {
    let mut d = harness();
    d.click(Target::suffix(&path(&["Page", "Left", "Frame", "Add"])));
    d.click(Target::suffix(&path(&["Page", "Right", "Frame", "Add"])));
    d.frames(2);
    assert_eq!(d.app().shared.count, 2);
    assert_eq!(
        d.harness().query_all_by_label("Count 2").count(),
        2,
        "both mounts show the shared count"
    );
}

#[test]
fn a_mount_is_as_tall_as_its_content_unless_its_classes_say() {
    let d = harness();
    let inset = d.shell().style().mount.bare_padding;
    let mount = d.rect(Target::suffix(&path(&["Page", "Other"])));
    let add = d.rect(Target::suffix(&path(&["Page", "Other", "Frame", "Add"])));
    assert!(add.height() > 0.0, "precondition: the button is laid out");
    assert_eq!(mount.height(), add.height() + 2.0 * inset);
    assert_eq!(
        d.rect(Target::suffix(&path(&["Page", "Fixed"]))).height(),
        48.0
    );
}

#[test]
fn a_mounted_menu_item_changes_the_mounted_data() {
    let mut d = harness();
    d.click(Target::suffix(&path(&["Page", "Other", "Frame", "Add"])));
    d.click(Target::suffix(&path(&["Page", "Other", "Frame", "More"])));
    d.frames(6);
    d.harness().get_by_label("Reset").click();
    d.frames(6);
    assert_eq!(d.app().other.count, 0);
    assert!(
        d.harness().query_by_label("Reset").is_none(),
        "the menu closed on its item"
    );
}

#[test]
fn escape_closes_a_mounted_menu() {
    let mut d = harness();
    d.click(Target::suffix(&path(&["Page", "Other", "Frame", "More"])));
    d.frames(6);
    assert!(
        d.harness().query_by_label("Reset").is_some(),
        "precondition: open"
    );
    press(&mut d, Modifiers::NONE, Key::Escape);
    d.frames(6);
    assert!(d.harness().query_by_label("Reset").is_none());
}

#[test]
fn a_mounted_virtual_list_builds_its_rows_under_the_mount() {
    let mut d = harness();
    d.harness().get_by_label("Row 1").click();
    d.frames(4);
    assert_eq!(d.app().rows.picked, Some(1));
    assert!(
        d.harness().query_by_label("Row 199").is_none(),
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
                    )
                    .add();
                }
            });
    }

    #[test]
    fn a_mount_in_a_row_is_built_near_the_viewport_and_keeps_its_data() {
        let page = Page {
            blocks: (0..50).collect(),
            counters: (0..50).map(|_| Counter::default()).collect(),
        };
        let mut d = support::driver(vec2(300.0, 300.0), base_text(), page, view).build();
        d.frames(4);
        assert!(
            d.harness().query_all_by_label("Add").count() < 10,
            "precondition: only the rows near the viewport are built"
        );
        d.click(Target::suffix("/List/<rows>/0/Row(0)/Frame/Add"));
        d.frames(2);
        assert_eq!(d.app().counters[0].count, 1);
        assert_eq!(d.harness().query_all_by_label("Count 1").count(), 1);
    }
}

#[test]
fn a_mount_inside_a_mount_reaches_its_data_through_both_lenses() {
    let mut d = harness();
    d.click(Target::suffix(&path(&[
        "Page", "Nest", "Frame", "Inner", "Frame", "Add",
    ])));
    d.frames(2);
    assert_eq!(d.app().nest.inner.count, 1);
}

#[test]
fn typing_in_a_mounted_field_changes_the_mounted_data() {
    let mut d = harness();
    d.click(Target::suffix(&path(&["Page", "Notes", "Frame", "Note"])));
    d.frames(4);
    d.harness().get_by_label("Note").type_text("hi");
    d.frames(4);
    assert_eq!(d.app().notes.text, "hi");
}

mod commands {
    use super::*;

    fn focus_the_note(d: &mut Driver<App>) {
        d.click(Target::suffix(&path(&["Page", "Notes", "Frame", "Note"])));
        d.frames(6);
        assert_eq!(
            focused_path(d).as_deref(),
            Some("<widget>"),
            "precondition: the field has focus"
        );
    }

    #[test]
    fn a_chord_both_have_goes_to_the_mount_while_focus_is_within_it() {
        let mut d = harness();
        focus_the_note(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::K);
        let app = &d.app();
        assert_eq!(
            (app.notes.log.as_slice(), app.log.as_slice()),
            (&["notes k"][..], &[][..])
        );
    }

    #[test]
    fn a_chord_only_the_page_has_falls_through_from_the_mount() {
        let mut d = harness();
        focus_the_note(&mut d);
        press(&mut d, Modifiers::COMMAND, Key::J);
        assert_eq!(d.app().log, ["page j"]);
    }

    #[test]
    fn a_chord_both_have_goes_to_the_page_while_focus_is_outside_the_mount() {
        let mut d = harness();
        d.click(Target::suffix(&path(&["Page", "Before"])));
        d.frames(4);
        press(&mut d, Modifiers::COMMAND, Key::K);
        let app = &d.app();
        assert_eq!(
            (app.notes.log.as_slice(), app.log.as_slice()),
            (&[][..], &["page k"][..])
        );
    }
}

#[test]
fn tab_moves_into_a_mount_and_out_of_it() {
    let mut d = harness();
    support::drive_past_bar(&mut d);
    let mut seen = Vec::new();
    for _ in 0..12 {
        press(&mut d, Modifiers::NONE, Key::Tab);
        seen.extend(focused_path(&d));
    }
    let at = |wanted: &str| seen.iter().position(|path| path == wanted);
    let before = at(&path(&["Page", "Before"])).expect("Tab reaches Before");
    let add = at(&path(&["Page", "Left", "Frame", "Add"])).expect("Tab reaches the mount");
    let more = at(&path(&["Page", "Left", "Frame", "More"])).expect("Tab moves on in the mount");
    let next = at(&path(&["Page", "Right", "Frame", "Add"])).expect("Tab leaves the mount");
    assert!(before < add && add < more && more < next, "{seen:?}");
}

/// Misuse inside and around a mount is reported, and the frame goes on.
mod misuse {
    use eggshell_ui::Problem;
    use eggshell_ui::testing::{Headless, assert_reported};

    use super::*;

    #[derive(Default)]
    struct Two {
        first: Notes,
        second: Notes,
    }

    fn twice<'f>(b: &mut Builder<'_, 'f, Two>, two: &'f Two) {
        b.mount("m", tw(), &two.first, |two: &mut Two| &mut two.first, notes)
            .add();
        b.mount(
            "m",
            tw(),
            &two.second,
            |two: &mut Two| &mut two.second,
            notes,
        )
        .add();
    }

    #[test]
    fn a_mount_key_used_twice_is_reported_and_each_mount_keeps_its_own_commands() {
        let mut d = support::driver(vec2(400.0, 300.0), base_text(), Two::default(), twice).build();
        d.frames(4);
        d.expect_reported(Problem::DuplicateKey);
        let fields: Vec<_> = d.harness().query_all_by_label("Note").collect();
        assert_eq!(fields.len(), 2, "precondition: both mounts are built");
        fields[1].click();
        d.frames(6);
        d.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::K))
            .frames(3);
        let two = &d.app();
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
            )
            .add();
        });
        let report = assert_reported(headless.ctx(), Problem::DuplicateKey);
        assert!(
            report.message.contains(r#"/"m"/Frame/"x""#),
            "{}",
            report.message
        );
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
            b.mount("m", tw(), notes, |notes: &mut Notes| notes, notes_twice)
                .add();
        }
        let mut d =
            support::driver(vec2(400.0, 300.0), base_text(), Notes::default(), view).build();
        d.frames(4);
        d.expect_reported(Problem::CommandsTwice);
        d.harness().get_by_label("Note").click();
        d.frames(6);
        d.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::K))
            .frames(3);
        assert_eq!(d.app().log, ["notes k"]);
    }
}

/// A paste command in a mount gets what it pastes, and hears a failed read,
/// through the lens.
mod paste {
    use std::sync::Arc;

    use eggshell_ui::testing::FakeClipboard;
    use eggshell_ui::{ClipboardError, Paste, Via};
    use egui::Event;

    use super::*;

    #[derive(Default)]
    struct Clip {
        pasted: Vec<(Paste, Via)>,
        errors: Vec<ClipboardError>,
    }

    #[derive(Default)]
    struct Page {
        clip: Clip,
    }

    fn clip<'f>(b: &mut Builder<'_, 'f, Clip>, clip: &'f Clip) {
        let table = Commands::new().add(
            "paste",
            Command::paste("Paste", |clip: &mut Clip, paste, via| {
                clip.pasted.push((paste, via));
            })
            .shortcut(chord(Modifiers::COMMAND, Key::V))
            .on_error(|clip, error| clip.errors.push(error)),
        );
        b.commands(&table, clip);
        b.dropdown_menu("edit", "Edit")
            .items(|m| m.command("paste").add());
    }

    fn view<'f>(b: &mut Builder<'_, 'f, Page>, page: &'f Page) {
        b.mount(
            "m",
            tw(),
            &page.clip,
            |page: &mut Page| &mut page.clip,
            clip,
        )
        .add();
    }

    fn harness() -> (Driver<Page>, FakeClipboard) {
        let fake = FakeClipboard::default();
        let mut d = support::driver(vec2(300.0, 200.0), base_text(), Page::default(), view).build();
        d.layout().set_clipboard(Some(Arc::new(fake.clone())));
        d.frames(4);
        (d, fake)
    }

    fn click_paste(d: &mut Driver<Page>) {
        d.click(Target::suffix(r#"/"m"/Frame/"edit""#)).frames(5);
        d.click(Target::suffix(r#"/"m"/Frame/("edit", Menu)/Box/"paste""#))
            .frames(5);
    }

    #[test]
    fn a_mounted_paste_gets_the_click_and_the_chord_through_the_lens() {
        let (mut d, fake) = harness();
        fake.set_text("hello");
        click_paste(&mut d);
        support::drive_past_bar(&mut d);
        d.press(Key::Tab).frames(3);
        assert_eq!(
            focused_path(&d).as_deref(),
            Some(r#"/"m"/Frame/"edit""#),
            "precondition: focus is within the mount"
        );
        d.event(Event::Paste("hello".into())).frames(3);
        assert_eq!(
            d.app().clip.pasted,
            [
                (Paste::Text("hello".into()), Via::Read),
                (Paste::Text("hello".into()), Via::Event)
            ]
        );
    }

    #[test]
    fn a_mounted_paste_hears_a_failed_read_through_the_lens() {
        let (mut d, fake) = harness();
        fake.refuse(Some(ClipboardError::Failed("denied".into())));
        click_paste(&mut d);
        let clip = &d.app().clip;
        assert!(clip.pasted.is_empty());
        assert_eq!(clip.errors, [ClipboardError::Failed("denied".into())]);
    }
}

/// A caption sits above the mount, which is framed; a mount without one is
/// not.
mod caption {
    use eggshell_ui::Style;

    use super::*;

    #[derive(Default)]
    struct Page {
        counter: Counter,
    }

    fn view<'f>(b: &mut Builder<'_, 'f, Page>, page: &'f Page) {
        b.div_id(Part::Page, tw().flex().flex_col().gap(4.0).p(2.0), |b| {
            b.mount(
                Part::Left,
                tw(),
                &page.counter,
                |page: &mut Page| &mut page.counter,
                counter,
            )
            .with_caption("Example: counter")
            .add();
            b.mount(
                Part::Right,
                tw(),
                &page.counter,
                |page: &mut Page| &mut page.counter,
                counter,
            )
            .add();
        });
    }

    /// A frame and a gap that show: every measure apart from the others.
    fn framed(style: &mut Style) {
        style.mount.gap = 6.0;
        style.mount.padding = 10.0;
        style.mount.border_width = 2.0;
        style.mount.border = Color32::from_rgb(200, 20, 20);
    }

    fn harness() -> Driver<Page> {
        let mut d = support::driver(vec2(480.0, 400.0), base_text(), Page::default(), view)
            .style(support::shadcn_with(framed))
            .build();
        d.frames(4);
        d
    }

    #[test]
    fn a_caption_sits_above_the_mount_framed_by_the_style() {
        let d = harness();
        let look = d.shell().style().mount.clone();
        let caption = d.rect(Target::suffix("/Page/Left/Caption"));
        let frame = d.rect(Target::suffix("/Page/Left/Frame"));
        let add = d.rect(Target::suffix("/Page/Left/Frame/Add"));
        assert!(
            look.gap > 0.0 && look.padding > 0.0,
            "precondition: a style that shows it"
        );
        let roles: Vec<_> = d
            .harness()
            .query_all_by_label("Example: counter")
            .map(|node| node.accesskit_node().role())
            .collect();
        assert!(
            roles.contains(&egui::accesskit::Role::Group),
            "the mount is a group named by its caption: {roles:?}"
        );
        assert_eq!(frame.min.y - caption.max.y, look.gap, "the gap between");
        let inset = look.padding + look.border_width;
        assert_eq!(
            add.min - frame.min,
            vec2(inset, inset),
            "the view inside the frame"
        );
        assert!(
            support::shapes(&d).iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(rect)
                    if rect.rect == frame && rect.stroke.width == look.border_width
                        && rect.stroke.color == look.border
            )),
            "outlined in the style's border"
        );
    }

    #[test]
    fn a_mount_without_a_caption_is_inset_by_its_bare_padding_and_not_outlined() {
        let d = harness();
        let inset = d.shell().style().mount.bare_padding;
        assert!(inset > 0.0, "precondition: the style insets it");
        let mount = d.rect(Target::suffix("/Page/Right"));
        let frame = d.rect(Target::suffix("/Page/Right/Frame"));
        let add = d.rect(Target::suffix("/Page/Right/Frame/Add"));
        assert_eq!(frame, mount);
        assert_eq!(add.min, frame.min + vec2(inset, inset));
        assert!(
            support::shapes(&d).iter().all(|shape| !matches!(
                shape,
                egui::Shape::Rect(rect) if rect.rect == frame && rect.stroke.width > 0.0
            )),
            "no outline around it"
        );
    }
}

/// One mount on a page is selected at a time: a press or keyboard focus in
/// it selects it, a press or focus outside every mount, or Escape that
/// nothing else takes, deselects it. A selected mount's commands run and its
/// scroll containers take the wheel; an unselected one leaves both to the
/// page.
mod selection {
    use egui::{Event, MouseWheelUnit, Pos2, Rect};

    use super::*;

    struct Page {
        a: Counter,
        notes: Notes,
        rows: Rows,
        nest: Nest,
        log: Vec<&'static str>,
    }

    impl Default for Page {
        fn default() -> Self {
            Self {
                a: Counter::default(),
                notes: Notes::default(),
                rows: Rows {
                    rows: (0..40).collect(),
                    picked: None,
                },
                nest: Nest::default(),
                log: Vec::new(),
            }
        }
    }

    fn page_commands() -> Commands<Page> {
        Commands::new().add(
            "page k",
            Command::new("Page K", |page: &mut Page| page.log.push("page k")).shortcut(SHARED),
        )
    }

    fn view<'f>(b: &mut Builder<'_, 'f, Page>, page: &'f Page) {
        b.commands(&page_commands(), page);
        b.scroll(Part::Page, tw().w_px(400.0).h_px(360.0))
            .children(|s| {
                s.div(tw().flex().flex_col().gap(2.0).p(2.0), |s| {
                    s.button(Part::Before, "Before").on_click(|_: &mut Page| {});
                    s.mount(
                        Part::Left,
                        tw(),
                        &page.a,
                        |page: &mut Page| &mut page.a,
                        counter,
                    )
                    .with_caption("Example A")
                    .add();
                    s.mount(
                        Part::Notes,
                        tw(),
                        &page.notes,
                        |page: &mut Page| &mut page.notes,
                        notes,
                    )
                    .named("Example notes")
                    .add();
                    s.mount(
                        Part::Rows,
                        tw(),
                        &page.rows,
                        |page: &mut Page| &mut page.rows,
                        rows,
                    )
                    .named("Example rows")
                    .add();
                    s.mount(
                        Part::Nest,
                        tw(),
                        &page.nest,
                        |page: &mut Page| &mut page.nest,
                        nest,
                    )
                    .named("Example nest")
                    .add();
                    s.div(tw().h_px(600.0), |_| {});
                    s.button(Part::After, "After").on_click(|_: &mut Page| {});
                });
            });
    }

    fn harness() -> Driver<Page> {
        let mut d = support::driver(vec2(480.0, 420.0), base_text(), Page::default(), view).build();
        d.frames(4);
        d
    }

    /// The rect of the one node whose path ends with `suffix`.
    fn rect_at(d: &Driver<Page>, suffix: &str) -> Rect {
        let nodes = support::nodes(d);
        let found: Vec<_> = nodes
            .iter()
            .filter(|(path, _)| path.ends_with(suffix))
            .collect();
        assert_eq!(found.len(), 1, "one node ends with {suffix}: {found:?}");
        found[0].1
    }

    fn click(d: &mut Driver<Page>, at: Pos2) {
        d.hover(at);
        for pressed in [true, false] {
            d.event(Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        d.frames(4);
    }

    fn click_on(d: &mut Driver<Page>, suffix: &str) {
        let at = rect_at(d, suffix).center();
        click(d, at);
    }

    /// The groups selected, by name.
    fn selected(d: &Driver<Page>) -> Vec<String> {
        let mut names: Vec<String> = ["Example A", "Example notes", "Example rows", "Example nest"]
            .into_iter()
            .filter(|name| {
                d.harness().query_all_by_label(name).any(|node| {
                    let node = node.accesskit_node();
                    node.role() == egui::accesskit::Role::Group && node.is_selected() == Some(true)
                })
            })
            .map(str::to_owned)
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_press_selects_the_example_it_lands_in_and_no_other() {
        let mut d = harness();
        assert!(selected(&d).is_empty(), "precondition: none at first");
        click_on(&mut d, "/Left/Frame/Add");
        assert_eq!(selected(&d), ["Example A"]);
        click_on(&mut d, "/Notes/Frame/Note");
        assert_eq!(selected(&d), ["Example notes"]);
        click_on(&mut d, "/Left/Caption");
        assert_eq!(selected(&d), ["Example A"], "its caption is in it too");
        click_on(&mut d, "/Before");
        assert!(selected(&d).is_empty(), "a press outside every example");
    }

    #[test]
    fn a_press_in_a_nested_example_selects_it_and_the_one_around_it() {
        let mut d = harness();
        click_on(&mut d, "/Nest/Frame/Inner/Frame/Add");
        assert_eq!(selected(&d), ["Example nest"]);
        assert!(d.app().nest.inner.count == 1, "the press activated too");
        click_on(&mut d, "/Left/Frame/Add");
        assert_eq!(selected(&d), ["Example A"]);
    }

    #[test]
    fn tab_selects_the_example_focus_lands_in_and_leaving_deselects() {
        let mut d = harness();
        support::drive_past_bar(&mut d);
        let mut seen = Vec::new();
        for _ in 0..80 {
            d.press(Key::Tab).frames(2);
            let focused = support::focused_node(&d);
            let done = focused
                .as_deref()
                .is_some_and(|path| path.ends_with("/After"));
            seen.push((focused, selected(&d)));
            if done {
                break;
            }
        }
        let rows: Vec<_> = seen
            .iter()
            .filter(|(focused, _)| {
                focused
                    .as_deref()
                    .is_some_and(|path| path.contains("/Row("))
            })
            .collect();
        assert!(
            rows.len() > 3,
            "precondition: Tab went past the list's viewport"
        );
        assert!(
            rows.iter()
                .all(|(_, selected)| selected == &["Example rows".to_owned()]),
            "every row of the list keeps it selected: {rows:?}"
        );
        let at = |suffix: &str| {
            seen.iter()
                .find(|(focused, _)| {
                    focused
                        .as_deref()
                        .is_some_and(|path| path.ends_with(suffix))
                })
                .map(|(_, selected)| selected.clone())
        };
        assert_eq!(at("/Before"), Some(Vec::new()), "{seen:?}");
        assert_eq!(
            at("/Left/Frame/Add"),
            Some(vec!["Example A".to_owned()]),
            "{seen:?}"
        );
        assert_eq!(
            at("/Notes/Frame/Note"),
            Some(vec!["Example notes".to_owned()]),
            "{seen:?}"
        );
        assert_eq!(at("/After"), Some(Vec::new()), "{seen:?}");
    }

    #[test]
    fn escape_closes_the_examples_menu_first_and_then_deselects_it() {
        let mut d = harness();
        click_on(&mut d, "/Left/Frame/More");
        assert!(
            d.harness().query_by_label("Reset").is_some(),
            "precondition: the menu is open"
        );
        assert_eq!(selected(&d), ["Example A"]);
        d.press(Key::Escape).frames(5);
        assert!(
            d.harness().query_by_label("Reset").is_none(),
            "the menu closed"
        );
        assert_eq!(
            selected(&d),
            ["Example A"],
            "and the example stays selected"
        );
        d.press(Key::Escape).frames(3);
        assert!(selected(&d).is_empty());
    }

    #[test]
    fn a_press_in_the_examples_own_menu_keeps_it_selected() {
        let mut d = harness();
        click_on(&mut d, "/Left/Frame/Add");
        click_on(&mut d, "/Left/Frame/More");
        let reset = d.harness().get_by_label("Reset").rect().center();
        click(&mut d, reset);
        assert_eq!(d.app().a.count, 0, "precondition: the item ran");
        assert_eq!(selected(&d), ["Example A"]);
    }

    #[test]
    fn a_chord_reaches_the_selected_example_and_the_page_otherwise() {
        let mut d = harness();
        click_on(&mut d, "/Notes/Frame/Note");
        d.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::K))
            .frames(3);
        click_on(&mut d, "/Before");
        d.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::K))
            .frames(3);
        let page = d.app();
        assert_eq!(
            (page.notes.log.as_slice(), page.log.as_slice()),
            (&["notes k"][..], &["page k"][..])
        );
    }

    fn wheel(d: &mut Driver<Page>, at: Pos2, times: usize) {
        d.carry_to(at);
        for _ in 0..times {
            d.event(Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: vec2(0.0, -120.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
            })
            .frames(9);
        }
        d.frames(30);
    }

    fn page_offset(d: &Driver<Page>) -> f32 {
        rect_at(d, "/Before").min.y
    }

    fn list_offset(d: &Driver<Page>) -> f32 {
        rect_at(d, "/Rows/Frame/List").min.y - rect_at(d, "/Row(0)").min.y
    }

    #[test]
    fn the_wheel_over_an_unselected_examples_list_scrolls_the_page() {
        let mut d = harness();
        let list = rect_at(&d, "/Rows/Frame/List");
        let (page, rows) = (page_offset(&d), list_offset(&d));
        wheel(&mut d, list.center(), 1);
        assert!(selected(&d).is_empty(), "precondition: nothing selected");
        assert_eq!(list_offset(&d), rows, "the list stays put");
        assert!(page_offset(&d) < page, "the page scrolled");
    }

    #[test]
    fn the_wheel_over_a_selected_examples_list_scrolls_it_and_stops_at_its_end() {
        let mut d = harness();
        click_on(&mut d, "/Row(0)");
        assert_eq!(selected(&d), ["Example rows"], "precondition");
        let list = rect_at(&d, "/Rows/Frame/List");
        let page = page_offset(&d);
        wheel(&mut d, list.center(), 1);
        assert!(list_offset(&d) > 0.0, "the list scrolled");
        assert_eq!(page_offset(&d), page, "the page did not");
        wheel(&mut d, list.center(), 20);
        assert_eq!(page_offset(&d), page, "nor past the list's end");
    }

    #[test]
    fn an_uncaptioned_examples_inset_keeps_a_focused_controls_ring_off_the_selected_ring() {
        let mut d = harness();
        let style = d.shell().style();
        let before = rect_at(&d, "/Notes/Frame/Note");
        click_on(&mut d, "/Notes/Frame/Note");
        assert_eq!(selected(&d), ["Example notes"]);
        assert!(
            support::focused_node(&d).is_some_and(|path| path.ends_with("/Notes/Frame/Note")),
            "precondition: the field has focus"
        );
        let frame = rect_at(&d, "/Notes/Frame");
        let field = rect_at(&d, "/Notes/Frame/Note");
        assert_eq!(field, before, "selecting moves nothing");
        let content = frame.shrink(style.mount.bare_padding);
        assert_eq!(
            field.min, content.min,
            "precondition: the field sits at the content's edge"
        );
        let focus_ring = field.expand(style.focus().width);
        let selected_ring = frame.expand(style.focus().width);
        assert!(
            frame.contains_rect(focus_ring),
            "the focus ring {focus_ring:?} stays clear of the selected ring's band, \
             from {frame:?} out to {selected_ring:?}"
        );
    }

    #[test]
    fn a_captioned_examples_selected_ring_stays_off_its_caption() {
        let mut d = harness();
        let style = d.shell().style();
        let before = rect_at(&d, "/Left/Caption");
        click_on(&mut d, "/Left/Frame/Add");
        assert_eq!(selected(&d), ["Example A"]);
        let caption = rect_at(&d, "/Left/Caption");
        let frame = rect_at(&d, "/Left/Frame");
        assert_eq!(caption, before, "selecting moves nothing");
        assert!(
            caption.max.y <= frame.min.y && caption.x_range().intersects(frame.x_range()),
            "precondition: the caption {caption:?} sits above the frame {frame:?}"
        );
        let selected_ring = frame.expand(style.focus().width);
        assert!(
            !selected_ring.intersects(caption),
            "the selected ring {selected_ring:?} stays clear of the caption {caption:?}"
        );
    }
}
