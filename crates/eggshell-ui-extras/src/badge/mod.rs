//! TODO(prose): badges, small labels after shadcn/ui's Badge, and chips,
//! badges that can be clicked or removed.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui_extras::badge::BadgeExt;
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

use eggshell_ui::contract::{Look, Variant};
use eggshell_ui::sheet::{Defaults, Key as SheetKey, Part};
use eggshell_ui::{Builder, Icon, Key, Tw, tw};

/// TODO(prose): a badge's anatomy for the style's sheet: shadcn's `badge`.
/// A chip is a badge too, one that can be clicked or removed.
///
/// TODO(prose): the keys its rules see: the variant, and `data-[link=true]`
/// on a chip that can be clicked, shadcn's badge rendered as a link (`[a]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BadgePart {
    /// TODO(prose): the badge.
    Root,
}

impl Part for BadgePart {
    const SOURCE: &'static str = "badge.tsx";

    fn slot(self) -> &'static str {
        "badge"
    }
}

/// The attribute of a chip that can be clicked: shadcn's badge as a link.
const LINK: (&str, &str) = ("link", "true");

pub use chip::{ChipChain, Handler, Unset};
pub use style::{BadgeStyle, RemoveStyle};

/// TODO(prose): adds [`badge`](Self::badge) and [`chip`](Self::chip) to the
/// builder.
pub trait BadgeExt<'b, 'f, A> {
    /// TODO(prose): a badge reading `text`: a label that takes no input.
    /// Finish it with [`add`](BadgeChain::add).
    fn badge<'s, K: Key>(
        &'s mut self,
        key: K,
        text: impl Into<String>,
    ) -> BadgeChain<'s, 'b, 'f, A, K>;

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

impl<'b, 'f, A: 'static> BadgeExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn badge<'s, K: Key>(
        &'s mut self,
        key: K,
        text: impl Into<String>,
    ) -> BadgeChain<'s, 'b, 'f, A, K> {
        BadgeChain {
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
    fn look(&self, ctx: &egui::Context, style: &BadgeStyle) -> Look {
        *crate::styles::entry(ctx, &style.variants, self.variant(style))
    }

    fn variant(&self, style: &BadgeStyle) -> Variant {
        self.options.variant.unwrap_or(style.variants.default_key())
    }

    /// The variant the style draws for the badge's: a variant it lacks is
    /// drawn as its default.
    fn drawn(&self, style: &BadgeStyle) -> Variant {
        style.variants.key_for(self.variant(style))
    }

    /// The badge's classes: `own`, its look, as the defaults of its part,
    /// under the style's sheet, then `call_site` and the caller's over them.
    #[track_caller]
    fn part<A>(
        &mut self,
        b: &mut Builder<'_, '_, A>,
        style: &BadgeStyle,
        (own, link): (Tw, bool),
        call_site: Tw,
    ) -> Tw {
        let mut keys = vec![SheetKey::Variant(self.drawn(style))];
        let stop = match link {
            true => {
                keys.push(SheetKey::Attr(LINK.0, LINK.1));
                tw().sense_click()
            }
            false => tw(),
        };
        let caller = std::mem::replace(&mut self.options.classes, tw());
        b.part_over(
            BadgePart::Root,
            &keys,
            Defaults(own),
            stop.merge(call_site).merge(caller),
        )
    }

    /// The classes that size and lay out the face's box, before its colours.
    fn frame(&self, style: &BadgeStyle, end: End) -> Tw {
        let start = match self.options.icon {
            Some(_) => style.icon_padding,
            None => style.padding_x,
        };
        let end = match end {
            End::Label => style.padding_x,
            End::Remove => style.remove.padding_end,
        };
        tw().flex()
            .items_center()
            .justify_center()
            .w_fit()
            .shrink_0()
            .h_px(style.height)
            .pl(start / 4.0)
            .pr(end / 4.0)
            .gap(style.gap / 4.0)
            .rounded_px(f32::from(style.radius))
            .role(style.text.clone())
            .whitespace_nowrap()
            .select_none()
    }

    /// Builds the icon and the label.
    fn content<A>(self, b: &mut Builder<'_, '_, A>, style: &BadgeStyle) {
        if let Some(icon) = self.options.icon {
            b.glyph(
                tw().size_px(style.glyph)
                    .shrink_0()
                    .attr("icon", "inline-start"),
                icon,
            );
        }
        b.text(tw(), self.label);
    }
}

/// TODO(prose): a badge, after shadcn/ui's Badge: a label drawn in the
/// [`BadgeStyle`] variant's rest colours, which takes no input. Start one
/// with [`BadgeExt::badge`] and finish it with [`add`](Self::add).
///
/// ```html
/// <span class="inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 rounded-4xl border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap bg-primary text-primary-foreground">Badge</span>
/// ```
#[must_use = "a badge is not added until you call `.add()`"]
pub struct BadgeChain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    face: Face,
}

impl<A: 'static, K: Key> BadgeChain<'_, '_, '_, A, K> {
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
        let style = b.style().extension::<BadgeStyle>();
        let look = face.look(b.ctx(), &style);
        let rest = look.paint.states.rest;
        let own = face
            .frame(&style, End::Label)
            .border(look.border_width)
            .bg(rest.fill)
            .border_color(rest.border)
            .text(rest.text);
        let classes = face.part(b, &style, (own, false), tw());
        b.div_id(key, classes, |s| face.content(s, &style));
    }
}
