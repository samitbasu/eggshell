//! TODO(prose)
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui::prelude::*;
//! use eggshell_ui::owned::markdown::Directives;
//!
//! struct Docs {
//!     page: String,
//! }
//!
//! fn view<'f>(b: &mut Builder<'_, 'f, Docs>, docs: &'f Docs, directives: &Directives<Docs>) {
//!     b.markdown("intro", "# Hello\n\nSee [the guide](guide.md).")
//!         .on_link(|docs: &mut Docs, href: &str| docs.page = href.to_owned())
//!         .directives(directives, docs)
//!         .add();
//! }
//! ```

mod build;
mod parse;
mod style;
mod typeset;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::{Builder, Key, Problem, Scope};
use egui::cache::{ComputerMut, FrameCache};

pub use parse::{Heading, Level};
pub use style::{HeadingFlow, NarrowStyle, RichLook, Style};
pub use typeset::{Measure, Narrow, Typeset};

/// TODO(prose): a Markdown document's anatomy for the style's sheet. shadcn
/// has no Markdown component: these are eggshell's, and [`Typeset`] writes
/// their rules, after typeset.css. A thematic break (`---`) is a
/// [`separator`](crate::owned::separator).
///
/// TODO(prose): the keys its rules see: a heading's `data-[level=1..6]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the document.
    Root,
    /// TODO(prose): a paragraph, and a table cell's text.
    Paragraph,
    /// TODO(prose): a heading, keyed by its level.
    Heading,
    /// TODO(prose)
    Blockquote,
    /// TODO(prose): a fenced or indented code block's box.
    CodeBlock,
    /// TODO(prose): a code block's text, and the source of what is not shown.
    Code,
    /// TODO(prose)
    List,
    /// TODO(prose): a list item: its marker and its content.
    ListItem,
    /// TODO(prose)
    ListMarker,
    /// TODO(prose): the box of a construct the widget does not show.
    Unsupported,
    /// TODO(prose): the label naming it, and a pending image's alt text.
    UnsupportedLabel,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "typeset.css";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "markdown",
            Self::Paragraph => "markdown-paragraph",
            Self::Heading => "markdown-heading",
            Self::Blockquote => "markdown-blockquote",
            Self::CodeBlock => "markdown-code-block",
            Self::Code => "markdown-code",
            Self::List => "markdown-list",
            Self::ListItem => "markdown-list-item",
            Self::ListMarker => "markdown-list-marker",
            Self::Unsupported => "markdown-unsupported",
            Self::UnsupportedLabel => "markdown-unsupported-label",
        }
    }
}

impl Level {
    const ALL: [Self; 6] = [Self::H1, Self::H2, Self::H3, Self::H4, Self::H5, Self::H6];

    /// The `level` attribute of a heading at this level.
    fn attr(self) -> &'static str {
        match self {
            Self::H1 => "1",
            Self::H2 => "2",
            Self::H3 => "3",
            Self::H4 => "4",
            Self::H5 => "5",
            Self::H6 => "6",
        }
    }
}

/// TODO(prose)
pub trait Ext<'b, 'f, A> {
    /// TODO(prose)
    fn markdown<'s, K: Key + Clone>(
        &'s mut self,
        key: K,
        source: &'s str,
    ) -> Markdown<'s, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn markdown<'s, K: Key + Clone>(
        &'s mut self,
        key: K,
        source: &'s str,
    ) -> Markdown<'s, 'b, 'f, A, K> {
        Markdown {
            b: self,
            key,
            source,
            on_link: None,
            directives: None,
        }
    }
}

/// TODO(prose)
#[must_use = "the Markdown is not added until you call `.add()`"]
pub struct Markdown<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    source: &'s str,
    on_link: Option<OnLink<A>>,
    directives: Option<(&'s Directives<A>, &'f A)>,
}

type OnLink<A> = Rc<dyn Fn(&mut A, &str)>;

impl<'s, 'b, 'f, A: 'static, K: Key + Clone> Markdown<'s, 'b, 'f, A, K> {
    /// TODO(prose)
    pub fn on_link(self, on_link: impl Fn(&mut A, &str) + 'static) -> Self {
        Self {
            on_link: Some(Rc::new(on_link)),
            ..self
        }
    }

    /// TODO(prose)
    pub fn directives(self, directives: &'s Directives<A>, app: &'f A) -> Self {
        Self {
            directives: Some((directives, app)),
            ..self
        }
    }

    /// TODO(prose)
    #[track_caller]
    pub fn add(self) {
        let ctx = self.b.ctx().clone();
        let parsed = parsed(&ctx, self.source);
        let doc = self.b.id_of(self.key.clone());
        report_malformed(&ctx, doc, &parsed.malformed);
        let page = self.b.style().markdown.clone();
        let width = ctx.input_for(self.b.viewport(), |i| i.viewport_rect().width());
        let (style, viewport) = match page.narrow {
            Some(narrow) if width < narrow.below => (narrow.style, typeset::Viewport::Narrow),
            _ => (page, typeset::Viewport::Any),
        };
        let context = build::Context {
            style: Arc::new(style),
            viewport,
            links: Rc::new(build::Links::new(self.on_link, self.b.shell(), doc)),
            directives: self.directives,
            ctx,
        };
        build::document(self.b, self.key, &parsed.blocks, &context);
    }
}

/// Reports what of the document `doc` the parser had to recover from.
#[track_caller]
fn report_malformed(ctx: &egui::Context, doc: egui::Id, malformed: &[parse::Malformed]) {
    for &problem in malformed {
        crate::report(
            ctx,
            Problem::MalformedMarkdown,
            Some(doc.with(problem.name())),
            || format!("{}; {}", problem.name(), problem.recovery()),
        );
    }
}

/// TODO(prose)
pub struct Directive<'a> {
    /// The fence's first info word: the directive's name.
    pub name: &'a str,
    /// The rest of the fence's info string.
    pub arg: &'a str,
    /// The fence's text.
    pub body: &'a str,
}

type Mount<A> = Box<dyn for<'x, 'f> Fn(&mut Scope<'x, 'f, A>, &'f A, &Directive<'_>)>;

/// TODO(prose)
///
/// A fenced code block whose first info word names a directive mounts it in
/// place of the code.
pub struct Directives<A> {
    mounts: HashMap<String, Mount<A>>,
}

impl<A> Default for Directives<A> {
    fn default() -> Self {
        Self {
            mounts: HashMap::new(),
        }
    }
}

impl<A> Directives<A> {
    /// TODO(prose)
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO(prose)
    #[must_use]
    pub fn with(
        mut self,
        name: impl Into<String>,
        mount: impl for<'x, 'f> Fn(&mut Scope<'x, 'f, A>, &'f A, &Directive<'_>) + 'static,
    ) -> Self {
        self.mounts.insert(name.into(), Box::new(mount));
        self
    }

    fn mount(&self, name: &str) -> Option<&Mount<A>> {
        self.mounts.get(name)
    }
}

/// TODO(prose)
///
/// ```
/// # use eggshell_ui::{Builder, Shell};
/// # use eggshell_ui::prelude::*;
/// # use eggshell_ui::owned::markdown::heading_id;
/// fn page(b: &mut Builder<'_, '_, ()>, land_on: Option<&str>) {
///     let shell: Shell = b.shell();
///     if let Some(anchor) = land_on {
///         shell.scroll_into_view(heading_id(b.id_of("doc"), anchor), Some(egui::Align::Min));
///     }
///     b.markdown("doc", "# Install").add();
/// }
/// ```
pub fn heading_id(doc: egui::Id, anchor: &str) -> egui::Id {
    build::heading_id(doc, anchor)
}

/// TODO(prose)
pub fn headings(source: &str) -> Vec<Heading> {
    parse::headings(&parse::parse(source).blocks)
}

#[derive(Default)]
struct Parse;

impl ComputerMut<&str, Arc<parse::Parsed>> for Parse {
    fn compute(&mut self, source: &str) -> Arc<parse::Parsed> {
        Arc::new(parse::parse(source))
    }
}

/// The blocks of `source`, parsed once while it is shown. The cache is the
/// context's, not the layout's, and keyed by the source alone: sound because
/// the blocks are a function of the source and nothing else. Link handling,
/// directives and the style are applied when the blocks are built into
/// nodes, never when they are parsed.
fn parsed(ctx: &egui::Context, source: &str) -> Arc<parse::Parsed> {
    ctx.memory_mut(|memory| {
        memory
            .caches
            .cache::<FrameCache<Arc<parse::Parsed>, Parse>>()
            .get(source)
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use crate::testing::{assert_no_reports, assert_reported};

    use super::*;

    #[test]
    fn what_the_parser_recovered_from_is_reported_once_per_kind() {
        let ctx = egui::Context::default();
        let doc = egui::Id::NULL.with("doc");
        let malformed = [
            parse::Malformed::CellOutsideTable,
            parse::Malformed::BlockInTable,
            parse::Malformed::CellOutsideTable,
        ];
        report_malformed(&ctx, doc, &malformed);
        let reports = crate::testing::reports(&ctx);
        assert_eq!(reports.len(), 2, "{reports:#?}");
        assert!(
            reports
                .iter()
                .all(|report| report.problem == Problem::MalformedMarkdown)
        );
        let cell = assert_reported(&ctx, Problem::MalformedMarkdown);
        assert_eq!(cell.count, 2);
        assert!(cell.message.contains("paragraph"), "{}", cell.message);
        assert!(
            reports[1].message.contains("inside a table"),
            "{}",
            reports[1].message
        );
    }

    #[test]
    fn a_well_formed_document_reports_nothing() {
        let ctx = egui::Context::default();
        report_malformed(
            &ctx,
            egui::Id::NULL.with("doc"),
            &parse::parse("| a |\n|---|\n| b |").malformed,
        );
        assert_no_reports(&ctx);
    }
}
