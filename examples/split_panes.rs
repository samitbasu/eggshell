//! TODO(prose): a sidebar, an editor over a terminal and an inspector, each
//! resized by a splitter.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::owned::PaneSize;
use eggshell_ui::{Builder, tw};

const FILES: [&str; 6] = [
    "main.rs",
    "layout.rs",
    "splitter.rs",
    "style.rs",
    "tests.rs",
    "Cargo.toml",
];

const CODE: [&str; 6] = [
    "fn main() {",
    "    let panes = Panes::default();",
    "    run(panes);",
    "}",
    "",
    "// drag the handles, or Tab to one and use the arrows",
];

/// The panes' sizes, kept by the app.
pub struct Panes {
    pub sidebar: PaneSize,
    pub terminal: PaneSize,
    pub inspector: PaneSize,
}

impl Default for Panes {
    fn default() -> Self {
        Self {
            sidebar: PaneSize::new(220.0),
            terminal: PaneSize::new(160.0),
            inspector: PaneSize::new(240.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Frame,
    Sidebar,
    Left,
    Editor,
    Code,
    Bottom,
    Terminal,
    Right,
    Inspector,
}

fn size(size: PaneSize) -> String {
    format!("{:.0} pt", f32::from(size))
}

/// ```html
/// <div class="flex size-full bg-background text-foreground">
///   <aside style="width: 220px"/> <splitter/>
///   <div class="flex flex-col flex-1 min-w-0"> <main class="flex-1"/> <splitter/> <div style="height: 160px"/> </div>
///   <splitter/> <aside style="width: 240px"/>
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, Panes>, panes: &Panes) {
    let t = common::tokens(b.ctx());
    let heading = || tw().font_medium().text(t.foreground);
    let muted = || tw().text(t.muted_foreground);
    b.div_id(
        Part::Frame,
        tw().flex().size_full().bg(t.background).text(t.foreground),
        |b| {
            b.div_id(
                Part::Sidebar,
                tw().flex()
                    .flex_col()
                    .gap(1.0)
                    .p(3.0)
                    .w_px(panes.sidebar.into())
                    .min_w_0()
                    .overflow_hidden()
                    .bg(t.muted),
                |b| {
                    b.text(heading(), "Files");
                    b.text(muted(), size(panes.sidebar));
                    for file in FILES {
                        b.text(tw(), file);
                    }
                },
            );
            b.splitter(Part::Left)
                .before(panes.sidebar)
                .range(160.0..=400.0)
                .reset_to(220.0)
                .named("Sidebar")
                .on_resize(|panes: &mut Panes, size| panes.sidebar = size);
            b.div_id(
                Part::Editor,
                tw().flex().flex_col().flex_1().min_w_0(),
                |b| {
                    b.div_id(
                        Part::Code,
                        tw().flex().flex_col().flex_1().min_h_0().p(3.0),
                        |b| {
                            for line in CODE {
                                b.text(tw(), line);
                            }
                        },
                    );
                    b.splitter(Part::Bottom)
                        .after(panes.terminal)
                        .range(80.0..=360.0)
                        .reset_to(160.0)
                        .named("Terminal")
                        .on_resize(|panes: &mut Panes, size| panes.terminal = size);
                    b.div_id(
                        Part::Terminal,
                        tw().flex()
                            .flex_col()
                            .gap(1.0)
                            .p(3.0)
                            .h_px(panes.terminal.into())
                            .min_h_0()
                            .overflow_hidden()
                            .bg(t.muted),
                        |b| {
                            b.text(heading(), "Terminal");
                            b.text(muted(), size(panes.terminal));
                            b.text(tw(), "$ cargo test");
                        },
                    );
                },
            );
            b.splitter(Part::Right)
                .after(panes.inspector)
                .range(180.0..)
                .reset_to(240.0)
                .named("Inspector")
                .on_resize(|panes: &mut Panes, size| panes.inspector = size);
            b.div_id(
                Part::Inspector,
                tw().flex()
                    .flex_col()
                    .gap(1.0)
                    .p(3.0)
                    .w_px(panes.inspector.into())
                    .min_w_0()
                    .overflow_hidden()
                    .bg(t.muted),
                |b| {
                    b.text(heading(), "Inspector");
                    b.text(muted(), size(panes.inspector));
                },
            );
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("split_panes", Panes::default(), layout)
}
