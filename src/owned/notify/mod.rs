//! TODO(prose)
//!
//! ```
//! use core::time::Duration;
//! use eggshell_ui::{Builder, Distance};
//! use eggshell_ui::owned::notify::{Lifetime, Notice, NoticeId, Ext, ScreenAnchor};
//!
//! #[derive(Clone, Copy)]
//! enum Act {
//!     CopyDetails,
//! }
//!
//! struct App {
//!     toast: Option<Notice>,
//!     notices: Vec<Notice<Act>>,
//!     copied: Option<NoticeId>,
//! }
//!
//! fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
//!     b.notifications("toast", app.toast.as_slice())
//!         .at(ScreenAnchor::BottomCenter, Distance::new(28.0))
//!         .lifetime(Lifetime::Dwell(Duration::from_millis(2400)))
//!         .on_dismiss(|app: &mut App, _, _| app.toast = None)
//!         .add();
//!     b.notifications("notices", &app.notices)
//!         .at(ScreenAnchor::TopRight, Distance::new(16.0))
//!         .on_action(|app: &mut App, id, Act::CopyDetails| app.copied = Some(id))
//!         .on_dismiss(|app: &mut App, id, _| app.notices.retain(|n| n.id != id))
//!         .add();
//! }
//!
//! let mut app = App { toast: None, notices: Vec::new(), copied: None };
//! app.toast = Some(Notice::new(1, "Copied"));
//! app.notices.push(Notice::new(2, "Export failed.").action("Copy details", Act::CopyDetails));
//! ```

mod dwell;
mod look;
mod style;

use core::time::Duration;
use std::collections::HashMap;
use std::rc::Rc;

use crate::overlay::{Align, Anchor, Band, DismissOn, Focus, Outside, Placement, Side};
use crate::{Boundary, Builder, Distance, Icon, Key, NodeUi as _, Scope, Sides, icons, tw};
use egui::accesskit::Live;
use egui::{Context, Id, Modifiers, Role};

use dwell::{Countdown, Dwell, Hold};
use look::Look;
pub use style::NotifyStyle;

/// TODO(prose): a notice's anatomy for the style's sheet, as shadcn's
/// `sonner.tsx` names it. shadcn's classes style over sonner's own
/// stylesheet, which is the notice's look here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotifyPart {
    /// TODO(prose): a notice's card.
    Toast,
}

impl crate::sheet::Part for NotifyPart {
    const SOURCE: &'static str = "sonner.tsx";

    fn slot(self) -> &'static str {
        "toast"
    }
}

/// TODO(prose): the application's name for a notice. A notice with a new id
/// is a new notice: its dwell starts afresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoticeId(u64);

impl NoticeId {
    /// TODO(prose)
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

impl From<u64> for NoticeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<NoticeId> for u64 {
    fn from(id: NoticeId) -> u64 {
        id.0
    }
}

/// TODO(prose): the action of a notice that has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoAction {}

/// TODO(prose): one notification, held by the application: an `Option` of
/// them is a slot a new one replaces, a `Vec` a stack.
#[derive(Clone, Debug, PartialEq)]
pub struct Notice<Act = NoAction> {
    /// TODO(prose)
    pub id: NoticeId,
    /// TODO(prose)
    pub text: String,
    /// TODO(prose): shown as buttons, in this order.
    pub actions: Vec<NoticeAction<Act>>,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
pub struct NoticeAction<Act> {
    /// TODO(prose): the button's label.
    pub label: String,
    /// TODO(prose): what [`on_action`](Notifications::on_action) is given.
    pub act: Act,
}

impl<Act> Notice<Act> {
    /// TODO(prose)
    pub fn new(id: impl Into<NoticeId>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            actions: Vec::new(),
        }
    }

    /// TODO(prose)
    #[must_use]
    pub fn action(mut self, label: impl Into<String>, act: Act) -> Self {
        self.actions.push(NoticeAction {
            label: label.into(),
            act,
        });
        self
    }
}

/// TODO(prose): how long the notices of a stack stay up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// TODO(prose): each for this long on screen, on egui's clock, not
    /// counting the time the stack is hovered or focused or the window is
    /// in the background.
    Dwell(Duration),
    /// TODO(prose): until the application removes them.
    UntilDismissed,
}

/// TODO(prose): why [`on_dismiss`](Notifications::on_dismiss) ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dismissed {
    /// TODO(prose): its dwell ran out.
    Expired,
    /// TODO(prose): its × was pressed.
    Closed,
    /// TODO(prose): Esc was pressed with focus inside it.
    Escape,
}

/// TODO(prose): the corner or edge of the window a stack sits at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenAnchor {
    /// TODO(prose)
    TopLeft,
    /// TODO(prose)
    TopCenter,
    /// TODO(prose)
    TopRight,
    /// TODO(prose)
    BottomLeft,
    /// TODO(prose)
    BottomCenter,
    /// TODO(prose)
    BottomRight,
}

impl ScreenAnchor {
    /// TODO(prose): core's placement of a stack at this anchor, `inset` from
    /// every edge of the window.
    pub fn placement(self, inset: Distance) -> Placement {
        let align = match self {
            Self::TopLeft | Self::BottomLeft => Align::Start,
            Self::TopCenter | Self::BottomCenter => Align::Center,
            Self::TopRight | Self::BottomRight => Align::End,
        };
        Placement::edge(self.side())
            .align(align)
            .boundary(Boundary::Window.inset(Sides::all(inset)))
    }

    fn side(self) -> Side {
        match self {
            Self::TopLeft | Self::TopCenter | Self::TopRight => Side::Top,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight => Side::Bottom,
        }
    }
}

/// Whether a transient toast shows its ×.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseButton {
    Hidden,
    Shown,
}

/// TODO(prose)
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): the stack `key` showing `items`, newest (last) nearest
    /// its anchor. Nothing is shown while `items` is empty.
    fn notifications<'s, K: Key + Clone, Act: Clone + 'static>(
        &'s mut self,
        key: K,
        items: &'s [Notice<Act>],
    ) -> Notifications<'s, 'b, 'f, A, K, Act>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn notifications<'s, K: Key + Clone, Act: Clone + 'static>(
        &'s mut self,
        key: K,
        items: &'s [Notice<Act>],
    ) -> Notifications<'s, 'b, 'f, A, K, Act> {
        Notifications {
            b: self,
            key,
            items,
            anchor: ScreenAnchor::BottomRight,
            inset: None,
            lifetime: Lifetime::UntilDismissed,
            close: CloseButton::Hidden,
            name: "Notifications".to_owned(),
            on_action: None,
            on_dismiss: None,
        }
    }
}

type OnAction<A, Act> = Rc<dyn Fn(&mut A, NoticeId, Act)>;
type OnDismiss<A> = Rc<dyn Fn(&mut A, NoticeId, Dismissed)>;

/// TODO(prose): a stack of notices, drawn above the page and its dialogs,
/// and announced politely as it changes.
#[must_use = "the notifications are not added until you call `.add()`"]
pub struct Notifications<'s, 'b, 'f, A, K, Act> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    items: &'s [Notice<Act>],
    anchor: ScreenAnchor,
    inset: Option<Distance>,
    lifetime: Lifetime,
    close: CloseButton,
    name: String,
    on_action: Option<OnAction<A, Act>>,
    on_dismiss: Option<OnDismiss<A>>,
}

impl<'s, 'b, 'f, A: 'static, K: Key + Clone, Act: Clone + 'static>
    Notifications<'s, 'b, 'f, A, K, Act>
{
    /// TODO(prose): where the stack sits, `inset` from the window's edges.
    /// [`ScreenAnchor::BottomRight`], the style's `inset` in, unless set.
    pub fn at(self, anchor: ScreenAnchor, inset: Distance) -> Self {
        Self {
            anchor,
            inset: Some(inset),
            ..self
        }
    }

    /// TODO(prose): [`Lifetime::UntilDismissed`] unless set.
    pub fn lifetime(self, lifetime: Lifetime) -> Self {
        Self { lifetime, ..self }
    }

    /// TODO(prose): gives each notice of a [`Lifetime::Dwell`] stack a ×,
    /// which the notices of an [`UntilDismissed`](Lifetime::UntilDismissed)
    /// stack always have.
    pub fn closable(self) -> Self {
        Self {
            close: CloseButton::Shown,
            ..self
        }
    }

    /// TODO(prose): the stack's accessible name; "Notifications" unless set.
    pub fn named(self, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    /// TODO(prose): runs when an action's button is pressed. The notice
    /// stays up unless the handler removes it.
    pub fn on_action(self, on_action: impl Fn(&mut A, NoticeId, Act) + 'static) -> Self {
        Self {
            on_action: Some(Rc::new(on_action)),
            ..self
        }
    }

    /// TODO(prose): runs when a notice expires, its × is pressed, or Esc is
    /// pressed with focus inside it: remove it. A notice no handler removes
    /// stays up, and expires only once.
    pub fn on_dismiss(self, on_dismiss: impl Fn(&mut A, NoticeId, Dismissed) + 'static) -> Self {
        Self {
            on_dismiss: Some(Rc::new(on_dismiss)),
            ..self
        }
    }

    /// TODO(prose)
    #[track_caller]
    pub fn add(self) {
        let Self {
            b,
            key,
            items,
            anchor,
            inset,
            lifetime,
            close,
            name,
            on_action,
            on_dismiss,
        } = self;
        let dwells = b.id_of(key.clone()).with("eggshell-ui-notify-dwells");
        if items.is_empty() {
            b.ctx().data_mut(|data| data.remove::<Dwells>(dwells));
            return;
        }
        let close = match lifetime {
            Lifetime::UntilDismissed => CloseButton::Shown,
            Lifetime::Dwell(_) => close,
        };
        let handlers = Handlers {
            on_action,
            on_dismiss,
        };
        let look = Look::new(std::sync::Arc::new(b.style().notify.clone()));
        let inset = inset.unwrap_or(look.style.inset);
        let items = items
            .get(items.len().saturating_sub(look.style.visible)..)
            .unwrap_or(items);
        let ids: Vec<NoticeId> = items.iter().map(|notice| notice.id).collect();
        let nearest_first = anchor.side() == Side::Top;
        b.overlay(key)
            .anchor(Anchor::Screen)
            .placement(anchor.placement(inset))
            .band(Band::Popup)
            .focus(Focus::None)
            .dismiss_on(DismissOn::Never)
            .outside(Outside::PassThrough)
            .children(|s| {
                let stack = s
                    .div_id("stack", look.stack(), |s| {
                        let mut cards =
                            |notice: &Notice<Act>| card(s, notice, &look, close, &handlers);
                        if nearest_first {
                            items.iter().rev().for_each(&mut cards);
                        } else {
                            items.iter().for_each(&mut cards);
                        }
                    })
                    .role(Role::Status)
                    .named(name)
                    .live(Live::Polite);
                if let Lifetime::Dwell(length) = lifetime {
                    let on_dismiss = handlers.on_dismiss.clone();
                    stack.draw(move |app, ui| {
                        let hold = if ui.node_response().contains_pointer()
                            || ui.has_focus_within()
                            || !ui.input(|input| input.focused)
                        {
                            Hold::Held
                        } else {
                            Hold::Running
                        };
                        let expired = tick(ui.ctx(), dwells, &ids, length, hold);
                        if let Some(on_dismiss) = on_dismiss {
                            for id in expired {
                                on_dismiss(app, id, Dismissed::Expired);
                            }
                        }
                    });
                }
            });
    }
}

struct Handlers<A, Act> {
    on_action: Option<OnAction<A, Act>>,
    on_dismiss: Option<OnDismiss<A>>,
}

/// The dwells of one stack's visible notices.
#[derive(Clone, Default)]
struct Dwells(HashMap<NoticeId, Dwell>);

/// Moves the dwells of `ids` to egui's clock and books a repaint for the
/// next expiry. Returns the notices that expire now.
fn tick(
    ctx: &Context,
    dwells: Id,
    ids: &[NoticeId],
    length: Duration,
    hold: Hold,
) -> Vec<NoticeId> {
    let now = Duration::try_from_secs_f64(ctx.input(|input| input.time).max(0.0))
        .unwrap_or(Duration::MAX);
    let mut expired = Vec::new();
    let mut next: Option<Duration> = None;
    ctx.data_mut(|data| {
        let Dwells(dwells) = data.get_temp_mut_or_default::<Dwells>(dwells);
        dwells.retain(|id, _| ids.contains(id));
        for &id in ids {
            let dwell = dwells.entry(id).or_insert_with(|| Dwell::new(now));
            match dwell.tick(now, length, hold) {
                Countdown::Left(left) => next = Some(next.map_or(left, |next| next.min(left))),
                Countdown::Expires => expired.push(id),
                Countdown::Over => {}
            }
        }
    });
    if let (Hold::Running, Some(next)) = (hold, next) {
        ctx.request_repaint_after(next);
    }
    expired
}

fn card<A: 'static, Act: Clone + 'static>(
    s: &mut Scope<'_, '_, A>,
    notice: &Notice<Act>,
    look: &Look,
    close: CloseButton,
    handlers: &Handlers<A, Act>,
) {
    let id = notice.id;
    let classes = s.part(NotifyPart::Toast, &[], tw());
    let card = s.div_id(id, classes, |s| {
        s.text(look.sentence(), notice.text.clone());
        for (at, action) in notice.actions.iter().enumerate() {
            let (act, on_action) = (action.act.clone(), handlers.on_action.clone());
            let label = action.label.clone();
            s.div_id(("action", at), look.action(), |s| {
                s.text(tw(), label.clone());
            })
            .role(Role::Button)
            .named(label)
            .on_click(move |app| {
                if let Some(on_action) = on_action {
                    on_action(app, id, act);
                }
            });
        }
        if close == CloseButton::Shown {
            let on_dismiss = handlers.on_dismiss.clone();
            let name = format!("Dismiss: {}", notice.text);
            s.div_id("close", look.close(), |s| {
                s.glyph(look.close_glyph(), Icon::new(icons::CLOSE));
            })
            .role(Role::Button)
            .named(name)
            .on_click(move |app| {
                if let Some(on_dismiss) = on_dismiss {
                    on_dismiss(app, id, Dismissed::Closed);
                }
            });
        }
    });
    if let Some(on_dismiss) = handlers.on_dismiss.clone() {
        card.draw(move |app, ui| {
            if focus_was_inside(ui)
                && ui.input_mut(|input| input.consume_key(Modifiers::NONE, egui::Key::Escape))
            {
                on_dismiss(app, id, Dismissed::Escape);
            }
        });
    }
}

/// Marks a notice that held focus at the end of its last draw.
#[derive(Clone, Copy, Default)]
struct FocusInside;

/// Whether focus is inside the node now or was in its last draw: egui drops
/// the focus at the start of the pass an Esc is pressed in.
fn focus_was_inside(ui: &egui::Ui) -> bool {
    let Some(node) = ui.node_id() else {
        return false;
    };
    let mark = node.with("eggshell-ui-notify-focus");
    let now = ui.has_focus_within();
    let before = ui.data_mut(|data| {
        let before = data.remove_temp::<FocusInside>(mark).is_some();
        if now {
            data.insert_temp(mark, FocusInside);
        }
        before
    });
    now || before
}

/// Writes the notice's recorded exception: its card, sonner's toast, which
/// the generated look does not carry, from `notify`.
pub(crate) fn write_exceptions(notify: &NotifyStyle, sheet: &mut crate::sheet::Sheet) {
    let notify = std::sync::Arc::new(notify.clone());
    sheet.rule_under(NotifyPart::Toast, move |_, _| {
        look::Look::new(notify.clone()).card()
    });
}
