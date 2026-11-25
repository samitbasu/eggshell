//! TODO(prose): badges, small labels after shadcn/ui's Badge, and chips,
//! badges that can be clicked or removed.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::prelude::*;
//!
//! struct App { tags: Vec<String>, filter: Option<String> }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.badge("new", "New").add();
//!     for (index, tag) in app.tags.iter().enumerate() {
//!         let picked = tag.clone();
//!         b.chip(("tag", index), tag.as_str())
//!             .on_click(move |app: &mut App| app.filter = Some(picked.clone()))
//!             .on_remove(move |app: &mut App| {
//!                 app.tags.remove(index);
//!             })
//!             .add();
//!     }
//! }
//! ```

mod chip;
mod style;

use crate::contract::{Look, Variant};
use crate::sheet::Key as SheetKey;
use crate::{Builder, Icon, Key, Tw, tw};

/// TODO(prose): a badge's anatomy for the style's sheet: shadcn's `badge`.
/// A chip is a badge too, one that can be clicked or removed.
///
/// TODO(prose): the keys its rules see: the variant, and `data-[link=true]`
/// on a chip that can be clicked, shadcn's badge rendered as a link (`[a]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the badge.
    Root,
}

/// Whether a badge's root takes clicks, as a chip that is a button does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Root {
    Static,
    Clickable,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "badge.tsx";

    fn slot(self) -> &'static str {
        "badge"
    }
}

/// The attribute of a chip that can be clicked: shadcn's badge as a link.
const LINK: (&str, &str) = ("link", "true");

pub use chip::{ChipChain, Handler, Unset};
pub use style::{RemoveStyle, Style};

/// TODO(prose): adds [`badge`](Self::badge) and [`chip`](Self::chip) to the
/// builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): a badge reading `text`: a label that takes no input.
    /// Finish it with [`add`](Chain::add).
    fn badge<'s, K: Key>(&'s mut self, key: K, text: impl Into<String>) -> Chain<'s, 'b, 'f, A, K>;

    /// TODO(prose): a chip reading `label`: a badge that can be clicked, or
    /// removed with its × or with Backspace or Delete while it has focus, or
    /// both. It is added by [`add`](ChipChain::add) once it has an
    /// [`on_click`](ChipChain::on_click) or an
    /// [`on_remove`](ChipChain::on_remove).
    fn chip<'s, K: Key>(
        &'s mut self,
        key: K,
        label: impl Into<String>,
    ) -> ChipChain<'s, 'b, 'f, A, K, Unset, Unset>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn badge<'s, K: Key>(&'s mut self, key: K, text: impl Into<String>) -> Chain<'s, 'b, 'f, A, K> {
        Chain {
            b: self,
            key,
            face: Face {
                label: text.into(),
                options: Options::new(),
            },
        }
    }

    fn chip<'s, K: Key>(
        &'s mut self,
        key: K,
        label: impl Into<String>,
    ) -> ChipChain<'s, 'b, 'f, A, K, Unset, Unset> {
        ChipChain::new(self, key, label.into())
    }
}

/// What a badge and a chip both show and are drawn with.
struct Face {
    label: String,
    options: Options,
}

struct Options {
    variant: Option<Variant>,
    icon: Option<Icon>,
    classes: Tw,
}

impl Options {
    fn new() -> Self {
        Self {
            variant: None,
            icon: None,
            classes: tw(),
        }
    }
}

/// What ends a badge on the right.
#[derive(Clone, Copy, PartialEq, Eq)]
enum End {
    Label,
    Remove,
}

impl Face {
    /// The look of the face's variant, else the style's default.
    #[track_caller]
    /// Reports a variant `style` does not define.
    fn check_variant(&self, ctx: &egui::Context, style: &Style) {
        crate::owned::styles::entry(ctx, &style.variants, self.variant(style));
    }

    fn look(&self, ctx: &egui::Context, style: &Style) -> Look {
        *crate::owned::styles::entry(ctx, &style.variants, self.variant(style))
    }

    fn variant(&self, style: &Style) -> Variant {
        self.options.variant.unwrap_or(style.variants.default_key())
    }

    /// The variant the style draws for the badge's: a variant it lacks is
    /// drawn as its default.
    fn drawn(&self, style: &Style) -> Variant {
        style.variants.key_for(self.variant(style))
    }

    /// The badge's classes: the style's sheet, then `call_site` and the
    /// caller's over them.
    #[track_caller]
    fn part<A>(
        &mut self,
        b: &mut Builder<'_, '_, A>,
        style: &Style,
        root: Root,
        call_site: Tw,
    ) -> Tw {
        let mut keys = vec![SheetKey::Variant(self.drawn(style))];
        let stop = match root {
            Root::Clickable => {
                keys.push(SheetKey::Attr(LINK.0, LINK.1));
                tw().sense_click()
            }
            Root::Static => tw(),
        };
        let caller = std::mem::replace(&mut self.options.classes, tw());
        b.part(Part::Root, &keys, stop.merge(call_site).merge(caller))
    }

    /// Builds the icon and the label.
    fn content<A>(self, b: &mut Builder<'_, '_, A>) {
        if let Some(icon) = self.options.icon {
            b.glyph(tw().shrink_0().attr("icon", "inline-start"), icon);
        }
        b.text(tw(), self.label);
    }
}

/// TODO(prose): a badge, after shadcn/ui's Badge: a label drawn in the
/// [`Style`] variant's rest colours, which takes no input. Start one
/// with [`Ext::badge`] and finish it with [`add`](Self::add).
///
/// ```html
/// <span class="inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 rounded-4xl border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap bg-primary text-primary-foreground">Badge</span>
/// ```
#[must_use = "a badge is not added until you call `.add()`"]
pub struct Chain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    face: Face,
}

impl<A: 'static, K: Key> Chain<'_, '_, '_, A, K> {
    /// TODO(prose): draws the badge as the style's variant `variant`.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.face.options.variant = Some(variant);
        self
    }

    /// TODO(prose): puts `icon` before the text, in the text's colour.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.face.options.icon = Some(icon);
        self
    }

    /// TODO(prose): classes merged over the styled ones; the later class
    /// wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        let options = &mut self.face.options;
        options.classes = std::mem::replace(&mut options.classes, tw()).merge(classes);
        self
    }

    /// TODO(prose): adds the badge.
    #[track_caller]
    pub fn add(self) {
        let Self { b, key, mut face } = self;
        let style = b.style().badge.clone();
        face.check_variant(b.ctx(), &style);
        let classes = face.part(b, &style, Root::Static, tw());
        b.div_id(key, classes, |s| face.content(s));
    }
}
