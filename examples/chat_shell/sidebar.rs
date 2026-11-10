//! The left pane: actions, navigation, recent sessions and the account.

use core::time::Duration;

use bullseye::{Builder, NodeUi, Scope, Tw, tw};

use super::common;
use super::icons::{self, Glyph};
use super::state::{App, Pane, Session, Status};
use super::theme::{Role, Theme, Tok};
use super::view::Part;

type B<'b, 'f> = Builder<'b, 'f, App>;
type S<'b, 'f> = Scope<'b, 'f, App>;

const FAST: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Link {
    New,
    Search,
    Projects,
    Artifacts,
    Customize,
    More,
}

impl Link {
    fn label(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Search => "Search",
            Self::Projects => "Projects",
            Self::Artifacts => "Artifacts",
            Self::Customize => "Customize",
            Self::More => "More",
        }
    }

    fn glyph(self) -> Glyph {
        match self {
            Self::New => Glyph::Plus,
            Self::Search => Glyph::Search,
            Self::Projects => Glyph::Folder,
            Self::Artifacts => Glyph::Cube,
            Self::Customize => Glyph::Sliders,
            Self::More => Glyph::Dots,
        }
    }

    fn hint(self) -> Option<&'static str> {
        match self {
            Self::New => Some("Ctrl+N"),
            Self::Search => Some("Ctrl+K"),
            _ => None,
        }
    }
}

/// The look every row shares; `hover` is how it responds to the pointer.
///
/// ```html
/// <div class="flex items-center gap-2.5 h-8 px-2.5 rounded-md text-muted-foreground">...</div>
/// ```
fn row(k: &Theme) -> Tw {
    tw().flex()
        .items_center()
        .gap(2.5)
        .h_px(32.0)
        .px(2.5)
        .rounded_md()
        .text(k[Tok::InkMid])
        .role(k[Role::Body].clone())
}

/// A link row: icon, label and an optional keyboard hint on the right.
///
/// ```html
/// <a class="flex items-center gap-2.5 h-8 px-2.5 rounded-md text-muted-foreground
///           hover:bg-accent hover:text-foreground transition duration-100">
///   <svg class="size-4 shrink-0"/> <span class="grow truncate">New</span>
///   <span class="text-[11px] text-muted-foreground">Ctrl+N</span>
/// </a>
/// ```
fn link(b: &mut B<'_, '_>, k: &Theme, link: Link) {
    b.div_id(
        Part::Link(link),
        row(k)
            .hover(|p| p.bg(k[Tok::Hover]).text(k[Tok::Ink]))
            .transition(FAST),
        |b| {
            b.paint(
                tw().size_px(16.0).shrink_0(),
                icons::glyph(link.glyph(), k[Tok::InkMid]),
            );
            b.text(tw().grow().truncate(), link.label());
            if let Some(hint) = link.hint() {
                b.text(tw().role(k[Role::Meta].clone()).text(k[Tok::InkDim]), hint);
            }
        },
    );
}

fn status_color(k: &Theme, status: Status) -> egui::Color32 {
    match status {
        Status::Working => k[Tok::Accent],
        Status::NeedsInput => k[Tok::InkMid],
    }
}

/// A recent session. The selected one is base paint and does not respond to
/// hover; the others do.
///
/// ```html
/// <a class="flex items-center gap-2.5 h-8 px-2.5 rounded-md text-muted-foreground
///           hover:bg-accent hover:text-foreground transition duration-100 cursor-pointer">
///   <span class="grow truncate">Why does the build fail on CI?</span>
///   <span class="size-1.5 rounded-full bg-primary"></span>
/// </a>
/// <a class="... bg-border text-foreground">Refactor the tokenizer</a>
/// ```
fn recent(b: &mut B<'_, '_>, k: &Theme, session: &Session, selected: bool) {
    let key = session.key;
    let look = row(k).sense_click();
    let look = if selected {
        look.bg(k[Tok::Selected]).text(k[Tok::Ink])
    } else {
        look.hover(|p| p.bg(k[Tok::Hover]).text(k[Tok::Ink]))
            .transition(FAST)
    };
    b.div_id(key, look, |b| {
        b.text(tw().grow().truncate(), session.title);
        if let Some(status) = session.status {
            b.rule(
                tw().size_px(6.0)
                    .shrink_0()
                    .rounded_full()
                    .bg(status_color(k, status)),
            );
        }
    })
    .draw(move |app: &mut App, ui| {
        if ui.node_response().clicked() {
            app.selected = key;
        }
    });
}

/// ```html
/// <div class="flex flex-col gap-2 pt-2 border-t border-border">
///   <div class="flex items-center gap-2.5 px-2.5">
///     <div class="flex items-center justify-center size-8 rounded-full bg-primary text-primary-foreground">A</div>
///     <div class="flex flex-col"><span>Alex Doe</span><span class="text-[11px] text-muted-foreground">Free plan</span></div>
///   </div>
///   <div class="tab-list">System Light Dark</div>
/// </div>
/// ```
fn account(b: &mut B<'_, '_>, k: &Theme) {
    b.div_id(
        Part::Account,
        tw().flex()
            .flex_col()
            .gap(2.0)
            .pt(2.0)
            .border_t(1.0)
            .border_color(k[Tok::Line]),
        |b| {
            b.div(tw().flex().items_center().gap(2.5).px(2.5), |b| {
                b.div(
                    tw().flex()
                        .items_center()
                        .justify_center()
                        .size_px(32.0)
                        .shrink_0()
                        .rounded_full()
                        .bg(k[Tok::Accent])
                        .text(k[Tok::OnAccent])
                        .role(k[Role::Title].clone()),
                    |b| b.text(tw(), "A"),
                );
                b.div(tw().flex().flex_col(), |b| {
                    b.text(tw(), "Alex Doe");
                    b.text(
                        tw().role(k[Role::Meta].clone()).text(k[Tok::InkDim]),
                        "Free plan",
                    );
                });
            });
            b.div(tw().flex().flex_col().gap(2.0), |b| {
                common::style_toggle(b, Part::Style);
                common::theme_toggle(b, Part::Appearance);
            });
        },
    );
}

/// ```html
/// <aside class="flex flex-col shrink-0 gap-0.5 p-2 bg-card" style="width: 260px">
///   <h1 class="px-2.5 py-2 text-[15px]">Assistant</h1>
///   <a>New</a> <a>Search</a>
///   <hr class="my-2"> <a>Projects</a> <a>Artifacts</a> <a>Customize</a> <a>More</a>
///   <h2 class="px-2.5 pt-4 pb-1 text-[11px] text-muted-foreground">Recents</h2>
///   <div class="flex flex-col grow min-h-0 overflow-hidden">...</div>
///   <div>account</div>
/// </aside>
/// ```
pub fn sidebar(s: &mut S<'_, '_>, app: &App) {
    let k = &Theme::of(s.ctx());
    s.text(
        tw().role(k[Role::Title].clone()).px(2.5).py(2.0),
        "Assistant",
    );
    link(s, k, Link::New);
    link(s, k, Link::Search);
    s.rule(tw().h_px(1.0).bg(k[Tok::Line]).my(2.0).mx(2.5));
    for next in [Link::Projects, Link::Artifacts, Link::Customize, Link::More] {
        link(s, k, next);
    }
    s.text(
        tw().role(k[Role::Meta].clone())
            .text(k[Tok::InkDim])
            .px(2.5)
            .pt(4.0)
            .pb(1.0),
        "Recents",
    );
    s.div(
        tw().flex()
            .flex_col()
            .gap(0.5)
            .grow()
            .min_h_0()
            .overflow_hidden(),
        |b| {
            for session in &app.sessions {
                recent(b, k, session, session.key == app.selected);
            }
        },
    );
    account(s, k);
}

/// The sidebar's box, which its caller keys.
pub fn sidebar_style(k: &Theme, app: &App) -> Tw {
    tw().flex()
        .flex_col()
        .w_px(app.width(Pane::Sidebar).into())
        .shrink_0()
        .gap(0.5)
        .p(2.0)
        .bg(k[Tok::Panel])
        .overflow_hidden()
}
