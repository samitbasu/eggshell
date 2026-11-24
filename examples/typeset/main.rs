//! shadcn/typeset's fixtures in the Markdown widget, at shadcn's defaults.
//!
//! The article, docs and changelog are shadcn's typeset fixtures
//! (shadcn-ui/ui, `apps/v4/app/(app)/(typeset)/lib/fixtures`, MIT, © shadcn),
//! turned from HTML into Markdown with their text unchanged. What Markdown
//! cannot say is left as HTML, which the widget names and does not show: the
//! docs' MathML. The article's figure captions are dropped.

#[path = "../common/mod.rs"]
mod common;

use eggshell_ui::owned::markdown::{MarkdownExt, Typeset};
use eggshell_ui::{Builder, tw};

/// The parts the layout names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Bar,
    Fixture,
    Page,
    Doc,
}

/// One of shadcn's fixtures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Fixture {
    Article,
    #[default]
    Docs,
    Changelog,
}

impl Fixture {
    pub fn source(self) -> &'static str {
        match self {
            Self::Article => include_str!("article.md"),
            Self::Docs => include_str!("docs.md"),
            Self::Changelog => include_str!("changelog.md"),
        }
    }
}

const FIXTURES: [(Fixture, &str); 3] = [
    (Fixture::Article, "Article"),
    (Fixture::Docs, "Docs"),
    (Fixture::Changelog, "Changelog"),
];

#[derive(Default)]
pub struct State {
    pub fixture: Fixture,
}

/// ```html
/// <div class="flex flex-col size-full">
///   <header class="flex gap-3 p-3 border-b">[Article|Docs|Changelog]</header>
///   <main class="overflow-y-auto grow p-6"><article class="typeset max-w-[37em]">…</article></main>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    let tokens = common::tokens(b.ctx());
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
                b.segmented(Part::Fixture, state.fixture, FIXTURES)
                    .on_change(|state: &mut State, fixture| state.fixture = fixture);
            },
        );
        b.scroll(Part::Page, tw().grow().p(6.0)).children(|s| {
            let column = tw().max_w_px(Typeset::default().measure_width());
            s.div(column, |b| {
                b.markdown(Part::Doc, state.fixture.source()).add();
            });
        });
    });
}

fn main() -> eframe::Result {
    common::run_app("typeset", State::default(), layout)
}
