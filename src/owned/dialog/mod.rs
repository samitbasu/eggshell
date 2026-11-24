//! TODO(prose): the dialog surface, its header and footer, the sheet and the
//! popover. Each ends a core overlay chain: placement, the scrim, dismissal,
//! focus, motion and the Esc and Return rules stay core's.
//!
//! ```
//! use eggshell_ui::overlay::{Modal, Shown};
//! use eggshell_ui::Builder;
//! use eggshell_ui_extras::dialog::{Dialog, DialogExt};
//!
//! struct App { edit: Shown, name: String }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.overlay("edit")
//!         .modal(Modal::Center)
//!         .shown(app.edit)
//!         .on_dismiss(|app: &mut App| app.edit = Shown::Closed)
//!         .dialog(
//!             Dialog::new("Edit profile").description("Make changes to your profile here."),
//!             |d| {
//!                 d.body(|s| {
//!                     s.text(eggshell_ui::tw(), "Fields go here");
//!                 });
//!                 d.footer(|f| {
//!                     f.cancel("Cancel", |button| button.add())
//!                         .default("Save changes", |button| {
//!                             button.on_click(|app: &mut App| app.edit = Shown::Closed)
//!                         })
//!                 });
//!             },
//!         );
//! }
//! ```

mod parts;
mod style;

pub use parts::{AlertDialogPart, DialogPart, PopoverPart, SheetPart};
pub use style::{DialogStyle, Surface, shadow_lg};

use eggshell_ui::overlay::{DismissOn, Floating, Modal, Modally, OverlayChain, Side};
use eggshell_ui::owned::{ButtonChain, Face};
use eggshell_ui::sheet::{Defaults, Key as SheetKey};
use eggshell_ui::{Builder, Icon, Scope, TextSpec, Tw, icons, tw};
use egui::Role;

/// The keys of a dialog's parts, under the overlay's key: tests and
/// [`focus_on_open`](eggshell_ui::NodeHandle::focus_on_open) name them.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DialogKey {
    /// The box: the dialog, the sheet or the popover.
    Box,
    /// The title and the description.
    Header,
    /// What [`DialogContent::body`] builds.
    Body,
    /// The footer's buttons.
    Footer,
    /// The footer's leading button.
    Leading,
    /// The footer's cancel button.
    Cancel,
    /// The footer's default button.
    Default,
    /// The close button (×).
    Close,
}

/// Whether a dialog has a close button (×).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseButton {
    Shown,
    Hidden,
}

/// TODO(prose): what a dialog says of itself: its title, which names the
/// overlay, an optional description, which describes it, and classes merged
/// over the style's box.
#[derive(Clone, Debug)]
#[must_use]
pub struct Dialog {
    title: String,
    description: Option<String>,
    classes: Tw,
    close: CloseButton,
}

impl Dialog {
    /// A dialog titled `title`, which is also its accessible name.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            classes: tw(),
            close: CloseButton::Shown,
        }
    }

    /// The sentence under the title, which is also the overlay's accessible
    /// description.
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// Classes merged over the style's box, such as its width.
    pub fn classes(self, classes: Tw) -> Self {
        Self {
            classes: self.classes.merge(classes),
            ..self
        }
    }

    /// Without the close button (×). An alert dialog never has one.
    pub fn without_close(self) -> Self {
        Self {
            close: CloseButton::Hidden,
            ..self
        }
    }
}

type Body<'c, 'f, A> = Box<dyn for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'c>;
type Then<'c, 'f, A> = Box<dyn for<'s> FnOnce(ButtonChain<'s, 'f, A>) + 'c>;

/// What the dialog holds besides its header: given to the closure of
/// [`DialogExt::dialog`], and built in a fixed order once it returns.
pub struct DialogContent<'c, 'f, A> {
    body: Option<Body<'c, 'f, A>>,
    footer: Option<Footer<'c, 'f, A>>,
}

impl<'c, 'f, A> DialogContent<'c, 'f, A> {
    /// The dialog's body: an ordinary builder body between the header and
    /// the footer. A second call replaces the first.
    pub fn body(&mut self, body: impl for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'c) {
        self.body = Some(Box::new(body));
    }

    /// The dialog's footer buttons. A second call replaces the first.
    pub fn footer(&mut self, buttons: impl FnOnce(Footer<'c, 'f, A>) -> Footer<'c, 'f, A>) {
        self.footer = Some(buttons(Footer::empty()));
    }
}

struct Button<'c, 'f, A> {
    face: Face,
    then: Then<'c, 'f, A>,
}

impl<'c, 'f, A> Button<'c, 'f, A> {
    fn new(
        face: impl Into<Face>,
        then: impl for<'s> FnOnce(ButtonChain<'s, 'f, A>) + 'c,
    ) -> Option<Self> {
        Some(Self {
            face: face.into(),
            then: Box::new(then),
        })
    }
}

/// TODO(prose): a dialog's footer: a leading button, a cancel button and a
/// default button, each optional. They are built in that order on every
/// platform, the default last, so Tab order follows what is on screen.
///
/// Each closure gets the core button with the style's variant and its action
/// already applied, and adds it (`.add()`, `.on_click(..)`). Setting a slot
/// again replaces it.
#[must_use = "the footer is built from what its closure returns"]
pub struct Footer<'c, 'f, A> {
    leading: Option<Button<'c, 'f, A>>,
    cancel: Option<Button<'c, 'f, A>>,
    default: Option<Button<'c, 'f, A>>,
}

impl<'c, 'f, A> Footer<'c, 'f, A> {
    fn empty() -> Self {
        Self {
            leading: None,
            cancel: None,
            default: None,
        }
    }

    /// The button Esc presses: the style's cancel variant,
    /// [`cancel_action`](ButtonChain::cancel_action) and
    /// [`dismisses`](ButtonChain::dismisses). One added with no handler of
    /// its own runs the overlay's `on_dismiss`.
    pub fn cancel(
        self,
        face: impl Into<Face>,
        then: impl for<'s> FnOnce(ButtonChain<'s, 'f, A>) + 'c,
    ) -> Self {
        Self {
            cancel: Button::new(face, then),
            ..self
        }
    }

    /// The button Return presses: the style's default variant and
    /// [`default_action`](ButtonChain::default_action). Always last.
    pub fn default(
        self,
        face: impl Into<Face>,
        then: impl for<'s> FnOnce(ButtonChain<'s, 'f, A>) + 'c,
    ) -> Self {
        Self {
            default: Button::new(face, then),
            ..self
        }
    }

    /// A button at the far start, with neither action: "Delete", or "Don't
    /// save".
    pub fn leading(
        self,
        face: impl Into<Face>,
        then: impl for<'s> FnOnce(ButtonChain<'s, 'f, A>) + 'c,
    ) -> Self {
        Self {
            leading: Button::new(face, then),
            ..self
        }
    }
}

/// Which kind of dialog is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Dialog,
    Alert,
}

/// The dialog surface, ending a modal overlay chain.
pub trait DialogExt<'c, 'f, A> {
    /// TODO(prose): builds the overlay as a dialog ([`Modal::Center`]) or a
    /// sheet ([`Modal::Edge`]): the style's box, a header with the title and
    /// description, `content`'s body and footer, and a close button (×)
    /// built last, so that first focus never lands on it. Sets
    /// [`Role::Dialog`], the name and the description.
    fn dialog(self, dialog: Dialog, content: impl FnOnce(&mut DialogContent<'c, 'f, A>));

    /// TODO(prose): a confirmation that must be answered: as
    /// [`dialog`](Self::dialog), with [`Role::AlertDialog`], no close
    /// button, and [`DismissOn::EscOnly`], which overrides an earlier
    /// `dismiss_on`.
    fn alert_dialog(self, dialog: Dialog, content: impl FnOnce(&mut DialogContent<'c, 'f, A>));
}

impl<'c, 'f, A: 'static, M> DialogExt<'c, 'f, A> for OverlayChain<'_, 'f, A, Modally, M> {
    fn dialog(self, dialog: Dialog, content: impl FnOnce(&mut DialogContent<'c, 'f, A>)) {
        surface(self.role(Role::Dialog), Kind::Dialog, dialog, content);
    }

    fn alert_dialog(self, dialog: Dialog, content: impl FnOnce(&mut DialogContent<'c, 'f, A>)) {
        let chain = self.role(Role::AlertDialog).dismiss_on(DismissOn::EscOnly);
        surface(chain, Kind::Alert, dialog.without_close(), content);
    }
}

fn surface<'c, 'f, A: 'static, M>(
    chain: OverlayChain<'_, 'f, A, Modally, M>,
    kind: Kind,
    dialog: Dialog,
    content: impl FnOnce(&mut DialogContent<'c, 'f, A>),
) {
    let modal = chain.presentation();
    let chain = chain.named(dialog.title.clone());
    let chain = match &dialog.description {
        Some(description) => chain.description(description.clone()),
        None => chain,
    };
    let mut parts = DialogContent {
        body: None,
        footer: None,
    };
    content(&mut parts);
    chain.children(move |s| {
        let style = s.style().extension::<DialogStyle>();
        let shape = Shape::of(modal, kind);
        let classes = shape.part(
            s,
            Slot::Content,
            shape.box_classes(&style),
            tw().relative().merge(dialog.classes.clone()),
        );
        s.div_id(DialogKey::Box, classes, |s| {
            header(s, &style, shape, &dialog);
            if let Some(body) = parts.body {
                s.div_id(DialogKey::Body, shape.body_classes(), body);
            }
            if let Some(footer) = parts.footer {
                buttons(s, &style, shape, footer);
            }
            if dialog.close == CloseButton::Shown {
                // shadcn's close is the button itself; here a box in the
                // close's place holds it, as the button says `relative`
                // over what its caller gives it.
                let place = shape.part(
                    s,
                    Slot::Close,
                    tw().absolute()
                        .top(style.close_inset)
                        .right(style.close_inset),
                    tw(),
                );
                s.div(place, |s| {
                    s.button(DialogKey::Close, Icon::new(icons::CLOSE))
                        .named("Close")
                        .variant(style.close)
                        .size(style.close_size)
                        .dismisses()
                        .add();
                });
            }
        });
    });
}

/// The shape of the box: a centred dialog, or a sheet docked to a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Centered(Kind),
    Sheet(Side),
}

/// Tailwind spacing units, from shadcn's `dialog.tsx` and `sheet.tsx`.
mod units {
    pub(super) const DIALOG_PADDING: f32 = 6.0;
    pub(super) const DIALOG_GAP: f32 = 4.0;
    pub(super) const DIALOG_MAX_W: f32 = 512.0;
    pub(super) const SHEET_PADDING: f32 = 4.0;
    pub(super) const SHEET_MAX_W: f32 = 384.0;
    pub(super) const HEADER_GAP: f32 = 2.0;
    pub(super) const SHEET_HEADER_GAP: f32 = 1.5;
    pub(super) const FOOTER_GAP: f32 = 2.0;
    pub(super) const POPOVER_W: f32 = 288.0;
    pub(super) const POPOVER_PADDING: f32 = 4.0;
}

impl Shape {
    fn of(modal: Modal, kind: Kind) -> Self {
        match modal {
            Modal::Center => Self::Centered(kind),
            Modal::Edge(side) => Self::Sheet(side),
        }
    }

    fn box_classes(self, style: &DialogStyle) -> Tw {
        let painted = style
            .surface
            .classes()
            .relative()
            .flex()
            .flex_col()
            .gap(units::DIALOG_GAP);
        match self {
            Self::Centered(_) => painted
                .w_full()
                .max_w_px(units::DIALOG_MAX_W)
                .p(units::DIALOG_PADDING),
            Self::Sheet(side) => {
                let edge = painted.rounded_px(0.0).border(0.0);
                let width = style.surface.border_width;
                match side {
                    Side::Left => edge
                        .border_r(width)
                        .h_full()
                        .w_frac(3.0, 4.0)
                        .max_w_px(units::SHEET_MAX_W),
                    Side::Right => edge
                        .border_l(width)
                        .h_full()
                        .w_frac(3.0, 4.0)
                        .max_w_px(units::SHEET_MAX_W),
                    Side::Top => edge.border_b(width).w_full(),
                    Side::Bottom => edge.border_t(width).w_full(),
                }
            }
        }
    }

    fn header_classes(self) -> Tw {
        match self {
            Self::Centered(_) => tw().flex().flex_col().gap(units::HEADER_GAP),
            Self::Sheet(_) => tw()
                .flex()
                .flex_col()
                .gap(units::SHEET_HEADER_GAP)
                .p(units::SHEET_PADDING),
        }
    }

    fn title_role(self, style: &DialogStyle) -> TextSpec {
        match self {
            Self::Centered(_) => style.title.clone(),
            Self::Sheet(_) => TextSpec {
                size: 16.0,
                line_height: 24.0,
                ..style.title.clone()
            },
        }
    }

    fn body_classes(self) -> Tw {
        let body = tw().flex().flex_col().gap(units::DIALOG_GAP);
        match self {
            Self::Centered(_) => body,
            Self::Sheet(_) => body.flex_1().min_h_0().px(units::SHEET_PADDING),
        }
    }

    fn footer_classes(self) -> Tw {
        match self {
            Self::Centered(_) => tw()
                .flex()
                .items_center()
                .justify_end()
                .gap(units::FOOTER_GAP),
            Self::Sheet(_) => tw()
                .mt_auto()
                .flex()
                .flex_col()
                .gap(units::FOOTER_GAP)
                .p(units::SHEET_PADDING),
        }
    }
}

/// A part of the surface, as each of shadcn's components names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Content,
    Close,
    Header,
    Footer,
    Title,
    Description,
}

impl Shape {
    /// The classes of `slot` as the shadcn component that draws this shape
    /// names it, with `defaults`, the surface's own look, under the sheet,
    /// and `own` over it: a centred dialog is shadcn's dialog, an alert
    /// dialog its alert dialog, and an edge's its sheet, which says its side.
    fn part<A>(self, b: &mut Builder<'_, '_, A>, slot: Slot, defaults: Tw, own: Tw) -> Tw {
        let defaults = Defaults(defaults);
        match self {
            Self::Centered(Kind::Alert) if slot != Slot::Close => {
                let part = match slot {
                    Slot::Header => AlertDialogPart::Header,
                    Slot::Footer => AlertDialogPart::Footer,
                    Slot::Title => AlertDialogPart::Title,
                    Slot::Description => AlertDialogPart::Description,
                    Slot::Content | Slot::Close => AlertDialogPart::Content,
                };
                b.part_over(part, &[], defaults, own)
            }
            Self::Centered(_) => {
                let part = match slot {
                    Slot::Content => DialogPart::Content,
                    Slot::Close => DialogPart::Close,
                    Slot::Header => DialogPart::Header,
                    Slot::Footer => DialogPart::Footer,
                    Slot::Title => DialogPart::Title,
                    Slot::Description => DialogPart::Description,
                };
                b.part_over(part, &[], defaults, own)
            }
            Self::Sheet(side) => {
                let part = match slot {
                    Slot::Content => SheetPart::Content,
                    Slot::Close => SheetPart::Close,
                    Slot::Header => SheetPart::Header,
                    Slot::Footer => SheetPart::Footer,
                    Slot::Title => SheetPart::Title,
                    Slot::Description => SheetPart::Description,
                };
                let side = match side {
                    Side::Top => "top",
                    Side::Bottom => "bottom",
                    Side::Left => "left",
                    Side::Right => "right",
                };
                b.part_over(part, &[SheetKey::Attr("side", side)], defaults, own)
            }
        }
    }
}

fn header<A>(s: &mut Scope<'_, '_, A>, style: &DialogStyle, shape: Shape, dialog: &Dialog) {
    let classes = shape.part(s, Slot::Header, shape.header_classes(), tw());
    let title = shape.part(s, Slot::Title, tw().role(shape.title_role(style)), tw());
    let description = shape.part(
        s,
        Slot::Description,
        tw().role(style.description.clone())
            .text(style.description_color),
        tw(),
    );
    s.div_id(DialogKey::Header, classes, |s| {
        s.text(title, dialog.title.as_str());
        if let Some(text) = &dialog.description {
            s.text(description, text.as_str());
        }
    });
}

fn buttons<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    style: &DialogStyle,
    shape: Shape,
    footer: Footer<'_, 'f, A>,
) {
    let Footer {
        leading,
        cancel,
        default,
    } = footer;
    let classes = shape.part(s, Slot::Footer, shape.footer_classes(), tw());
    s.div_id(DialogKey::Footer, classes, |s| {
        if let Some(Button { face, then }) = leading {
            let push = match shape {
                Shape::Centered(_) => tw().mr_auto(),
                Shape::Sheet(_) => tw(),
            };
            then(s.button(DialogKey::Leading, face).classes(push));
        }
        if let Some(Button { face, then }) = cancel {
            then(
                s.button(DialogKey::Cancel, face)
                    .variant(style.cancel)
                    .cancel_action()
                    .dismisses(),
            );
        }
        if let Some(Button { face, then }) = default {
            then(
                s.button(DialogKey::Default, face)
                    .variant(style.default)
                    .default_action(),
            );
        }
    });
}

/// The popover surface, ending a floating overlay chain.
pub trait PopoverExt<'f, A> {
    /// TODO(prose): builds the overlay's content in the style's popover box
    /// (shadcn's `PopoverContent`). Sets no role: name the overlay by what it
    /// holds.
    fn popover(self, body: impl FnOnce(&mut Scope<'_, 'f, A>));
}

impl<'f, A, M> PopoverExt<'f, A> for OverlayChain<'_, 'f, A, Floating, M> {
    fn popover(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        self.children(|s| {
            let style = s.style().extension::<DialogStyle>();
            let own = style
                .popover
                .classes()
                .gap(units::HEADER_GAP)
                .w_px(units::POPOVER_W)
                .p(units::POPOVER_PADDING);
            let classes = s.part_over(
                PopoverPart::Content,
                &[],
                Defaults(own),
                tw().flex().flex_col(),
            );
            s.div_id(DialogKey::Box, classes, body);
        });
    }
}
