//! TODO(prose): pop-out windows, a keyed subtree shown inline or in a window
//! of its own.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use egui::{Event, Id, Pos2, Vec2, ViewportBuilder, ViewportId};

use crate::builder::{Builder, Scope, Slot};
use crate::kept::Handle;
use crate::{Tw, tw};

/// TODO(prose): where a pop-out's subtree is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Popped {
    /// TODO(prose): at its place in the layout.
    #[default]
    In,
    /// TODO(prose): in a window of its own.
    Out,
}

/// TODO(prose): a window's outer position and inner size, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowFrame {
    /// TODO(prose): the outer top-left corner, on the screen; `None` where it
    /// is not known (Wayland) or left to the platform.
    pub position: Option<Pos2>,
    /// TODO(prose): the size of the content.
    pub size: Vec2,
}

/// TODO(prose): the smallest a pop-out window can be made.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MinSize {
    /// TODO(prose): the subtree's min-content size.
    #[default]
    Content,
    /// TODO(prose): a size in points.
    Points(Vec2),
}

type ViewportFn = Arc<dyn Fn(ViewportBuilder) -> ViewportBuilder + Send + Sync>;

/// TODO(prose): the window a pop-out is shown in while it is popped out.
#[derive(Clone)]
pub struct Window {
    pub(crate) title: String,
    pub(crate) frame: Option<WindowFrame>,
    pub(crate) min_size: MinSize,
    pub(crate) viewport: Option<ViewportFn>,
}

impl Window {
    /// TODO(prose): a window titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            frame: None,
            min_size: MinSize::Content,
            viewport: None,
        }
    }

    /// TODO(prose): where the window opens, and how large.
    #[must_use]
    pub fn frame(mut self, frame: Option<WindowFrame>) -> Self {
        self.frame = frame;
        self
    }

    /// TODO(prose): the smallest the window can be made.
    #[must_use]
    pub fn min_size(mut self, min_size: MinSize) -> Self {
        self.min_size = min_size;
        self
    }

    /// TODO(prose): changes the window's [`ViewportBuilder`] before it opens.
    /// The size the subtree is laid out at is [`frame`](Self::frame)'s, not
    /// an inner size set here, and the title is always [`Window::new`]'s.
    #[must_use]
    pub fn viewport(
        mut self,
        f: impl Fn(ViewportBuilder) -> ViewportBuilder + Send + Sync + 'static,
    ) -> Self {
        self.viewport = Some(Arc::new(f));
        self
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("title", &self.title)
            .field("frame", &self.frame)
            .field("min_size", &self.min_size)
            .field("viewport", &self.viewport.as_ref().map(|_| ".."))
            .finish()
    }
}

pub(crate) type CloseFn<A> = Box<dyn FnOnce(&mut A)>;
pub(crate) type FrameFn<A> = Box<dyn FnOnce(&mut A, WindowFrame)>;

/// What the application runs when its window is closed or moved.
pub(crate) struct Hooks<A> {
    pub(crate) on_close: Option<CloseFn<A>>,
    pub(crate) on_frame: Option<FrameFn<A>>,
}

impl<A: 'static> Hooks<A> {
    /// The same hooks, for the application data `lens` reaches these in.
    pub(crate) fn lift<R: 'static>(self, lens: crate::mount::Lens<R, A>) -> Hooks<R> {
        let close = lens.clone();
        Hooks {
            on_close: self
                .on_close
                .map(|then| -> CloseFn<R> { Box::new(move |app| then(close(app))) }),
            on_frame: self
                .on_frame
                .map(|then| -> FrameFn<R> { Box::new(move |app, frame| then(lens(app), frame)) }),
        }
    }
}

impl<A> Default for Hooks<A> {
    fn default() -> Self {
        Self {
            on_close: None,
            on_frame: None,
        }
    }
}

/// A popped-out subtree's window, as its chain declared it.
#[derive(Debug)]
pub(crate) struct WindowDecl {
    pub(crate) id: Id,
    pub(crate) window: Window,
}

impl WindowDecl {
    /// The viewport of the window: the pop-out's id, which its path names,
    /// so the same pop-out has the same viewport from run to run.
    pub(crate) fn viewport(&self) -> ViewportId {
        ViewportId(self.id)
    }
}

/// How a pop-out's window is shown: as a native window, or, where the
/// backend has one viewport (egui's `embed_viewports`), as an `egui::Window`
/// on the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Presentation {
    Native,
    Embedded,
}

/// A popped-out subtree's window after layout.
#[derive(Clone, Debug)]
pub(crate) struct PlacedWindow {
    pub(crate) id: Id,
    pub(crate) viewport: ViewportId,
    pub(crate) window: Window,
    pub(crate) presentation: Presentation,
    /// Where the subtree was laid out: the window's content as it was last
    /// seen, in the window's own coordinates when it is native.
    pub(crate) content: egui::Rect,
    /// The smallest content the window allows.
    pub(crate) min: Vec2,
}

impl PlacedWindow {
    pub(crate) fn viewport(&self) -> ViewportId {
        self.viewport
    }
}

/// What a layout remembers of a pop-out's window between frames.
#[derive(Clone, Debug, Default)]
pub(crate) struct Seen {
    /// The window's content as last drawn: the size the next layout lays the
    /// subtree out at.
    pub(crate) content: Option<egui::Rect>,
    /// The frame last told to the application.
    pub(crate) reported: Option<WindowFrame>,
    /// What the native window was opened with, given to egui unchanged for
    /// as long as it stays open.
    pub(crate) opened: Option<ViewportBuilder>,
}

/// Where a pop-out's subtree was last drawn, and the widget in it that had
/// focus there: egui keeps focus per viewport, so focus follows the subtree
/// to another one by asking for it there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Carried {
    viewport: ViewportId,
    focused: Option<Id>,
}

/// The pop-outs of `pop_outs` that the pass `here` draws.
fn drawn_here<'p>(
    pop_outs: &'p [Id],
    roots: &'p [crate::roots::PlacedRoot],
    here: crate::roots::Here,
) -> impl Iterator<Item = Id> + 'p {
    pop_outs.iter().copied().filter(move |&id| {
        let home = roots
            .iter()
            .find(|root| root.window().is_some_and(|window| window.id == id))
            .map_or(crate::roots::Home::Page, |root| root.home);
        home == here.home
    })
}

/// Gives focus, in the pass `here`, to the widget that had it in a subtree
/// that was last drawn in another viewport.
pub(crate) fn carry_focus_in(
    ctx: &egui::Context,
    kept: &Handle,
    (pop_outs, roots): (&[Id], &[crate::roots::PlacedRoot]),
    here: crate::roots::Here,
) {
    let viewport = ctx.viewport_id();
    for id in drawn_here(pop_outs, roots, here) {
        let carried = kept.with(|kept| kept.carried.get(&id).copied());
        if let Some(Carried {
            viewport: from,
            focused: Some(focused),
        }) = carried
            && from != viewport
        {
            ctx.memory_mut(|memory| memory.request_focus(focused));
        }
    }
}

/// Notes, at the end of the pass `here`, which widget has focus in each
/// subtree the pass drew.
pub(crate) fn note_focus(
    ctx: &egui::Context,
    kept: &Handle,
    (pop_outs, roots): (&[Id], &[crate::roots::PlacedRoot]),
    here: crate::roots::Here,
) {
    let shell = crate::Shell::new(ctx, kept);
    let focused = ctx.memory(|memory| memory.focused());
    for id in drawn_here(pop_outs, roots, here) {
        let carried = Carried {
            viewport: ctx.viewport_id(),
            focused: focused.filter(|_| shell.focus_within(id)),
        };
        kept.with(|kept| kept.carried.insert(id, carried));
    }
}

/// The key of the inline slot that stands in for a popped-out subtree.
struct Placeholder<'a>(&'a crate::builder::Label);

impl fmt::Debug for Placeholder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Placeholder({:?})", self.0)
    }
}

type PlaceholderFn<'p, 'f, A> = Box<dyn FnOnce(&mut Scope<'_, 'f, A>) + 'p>;

/// TODO(prose): a pop-out being declared: [`Builder::pop_out`] starts it, and
/// [`children`](Self::children) builds its subtree and ends it.
///
/// ```
/// use eggshell_ui::window::{Popped, Window};
/// use eggshell_ui::{Builder, tw};
///
/// struct App { popped: Popped }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.pop_out("pane")
///         .popped(app.popped)
///         .window(Window::new("Pane"))
///         .on_close(|app: &mut App| app.popped = Popped::In)
///         .placeholder(|s| s.text(tw(), "Shown in its own window"))
///         .children(|s| s.text(tw().p(2.0), "Hello"));
/// }
/// ```
#[must_use = "a pop-out is declared by `children`"]
pub struct PopOutChain<'b, 'f, A> {
    b: Builder<'b, 'f, A>,
    slot: Slot,
    classes: Tw,
    popped: Popped,
    window: Window,
    hooks: Hooks<A>,
    placeholder: Option<PlaceholderFn<'b, 'f, A>>,
}

impl<'b, 'f, A> PopOutChain<'b, 'f, A> {
    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        crate::owned::options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        crate::owned::options::when_some(self, value, then)
    }
}

impl<'b, 'f, A: 'static> PopOutChain<'b, 'f, A> {
    pub(crate) fn new(b: Builder<'b, 'f, A>, slot: Slot) -> Self {
        Self {
            b,
            slot,
            classes: tw(),
            popped: Popped::In,
            window: Window::new(String::new()),
            hooks: Hooks::default(),
            placeholder: None,
        }
    }

    /// TODO(prose): the classes of the subtree's container, inline and in its
    /// window.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// TODO(prose): where the subtree is shown this frame.
    pub fn popped(mut self, popped: Popped) -> Self {
        self.popped = popped;
        self
    }

    /// TODO(prose): the window the subtree is shown in while popped out.
    pub fn window(mut self, window: Window) -> Self {
        self.window = window;
        self
    }

    /// TODO(prose): runs when the window's close button is used.
    pub fn on_close(mut self, then: impl FnOnce(&mut A) + 'static) -> Self {
        self.hooks.on_close = Some(Box::new(then));
        self
    }

    /// TODO(prose): runs with the window's frame when it is moved or resized.
    pub fn on_frame(mut self, then: impl FnOnce(&mut A, WindowFrame) + 'static) -> Self {
        self.hooks.on_frame = Some(Box::new(then));
        self
    }

    /// TODO(prose): what the inline slot shows while the subtree is popped
    /// out.
    pub fn placeholder(mut self, body: impl FnOnce(&mut Scope<'_, 'f, A>) + 'b) -> Self {
        self.placeholder = Some(Box::new(body));
        self
    }

    /// TODO(prose): builds the subtree and ends the chain.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Self {
            mut b,
            slot,
            classes,
            popped,
            window,
            hooks,
            placeholder,
        } = self;
        b.note_pop_out(slot.id);
        match popped {
            Popped::In => {
                b.div_id_with(&slot, classes, body);
            }
            Popped::Out => {
                if let Some(placeholder) = placeholder {
                    let stand_in = b.slot_of(
                        slot.id.with("placeholder"),
                        format!("{:?}", Placeholder(&slot.label)),
                    );
                    b.div_id_with(&stand_in, classes.clone(), placeholder);
                }
                let decl = WindowDecl {
                    id: slot.id,
                    window,
                };
                b.window_with(&slot, decl, hooks, classes, body);
            }
        }
    }
}

/// What became of a pop-out's window in the pass that drew it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shown {
    /// The index of the window's root.
    pub(crate) root: usize,
    /// Whether the window's close button was used.
    pub(crate) close_asked: bool,
    /// The window's content as drawn: in its own coordinates when native.
    pub(crate) content: egui::Rect,
    /// The window's outside, where the backend says.
    pub(crate) outer: Option<egui::Rect>,
}

impl Shown {
    /// What the native window of the root at `root` reports in its own pass,
    /// whose content was `content`.
    pub(crate) fn native(ctx: &egui::Context, root: usize, content: egui::Rect) -> Self {
        let (asked, outer) = ctx.input(|input| {
            let viewport = input.viewport();
            (viewport.close_requested(), viewport.outer_rect)
        });
        Self {
            root,
            close_asked: asked,
            content,
            outer,
        }
    }
}

/// The builder the native window of `window` is shown with: the one it
/// opened with, kept while it stays open so that egui sends the window no
/// command that would undo the user's move or resize, under the title and
/// min size the window has now.
pub(crate) fn builder(kept: &Handle, window: &PlacedWindow) -> ViewportBuilder {
    let decl = &window.window;
    kept.with(|kept| {
        let seen = kept.windows.entry(window.id).or_default();
        seen.opened
            .get_or_insert_with(|| {
                let mut opening = ViewportBuilder::default().with_inner_size(window.content.size());
                if let Some(position) = decl.frame.and_then(|frame| frame.position) {
                    opening = opening.with_position(position);
                }
                match &decl.viewport {
                    Some(change) => change(opening),
                    None => opening,
                }
            })
            .clone()
            .with_title(decl.title.clone())
            .with_min_inner_size(window.min)
    })
}

/// What an `egui::Window` draws around its content: its title bar, frame
/// and margins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Chrome {
    /// From the window's top-left corner to its content's.
    pub(crate) offset: Vec2,
    /// What the window is larger than its content.
    pub(crate) extra: Vec2,
}

/// The `egui::Window` that stands in for `window`'s native window on a
/// backend with one viewport, its content at `window.content`.
/// `open` is cleared by its close button.
///
/// egui sizes and places a window by its outside, so its content's place is
/// known only once `chrome` has been measured on a window drawn before. Until
/// then the window is drawn under an id of its own, which the next frame,
/// with the chrome known, leaves for a window opened at the right size.
pub(crate) fn embedded<'o>(
    window: &PlacedWindow,
    chrome: Option<Chrome>,
    open: &'o mut bool,
) -> egui::Window<'o> {
    let around = chrome.unwrap_or(Chrome {
        offset: Vec2::ZERO,
        extra: Vec2::ZERO,
    });
    egui::Window::new(window.window.title.clone())
        .id(window.id.with(("window", chrome.is_some())))
        .open(open)
        .default_pos(window.content.min - around.offset)
        .default_size(window.content.size() + around.extra)
        .min_size(window.min)
        .resizable(true)
        .collapsible(false)
}

/// Tells the application what became of `window`: its frame when it moved
/// or was resized, and the use of its close button. Asks the page's
/// viewport (`page`) for another frame when the window's content is not
/// the size it was laid out at.
pub(crate) fn report<A>(
    ctx: &egui::Context,
    (kept, page): (&Handle, ViewportId),
    window: &PlacedWindow,
    shown: Shown,
    hooks: &mut HashMap<Id, Hooks<A>>,
    app: &mut A,
) {
    let laid = match window.presentation {
        Presentation::Native => shown.content.size() != window.content.size(),
        Presentation::Embedded => shown.content != window.content,
    };
    if laid {
        ctx.request_repaint_of(page);
    }
    let reported = kept.with(|kept| {
        if window.presentation == Presentation::Embedded
            && let Some(outer) = shown.outer
        {
            let measuring = kept.chrome.is_none();
            kept.chrome = Some(Chrome {
                offset: shown.content.min - outer.min,
                extra: outer.size() - shown.content.size(),
            });
            if measuring {
                return None;
            }
        }
        let seen = kept.windows.entry(window.id).or_default();
        let before = seen.reported.or(window.window.frame);
        let frame = WindowFrame {
            position: shown
                .outer
                .map(|outer| outer.min)
                .or(before.and_then(|before| before.position)),
            size: shown.content.size(),
        };
        seen.content = Some(shown.content);
        seen.reported = Some(frame);
        (before != Some(frame)).then_some(frame)
    });
    let hooks = hooks.entry(window.id).or_default();
    if let Some(frame) = reported
        && let Some(then) = hooks.on_frame.take()
    {
        then(app, frame);
    }
    if shown.close_asked
        && let Some(then) = hooks.on_close.take()
    {
        then(app);
    }
}

/// Starts the pass of a native window, `here`: gives focus back for the
/// overlays of the window that closed since its last pass, and notes where
/// focus is for those that opened, which egui knows only in this pass.
pub(crate) fn enter_pass(
    ctx: &egui::Context,
    kept: &Handle,
    roots: &[crate::roots::PlacedRoot],
    here: crate::roots::Here,
) {
    let viewport = ctx.viewport_id();
    let closed = kept.viewport(viewport, |here| std::mem::take(&mut here.focus.closed));
    crate::focus::restore(ctx, kept, &closed);
    let focused = ctx.memory(|memory| memory.focused());
    let mut stack = crate::stack::Stack::load(kept, viewport);
    for root in roots.iter().filter(|root| root.home == here.home) {
        let Some(overlay) = root.overlay() else {
            continue;
        };
        if overlay.opening == crate::dismiss::Opening::Opened
            && overlay.behavior.focus != crate::overlay::Focus::None
            && let Some(open) = stack.get_mut(overlay.id)
        {
            open.restore = focused;
        }
    }
    stack.store(kept, viewport);
}

/// Whether `events` say the window lost the keyboard focus.
pub(crate) fn blurred(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::WindowFocused(false)))
}
