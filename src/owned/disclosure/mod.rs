//! TODO(prose): a collapsible section with an owned header, and the
//! disclosure button that folds a tree row.
//!
//! A [`section`](SectionExt::section) is a header over a
//! [`collapsible`](crate::Builder::collapsible): the header is a
//! button that says whether the section is open and which region it
//! controls, and its chevron turns with the region's height. A
//! [`disclosure`](Ext::disclosure) is the chevron alone, as a
//! button: it snaps.
//!
//! ```
//! use eggshell_ui::overlay::Shown;
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui::prelude::*;
//!
//! struct App { env: Shown, src: Shown }
//!
//! fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
//!     b.section("env", "Environment", app.env)
//!         .on_change(|app: &mut App, shown| app.env = shown)
//!         .children(|b| b.text(tw(), "main"));
//!     b.disclosure("src", app.src)
//!         .named("src")
//!         .on_change(|app: &mut App, shown| app.src = shown);
//! }
//! ```

mod section;
mod style;

use crate::owned::accordion;
use std::rc::Rc;

use crate::overlay::Shown;
use crate::owned::options::{Configure, Interactive, Options, Resolved, Respond};
use crate::owned::{Enablement, OnResponse, button::Face, hooked};
use crate::{Builder, Key, NodeUi as _, Problem, Progress, tw};
use egui::{Color32, Role};

pub use section::SectionChain;
pub use style::{HeaderStyle, Style};

/// TODO(prose): a section's `on_change`, not yet given.
pub struct Unset;

/// TODO(prose): a section's `on_change`, given.
pub struct OnChange<A>(ChangeFn<A>);

/// What a section runs with the state its header asks for.
type ChangeFn<A> = Rc<dyn Fn(&mut A, &egui::Context, Shown)>;

/// TODO(prose): writes the looks the user chose where they differ from
/// shadcn's accordion (#350, #384) over a preset's generated sheet: the
/// header keeps its gap between the title and the chevron, and the chevron
/// its nudge down to the title's line.
///
/// shadcn rules its items off with `not-last:border-b` on each item. An
/// accordion's items are built one at a time, so whether an item is the last
/// is not known when its classes are; the accordion's column draws the same
/// lines between its items with `divide-y`, which, unlike a border, takes no
/// room, as the separator has always been drawn.
pub fn write_exceptions(sheet: &mut crate::sheet::Sheet, style: &Style) {
    sheet.rule(accordion::Part::Root, |t, k| t.divide_y(1.0, k.border));
    let (gap, nudge) = (style.header.gap, style.nudge);
    sheet.rule(accordion::Part::Trigger, move |t, _| t.gap_px(gap));
    sheet.rule(accordion::Part::TriggerIcon, move |t, _| {
        t.mt_px(nudge).mb_px(-nudge)
    });
}

/// TODO(prose): a disclosure button's anatomy for the style's sheet, as
/// shadcn names a lone open/close trigger: `collapsible.tsx`'s.
///
/// TODO(prose): the keys its rules see: `aria-expanded` and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollapsiblePart {
    /// TODO(prose): the button.
    Trigger,
    /// TODO(prose): its chevron.
    TriggerIcon,
}

impl crate::sheet::Part for CollapsiblePart {
    const SOURCE: &'static str = "collapsible.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Trigger => "collapsible-trigger",
            Self::TriggerIcon => "collapsible-trigger-icon",
        }
    }
}

/// Writes the disclosure button's recorded exceptions: shadcn's collapsible
/// has no classes, so the look is the one the user chose for it: a 24 pt
/// box, `rounded-md` (square in Lyra), its state changes over `transition`,
/// the accent under the pointer, the focus ring with a `ring` border, faded
/// to half while disabled; a 16 pt chevron in `muted-foreground`. A section
/// header's face icon is `size-4`, as the accordion's chevron is.
pub(crate) fn write_collapsible_exceptions(
    sheet: &mut crate::sheet::Sheet,
    transition: std::time::Duration,
) {
    use crate::sheet::When;
    use crate::theme::ShadcnStyle;
    sheet.rule(CollapsiblePart::Trigger, move |t, k| {
        let radius = match k.style {
            ShadcnStyle::Vega => f32::from(k.radius_md()),
            ShadcnStyle::Lyra => 0.0,
        };
        let (ring, accent, focus_border) = (k.focus(), k.accent, k.ring);
        t.size_px(24.0)
            .rounded_px(radius)
            .transition(transition)
            .border(1.0)
            .border_color(Color32::TRANSPARENT)
            .hover(move |paint| paint.bg(accent))
            .focus(move |paint| {
                paint
                    .ring(ring.width, ring.color)
                    .border_color(focus_border)
            })
    });
    sheet.rule_when(
        CollapsiblePart::Trigger,
        When::always().aria(crate::Aria::Disabled),
        |t, _| t.opacity(crate::Opacity::new(0.5)),
    );
    sheet.rule(CollapsiblePart::TriggerIcon, |t, k| {
        t.size_px(16.0).text(k.muted_foreground)
    });
    sheet.rule(accordion::Part::Trigger, |t, _| {
        t.descendants(crate::Match::glyph(), |glyph| glyph.size(4.0))
    });
}

fn toggled(shown: Shown) -> Shown {
    match shown {
        Shown::Open => Shown::Closed,
        Shown::Closed => Shown::Open,
    }
}

fn progress_of(shown: Shown) -> Progress {
    match shown {
        Shown::Open => Progress::END,
        Shown::Closed => Progress::START,
    }
}

/// The accessible name a face gives.
fn name_of(face: &Face) -> &str {
    match face {
        Face::Label(name) | Face::Icon(_, name) | Face::Empty(name) => name,
    }
}

/// TODO(prose): adds [`section`](Self::section) to the builder.
pub trait SectionExt<'b, 'f, A> {
    /// TODO(prose): a section headed by `face`, open as `current` says. It
    /// is built by [`children`](SectionChain::children) once it has an
    /// [`on_change`](SectionChain::on_change):
    ///
    /// ```compile_fail
    /// use eggshell_ui::Builder;
    /// use eggshell_ui::overlay::Shown;
    /// use eggshell_ui::prelude::*;
    ///
    /// fn view(b: &mut Builder<'_, '_, ()>) {
    ///     b.section("env", "Environment", Shown::Open).children(|_| {});
    /// }
    /// ```
    fn section<'s, K: Key>(
        &'s mut self,
        key: K,
        face: impl Into<Face>,
        current: Shown,
    ) -> SectionChain<'s, 'b, 'f, A, K, Unset>;
}

impl<'b, 'f, A: 'static> SectionExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn section<'s, K: Key>(
        &'s mut self,
        key: K,
        face: impl Into<Face>,
        current: Shown,
    ) -> SectionChain<'s, 'b, 'f, A, K, Unset> {
        SectionChain::new(self, key, face.into(), current)
    }
}

/// TODO(prose): adds [`disclosure`](Self::disclosure) to the builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): a disclosure button, a chevron that says whether what it
    /// folds is open as `current` says, for a tree row. It snaps. Name it
    /// with [`named`](Configure::named): an unnamed one is reported
    /// as [`Unnamed`](Problem::Unnamed). It is added by
    /// [`on_change`](Chain::on_change).
    fn disclosure<'s, K: Key>(&'s mut self, key: K, current: Shown) -> Chain<'s, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn disclosure<'s, K: Key>(&'s mut self, key: K, current: Shown) -> Chain<'s, 'b, 'f, A, K> {
        Chain {
            b: self,
            key,
            current,
            opts: Options::new(),
            on_response: OnResponse::default(),
        }
    }
}

/// TODO(prose): a disclosure button: see
/// [`Ext::disclosure`].
///
/// Its shared options come from [`Configure`], [`Interactive`] and
/// [`Respond`], in scope through the [`prelude`](crate::prelude), and are
/// all the button's. Its name is the row's label, the same open or closed;
/// `expanded` says which.
#[must_use = "a disclosure button is added by `.on_change(..)`"]
pub struct Chain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    current: Shown,
    opts: Options,
    on_response: OnResponse<A>,
}

impl<A, K> Configure for Chain<'_, '_, '_, A, K> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, K> Interactive for Chain<'_, '_, '_, A, K> {}

impl<A, K> Respond for Chain<'_, '_, '_, A, K> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<A: 'static, K: Key> Chain<'_, '_, '_, A, K> {
    /// TODO(prose): adds the button; `on_change` runs with the state a
    /// click, Enter or Space asks for.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, Shown) + 'static) {
        self.on_change_with(move |app, _, shown| on_change(app, shown));
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets the egui
    /// context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, on_change: impl Fn(&mut A, &egui::Context, Shown) + 'static) {
        let Self {
            b,
            key,
            current,
            opts,
            on_response,
        } = self;
        let slot = b.slot(&key);
        let Resolved {
            name,
            classes,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(b, slot.id);
        let style = b.style().disclosure.clone();
        let mut keys = vec![];
        if current.is_open() {
            keys.push(crate::sheet::Key::Aria(crate::Aria::Expanded));
        }
        let sense = match enablement {
            Enablement::Enabled => tw().sense_click(),
            Enablement::Disabled => {
                keys.push(crate::sheet::Key::Aria(crate::Aria::Disabled));
                tw().sense_click().inert()
            }
        };
        let box_classes = b.part(
            CollapsiblePart::Trigger,
            &keys,
            sense
                .flex()
                .items_center()
                .justify_center()
                .shrink_0()
                .merge(classes),
        );
        let glyph = style.tree.at(progress_of(current));
        let glyph_classes = b.part(CollapsiblePart::TriggerIcon, &keys, tw().shrink_0());
        let button = b
            .div_id_with(&slot, box_classes, |b| b.glyph(glyph_classes, glyph))
            .role(Role::Button)
            .expanded(current.is_open())
            .open_focus(autofocus.open_focus());
        let button = match name {
            Some(name) => button.named(name),
            None => button.draw(|_, ui| {
                crate::report(
                    ui.ctx(),
                    Problem::Unnamed,
                    Some(ui.node_response().id),
                    || {
                        "a disclosure button has no name; give it the row's label with `.named(..)`"
                            .to_owned()
                    },
                );
            }),
        };
        let button = button.draw(hooked(tooltip, autofocus, on_response));
        if enablement == Enablement::Enabled {
            button.on_click_with(move |app: &mut A, ctx| on_change(app, ctx, toggled(current)));
        }
    }
}
