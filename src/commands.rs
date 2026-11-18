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

use crate::Problem;
use crate::clipboard::{ClipboardError, Paste, Via};
use crate::key::{Key, child_id};
use crate::overlay_input::Typing;
use crate::owned::{Enablement, Mark};
use crate::report::report;
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
    binding: Binding<A>,
    shows: Shows<A>,
}

/// TODO(prose): what a command's items show of the application data, read with [`Command::state`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandState {
    /// TODO(prose): an action, shown with nothing before its title.
    Plain,
    /// TODO(prose): a setting that is on or off, shown with a check mark where it is on.
    Checked(Mark),
    /// TODO(prose): one choice among several, shown with a dot where it is chosen.
    Chosen(Mark),
}

/// What a command reads of the application data to say its state.
enum Shows<A> {
    Plain,
    Checked(fn(&A) -> Mark),
    Chosen(fn(&A) -> Mark),
}

impl<A> Clone for Shows<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Shows<A> {}

/// When a command runs: shared by plain and paste commands.
struct Binding<A> {
    enabled: fn(&A) -> bool,
    shortcut: Option<Chord>,
    in_text: Option<InText>,
}

impl<A> Clone for Binding<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Binding<A> {}

impl<A> Default for Binding<A> {
    fn default() -> Self {
        Self {
            enabled: |_| true,
            shortcut: None,
            in_text: None,
        }
    }
}

/// TODO(prose): a key and the modifiers held for it, which runs a command. A
/// bare Esc or Enter is not a chord: the library keeps them for dismissal and
/// the default button.
///
/// ```
/// use eggshell_ui::Chord;
/// use egui::{Key, Modifiers};
///
/// assert!(Chord::new(Modifiers::COMMAND, Key::S).is_some());
/// assert!(Chord::new(Modifiers::COMMAND, Key::Enter).is_some());
/// assert!(Chord::new(Modifiers::NONE, Key::Escape).is_none());
/// ```
///
/// Made in a const, a chord the library keeps is a compile error:
///
/// ```compile_fail
/// use eggshell_ui::Chord;
/// use egui::{Key, Modifiers};
///
/// const CLOSE: Chord = Chord::new(Modifiers::NONE, Key::Escape).expect("reserved");
/// ```
///
/// A command's shortcut is a `Chord`, so a bare `KeyboardShortcut` does not
/// compile:
///
/// ```compile_fail
/// use eggshell_ui::Command;
/// use egui::{Key, KeyboardShortcut, Modifiers};
///
/// Command::new("Close", |_: &mut ()| {})
///     .shortcut(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord(KeyboardShortcut);

impl Chord {
    /// TODO(prose): `key` with `modifiers` held, or `None` for a bare Esc or
    /// Enter.
    ///
    /// ```
    /// use eggshell_ui::Chord;
    /// use egui::{Key, Modifiers};
    ///
    /// const SAVE: Chord = Chord::new(Modifiers::COMMAND, Key::S).expect("not reserved");
    /// assert_eq!(Some(SAVE), Chord::new(Modifiers::COMMAND, Key::S));
    /// assert!(Chord::new(Modifiers::NONE, Key::Enter).is_none());
    /// ```
    pub const fn new(modifiers: Modifiers, key: EguiKey) -> Option<Self> {
        let Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        } = modifiers;
        let bare = !(alt || ctrl || shift || mac_cmd || command);
        if bare && matches!(key, EguiKey::Escape | EguiKey::Enter) {
            None
        } else {
            Some(Self(KeyboardShortcut::new(modifiers, key)))
        }
    }
}

impl From<Chord> for KeyboardShortcut {
    fn from(chord: Chord) -> Self {
        chord.0
    }
}

/// TODO(prose): a command that pastes, made with [`Command::paste`]; only it
/// has [`on_error`](Self::on_error). [`Commands::add`] takes it as a
/// [`Command`].
///
/// ```
/// use eggshell_ui::{Chord, Command, Commands, Paste};
/// use egui::{Key, Modifiers};
///
/// #[derive(Hash, Debug)]
/// enum Cmd { Paste }
/// #[derive(Default)]
/// struct App { text: String, failed: bool }
///
/// let table: Commands<App> = Commands::new().add(
///     Cmd::Paste,
///     Command::paste("Paste", |app: &mut App, paste, _| {
///         if let Paste::Text(text) = paste {
///             app.text.push_str(&text);
///         }
///     })
///     .shortcut(const { Chord::new(Modifiers::COMMAND, Key::V).expect("not reserved") })
///     .on_error(|app, _| app.failed = true),
/// );
/// ```
///
/// A plain command reads nothing, so it has no `on_error`:
///
/// ```compile_fail
/// use eggshell_ui::Command;
///
/// Command::new("Save", |_: &mut ()| {}).on_error(|_, _| {});
/// ```
pub struct PasteCommand<A> {
    title: &'static str,
    run: fn(&mut A, Paste, Via),
    on_error: Option<fn(&mut A, ClipboardError)>,
    binding: Binding<A>,
}

impl<A> PasteCommand<A> {
    /// What happens when a click's read of the clipboard fails.
    pub fn on_error(mut self, hear: fn(&mut A, ClipboardError)) -> Self {
        self.on_error = Some(hear);
        self
    }

    /// The chord that runs the command, as [`Command::shortcut`].
    pub fn shortcut(mut self, shortcut: Chord) -> Self {
        self.binding.shortcut = Some(shortcut);
        self
    }

    /// Whether the command can run now, as [`Command::enabled`].
    pub fn enabled(mut self, enabled: fn(&A) -> bool) -> Self {
        self.binding.enabled = enabled;
        self
    }

    /// What the command does while a text field has the keyboard, as
    /// [`Command::in_text`].
    pub fn in_text(mut self, in_text: InText) -> Self {
        self.binding.in_text = Some(in_text);
        self
    }
}

impl<A> From<PasteCommand<A>> for Command<A> {
    fn from(paste: PasteCommand<A>) -> Self {
        let PasteCommand {
            title,
            run,
            on_error,
            binding,
        } = paste;
        Self {
            title,
            run: Run::Paste { run, on_error },
            binding,
            shows: Shows::Plain,
        }
    }
}

/// What a command does.
enum Run<A> {
    Plain(fn(&mut A)),
    Shell(fn(&mut A, &Shell)),
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
        Self::running(title, Run::Plain(run))
    }

    /// TODO(prose): a command whose `run` also gets the [`Shell`] of the layout it runs in.
    pub fn with_shell(title: &'static str, run: fn(&mut A, &Shell)) -> Self {
        Self::running(title, Run::Shell(run))
    }

    fn running(title: &'static str, run: Run<A>) -> Self {
        Self {
            title,
            run,
            binding: Binding::default(),
            shows: Shows::Plain,
        }
    }

    /// A command that pastes: `run` gets what the clipboard holds. From the
    /// chord that is what the paste event carried; from a menu item or a
    /// button it is a read of the clipboard. Both are collapsed the same way,
    /// to one [`Paste`]: the text if there is any, else the image; the [`Via`]
    /// says which it was. A read that
    /// fails does not run the command: [`on_error`](PasteCommand::on_error) hears it.
    pub fn paste(title: &'static str, run: fn(&mut A, Paste, Via)) -> PasteCommand<A> {
        PasteCommand {
            title,
            run,
            on_error: None,
            binding: Binding::default(),
        }
    }

    /// The chord that runs the command. Use [`egui::Modifiers::COMMAND`] for
    /// the platform's command key: it is shown as `Cmd` on macOS and `Ctrl`
    /// elsewhere. `Cmd+X`, `Cmd+C` and `Cmd+V` are matched as the cut, copy and
    /// paste events egui's backends send for them.
    pub fn shortcut(mut self, shortcut: Chord) -> Self {
        self.binding.shortcut = Some(shortcut);
        self
    }

    /// Whether the command can run now. A disabled command's menu item is
    /// disabled and its chord is left alone, so whatever is underneath it
    /// still gets the key.
    pub fn enabled(mut self, enabled: fn(&A) -> bool) -> Self {
        self.binding.enabled = enabled;
        self
    }

    /// Overrides what the command does while a text field has the keyboard.
    pub fn in_text(mut self, in_text: InText) -> Self {
        self.binding.in_text = Some(in_text);
        self
    }

    /// TODO(prose): makes the command a setting that is on or off: its menu items show a check mark where `checked` is on.
    pub fn checked(mut self, checked: fn(&A) -> Mark) -> Self {
        self.shows = Shows::Checked(checked);
        self
    }

    /// TODO(prose): makes the command one choice among several: its menu items show a dot where `chosen` is on.
    pub fn chosen(mut self, chosen: fn(&A) -> Mark) -> Self {
        self.shows = Shows::Chosen(chosen);
        self
    }

    /// TODO(prose): whether the command is a plain action, or a setting or a choice and how `app` has it.
    pub fn state(&self, app: &A) -> CommandState {
        match self.shows {
            Shows::Plain => CommandState::Plain,
            Shows::Checked(checked) => CommandState::Checked(checked(app)),
            Shows::Chosen(chosen) => CommandState::Chosen(chosen(app)),
        }
    }

    /// What a menu item for the command says.
    pub fn title(&self) -> &'static str {
        self.title
    }

    /// The chord that runs the command.
    pub fn chord(&self) -> Option<KeyboardShortcut> {
        self.binding.shortcut.map(KeyboardShortcut::from)
    }

    /// TODO(prose): the chord as the platform writes it, as a menu item for the command shows it.
    pub fn shortcut_text(&self, ctx: &Context) -> Option<String> {
        self.chord().map(|chord| ctx.format_shortcut(&chord))
    }

    pub(crate) fn is_enabled(&self, app: &A) -> bool {
        (self.binding.enabled)(app)
    }

    /// Runs the command as a click does, with nothing from an event.
    pub(crate) fn activate(&self, app: &mut A, shell: &Shell) {
        self.fire(app, None, shell);
    }

    /// `pasted` is what the chord's event carried, if it was one.
    fn fire(&self, app: &mut A, pasted: Option<Paste>, shell: &Shell) {
        match self.run {
            Run::Plain(run) => run(app),
            Run::Shell(run) => run(app, shell),
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

    /// The command's chord, if a press of it would run the command now.
    fn armed(&self, typing: Typing, app: &A) -> Option<KeyboardShortcut> {
        let shortcut = self.chord()?;
        if typing == Typing::Yes && !self.fires_while_typing(shortcut) {
            return None;
        }
        self.is_enabled(app).then_some(shortcut)
    }

    fn fires_while_typing(&self, shortcut: KeyboardShortcut) -> bool {
        match self.binding.in_text {
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
    pub fn add<K: Key>(mut self, id: K, command: impl Into<Command<A>>) -> Self {
        self.entries.push(Entry {
            id: child_id(Id::NULL, &id),
            readable: format!("{id:?}"),
            command: command.into(),
        });
        self
    }

    /// The commands, in the order they are tried, for a reader of the table
    /// other than the menus and the dispatcher, such as a native menu bar.
    pub fn iter(&self) -> impl Iterator<Item = &Command<A>> {
        self.entries.iter().map(|entry| &entry.command)
    }

    /// TODO(prose): the command added under `id`, or `None` if there is none.
    pub fn get<K: Key>(&self, id: K) -> Option<&Command<A>> {
        self.find(child_id(Id::NULL, &id))
    }

    pub(crate) fn find(&self, id: Id) -> Option<&Command<A>> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| &entry.command)
    }

    /// Reports what makes a table ambiguous to dispatch: the same key
    /// twice, or the same chord twice. The first entry of a key or chord is
    /// the one that runs.
    #[track_caller]
    pub(crate) fn validate(&self, ctx: &Context) {
        self.validate_on(ctx, Host::CURRENT);
    }

    #[track_caller]
    fn validate_on(&self, ctx: &Context, host: Host) {
        for (at, entry) in self.entries.iter().enumerate() {
            let Entry {
                readable, command, ..
            } = entry;
            let earlier = self.entries.get(..at).unwrap_or_default();
            if let Some(first) = earlier.iter().find(|other| other.id == entry.id) {
                report(ctx, Problem::DuplicateCommand, Some(entry.id), || {
                    format!(
                        "command {readable} is added twice (first as {}); the first runs",
                        first.readable
                    )
                });
            }
            let Some(shortcut) = command.chord() else {
                continue;
            };
            if host == Host::Web && cfg!(debug_assertions) && browser_keeps(shortcut) {
                report(ctx, Problem::BrowserChord, Some(entry.id), || {
                    format!(
                        "command {readable} is bound to {}, which the browser keeps: on the web the chord never reaches the app",
                        ctx.format_shortcut(&shortcut)
                    )
                });
            }
            if let Some(other) = earlier.iter().find(|other| {
                other
                    .command
                    .chord()
                    .is_some_and(|other| same_chord(other, shortcut))
            }) {
                report(ctx, Problem::DuplicateChord, Some(entry.id), || {
                    format!(
                        "commands {} and {readable} are bound to the same chord; the first runs",
                        other.readable
                    )
                });
            }
        }
    }
}

fn same_chord(a: KeyboardShortcut, b: KeyboardShortcut) -> bool {
    presses(b, a.logical_key, a.modifiers)
}

/// Whether pressing `key` with `held` is a press of `chord`, its modifiers
/// matched exactly.
pub(crate) fn presses(chord: KeyboardShortcut, key: EguiKey, held: Modifiers) -> bool {
    key == chord.logical_key && held.matches_exact(chord.modifiers)
}

/// Whether `chord` arrives as a cut, copy or paste event rather than a key
/// press: `Cmd+X`, `Cmd+C` or `Cmd+V`.
pub(crate) fn is_clipboard_chord(chord: KeyboardShortcut) -> bool {
    matches!(chord.logical_key, EguiKey::X | EguiKey::C | EguiKey::V)
        && Modifiers::COMMAND.matches_exact(chord.modifiers)
}

/// Where the application runs, for what reaches it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Host {
    Web,
    Native,
}

impl Host {
    const CURRENT: Self = if cfg!(target_arch = "wasm32") {
        Self::Web
    } else {
        Self::Native
    };
}

/// Whether a browser acts on `chord` before the page sees it, so a page
/// cannot take it: closing, opening and switching tabs and windows, and
/// quitting.
fn browser_keeps(chord: KeyboardShortcut) -> bool {
    let KeyboardShortcut {
        modifiers,
        logical_key,
    } = chord;
    let command = modifiers.command || modifiers.ctrl || modifiers.mac_cmd;
    if !command || modifiers.alt {
        return false;
    }
    match logical_key {
        EguiKey::W | EguiKey::T | EguiKey::N | EguiKey::Q | EguiKey::Tab => true,
        EguiKey::OpenBracket | EguiKey::CloseBracket => modifiers.shift,
        _ => false,
    }
}

/// What a walk of the live tables does with each command a chord would run
/// now.
pub(crate) enum Visit<'v> {
    /// Runs the command if the frame pressed its chord.
    Fire,
    /// Collects the chord.
    Claim(&'v mut Vec<KeyboardShortcut>),
}

/// What runs a table's chords on the application data.
pub(crate) type Dispatch<A> = Box<dyn Fn(&Context, &Shell, Typing, &mut A, &mut Visit<'_>)>;

/// A command table as the draw runs it: the root it was declared in and, for
/// a mount's, the mount that focus must be within.
pub(crate) struct Declared<A> {
    pub(crate) root: usize,
    pub(crate) within: Option<Within>,
    pub(crate) table: Table<A>,
}

/// A table of the application data's own type, or a mount's, which reaches
/// its data through the mount's lens.
pub(crate) enum Table<A> {
    Own(Commands<A>),
    Lifted(Dispatch<A>),
}

/// The mount a table was declared in, and how many mounts deep it is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Within {
    pub(crate) mount: Id,
    pub(crate) depth: usize,
}

impl<A> Table<A> {
    fn visit(
        &self,
        ctx: &Context,
        shell: &Shell,
        typing: Typing,
        app: &mut A,
        visit: &mut Visit<'_>,
    ) {
        match self {
            Self::Own(table) => table.visit(ctx, shell, typing, app, visit),
            Self::Lifted(dispatch) => dispatch(ctx, shell, typing, app, visit),
        }
    }
}

/// Walks the tables of the live roots, topmost first, each root's mounts that
/// hold focus before the root's own table, the innermost mount first: a
/// chord goes to the innermost table that has it.
pub(crate) fn visit_all<A>(
    ctx: &Context,
    shell: &Shell,
    typing: Typing,
    (declared, live): (&[Declared<A>], &[usize]),
    app: &mut A,
    visit: &mut Visit<'_>,
) {
    for &root in live {
        let mut here: Vec<&Declared<A>> = declared
            .iter()
            .filter(|table| table.root == root)
            .filter(|table| {
                table
                    .within
                    .is_none_or(|within| shell.mount_selected(within.mount))
            })
            .collect();
        here.sort_by_key(|table| {
            std::cmp::Reverse(table.within.map_or(0, |within| within.depth + 1))
        });
        for table in here {
            table.table.visit(ctx, shell, typing, app, visit);
        }
    }
}

/// A command as `app` has it now, for an item or a button that runs it.
pub(crate) struct Bound<A> {
    pub(crate) command: Command<A>,
    pub(crate) enablement: Enablement,
    pub(crate) state: CommandState,
}

/// The command `id` names in `tables`, from the tables whose chords are live
/// in the root being built, as `app` has it.
pub(crate) fn look_up<A>(
    core: &crate::pipeline::FrameCore<'_>,
    tables: &[(usize, Commands<A>)],
    app: &A,
    id: Id,
) -> Option<Bound<A>> {
    let roots = &core.roots;
    let live = live_roots(
        core.current,
        |root| roots.get(root).and_then(crate::roots::RootDecl::parent),
        |root| roots.get(root).is_some_and(|root| root.is_modal()),
    );
    let command = live
        .into_iter()
        .filter_map(|root| tables.iter().find(|(at, _)| *at == root))
        .find_map(|(_, table)| table.find(id))
        .copied()?;
    Some(Bound {
        command,
        enablement: command.is_enabled(app).into(),
        state: command.state(app),
    })
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

impl<A> Commands<A> {
    /// Visits the commands of the table a chord would run now, in table
    /// order: firing runs each whose chord this frame pressed, once, taking
    /// the key from the frame's input. A chord is left alone for a command
    /// that is disabled, or that yields to a text field that has the
    /// keyboard.
    pub(crate) fn visit(
        &self,
        ctx: &Context,
        shell: &Shell,
        typing: Typing,
        app: &mut A,
        visit: &mut Visit<'_>,
    ) {
        for Entry { command, .. } in &self.entries {
            let Some(shortcut) = command.armed(typing, app) else {
                continue;
            };
            match visit {
                Visit::Fire => {
                    if let Some(press) = consume(ctx, shortcut) {
                        command.fire(app, press.pasted(), shell);
                        ctx.request_repaint();
                    }
                }
                Visit::Claim(chords) => chords.push(shortcut),
            }
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
    let logical_key = shortcut.logical_key;
    let clipboard = |key| logical_key == key && is_clipboard_chord(shortcut);
    ctx.input_mut(|input| {
        let mut pressed = None;
        input.events.retain(|event| {
            let press = match event {
                Event::Key {
                    key,
                    modifiers: held,
                    pressed: true,
                    ..
                } => presses(shortcut, *key, *held).then_some(Press::Key),
                Event::Cut => clipboard(EguiKey::X).then_some(Press::Key),
                Event::Copy => clipboard(EguiKey::C).then_some(Press::Key),
                Event::Paste(_) | Event::PasteImage(_) => clipboard(EguiKey::V)
                    .then(|| Paste::from_event(event).map(Press::Paste))
                    .flatten(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Hash, Debug)]
    enum Cmd {
        Close,
        Grid,
    }

    fn chord(modifiers: Modifiers, key: EguiKey) -> Chord {
        Chord::new(modifiers, key).expect("not a chord the library keeps")
    }

    fn table(close: Chord) -> Commands<()> {
        Commands::new()
            .add(Cmd::Close, Command::new("Close", |_| {}).shortcut(close))
            .add(
                Cmd::Grid,
                Command::new("Grid", |_| {})
                    .shortcut(chord(Modifiers::COMMAND | Modifiers::ALT, EguiKey::G)),
            )
    }

    fn browser_chord_reports(ctx: &Context) -> Vec<crate::Report> {
        crate::report::reports(ctx)
            .into_iter()
            .filter(|report| report.problem == Problem::BrowserChord)
            .collect()
    }

    #[test]
    fn a_command_on_a_chord_the_browser_keeps_reports_on_the_web() {
        let ctx = Context::default();
        table(chord(Modifiers::COMMAND, EguiKey::W)).validate_on(&ctx, Host::Web);
        let reports = browser_chord_reports(&ctx);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].message.contains("Close") && reports[0].message.contains("browser keeps"),
            "{}",
            reports[0].message
        );
    }

    #[test]
    fn a_chord_the_browser_keeps_does_not_report_natively() {
        let ctx = Context::default();
        table(chord(Modifiers::COMMAND, EguiKey::W)).validate_on(&ctx, Host::Native);
        assert!(browser_chord_reports(&ctx).is_empty());
    }

    #[test]
    fn the_browser_keeps_tab_and_window_chords_only() {
        let kept = [
            (Modifiers::COMMAND, EguiKey::W),
            (Modifiers::COMMAND, EguiKey::T),
            (Modifiers::COMMAND, EguiKey::N),
            (Modifiers::COMMAND, EguiKey::Q),
            (Modifiers::COMMAND | Modifiers::SHIFT, EguiKey::T),
            (Modifiers::CTRL, EguiKey::Tab),
            (Modifiers::CTRL | Modifiers::SHIFT, EguiKey::Tab),
            (Modifiers::COMMAND | Modifiers::SHIFT, EguiKey::OpenBracket),
            (Modifiers::COMMAND | Modifiers::SHIFT, EguiKey::CloseBracket),
        ];
        for (modifiers, key) in kept {
            assert!(
                browser_keeps(KeyboardShortcut::new(modifiers, key)),
                "{modifiers:?} {key:?}"
            );
        }
        let free = [
            (Modifiers::COMMAND | Modifiers::ALT, EguiKey::G),
            (Modifiers::COMMAND | Modifiers::ALT, EguiKey::W),
            (Modifiers::NONE, EguiKey::W),
            (Modifiers::COMMAND, EguiKey::OpenBracket),
            (Modifiers::NONE, EguiKey::Tab),
        ];
        for (modifiers, key) in free {
            assert!(
                !browser_keeps(KeyboardShortcut::new(modifiers, key)),
                "{modifiers:?} {key:?}"
            );
        }
    }
}
