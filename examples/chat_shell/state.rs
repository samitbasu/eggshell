//! What the application keeps between frames.

use std::ops::RangeInclusive;

use eggshell_ui::Progress;
use eggshell_ui::owned::PaneSize;
use eggshell_ui::theme::ThemeColor;
use egui::Vec2;

/// A pane whose width the user drags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pane {
    Sidebar,
    Details,
}

impl Pane {
    pub const fn limits(self) -> RangeInclusive<f32> {
        match self {
            Self::Sidebar => 180.0..=420.0,
            Self::Details => 220.0..=480.0,
        }
    }

    pub const fn default_width(self) -> f32 {
        match self {
            Self::Sidebar => 260.0,
            Self::Details => 288.0,
        }
    }
}

/// Which panes the window leaves room for: the details panel is the first to
/// go, then the sidebar, and the conversation always stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panes {
    All,
    WithoutDetails,
    CentreOnly,
}

impl Panes {
    /// The window width from which the sidebar is shown.
    const SIDEBAR_FROM: f32 = 720.0;
    /// The window width from which the details panel is shown.
    const DETAILS_FROM: f32 = 1000.0;

    /// The panes shown in a window of `room`.
    pub fn fitting(room: Vec2) -> Self {
        if room.x >= Self::DETAILS_FROM {
            Self::All
        } else if room.x >= Self::SIDEBAR_FROM {
            Self::WithoutDetails
        } else {
            Self::CentreOnly
        }
    }

    pub fn shows(self, pane: Pane) -> bool {
        match pane {
            Pane::Sidebar => self != Self::CentreOnly,
            Pane::Details => self == Self::All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionKey(pub u32);

/// The models the top bar's select offers.
pub const MODELS: [&str; 3] = ["Assistant 1", "Assistant 2", "Assistant 3"];

/// How much care the assistant takes with a reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Quick,
    Standard,
    Thorough,
}

impl Mode {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Quick, "Quick"),
        (Self::Standard, "Standard"),
        (Self::Thorough, "Thorough"),
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Working,
    NeedsInput,
}

pub struct Session {
    pub key: SessionKey,
    pub title: &'static str,
    pub status: Option<Status>,
}

pub enum Block {
    Paragraph(String),
    Code(String),
}

pub enum Message {
    Assistant(Vec<Block>),
    User(String),
}

/// A message's data key, given when it joins the conversation and never
/// given again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(u32);

/// The messages, oldest first, each under its own key.
#[derive(Default)]
pub struct Conversation {
    ids: Vec<MessageId>,
    messages: Vec<Message>,
    next: u32,
}

impl Conversation {
    pub fn push(&mut self, message: Message) {
        self.ids.push(MessageId(self.next));
        self.messages.push(message);
        self.next += 1;
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.messages.clear();
    }

    /// The keys of the messages, oldest first.
    pub fn ids(&self) -> &[MessageId] {
        &self.ids
    }

    pub fn get(&self, id: MessageId) -> Option<&Message> {
        let at = self.ids.binary_search(&id).ok()?;
        Some(&self.messages[at])
    }

    pub fn iter(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    pub fn is_first(&self, id: MessageId) -> bool {
        self.ids.first() == Some(&id)
    }

    pub fn is_last(&self, id: MessageId) -> bool {
        self.ids.last() == Some(&id)
    }
}

impl FromIterator<Message> for Conversation {
    fn from_iter<I: IntoIterator<Item = Message>>(messages: I) -> Self {
        let mut conversation = Self::default();
        for message in messages {
            conversation.push(message);
        }
        conversation
    }
}

pub struct App {
    /// The shadcn theme colour that tints the meter, the send button, the
    /// busy markers and focus.
    pub theme_color: ThemeColor,
    pub sessions: Vec<Session>,
    pub selected: SessionKey,
    pub conversation: Conversation,
    pub context_used: Progress,
    pub draft: String,
    /// An index into [`MODELS`].
    pub model: usize,
    pub mode: Mode,
    sidebar: PaneSize,
    details: PaneSize,
    /// The titles of the details sections that are folded.
    folded: Vec<&'static str>,
}

/// How many messages come before the current exchange.
pub const HISTORY: usize = 3_000;

const ASKS: [&str; 6] = [
    "Can the cache keep the last layout when nothing changed?",
    "Why does the list jump when a row grows?",
    "Is there a faster way to measure the text?",
    "What should the parser do with a stray brace?",
    "Could the build skip the unchanged crates?",
    "How do I keep the focus when the panel closes?",
];

const ANSWERS: [&str; 6] = [
    "It can. Keep the inputs the layout depends on next to the result, and compare them before solving again.",
    "The rows above the one you are reading changed height, so everything below them moved. Hold the first visible row in place and the jump goes away.",
    "Measure each word once and reuse it: most frames lay out the same strings at the same size.",
    "Report it with the position it was found at, then skip to the next statement so that one mistake does not hide the rest.",
    "Yes. Hash each crate's sources and its dependencies' hashes, and rebuild only when one changes.",
    "Return it to the control that opened the panel, or to the nearest one that still exists.",
];

const SNIPPETS: [&str; 3] = [
    "if inputs == last.inputs { return last.layout; }",
    "let id = list_id.with(row_key);",
    "focus.restore_to(opener);",
];

/// The messages before the current exchange: questions and their answers,
/// some with code.
fn history() -> impl Iterator<Item = Message> {
    (0..HISTORY).map(|at| {
        let turn = at / 2;
        if at % 2 == 0 {
            return Message::User(ASKS[turn % ASKS.len()].to_owned());
        }
        let mut blocks = vec![paragraph(ANSWERS[turn % ANSWERS.len()])];
        if turn % 3 == 1 {
            blocks.push(code(SNIPPETS[turn % SNIPPETS.len()]));
        }
        if turn % 4 == 2 {
            blocks.push(paragraph(ANSWERS[(turn + 1) % ANSWERS.len()]));
        }
        Message::Assistant(blocks)
    })
}

fn paragraph(text: &str) -> Block {
    Block::Paragraph(text.to_owned())
}

fn code(text: &str) -> Block {
    Block::Code(text.to_owned())
}

impl Default for App {
    fn default() -> Self {
        let session = |key, title, status| Session {
            key: SessionKey(key),
            title,
            status,
        };
        Self {
            theme_color: ThemeColor::Blue,
            sessions: vec![
                session(1, "Refactor the tokenizer", None),
                session(2, "Why does the build fail on CI?", Some(Status::Working)),
                session(3, "Plan the release notes", None),
                session(4, "Review the layout cache", Some(Status::NeedsInput)),
                session(5, "Sketch a settings screen", None),
                session(6, "Explain the borrow checker error", None),
            ],
            selected: SessionKey(1),
            conversation: history()
                .chain([
                Message::User(
                    "The tokenizer allocates a new string for every token. Can we avoid that?"
                        .into(),
                ),
                Message::Assistant(vec![
                    paragraph(
                        "Yes. Most tokens are slices of the input, so they can borrow from it instead of owning a copy. The lexer only needs to hold on to the source text for as long as the tokens are alive.",
                    ),
                    paragraph(
                        "The change is small. Give the token type a lifetime and store the slice:",
                    ),
                    code("pub struct Token<'src> { text: &'src str }"),
                    paragraph(
                        "Callers that keep tokens around after the source is dropped will need to convert them to owned values at that point.",
                    ),
                ]),
                Message::User("Good. Please update the parser tests too.".into()),
                Message::Assistant(vec![paragraph(
                    "Done. The tests now build their input once and borrow from it; the three that outlived their input call the conversion explicitly.",
                )]),
            ])
            .collect(),
            context_used: Progress::new(0.42),
            draft: String::new(),
            model: 0,
            mode: Mode::Standard,
            sidebar: PaneSize::new(Pane::Sidebar.default_width()),
            details: PaneSize::new(Pane::Details.default_width()),
            folded: Vec::new(),
        }
    }
}

impl App {
    pub fn is_folded(&self, section: &'static str) -> bool {
        self.folded.contains(&section)
    }

    pub fn toggle_section(&mut self, section: &'static str) {
        if let Some(at) = self.folded.iter().position(|&folded| folded == section) {
            self.folded.remove(at);
        } else {
            self.folded.push(section);
        }
    }

    pub fn width(&self, pane: Pane) -> PaneSize {
        match pane {
            Pane::Sidebar => self.sidebar,
            Pane::Details => self.details,
        }
    }

    pub fn set_width(&mut self, pane: Pane, size: PaneSize) {
        match pane {
            Pane::Sidebar => self.sidebar = size,
            Pane::Details => self.details = size,
        }
    }

    /// Appends the trimmed draft to the conversation as a user message and
    /// clears it; an empty draft does nothing.
    pub fn send(&mut self) {
        let text = self.draft.trim();
        if text.is_empty() {
            return;
        }
        self.conversation.push(Message::User(text.to_owned()));
        self.draft.clear();
    }
}
