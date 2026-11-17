//! A documentation page with example apps mounted in its Markdown.
//!
//! Each `eggshell-example NAME` fence mounts a small app over a field of the
//! page's state with `b.mount`. The two counters share one field. The bar at
//! the top switches the frame drawn around every mount.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::owned::Mark;
use eggshell_ui::prim::text_field::Field;
use eggshell_ui::{Builder, Chord, Command, Commands, Scope, tw};
use eggshell_ui_extras::markdown::{Directive, Directives, MarkdownExt};
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

/// How a selected example is marked: the candidates, shown on the menu
/// (captioned) and the scroll list (uncaptioned) as a mock-up until
/// selection is built.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Selected {
    #[default]
    Ring,
    Chip,
    Tint,
}

impl Selected {
    const ALL: [(Self, &str); 3] = [
        (Self::Ring, "Ring"),
        (Self::Chip, "Chip"),
        (Self::Tint, "Tint"),
    ];
}

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
    pub selected: Selected,
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
    Selected,
    SelectedLabel,
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

/// Whether the example NAME is captioned, and whether it is shown selected
/// in the mock-up.
fn shown(name: &str) -> (bool, bool) {
    match name {
        "counter" => (true, false),
        "menu" => (true, true),
        "form" => (false, false),
        _ => (false, true),
    }
}

fn example<'f>(s: &mut Scope<'_, 'f, State>, state: &'f State, directive: &Directive<'_>) {
    let name = directive.arg;
    let (captioned, selected) = shown(name);
    if selected {
        mock_selected(s, state, name, captioned);
        return;
    }
    let caption = captioned.then(|| format!("Example: {name}"));
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

/// The example NAME as it would look selected, in the candidate look the
/// page's switch picks: the caption and frame a captioned mount draws, from
/// the same style, around the mount uncaptioned.
fn mock_selected<'f>(s: &mut Scope<'_, 'f, State>, state: &'f State, name: &str, captioned: bool) {
    let style = eggshell_ui::Style::of(s.ctx());
    let look = &style.mount;
    let tokens = common::tokens(s.ctx());
    let primary = tokens.primary;
    let framed = tw()
        .border(look.border_width)
        .border_color(look.border)
        .rounded_px(f32::from(look.radius))
        .p(look.padding / 4.0);
    let frame = match (captioned, state.selected) {
        (true, Selected::Ring) => framed.border_color(primary).ring(1.0, primary),
        (true, Selected::Chip) => framed.border_color(primary),
        (true, Selected::Tint) => framed
            .border_color(primary)
            .bg(primary.gamma_multiply(0.06)),
        (false, selected) => {
            // Grown past the view by as much as it is padded, so the ring
            // clears the view without moving it.
            let around = tw().m(-2.0).p(2.0).rounded_px(f32::from(look.radius));
            match selected {
                Selected::Ring => around.ring(2.0, primary),
                Selected::Chip => around.ring(1.0, primary),
                Selected::Tint => around.ring(1.0, primary).bg(primary.gamma_multiply(0.06)),
            }
        }
    };
    let caption = match state.selected {
        Selected::Chip => tw()
            .self_start()
            .px(2.0)
            .rounded_md()
            .bg(primary)
            .text(tokens.primary_foreground),
        _ => tw().text(look.caption),
    };
    s.div_id(
        Part::Mock,
        tw().flex().flex_col().gap(look.gap / 4.0),
        |s| {
            if captioned {
                s.span(Part::Caption, caption, format!("Example: {name}"));
            }
            s.div_id(Part::Frame, frame, |s| mount(s, state, name, None));
        },
    );
}

/// ```html
/// <div class="flex flex-col size-full">
///   <header class="flex gap-3 p-3 border-b">Frame: [None|Outline|…] · log</header>
///   <main class="overflow-y-auto grow p-6"><article class="max-w-[640px]">…</article></main>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
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
                b.span(Part::SelectedLabel, tw(), "Selected");
                b.segmented(Part::Selected, state.selected, Selected::ALL)
                    .on_change(|state: &mut State, selected| state.selected = selected);
                let last = state
                    .page_log
                    .last()
                    .copied()
                    .unwrap_or("no page command yet");
                b.span(Part::Log, tw().text(tokens.muted_foreground), last);
            },
        );
        b.scroll(Part::Page, tw().grow().p(6.0)).children(|s| {
            s.div(tw().flex().flex_col().gap(6.0).max_w_px(640.0), |b| {
                b.markdown(Part::Doc, &source)
                    .directives(&directives, state)
                    .add();
            });
        });
    });
}

#[allow(dead_code, reason = "the tests mount the page without running it")]
fn main() -> eframe::Result {
    common::run_app("docs_embed", State::default(), layout)
}
