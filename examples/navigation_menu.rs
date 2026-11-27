//! A navigation menu: shadcn's demo, a site's top bar with two panels of
//! links and a plain link, over the page it shows.
//!
//! Rest the pointer on "Getting started" or "Components": its panel opens
//! after 200 ms, and while it is open the other trigger opens at once. Click
//! a trigger to keep its panel open; click it again to close it. Tab moves
//! along the row and into an open panel, the arrow keys move along the row
//! and within a panel, Down on a trigger opens it and goes in, and Esc
//! closes it and puts the focus back on the trigger.

use eggshell_examples::common;

use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, tw};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Page {
    #[default]
    Introduction,
    Installation,
    Typography,
    AlertDialog,
    HoverCard,
    Progress,
    ScrollArea,
    Tabs,
    Tooltip,
    Docs,
}

#[derive(Default)]
struct State {
    page: Page,
    indicator: bool,
}

const STARTED: [(Page, &str, &str); 3] = [
    (
        Page::Introduction,
        "Introduction",
        "Re-usable components built using Radix UI and Tailwind CSS.",
    ),
    (
        Page::Installation,
        "Installation",
        "How to install dependencies and structure your app.",
    ),
    (
        Page::Typography,
        "Typography",
        "Styles for headings, paragraphs, lists...etc",
    ),
];

const COMPONENTS: [(Page, &str, &str); 6] = [
    (
        Page::AlertDialog,
        "Alert Dialog",
        "A modal dialog that interrupts the user with important content and expects a response.",
    ),
    (
        Page::HoverCard,
        "Hover Card",
        "For sighted users to preview content available behind a link.",
    ),
    (
        Page::Progress,
        "Progress",
        "Displays an indicator showing the completion progress of a task, typically displayed as a progress bar.",
    ),
    (
        Page::ScrollArea,
        "Scroll-area",
        "Visually or semantically separates content.",
    ),
    (
        Page::Tabs,
        "Tabs",
        "A set of layered sections of content—known as tab panels—that are displayed one at a time.",
    ),
    (
        Page::Tooltip,
        "Tooltip",
        "A popup that displays information related to an element when the element receives keyboard focus or the mouse hovers over it.",
    ),
];

fn links(
    b: &mut Builder<'_, '_, State>,
    current: Page,
    links: &[(Page, &'static str, &'static str)],
) {
    for &(page, title, description) in links {
        b.nav_link(title, title)
            .description(description)
            .current(page == current)
            .on_click(move |state: &mut State| state.page = page);
    }
}

fn view(b: &mut Builder<'_, '_, State>, state: &State) {
    let current = state.page;
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(6.0).p(8.0).items_start(), |b| {
        let menu = b
            .navigation_menu("site")
            .named("Main")
            .panel("started", "Getting started", move |p| {
                p.div(tw().flex().flex_col().gap(1.0).w_px(300.0), |g| {
                    links(g, current, &STARTED);
                });
            })
            .panel("components", "Components", move |p| {
                p.div(tw().grid().grid_cols(2).gap(2.0).w_px(500.0), |g| {
                    links(g, current, &COMPONENTS);
                });
            })
            .link("docs", "Docs", move |link| {
                link.current(current == Page::Docs)
                    .on_click(|state: &mut State| state.page = Page::Docs);
            });
        if state.indicator {
            menu.indicator().add();
        } else {
            menu.add();
        }
        b.text(tw().font_semibold(), format!("{:?}", state.page));
        b.text(
            tw().text(muted),
            "The page the menu's links lead to. The current one is marked in the menu.",
        );
        b.button(
            "indicator",
            if state.indicator {
                "Hide the indicator"
            } else {
                "Show the indicator"
            },
        )
        .on_click(|state: &mut State| state.indicator = !state.indicator);
    });
}

fn main() -> eframe::Result {
    common::run_app("navigation menu", State::default(), view)
}
