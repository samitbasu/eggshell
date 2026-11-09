//! The middle pane: title bar, conversation and composer.

use bullseye::owned::Select;
use bullseye::owned::{Cell, Segmented};
use bullseye::prim::text_field::Field;
use bullseye::{Builder, NodeUi, Opacity, Scope, Tw, none, tw};
use egui::{Key, WidgetInfo};

use super::icons::{self, Glyph};
use super::state::{App, Block, MODELS, Message, Mode};
use super::theme::{Role, Theme, Tok};
use super::view::Part;

type B<'b, 'f> = Builder<'b, 'f, App>;
type S<'b, 'f> = Scope<'b, 'f, App>;

/// The model select, in the shape of a chip.
///
/// ```html
/// <header class="flex items-center justify-between h-12 shrink-0 px-4 border-b border-border">
///   <h2 class="text-[15px]">Refactor the tokenizer</h2>
///   <select class="rounded-lg border border-border bg-muted px-2.5 py-1 text-[13px] text-muted-foreground">
///     <option>Assistant 1</option> <!-- Assistant 2, Assistant 3 -->
///   </select>
/// </header>
/// ```
fn top_bar(b: &mut B<'_, '_>, app: &App) {
    let k = &Theme::of(b.ctx());
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
            let models: Vec<(usize, &str)> = MODELS.iter().copied().enumerate().collect();
            b.select(
                Part::Model,
                Select::new(app.model, &models).named("Model"),
                |app: &mut App, _, model| app.model = model,
            );
        },
    );
}

/// An assistant message: paragraphs, with code on its own row as a chip.
///
/// ```html
/// <div class="flex flex-col gap-2.5 text-[15px] leading-6">
///   <p>Yes. Most tokens are slices of the input...</p>
///   <code class="self-start px-2 py-0.5 rounded bg-muted font-mono text-[13px]">...</code>
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
/// <div class="self-end max-w-[480px] px-4 py-2.5 rounded-2xl bg-muted text-[15px] leading-6">...</div>
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
    let k = &Theme::of(b.ctx());
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
        .hover(|p| p.bg(k[Tok::Line]))
}

/// The reply field is as tall as its text, from `COMPOSER_MIN_ROWS` rows to
/// `COMPOSER_MAX_ROWS`, and scrolls beyond that. Ctrl+Enter (Cmd+Enter on
/// macOS) sends.
const COMPOSER_MIN_ROWS: usize = 3;
const COMPOSER_MAX_ROWS: usize = 8;

fn field(b: &mut B<'_, '_>, app: &App) {
    let field = Field::multiline(&app.draft, COMPOSER_MIN_ROWS)
        .max_rows(COMPOSER_MAX_ROWS)
        .hint("Reply…")
        .bare();
    b.text_field(
        Part::Field,
        tw().w_full().px(1.0),
        field,
        |app: &mut App| &mut app.draft,
        |app, ui, response| {
            response.widget_info(|| {
                WidgetInfo::labeled(egui::Role::MultilineTextInput, ui.is_enabled(), "Message")
            });
            let chord = ui.input(|input| input.modifiers.command && input.key_pressed(Key::Enter));
            if chord && response.has_focus() {
                app.send();
            }
        },
    );
}

/// The mode, as a segmented control of small cells.
///
/// ```html
/// <div class="flex items-center gap-0.5 text-[13px]">
///   <span class="px-2.5 py-1 rounded-lg text-muted-foreground hover:bg-accent focus-visible:ring-2">Quick</span>
///   <span class="px-2.5 py-1 rounded-lg bg-border text-foreground">Standard</span> <!-- pressed -->
///   <span class="px-2.5 py-1 rounded-lg text-muted-foreground hover:bg-accent">Thorough</span>
/// </div>
/// ```
fn mode(b: &mut B<'_, '_>, app: &App) {
    let k = &Theme::of(b.ctx());
    b.segmented_with(
        Part::Mode,
        Segmented::new(app.mode, &Mode::ALL),
        tw().flex()
            .items_center()
            .gap(0.5)
            .role(k[Role::Small].clone()),
        |cell| {
            let look = tw()
                .px(2.5)
                .py(1.0)
                .rounded_lg()
                .focus(|p| p.ring(2.0, k[Tok::Accent]));
            match cell {
                Cell::Pressed => look.bg(k[Tok::Line]).text(k[Tok::Ink]),
                Cell::Released => look.text(k[Tok::InkMid]).hover(|p| p.bg(k[Tok::Hover])),
            }
        },
        |app, _, mode| app.mode = mode,
    );
}

/// The composer is the last child of a column whose conversation grows, so it
/// sits at the bottom whatever the conversation holds.
///
/// ```html
/// <div class="flex justify-center shrink-0 px-4 pb-4">
///   <div class="flex flex-col gap-3 w-full max-w-[720px] p-3 rounded-2xl border border-border bg-muted">
///     <textarea class="w-full px-1 resize-none bg-transparent outline-none" rows="3" placeholder="Reply…"></textarea> <!-- grows to 8 rows -->
///     <div class="flex items-center justify-between">
///       <div class="flex items-center gap-1.5">
///         <button class="size-8 rounded-lg hover:bg-accent">+</button>
///         <div class="segmented">Quick Standard Thorough</div>
///       </div>
///       <button class="size-8 rounded-full bg-primary hover:bg-primary/90 disabled:opacity-50">↑</button>
///     </div>
///   </div>
/// </div>
/// ```
fn composer(b: &mut B<'_, '_>, app: &App) {
    let k = &Theme::of(b.ctx());
    let send_button = |shape: Tw| {
        if app.draft.trim().is_empty() {
            shape.inert().opacity(Opacity::new(0.4))
        } else {
            shape
        }
    };
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
                    field(b, app);
                    b.div(tw().flex().items_center().justify_between(), |b| {
                        b.div(tw().flex().items_center().gap(1.5), |b| {
                            b.div_id(Part::Attach, tool_button(k, tw().rounded_lg()), none, |b| {
                                b.paint(
                                    tw().size_px(18.0),
                                    icons::glyph(Glyph::Plus, k[Tok::InkMid]),
                                );
                            });
                            mode(b, app);
                        });
                        b.div_id(
                            Part::Send,
                            send_button(
                                tool_button(k, tw().rounded_full().bg(k[Tok::Accent]))
                                    .hover(|p| p.bg(k[Tok::AccentHover]))
                                    .sense_click(),
                            ),
                            |app: &mut App, ui| {
                                if ui.node_response().clicked() {
                                    app.send();
                                }
                            },
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
/// <main class="flex flex-col grow min-w-0 overflow-hidden bg-background">
///   <header>...</header>
///   <div class="flex flex-col items-center grow min-h-0 overflow-hidden">...</div>
///   <div class="composer">...</div>
/// </main>
/// ```
pub fn centre(s: &mut S<'_, '_>, app: &App) {
    top_bar(s, app);
    conversation(s, app);
    composer(s, app);
}

/// The centre pane's box, which its caller keys.
pub fn centre_style(k: &Theme) -> Tw {
    tw().flex()
        .flex_col()
        .grow()
        .min_w_0()
        .overflow_hidden()
        .bg(k[Tok::Ground])
}
