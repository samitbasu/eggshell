//! What the application keeps between frames.

use bullseye::Progress;
use bullseye::style::Accent;

/// A pane whose width the user drags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pane {
    Sidebar,
    Details,
}

impl Pane {
    const fn limits(self) -> (f32, f32) {
        match self {
            Self::Sidebar => (180.0, 420.0),
            Self::Details => (220.0, 480.0),
        }
    }

    const fn default_width(self) -> f32 {
        match self {
            Self::Sidebar => 260.0,
            Self::Details => 288.0,
        }
    }

    /// Which way a pointer moving right moves the pane's inner edge.
    const fn grows_rightward(self) -> bool {
        matches!(self, Self::Sidebar)
    }
}

/// A pane's width in points, always within its pane's limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneWidth(f32);

impl PaneWidth {
    /// `points` clamped to the limits of `pane`; NaN gives its default.
    pub fn new(pane: Pane, points: f32) -> Self {
        let (min, max) = pane.limits();
        if points.is_nan() {
            Self(pane.default_width())
        } else {
            Self(points.clamp(min, max))
        }
    }
}

impl From<PaneWidth> for f32 {
    fn from(width: PaneWidth) -> f32 {
        width.0
    }
}

/// A splitter drag in progress.
#[derive(Clone, Copy, Debug)]
struct Drag {
    pane: Pane,
    width_at_press: PaneWidth,
    pointer_x_at_press: f32,
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

pub struct App {
    /// The shadcn theme colour that tints the meter, the send button, the
    /// busy markers and focus.
    pub accent: Accent,
    pub sessions: Vec<Session>,
    pub selected: SessionKey,
    pub conversation: Vec<Message>,
    pub context_used: Progress,
    pub draft: String,
    /// An index into [`MODELS`].
    pub model: usize,
    pub mode: Mode,
    sidebar: PaneWidth,
    details: PaneWidth,
    drag: Option<Drag>,
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
            accent: Accent::Blue,
            sessions: vec![
                session(1, "Refactor the tokenizer", None),
                session(2, "Why does the build fail on CI?", Some(Status::Working)),
                session(3, "Plan the release notes", None),
                session(4, "Review the layout cache", Some(Status::NeedsInput)),
                session(5, "Sketch a settings screen", None),
                session(6, "Explain the borrow checker error", None),
            ],
            selected: SessionKey(1),
            conversation: vec![
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
            ],
            context_used: Progress::new(0.42),
            draft: String::new(),
            model: 0,
            mode: Mode::Standard,
            sidebar: PaneWidth::new(Pane::Sidebar, Pane::Sidebar.default_width()),
            details: PaneWidth::new(Pane::Details, Pane::Details.default_width()),
            drag: None,
        }
    }
}

impl App {
    pub fn width(&self, pane: Pane) -> PaneWidth {
        match pane {
            Pane::Sidebar => self.sidebar,
            Pane::Details => self.details,
        }
    }

    fn width_mut(&mut self, pane: Pane) -> &mut PaneWidth {
        match pane {
            Pane::Sidebar => &mut self.sidebar,
            Pane::Details => &mut self.details,
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

    pub fn begin_drag(&mut self, pane: Pane, pointer_x: f32) {
        self.drag = Some(Drag {
            pane,
            width_at_press: self.width(pane),
            pointer_x_at_press: pointer_x,
        });
    }

    /// Resizes the dragged pane so its edge follows the pointer, measured from
    /// where the drag started so that clamping does not accumulate drift.
    pub fn drag_to(&mut self, pointer_x: f32) {
        let Some(Drag {
            pane,
            width_at_press,
            pointer_x_at_press,
        }) = self.drag
        else {
            return;
        };
        let moved = pointer_x - pointer_x_at_press;
        let signed = if pane.grows_rightward() {
            moved
        } else {
            -moved
        };
        *self.width_mut(pane) = PaneWidth::new(pane, f32::from(width_at_press) + signed);
    }

    pub fn end_drag(&mut self) {
        self.drag = None;
    }
}
