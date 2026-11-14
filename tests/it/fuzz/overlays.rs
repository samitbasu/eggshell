//! Randomised sessions over every kind of overlay at once: a stock dropdown
//! menu with a submenu, a stock select, popovers (one anchored in a scroll
//! container), a modal dialog with a nested confirmation and a select in it,
//! tooltips, a context menu and a text field, driven by pointer, keys, scroll
//! and window size.
//!
//! After every frame the overlays must be inside their bounds, stacked as the
//! library stacks them, and no tooltip may take the pointer; after every
//! action (which ends with frames for the library to settle in) the
//! invariants of `docs/overlays.md` section 12 must hold, and the whole
//! session must be continuous.

use eggshell_ui::overlay::{
    Anchor, Anchored, Band, Distance, Focus, Modal, Outside, Placement, Shown,
};
use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::scroll::Axis;
use eggshell_ui::testing::{Points, leaves_bounds};
use eggshell_ui::{Builder, LayoutState, Placed, Scope, TextStyle, focus_within, tw};
use egui::{Color32, Event, Id, Key, LayerId, Modifiers, PointerButton, Pos2, Rect, Role, vec2};
use egui_kittest::Harness;
use proptest::prelude::*;

use super::{MAX_GRID_STEP, Session, config, verdict};
use crate::support::Bar;
use crate::sweep::IDLE;

const WINDOW: egui::Vec2 = vec2(400.0, 360.0);
const SMALLEST: egui::Vec2 = vec2(240.0, 220.0);
const LARGEST: egui::Vec2 = vec2(520.0, 440.0);
const EDGE_STEP: f32 = super::EDGE_STEP;
/// Frames are this far apart.
const STEP_DT: f32 = 0.05;
/// Frames that let a tooltip's delay run out and the motions of the
/// overlays finish.
const DWELL: usize = 12;
/// Frames after an action in which the library's one-frame lags (focus
/// returning, a cancel button's click) have happened.
const SETTLE: usize = 3;
/// The most frames an exit may take.
const EXIT_LIMIT: usize = 30;

const SCROLLER_ROWS: u32 = 10;
const ANCHOR_ROW: u32 = 1;

#[derive(Default)]
struct App {
    pick: usize,
    inner: usize,
    name: String,
    popover: bool,
    scroll_popover: bool,
    dialog: Shown,
    confirm: Shown,
    /// Presses that reached the page: a click on the pad or the page button.
    page_clicks: u32,
    chosen: u32,
}

fn dialog_box() -> eggshell_ui::Tw {
    tw().flex()
        .flex_col()
        .gap(3.0)
        .p(4.0)
        .w_full()
        .max_w_px(300.0)
        .bg(Color32::WHITE)
        .border(1.0)
        .border_color(Color32::GRAY)
        .rounded_px(8.0)
}

fn popover_box() -> eggshell_ui::Tw {
    tw().flex()
        .flex_col()
        .gap(2.0)
        .p(3.0)
        .w_px(150.0)
        .bg(Color32::WHITE)
        .border(1.0)
        .border_color(Color32::GRAY)
        .rounded_px(6.0)
}

fn popover<'f>(
    s: &mut Scope<'_, 'f, App>,
    key: &'static str,
    anchor: &'static str,
    open: fn(&mut App) -> &mut bool,
) {
    let anchor = s.id_of(anchor);
    s.overlay(key)
        .anchor(Anchor::Node(anchor))
        .placement(Placement::below().gap(Distance::new(4.0)))
        .role(Role::Dialog)
        .named("Popover")
        .on_dismiss(move |app: &mut App| *open(app) = false)
        .children(|s| {
            s.div_id("box", popover_box(), |s| {
                s.text(tw(), "A popover with a field and a button.");
                s.button("popover-button", "Choose")
                    .on_click(|app: &mut App| app.chosen += 1);
            });
        });
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(3.0).p(3.0).w_full().h_full(),
        |s| {
            s.div_id("Top", tw().flex().flex_wrap().gap(2.0), |s| {
                s.dropdown_menu("menu", "Menu").items(|m| {
                    m.item(("item", 0), "Copy")
                        .on_click(|app: &mut App| app.chosen += 1);
                    m.item(("item", 1), "Paste")
                        .enablement(Enablement::Disabled)
                        .on_click(|_| {});
                    m.separator();
                    m.checkbox_item(("item", 2), "Grid", Mark::Off)
                        .on_change(|app: &mut App, _| app.chosen += 1);
                    m.submenu("more", "More", |m| {
                        m.item(("item", 3), "Deep")
                            .on_click(|app: &mut App| app.chosen += 1);
                        m.item(("item", 4), "Deeper")
                            .on_click(|app: &mut App| app.chosen += 1);
                    });
                    m.item(("item", 5), "Open dialog")
                        .on_click(|app: &mut App| app.dialog = Shown::Open);
                    m.item(("item", 6), "Delete")
                        .destructive()
                        .on_click(|app: &mut App| app.chosen += 1);
                });
                s.select("pick", app.pick, (0..6).map(|n| (n, format!("Option {n}"))))
                    .named("Pick")
                    .tooltip("Pick an option")
                    .classes(tw().w_px(110.0))
                    .on_change(|app: &mut App, value| app.pick = value);
                s.button("popover", "Popover")
                    .tooltip("Opens a popover")
                    .on_click(|app: &mut App| app.popover = !app.popover);
                s.button("dialog", "Dialog")
                    .tooltip("Opens the dialog")
                    .on_click(|app: &mut App| app.dialog = Shown::Open);
                s.button("page", "Page")
                    .tooltip("A page button")
                    .on_click(|app: &mut App| app.page_clicks += 1);
                if app.popover {
                    popover(s, "popover-overlay", "popover", |app| &mut app.popover);
                }
            });
            s.text_field("name", app.name.clone())
                .named("Name")
                .classes(tw().w_px(160.0))
                .on_change(|app: &mut App, text| app.name = text);
            s.scroll("scroller", tw().w_px(220.0).h_px(90.0))
                .axis(Axis::Y)
                .children(|s| {
                    for row in 0..SCROLLER_ROWS {
                        if row == ANCHOR_ROW {
                            s.button("anchor", "Anchor")
                                .tooltip("Anchors a popover")
                                .on_click(|app: &mut App| app.scroll_popover = !app.scroll_popover);
                            if app.scroll_popover {
                                popover(s, "scroll-popover", "anchor", |app| {
                                    &mut app.scroll_popover
                                });
                            }
                        } else {
                            s.text(tw().h_px(24.0), format!("Row {row}"));
                        }
                    }
                });
            s.div_id(
                "canvas",
                tw().ml(8.0).w_px(140.0).h_px(60.0).bg(Color32::LIGHT_GRAY),
                |_| {},
            )
            .context_menu(|m| {
                m.item(("item", 10), "Cut")
                    .on_click(|app: &mut App| app.chosen += 1);
                m.submenu("context-more", "More", |m| {
                    m.item(("item", 11), "Deep")
                        .on_click(|app: &mut App| app.chosen += 1);
                });
                m.item(("item", 12), "Paste")
                    .on_click(|app: &mut App| app.chosen += 1);
            });
            s.div_id(
                "pad",
                tw().sense_click()
                    .grow()
                    .min_h_px(20.0)
                    .bg(Color32::LIGHT_BLUE),
                |_| {},
            )
            .on_click(|app: &mut App| app.page_clicks += 1);
        },
    );
    dialog(b, app);
}

fn dialog(b: &mut Builder<'_, '_, App>, app: &App) {
    b.overlay("dialog-overlay")
        .modal(Modal::Center)
        .shown(app.dialog)
        .role(Role::Dialog)
        .named("Dialog")
        .on_dismiss(|app: &mut App| app.dialog = Shown::Closed)
        .children(|s| {
            s.div_id("box", dialog_box(), |s| {
                s.text(tw(), "A dialog");
                s.select("inner", app.inner, (0..4).map(|n| (n, format!("Role {n}"))))
                    .named("Inner")
                    .on_change(|app: &mut App, value| app.inner = value);
                s.div(tw().flex().justify_between(), |s| {
                    s.button("delete", "Delete")
                        .on_click(|app: &mut App| app.confirm = Shown::Open);
                    s.button("cancel", "Cancel")
                        .cancel_action()
                        .on_click(|app: &mut App| app.dialog = Shown::Closed);
                    s.button("save", "Save")
                        .default_action()
                        .on_click(|app: &mut App| app.dialog = Shown::Closed);
                });
                confirm(s, app);
            });
        });
}

fn confirm(s: &mut Scope<'_, '_, App>, app: &App) {
    s.overlay("confirm-overlay")
        .modal(Modal::Center)
        .shown(app.confirm)
        .role(Role::AlertDialog)
        .named("Sure?")
        .dismiss_on(eggshell_ui::overlay::DismissOn::EscOnly)
        .on_dismiss(|app: &mut App| app.confirm = Shown::Closed)
        .children(|s| {
            s.div_id("box", dialog_box().max_w_px(240.0), |s| {
                s.text(tw(), "Are you sure?");
                s.div(tw().flex().justify_end().gap(2.0), |s| {
                    s.button("no", "Cancel")
                        .cancel_action()
                        .on_click(|app: &mut App| app.confirm = Shown::Closed);
                    s.button("yes", "Continue")
                        .default_action()
                        .on_click(|app: &mut App| {
                            app.confirm = Shown::Closed;
                            app.dialog = Shown::Closed;
                        });
                });
            });
        });
}

/// An overlay as the last frame placed it.
#[derive(Clone, Debug)]
struct Open {
    id: Id,
    path: String,
    rect: Rect,
    layer: LayerId,
    band: Band,
    parent: Option<Id>,
    stack_index: usize,
    exiting: bool,
    modal: bool,
    focus: Focus,
    outside: Outside,
    bounds: Rect,
    anchored: Anchored,
    anchor_ids: Vec<Id>,
}

impl Open {
    /// Open for the user: not leaving, and not a tooltip (the library opens
    /// and closes those itself).
    fn is_open(&self) -> bool {
        !self.exiting && self.band != Band::Tooltip
    }

    fn is_menu(&self) -> bool {
        self.is_open() && self.parent.is_none() && self.outside == Outside::Swallow
    }
}

#[derive(Clone, Debug, Default)]
struct Seen {
    overlays: Vec<Open>,
    nodes: Vec<(String, Id, Rect)>,
}

impl Seen {
    fn of(placed: &Placed<App>) -> Self {
        let nodes: Vec<(String, Id, Rect)> = placed
            .nodes()
            .filter_map(|node| Some((node.path().to_owned(), node.id()?, node.rect())))
            .collect();
        let overlays = placed
            .overlays()
            .iter()
            .map(|overlay| Open {
                id: overlay.id(),
                path: overlay.path().to_owned(),
                rect: overlay.rect(),
                layer: overlay.layer(),
                band: overlay.band(),
                parent: overlay.nested_in(),
                stack_index: overlay.stack_index(),
                exiting: overlay.is_exiting(),
                modal: overlay.modal().is_some(),
                focus: overlay.focus(),
                outside: overlay.outside(),
                bounds: overlay.bounds(),
                anchored: overlay.anchored(),
                anchor_ids: nodes
                    .iter()
                    .filter(|(_, _, rect)| rects_agree(*rect, overlay.anchor()))
                    .map(|(_, id, _)| *id)
                    .collect(),
            })
            .collect();
        Self { overlays, nodes }
    }

    fn open(&self) -> impl Iterator<Item = &Open> {
        self.overlays.iter().filter(|overlay| overlay.is_open())
    }

    /// The open overlays that take part in input: seen, and not leaving.
    fn attached_open(&self) -> usize {
        self.open()
            .filter(|overlay| overlay.anchored == Anchored::Attached)
            .count()
    }

    fn node(&self, last: &str) -> Option<(Id, Rect)> {
        let suffix = format!("/{last}");
        self.nodes
            .iter()
            .find(|(path, _, _)| path.ends_with(&suffix))
            .map(|(_, id, rect)| (*id, *rect))
    }

    /// The items of the overlays that are open, in paint order.
    fn items(&self) -> Vec<Rect> {
        self.nodes
            .iter()
            .filter(|(path, _, rect)| {
                let last = path.rsplit('/').next().unwrap_or_default();
                let item = last.starts_with("(\"item\"")
                    || last.starts_with("Option(")
                    || last == "\"more\""
                    || last == "\"context-more\"";
                item && self
                    .open()
                    .any(|overlay| overlay.rect.contains_rect(rect.shrink(0.1)))
            })
            .map(|(_, _, rect)| *rect)
            .collect()
    }

    fn modal_is_up(&self) -> bool {
        self.overlays.iter().any(|overlay| overlay.modal)
    }
}

fn rects_agree(a: Rect, b: Rect) -> bool {
    (a.min - b.min).abs().max_elem() < 0.5 && (a.max - b.max).abs().max_elem() < 0.5
}

struct Rig {
    app: App,
    layout: LayoutState,
    bar: Bar,
    seen: Seen,
}

fn rig_harness() -> Harness<'static, Rig> {
    let tokens = eggshell_ui_shadcn::Tokens::for_theme(egui::Theme::Light);
    let base = TextStyle::new(tokens.text_role(), tokens.foreground);
    let rig = Rig {
        app: App::default(),
        layout: LayoutState::new().with_base_text(base.clone()),
        bar: Bar::new(base),
        seen: Seen::default(),
    };
    Harness::builder()
        .with_size(WINDOW)
        .with_step_dt(STEP_DT)
        .build_ui_state(
            |ui, rig: &mut Rig| {
                let area = ui.max_rect();
                let Rig {
                    app,
                    layout,
                    bar,
                    seen,
                } = rig;
                if eggshell_ui_shadcn::tokens(ui.ctx()).is_none() {
                    eggshell_ui_shadcn::Shadcn::apply_both(ui.ctx());
                }
                let background = eggshell_ui_shadcn::Shadcn::of(ui.ctx()).tokens.background;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, background);
                bar.show(ui);
                let placed = layout.build(ui.ctx(), |b| view(b, &*app)).layout(area);
                *seen = Seen::of(&placed);
                placed.draw(app, ui);
            },
            rig,
        )
}

#[derive(Clone, Debug)]
enum Action {
    OpenMenu,
    OpenSubmenu,
    OpenSelect,
    OpenPopover,
    OpenAnchorPopover,
    OpenContextMenu(u32, u32),
    OpenModal,
    OpenConfirm,
    HoverItem(usize),
    ClickItem(usize),
    HoverTrigger(usize),
    PressAt(u32, u32),
    ReleaseAt(u32, u32),
    ClickAt(u32, u32),
    Esc,
    Tab,
    ShiftTab,
    ArrowDown,
    ArrowUp,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    Enter,
    ScrollAnchorContainer(i32),
    ResizeWindow(u32, u32),
    Idle(usize),
}

const TRIGGERS: [&str; 6] = [
    "\"menu\"",
    "\"pick\"",
    "\"popover\"",
    "\"dialog\"",
    "\"page\"",
    "\"anchor\"",
];

fn action() -> impl Strategy<Value = Action> {
    let percent = || 0u32..=100;
    prop_oneof![
        4 => Just(Action::OpenMenu),
        3 => Just(Action::OpenSubmenu),
        2 => Just(Action::OpenSelect),
        2 => Just(Action::OpenPopover),
        2 => Just(Action::OpenAnchorPopover),
        2 => (percent(), percent()).prop_map(|(x, y)| Action::OpenContextMenu(x, y)),
        3 => Just(Action::OpenModal),
        2 => Just(Action::OpenConfirm),
        4 => (0usize..8).prop_map(Action::HoverItem),
        4 => (0usize..8).prop_map(Action::ClickItem),
        3 => (0..TRIGGERS.len()).prop_map(Action::HoverTrigger),
        3 => (percent(), percent()).prop_map(|(x, y)| Action::PressAt(x, y)),
        3 => (percent(), percent()).prop_map(|(x, y)| Action::ReleaseAt(x, y)),
        3 => (percent(), percent()).prop_map(|(x, y)| Action::ClickAt(x, y)),
        6 => Just(Action::Esc),
        3 => Just(Action::Tab),
        2 => Just(Action::ShiftTab),
        4 => Just(Action::ArrowDown),
        3 => Just(Action::ArrowUp),
        2 => Just(Action::ArrowLeft),
        2 => Just(Action::ArrowRight),
        1 => Just(Action::Home),
        1 => Just(Action::End),
        2 => Just(Action::Enter),
        3 => (-200i32..=200).prop_map(Action::ScrollAnchorContainer),
        2 => (
            SMALLEST.x as u32..=LARGEST.x as u32,
            SMALLEST.y as u32..=LARGEST.y as u32
        )
            .prop_map(|(w, h)| Action::ResizeWindow(w, h)),
        2 => (1usize..=8).prop_map(Action::Idle),
    ]
}

fn overlay_session() -> impl Strategy<Value = Vec<Action>> {
    prop::collection::vec(action(), 1..=16)
}

fn ensure(holds: bool, message: impl FnOnce() -> String) -> Result<(), TestCaseError> {
    if holds {
        Ok(())
    } else {
        Err(TestCaseError::fail(message()))
    }
}

/// A user at the app: where the pointer is, whether a button is down, and
/// what the last frames showed.
struct Desk {
    harness: Harness<'static, Rig>,
    at: Pos2,
    down: bool,
    size: egui::Vec2,
    /// The widget that had focus when each overlay opened, as the library
    /// stores it to give it back.
    openers: Vec<(Id, Option<Id>)>,
    strictness: Strictness,
    session: Session,
}

/// Which invariants a session judges. `Strict` adds two that the library's
/// documented rules leave open (see the ignored test): where focus is after
/// each action, and whether a menu hidden because its anchor left the window
/// still counts as open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strictness {
    Decided,
    Strict,
}

impl Desk {
    fn new(strictness: Strictness) -> Self {
        let mut harness = rig_harness();
        harness.run_steps(4);
        let session = Session::start(&harness.ctx);
        let mut desk = Self {
            harness,
            at: Pos2::ZERO,
            down: false,
            size: WINDOW,
            openers: Vec::new(),
            strictness,
            session,
        };
        desk.harness.step();
        desk
    }

    fn seen(&self) -> &Seen {
        &self.harness.state().seen
    }

    fn app(&self) -> &App {
        &self.harness.state().app
    }

    fn focused(&self) -> Option<Id> {
        self.harness.ctx.memory(|memory| memory.focused())
    }

    /// One frame, and the invariants that hold in every frame.
    fn step(&mut self) -> Result<(), TestCaseError> {
        let before = self.focused();
        let previous: Vec<Id> = self.seen().overlays.iter().map(|o| o.id).collect();
        self.harness.step();
        for overlay in &self.harness.state().seen.overlays {
            if !previous.contains(&overlay.id) {
                self.openers.retain(|(id, _)| *id != overlay.id);
                self.openers.push((overlay.id, before));
            }
        }
        self.in_bounds()?;
        self.stacked()?;
        self.tooltips_leave_the_pointer_alone()
    }

    fn steps(&mut self, count: usize) -> Result<(), TestCaseError> {
        (0..count).try_for_each(|_| self.step())
    }

    fn in_bounds(&self) -> Result<(), TestCaseError> {
        for overlay in &self.seen().overlays {
            ensure(!leaves_bounds(overlay.rect, overlay.bounds), || {
                format!(
                    "overlay {} is at {:?}, outside the bounds {:?} that can hold it",
                    overlay.path, overlay.rect, overlay.bounds
                )
            })?;
        }
        Ok(())
    }

    /// egui's layer order has the drawn overlays in the library's stack
    /// order within a band, and every nested overlay over its parent.
    fn stacked(&self) -> Result<(), TestCaseError> {
        let order: Vec<LayerId> = self
            .harness
            .ctx
            .memory(|memory| memory.layer_ids().collect());
        let rank = |layer: LayerId| order.iter().position(|candidate| *candidate == layer);
        let drawn: Vec<&Open> = self
            .seen()
            .overlays
            .iter()
            .filter(|overlay| overlay.anchored == Anchored::Attached)
            .collect();
        for overlay in &drawn {
            ensure(rank(overlay.layer).is_some(), || {
                format!(
                    "overlay {} is drawn but egui has no layer for it",
                    overlay.path
                )
            })?;
            let Some(parent_id) = overlay.parent else {
                continue;
            };
            let parent = self
                .seen()
                .overlays
                .iter()
                .find(|candidate| candidate.id == parent_id);
            let parent = parent.ok_or_else(|| {
                TestCaseError::fail(format!(
                    "overlay {} is open without its parent",
                    overlay.path
                ))
            })?;
            ensure(overlay.stack_index > parent.stack_index, || {
                format!(
                    "overlay {} (stack {}) is not above its parent {} (stack {})",
                    overlay.path, overlay.stack_index, parent.path, parent.stack_index
                )
            })?;
            if parent.anchored == Anchored::Attached && parent.band == overlay.band {
                ensure(rank(overlay.layer) > rank(parent.layer), || {
                    format!(
                        "egui draws overlay {} below its parent {}",
                        overlay.path, parent.path
                    )
                })?;
            }
        }
        for band in [Band::Floating, Band::Popup, Band::Tooltip] {
            let mut in_band: Vec<&&Open> = drawn
                .iter()
                .filter(|overlay| overlay.band == band)
                .collect();
            in_band.sort_by_key(|overlay| overlay.stack_index);
            for pair in in_band.windows(2) {
                ensure(rank(pair[0].layer) < rank(pair[1].layer), || {
                    format!(
                        "egui draws {} (stack {}) over {} (stack {}) though it opened earlier",
                        pair[0].path, pair[0].stack_index, pair[1].path, pair[1].stack_index
                    )
                })?;
            }
        }
        Ok(())
    }

    fn tooltips_leave_the_pointer_alone(&self) -> Result<(), TestCaseError> {
        for tip in self
            .seen()
            .overlays
            .iter()
            .filter(|overlay| overlay.band == Band::Tooltip)
        {
            let r = tip.rect;
            for point in [
                r.center(),
                r.left_top() + vec2(1.0, 1.0),
                r.right_bottom() - vec2(1.0, 1.0),
            ] {
                let layer = self.harness.ctx.layer_id_at(point);
                ensure(layer != Some(tip.layer), || {
                    format!("the tooltip {} takes the pointer at {point:?}", tip.path)
                })?;
            }
        }
        Ok(())
    }

    fn at_percent(&self, x: u32, y: u32) -> Pos2 {
        Pos2::new(
            self.size.x * x as f32 / 100.0,
            self.size.y * y as f32 / 100.0,
        )
    }

    fn move_to(&mut self, to: Pos2) -> Result<(), TestCaseError> {
        self.at = to;
        self.harness.hover_at(to);
        self.step()
    }

    fn button(&mut self, button: PointerButton, pressed: bool) -> Result<(), TestCaseError> {
        self.harness.event(Event::PointerButton {
            pos: self.at,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
        self.step()
    }

    fn click(&mut self, to: Pos2, button: PointerButton) -> Result<(), TestCaseError> {
        self.move_to(to)?;
        self.steps(2)?;
        self.button(button, true)?;
        self.button(button, false)
    }

    fn click_node(&mut self, last: &str) -> Result<(), TestCaseError> {
        match self.seen().node(last) {
            Some((_, rect)) if !self.down => self.click(rect.center(), PointerButton::Primary),
            _ => Ok(()),
        }
    }

    fn key(&mut self, modifiers: Modifiers, key: Key) -> Result<(), TestCaseError> {
        self.harness.key_press_modifiers(modifiers, key);
        self.step()
    }

    fn item(&self, index: usize) -> Option<Pos2> {
        let items = self.seen().items();
        (!items.is_empty()).then(|| items[index % items.len()].center())
    }

    fn act(&mut self, action: &Action) -> Result<(), TestCaseError> {
        let modal_was_up = self.seen().modal_is_up();
        let clicks = self.app().page_clicks;
        let open_before = self.seen().attached_open();
        let escaped = matches!(action, Action::Esc).then(|| self.seen().clone());
        let focus_before = self.focused();
        self.perform(action)?;
        self.settle()?;
        ensure(!(modal_was_up && self.app().page_clicks != clicks), || {
            format!(
                "a press reached the page under a modal: page clicks {clicks} -> {}",
                self.app().page_clicks
            )
        })?;
        if let Some(before) = escaped {
            self.escape_closed_one(&before, open_before, focus_before)?;
        }
        self.at_most_one_menu()?;
        match self.strictness {
            Strictness::Decided => Ok(()),
            Strictness::Strict => self.focus_is_where_it_belongs(),
        }
    }

    /// Frames for the library's lags to pass, and for every overlay that is
    /// leaving to be gone: focus returns to the opener when the exit has run.
    fn settle(&mut self) -> Result<(), TestCaseError> {
        self.steps(SETTLE)?;
        for waited in 0..=EXIT_LIMIT {
            if !self.seen().overlays.iter().any(|overlay| overlay.exiting) {
                return if waited == 0 {
                    Ok(())
                } else {
                    self.steps(SETTLE)
                };
            }
            self.step()?;
        }
        Err(TestCaseError::fail(format!(
            "an overlay is still leaving after {EXIT_LIMIT} frames"
        )))
    }

    /// A context menu is anchored to a point that a smaller window can leave
    /// outside it: that is the second open question.
    fn context_menu_is_open(&self) -> bool {
        self.seen()
            .overlays
            .iter()
            .any(|overlay| overlay.path.contains("ContextMenu(") && !overlay.exiting)
    }

    fn perform(&mut self, action: &Action) -> Result<(), TestCaseError> {
        match *action {
            Action::ArrowDown | Action::ArrowUp | Action::Home | Action::End
                if self.strictness == Strictness::Decided && self.down =>
            {
                Ok(())
            }
            Action::OpenMenu => self.click_node("\"menu\""),
            Action::OpenSelect => self.click_node("\"pick\""),
            Action::OpenPopover => self.click_node("\"popover\""),
            Action::OpenAnchorPopover => self.click_node("\"anchor\""),
            Action::OpenModal => self.click_node("\"dialog\""),
            Action::OpenConfirm => self.click_node("\"delete\""),
            Action::OpenSubmenu => {
                let more = if self.seen().node("\"context-more\"").is_some() {
                    "\"context-more\""
                } else {
                    "\"more\""
                };
                self.click_node(more)
            }
            Action::OpenContextMenu(x, y) => {
                let Some((_, canvas)) = self.seen().node("\"canvas\"") else {
                    return Ok(());
                };
                if self.down {
                    return Ok(());
                }
                let to = canvas.min + (canvas.size() * vec2(x as f32, y as f32) / 100.0);
                self.click(to, PointerButton::Secondary)
            }
            Action::HoverItem(index) => match self.item(index) {
                Some(to) => self.move_to(to),
                None => Ok(()),
            },
            Action::ClickItem(index) => match self.item(index) {
                Some(to) if !self.down => self.click(to, PointerButton::Primary),
                _ => Ok(()),
            },
            Action::HoverTrigger(index) => {
                if let Some((_, rect)) = self.seen().node(TRIGGERS[index]) {
                    self.move_to(rect.center())?;
                    self.steps(DWELL)?;
                }
                Ok(())
            }
            Action::PressAt(x, y) if !self.down => {
                let to = self.at_percent(x, y);
                self.move_to(to)?;
                self.steps(2)?;
                self.button(PointerButton::Primary, true)?;
                self.down = true;
                Ok(())
            }
            Action::ReleaseAt(x, y) if self.down => {
                let to = self.at_percent(x, y);
                self.move_to(to)?;
                self.button(PointerButton::Primary, false)?;
                self.down = false;
                Ok(())
            }
            Action::ClickAt(x, y) if !self.down => {
                let to = self.at_percent(x, y);
                self.click(to, PointerButton::Primary)
            }
            Action::PressAt(..) | Action::ReleaseAt(..) | Action::ClickAt(..) => Ok(()),
            Action::Esc => self.key(Modifiers::NONE, Key::Escape),
            Action::Tab => self.key(Modifiers::NONE, Key::Tab),
            Action::ShiftTab => self.key(Modifiers::SHIFT, Key::Tab),
            Action::ArrowDown => self.key(Modifiers::NONE, Key::ArrowDown),
            Action::ArrowUp => self.key(Modifiers::NONE, Key::ArrowUp),
            Action::ArrowLeft => self.key(Modifiers::NONE, Key::ArrowLeft),
            Action::ArrowRight => self.key(Modifiers::NONE, Key::ArrowRight),
            Action::Home => self.key(Modifiers::NONE, Key::Home),
            Action::End => self.key(Modifiers::NONE, Key::End),
            Action::Enter => self.key(Modifiers::NONE, Key::Enter),
            Action::ScrollAnchorContainer(delta) => {
                if let Some((id, _)) = self.seen().node("\"scroller\"") {
                    eggshell_ui::scroll::scroll_by(&self.harness.ctx, id, vec2(0.0, delta as f32));
                }
                self.step()
            }
            Action::ResizeWindow(..)
                if self.strictness == Strictness::Decided && self.context_menu_is_open() =>
            {
                Ok(())
            }
            Action::ResizeWindow(w, h) => {
                let target = vec2(w as f32, h as f32);
                while self.size != target {
                    let toward = target - self.size;
                    self.size += vec2(
                        toward.x.clamp(-EDGE_STEP, EDGE_STEP),
                        toward.y.clamp(-EDGE_STEP, EDGE_STEP),
                    );
                    self.harness.set_size(self.size);
                    self.step()?;
                }
                Ok(())
            }
            Action::Idle(count) => self.steps(count),
        }
    }

    /// Esc closed exactly one overlay that is open for the user, if there was
    /// one that takes part in input: the topmost one that takes the key,
    /// which an overlay with a cancel button does by pressing it. (No drag is
    /// live in this app, and a text field's Esc is its own only when no
    /// overlay is open.) Focus then returns to the widget that had it when the
    /// overlay opened, unless a pointer button is held: the library waits for
    /// it to be released.
    fn escape_closed_one(
        &self,
        before: &Seen,
        open_before: usize,
        focus_before: Option<Id>,
    ) -> Result<(), TestCaseError> {
        let after = self.seen().attached_open();
        let expected = open_before.saturating_sub(1);
        ensure(after == expected, || {
            let names = |seen: &Seen| {
                seen.open()
                    .map(|o| o.path.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "Esc left {after} overlay(s) open, expected {expected}: before [{}], after [{}]",
                names(before),
                names(self.seen())
            )
        })?;
        let closed: Vec<&Open> = before
            .open()
            .filter(|gone| self.seen().open().all(|still| still.id != gone.id))
            .collect();
        if let [gone] = closed[..]
            && gone.focus == Focus::Enter
            && !self.down
            && let Some(opener) = self.opener_of(gone.id)
            && self.seen().nodes.iter().any(|(_, id, _)| *id == opener)
        {
            ensure(self.focused() == Some(opener), || {
                format!(
                    "Esc closed {} but focus is {:?}, not back on {opener:?} that had it when it opened (focus before Esc: {focus_before:?})",
                    gone.path,
                    self.focused()
                )
            })?;
        }
        Ok(())
    }

    fn opener_of(&self, id: Id) -> Option<Id> {
        self.openers
            .iter()
            .find(|(open, _)| *open == id)
            .and_then(|(_, opener)| *opener)
    }

    /// An overlay opened while a menu was open closed it, unless it is
    /// nested in it.
    fn at_most_one_menu(&self) -> Result<(), TestCaseError> {
        let counts = |overlay: &&Open| {
            overlay.is_menu()
                && (self.strictness == Strictness::Strict || overlay.anchored == Anchored::Attached)
        };
        let menus: Vec<&str> = self
            .seen()
            .overlays
            .iter()
            .filter(counts)
            .map(|overlay| overlay.path.as_str())
            .collect();
        ensure(menus.len() <= 1, || {
            format!("more than one menu is open: {menus:?}")
        })
    }

    /// Focus is nowhere, inside the topmost overlay that takes it in, or on
    /// that overlay's anchor.
    fn focus_is_where_it_belongs(&self) -> Result<(), TestCaseError> {
        let Some(top) = self
            .seen()
            .open()
            .filter(|overlay| {
                overlay.focus == Focus::Enter && overlay.anchored == Anchored::Attached
            })
            .max_by_key(|overlay| overlay.stack_index)
        else {
            return Ok(());
        };
        let Some(focused) = self.focused() else {
            return Ok(());
        };
        let ctx = &self.harness.ctx;
        let inside = focus_within(ctx, top.id)
            || top
                .anchor_ids
                .iter()
                .any(|anchor| focus_within(ctx, *anchor))
            || self
                .seen()
                .open()
                .filter(|overlay| overlay.stack_index > top.stack_index)
                .any(|above| focus_within(ctx, above.id));
        ensure(inside, || {
            let path = self
                .seen()
                .nodes
                .iter()
                .find(|(_, id, _)| *id == focused)
                .map_or("<widget>", |(path, _, _)| path.as_str());
            format!(
                "focus is on {path}, outside the topmost overlay that takes focus in, {}",
                top.path
            )
        })
    }
}

fn run(actions: &[Action], strictness: Strictness) -> Result<(), TestCaseError> {
    let mut desk = Desk::new(strictness);
    for action in actions {
        desk.act(action)
            .map_err(|error| TestCaseError::fail(format!("after {action:?}: {error}")))?;
    }
    if desk.down {
        desk.harness.drop_at(desk.at);
        desk.down = false;
    }
    desk.steps(IDLE + DWELL)?;
    let Desk {
        harness, session, ..
    } = desk;
    verdict(
        &session
            .timeline(&harness.ctx)
            .max_step(Points::new(MAX_GRID_STEP)),
    )
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn overlays_hold_their_invariants_under_any_session(actions in overlay_session()) {
        run(&actions, Strictness::Decided)?;
    }

    /// What the documented rules leave open, as `docs/overlays.md` section 12
    /// states the invariants:
    ///
    /// - Focus is inside the topmost overlay that takes it in, nowhere, or on
    ///   its anchor. A menu opened with the pointer takes no focus, and Tab
    ///   then moves focus to the page without closing it (`dismiss.rs` pins
    ///   that `Nowhere -> Elsewhere` is not focus leaving).
    /// - At most one menu is open. A context menu whose point anchor left the
    ///   window (a smaller window) is hidden but open, and takes no part in
    ///   dismissal, so another menu can open beside it (`open_overlays` leaves
    ///   detached overlays out of `exclusive`).
    #[test]
    #[ignore = "open questions, see the doc comment and docs/overlays.md, As built in PR 8"]
    fn focus_stays_with_the_overlays_under_any_session(actions in overlay_session()) {
        run(&actions, Strictness::Strict)?;
    }
}

/// The shrunk case of the first open question: Tab from a menu opened with
/// the pointer.
/// The third open finding, outside the overlays: with a pointer button held,
/// End on a focused button in a scroll container scrolls it for the frame of
/// the key press and puts it back in the frame of the release, which hides
/// the tooltip of the button (a gap) for that frame. Without the button held
/// the container stays scrolled. The decided session does not press the
/// scroll keys while a button is held.
#[test]
#[ignore = "open finding: End with a button held scrolls for one frame and snaps back"]
fn end_with_a_button_held_scrolls_and_stays() {
    run(
        &[
            Action::ShiftTab,
            Action::PressAt(58, 44),
            Action::ShiftTab,
            Action::End,
        ],
        Strictness::Strict,
    )
    .expect("holds");
}

/// Focus is not given back while a button is held (an Esc during a press).
#[test]
fn escape_with_a_button_held_closes_the_menu_and_waits_to_give_focus_back() {
    run(
        &[
            Action::Tab,
            Action::ArrowDown,
            Action::PressAt(0, 0),
            Action::ArrowDown,
            Action::Esc,
        ],
        Strictness::Decided,
    )
    .expect("holds");
}

#[test]
#[ignore = "open design question: focus moves to the page while the menu stays open"]
fn tab_from_a_menu_opened_with_the_pointer_closes_it() {
    run(&[Action::OpenMenu, Action::Tab], Strictness::Strict).expect("holds");
}

/// The shrunk case of the second: a context menu hidden by a smaller window
/// does not make way for the dropdown.
#[test]
#[ignore = "open design question: a detached menu stays open beside another"]
fn a_menu_hidden_by_a_smaller_window_makes_way_for_the_next() {
    run(
        &[
            Action::OpenContextMenu(0, 7),
            Action::ResizeWindow(240, 220),
            Action::OpenMenu,
        ],
        Strictness::Strict,
    )
    .expect("holds");
}

/// A tooltip over the button that is pressed was raised by egui over the
/// tooltip that opened after it: its area was seeded as interactable.
#[test]
fn a_press_under_a_leaving_tooltip_does_not_raise_it() {
    run(
        &[
            Action::HoverTrigger(1),
            Action::ResizeWindow(240, 220),
            Action::OpenModal,
        ],
        Strictness::Decided,
    )
    .expect("holds");
}

/// A popover hidden because its anchor left the window comes back over the
/// dialog that opened meanwhile: egui raises an area that was not shown last
/// frame, and the stack repair did not know.
#[test]
fn a_popover_that_comes_back_does_not_rise_over_a_later_dialog() {
    run(
        &[
            Action::OpenAnchorPopover,
            Action::ResizeWindow(240, 220),
            Action::OpenModal,
            Action::ResizeWindow(240, 308),
        ],
        Strictness::Decided,
    )
    .expect("holds");
}

/// A tooltip follows its trigger when a resize reflows the page: layout
/// moves the box at once, which is not a jump of its entrance motion.
#[test]
fn a_tooltip_follows_a_trigger_that_a_resize_reflows() {
    run(
        &[
            Action::OpenContextMenu(0, 0),
            Action::OpenPopover,
            Action::ResizeWindow(240, 220),
        ],
        Strictness::Decided,
    )
    .expect("holds");
}

/// Esc on a confirmation answered by its cancel button gives focus back to
/// the button that opened it once the exit has run.
#[test]
fn escape_on_a_confirmation_returns_focus_to_its_opener() {
    run(
        &[Action::OpenModal, Action::Tab, Action::Enter, Action::Esc],
        Strictness::Decided,
    )
    .expect("holds");
}

fn open_paths(desk: &Desk) -> Vec<String> {
    desk.seen()
        .overlays
        .iter()
        .filter(|overlay| !overlay.exiting)
        .map(|overlay| overlay.path.clone())
        .collect()
}

/// The actions of the fuzzer really open each kind of overlay: a session
/// that opened none would pass every invariant.
#[test]
fn the_overlay_fuzzer_reaches_every_kind_of_overlay() {
    let reaches = |actions: &[Action], expected: &str| {
        let mut desk = Desk::new(Strictness::Strict);
        for action in actions {
            desk.act(action).expect("holds");
        }
        let open = open_paths(&desk);
        assert!(
            open.iter().any(|path| path.contains(expected)),
            "{actions:?} did not open {expected}: {open:?}"
        );
    };
    reaches(&[Action::OpenMenu], "(\"menu\", Menu)");
    reaches(&[Action::OpenMenu, Action::OpenSubmenu], "more");
    reaches(&[Action::OpenSelect], "(\"pick\", List)");
    reaches(&[Action::OpenPopover], "popover-overlay");
    reaches(&[Action::OpenAnchorPopover], "scroll-popover");
    reaches(&[Action::OpenModal], "dialog-overlay");
    reaches(&[Action::OpenModal, Action::OpenConfirm], "confirm-overlay");
    reaches(&[Action::OpenContextMenu(50, 50)], "canvas");
    reaches(&[Action::HoverTrigger(4)], "Tooltip(");
}

/// The invariants find a bug that is there: a press that reaches the page
/// under a modal, planted by counting a click on the dialog's scrim.
#[test]
fn a_press_on_the_page_is_counted_only_when_no_modal_covers_it() {
    let mut desk = Desk::new(Strictness::Strict);
    let pad = desk.seen().node("\"pad\"").expect("drawn").1.center();
    desk.act(&Action::ClickAt(
        (pad.x / desk.size.x * 100.0) as u32,
        (pad.y / desk.size.y * 100.0) as u32,
    ))
    .expect("holds");
    assert_eq!(desk.app().page_clicks, 1, "precondition: the pad counts");
    desk.act(&Action::OpenModal).expect("holds");
    desk.act(&Action::ClickAt(
        (pad.x / desk.size.x * 100.0) as u32,
        (pad.y / desk.size.y * 100.0) as u32,
    ))
    .expect("holds");
    assert_eq!(desk.app().page_clicks, 1, "the scrim took the press");
}
