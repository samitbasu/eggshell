//! The three-pane frame and the splitters between the panes.

use super::common;
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, tw};

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

/// The splitter that sizes `pane`, the one before it or after it.
fn splitter(b: &mut B<'_, '_>, app: &App, pane: Pane) {
    let width = app.width(pane);
    let start = b.splitter(Part::Splitter(pane));
    let (chain, name) = match pane {
        Pane::Sidebar => (start.before(width), "Sidebar"),
        Pane::Details => (start.after(width), "Details"),
    };
    chain
        .named(name)
        .range(pane.limits())
        .reset_to(pane.default_width())
        .on_resize(move |app: &mut App, size| app.set_width(pane, size));
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
pub fn view<'f>(b: &mut B<'_, 'f>, app: &'f App) {
    b.region(Part::Frame, tw().size_full(), move |b, room| {
        frame(b, app, Panes::fitting(room));
    });
}

fn frame<'f>(b: &mut B<'_, 'f>, app: &'f App, panes: Panes) {
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
                splitter(b, app, Pane::Sidebar);
            }
            b.div_id(Part::Centre, centre_style(k), |s| centre(s, app));
            if panes.shows(Pane::Details) {
                splitter(b, app, Pane::Details);
                b.div_id(Part::Details, details_style(k, app), |s| details(s, app));
            }
        },
    );
}
