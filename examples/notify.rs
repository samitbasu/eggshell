//! Notifications (#347): a transient toast that replaces the last and expires
//! after its dwell, and persistent notices that stack and carry actions.
//!
//! "Show toast" posts a toast bottom-centre: it expires after 4 s, holds while
//! the pointer is over it or focus is inside it, and Esc dismisses it with
//! focus inside. "Post notice" pushes a notice top-right with two actions and a
//! ×; the newest sits nearest the top and three show at once.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;

use eggshell_ui::owned::notify::{Lifetime, Notice, NoticeId, NotifyExt, ScreenAnchor};
use eggshell_ui::theme::variant;
use eggshell_ui::{Builder, tw};

#[derive(Clone, Copy)]
enum Act {
    CopyDetails,
    OpenLog,
}

#[derive(Default)]
pub struct State {
    toast: Option<Notice>,
    notices: Vec<Notice<Act>>,
    next: u64,
    said: String,
}

impl State {
    fn id(&mut self) -> NoticeId {
        self.next += 1;
        NoticeId::new(self.next)
    }
}

pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.div(tw().flex().flex_col().gap(4.0).p(8.0).items_start(), |b| {
        b.text(tw().font_semibold(), "Notifications");
        b.div(tw().flex().gap(2.0), |b| {
            b.button("toast", "Show toast")
                .variant(variant::OUTLINE)
                .on_click(|state: &mut State| {
                    let id = state.id();
                    state.toast = Some(Notice::new(id, format!("Copied ({})", state.next)));
                });
            b.button("notice", "Post notice")
                .variant(variant::OUTLINE)
                .on_click(|state: &mut State| {
                    let id = state.id();
                    state.notices.push(
                        Notice::new(id, "Export failed.")
                            .action("Copy details", Act::CopyDetails)
                            .action("Open log", Act::OpenLog),
                    );
                });
            b.button("clear", "Clear notices")
                .variant(variant::GHOST)
                .on_click(|state: &mut State| state.notices.clear());
        });
        b.text(tw(), format!("Last action: {}", state.said));
    });
    b.notifications("toast-stack", state.toast.as_slice())
        .at(ScreenAnchor::BottomCenter, eggshell_ui::Distance::new(24.0))
        .lifetime(Lifetime::Dwell(Duration::from_millis(4000)))
        .on_dismiss(|state: &mut State, _, _| state.toast = None)
        .add();
    b.notifications("notices-stack", &state.notices)
        .at(ScreenAnchor::TopRight, eggshell_ui::Distance::new(24.0))
        .on_action(|state: &mut State, id, act| {
            state.said = match act {
                Act::CopyDetails => format!("copy details of {id:?}"),
                Act::OpenLog => format!("open the log for {id:?}"),
            };
        })
        .on_dismiss(|state: &mut State, id, _| state.notices.retain(|notice| notice.id != id))
        .add();
}

fn main() -> eframe::Result {
    common::run_app("notify", State::default(), layout)
}
