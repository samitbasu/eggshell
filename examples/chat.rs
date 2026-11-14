//! A chat: a long history of bubbles of wrapping text, the other people's on
//! the left with their names and mine on the right, and messages that keep
//! arriving.
//!
//! A port of the chat in hello_egui's fancy example
//! (github.com/lucasmerlin/hello_egui, `fancy-example/src/chat.rs`, MIT),
//! rebuilt with `virtual_list`. The original loads its history a page at a
//! time as it is scrolled up and sticks to the bottom as messages arrive;
//! here the whole history is a virtual list that opens at its end
//! (`start_at_end`) and holds it while messages arrive, and the history is
//! made up rather than copied. A new message arrives every two seconds
//! until the last of them.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Scope, Tw, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How many messages come before the ones that arrive.
pub const HISTORY: u32 = 2_000;

/// How far apart, in seconds, the arriving messages are.
pub const ARRIVAL: f64 = 2.0;

const PEOPLE: [&str; 2] = ["Ada", "Ben"];

const LINES: [&str; 8] = [
    "Have you tried the new layout?",
    "Yes, the list keeps up even with thousands of rows.",
    "Only the rows on screen are built, so it costs the same however long it gets.",
    "Nice. Does it remember how tall each row was?",
    "It does, until the width changes; then it measures them again.",
    "And when a row above me grows, what I am reading stays put.",
    "That was the part I worried about.",
    "Me too, but it holds.",
];

const ARRIVING: [&str; 4] = [
    "Still there?",
    "I just pushed the fix for the end of the list.",
    "Scroll up and back down: it lands on the last message.",
    "See you tomorrow!",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    List,
    Clock,
}

/// A message's key: messages are only ever appended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MessageId(pub u32);

pub struct Message {
    /// Who sent it, or `None` for me.
    pub from: Option<&'static str>,
    pub text: String,
}

pub struct State {
    pub ids: Vec<MessageId>,
    pub messages: Vec<Message>,
    /// When the first frame was drawn, on egui's clock.
    started: Option<f64>,
    arrived: usize,
}

impl State {
    fn push(&mut self, message: Message) {
        self.ids.push(MessageId(self.messages.len() as u32));
        self.messages.push(message);
    }

    /// Adds the messages due by `now`, and returns how long until the next.
    fn arrive(&mut self, now: f64) -> Option<f64> {
        let started = *self.started.get_or_insert(now);
        while self.arrived < ARRIVING.len() && now >= started + ARRIVAL * (self.arrived + 1) as f64
        {
            self.push(Message {
                from: Some(PEOPLE[self.arrived % PEOPLE.len()]),
                text: ARRIVING[self.arrived].to_owned(),
            });
            self.arrived += 1;
        }
        (self.arrived < ARRIVING.len()).then(|| started + ARRIVAL * (self.arrived + 1) as f64 - now)
    }
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            ids: Vec::new(),
            messages: Vec::new(),
            started: None,
            arrived: 0,
        };
        for at in 0..HISTORY as usize {
            let from = match at % 3 {
                0 => None,
                person => Some(PEOPLE[person - 1]),
            };
            let line = LINES[at % LINES.len()];
            let text = if at % 5 == 4 {
                format!("{line} {}", LINES[(at + 3) % LINES.len()])
            } else {
                line.to_owned()
            };
            state.push(Message { from, text });
        }
        state
    }
}

/// My bubbles are on the right and the others' on the left, with the corner
/// nearest the speaker square.
fn bubble(t: &eggshell_ui_shadcn::Tokens, mine: bool) -> Tw {
    let bubble = tw()
        .flex()
        .flex_col()
        .gap(0.5)
        .max_w_px(420.0)
        .px(2.0)
        .py(2.0)
        .m(2.0)
        .rounded_lg();
    if mine {
        bubble.self_end().rounded_tr_px(0.0).bg(t.muted)
    } else {
        bubble
            .self_start()
            .rounded_tl_px(0.0)
            .border(1.0)
            .border_color(t.border)
            .bg(t.card)
    }
}

/// ```html
/// <div class="flex flex-col">
///   <div class="self-start max-w-[420px] m-2 px-2 py-2 rounded-lg rounded-tl-none border bg-card">
///     <strong>Ada</strong> <p>Have you tried the new layout?</p>
///   </div>
/// </div>
/// ```
fn message<'f>(state: &'f State) -> impl FnMut(&mut Scope<'_, 'f, State>, MessageId) + 'f {
    move |s, MessageId(index)| {
        let t = common::tokens(s.ctx());
        let message = &state.messages[index as usize];
        s.div(tw().flex().flex_col(), |s| {
            s.div(bubble(&t, message.from.is_none()), |s| {
                if let Some(from) = message.from {
                    s.text(tw().text(t.muted_foreground), from);
                }
                s.text(tw(), message.text.as_str());
            });
        });
    }
}

/// ```html
/// <div class="flex flex-col size-full p-4">
///   <div class="grow min-h-0 overflow-y-auto rounded-md border">messages</div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, state: &'f State) {
    let t = common::tokens(b.ctx());
    b.div(tw().flex().flex_col().size_full().p(4.0), |b| {
        b.custom(Part::Clock, tw().size_px(0.0))
            .ui(|state: &mut State, ui| {
                if let Some(next) = state.arrive(ui.input(|input| input.time)) {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs_f64(next));
                }
            });
        b.virtual_list(
            Part::List,
            tw().grow()
                .min_h_0()
                .rounded_md()
                .border(1.0)
                .border_color(t.border),
            &state.ids,
        )
        .estimate_px(64.0)
        .start_at_end()
        .items(message(state));
    });
}

fn main() -> eframe::Result {
    common::run_app("chat", State::default(), layout)
}
