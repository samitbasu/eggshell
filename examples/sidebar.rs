//! Port of the sidebar of hello_egui's fancy example: categories of
//! selectable entries, a wrapping set of pill buttons and a link pinned to the
//! bottom.
//!
//! After the sidebar in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT), rebuilt with bullseye. The selection is app state known at build
//! time, so it is base paint; hover is a variant, so the selected entry keeps
//! its fill under the pointer. The entries are placeholders for the original's
//! routes.

#[path = "common/mod.rs"]
mod common;

use bullseye::{Builder, NodeUi, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

pub const ENTRY_HOVER: Color32 = Color32::from_gray(228);
pub const ENTRY_SELECTED: Color32 = Color32::from_rgb(190, 211, 240);
pub const PILL: Color32 = Color32::from_gray(236);
pub const PILL_HOVER: Color32 = Color32::from_gray(222);
const RULE: Color32 = Color32::from_gray(210);
const MUTED: Color32 = Color32::from_gray(110);
const LINK: Color32 = Color32::from_rgb(0, 92, 190);
const INK: Color32 = Color32::from_gray(20);

pub const REPO: &str = "https://github.com/lucasmerlin/hello_egui";

pub struct Category {
    pub name: &'static str,
    pub examples: &'static [&'static str],
}

pub const EXAMPLES: &[Category] = &[
    Category {
        name: "Layout",
        examples: &["Flex", "Grid", "Wrapping"],
    },
    Category {
        name: "Lists",
        examples: &["Drag and drop", "Infinite scroll", "Virtual list"],
    },
    Category {
        name: "Forms",
        examples: &["Validation", "Suspense"],
    },
];

pub const CRATES: &[&str] = &[
    "flex",
    "dnd",
    "form",
    "inbox",
    "router",
    "suspense",
    "thumbhash",
    "virtual_list",
    "infinite_scroll",
    "pull_to_refresh",
];

/// What the main pane shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Example(&'static str),
    Crate(&'static str),
}

impl Route {
    fn title(self) -> String {
        match self {
            Self::Example(name) => format!("Example: {name}"),
            Self::Crate(name) => format!("Crate: egui_{name}"),
        }
    }
}

pub struct State {
    pub route: Route,
}

impl Default for State {
    fn default() -> Self {
        Self {
            route: Route::Example(EXAMPLES[0].examples[0]),
        }
    }
}

/// An entry that navigates to `route` when clicked.
///
/// ```html
/// <a class="px-1.5 py-1 rounded text-[14px] hover:bg-[#e4e4e4] transition duration-100">Flex</a>
/// <a class="px-1.5 py-1 rounded text-[14px] bg-[#bed3f0]">Grid</a>
/// ```
fn entry(b: &mut B<'_, '_, State>, state: &State, route: Route, label: &str) {
    let look = tw()
        .px(1.5)
        .py(1.0)
        .rounded()
        .role(common::sized(14.0))
        .truncate();
    let look = if state.route == route {
        look.bg(ENTRY_SELECTED)
    } else {
        look.hover(|p| p.bg(ENTRY_HOVER))
            .transition(core::time::Duration::from_millis(100))
    };
    b.span(
        route,
        look.sense_click(),
        label,
        common::on_click::<State>(move |state| state.route = route),
    );
}

/// A wrapping pill for a crate.
///
/// ```html
/// <a class="px-1.5 py-1 rounded-2xl bg-[#ececec] hover:bg-[#dedede]">dnd</a>
/// ```
fn pill(b: &mut B<'_, '_, State>, state: &State, name: &'static str) {
    let route = Route::Crate(name);
    let look = tw().px(1.5).py(1.0).rounded_2xl();
    let look = if state.route == route {
        look.bg(ENTRY_SELECTED)
    } else {
        look.bg(PILL).hover(|p| p.bg(PILL_HOVER))
    };
    b.span(
        route,
        look.sense_click(),
        name,
        common::on_click::<State>(move |state| state.route = route),
    );
}

/// ```html
/// <nav class="flex flex-col w-[240px] shrink-0 gap-1 p-2 border-r border-[#d2d2d2]">
///   <h2 class="text-lg">hello_egui</h2>
///   <span>Examples</span>
///   <small class="text-[#6e6e6e]">Layout</small> <a>Flex</a> <a>Grid</a> ...
///   <hr>
///   <span>Crates in hello_egui</span>
///   <div class="flex flex-wrap gap-2"><a class="pill">dnd</a> ...</div>
///   <a class="mt-auto self-center text-[#005cbe] underline hover:text-[#141414]">GitHub: hello_egui</a>
/// </nav>
/// <main class="grow p-4"><h1 class="text-2xl">Example: Flex</h1></main>
/// ```
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    b.div(tw().flex().size_full(), |b| {
        b.div(
            tw().flex()
                .flex_col()
                .w_px(240.0)
                .shrink_0()
                .gap(1.0)
                .p(2.0)
                .border_r(1.0)
                .border_color(RULE),
            |b| {
                b.text(tw().role(common::sized(20.0)).py(1.0), "hello_egui");
                b.text(tw(), "Examples");
                for category in EXAMPLES {
                    b.text(
                        tw().role(common::sized(11.0)).text(MUTED).pt(1.0),
                        category.name,
                    );
                    for &name in category.examples {
                        entry(b, state, Route::Example(name), name);
                    }
                }
                b.rule(tw().h_px(1.0).bg(RULE).my(2.0));
                b.text(tw(), "Crates in hello_egui");
                b.div(tw().flex().flex_wrap().gap(2.0).pt(1.0), |b| {
                    for &name in CRATES {
                        pill(b, state, name);
                    }
                });
                b.span(
                    "github",
                    tw().mt_auto()
                        .self_center()
                        .text(LINK)
                        .underline()
                        .sense_click()
                        .hover(|p| p.text(INK)),
                    "GitHub: hello_egui",
                    |_, ui| {
                        if ui.node_response().clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(REPO));
                        }
                    },
                );
            },
        );
        b.div(tw().grow().p(4.0), |b| {
            b.text(tw().role(common::sized(24.0)), state.route.title());
        });
    });
}

fn main() -> eframe::Result {
    common::run_app("sidebar", State::default(), layout)
}
