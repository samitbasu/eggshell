//! Exclusive toggles: a tool cluster of icon cells with a rule and a caret
//! button among them, and a 2×2 picker of cards. Both are one `ToggleGroup`
//! whose members sit wherever the layout puts them.

#[path = "common/mod.rs"]
mod common;

use bullseye::contract::Size;
use bullseye::owned::{ButtonOptions, Face, ToggleGroup};
use bullseye::{Builder, Scope, tw};
use bullseye_shadcn::{size, variant};
use common::glyphs::{self, Glyph};

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Tools,
    Lettering,
    PortOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Pan,
    Zoom,
    AddPort,
    Delete,
}

const TOOLS: [(Tool, Glyph, &str); 5] = [
    (Tool::Select, Glyph::Sun, "Select"),
    (Tool::Pan, Glyph::Moon, "Pan"),
    (Tool::Zoom, Glyph::Monitor, "Zoom"),
    (Tool::AddPort, Glyph::Plus, "Add port"),
    (Tool::Delete, Glyph::Cross, "Delete"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lettering {
    Sans,
    Mono,
    Display,
    Compact,
}

impl Lettering {
    const ALL: [Self; 4] = [Self::Sans, Self::Mono, Self::Display, Self::Compact];

    fn label(self) -> &'static str {
        match self {
            Self::Sans => "Sans",
            Self::Mono => "Mono",
            Self::Display => "Display",
            Self::Compact => "Compact",
        }
    }

    fn sample(self) -> bullseye::Tw {
        tw().role(match self {
            Self::Sans => common::sized(24.0),
            Self::Mono => common::mono(24.0),
            Self::Display => common::sized(32.0),
            Self::Compact => common::sized(16.0),
        })
    }
}

pub struct State {
    pub tool: Tool,
    pub lettering: Lettering,
    pub port_options_opened: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tool: Tool::Select,
            lettering: Lettering::Sans,
            port_options_opened: 0,
        }
    }
}

/// ```html
/// <div class="flex items-center gap-1 p-1">
///   <button>…</button> <button>…</button>
///   <div class="h-5 w-px bg-border"/>
///   <button>…</button>
///   <div class="flex"><button>…</button><button>▾</button></div>
/// </div>
/// ```
fn tool_cluster(b: &mut Builder<'_, '_, State>, state: &State) {
    let border = common::tokens(b.ctx()).border;
    let tools = ToggleGroup::new(Part::Tools, state.tool)
        .with_options(ButtonOptions::new().size(size::ICON))
        .on_change(|state: &mut State, tool| state.tool = tool);
    let icon = |b: &Builder<'_, '_, State>, tool: Tool| {
        let (_, glyph, name) = TOOLS.iter().find(|(t, ..)| *t == tool).expect("a tool");
        (glyphs::icon(b.ctx(), *glyph), *name)
    };
    b.div(tw().flex().items_center().gap(1.0), |b| {
        for tool in [Tool::Select, Tool::Pan, Tool::Zoom] {
            let (face, name) = icon(b, tool);
            b.toggle_item(&tools, tool, face)
                .named(name)
                .tooltip(name)
                .add();
        }
        b.div(tw().w_px(1.0).h_px(20.0).bg(border), |_| {});
        b.div(tw().flex(), |b| {
            let (face, name) = icon(b, Tool::AddPort);
            b.toggle_item(&tools, Tool::AddPort, face).named(name).add();
            let caret = glyphs::icon(b.ctx(), Glyph::Caret);
            b.button(Part::PortOptions, caret)
                .named("Port options")
                .variant(variant::GHOST)
                .size(size::ICON)
                .on_click(|state: &mut State| state.port_options_opened += 1);
        });
        let (face, name) = icon(b, Tool::Delete);
        b.toggle_item(&tools, Tool::Delete, face).named(name).add();
    });
}

/// ```html
/// <div class="flex flex-wrap gap-2 w-[296px]">
///   <button class="relative flex flex-col items-start gap-2 p-3 w-36">
///     <span class="text-2xl">Aa</span><span>Sans</span><svg class="absolute top-2 right-2"/>
///   </button> …
/// </div>
/// ```
fn lettering_cards(b: &mut Builder<'_, '_, State>, state: &State) {
    let check = glyphs::icon(b.ctx(), Glyph::Check);
    let lettering = ToggleGroup::new(Part::Lettering, state.lettering)
        .with_options(
            ButtonOptions::new().classes(
                tw().flex_col()
                    .items_start()
                    .h_px(80.0)
                    .w_px(144.0)
                    .p(3.0)
                    .gap(2.0),
            ),
        )
        .on_change(|state: &mut State, lettering| state.lettering = lettering);
    b.div(
        tw().flex().flex_wrap().gap(2.0).w_px(296.0).shrink_0(),
        |b| {
            for option in Lettering::ALL {
                let chosen = state.lettering == option;
                let check = check.clone();
                b.toggle_item(&lettering, option, Face::Empty)
                    .named(option.label())
                    .size(Size::DEFAULT)
                    .children(|s: &mut Scope<'_, '_, State>| {
                        s.text(option.sample(), "Aa");
                        s.text(tw(), option.label());
                        if chosen {
                            s.glyph(
                                tw().size_px(16.0).absolute().top_px(8.0).right_px(8.0),
                                check,
                            );
                        }
                    })
                    .add();
            }
        },
    );
}

pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(3.0).p(4.0), |b| {
        b.text(tw().text(muted), "Tools");
        tool_cluster(b, state);
        b.text(tw().text(muted), "Lettering");
        lettering_cards(b, state);
    });
}

fn main() -> eframe::Result {
    common::run_app("toggle_group", State::default(), layout)
}
