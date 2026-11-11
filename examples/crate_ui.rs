//! Port of the crate pages of hello_egui's fancy example: a scrolling page of
//! cards, one for each crate, each with a row of links and a row of tags that
//! wraps.
//!
//! After the example in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT) by Lucas Meurer, rebuilt with eggshell rather than copied. The
//! original shows a crate's README under its links and, below each demo, the
//! crates it uses as pills in a wrapping row (`crate_usage_ui`); here every
//! crate is a card with its description, its links and the pills of the crates
//! it is shown with. The tags wrap at the card's width, which the scroll
//! container gives the page. Clicking a tag selects it.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Tw, tw};
use eggshell_ui_shadcn::Tokens;

type B<'b, 'f> = Builder<'b, 'f, State>;

/// `(name, description, the crates it is used with)`.
const CRATES: [(&str, &str, &[&str]); 11] = [
    (
        "egui_animation",
        "Animation utilities for egui",
        &["egui_flex", "egui_router"],
    ),
    (
        "egui_dnd",
        "Drag and drop sorting for egui",
        &["egui_animation", "egui_flex", "egui_form"],
    ),
    (
        "egui_flex",
        "Flex layout for egui",
        &[
            "egui_dnd",
            "egui_form",
            "egui_virtual_list",
            "egui_infinite_scroll",
            "egui_suspense",
        ],
    ),
    (
        "egui_form",
        "Form validation for egui",
        &["egui_flex", "egui_suspense"],
    ),
    (
        "egui_inbox",
        "Channel with ergonomics optimized for egui",
        &["egui_suspense", "egui_router", "egui_infinite_scroll"],
    ),
    (
        "egui_infinite_scroll",
        "Infinite scroll widget for egui",
        &[
            "egui_thumbhash",
            "egui_pull_to_refresh",
            "egui_virtual_list",
            "egui_inbox",
        ],
    ),
    (
        "egui_pull_to_refresh",
        "Pull to refresh widget for egui",
        &["egui_infinite_scroll"],
    ),
    (
        "egui_router",
        "SPA-like router for egui",
        &[
            "egui_inbox",
            "egui_animation",
            "egui_suspense",
            "egui_form",
            "egui_flex",
            "egui_dnd",
        ],
    ),
    (
        "egui_suspense",
        "Suspense widget for egui for ergonomic data fetching",
        &["egui_inbox", "egui_router"],
    ),
    (
        "egui_thumbhash",
        "Image loading and caching for egui",
        &["egui_infinite_scroll", "egui_virtual_list"],
    ),
    (
        "egui_virtual_list",
        "Virtual list widget for egui with support for varying heights and custom layouts",
        &[
            "egui_infinite_scroll",
            "egui_thumbhash",
            "egui_flex",
            "egui_inbox",
        ],
    ),
];

const LINKS: [&str; 3] = ["docs.rs", "crates.io", "github"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Page,
    Card(&'static str),
    Tag(&'static str),
}

#[derive(Default)]
pub struct State {
    pub selected: Option<&'static str>,
}

fn pill(t: &Tokens, selected: bool) -> Tw {
    let pill = tw()
        .px(1.5)
        .py(1.0)
        .rounded_full()
        .border(1.0)
        .border_color(t.border)
        .text(t.foreground)
        .sense_click()
        .hover(|p| p.bg(t.accent));
    if selected { pill.bg(t.accent) } else { pill }
}

/// ```html
/// <section class="flex flex-col gap-2 p-4 rounded-lg border bg-card">
///   <h3>egui_flex</h3> <p class="text-muted-foreground">Flex layout for egui</p>
///   <div class="flex flex-wrap gap-4 text-muted-foreground">docs.rs crates.io github</div>
///   <small>Crates used:</small>
///   <div class="flex flex-wrap gap-1"><button class="rounded-full border px-1.5 py-1">egui_dnd</button> …</div>
/// </section>
/// ```
fn card(
    b: &mut B<'_, '_>,
    state: &State,
    (name, description, tags): (&'static str, &str, &[&'static str]),
) {
    let t = common::tokens(b.ctx());
    b.div_id(
        Part::Card(name),
        tw().flex()
            .flex_col()
            .gap(2.0)
            .p(4.0)
            .rounded_lg()
            .border(1.0)
            .border_color(t.border)
            .bg(t.card),
        |b| {
            b.text(tw().role(common::sized(16.0)), name);
            b.text(tw().text(t.muted_foreground), description);
            b.div(tw().flex().flex_wrap().gap_x(4.0), |b| {
                for link in LINKS {
                    b.text(tw().text(t.muted_foreground).underline(), link);
                }
            });
            b.text(
                tw().role(common::sized(12.0)).text(t.muted_foreground),
                "Crates used:",
            );
            b.div(tw().flex().flex_wrap().gap(1.0), |b| {
                for &tag in tags {
                    b.div_id(Part::Tag(tag), pill(&t, state.selected == Some(tag)), |b| {
                        b.text(tw(), tag)
                    })
                    .on_click(move |app: &mut State| app.selected = Some(tag));
                }
            });
        },
    );
}

/// ```html
/// <div class="size-full overflow-y-auto p-4">
///   <div class="flex flex-col gap-4 w-full max-w-[1000px] mx-auto"> eleven cards </div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Page, tw().size_full().p(4.0)).children(|s| {
        s.div(
            tw().flex()
                .flex_col()
                .gap(4.0)
                .w_full()
                .max_w_px(1000.0)
                .mx_auto()
                .text(t.foreground),
            |s| {
                for entry in CRATES {
                    card(s, state, entry);
                }
            },
        );
    });
}

fn main() -> eframe::Result {
    common::run_app("crate_ui", State::default(), layout)
}
