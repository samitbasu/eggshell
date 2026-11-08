//! The three-pane frame and the splitters between the panes.

use bullseye::{Builder, NodeUi, tw};
use egui::CursorIcon;

use super::centre::centre;
use super::details::details;
use super::sidebar::{Link, sidebar};
use super::state::{App, Pane};
use super::theme::Tok;

type B<'b, 'f> = Builder<'b, 'f, App>;

/// The keys of the identified nodes the tests and the draw closures name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Sidebar,
    Centre,
    Details,
    Splitter(Pane),
    Link(Link),
    Account,
    TopBar,
    ModelChip,
    Conversation,
    Composer,
    Attach,
    Send,
    Field,
    Meter,
}

/// A thin node between two panes: a drag handle with a one-point line that
/// lights up under the pointer.
///
/// ```html
/// <div class="group flex justify-center w-1.5 shrink-0 cursor-col-resize">
///   <div class="w-px h-full bg-[#383838] group-hover:bg-[#6f8fb3]"></div>
/// </div>
/// ```
fn splitter(b: &mut B<'_, '_>, app: &App, pane: Pane) {
    let k = &app.theme;
    b.div_id(
        Part::Splitter(pane),
        tw().flex()
            .justify_center()
            .w_px(6.0)
            .shrink_0()
            .bg(k[Tok::Ground])
            .sense_drag()
            .group(),
        move |app: &mut App, ui| {
            let response = ui.node_response();
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            let Some(pointer) = response.interact_pointer_pos() else {
                return;
            };
            if response.drag_started() {
                app.begin_drag(pane, pointer.x);
            }
            if response.dragged() {
                app.drag_to(pointer.x);
            }
            if response.drag_stopped() {
                app.end_drag();
            }
        },
        |b| {
            b.rule(
                tw().w_px(1.0)
                    .h_full()
                    .bg(k[Tok::Line])
                    .group_hover(|p| p.bg(k[Tok::Accent])),
            );
        },
    );
}

/// ```html
/// <div class="flex size-full overflow-hidden bg-[#1f1f1f] text-[#ececec]">
///   <aside/> <div class="splitter"/> <main/> <div class="splitter"/> <aside/>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, app: &App) {
    b.div(
        tw().flex()
            .size_full()
            .overflow_hidden()
            .bg(app.theme[Tok::Ground])
            .text(app.theme[Tok::Ink]),
        |b| {
            sidebar(b, app);
            splitter(b, app, Pane::Sidebar);
            centre(b, app);
            splitter(b, app, Pane::Details);
            details(b, app);
        },
    );
}
