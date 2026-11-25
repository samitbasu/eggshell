//! A documentation page with example apps mounted in its Markdown.
//!
//! Each `eggshell-example NAME` fence mounts a small app over a field of the
//! page's state with `b.mount`. The two counters share one field. The bar at
//! the top switches the theme colour, which colours the selected example's
//! ring.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::owned::Mark;
use eggshell_ui::owned::markdown::{Directive, Directives, Ext as _, Typeset};
use eggshell_ui::owned::text_field::Field;
use eggshell_ui::theme::ThemeColor;
use eggshell_ui::{Builder, Chord, Command, Commands, Scope, tw};
use egui::{Key, Modifiers};

const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud \
exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";

fn source() -> String {
    let para = |n: usize| vec![LOREM; n].join(" ");
    format!(
        "# Lorem ipsum\n\n{p1}\n\n\
         ## Dolor sit amet\n\n{p1}\n\n\
         ```eggshell-example counter\n```\n\n\
         {p1}\n\n\
         ```eggshell-example counter\n```\n\n\
         ## Consectetur\n\n{p2}\n\n\
         ```eggshell-example menu\n```\n\n\
         ## Adipiscing elit\n\n{p1}\n\n\
         ```eggshell-example form\n```\n\n\
         ## Sed do eiusmod\n\n{p2}\n\n\
         ```eggshell-example scroll\n```\n\n\
         ## Tempor incididunt\n\n{p3}\n\n{p3}\n",
        p1 = para(1),
        p2 = para(2),
        p3 = para(3),
    )
}

/// The theme colours the page can be shown in: the selected example's ring
/// is the theme's primary colour.
const THEMES: [(ThemeColor, &str); 3] = [
    (ThemeColor::Base, "Neutral"),
    (ThemeColor::Blue, "Blue"),
    (ThemeColor::Violet, "Violet"),
];

#[derive(Default)]
pub struct Counter {
    pub count: u32,
}

#[derive(Default)]
pub struct Menu {
    pub grid: bool,
    pub chosen: Option<&'static str>,
}

#[derive(Default)]
pub struct Form {
    pub name: String,
    pub saved: Vec<String>,
}

pub struct List {
    pub rows: Vec<u32>,
    pub picked: Option<u32>,
}

impl Default for List {
    fn default() -> Self {
        Self {
            rows: (1..=40).collect(),
            picked: None,
        }
    }
}

#[derive(Default)]
pub struct State {
    pub theme: ThemeColor,
    pub clicks: Counter,
    pub menu: Menu,
    pub form: Form,
    pub list: List,
    pub page_log: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Bar,
    Theme,
    ThemeLabel,
    Log,
    Doc,
    App,
    Caption,
    Mock,
    Frame,
    Add,
    Count,
    Actions,
    Grid,
    Hint,
    Choose(&'static str),
    Chosen,
    Name,
    Saved,
    Rows,
    Row(u32),
    Picked,
}

const SAVE: Chord = const { Chord::new(Modifiers::COMMAND, Key::S).expect("not reserved") };
const FIND: Chord = const { Chord::new(Modifiers::COMMAND, Key::K).expect("not reserved") };

fn page_commands() -> Commands<State> {
    Commands::new()
        .add(
            "save",
            Command::new("Save page", |state: &mut State| {
                state.page_log.push("page: save")
            })
            .shortcut(SAVE),
        )
        .add(
            "find",
            Command::new("Find", |state: &mut State| {
                state.page_log.push("page: find")
            })
            .shortcut(FIND),
        )
}

fn form_commands() -> Commands<Form> {
    Commands::new().add(
        "save",
        Command::new("Save form", |form: &mut Form| {
            let name = form.name.clone();
            form.saved.push(name);
        })
        .shortcut(SAVE),
    )
}

pub fn counter<'f>(b: &mut Builder<'_, 'f, Counter>, counter: &'f Counter) {
    b.div(tw().flex().flex_row().items_center().gap(3.0), |b| {
        b.button(Part::Add, "Add one")
            .on_click(|counter: &mut Counter| counter.count += 1);
        b.span(Part::Count, tw(), format!("Counted {}", counter.count));
    });
}

pub fn menu<'f>(b: &mut Builder<'_, 'f, Menu>, menu: &'f Menu) {
    b.div(tw().flex().flex_row().items_center().gap(3.0), |b| {
        b.dropdown_menu(Part::Actions, "Actions").items(|m| {
            m.checkbox_item(Part::Grid, "Show grid", Mark::from(menu.grid))
                .on_change(|menu: &mut Menu, mark| menu.grid = mark == Mark::On);
            m.separator();
            for name in ["Copy", "Paste", "Delete"] {
                m.item(Part::Choose(name), name)
                    .on_click(move |menu: &mut Menu| menu.chosen = Some(name));
            }
        });
        b.button(Part::Hint, "Hover me")
            .tooltip("Lorem ipsum dolor sit amet")
            .on_click(|_: &mut Menu| {});
        let chosen = menu.chosen.unwrap_or("nothing");
        let grid = if menu.grid { "on" } else { "off" };
        b.span(Part::Chosen, tw(), format!("Chose {chosen}; grid {grid}"));
    });
}

pub fn form<'f>(b: &mut Builder<'_, 'f, Form>, form: &'f Form) {
    b.commands(&form_commands(), form);
    b.div(tw().flex().flex_col().gap(2.0), |b| {
        b.text_field_with(Part::Name, Field::singleline(&form.name).named("Name"))
            .classes(tw().w_px(240.0))
            .on_change(|form: &mut Form, name| form.name = name);
        let saved = form.saved.last().map_or("nothing yet", String::as_str);
        b.span(
            Part::Saved,
            tw(),
            format!("Saved {} times; last {saved:?}", form.saved.len()),
        );
    });
}

pub fn list<'f>(b: &mut Builder<'_, 'f, List>, list: &'f List) {
    b.div(tw().flex().flex_col().gap(2.0), |b| {
        b.scroll(Part::Rows, tw().h_px(160.0).w_px(240.0))
            .children(|s| {
                s.div(tw().flex().flex_col().gap(1.0), |b| {
                    for &row in &list.rows {
                        b.button(Part::Row(row), format!("Row {row}"))
                            .on_click(move |list: &mut List| list.picked = Some(row));
                    }
                });
            });
        let picked = list
            .picked
            .map_or_else(|| "none".to_owned(), |row| row.to_string());
        b.span(Part::Picked, tw(), format!("Picked {picked}"));
    });
}

/// The directives of the page: `eggshell-example NAME` mounts the app NAME
/// over its field of the page's state.
pub fn directives() -> Directives<State> {
    Directives::new().with("eggshell-example", example)
}

/// Whether the example NAME is shown under a caption.
fn captioned(name: &str) -> bool {
    matches!(name, "counter" | "menu")
}

fn example<'f>(s: &mut Scope<'_, 'f, State>, state: &'f State, directive: &Directive<'_>) {
    let name = directive.arg;
    let caption = captioned(name).then(|| format!("Example: {name}"));
    mount(s, state, name, caption);
}

/// Mounts the example NAME, captioned by `caption` if there is one.
fn mount<'f>(s: &mut Scope<'_, 'f, State>, state: &'f State, name: &str, caption: Option<String>) {
    fn add<'f, A: 'static, S: 'static, L, V>(
        chain: eggshell_ui::MountChain<'_, 'f, A, S, L, V>,
        caption: Option<String>,
    ) where
        L: Fn(&mut A) -> &mut S + 'static,
        V: FnOnce(&mut Builder<'_, 'f, S>, &'f S),
    {
        match caption {
            Some(caption) => chain.with_caption(caption).add(),
            None => chain.add(),
        };
    }
    match name {
        "counter" => add(
            s.mount(
                Part::App,
                tw(),
                &state.clicks,
                |s: &mut State| &mut s.clicks,
                counter,
            ),
            caption,
        ),
        "menu" => add(
            s.mount(
                Part::App,
                tw(),
                &state.menu,
                |s: &mut State| &mut s.menu,
                menu,
            ),
            caption,
        ),
        "form" => add(
            s.mount(
                Part::App,
                tw(),
                &state.form,
                |s: &mut State| &mut s.form,
                form,
            ),
            caption,
        ),
        "scroll" => add(
            s.mount(
                Part::App,
                tw(),
                &state.list,
                |s: &mut State| &mut s.list,
                list,
            ),
            caption,
        ),
        name => {
            let muted = common::tokens(s.ctx()).muted_foreground;
            s.span(
                Part::App,
                tw().text(muted),
                format!("No example named {name:?}"),
            );
        }
    }
}

/// ```html
/// <div class="flex flex-col size-full">
///   <header class="flex gap-3 p-3 border-b">Theme: [Neutral|Blue|Violet] · log</header>
///   <main class="overflow-y-auto grow p-6"><article class="max-w-[37em]">…</article></main>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    common::use_theme_color(b.ctx(), state.theme);
    b.commands(&page_commands(), state);
    let tokens = common::tokens(b.ctx());
    let directives = directives();
    let source = source();
    b.div(tw().flex().flex_col().size_full(), |b| {
        b.div_id(
            Part::Bar,
            tw().flex()
                .flex_row()
                .items_center()
                .gap(3.0)
                .p(3.0)
                .border_b(1.0)
                .border_color(tokens.border),
            |b| {
                b.span(Part::ThemeLabel, tw(), "Theme");
                b.segmented(Part::Theme, state.theme, THEMES)
                    .on_change(|state: &mut State, theme| state.theme = theme);
                let last = state
                    .page_log
                    .last()
                    .copied()
                    .unwrap_or("no page command yet");
                b.span(Part::Log, tw().text(tokens.muted_foreground), last);
            },
        );
        b.scroll(Part::Page, tw().grow().p(6.0)).children(|s| {
            s.div(
                tw().flex()
                    .flex_col()
                    .gap(6.0)
                    .max_w_px(Typeset::default().measure_width()),
                |b| {
                    b.markdown(Part::Doc, &source)
                        .directives(&directives, state)
                        .add();
                },
            );
        });
    });
}

#[allow(dead_code, reason = "the tests mount the page without running it")]
fn main() -> eframe::Result {
    common::run_app("docs_embed", State::default(), layout)
}
