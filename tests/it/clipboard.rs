//! The clipboard, read on demand: a Paste command gets the same thing from
//! the chord (the event's contents) as from a menu click (a read), and a
//! layout's clipboard is its own.

use std::sync::Arc;

use eggshell_ui::testing::FakeClipboard;
use eggshell_ui::{
    Builder, ClipboardError, Command, Commands, LayoutState, Paste, TextSpec, TextStyle, Via, tw,
};
use egui::{Color32, ColorImage, Event, FontFamily, Key, KeyboardShortcut, Modifiers, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node};

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
        .shortcut(KeyboardShortcut::new(Modifiers::COMMAND, Key::V))
        .on_error(|app, error| app.errors.push(error)),
    )
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

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.commands(&commands(), app);
    b.div_id("Page", tw().flex().flex_col().p(2.0), |s| {
        s.dropdown_menu("file", "File")
            .items(|m| m.command(Cmd::Paste));
    });
}

fn harness() -> (Harness<'static, Rig<App>>, FakeClipboard) {
    let fake = FakeClipboard::default();
    let mut harness = support::harness(vec2(300.0, 200.0), base_text(), App::default(), view);
    harness
        .state()
        .layout
        .set_clipboard(Some(Arc::new(fake.clone())));
    harness.run_steps(4);
    (harness, fake)
}

fn pixel() -> ColorImage {
    ColorImage::example()
}

fn click_paste(harness: &mut Harness<'_, Rig<App>>) {
    click_node(harness, FILE);
    harness.run_steps(5);
    click_node(harness, ITEM);
    harness.run_steps(5);
}

fn chord(harness: &mut Harness<'_, Rig<App>>, event: Event) {
    harness.event(event);
    harness.run_steps(4);
}

#[test]
fn a_menu_click_and_the_chord_paste_the_same_text() {
    let (mut harness, fake) = harness();
    fake.set_text("hello");
    click_paste(&mut harness);
    chord(&mut harness, Event::Paste("hello".into()));
    assert_eq!(
        harness.state().app.pasted,
        [Paste::Text("hello".into()), Paste::Text("hello".into())]
    );
    assert_eq!(harness.state().app.via, [Via::Read, Via::Event]);
}

#[test]
fn text_wins_over_an_image_on_a_click_as_on_the_chord() {
    let (mut harness, fake) = harness();
    fake.set_text("hello");
    fake.set_image(pixel());
    click_paste(&mut harness);
    assert_eq!(harness.state().app.pasted, [Paste::Text("hello".into())]);
}

#[test]
fn a_menu_click_and_the_chord_paste_the_same_image() {
    let (mut harness, fake) = harness();
    fake.set_image(pixel());
    click_paste(&mut harness);
    chord(&mut harness, Event::PasteImage(Arc::new(pixel())));
    assert_eq!(
        harness.state().app.pasted,
        [Paste::Image(pixel()), Paste::Image(pixel())]
    );
}

#[test]
fn an_empty_clipboard_pastes_nothing_on_a_click() {
    let (mut harness, _fake) = harness();
    click_paste(&mut harness);
    assert_eq!(harness.state().app.pasted, [Paste::Nothing]);
}

#[test]
fn a_failed_read_does_not_run_the_command_and_is_heard() {
    let (mut harness, fake) = harness();
    fake.set_text("hello");
    fake.refuse(Some(ClipboardError::Failed("denied".into())));
    click_paste(&mut harness);
    assert!(harness.state().app.pasted.is_empty());
    assert_eq!(
        harness.state().app.errors,
        [ClipboardError::Failed("denied".into())]
    );
}

#[test]
fn the_chord_pastes_the_events_text_whatever_the_clipboard_says() {
    let (mut harness, fake) = harness();
    fake.refuse(Some(ClipboardError::Unavailable));
    chord(&mut harness, Event::Paste("typed".into()));
    assert_eq!(harness.state().app.pasted, [Paste::Text("typed".into())]);
    assert!(harness.state().app.errors.is_empty());
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
