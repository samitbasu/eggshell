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
use crate::owned::{Enablement, button::Face};
use crate::{Angle, Builder, Icon, Key, NodeUi as _, Problem, Progress, Tw, tw};
use egui::{Color32, Role};

pub use section::SectionChain;
pub use style::{Glyph, HeaderStyle, Hover, Style};

/// TODO(prose): a section's `on_change`, not yet given.
pub struct Unset;

/// TODO(prose): a section's `on_change`, given.
pub struct OnChange<A>(ChangeFn<A>);

/// What a section runs with the state its header asks for.
type ChangeFn<A> = Rc<dyn Fn(&mut A, Shown)>;

/// TODO(prose): writes the looks the user chose where they differ from
/// shadcn's accordion (#350, #384) over a preset's generated sheet: the
/// header keeps its gap between the title and the chevron, and the chevron
/// its nudge down to the title's line.
pub fn write_exceptions(sheet: &mut crate::sheet::Sheet, style: &Style) {
    let (gap, nudge) = (style.header.gap, style.nudge);
    sheet.rule(accordion::Part::Trigger, move |t, _| t.gap_px(gap));
    sheet.rule(accordion::Part::TriggerIcon, move |t, _| {
        t.mt_px(nudge).mb_px(-nudge)
    });
}

/// TODO(prose): `glyph`'s icon turned by its turn times `open`, zero closed
/// and one open: the chevron of a section header and of a disclosure
/// button, for an app that draws one of its own.
///
/// ```
/// use eggshell_ui::Progress;
/// use eggshell_ui::owned::disclosure::chevron;
///
/// let style = eggshell_ui::Style::default().disclosure;
/// let open = chevron(&style.section, Progress::END);
/// # let _ = open;
/// ```
pub fn chevron(glyph: &Glyph, open: Progress) -> Icon {
    glyph
        .icon
        .clone()
        .rotated(Angle::radians(f32::from(glyph.turn) * f32::from(open)))
}

/// The classes of a chevron drawn in `style`.
fn chevron_classes(style: &Style, glyph: &Glyph) -> Tw {
    let classes = tw()
        .size_px(glyph.size)
        .shrink_0()
        .opacity(style.tone_opacity);
    match style.tone {
        Some(tone) => classes.text(tone),
        None => classes,
    }
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

/// Disabled if the place it is built in, or its own setting, is.
fn enablement_in<A>(b: &Builder<'_, '_, A>, own: Enablement) -> Enablement {
    match (b.enablement(), own) {
        (Enablement::Enabled, Enablement::Enabled) => Enablement::Enabled,
        _ => Enablement::Disabled,
    }
}

/// The classes of a header or button box in `style`: a Tab stop with the
/// focus ring, and inert and faded while disabled, as shadcn's
/// `disabled:opacity-50`. The text keeps the colour it inherits.
fn box_classes(style: &Style, enablement: Enablement) -> Tw {
    let header = &style.header;
    let classes = tw()
        .sense_click()
        .rounded_px(f32::from(header.radius))
        .transition(header.transition);
    let classes = match header.focus_border {
        Some(_) => classes.border(1.0).border_color(Color32::TRANSPARENT),
        None => classes,
    };
    match enablement {
        Enablement::Enabled => classes.focus(|paint| {
            let paint = paint.ring(header.ring.width, header.ring.color);
            match header.focus_border {
                Some(border) => paint.border_color(border),
                None => paint,
            }
        }),
        Enablement::Disabled => classes.inert().opacity(header.disabled),
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
    /// with [`named`](Chain::named): an unnamed one is reported
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
            name: None,
            classes: tw(),
            enablement: Enablement::Enabled,
        }
    }
}

/// TODO(prose): a disclosure button: see
/// [`Ext::disclosure`].
#[must_use = "a disclosure button is added by `.on_change(..)`"]
pub struct Chain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    current: Shown,
    name: Option<String>,
    classes: Tw,
    enablement: Enablement,
}

impl<A: 'static, K: Key> Chain<'_, '_, '_, A, K> {
    /// TODO(prose): the accessible name: the row's label. It stays the same
    /// open or closed; `expanded` says which.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): classes merged over the styled ones; the later class
    /// wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = std::mem::replace(&mut self.classes, tw()).merge(classes);
        self
    }

    /// TODO(prose): whether the button takes input. Inside an inert
    /// subtree it is disabled either way.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        self.enablement = enablement.into();
        self
    }

    /// TODO(prose): adds the button; `on_change` runs with the state a
    /// click, Enter or Space asks for.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, Shown) + 'static) {
        let Self {
            b,
            key,
            current,
            name,
            classes,
            enablement,
        } = self;
        let _ctx = b.ctx().clone();
        let style = b.style().disclosure.clone();
        let enablement = enablement_in(b, enablement);
        let box_classes = box_classes(&style, enablement);
        let box_classes = match (style.button_hover, enablement) {
            (Some(fill), Enablement::Enabled) => box_classes.hover(|paint| paint.bg(fill)),
            _ => box_classes,
        };
        let box_classes = box_classes
            .flex()
            .items_center()
            .justify_center()
            .size_px(style.button)
            .shrink_0()
            .merge(classes);
        let glyph = chevron(&style.tree, progress_of(current));
        let glyph_classes = chevron_classes(&style, &style.tree);
        let button = b
            .div_id(key, box_classes, |b| b.glyph(glyph_classes, glyph))
            .role(Role::Button)
            .expanded(current.is_open());
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
        if enablement == Enablement::Enabled {
            button.on_click(move |app: &mut A| on_change(app, toggled(current)));
        }
    }
}
