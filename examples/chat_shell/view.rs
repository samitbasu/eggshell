//! The three-pane frame and the splitters between the panes.

use super::common;
use eggshell_ui::{Builder, tw};
use egui::CursorIcon;

use super::centre::{centre, centre_style};
use super::details::{details, details_style};
use super::sidebar::{Link, sidebar, sidebar_style};
use super::state::{App, Pane, Panes};
use super::theme::{Theme, Tok};

type B<'b, 'f> = Builder<'b, 'f, App>;

/// The keys of the identified nodes the tests and the draw closures name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Frame,
    Sidebar,
    Centre,
    Details,
    Splitter(Pane),
    Link(Link),
    Account,
    Appearance,
    Style,
    TopBar,
    Model,
    Mode,
    Conversation,
    Recents,
    Composer,
    Attach,
    Send,
    Field,
    Meter,
    Group,
    Row(&'static str),
}

/// A thin node between two panes: a drag handle with a one-point line that
/// lights up under the pointer.
///
/// ```html
/// <div class="group flex justify-center w-1.5 shrink-0 cursor-col-resize">
///   <div class="w-px h-full bg-border group-hover:bg-primary"></div>
/// </div>
/// ```
fn splitter(b: &mut B<'_, '_>, pane: Pane) {
    let k = &Theme::of(b.ctx());
    b.div_id(
        Part::Splitter(pane),
        tw().flex()
            .justify_center()
            .w_px(6.0)
            .shrink_0()
            .bg(k[Tok::Ground])
            .sense_drag()
            .focusable_none()
            .group(),
        |b| {
            b.rule(
                tw().w_px(1.0)
                    .h_full()
                    .bg(k[Tok::Line])
                    .group_hover(|p| p.bg(k[Tok::Accent])),
            );
        },
    )
    .on_response(move |app: &mut App, response| {
        if response.hovered() || response.dragged() {
            response.ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
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
    });
}

/// Gives egui's stock widgets, which the select and the composer's field are,
/// the preset's look.
pub fn prepare(ctx: &egui::Context, app: &App) {
    common::use_theme_color(ctx, app.theme_color);
}

/// The three panes as far as the window leaves room for them: the region
/// holds the whole window, so the panes shown follow the width it has.
///
/// ```html
/// <div class="flex size-full overflow-hidden bg-background text-foreground">
///   <aside/> <div class="splitter"/> <main/> <div class="splitter"/> <aside/>
/// </div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, app: &'f App) {
    b.region(Part::Frame, tw().size_full(), move |b, room| {
        frame(b, app, Panes::fitting(room));
    });
}

fn frame(b: &mut B<'_, '_>, app: &App, panes: Panes) {
    let k = &Theme::of(b.ctx());
    b.div(
        tw().flex()
            .size_full()
            .overflow_hidden()
            .bg(k[Tok::Ground])
            .text(k[Tok::Ink]),
        |b| {
            if panes.shows(Pane::Sidebar) {
                b.div_id(Part::Sidebar, sidebar_style(k, app), |s| sidebar(s, app));
                splitter(b, Pane::Sidebar);
            }
            b.div_id(Part::Centre, centre_style(k), |s| centre(s, app));
            if panes.shows(Pane::Details) {
                splitter(b, Pane::Details);
                b.div_id(Part::Details, details_style(k, app), |s| details(s, app));
            }
        },
    );
}
