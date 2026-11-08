//! The middle pane: title bar, conversation and composer.

use bullseye::{Builder, Tw, none, tw};

use super::icons::{self, Glyph};
use super::state::{App, Block, Message};
use super::theme::{Role, Theme, Tok};
use super::view::Part;

type B<'b, 'f> = Builder<'b, 'f, App>;

/// ```html
/// <header class="flex items-center justify-between h-12 shrink-0 px-4 border-b border-[#383838]">
///   <h2 class="text-[15px]">Refactor the tokenizer</h2>
///   <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-[#383838] text-[#a8a8a8]">
///     <span>Assistant 1</span><svg class="size-3"/>
///   </div>
/// </header>
/// ```
fn top_bar(b: &mut B<'_, '_>, app: &App) {
    let k = &app.theme;
    let title = app
        .sessions
        .iter()
        .find(|session| session.key == app.selected)
        .map_or("", |session| session.title);
    b.div_id(
        Part::TopBar,
        tw().flex()
            .items_center()
            .justify_between()
            .h_px(48.0)
            .shrink_0()
            .px(4.0)
            .border_b(1.0)
            .border_color(k[Tok::Line]),
        none,
        |b| {
            b.text(tw().role(k[Role::Title].clone()).truncate(), title);
            b.div_id(
                Part::ModelChip,
                tw().flex()
                    .items_center()
                    .gap(1.5)
                    .px(2.5)
                    .py(1.0)
                    .rounded_lg()
                    .border(1.0)
                    .border_color(k[Tok::Line])
                    .text(k[Tok::InkMid])
                    .role(k[Role::Small].clone()),
                none,
                |b| {
                    b.text(tw(), "Assistant 1");
                    b.paint(
                        tw().size_px(12.0),
                        icons::glyph(Glyph::Chevron, k[Tok::InkMid]),
                    );
                },
            );
        },
    );
}

/// An assistant message: paragraphs, with code on its own row as a chip.
///
/// ```html
/// <div class="flex flex-col gap-2.5 text-[15px] leading-6">
///   <p>Yes. Most tokens are slices of the input...</p>
///   <code class="self-start px-2 py-0.5 rounded bg-[#2b2b2b] font-mono text-[13px]">...</code>
/// </div>
/// ```
fn assistant(b: &mut B<'_, '_>, k: &Theme, blocks: &[Block]) {
    b.div(
        tw().flex()
            .flex_col()
            .gap(2.5)
            .role(k[Role::Message].clone()),
        |b| {
            for block in blocks {
                match block {
                    Block::Paragraph(text) => b.text(tw(), text.clone()),
                    Block::Code(text) => b.div(
                        tw().self_start()
                            .px(2.0)
                            .py(0.5)
                            .rounded()
                            .bg(k[Tok::Surface])
                            .text(k[Tok::InkMid])
                            .role(k[Role::Code].clone()),
                        |b| b.text(tw(), text.clone()),
                    ),
                }
            }
        },
    );
}

/// ```html
/// <div class="self-end max-w-[480px] px-4 py-2.5 rounded-2xl bg-[#2b2b2b] text-[15px] leading-6">...</div>
/// ```
fn user(b: &mut B<'_, '_>, k: &Theme, text: &str) {
    b.div(
        tw().self_end()
            .max_w_px(480.0)
            .px(4.0)
            .py(2.5)
            .rounded_2xl()
            .bg(k[Tok::Surface])
            .role(k[Role::Message].clone()),
        |b| b.text(tw(), text),
    );
}

fn conversation(b: &mut B<'_, '_>, app: &App) {
    let k = &app.theme;
    b.div_id(
        Part::Conversation,
        tw().flex()
            .flex_col()
            .items_center()
            .grow()
            .min_h_0()
            .overflow_hidden(),
        none,
        |b| {
            b.div(
                tw().flex()
                    .flex_col()
                    .gap(6.0)
                    .w_full()
                    .max_w_px(720.0)
                    .p(4.0),
                |b| {
                    for message in &app.conversation {
                        match message {
                            Message::Assistant(blocks) => assistant(b, k, blocks),
                            Message::User(text) => user(b, k, text),
                        }
                    }
                },
            );
        },
    );
}

fn tool_button(k: &Theme, shape: Tw) -> Tw {
    shape
        .flex()
        .items_center()
        .justify_center()
        .size_px(32.0)
        .shrink_0()
        .hover(|p| p.bg(k[Tok::Hover]))
}

/// The composer is the last child of a column whose conversation grows, so it
/// sits at the bottom whatever the conversation holds.
///
/// ```html
/// <div class="flex justify-center shrink-0 px-4 pb-4">
///   <div class="flex flex-col gap-3 w-full max-w-[720px] p-3 rounded-2xl border border-[#383838] bg-[#2b2b2b]">
///     <span class="text-[#7c7c7c]">Reply…</span>
///     <div class="flex items-center justify-between">
///       <button class="size-8 rounded-lg hover:bg-[#2e2e2e]">+</button>
///       <button class="size-8 rounded-full bg-[#6f8fb3] hover:bg-[#84a2c4]">↑</button>
///     </div>
///   </div>
/// </div>
/// ```
fn composer(b: &mut B<'_, '_>, k: &Theme) {
    b.div(
        tw().flex().justify_center().shrink_0().px(4.0).pb(4.0),
        |b| {
            b.div_id(
                Part::Composer,
                tw().flex()
                    .flex_col()
                    .gap(3.0)
                    .w_full()
                    .max_w_px(720.0)
                    .p(3.0)
                    .rounded_2xl()
                    .border(1.0)
                    .border_color(k[Tok::Line])
                    .bg(k[Tok::Surface]),
                none,
                |b| {
                    b.text(tw().text(k[Tok::InkDim]).px(1.0), "Reply…");
                    b.div(tw().flex().items_center().justify_between(), |b| {
                        b.div_id(Part::Attach, tool_button(k, tw().rounded_lg()), none, |b| {
                            b.paint(
                                tw().size_px(18.0),
                                icons::glyph(Glyph::Plus, k[Tok::InkMid]),
                            );
                        });
                        b.div_id(
                            Part::Send,
                            tool_button(k, tw().rounded_full().bg(k[Tok::Accent]))
                                .hover(|p| p.bg(k[Tok::AccentHover])),
                            none,
                            |b| {
                                b.paint(
                                    tw().size_px(18.0),
                                    icons::glyph(Glyph::ArrowUp, k[Tok::OnAccent]),
                                );
                            },
                        );
                    });
                },
            );
        },
    );
}

/// ```html
/// <main class="flex flex-col grow min-w-0 overflow-hidden bg-[#1f1f1f]">
///   <header>...</header>
///   <div class="flex flex-col items-center grow min-h-0 overflow-hidden">...</div>
///   <div class="composer">...</div>
/// </main>
/// ```
pub fn centre(b: &mut B<'_, '_>, app: &App) {
    b.div_id(
        Part::Centre,
        tw().flex()
            .flex_col()
            .grow()
            .min_w_0()
            .overflow_hidden()
            .bg(app.theme[Tok::Ground]),
        none,
        |b| {
            top_bar(b, app);
            conversation(b, app);
            composer(b, &app.theme);
        },
    );
}
