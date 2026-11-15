//! App commands: what the application can be asked to do, defined once.
//!
//! A [`Command`] is a title, a shortcut, whether it can run now and what it
//! runs. A [`Commands`] table names each by one of the application's own keys
//! and is declared on the builder with [`Builder::commands`], once a frame. A
//! stock menu item binds to a command with [`Menu::command`], taking its
//! label, shortcut text and enablement from the table, and one dispatcher
//! runs the commands whose shortcut the frame pressed.
//!
//! The dispatcher runs in the draw, right after the library has claimed Esc
//! and Return and before any draw closure, so a chord it takes is never seen
//! by a custom node. Modifiers are matched exactly: `Cmd+S` does not fire on
//! `Cmd+Shift+S`, as egui's `consume_shortcut` would. Which table is live
//! follows the modals: under an open modal only the modal's own table is, and
//! page commands do not fire under a dialog.
//!
//! [`Builder::commands`]: crate::Builder::commands
//! [`Menu::command`]: crate::owned::Menu::command

use egui::{Context, Event, Id, Key as EguiKey, KeyboardShortcut, Modifiers};

use crate::clipboard::{ClipboardError, Paste, Via};
use crate::key::{Key, child_id};
use crate::overlay_input::Typing;
use crate::shell::Shell;

/// What a command does when a text field has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InText {
    /// Fires, even for a chord that editing text also uses.
    Fire,
    /// Never fires, even for a chord that editing text leaves alone.
    Yield,
}

/// One thing the application can be asked to do.
///
/// While a text field has the keyboard a command fires only if its chord is
/// not one that editing text uses (a bare key, an arrow, `Cmd+C`, `Cmd+V`,
/// `Cmd+X`, `Cmd+A`, `Cmd+Z`, `Cmd+Y`, word deletion); [`in_text`](Self::in_text)
/// overrides that for one command.
pub struct Command<A> {
    title: &'static str,
    run: Run<A>,
    enabled: fn(&A) -> bool,
    shortcut: Option<KeyboardShortcut>,
    in_text: Option<InText>,
}

/// What a command does.
enum Run<A> {
    Plain(fn(&mut A)),
    /// Takes what the clipboard holds, from the chord's event or, for a
    /// click, a read; `on_error` hears a read that failed.
    Paste {
        run: fn(&mut A, Paste, Via),
        on_error: Option<fn(&mut A, ClipboardError)>,
    },
}

impl<A> Clone for Run<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Run<A> {}

impl<A> Clone for Command<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Command<A> {}

impl<A> Command<A> {
    /// A command that is always enabled and has no shortcut. `title` is what a
    /// menu item says.
    pub fn new(title: &'static str, run: fn(&mut A)) -> Self {
        Self {
            title,
            run: Run::Plain(run),
            enabled: |_| true,
            shortcut: None,
            in_text: None,
        }
    }

    /// A command that pastes: `run` gets what the clipboard holds. From the
    /// chord that is what the paste event carried; from a menu item or a
    /// button it is a read of the clipboard. Both are collapsed the same way,
    /// to one [`Paste`]: the text if there is any, else the image; the [`Via`]
    /// says which it was. A read that
    /// fails does not run the command: [`on_error`](Self::on_error) hears it.
    pub fn paste(title: &'static str, run: fn(&mut A, Paste, Via)) -> Self {
        Self {
            run: Run::Paste {
                run,
                on_error: None,
            },
            ..Self::new(title, |_| {})
        }
    }

    /// What happens when a click's read of the clipboard fails. Only a
    /// [`paste`](Self::paste) command reads; debug builds panic for any other.
    #[track_caller]
    pub fn on_error(mut self, hear: fn(&mut A, ClipboardError)) -> Self {
        match &mut self.run {
            Run::Paste { on_error, .. } => *on_error = Some(hear),
            Run::Plain(_) => {
                #[cfg(debug_assertions)]
                panic!("`on_error` is for a command made with `Command::paste`");
            }
        }
        self
    }

    /// The chord that runs the command. Use [`egui::Modifiers::COMMAND`] for
    /// the platform's command key: it is shown as `Cmd` on macOS and `Ctrl`
    /// elsewhere. `Cmd+X`, `Cmd+C` and `Cmd+V` are matched as the cut, copy and
    /// paste events egui's backends send for them.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Whether the command can run now. A disabled command's menu item is
    /// disabled and its chord is left alone, so whatever is underneath it
    /// still gets the key.
    pub fn enabled(mut self, enabled: fn(&A) -> bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Overrides what the command does while a text field has the keyboard.
    pub fn in_text(mut self, in_text: InText) -> Self {
        self.in_text = Some(in_text);
        self
    }

    /// What a menu item for the command says.
    pub fn title(&self) -> &'static str {
        self.title
    }

    /// The chord that runs the command.
    pub fn chord(&self) -> Option<KeyboardShortcut> {
        self.shortcut
    }

    pub(crate) fn is_enabled(&self, app: &A) -> bool {
        (self.enabled)(app)
    }

    /// Runs the command as a click does, with nothing from an event.
    pub(crate) fn activate(&self, app: &mut A, shell: &Shell) {
        self.fire(app, None, shell);
    }

    /// `pasted` is what the chord's event carried, if it was one.
    fn fire(&self, app: &mut A, pasted: Option<Paste>, shell: &Shell) {
        match self.run {
            Run::Plain(run) => run(app),
            Run::Paste { run, on_error } => {
                let paste = match pasted {
                    Some(paste) => Ok((paste, Via::Event)),
                    None => shell
                        .read_clipboard()
                        .map(|held| (Paste::from(held), Via::Read)),
                };
                match paste {
                    Ok((paste, via)) => run(app, paste, via),
                    Err(error) => {
                        if let Some(hear) = on_error {
                            hear(app, error);
                        }
                    }
                }
            }
        }
    }

    fn fires_while_typing(&self, shortcut: KeyboardShortcut) -> bool {
        match self.in_text {
            Some(InText::Fire) => true,
            Some(InText::Yield) => false,
            None => !edits_text(shortcut),
        }
    }
}

/// Whether a text field would act on the chord: egui's own list of chords that
/// conflict with editing, and the commands every text field answers.
fn edits_text(shortcut: KeyboardShortcut) -> bool {
    shortcut.conflicts_with_text_editing()
        || matches!(
            shortcut.logical_key,
            EguiKey::A
                | EguiKey::C
                | EguiKey::V
                | EguiKey::X
                | EguiKey::Y
                | EguiKey::Z
                | EguiKey::Backspace
                | EguiKey::Delete
        )
}

struct Entry<A> {
    id: Id,
    readable: String,
    command: Command<A>,
}

impl<A> Clone for Entry<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            readable: self.readable.clone(),
            command: self.command,
        }
    }
}

/// The commands of a root, in the order they are tried. Each is named by a key
/// of the application's own, usually one enum.
pub struct Commands<A> {
    entries: Vec<Entry<A>>,
}

impl<A> Default for Commands<A> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<A> Clone for Commands<A> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<A> Commands<A> {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `command` under `id`.
    pub fn add<K: Key>(mut self, id: K, command: Command<A>) -> Self {
        self.entries.push(Entry {
            id: child_id(Id::NULL, &id),
            readable: format!("{id:?}"),
            command,
        });
        self
    }

    /// The commands, in the order they are tried, for a reader of the table
    /// other than the menus and the dispatcher, such as a native menu bar.
    pub fn iter(&self) -> impl Iterator<Item = &Command<A>> {
        self.entries.iter().map(|entry| &entry.command)
    }

    pub(crate) fn find<K: Key>(&self, id: &K) -> Option<&Command<A>> {
        let id = child_id(Id::NULL, id);
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| &entry.command)
    }

    /// Panics, in debug builds, on a table that cannot be dispatched: the same
    /// key twice, the same chord twice, or a bare Esc or Enter, which the
    /// library keeps for dismissal and the default button.
    #[cfg(debug_assertions)]
    pub(crate) fn validate(&self) {
        for (at, entry) in self.entries.iter().enumerate() {
            let Entry {
                readable, command, ..
            } = entry;
            let earlier = &self.entries[..at];
            if let Some(first) = earlier.iter().find(|other| other.id == entry.id) {
                panic!(
                    "command {readable} is added twice (first as {})",
                    first.readable
                );
            }
            let Some(shortcut) = command.shortcut else {
                continue;
            };
            if shortcut.modifiers.is_none()
                && matches!(shortcut.logical_key, EguiKey::Escape | EguiKey::Enter)
            {
                panic!(
                    "command {readable} is bound to a bare {:?}, which the library keeps for \
                     dismissal and the default button",
                    shortcut.logical_key
                );
            }
            if let Some(other) = earlier.iter().find(|other| {
                other
                    .command
                    .shortcut
                    .is_some_and(|other| same_chord(other, shortcut))
            }) {
                panic!(
                    "commands {} and {readable} are bound to the same chord",
                    other.readable
                );
            }
        }
    }
}

#[cfg(debug_assertions)]
fn same_chord(a: KeyboardShortcut, b: KeyboardShortcut) -> bool {
    a.logical_key == b.logical_key && a.modifiers.matches_exact(b.modifiers)
}

/// The roots whose commands are live from `start`: `start`, then the root
/// that opened it, and so on up to the page, ending at the first modal, whose
/// own table is the last that runs. The dispatcher and a menu item's lookup
/// both go by this, so an item offers exactly the commands whose chords run.
pub(crate) fn live_roots(
    start: usize,
    parent_of: impl Fn(usize) -> Option<usize>,
    is_modal: impl Fn(usize) -> bool,
) -> Vec<usize> {
    let mut live = vec![start];
    let mut at = start;
    while !is_modal(at) {
        let Some(parent) = parent_of(at) else { break };
        live.push(parent);
        at = parent;
    }
    live
}

/// Runs the commands of `table` whose chord this frame pressed, each once, in
/// table order, taking the key from the frame's input. A chord is left alone
/// for a command that is disabled, or that yields to a text field that has
/// the keyboard.
pub(crate) fn dispatch<A>(
    ctx: &Context,
    shell: &Shell,
    typing: Typing,
    table: &Commands<A>,
    app: &mut A,
) {
    for Entry { command, .. } in &table.entries {
        let Some(shortcut) = command.shortcut else {
            continue;
        };
        if typing == Typing::Yes && !command.fires_while_typing(shortcut) {
            continue;
        }
        if !(command.enabled)(app) {
            continue;
        }
        if let Some(press) = consume(ctx, shortcut) {
            command.fire(app, press.pasted(), shell);
            ctx.request_repaint();
        }
    }
}

/// How a chord arrived.
enum Press {
    /// As a key press, or a cut or copy event: nothing came with it.
    Key,
    /// As a paste event, with what the backend read from the clipboard.
    Paste(Paste),
}

impl Press {
    fn pasted(self) -> Option<Paste> {
        match self {
            Self::Key => None,
            Self::Paste(paste) => Some(paste),
        }
    }
}

/// Takes the presses of `shortcut` from the frame, matching its modifiers
/// exactly. Cut, copy and paste are presses of `Cmd+X`, `Cmd+C` and `Cmd+V`
/// here, since egui's backends send those chords as events of their own and
/// no key press. A paste carries what the backend read: the first of them, if
/// there are several.
fn consume(ctx: &Context, shortcut: KeyboardShortcut) -> Option<Press> {
    let KeyboardShortcut {
        modifiers,
        logical_key,
    } = shortcut;
    let clipboard = |key| logical_key == key && Modifiers::COMMAND.matches_exact(modifiers);
    ctx.input_mut(|input| {
        let mut pressed = None;
        input.events.retain(|event| {
            let press = match event {
                Event::Key {
                    key,
                    modifiers: held,
                    pressed: true,
                    ..
                } => (*key == logical_key && held.matches_exact(modifiers)).then_some(Press::Key),
                Event::Cut => clipboard(EguiKey::X).then_some(Press::Key),
                Event::Copy => clipboard(EguiKey::C).then_some(Press::Key),
                Event::Paste(text) => {
                    clipboard(EguiKey::V).then(|| Press::Paste(Paste::Text(text.clone())))
                }
                Event::PasteImage(image) => {
                    clipboard(EguiKey::V).then(|| Press::Paste(Paste::Image((**image).clone())))
                }
                _ => None,
            };
            let taken = press.is_some();
            if pressed.is_none() {
                pressed = press;
            }
            !taken
        });
        pressed
    })
}
