//! The clipboard, read on demand: a Paste command gets the same thing from
//! the chord (the event's contents) as from a menu click (a read), and a
//! layout's clipboard is its own.

use std::sync::Arc;

use eggshell_ui::testing::FakeClipboard;
use eggshell_ui::{
    Builder, Chord, ClipboardError, Command, Commands, LayoutState, Paste, TextSpec, TextStyle,
    Via, Weight, tw,
};
use egui::{Color32, ColorImage, Event, FontFamily, Key, Modifiers, vec2};

use eggshell_ui::testing::{Driver, Target};

use crate::support;

const FILE: &str = r#"/"Page"/"file""#;
const ITEM: &str = r#"/"Page"/("file", Menu)/Box/Paste"#;

#[derive(Hash, Debug, Clone, Copy)]
enum Cmd {
    Paste,
}

#[derive(Default)]
struct App {
    pasted: Vec<Paste>,
    via: Vec<Via>,
    errors: Vec<ClipboardError>,
}

fn commands() -> Commands<App> {
    Commands::new().add(
        Cmd::Paste,
        Command::paste("Paste", |app: &mut App, paste, via| {
            app.pasted.push(paste);
            app.via.push(via);
        })
        .shortcut(const { Chord::new(Modifiers::COMMAND, Key::V).expect("not reserved") })
        .on_error(|app, error| app.errors.push(error)),
    )
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

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.commands(&commands(), app);
    b.div_id("Page", tw().flex().flex_col().p(2.0), |s| {
        s.dropdown_menu("file", "File")
            .items(|m| m.command(Cmd::Paste).add());
    });
}

fn harness() -> (Driver<App>, FakeClipboard) {
    let fake = FakeClipboard::default();
    let mut d = support::driver(vec2(300.0, 200.0), base_text(), App::default(), view).build();
    d.layout().set_clipboard(Some(Arc::new(fake.clone())));
    d.frames(4);
    (d, fake)
}

fn pixel() -> ColorImage {
    ColorImage::example()
}

fn click_paste(d: &mut Driver<App>) {
    d.click(Target::suffix(FILE)).frames(5);
    d.click(Target::suffix(ITEM)).frames(5);
}

fn chord(d: &mut Driver<App>, event: Event) {
    d.event(event).frames(3);
}

#[test]
fn a_menu_click_and_the_chord_paste_the_same_text() {
    let (mut d, fake) = harness();
    fake.set_text("hello");
    click_paste(&mut d);
    chord(&mut d, Event::Paste("hello".into()));
    assert_eq!(
        d.app().pasted,
        [Paste::Text("hello".into()), Paste::Text("hello".into())]
    );
    assert_eq!(d.app().via, [Via::Read, Via::Event]);
}

#[test]
fn text_wins_over_an_image_on_a_click_as_on_the_chord() {
    let (mut d, fake) = harness();
    fake.set_text("hello");
    fake.set_image(pixel());
    click_paste(&mut d);
    assert_eq!(d.app().pasted, [Paste::Text("hello".into())]);
}

#[test]
fn a_menu_click_and_the_chord_paste_the_same_image() {
    let (mut d, fake) = harness();
    fake.set_image(pixel());
    click_paste(&mut d);
    chord(&mut d, Event::PasteImage(Arc::new(pixel())));
    assert_eq!(
        d.app().pasted,
        [Paste::Image(pixel()), Paste::Image(pixel())]
    );
}

#[test]
fn an_empty_clipboard_pastes_nothing_on_a_click() {
    let (mut d, _fake) = harness();
    click_paste(&mut d);
    assert_eq!(d.app().pasted, [Paste::Nothing]);
}

#[test]
fn a_failed_read_does_not_run_the_command_and_is_heard() {
    let (mut d, fake) = harness();
    fake.set_text("hello");
    fake.refuse(Some(ClipboardError::Failed("denied".into())));
    click_paste(&mut d);
    assert!(d.app().pasted.is_empty());
    assert_eq!(d.app().errors, [ClipboardError::Failed("denied".into())]);
}

#[test]
fn the_chord_pastes_the_events_text_whatever_the_clipboard_says() {
    let (mut d, fake) = harness();
    fake.refuse(Some(ClipboardError::Unavailable));
    chord(&mut d, Event::Paste("typed".into()));
    assert_eq!(d.app().pasted, [Paste::Text("typed".into())]);
    assert!(d.app().errors.is_empty());
}

#[test]
fn a_layouts_clipboard_is_its_own() {
    let ctx = egui::Context::default();
    let (a, b) = (FakeClipboard::default(), FakeClipboard::default());
    a.set_text("a");
    b.set_text("b");
    let (a_layout, b_layout) = (
        LayoutState::named("a").with_clipboard(Arc::new(a.clone())),
        LayoutState::named("b").with_clipboard(Arc::new(b.clone())),
    );
    let text = |layout: &LayoutState| layout.shell(&ctx).read_clipboard().expect("read").text;
    assert_eq!(text(&a_layout).as_deref(), Some("a"));
    assert_eq!(text(&b_layout).as_deref(), Some("b"));
}

#[test]
fn an_image_written_through_the_shell_reaches_the_source_and_a_refusal_is_reported() {
    let ctx = egui::Context::default();
    let fake = FakeClipboard::default();
    let layout = LayoutState::new().with_clipboard(Arc::new(fake.clone()));
    let shell = layout.shell(&ctx);
    assert_eq!(shell.write_clipboard_image(&pixel()), Ok(()));
    assert_eq!(fake.written(), [pixel()]);
    fake.refuse(Some(ClipboardError::Unavailable));
    assert_eq!(
        shell.write_clipboard_image(&pixel()),
        Err(ClipboardError::Unavailable)
    );
}

#[test]
fn a_paste_event_carries_its_payload_and_other_events_none() {
    assert_eq!(
        Paste::from_event(&Event::Paste("typed".into())),
        Some(Paste::Text("typed".into()))
    );
    assert_eq!(
        Paste::from_event(&Event::PasteImage(Arc::new(pixel()))),
        Some(Paste::Image(pixel()))
    );
    for event in [Event::Copy, Event::Cut, Event::Text("v".into())] {
        assert_eq!(Paste::from_event(&event), None, "{event:?}");
    }
}

#[test]
fn a_detail_reads_without_naming_the_clipboard() {
    assert_eq!(
        ClipboardError::Failed("no owner".into()).detail(),
        "no owner"
    );
    assert_eq!(ClipboardError::Unavailable.detail(), "not available");
}
