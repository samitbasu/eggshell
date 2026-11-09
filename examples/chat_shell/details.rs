//! The right pane: environment, context, changes and background tasks.

use bullseye::{Builder, Scope, Tw, none, tw};
use egui::Color32;

use super::state::{App, MODELS, Pane};
use super::theme::{Role, Theme, Tok};
use super::view::Part;

type B<'b, 'f> = Builder<'b, 'f, App>;
type S<'b, 'f> = Scope<'b, 'f, App>;

const CHANGES: [(&str, &str, &str); 3] = [
    ("src/lexer.rs", "+24", "-8"),
    ("src/token.rs", "+6", "-2"),
    ("tests/parser.rs", "+31", "-12"),
];

#[derive(Clone, Copy)]
enum TaskState {
    Running,
    Queued,
    Done,
}

const TASKS: [(&str, TaskState); 3] = [
    ("Run the test suite", TaskState::Running),
    ("Build the docs", TaskState::Queued),
    ("Format the workspace", TaskState::Done),
];

impl TaskState {
    fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Queued => "Queued",
            Self::Done => "Done",
        }
    }

    fn color(self, k: &Theme) -> Color32 {
        match self {
            Self::Running => k[Tok::Accent],
            Self::Queued | Self::Done => k[Tok::InkDim],
        }
    }
}

/// A section: a small header over a grouped form.
///
/// ```html
/// <section class="flex flex-col gap-1.5">
///   <h3 class="text-[11px] uppercase text-muted-foreground">Environment</h3>
///   <div class="flex flex-col rounded-lg border bg-background divide-y">...rows</div>
/// </section>
/// ```
fn section(b: &mut B<'_, '_>, k: &Theme, title: &'static str, rows: impl FnOnce(&mut S<'_, '_>)) {
    b.div_id(title, tw().flex().flex_col().gap(1.5), none, |b| {
        b.text(
            tw().role(k[Role::Meta].clone())
                .text(k[Tok::InkDim])
                .uppercase()
                .pb(0.5),
            title,
        );
        b.div_id(
            Part::Group,
            tw().flex()
                .flex_col()
                .border(1.0)
                .border_color(k[Tok::Line])
                .rounded_lg()
                .bg(k[Tok::Ground])
                .divide_y(1.0, k[Tok::Line])
                .overflow_hidden(),
            none,
            rows,
        );
    });
}

/// ```html
/// <div class="flex items-center justify-between gap-2 px-3 py-2 text-[13px]">
///   <span class="text-muted-foreground">Branch</span><span class="truncate">main</span>
/// </div>
/// ```
fn row(b: &mut B<'_, '_>, k: &Theme, label: &'static str, value: impl FnOnce(&mut B<'_, '_>)) {
    b.div_id(
        Part::Row(label),
        tw().flex()
            .items_center()
            .justify_between()
            .gap(2.0)
            .px(3.0)
            .py(2.0)
            .role(k[Role::Small].clone()),
        none,
        |b| {
            b.text(tw().text(k[Tok::InkMid]), label);
            value(b);
        },
    );
}

/// ```html
/// <div class="flex flex-col gap-2 px-3 py-2 text-[13px]">
///   <div class="flex justify-between"><span>Context window</span><span>42%</span></div>
///   <div class="w-full h-1 rounded-full overflow-hidden bg-border">
///     <div class="h-full rounded-full bg-primary" style="width: 42%"></div>
///   </div>
/// </div>
/// ```
fn context(b: &mut B<'_, '_>, app: &App) {
    let k = &Theme::of(b.ctx());
    let used = f32::from(app.context_used);
    b.div_id(
        Part::Row("Context window"),
        tw().flex()
            .flex_col()
            .gap(2.0)
            .px(3.0)
            .py(2.0)
            .role(k[Role::Small].clone()),
        none,
        |b| {
            b.div(tw().flex().items_center().justify_between(), |b| {
                b.text(tw().text(k[Tok::InkMid]), "Context window");
                b.text(tw(), format!("{:.0}%", used * 100.0));
            });
            b.div_id(
                Part::Meter,
                tw().w_full()
                    .h_px(4.0)
                    .rounded_full()
                    .overflow_hidden()
                    .bg(k[Tok::Line]),
                none,
                |b| {
                    b.rule(
                        tw().w_frac(used, 1.0)
                            .h_full()
                            .rounded_full()
                            .bg(k[Tok::Accent]),
                    );
                },
            );
        },
    );
}

/// ```html
/// <aside class="flex flex-col shrink-0 gap-5 p-4 bg-card overflow-hidden" style="width: 288px">...</aside>
/// ```
pub fn details(s: &mut S<'_, '_>, app: &App) {
    let k = &Theme::of(s.ctx());
    section(s, k, "Environment", |b| {
        for (label, value) in [
            ("Branch", "main"),
            ("Runtime", "local"),
            ("Model", MODELS[app.model]),
        ] {
            row(b, k, label, |b| b.text(tw().truncate(), value));
        }
    });
    section(s, k, "Context", |b| context(b, app));
    section(s, k, "Changes", |b| {
        for (file, added, removed) in CHANGES {
            row(b, k, file, |b| {
                b.div(tw().flex().gap(1.5), |b| {
                    b.text(tw().text(k[Tok::Success]), added);
                    b.text(tw().text(k[Tok::Danger]), removed);
                });
            });
        }
    });
    section(s, k, "Background tasks", |b| {
        for (name, state) in TASKS {
            row(b, k, name, |b| {
                b.text(tw().text(state.color(k)), state.label());
            });
        }
    });
}

/// The details pane's box, which its caller keys.
pub fn details_style(k: &Theme, app: &App) -> Tw {
    tw().flex()
        .flex_col()
        .w_px(app.width(Pane::Details).into())
        .shrink_0()
        .gap(5.0)
        .p(4.0)
        .bg(k[Tok::Panel])
        .overflow_hidden()
}
