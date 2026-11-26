//! TODO(prose): alerts, after shadcn/ui's Alert: a callout with a title, an
//! optional description, an icon and one action at its top right.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::prelude::*;
//! use eggshell_ui::theme::variant::DESTRUCTIVE;
//!
//! struct App { armed: bool }
//!
//! fn view(b: &mut Builder<'_, '_, App>, _: &App) {
//!     b.alert("trip")
//!         .variant(DESTRUCTIVE)
//!         .title("Input clipped on channel 2")
//!         .description("Lower the gain, or re-arm.")
//!         .action("Re-arm", |button| button.on_click(|app: &mut App| app.armed = true))
//!         .add();
//! }
//! ```

use egui::Role;
use egui::accesskit::Live;

use crate::contract::Variant;
use crate::owned::button;
use crate::sheet::{Key as SheetKey, Part as _, Sheet, When};
use crate::theme::{ShadcnStyle, Tokens, size, variant};
use crate::{Builder, Icon, Key, Match, Track, Tw, icons, tw};

/// TODO(prose): an alert's anatomy for the style's sheet: shadcn's `alert`.
/// Its icon is not a part: shadcn styles it from the root (`*:[svg]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the alert's box.
    Root,
    /// TODO(prose): the title.
    Title,
    /// TODO(prose): the description under the title.
    Description,
    /// TODO(prose): the box at the top right that holds the action button.
    Action,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "alert.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "alert",
            Self::Title => "alert-title",
            Self::Description => "alert-description",
            Self::Action => "alert-action",
        }
    }
}

type ToneColor = fn(&Tokens) -> egui::Color32;

/// The tones' looks, which shadcn's alert does not have: each is the
/// destructive look in its tone's colour.
const TONES: [(Variant, ToneColor); 3] = [
    (variant::INFO, |k| k.info),
    (variant::SUCCESS, |k| k.success),
    (variant::WARNING, |k| k.warning),
];

/// Writes the alert's recorded exceptions over its generated look.
///
/// The icon's classes, which the importer cannot map yet: the grid gets an
/// `auto` column for it (`has-[>svg]:grid-cols-[auto_1fr]`), and it spans
/// the title's and the description's rows at `size-4`, nudged down
/// `translate-y-0.5` in Vega (as a top margin). The title's `col-start-2` beside an icon is
/// what auto-placement gives it, so it is not written. Lyra's description is
/// `text-xs/relaxed`, and its action sits `1.25` in from the corner.
///
/// The tones: [`INFO`](variant::INFO), [`SUCCESS`](variant::SUCCESS) and
/// [`WARNING`](variant::WARNING) are the destructive look in their tone's
/// colour: the text and icon in the tone, the description at 90%, the card's
/// fill and the normal border.
pub(crate) fn write_exceptions(sheet: &mut Sheet) {
    sheet.rule(Part::Root, |t, k| {
        // `translate-y-*` has no rest-state class; a top margin moves the
        // icon as far, as nothing grows from it.
        let nudge = match k.style {
            ShadcnStyle::Vega => 0.5,
            ShadcnStyle::Lyra => 0.0,
        };
        t.has_child(Match::glyph(), |b| {
            b.grid_cols_tracks(&[Track::auto(), Track::fr(1.0)])
        })
        .children(Match::glyph(), |t| t.row_span(2).size(4.0).mt(nudge))
    });
    sheet.rule(Part::Description, |t, k| match k.style {
        ShadcnStyle::Vega => t,
        ShadcnStyle::Lyra => t.text_step(k.text.xs).leading_relaxed(),
    });
    sheet.rule(Part::Action, |t, k| match k.style {
        ShadcnStyle::Vega => t,
        ShadcnStyle::Lyra => t.top(1.25).right(1.25),
    });
    for (tone, color) in TONES {
        sheet.rule_when(Part::Root, When::always().variant(tone), move |t, k| {
            let tone = color(k);
            t.text(tone)
                .bg(k.card)
                .children(Match::any().with_slot(Part::Description.slot()), |t| {
                    t.text(tone.gamma_multiply(0.9))
                })
        });
    }
}

/// The icon a look draws when the alert names none: shadcn's destructive
/// example's, and one per tone. The default look has none.
fn look_icon(variant: Variant) -> Option<Icon> {
    let source = match variant {
        variant::DESTRUCTIVE => icons::CIRCLE_ALERT,
        variant::INFO => icons::INFO,
        variant::SUCCESS => icons::CIRCLE_CHECK,
        variant::WARNING => icons::WARNING,
        _ => return None,
    };
    Some(Icon::new(source))
}

/// TODO(prose): how assistive technology hears an alert: its role and
/// whether it is a live region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Announce {
    /// TODO(prose): a status, announced at the next pause in speech.
    #[default]
    Polite,
    /// TODO(prose): an alert, announced at once, interrupting.
    Assertive,
    /// TODO(prose): not announced: a static callout, a group named by its
    /// title, if it has one.
    Off,
}

/// TODO(prose): adds [`alert`](Self::alert) to the builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): an alert. Finish it with [`add`](Chain::add).
    fn alert<'s, K: Key>(&'s mut self, key: K) -> Chain<'s, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn alert<'s, K: Key>(&'s mut self, key: K) -> Chain<'s, 'b, 'f, A, K> {
        Chain {
            b: self,
            key,
            variant: None,
            icon: Glyph::Look,
            title: None,
            description: None,
            lines: Lines::Wrap,
            action: None,
            announce: Announce::default(),
            classes: tw(),
        }
    }
}

/// Which icon the alert draws.
enum Glyph {
    /// The look's own, if it has one.
    Look,
    Set(Icon),
    None,
}

/// How the title and description fit their width.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lines {
    Wrap,
    Truncate,
}

type ActionButton<'s, 'f, A> = Box<dyn for<'c> FnOnce(button::Chain<'c, 'f, A>) + 's>;

/// TODO(prose): an alert, after shadcn/ui's Alert. Start one with
/// [`Ext::alert`] and finish it with [`add`](Self::add).
///
/// ```html
/// <div data-slot="alert" role="alert" class="relative grid w-full gap-0.5 rounded-lg border px-4 py-3 text-left text-sm bg-card text-card-foreground">…</div>
/// ```
#[must_use = "an alert is not added until you call `.add()`"]
pub struct Chain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    variant: Option<Variant>,
    icon: Glyph,
    title: Option<String>,
    description: Option<String>,
    lines: Lines,
    action: Option<(String, ActionButton<'s, 'f, A>)>,
    announce: Announce,
    classes: Tw,
}

impl<'s, 'f, A: 'static, K: Key> Chain<'s, '_, 'f, A, K> {
    /// TODO(prose): draws the alert as the style's look `variant`:
    /// [`DESTRUCTIVE`](variant::DESTRUCTIVE), [`INFO`](variant::INFO),
    /// [`SUCCESS`](variant::SUCCESS) or [`WARNING`](variant::WARNING).
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// TODO(prose): draws `icon` before the text, in the text's colour,
    /// instead of the look's.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Glyph::Set(icon);
        self
    }

    /// TODO(prose): draws no icon, not even the look's.
    pub fn without_icon(mut self) -> Self {
        self.icon = Glyph::None;
        self
    }

    /// TODO(prose): the title, the alert's message. An alert may have only a
    /// description, as shadcn's may.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// TODO(prose): the description under the title; also the alert's
    /// accessible description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// TODO(prose): keeps the title and the description to one line each,
    /// ending in an ellipsis when cut, with the whole text in a tooltip.
    pub fn truncate(mut self) -> Self {
        self.lines = Lines::Truncate;
        self
    }

    /// TODO(prose): a button reading `label` at the top right. `button` gets
    /// the core button, already outline and `xs`, and finishes it with its
    /// handler; it may set another variant or size.
    pub fn action(
        mut self,
        label: impl Into<String>,
        button: impl for<'c> FnOnce(button::Chain<'c, 'f, A>) + 's,
    ) -> Self {
        self.action = Some((label.into(), Box::new(button)));
        self
    }

    /// TODO(prose): how assistive technology hears the alert;
    /// [`Announce::Polite`] unless set.
    pub fn announce(mut self, announce: Announce) -> Self {
        self.announce = announce;
        self
    }

    /// TODO(prose): classes merged over the styled ones; the later class
    /// wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = std::mem::replace(&mut self.classes, tw()).merge(classes);
        self
    }

    /// TODO(prose): adds the alert.
    #[track_caller]
    pub fn add(self) {
        let Self {
            b,
            key,
            variant,
            icon,
            title,
            description,
            lines,
            action,
            announce,
            classes,
        } = self;
        let variant = variant.unwrap_or(Variant::DEFAULT);
        let icon = match icon {
            Glyph::Look => look_icon(variant),
            Glyph::Set(icon) => Some(icon),
            Glyph::None => None,
        };
        let line = match lines {
            Lines::Wrap => tw(),
            Lines::Truncate => tw().truncate().min_w_0(),
        };
        // A node shows its role to assistive technology only if it senses.
        let own = tw().sense_hover();
        let root = b.part(
            Part::Root,
            &[SheetKey::Variant(variant)],
            own.merge(classes),
        );
        let named = title.clone();
        let node = b.div_id(key, root, |b| {
            if let Some(icon) = icon {
                b.glyph(tw(), icon);
            }
            if let Some(title) = title {
                let classes = b.part(Part::Title, &[], line.clone());
                let title = b.span("title", classes, title);
                if lines == Lines::Truncate {
                    title.tooltip_when_truncated();
                }
            }
            if let Some(description) = &description {
                let classes = b.part(Part::Description, &[], line);
                let description = b.span("description", classes, description.clone());
                if lines == Lines::Truncate {
                    description.tooltip_when_truncated();
                }
            }
            if let Some((label, button)) = action {
                let classes = b.part(Part::Action, &[], tw());
                b.div_id("action", classes, |b| {
                    button(
                        b.button("button", label)
                            .variant(variant::OUTLINE)
                            .size(size::XS),
                    );
                });
            }
        });
        let node = match announce {
            Announce::Polite => node.role(Role::Status).live(Live::Polite),
            Announce::Assertive => node.role(Role::Alert).live(Live::Assertive),
            Announce::Off => match named {
                Some(title) => node.role(Role::Group).named(title),
                None => node.role(Role::Group),
            },
        };
        if let Some(description) = description {
            node.description(description);
        }
    }
}
