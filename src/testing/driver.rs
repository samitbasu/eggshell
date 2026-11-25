//! [`Driver`]: a test that drives a view through whole frames, as a user
//! does, and asks the frame what happened.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{
    Color32, Event, Id, Key as EguiKey, KeyboardShortcut, Modifiers, PointerButton, Pos2, Rect,
    Theme, Vec2,
};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::{PixelsPerPoint, PlacedFrame, list};
use crate::contract::Themed;
use crate::key::{Key, child_id};
use crate::{Builder, Chord, LayoutState, Problem, Report, Shell, Style, TextStyle};

/// egui's own animation time, which kittest sets to zero.
const FULL_MOTION: f32 = 1.0 / 12.0;

type DrawFrame<A> = Box<dyn FnMut(&mut egui::Ui, &mut A, &mut LayoutState) -> PlacedFrame>;
type Setup = Box<dyn FnOnce(&egui::Context)>;

/// TODO(prose): a view under test, driven through whole frames: each frame builds, lays
/// out and draws it as an application's root does, with the input of that
/// frame, so a click, a key or a drag takes the path a user's does.
///
/// It wraps an [`egui_kittest`] [`Harness`]: every event goes through the
/// harness's queue and every frame through its step, and
/// [`harness`](Self::harness) stays open for kittest's own queries.
///
/// ```
/// use eggshell_ui::testing::Driver;
/// use eggshell_ui::{Builder, tw};
///
/// #[derive(Default)]
/// struct App { clicks: u32 }
///
/// #[derive(Hash, Debug)]
/// enum Part { Save }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.div(tw().p(4.0), |b| {
///         b.button(Part::Save, format!("Saved {}", app.clicks))
///             .on_click(|app: &mut App| app.clicks += 1);
///     });
/// }
///
/// let mut d = Driver::new(App::default(), view).build();
/// d.click(d.path(Part::Save));
/// assert_eq!(d.app().clicks, 1);
/// assert!(d.painted().shows("Saved 1"));
/// ```
///
/// # After an action
///
/// Each action queues its events, runs one frame per event, then applies
/// the driver's [`After`] policy: by default it [settles](Self::settle),
/// running frames until nothing asks for an immediate repaint. A settle
/// that goes on past [`settle_within`](DriverBuilder::settle_within) fails
/// the test, naming what still asks for a repaint, so a view that animates
/// forever fails at once instead of hanging.
///
/// A repaint asked for later, such as a tooltip's delay or a toast's dwell,
/// ends a settle: wait for it with [`advance`](Self::advance).
///
/// ```
/// # use eggshell_ui::testing::Driver;
/// # use eggshell_ui::{Builder, tw};
/// # use std::time::Duration;
/// let view = |b: &mut Builder<'_, '_, ()>, _: &()| {
///     b.div_id("help", tw().w_px(80.0).h_px(30.0), |_| {})
///         .tooltip("More about this");
/// };
/// let mut d = Driver::new((), view).build();
/// d.hover(d.path("help"));
/// assert!(!d.painted().shows("More about this"));
/// let style = d.shell().style();
/// d.advance(style.tooltip.delay + style.motion.tooltip.enter + Duration::from_millis(100));
/// assert!(d.painted().shows("More about this"));
/// ```
///
/// # Reports
///
/// A driver checks, when it is dropped, that the view raised no
/// [`Report`]: a test that expects one claims it with
/// [`expect_reported`](Self::expect_reported), and a test that does not
/// care builds its driver with [`Reports::Unchecked`]. The check is
/// skipped when the test is already failing.
pub struct Driver<A: 'static> {
    harness: Harness<'static, Stage<A>>,
    step: Duration,
    after: After,
    max_steps: u64,
    reports: Reports,
    claimed: Vec<Problem>,
    start: f64,
}

/// TODO(prose): what the driver's harness holds: the app, its layout and the last frame
/// drawn. Reach the app through [`Driver::app`].
pub struct Stage<A> {
    app: A,
    layout: LayoutState,
    frame: Option<PlacedFrame>,
    motion: Animation,
}

/// TODO(prose): whether the driver runs motion as an application does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Animation {
    /// TODO(prose): transitions, slides and overlay motion run, as in an application.
    #[default]
    Full,
    /// TODO(prose): egui's animation time stays at kittest's zero, which eggshell takes
    /// as reduced motion: every motion jumps to its end.
    Reduced,
}

/// TODO(prose): what a driver runs after each action's own frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum After {
    /// TODO(prose): frames until nothing asks for an immediate repaint
    /// ([`Driver::settle`]).
    #[default]
    Settle,
    /// TODO(prose): this many frames more.
    Frames(usize),
}

/// TODO(prose): whether a driver checks, when dropped, that nothing was reported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reports {
    /// TODO(prose): a report the test did not claim fails it.
    #[default]
    Guarded,
    /// TODO(prose): reports are left for the test to read, or not.
    Unchecked,
}

/// TODO(prose): whether the window has the platform's focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFocus {
    /// TODO(prose): the window is the one the user is in.
    Focused,
    /// TODO(prose): the user went to another window.
    Blurred,
}

/// TODO(prose): keys to press: a [`Key`](egui::Key) alone, or with modifiers held as a
/// [`KeyboardShortcut`] or a [`Chord`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press(KeyboardShortcut);

impl From<EguiKey> for Press {
    fn from(key: EguiKey) -> Self {
        Self(KeyboardShortcut::new(Modifiers::NONE, key))
    }
}

impl From<KeyboardShortcut> for Press {
    fn from(shortcut: KeyboardShortcut) -> Self {
        Self(shortcut)
    }
}

impl From<Chord> for Press {
    fn from(chord: Chord) -> Self {
        Self(chord.into())
    }
}

/// TODO(prose): a frame run to quiet by [`Driver::try_settle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settled {
    /// TODO(prose): the frames it took.
    pub frames: u64,
}

/// TODO(prose): a view still asking for a repaint when [`Driver::try_settle`] gave up.
#[derive(Clone, Debug)]
pub struct Unsettled {
    /// TODO(prose): the frames run.
    pub frames: u64,
    /// TODO(prose): the clock when it gave up, since the driver was built.
    pub clock: Duration,
    /// TODO(prose): what asked for the last repaints.
    pub causes: Vec<egui::RepaintCause>,
}

/// TODO(prose): what has keyboard focus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focused {
    /// TODO(prose): an identified node, by its readable path.
    Node(String),
    /// TODO(prose): an egui widget that is not itself a node, such as the text edit of a
    /// field, inside the deepest node `within` that holds it, if any.
    Widget {
        /// The readable path of the deepest node that holds the widget.
        within: Option<String>,
    },
}

/// TODO(prose): a node to act on or ask about, resolved against the last frame drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// TODO(prose): typed keys from the root: [`Driver::path`].
    Path(Path),
    /// TODO(prose): the one node whose readable path ends with this. A node whose whole
    /// path is this wins over those it is only the end of.
    Suffix(String),
    /// TODO(prose): the one control with this accessibility role and name, found
    /// through kittest.
    Role(Role, String),
    /// TODO(prose): the one node that shows this text, as [`Query::text`](super::Query::text).
    Text(String),
    /// TODO(prose): the identified node with this id.
    Id(Id),
    /// TODO(prose): a point in the window.
    Point(Pos2),
}

impl Target {
    /// TODO(prose): [`Target::Suffix`].
    pub fn suffix(suffix: &str) -> Self {
        Self::Suffix(suffix.to_owned())
    }

    /// TODO(prose): [`Target::Role`].
    pub fn role(role: Role, name: &str) -> Self {
        Self::Role(role, name.to_owned())
    }

    /// TODO(prose): [`Target::Text`].
    pub fn text(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<Path> for Target {
    fn from(path: Path) -> Self {
        Self::Path(path)
    }
}

impl From<Pos2> for Target {
    fn from(point: Pos2) -> Self {
        Self::Point(point)
    }
}

impl From<Id> for Target {
    fn from(id: Id) -> Self {
        Self::Id(id)
    }
}

/// TODO(prose): an owned key path from the root of the driver's layout. Each key is
/// hashed as the builder hashes it, so it names the node the builder made,
/// and it borrows nothing: `d.click(d.path(Part::Card).at(Part::Save))`.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    steps: Vec<(String, Id)>,
}

impl Path {
    /// TODO(prose): the node declared with `key` below this one.
    pub fn at<K: Key>(mut self, key: K) -> Self {
        let parent = self.steps.last().map_or(Id::NULL, |(_, id)| *id);
        self.steps
            .push((format!("{key:?}"), child_id(parent, &key)));
        self
    }
}

impl<A: 'static> Driver<A> {
    /// TODO(prose): a driver that builds, lays out and draws `view` of `app` each frame,
    /// over the whole window.
    #[expect(
        clippy::new_ret_no_self,
        reason = "the view is named first, then the driver set up and built"
    )]
    pub fn new(
        app: A,
        mut view: impl for<'f> FnMut(&mut Builder<'_, 'f, A>, &'f A) + 'static,
    ) -> DriverBuilder<A> {
        Self::whole_frame(app, move |ui, app, layout| {
            let area = ui.max_rect();
            layout
                .build(ui.ctx(), |b| view(b, &*app))
                .layout(area)
                .draw(app, ui)
        })
    }

    /// TODO(prose): a driver for a view that draws the whole frame itself, as an
    /// application's `update` does: scroll areas, a bar beside the layout,
    /// work after the draw. `frame` draws with the [`LayoutState`] it is
    /// given and answers the frame its [`Placed::draw`](crate::Placed::draw)
    /// answered, which the driver asks.
    pub fn whole_frame(
        app: A,
        frame: impl FnMut(&mut egui::Ui, &mut A, &mut LayoutState) -> PlacedFrame + 'static,
    ) -> DriverBuilder<A> {
        DriverBuilder {
            app,
            draw: Box::new(frame),
            size: None,
            theme: None,
            pixels_per_point: None,
            step: Duration::from_nanos(1_000_000_000 / 60),
            motion: Animation::Full,
            base_text: None,
            style: None,
            setup: None,
            after: After::Settle,
            cap: Duration::from_secs(2),
            reports: Reports::Guarded,
            harness: None,
        }
    }
}

/// TODO(prose): how a [`Driver`] is set up; [`build`](Self::build) makes it.
#[must_use]
pub struct DriverBuilder<A: 'static> {
    app: A,
    draw: DrawFrame<A>,
    size: Option<Vec2>,
    theme: Option<Theme>,
    pixels_per_point: Option<PixelsPerPoint>,
    step: Duration,
    motion: Animation,
    base_text: Option<TextStyle>,
    style: Option<Themed<Arc<Style>>>,
    setup: Option<Setup>,
    after: After,
    cap: Duration,
    reports: Reports,
    harness: Option<HarnessBuilder<Stage<A>>>,
}

impl<A: 'static> DriverBuilder<A> {
    /// TODO(prose): the window's size in points; kittest's 800 by 600 when unset.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): the theme the window starts in; kittest's dark when unset.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// TODO(prose): the display's scale.
    pub fn pixels_per_point(mut self, scale: PixelsPerPoint) -> Self {
        self.pixels_per_point = Some(scale);
        self
    }

    /// TODO(prose): the clock time between frames: a sixtieth of a second when unset,
    /// a real frame, at which a double click registers.
    pub fn step(mut self, step: Duration) -> Self {
        self.step = step;
        self
    }

    /// TODO(prose): whether motion runs; [`Animation::Full`] when unset.
    pub fn motion(mut self, motion: Animation) -> Self {
        self.motion = motion;
        self
    }

    /// TODO(prose): the root text style of the layout, as [`LayoutState::with_base_text`].
    pub fn base_text(mut self, base: TextStyle) -> Self {
        self.base_text = Some(base);
        self
    }

    /// TODO(prose): the layout's style, as [`LayoutState::set_style`]; the default shadcn style when unset.
    pub fn style(mut self, style: Themed<Arc<Style>>) -> Self {
        self.style = Some(style);
        self
    }

    /// TODO(prose): runs `setup` on the context before the first frame: a style, fonts,
    /// [`record`](super::record). It replaces a setup the harness builder of
    /// [`with_harness`](Self::with_harness) carried.
    pub fn setup(mut self, setup: impl FnOnce(&egui::Context) + 'static) -> Self {
        self.setup = Some(Box::new(setup));
        self
    }

    /// TODO(prose): what runs after each action; [`After::Settle`] when unset.
    pub fn after(mut self, after: After) -> Self {
        self.after = after;
        self
    }

    /// TODO(prose): the most clock time a settle may take before it fails; two seconds
    /// when unset.
    pub fn settle_within(mut self, cap: Duration) -> Self {
        self.cap = cap;
        self
    }

    /// TODO(prose): whether reports fail the test; [`Reports::Guarded`] when unset.
    pub fn reports(mut self, reports: Reports) -> Self {
        self.reports = reports;
        self
    }

    /// TODO(prose): starts from a preconfigured kittest builder, such as
    /// `testing::screenshot::builder`'s. The driver's own settings are
    /// applied over it.
    pub fn with_harness(mut self, builder: HarnessBuilder<Stage<A>>) -> Self {
        self.harness = Some(builder);
        self
    }

    /// TODO(prose): the driver, with its first frame drawn. kittest runs frames until
    /// the view is quiet, at most the settle cap; under
    /// [`After::Settle`] a view still repainting fails here.
    ///
    /// # Panics
    ///
    /// If the step is zero, or the view does not settle.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn build(self) -> Driver<A> {
        let Self {
            app,
            mut draw,
            size,
            theme,
            pixels_per_point,
            step,
            motion,
            base_text,
            style,
            setup,
            after,
            cap,
            reports,
            harness,
        } = self;
        if step.is_zero() {
            panic!("a driver's step must be longer than zero");
        }
        let max_steps = frames_in(cap, step).max(1);
        let mut builder = harness
            .unwrap_or_default()
            .with_step_dt(step.as_secs_f32())
            .with_max_steps(u64::try_from(max_steps).unwrap_or(u64::MAX));
        if let Some(size) = size {
            builder = builder.with_size(size);
        }
        if let Some(theme) = theme {
            builder = builder.with_theme(theme);
        }
        if let Some(scale) = pixels_per_point {
            builder = builder.with_pixels_per_point(scale.into());
        }
        if let Some(setup) = setup {
            builder = builder.with_context_setup(setup);
        }
        let layout = LayoutState::new(style.unwrap_or_else(super::default_style));
        let layout = match base_text {
            Some(base) => layout.with_base_text(base),
            None => layout,
        };
        let stage = Stage {
            app,
            layout,
            frame: None,
            motion,
        };
        let harness = builder.build_ui_state(
            move |ui, stage: &mut Stage<A>| {
                if stage.motion == Animation::Full && ui.ctx().global_style().animation_time == 0.0
                {
                    ui.ctx()
                        .global_style_mut(|style| style.animation_time = FULL_MOTION);
                }
                stage.frame = Some(draw(ui, &mut stage.app, &mut stage.layout));
            },
            stage,
        );
        let start = harness.ctx.input(|input| input.time);
        let driver = Driver {
            harness,
            step,
            after,
            max_steps: u64::try_from(max_steps).unwrap_or(u64::MAX),
            reports,
            claimed: Vec::new(),
            start,
        };
        if after == After::Settle && driver.repainting() {
            driver.unsettled(driver.max_steps).fail();
        }
        driver
    }
}

/// How many frames of `step` it takes for `span` to pass.
fn frames_in(span: Duration, step: Duration) -> usize {
    let step = step.as_nanos().max(1);
    usize::try_from(span.as_nanos().div_ceil(step)).unwrap_or(usize::MAX)
}

impl Unsettled {
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    fn fail(&self) -> ! {
        panic!(
            "did not settle within {:?} ({} frames); still repainting: {}",
            self.clock,
            self.frames,
            list(self.causes.iter().map(ToString::to_string)),
        )
    }
}

// Time.
impl<A: 'static> Driver<A> {
    /// TODO(prose): runs `n` frames.
    pub fn frames(&mut self, n: usize) -> &mut Self {
        self.harness.run_steps(n);
        self
    }

    /// TODO(prose): runs frames until `by` of clock time has passed: a tooltip's delay,
    /// a toast's dwell.
    pub fn advance(&mut self, by: Duration) -> &mut Self {
        self.frames(frames_in(by, self.step))
    }

    /// TODO(prose): runs frames until nothing asks for an immediate repaint.
    ///
    /// # Panics
    ///
    /// If the view still asks for one after the settle cap, naming what
    /// asks.
    #[track_caller]
    pub fn settle(&mut self) -> &mut Self {
        if let Err(unsettled) = self.try_settle() {
            unsettled.fail();
        }
        self
    }

    /// TODO(prose): [`settle`](Self::settle), or what still repaints.
    ///
    /// # Errors
    ///
    /// If the view still asks for a repaint after the settle cap.
    pub fn try_settle(&mut self) -> Result<Settled, Unsettled> {
        match self.harness.run_ok() {
            Some(frames) => Ok(Settled { frames }),
            None => Err(self.unsettled(self.max_steps)),
        }
    }

    /// TODO(prose): the clock time since the driver was built.
    pub fn now(&self) -> Duration {
        let now = self.harness.ctx.input(|input| input.time);
        Duration::try_from_secs_f64(now - self.start).unwrap_or_default()
    }

    fn unsettled(&self, frames: u64) -> Unsettled {
        Unsettled {
            frames,
            clock: self.now(),
            causes: self.harness.ctx.repaint_causes(),
        }
    }

    fn repainting(&self) -> bool {
        self.harness
            .output()
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_some_and(|viewport| viewport.repaint_delay.is_zero())
    }

    fn after(&mut self) -> &mut Self {
        match self.after {
            After::Settle => self.settle(),
            After::Frames(n) => self.frames(n),
        }
    }

    /// Queues `events` and runs a frame for each.
    fn send(&mut self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            self.harness.event(event);
        }
        self.harness.step();
    }
}

fn button(at: Pos2, button: PointerButton, modifiers: Modifiers, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers,
    }
}

// The pointer.
impl<A: 'static> Driver<A> {
    /// TODO(prose): rests the pointer on `at` for three frames.
    #[track_caller]
    pub fn hover(&mut self, at: impl Into<Target>) -> &mut Self {
        let at = self.point(at.into());
        self.rest_on(at);
        self.after()
    }

    /// TODO(prose): clicks the primary button on `at`: the pointer rests there three
    /// frames, then presses and releases, a frame each.
    #[track_caller]
    pub fn click(&mut self, at: impl Into<Target>) -> &mut Self {
        self.click_with(at, PointerButton::Primary, Modifiers::NONE)
    }

    /// TODO(prose): [`click`](Self::click) with `button`, `modifiers` held.
    #[track_caller]
    pub fn click_with(
        &mut self,
        at: impl Into<Target>,
        button: PointerButton,
        modifiers: Modifiers,
    ) -> &mut Self {
        let at = self.press_point(at.into());
        self.rest_on(at);
        self.press_and_release(at, button, modifiers);
        self.after()
    }

    /// TODO(prose): two clicks on `at`, within egui's double-click window at the
    /// default step.
    #[track_caller]
    pub fn double_click(&mut self, at: impl Into<Target>) -> &mut Self {
        let at = self.press_point(at.into());
        self.rest_on(at);
        self.press_and_release(at, PointerButton::Primary, Modifiers::NONE);
        self.press_and_release(at, PointerButton::Primary, Modifiers::NONE);
        self.after()
    }

    /// TODO(prose): rests on `at` two frames and presses the primary button there,
    /// without releasing it.
    #[track_caller]
    pub fn pointer_down(&mut self, at: impl Into<Target>) -> &mut Self {
        let at = self.press_point(at.into());
        self.harness.hover_at(at);
        self.harness.run_steps(2);
        self.send([button(at, PointerButton::Primary, Modifiers::NONE, true)]);
        self.after()
    }

    /// TODO(prose): releases the primary button on `at`.
    #[track_caller]
    pub fn pointer_up(&mut self, at: impl Into<Target>) -> &mut Self {
        let at = self.point(at.into());
        self.send([button(at, PointerButton::Primary, Modifiers::NONE, false)]);
        self.after()
    }

    /// TODO(prose): presses on `from`, carries the pointer to `to` through their
    /// midpoint and releases there; the pointer then leaves the window.
    #[track_caller]
    pub fn drag(&mut self, from: impl Into<Target>, to: impl Into<Target>) -> &mut Self {
        let from = self.press_point(from.into());
        let to = self.point(to.into());
        self.rest_on(from);
        for event in [
            button(from, PointerButton::Primary, Modifiers::NONE, true),
            Event::PointerMoved(from.lerp(to, 0.5)),
            Event::PointerMoved(to),
        ] {
            self.send([event]);
        }
        self.harness.drop_at(to);
        self.harness.run_steps(3);
        self.after()
    }

    /// TODO(prose): presses on `from` and carries the pointer `past` it in two equal
    /// moves, without releasing: a drag lifted and held.
    #[track_caller]
    pub fn lift(&mut self, from: impl Into<Target>, past: Vec2) -> &mut Self {
        let from = self.press_point(from.into());
        self.rest_on(from);
        self.send([button(from, PointerButton::Primary, Modifiers::NONE, true)]);
        self.send([Event::PointerMoved(from + past / 2.0)]);
        self.send([Event::PointerMoved(from + past)]);
        self.after()
    }

    /// TODO(prose): moves the pointer to `to`, a button held or not.
    #[track_caller]
    pub fn carry_to(&mut self, to: impl Into<Target>) -> &mut Self {
        let to = self.point(to.into());
        self.send([Event::PointerMoved(to)]);
        self.after()
    }

    /// TODO(prose): rests on `at` and turns the wheel by `delta` points.
    #[track_caller]
    pub fn wheel(&mut self, at: impl Into<Target>, delta: Vec2) -> &mut Self {
        let at = self.point(at.into());
        self.rest_on(at);
        self.send([Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        }]);
        self.after()
    }

    fn rest_on(&mut self, at: Pos2) {
        self.harness.hover_at(at);
        self.harness.run_steps(3);
    }

    fn press_and_release(&mut self, at: Pos2, pressed: PointerButton, modifiers: Modifiers) {
        for down in [true, false] {
            self.send([button(at, pressed, modifiers, down)]);
        }
    }

    /// The point a pointer action on `target` uses: its centre.
    #[track_caller]
    fn point(&self, target: Target) -> Pos2 {
        self.rect(target).center()
    }

    /// The point a press on `target` uses: its centre, which must be in the
    /// window, so a node scrolled or placed off screen is never pressed at a
    /// point egui clips. A move or a release may go past the window's edge,
    /// as a held drag does.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    fn press_point(&self, target: Target) -> Pos2 {
        let rect = self.rect(target.clone());
        let at = rect.center();
        let window = self.harness.ctx.content_rect();
        if !window.contains(at) {
            panic!("{target:?} at {rect:?} is outside the window {window:?}");
        }
        at
    }
}

// The keyboard.
impl<A: 'static> Driver<A> {
    /// TODO(prose): presses and releases `keys`, with their modifiers held.
    pub fn press(&mut self, keys: impl Into<Press>) -> &mut Self {
        let Press(KeyboardShortcut {
            logical_key,
            modifiers,
        }) = keys.into();
        if modifiers.is_none() {
            self.harness.key_press(logical_key);
        } else {
            self.harness.key_press_modifiers(modifiers, logical_key);
        }
        self.harness.step();
        self.after()
    }

    /// TODO(prose): types `text` into what has focus.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        self.send([Event::Text(text.to_owned())]);
        self.after()
    }

    /// TODO(prose): presses the platform's select-all chord.
    pub fn select_all(&mut self) -> &mut Self {
        self.press(KeyboardShortcut::new(Modifiers::COMMAND, EguiKey::A))
    }

    /// TODO(prose): presses Tab until `target` has focus, or holds the widget that has.
    ///
    /// # Panics
    ///
    /// If one Tab per node of the frame does not reach it.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn tab_to(&mut self, target: impl Into<Target>) -> &mut Self {
        let target = target.into();
        let lap = self.frame().len();
        for _ in 0..=lap {
            if self.has_focus(&target) {
                return self;
            }
            self.press(EguiKey::Tab);
        }
        panic!(
            "Tab never reached {target:?}; focus is on {:?}",
            self.focused()
        );
    }

    #[track_caller]
    fn has_focus(&self, target: &Target) -> bool {
        let Some(id) = self.id_of(target.clone()) else {
            return false;
        };
        self.shell().focus_within(id)
    }
}

// The window and the platform.
impl<A: 'static> Driver<A> {
    /// TODO(prose): gives the window the platform's focus, or takes it away, as
    /// switching to another window does.
    pub fn window_focus(&mut self, focus: WindowFocus) -> &mut Self {
        let focused = focus == WindowFocus::Focused;
        self.harness.input_mut().focused = focused;
        self.send([Event::WindowFocused(focused)]);
        self.after()
    }

    /// TODO(prose): files dragged over the window from outside.
    pub fn hover_files(&mut self, files: &[PathBuf]) -> &mut Self {
        self.harness.input_mut().hovered_files = files
            .iter()
            .map(|path| egui::HoveredFile {
                path: Some(path.clone()),
                mime: String::new(),
            })
            .collect();
        self.harness.step();
        self.after()
    }

    /// TODO(prose): files dropped on the window from outside.
    pub fn drop_files(&mut self, files: &[PathBuf]) -> &mut Self {
        let input = self.harness.input_mut();
        input.hovered_files.clear();
        input.dropped_files = files
            .iter()
            .map(|path| Arc::new(DroppedPath(path.clone())) as egui::DroppedFileHandle)
            .collect();
        self.harness.step();
        self.after()
    }

    /// TODO(prose): resizes the window to `size` points.
    pub fn resize(&mut self, size: Vec2) -> &mut Self {
        self.harness.set_size(size);
        self.harness.step();
        self.after()
    }

    /// TODO(prose): switches the theme.
    pub fn set_theme(&mut self, theme: Theme) -> &mut Self {
        self.harness.ctx.set_theme(theme);
        self.harness.step();
        self.after()
    }

    /// TODO(prose): asks egui to focus `target`, as an application's code does.
    ///
    /// # Panics
    ///
    /// If `target` is not an identified node.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn focus(&mut self, target: impl Into<Target>) -> &mut Self {
        let target = target.into();
        let Some(id) = self.id_of(target.clone()) else {
            panic!("{target:?} is not an identified node, so it cannot take focus");
        };
        self.harness
            .ctx
            .memory_mut(|memory| memory.request_focus(id));
        self.harness.step();
        self.after()
    }

    /// TODO(prose): queues `event` and runs a frame for it.
    pub fn event(&mut self, event: Event) -> &mut Self {
        self.send([event]);
        self.after()
    }
}

#[derive(Debug)]
struct DroppedPath(PathBuf);

impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &std::path::Path {
        &self.0
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }

    #[cfg(target_arch = "wasm32")]
    fn bytes_async(&self) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + '_>> {
        Box::pin(std::future::ready(Ok(Vec::new())))
    }
}

// The app.
impl<A: 'static> Driver<A> {
    /// TODO(prose): the app.
    pub fn app(&self) -> &A {
        &self.harness.state().app
    }

    /// TODO(prose): the app, to change; the next frame shows the change.
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.harness.state_mut().app
    }

    /// TODO(prose): the state of the driver's layout.
    pub fn shell(&self) -> Shell {
        self.harness.state().layout.shell(&self.harness.ctx)
    }

    /// TODO(prose): the driver's layout: to install a clipboard, or to ask
    /// what it keeps.
    pub fn layout(&self) -> &LayoutState {
        &self.harness.state().layout
    }

    /// TODO(prose): swaps the layout's style, as [`LayoutState::set_style`]; the next frame uses it.
    pub fn set_style(&mut self, style: Themed<Arc<Style>>) -> &mut Self {
        self.harness.state_mut().layout.set_style(style);
        self
    }

    /// TODO(prose): the egui context.
    pub fn ctx(&self) -> &egui::Context {
        &self.harness.ctx
    }

    /// TODO(prose): whether motion runs; it applies from the next frame.
    pub fn set_motion(&mut self, motion: Animation) -> &mut Self {
        self.harness.state_mut().motion = motion;
        self
    }

    /// TODO(prose): the kittest harness: its queries by role and label, its
    /// accessibility check, a screenshot.
    pub fn harness(&self) -> &Harness<'static, Stage<A>> {
        &self.harness
    }

    /// TODO(prose): the kittest harness, to step or send input of its own.
    pub fn harness_mut(&mut self) -> &mut Harness<'static, Stage<A>> {
        &mut self.harness
    }
}

// Queries.
impl<A: 'static> Driver<A> {
    /// TODO(prose): the last frame drawn: [`at`](PlacedFrame::at),
    /// [`text`](super::Query::text), the overlays, the menus.
    ///
    /// # Panics
    ///
    /// If no frame was drawn yet, which [`build`](DriverBuilder::build)'s
    /// first frame rules out.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn frame(&self) -> &PlacedFrame {
        match &self.harness.state().frame {
            Some(frame) => frame,
            None => panic!("the driver's layout has not drawn a frame"),
        }
    }

    /// TODO(prose): the node declared with `key` at the top of the view, as a target.
    pub fn path<K: Key>(&self, key: K) -> Path {
        let root = self.harness.state().layout.root();
        Path {
            steps: vec![(format!("{key:?}"), child_id(root, &key))],
        }
    }

    /// TODO(prose): where `target` is in the last frame.
    ///
    /// # Panics
    ///
    /// If it is not there, or more than one node matches; the message lists
    /// the candidates.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn rect(&self, target: impl Into<Target>) -> Rect {
        match self.resolve(&target.into()) {
            Ok(rect) => rect,
            Err(miss) => panic!("{miss}"),
        }
    }

    /// TODO(prose): why the node `target` names has the value it has for `property`:
    /// the class that set it and the classes it beat. It pins the node and runs a
    /// frame, so the build keeps what lost, then unpins it. `None` when no class set
    /// the property. Debug builds only.
    ///
    /// # Panics
    ///
    /// If `target` is not one node.
    #[cfg(debug_assertions)]
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn why(
        &mut self,
        target: impl Into<Target>,
        property: crate::Property,
    ) -> Option<crate::Why> {
        let target = target.into();
        let path = match resolve_node(self.frame(), &target) {
            Ok(index) => self
                .frame()
                .nodes
                .get(index)
                .map(|node| node.path().to_owned()),
            Err(miss) => panic!("{miss}"),
        }?;
        crate::Debug::pin(self.ctx(), Some(path.clone()));
        self.frames(1);
        let why = self.frame().why(&path, property);
        crate::Debug::pin(self.ctx(), None);
        why
    }

    /// TODO(prose): [`rect`](Self::rect), or `None` when no one node matches.
    pub fn find(&self, target: impl Into<Target>) -> Option<Rect> {
        self.resolve(&target.into()).ok()
    }

    /// TODO(prose): whether `target` is one node of the last frame.
    pub fn exists(&self, target: impl Into<Target>) -> bool {
        self.find(target).is_some()
    }

    /// TODO(prose): what has keyboard focus, if anything does.
    pub fn focused(&self) -> Option<Focused> {
        let focused = self.harness.ctx.memory(egui::Memory::focused)?;
        let frame = self.harness.state().frame.as_ref()?;
        if let Some(node) = frame.node(focused) {
            return Some(Focused::Node(node.path().to_owned()));
        }
        let shell = self.shell();
        let within = frame
            .nodes()
            .filter(|node| node.id().is_some_and(|id| shell.focus_within(id)))
            .last()
            .map(|node| node.path().to_owned());
        Some(Focused::Widget { within })
    }

    /// TODO(prose): what the last frame painted.
    pub fn painted(&self) -> Painted<'_> {
        let mut texts = Vec::new();
        for clipped in &self.harness.output().shapes {
            collect_texts(&clipped.shape, clipped.clip_rect, &mut texts);
        }
        Painted { texts }
    }

    /// TODO(prose): the reports the context keeps, oldest first.
    pub fn reports(&self) -> Vec<Report> {
        super::reports(&self.harness.ctx)
    }

    /// TODO(prose): the first report of `problem`, claimed: the check when the driver is
    /// dropped lets every report of `problem` pass.
    ///
    /// # Panics
    ///
    /// If there is no report of `problem`.
    #[track_caller]
    pub fn expect_reported(&mut self, problem: Problem) -> Report {
        let report = super::assert_reported(&self.harness.ctx, problem);
        self.claimed.push(problem);
        report
    }

    /// TODO(prose): each text the last frame painted that does not [fit](PaintedText::fits),
    /// with the node that shows it: text that wraps, is cut short, or runs
    /// out of its clip. It reports and does not fail: a person reads the
    /// findings.
    pub fn text_findings(&self) -> Vec<TextFinding> {
        let frame = self.harness.state().frame.as_ref();
        self.painted()
            .texts()
            .filter(|text| !text.fits())
            .map(|text| TextFinding {
                text: text.text().to_owned(),
                node: frame.and_then(|frame| shown_by(frame, &text)),
                rect: text.rect(),
                clip: text.clip(),
                rows: text.rows(),
                elided: text.elided(),
            })
            .collect()
    }

    fn resolve(&self, target: &Target) -> Result<Rect, String> {
        match target {
            Target::Point(at) => Ok(Rect::from_min_size(*at, Vec2::ZERO)),
            Target::Role(role, name) => self.by_role(*role, name),
            target => {
                let frame = self.frame();
                let index = resolve_node(frame, target)?;
                frame
                    .nodes
                    .get(index)
                    .map(crate::PlacedNode::rect)
                    .ok_or_else(|| format!("{target:?} is not a node"))
            }
        }
    }

    fn by_role(&self, role: Role, name: &str) -> Result<Rect, String> {
        let found: Vec<Rect> = self
            .harness
            .query_all_by_role_and_label(role, name)
            .map(|node| node.rect())
            .collect();
        match found[..] {
            [rect] => Ok(rect),
            [] => Err(format!("no {role:?} named {name:?}")),
            _ => Err(format!("{} nodes are {role:?} named {name:?}", found.len())),
        }
    }

    /// The id of the node `target` names, if it is one identified node.
    fn id_of(&self, target: Target) -> Option<Id> {
        match target {
            Target::Id(id) => Some(id),
            Target::Point(_) | Target::Role(..) => None,
            target => {
                let frame = self.harness.state().frame.as_ref()?;
                frame.nodes.get(resolve_node(frame, &target).ok()?)?.id()
            }
        }
    }
}

impl<A: 'static> Drop for Driver<A> {
    fn drop(&mut self) {
        if self.reports != Reports::Guarded || std::thread::panicking() {
            return;
        }
        let unclaimed: Vec<Report> = self
            .reports()
            .into_iter()
            .filter(|report| !self.claimed.contains(&report.problem))
            .collect();
        unreported(&unclaimed);
    }
}

#[track_caller]
#[expect(clippy::panic, reason = "an assertion fails the test")]
fn unreported(unclaimed: &[Report]) {
    if !unclaimed.is_empty() {
        panic!(
            "the driver's view raised reports it did not claim with expect_reported: {unclaimed:#?}"
        );
    }
}

/// The index of the one node `target` names in `frame`.
fn resolve_node(frame: &PlacedFrame, target: &Target) -> Result<usize, String> {
    match target {
        Target::Path(path) => {
            let mut query = frame.root_query();
            for (readable, id) in &path.steps {
                query = query
                    .child_by_id(Some(*id), readable)
                    .ok_or_else(|| query.missing(readable))?;
            }
            query
                .index
                .ok_or_else(|| format!("{} is a scope, not a node", query.path))
        }
        Target::Suffix(suffix) => by_suffix(frame, suffix),
        Target::Text(text) => {
            let query = frame.root_query().find_text(text)?;
            query.index.ok_or_else(|| format!("no node shows {text:?}"))
        }
        Target::Id(id) => frame
            .by_id
            .get(id)
            .copied()
            .ok_or_else(|| format!("no node has the id {id:?}")),
        Target::Point(_) | Target::Role(..) => Err(format!("{target:?} names no node")),
    }
}

fn by_suffix(frame: &PlacedFrame, suffix: &str) -> Result<usize, String> {
    let matching: Vec<(usize, &str)> = frame
        .nodes()
        .enumerate()
        .map(|(index, node)| (index, node.path()))
        .filter(|(_, path)| path.ends_with(suffix))
        .collect();
    let exact: Vec<&(usize, &str)> = matching
        .iter()
        .filter(|(_, path)| *path == suffix)
        .collect();
    match (&exact[..], &matching[..]) {
        ([(index, _)], _) | ([], [(index, _)]) => Ok(*index),
        ([], []) => Err(format!(
            "no node's path ends with {suffix:?}; the nodes are: {}",
            list(frame.nodes().map(crate::PlacedNode::path))
        )),
        _ => Err(format!(
            "more than one node's path ends with {suffix:?}: {}",
            list(matching.iter().map(|(_, path)| *path))
        )),
    }
}

/// The node of `frame` that shows `text`: one whose text is the painted
/// text, holding where it was painted.
fn shown_by(frame: &PlacedFrame, text: &PaintedText<'_>) -> Option<String> {
    let at = text.rect().center();
    frame
        .nodes()
        .zip(&frame.prims)
        .filter(|(node, prim)| {
            prim.as_ref()
                .and_then(crate::prim::Prim::text)
                .is_some_and(|shown| shown == text.text())
                && node.rect().contains(at)
        })
        .map(|(node, _)| node.path().to_owned())
        .last()
}

fn collect_texts<'d>(shape: &'d Shape, clip: Rect, into: &mut Vec<PaintedText<'d>>) {
    match shape {
        Shape::Vec(shapes) => {
            for shape in shapes {
                collect_texts(shape, clip, into);
            }
        }
        Shape::Text(text) => into.push(PaintedText {
            galley: &text.galley,
            pos: text.pos,
            clip,
            color: text.override_text_color.unwrap_or(text.fallback_color),
        }),
        _ => {}
    }
}

/// TODO(prose): the texts a frame painted, in paint order.
pub struct Painted<'d> {
    texts: Vec<PaintedText<'d>>,
}

impl<'d> Painted<'d> {
    /// TODO(prose): every text, in paint order.
    pub fn texts(&self) -> impl Iterator<Item = PaintedText<'d>> + '_ {
        self.texts.iter().copied()
    }

    /// TODO(prose): the first text painted that is exactly `text`.
    pub fn text(&self, text: &str) -> Option<PaintedText<'d>> {
        self.texts().find(|painted| painted.text() == text)
    }

    /// TODO(prose): every text painted that is exactly `text`, in paint order.
    pub fn all(&self, text: &str) -> Vec<PaintedText<'d>> {
        self.texts()
            .filter(|painted| painted.text() == text)
            .collect()
    }

    /// TODO(prose): whether `text` was painted.
    pub fn shows(&self, text: &str) -> bool {
        self.text(text).is_some()
    }

    /// TODO(prose): the first text painted that is exactly `text`.
    ///
    /// # Panics
    ///
    /// If none is, listing what was painted.
    #[track_caller]
    #[expect(clippy::panic, reason = "an assertion fails the test")]
    pub fn expect(&self, text: &str) -> PaintedText<'d> {
        self.text(text).unwrap_or_else(|| {
            panic!(
                "{text:?} was not painted; the texts painted are: {}",
                list(self.texts().map(|painted| format!("{:?}", painted.text())))
            )
        })
    }
}

/// TODO(prose): a text the frame painted: its galley, where, under which clip, in which
/// colour.
#[derive(Clone, Copy)]
pub struct PaintedText<'d> {
    galley: &'d Arc<egui::Galley>,
    pos: Pos2,
    clip: Rect,
    color: Color32,
}

impl<'d> PaintedText<'d> {
    /// TODO(prose): the text.
    pub fn text(&self) -> &str {
        self.galley.text()
    }

    /// TODO(prose): where the text was painted, in screen points.
    pub fn rect(&self) -> Rect {
        self.galley.rect.translate(self.pos.to_vec2())
    }

    /// TODO(prose): what it was clipped to.
    pub fn clip(&self) -> Rect {
        self.clip
    }

    /// TODO(prose): the colour it was painted in.
    pub fn color(&self) -> Color32 {
        self.color
    }

    /// TODO(prose): the laid-out text.
    pub fn galley(&self) -> &'d egui::Galley {
        self.galley
    }

    /// TODO(prose): where the galley was painted: its origin, in screen points.
    pub fn pos(&self) -> Pos2 {
        self.pos
    }

    /// TODO(prose): how many rows the text was laid out in.
    pub fn rows(&self) -> usize {
        self.galley.rows.len()
    }

    /// TODO(prose): whether it was cut short with an ellipsis.
    pub fn elided(&self) -> bool {
        self.galley.elided
    }

    /// TODO(prose): where `part` of the text was painted, if it is part of it and on one
    /// row.
    pub fn part_rect(&self, part: &str) -> Option<Rect> {
        let text = self.text();
        let byte = text.find(part)?;
        let start = text.get(..byte)?.chars().count();
        let end = start + part.chars().count();
        let first = self.galley.pos_from_cursor(egui::text::CCursor {
            prefer_next_row: true,
            ..egui::text::CCursor::new(start)
        });
        let last = self.galley.pos_from_cursor(egui::text::CCursor::new(end));
        ((first.center().y - last.center().y).abs() < 0.5).then(|| {
            Rect::from_min_max(first.min, egui::pos2(last.max.x, first.max.y))
                .translate(self.pos.to_vec2())
        })
    }

    /// TODO(prose): one row, not elided, and inside its clip rect.
    pub fn fits(&self) -> bool {
        self.rows() <= 1 && !self.elided() && self.clip.contains_rect(self.rect())
    }
}

/// TODO(prose): a painted text that does not [fit](PaintedText::fits), from
/// [`Driver::text_findings`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextFinding {
    /// TODO(prose): the text.
    pub text: String,
    /// TODO(prose): the readable path of the node that shows it, when one is found.
    pub node: Option<String>,
    /// TODO(prose): where it was painted.
    pub rect: Rect,
    /// TODO(prose): what it was clipped to.
    pub clip: Rect,
    /// TODO(prose): the rows it was laid out in.
    pub rows: usize,
    /// TODO(prose): whether it was cut short with an ellipsis.
    pub elided: bool,
}

impl TextFinding {
    /// TODO(prose): whether it wraps onto more than one row.
    pub fn wraps(&self) -> bool {
        self.rows > 1
    }

    /// TODO(prose): whether it runs out of its clip rect.
    pub fn overruns(&self) -> bool {
        !self.clip.contains_rect(self.rect)
    }
}
