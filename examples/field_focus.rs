//! Text field focus control: Enter that keeps focus, and focus on demand.
//!
//! Type in the message field and press Enter: the message is sent and the
//! field keeps focus, so you can type the next one. Click the Rename button:
//! the name field takes focus (once per click), wherever it was.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::prim::text_field::Enter;
use eggshell_ui::{Builder, tw};

/// The parts the layout names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Page,
    Log,
    Message,
    Name,
    Rename,
}

#[derive(Default)]
pub struct State {
    pub sent: Vec<String>,
    pub draft: String,
    pub name: String,
    pub focus_name: u32,
}

/// ```html
/// <div class="flex flex-col gap-3 p-4">
///   <p>hello</p> <p>world</p>
///   <input placeholder="Message, Enter sends"/>
///   <div class="flex gap-2"> <input value="Name"/> <button>Rename</button> </div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    let t = common::tokens(b.ctx());
    b.div_id(Part::Page, tw().flex().flex_col().gap(3.0).p(4.0), |s| {
        s.div_id(
            Part::Log,
            tw().flex().flex_col().gap(1.0).min_h_px(80.0),
            |s| {
                for (at, message) in state.sent.iter().enumerate() {
                    s.span(at, tw(), message.clone());
                }
                if state.sent.is_empty() {
                    s.span("none", tw().text(t.muted_foreground), "No messages yet.");
                }
            },
        );
        s.text_field(Part::Message, state.draft.clone())
            .named("Message")
            .hint("Message, Enter sends")
            .on_enter(Enter::KeepFocus)
            .when_submitted(|state: &mut State| {
                let sent = std::mem::take(&mut state.draft);
                if !sent.is_empty() {
                    state.sent.push(sent);
                }
            })
            .on_change(|state, text| state.draft = text);
        s.div(tw().flex().gap(2.0).items_center(), |s| {
            s.text_field(Part::Name, state.name.clone())
                .named("Name")
                .hint("Name")
                .focus_on_change(state.focus_name)
                .classes(tw().w_px(160.0))
                .on_change(|state, text| state.name = text);
            s.button(Part::Rename, "Rename")
                .variant(eggshell_ui_shadcn::variant::OUTLINE)
                .on_click(|state: &mut State| state.focus_name += 1);
        });
    });
}

fn main() -> eframe::Result {
    common::run_app("field_focus", State::default(), layout)
}
