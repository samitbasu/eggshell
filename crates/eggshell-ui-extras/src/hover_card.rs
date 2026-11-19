//! TODO(prose): a hover card, shadcn's `HoverCard`: a box that opens when the
//! pointer rests on a trigger, stays open while the pointer moves into it,
//! and holds content the pointer can use, such as a link preview with a
//! button.
//!
//! ```html
//! <span class="flex"><a href="#">@nextjs</a></span>
//! <div class="w-64 rounded-md border bg-popover p-4 text-popover-foreground shadow-md">
//!   <p class="font-semibold">@nextjs</p> <p>The React Framework</p>
//! </div>  <!-- 4 px below, centred -->
//! ```
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui_extras::hover_card::HoverCardExt as _;
//!
//! struct App { follows: u32 }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.hover_card("author")
//!         .trigger(tw().flex(), |t| {
//!             t.button("handle", "@nextjs").on_click(|_: &mut App| {});
//!         })
//!         .content(|c| {
//!             c.text(tw().font_semibold(), "@nextjs");
//!             c.text(tw(), format!("The React Framework, followed {} times", app.follows));
//!             c.button("follow", "Follow").on_click(|app: &mut App| app.follows += 1);
//!         });
//! }
//! ```
//!
//! The card is a supplement, as in Radix: Tab never stops in it and focus
//! never moves into it, so what it holds that matters must be reachable some
//! other way. Its text describes the trigger's widgets to assistive
//! technology.

use core::time::Duration;
use std::sync::Arc;

use eggshell_ui::contract::Look;
use eggshell_ui::overlay::{Align, Distance, HoverIntent, Placement};
use eggshell_ui::{Builder, Key, Scope, Style, Tw, tw};
use egui::{Context, Id, Theme};

/// TODO(prose): how hover cards look and when they open. [`of`](Self::of)
/// is the one installed for the context's theme, or one made from the
/// page's [`Style`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverCardStyle {
    /// TODO(prose): how long the pointer or focus rests on the trigger before
    /// the card opens.
    pub open_delay: Duration,
    /// TODO(prose): how long the card stays once neither the trigger nor the
    /// card is hovered.
    pub close_delay: Duration,
    /// TODO(prose): the box: its `rest` colours, border and shadow.
    pub look: Look,
    /// TODO(prose): corner radius of the box.
    pub radius: u8,
    /// TODO(prose): padding inside the box, in points.
    pub padding: f32,
    /// TODO(prose): width of the box, in points.
    pub width: f32,
    /// TODO(prose): the gap between the trigger and the box.
    pub offset: Distance,
}

impl HoverCardStyle {
    /// TODO(prose): Radix's delays, and the box of `style`'s menus.
    pub fn from_style(style: &Style) -> Self {
        Self {
            open_delay: Duration::from_millis(700),
            close_delay: Duration::from_millis(300),
            look: style.menu.popup,
            radius: style.menu.radius,
            padding: 16.0,
            width: 256.0,
            offset: style.menu.offset,
        }
    }

    /// TODO(prose)
    pub fn of(ctx: &Context) -> Arc<Self> {
        ctx.data(|data| data.get_temp::<Arc<Self>>(id(ctx.theme())))
            .unwrap_or_else(|| Arc::new(Self::from_style(&Style::of(ctx))))
    }

    /// TODO(prose)
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        ctx.data_mut(|data| data.insert_temp(id(theme), Arc::new(self)));
    }

    /// The classes of the card's box.
    fn classes(&self) -> Tw {
        let colors = self.look.paint.states.rest;
        let classes = tw()
            .flex()
            .flex_col()
            .w_px(self.width)
            .p(self.padding / 4.0)
            .border(self.look.border_width)
            .border_color(colors.border)
            .bg(colors.fill)
            .text(colors.text)
            .rounded_px(f32::from(self.radius));
        match self.look.paint.shadow {
            Some(shadow) => classes.shadow(shadow),
            None => classes,
        }
    }

    fn placement(&self) -> Placement {
        Placement::below().align(Align::Center).gap(self.offset)
    }
}

fn id(theme: Theme) -> Id {
    Id::NULL.with(("eggshell-ui-extras-hover-card-style", theme))
}

/// TODO(prose): adds [`hover_card`](Self::hover_card) to the builder.
pub trait HoverCardExt<'b, 'f, A> {
    /// TODO(prose): a hover card named `key`: give it a
    /// [`trigger`](HoverCard::trigger), then its
    /// [`content`](HoverCardTrigger::content).
    fn hover_card<K: Key + Clone>(&mut self, key: K) -> HoverCard<'_, 'b, 'f, A, K>;
}

impl<'b, 'f, A> HoverCardExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn hover_card<K: Key + Clone>(&mut self, key: K) -> HoverCard<'_, 'b, 'f, A, K> {
        HoverCard {
            b: self,
            key,
            placement: None,
            open_delay: None,
            close_delay: None,
        }
    }
}

/// TODO(prose): a hover card being declared, before its trigger.
#[must_use = "a hover card is added by `.trigger(..).content(..)`"]
pub struct HoverCard<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    placement: Option<Placement>,
    open_delay: Option<Duration>,
    close_delay: Option<Duration>,
}

impl<'s, 'b, 'f, A, K: Key + Clone> HoverCard<'s, 'b, 'f, A, K> {
    /// TODO(prose): where the card goes against the trigger: below it,
    /// centred, the style's `offset` apart, unless set.
    pub fn placement(self, placement: Placement) -> Self {
        Self {
            placement: Some(placement),
            ..self
        }
    }

    /// TODO(prose): the style's `open_delay` for this card.
    pub fn open_delay(self, delay: Duration) -> Self {
        Self {
            open_delay: Some(delay),
            ..self
        }
    }

    /// TODO(prose): the style's `close_delay` for this card.
    pub fn close_delay(self, delay: Duration) -> Self {
        Self {
            close_delay: Some(delay),
            ..self
        }
    }

    /// TODO(prose): the trigger: a container of `classes` that `body` fills.
    /// The pointer anywhere over it, or keyboard focus on anything inside
    /// it, opens the card; it replaces their tooltips.
    pub fn trigger<T>(self, classes: Tw, body: T) -> HoverCardTrigger<'s, 'b, 'f, A, K, T>
    where
        T: FnOnce(&mut Scope<'_, 'f, A>),
    {
        HoverCardTrigger {
            card: self,
            classes,
            body,
        }
    }
}

/// TODO(prose): a hover card with its trigger, waiting for its content.
#[must_use = "a hover card is added by `.content(..)`"]
pub struct HoverCardTrigger<'s, 'b, 'f, A, K, T> {
    card: HoverCard<'s, 'b, 'f, A, K>,
    classes: Tw,
    body: T,
}

impl<'f, A, K, T> HoverCardTrigger<'_, '_, 'f, A, K, T>
where
    K: Key + Clone,
    T: FnOnce(&mut Scope<'_, 'f, A>),
{
    /// TODO(prose): adds the card, whose box `content` fills. The content is
    /// built while the card is shown, and may borrow the application data
    /// and take input. While assistive technology is on it is also built
    /// where nothing shows it, for its text, which describes the trigger.
    pub fn content(self, content: impl Fn(&mut Builder<'_, 'f, A>)) {
        let Self {
            card,
            classes,
            body,
        } = self;
        let HoverCard {
            b,
            key,
            placement,
            open_delay,
            close_delay,
        } = card;
        let style = HoverCardStyle::of(b.ctx());
        let trigger = b.id_of(key.clone());
        let mut intent = HoverIntent::new(
            open_delay.unwrap_or(style.open_delay),
            close_delay.unwrap_or(style.close_delay),
        );
        if let Some(text) = b.description_of(("hover-card", trigger), &content) {
            intent = intent.description(text);
        }
        let placement = placement.unwrap_or_else(|| style.placement());
        b.div_id(key, classes, |t| {
            body(t);
            t.overlay("Card")
                .hover_anchored(trigger, intent)
                .placement(placement)
                .children(|c| {
                    c.div_id("Box", style.classes(), |c| content(c));
                });
        });
    }
}
