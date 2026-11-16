//! TODO(prose)
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui_extras::markdown::{Directives, MarkdownExt};
//!
//! struct Docs {
//!     page: String,
//! }
//!
//! fn view(b: &mut Builder<'_, '_, Docs>, directives: &Directives<Docs>) {
//!     b.markdown("intro", "# Hello\n\nSee [the guide](guide.md).")
//!         .on_link(|docs: &mut Docs, href: &str| docs.page = href.to_owned())
//!         .directives(directives)
//!         .add();
//! }
//! ```

mod build;
mod parse;
mod style;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use eggshell_ui::{Builder, Key, Scope};
use egui::cache::{ComputerMut, FrameCache};

pub use parse::{Heading, Level};
pub use style::{
    CodeBlockStyle, HeadingRule, HeadingStyle, ListStyle, MarkdownStyle, QuoteStyle,
    UnsupportedStyle,
};

/// TODO(prose)
pub trait MarkdownExt<'b, 'f, A> {
    /// TODO(prose)
    fn markdown<'s, K: Key + Clone>(
        &'s mut self,
        key: K,
        source: &'s str,
    ) -> Markdown<'s, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> MarkdownExt<'b, 'f, A> for Builder<'b, 'f, A> {
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
    directives: Option<&'s Directives<A>>,
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
    pub fn directives(self, directives: &'s Directives<A>) -> Self {
        Self {
            directives: Some(directives),
            ..self
        }
    }

    /// TODO(prose)
    #[track_caller]
    pub fn add(self) {
        let ctx = self.b.ctx().clone();
        let blocks = parsed(&ctx, self.source);
        let context = build::Context {
            style: MarkdownStyle::of(&ctx),
            links: Rc::new(build::Links::new(
                self.on_link,
                self.b.shell(),
                self.b.id_of(self.key.clone()),
            )),
            directives: self.directives,
            ctx,
        };
        build::document(self.b, self.key, &blocks, &context);
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

type Mount<A> = Box<dyn for<'x, 'f> Fn(&mut Scope<'x, 'f, A>, &Directive<'_>)>;

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
        mount: impl for<'x, 'f> Fn(&mut Scope<'x, 'f, A>, &Directive<'_>) + 'static,
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
/// # use eggshell_ui_extras::markdown::{MarkdownExt, heading_id};
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
    parse::headings(&parse::parse(source))
}

#[derive(Default)]
struct Parse;

impl ComputerMut<&str, Arc<Vec<parse::Block>>> for Parse {
    fn compute(&mut self, source: &str) -> Arc<Vec<parse::Block>> {
        Arc::new(parse::parse(source))
    }
}

/// The blocks of `source`, parsed once while it is shown. The cache is the
/// context's, not the layout's, and keyed by the source alone: sound because
/// the blocks are a function of the source and nothing else. Link handling,
/// directives and the style are applied when the blocks are built into
/// nodes, never when they are parsed.
fn parsed(ctx: &egui::Context, source: &str) -> Arc<Vec<parse::Block>> {
    ctx.memory_mut(|memory| {
        memory
            .caches
            .cache::<FrameCache<Arc<Vec<parse::Block>>, Parse>>()
            .get(source)
            .clone()
    })
}
