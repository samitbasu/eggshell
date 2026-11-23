//! The owned button: a node that senses clicks, with a text child.

use std::fmt::Debug;

use egui::{Context, Role};

use super::Mark;
use super::{Enablement, Gate, IntoTip, OnResponse, centered, described, options};
use crate::Aria;
use crate::actions::Action;
use crate::builder::Slot;
use crate::contract::{Disabled, Keyed, Look, Metrics, Size, Variant};
use crate::focus::{Autofocus, Select};
use crate::sheet::{Key, Part, Sheet, When};
use crate::text::TextSpec;
use crate::tw::Match;
use crate::{Builder, Icon, Scope, Tw, tw};

/// TODO(prose): a button's anatomy for the [`Sheet`]: its box. shadcn styles
/// the icons inside it from the box (`[&_svg]:size-4`), so an icon is not a
/// part of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ButtonPart {
    /// TODO(prose): the button's box.
    Root,
}

impl Part for ButtonPart {
    const SOURCE: &'static str = "button.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "button",
        }
    }
}

/// What a style without a preset draws buttons with: one look, its sizes,
/// and how it focuses, fades and moves. [`button_defaults`] writes them as
/// the sheet's defaults.
pub(crate) struct ButtonDefaults {
    pub look: Look,
    pub sizes: Keyed<Size, Metrics>,
    pub text: TextSpec,
    pub transition: std::time::Duration,
    pub press_shift: f32,
}

/// Writes `defaults` as Button's rules in `sheet`: the look of every
/// variant, enabled and disabled, then each size.
pub(crate) fn button_defaults(sheet: &mut Sheet, defaults: ButtonDefaults) {
    let ButtonDefaults {
        look,
        sizes,
        text,
        transition,
        press_shift,
    } = defaults;
    let radius = sizes.get(sizes.default_key()).radius;
    sheet.rule_when(
        ButtonPart::Root,
        When::always().not_aria(Aria::Disabled),
        move |t, k| {
            let classes = t.merge(look.paint.classes_in(
                look.border_width,
                (k.focus, k.disabled),
                radius,
                Enablement::Enabled,
                transition,
            ));
            centered(classes).role(text.clone()).whitespace_nowrap()
        },
    );
    sheet.rule_when(
        ButtonPart::Root,
        When::always().aria(Aria::Disabled),
        move |t, k| {
            let classes = t.merge(look.paint.classes_in(
                look.border_width,
                (k.focus, k.disabled),
                radius,
                Enablement::Disabled,
                transition,
            ));
            let classes = match k.disabled {
                Disabled::Faded(opacity) => classes.opacity(opacity),
                Disabled::Colors => classes,
            };
            centered(classes).whitespace_nowrap()
        },
    );
    if press_shift != 0.0 {
        sheet.rule_when(
            ButtonPart::Root,
            When::always()
                .not_aria(Aria::Disabled)
                .not_aria(Aria::HasPopup),
            move |t, _| t.active(|p| p.translate_y(press_shift)),
        );
    }
    for size in sizes.keys() {
        let m = *sizes.get(size);
        sheet.rule_when(ButtonPart::Root, When::always().size(size), move |t, _| {
            t.h_px(m.height)
                .min_w_px(m.min_width)
                .px_px(m.padding_x)
                .gap_px(m.gap)
                .rounded_px(f32::from(m.radius))
                .descendants(Match::glyph(), |g| g.size_px(m.glyph).shrink_0())
        });
    }
}

/// Whether pressing a button shifts it down, as web buttons commonly do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Shifts,
    Stays,
}

/// What a button shows: a label, or only an icon.
///
/// A `&str` or `String` is a label, read by its text. An [`Icon`] with a
/// name is an icon-only face, which assistive technology reads by the name.
/// [`Face::Empty`] shows nothing, for a button whose content is all
/// [`children`](ButtonChain::children), and is named too. The button's
/// [`named`](ButtonChain::named) overrides any of these.
///
/// ```
/// use eggshell_ui::owned::Face;
/// use eggshell_ui::{Builder, Icon};
///
/// fn view(b: &mut Builder<'_, '_, ()>, close: Icon) {
///     b.button("save", "Save").add();
///     b.button("close", (close.clone(), "Close")).add();
///     b.button("shut", close).named("Close").add();
///     b.button("card", Face::Empty("Open the card".into()))
///         .children(|_| {})
///         .add();
/// }
/// ```
///
/// TODO(prose): an icon alone has no name, and a button showing one is
/// reported as [`Unnamed`](crate::Problem::Unnamed) in debug builds unless
/// it is [`named`](ButtonChain::named).
#[derive(Clone, Debug)]
pub enum Face {
    /// Text, with no icon unless [`icon`](ButtonChain::icon) adds one.
    Label(String),
    /// An icon and no text, and the accessible name, empty for none.
    Icon(Icon, String),
    /// Nothing: the content is the button's [`children`](ButtonChain::children).
    /// Holds the accessible name.
    Empty(String),
}

impl Face {
    /// What assistive technology reads, unless the button is
    /// [`named`](ButtonChain::named).
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Label(name) | Self::Icon(_, name) | Self::Empty(name) => name,
        }
    }
}

impl From<&str> for Face {
    fn from(label: &str) -> Self {
        Self::Label(label.to_owned())
    }
}

impl From<String> for Face {
    fn from(label: String) -> Self {
        Self::Label(label)
    }
}

impl From<Icon> for Face {
    fn from(icon: Icon) -> Self {
        Self::Icon(icon, String::new())
    }
}

impl<S: Into<String>> From<(Icon, S)> for Face {
    fn from((icon, name): (Icon, S)) -> Self {
        Self::Icon(icon, name.into())
    }
}

/// A button drawn from the [`Style`](crate::Style): one of its variants, with
/// an icon, a label or both. Start one with [`Builder::button`] and finish it
/// with [`on_click`](Self::on_click), or [`add`](Self::add) for a button that
/// does nothing.
///
/// A button without a visible label has nothing for assistive technology to
/// read but its name: an icon-only or empty [`Face`] holds one, and
/// [`named`](Self::named) overrides it. In debug builds, a button whose name
/// is empty is reported as [`Unnamed`](crate::Problem::Unnamed).
///
/// TODO(prose)
///
/// ```
/// use eggshell_ui::Builder;
///
/// fn view(b: &mut Builder<'_, '_, ()>) {
///     b.dropdown_menu("crumbs", "···").named("The levels between").items(|_| {});
///     b.button("role", "Viewer").named("Role").add();
/// }
/// ```
#[must_use = "a button is not added until you call `.on_click(..)` or `.add()`"]
pub struct ButtonChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: String,
    name: String,
    toggle: Option<Mark>,
    role: Role,
    opts: ButtonOptions,
    on_response: OnResponse<A>,
    children: Option<Children<'a, 'f, A>>,
    action: Option<Action>,
    autofocus: Autofocus,
    closes: Closes,
    /// The key of an open popup's trigger, while its popup is open.
    expanded: Option<Key>,
}

/// What pressing a button does to the overlay it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Closes {
    Nothing,
    Overlay,
}

type Children<'a, 'f, A> = Box<dyn for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'a>;

/// The settings of a button or toggle that do not depend on its key or its
/// face, as a plain value that can be built once and applied to many buttons
/// with [`with_options`](ButtonChain::with_options).
///
/// A setting that was never set leaves the button's own (or the style's) in
/// place. When two sets of options meet in [`merge`](Self::merge), a setting
/// the second one has set wins, and classes merge as [`Tw::merge`] does.
///
/// ```
/// use eggshell_ui::owned::ButtonOptions;
/// use eggshell_ui::Builder;
/// use eggshell_ui::contract::Size;
///
/// fn verb() -> ButtonOptions {
///     ButtonOptions::new().size(Size::DEFAULT).tooltip("Does the thing")
/// }
///
/// fn view(b: &mut Builder<'_, '_, ()>) {
///     b.button("save", "Save").with_options(verb()).add();
///     b.button("send", "Send").with_options(verb()).tooltip("Sends it now").add();
/// }
/// ```
#[derive(Clone)]
pub struct ButtonOptions {
    icon: Option<Icon>,
    variant: Option<Variant>,
    size: Option<Size>,
    name: Option<String>,
    gate: Gate,
    enablement: Option<Enablement>,
    press: Option<Press>,
    classes: Tw,
}

impl Default for ButtonOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonOptions {
    /// Options that set nothing.
    pub fn new() -> Self {
        Self {
            icon: None,
            variant: None,
            size: None,
            name: None,
            gate: Gate::default(),
            enablement: None,
            press: None,
            classes: tw(),
        }
    }

    /// `self` with every setting `other` has set taking `other`'s value, and
    /// the classes of both merged, `other`'s winning ([`Tw::merge`]).
    pub fn merge(self, other: Self) -> Self {
        Self {
            icon: other.icon.or(self.icon),
            variant: other.variant.or(self.variant),
            size: other.size.or(self.size),
            name: other.name.or(self.name),
            gate: self.gate.merge(other.gate),
            enablement: other.enablement.or(self.enablement),
            press: other.press.or(self.press),
            classes: self.classes.merge(other.classes),
        }
    }

    /// Puts `icon` before the label, tinted by the button's text colour in
    /// each state and sized by the style's glyph size for the button's size.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draws the button as the style's variant `variant`. A toggle takes the
    /// variant's off and on looks from the style's toggle variants.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Draws the button at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Marks a button that opens a popup, which stays put when pressed.
    pub fn opens_popup(mut self) -> Self {
        self.press = Some(Press::Stays);
        self
    }

    /// Classes merged over the styled ones, such as the button's width or how
    /// it grows; the later class wins ([`Tw::merge`]). The style's hover and
    /// pressed looks still apply over them in their states: see
    /// [the module docs](crate::owned#classes-and-the-styles-state-looks).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// TODO(prose): the accessible name, in place of the face's, as
    /// `aria-label` is.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Disabled while the options refuse the button.
    pub(super) fn refusal(&self) -> Enablement {
        match self.gate.refused() {
            super::options::Refused::Yes => Enablement::Disabled,
            super::options::Refused::No => Enablement::Enabled,
        }
    }

    /// The name assistive technology reads for a button showing `face`.
    pub(crate) fn name_or<'s>(&'s self, face: &'s Face) -> &'s str {
        self.name.as_deref().unwrap_or_else(|| face.name())
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control is
    /// disabled, as an HTML `title` is. Rich text keeps egui's formatting. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.gate.tooltip(text);
        self
    }

    /// Whether the button takes input.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.enablement = Some(enablement);
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the last
    /// call wins.
    pub fn refused(mut self, reason: impl IntoTip) -> Self {
        self.gate.refuse(reason);
        self
    }
}

impl<'a, 'f, A: 'static> ButtonChain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K, face: Face) -> Self {
        let name = face.name().to_owned();
        let (label, icon) = match face {
            Face::Label(label) => (label, None),
            Face::Icon(icon, _) => (String::new(), Some(icon)),
            Face::Empty(_) => (String::new(), None),
        };
        Self {
            slot: b.slot(&key),
            b,
            label,
            name,
            toggle: None,
            role: Role::Button,
            opts: ButtonOptions {
                icon,
                ..ButtonOptions::new()
            },
            on_response: OnResponse::default(),
            children: None,
            action: None,
            autofocus: Autofocus::Never,
            closes: Closes::Nothing,
            expanded: None,
        }
    }

    /// Says whether the popup the button opens is open, for the sheet's
    /// `aria-expanded:` rules and its containers' `has-aria-expanded`.
    pub(super) fn expanded(mut self, open: bool) -> Self {
        self.expanded = super::expanded_key(open);
        self
    }

    /// Makes the button a toggle that shows `mark`.
    pub(super) fn marked(mut self, mark: Mark) -> Self {
        self.toggle = Some(mark);
        self
    }

    /// Asks for focus as `autofocus` says.
    pub(super) fn autofocus(mut self, autofocus: Autofocus) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Reports the button to assistive technology as `role`.
    pub(super) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Makes this the button Return presses: the default of the nearest
    /// overlay root around the focus, or of the window when it is not in an
    /// overlay. Return does nothing to a button that is disabled, and is left
    /// to a focused multi-line text field or button. If a root has several
    /// default buttons, the first declared is the one.
    pub fn default_action(mut self) -> Self {
        self.action = Some(Action::Default);
        self
    }

    /// Makes this the button Esc presses: the cancel button of the topmost
    /// overlay that takes Esc, or of the window when no overlay does. The
    /// button's handler runs in place of the overlay's
    /// [`on_dismiss`](crate::overlay::OverlayChain::on_dismiss) (a press on a
    /// modal's scrim still calls that). Esc does nothing to an overlay whose
    /// cancel button is disabled. If a root has several cancel buttons, the
    /// first declared is the one.
    pub fn cancel_action(mut self) -> Self {
        self.action = Some(Action::Cancel);
        self
    }

    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    pub fn focus_on_open(mut self) -> Self {
        self.autofocus = Autofocus::OnOverlayOpen;
        self
    }

    /// TODO(prose): takes focus on every frame this is set.
    pub fn focus_now(mut self) -> Self {
        self.autofocus = Autofocus::Now(Select::None);
        self
    }

    /// Makes pressing the button dismiss the overlay it is in, after its own
    /// handler has run: the overlay's
    /// [`on_dismiss`](crate::overlay::OverlayChain::on_dismiss) is called, as
    /// for a press outside or Esc, so the code that closes the overlay is
    /// written once. The overlay is dismissed whatever its
    /// [`DismissOn`](crate::overlay::DismissOn) says. A dialog's close
    /// button, or a cancel button with no handler of its own:
    ///
    /// ```
    /// use eggshell_ui::overlay::{Modal, Shown};
    /// use eggshell_ui::{Builder, tw};
    ///
    /// struct App { open: Shown }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    ///     b.overlay("dialog")
    ///         .modal(Modal::Center)
    ///         .shown(app.open)
    ///         .on_dismiss(|app: &mut App| app.open = Shown::Closed)
    ///         .children(|s| {
    ///             s.div(tw().p(6.0), |s| {
    ///                 s.button("cancel", "Cancel").cancel_action().dismisses().add();
    ///             });
    ///         });
    /// }
    /// ```
    ///
    /// Outside an overlay it does nothing more than its handler.
    pub fn dismisses(mut self) -> Self {
        self.closes = Closes::Overlay;
        self
    }

    /// Applies shared `options`: what they set overrides what the chain has
    /// so far, and what is set after them overrides them.
    pub fn with_options(mut self, options: ButtonOptions) -> Self {
        self.opts = self.opts.merge(options);
        self
    }

    /// Puts `icon` before the label, tinted by the button's text colour in
    /// each state and sized by the style's glyph size for the button's size.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.opts = self.opts.icon(icon);
        self
    }

    /// Draws the button as the style's variant `variant`. A toggle takes the
    /// variant's off and on looks from the style's toggle variants.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.opts = self.opts.variant(variant);
        self
    }

    /// Draws the button at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.opts = self.opts.size(size);
        self
    }

    /// Marks a button that opens a popup, which stays put when pressed.
    pub fn opens_popup(mut self) -> Self {
        self.opts = self.opts.opens_popup();
        self
    }

    /// Classes merged over the styled ones, such as the button's width or how
    /// it grows; the later class wins ([`Tw::merge`]). The style's hover and
    /// pressed looks still apply over them in their states: see
    /// [the module docs](crate::owned#classes-and-the-styles-state-looks).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.opts = self.opts.classes(classes);
        self
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control is
    /// disabled, as an HTML `title` is. Rich text keeps egui's formatting. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.opts = self.opts.tooltip(text);
        self
    }

    /// TODO(prose): the accessible name, in place of the face's, as
    /// `aria-label` is.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.opts = self.opts.named(name);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box, such as text and a [`kbd`](crate::Builder::kbd) chip.
    /// The content takes no input, and its text describes the control to
    /// assistive technology.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut crate::Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.opts.gate.tooltip_with(build);
        self
    }

    /// Runs `then` with the button's response each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Whether the button takes input.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.opts = self.opts.enablement(enablement);
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the last
    /// call wins. A [`command`](Self::command)'s button is disabled while its
    /// command is, whatever this says.
    pub fn refused(mut self, reason: impl IntoTip) -> Self {
        self.opts = self.opts.refused(reason);
        self
    }

    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<T>(self, value: Option<T>, then: impl FnOnce(Self, T) -> Self) -> Self {
        options::when_some(self, value, then)
    }

    /// Builds `body`'s nodes inside the button, after its icon and label, as
    /// React's `children` do. The button is `relative`, so an absolute child
    /// is placed against its border box; a flow child follows the face in the
    /// button's row. Children move with the press, are clipped wherever the
    /// button is (the button itself does not clip them, as in CSS), fade with
    /// it when disabled and take no clicks. Their keys are scoped
    /// under the button's key. Colours that depend on the button's state are
    /// the closure's to choose.
    ///
    /// `body` may borrow application data and locals, and runs when the
    /// button is added.
    ///
    /// ```
    /// use eggshell_ui::{Builder, Icon, tw};
    ///
    /// struct App { active: bool }
    ///
    /// fn tool(b: &mut Builder<'_, '_, App>, app: &App, icon: Icon) {
    ///     let active = app.active;
    ///     b.button("tool", (icon, "Tool"))
    ///         .children(|s| {
    ///             if active {
    ///                 s.div(
    ///                     tw().absolute().inset_x(0.0).mx_auto().bottom(1.0)
    ///                         .w_px(12.0).h_px(2.0).rounded_full().bg(egui::Color32::BLUE),
    ///                     |_| {},
    ///                 );
    ///             }
    ///         })
    ///         .add();
    /// }
    /// ```
    pub fn children(mut self, body: impl for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'a) -> Self {
        self.children = Some(Box::new(body));
        self
    }

    /// Adds the button as the command `id` names in the
    /// [`Commands`](crate::Commands) declared on the builder: it is disabled
    /// while the command is, and clicking it runs the command, as a menu item
    /// for it does (see [`Menu::command`](crate::owned::Menu::command)). The
    /// label is the button's own.
    ///
    /// If no live table declares `id`, the button is added disabled and is
    /// reported as [`UnknownCommand`](crate::Problem::UnknownCommand).
    #[track_caller]
    pub fn command<K: crate::Key>(mut self, id: K) {
        let Some(bound) = self.b.command(&id) else {
            crate::report::report(
                self.b.ctx(),
                crate::Problem::UnknownCommand,
                Some(self.slot.id),
                || {
                    format!(
                        "button {} for {id:?}: no live command table declares it; it is disabled",
                        self.slot.readable()
                    )
                },
            );
            self.opts.enablement = Some(Enablement::Disabled);
            self.add();
            return;
        };
        if bound.enablement == Enablement::Disabled {
            self.opts.enablement = Some(Enablement::Disabled);
        }
        let (command, shell) = (bound.command, self.b.shell());
        self.on_click(move |app| command.run(app, &shell));
    }

    /// Adds a button that does nothing when it is clicked: a placeholder, or
    /// one that is disabled.
    #[track_caller]
    pub fn add(self) {
        self.on_click(|_| {});
    }

    /// Adds the button. `on_click` runs when it is clicked or activated with
    /// Space or Enter.
    #[track_caller]
    pub fn on_click(self, on_click: impl FnOnce(&mut A) + 'static) {
        self.on_click_with(move |app, _| on_click(app));
    }

    /// Adds the button. `on_click` gets the application and the egui context,
    /// for an effect the context performs, such as copying text.
    #[track_caller]
    pub fn on_click_with(self, on_click: impl FnOnce(&mut A, &Context) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            name: face_name,
            toggle,
            role,
            opts:
                ButtonOptions {
                    icon,
                    variant,
                    size,
                    name,
                    gate,
                    enablement,
                    press,
                    classes: caller,
                },
            on_response,
            children,
            action,
            autofocus,
            closes,
            expanded,
        } = self;
        let name = name.unwrap_or(face_name);
        let (enablement, tooltip) =
            gate.resolve(&mut b, slot.id, enablement.unwrap_or(Enablement::Enabled));
        let size = b.size_of(size);
        let press = press.unwrap_or(Press::Shifts);
        let ctx = b.ctx();
        #[cfg(debug_assertions)]
        if name.is_empty() {
            crate::report::report(ctx, crate::Problem::Unnamed, Some(slot.id), || {
                format!(
                    "the button {} has no label and no name: assistive technology cannot read it",
                    slot.readable()
                )
            });
        }
        let style = b.style().clone();
        let disabled = enablement == Enablement::Disabled;
        let (classes, selected, glyph) = match toggle {
            None => {
                let mut keys = vec![
                    Key::Variant(variant.unwrap_or(Variant::DEFAULT)),
                    Key::Size(size),
                ];
                if disabled {
                    keys.push(Key::Aria(Aria::Disabled));
                }
                if press == Press::Stays {
                    keys.push(Key::Aria(Aria::HasPopup));
                }
                keys.extend(expanded);
                let own = tw().relative().sense_click();
                let own = if disabled { own.inert() } else { own };
                (
                    b.part(ButtonPart::Root, &keys, own.merge(caller)),
                    None,
                    None,
                )
            }
            Some(mark) => {
                let toggles = &style.toggle;
                let looks = match mark {
                    Mark::Off => &toggles.off,
                    Mark::On => &toggles.on,
                };
                let look = looks
                    .lookup(ctx, variant.unwrap_or(looks.default_key()))
                    .bordered(toggles.border_width);
                let metrics = *toggles.sizes.lookup(ctx, size);
                let mut classes =
                    look.classes(&style, metrics.radius, enablement, toggles.transition);
                if enablement == Enablement::Enabled && press == Press::Shifts {
                    classes = classes.active(|p| p.translate_y(toggles.press_shift));
                }
                let classes = centered(classes)
                    .relative()
                    .h_px(metrics.height)
                    .min_w_px(metrics.min_width)
                    .px_px(metrics.padding_x)
                    .gap_px(metrics.gap)
                    .role(style.role(&toggles.text).clone())
                    .whitespace_nowrap();
                (
                    enablement.fade(classes, &style).merge(caller),
                    Some(mark),
                    Some(metrics.glyph),
                )
            }
        };
        let labelled = !label.is_empty();
        let node = b.div_id_with(&slot, classes, |b| {
            if let Some(icon) = icon {
                let classes = match glyph {
                    Some(glyph) => tw().size_px(glyph).shrink_0(),
                    None if labelled => tw().attr("icon", "inline-start"),
                    None => tw(),
                };
                b.glyph(classes, icon);
            }
            if labelled {
                b.text(tw(), label);
            }
            if let Some(children) = children {
                children(b);
            }
        });
        let node = node.announces(role).open_focus(autofocus.open_focus());
        let node = match action {
            Some(action) => node.action(action),
            None => node,
        };
        node.draw(move |app, ui| {
            let response = described(ui, role, selected, &name, &tooltip);
            autofocus.apply(ui.ctx(), &response);
            if response.clicked() || crate::nav::take_activation(ui, response.id) {
                on_click(app, ui.ctx());
                if closes == Closes::Overlay
                    && let Some(shell) = crate::shell::shell_or_report(ui, "a dismissing button")
                {
                    crate::actions::note_closer(ui.ctx(), &shell.kept, response.id);
                }
            }
            on_response.run(app, &response);
        });
    }
}
