//! Stock menus: a dropdown that owns its trigger, a context menu on any
//! node, and the items, separators, headings and submenus they hold. Every
//! menu is an overlay of [`Nav::Menu`], and a submenu one of
//! [`Nav::Submenu`]. The widgets own whether they are open, in the
//! `LayoutState` (`Kept`), and close the whole chain when an item is
//! activated.

use std::collections::HashMap;
use std::sync::Arc;

use egui::accesskit::HasPopup;
use egui::{Color32, Context, EventFilter, Id, Key as EguiKey, Painter, Pos2, Rect, Role, Vec2};

use super::button::{ButtonChain, ButtonOptions, Face};
use super::{Chevron, Enablement, Mark, OnResponse, describe, paint_check, paint_chevron, painted};
use crate::builder::{Scope, Slot};
use crate::contract::{MenuStyle, Style, Variant};
use crate::kept::Handle;
use crate::overlay::{Anchor, Distance, Nav, Outside, Placement};
use crate::overlay_chain::OverlayChain;
use crate::{Builder, Icon, NodeUi as _, TextSpec, Tw, nav, tw};

/// The parts of a menu.
#[derive(Hash, Debug, Clone, Copy)]
enum Part {
    Menu,
    Box,
    Submenu,
    Context,
}

/// The open menus, by the id of the node that opened each: the trigger of a
/// dropdown, the node of a context menu.
#[derive(Default)]
pub(crate) struct Menus(HashMap<Id, Open>);

struct Open {
    /// Where a context menu was opened.
    at: Option<Pos2>,
    /// The item that owns the open submenu at each depth, outermost first.
    submenus: Vec<Id>,
}

/// What an item does to the submenu open beside it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owns {
    Nothing,
    Submenu,
}

impl Menus {
    fn is_open(&self, root: Id) -> bool {
        self.0.contains_key(&root)
    }

    fn open(&mut self, root: Id, at: Option<Pos2>) {
        self.0.insert(
            root,
            Open {
                at,
                submenus: Vec::new(),
            },
        );
    }

    fn close(&mut self, root: Id) {
        self.0.remove(&root);
    }

    fn toggle(&mut self, root: Id) {
        if self.is_open(root) {
            self.close(root);
        } else {
            self.open(root, None);
        }
    }

    fn position(&self, root: Id) -> Option<Pos2> {
        self.0.get(&root)?.at
    }

    fn submenu(&self, root: Id, depth: usize) -> Option<Id> {
        self.0.get(&root)?.submenus.get(depth).copied()
    }

    fn open_submenu(&mut self, root: Id, depth: usize, item: Id) {
        if let Some(open) = self.0.get_mut(&root) {
            open.submenus.truncate(depth);
            open.submenus.push(item);
        }
    }

    fn close_submenu(&mut self, root: Id, depth: usize) {
        if let Some(open) = self.0.get_mut(&root) {
            open.submenus.truncate(depth);
        }
    }

    /// The pointer moved onto `item` at `depth`. A submenu already open
    /// there follows the pointer: it switches to a sibling that owns one and
    /// closes for one that does not. Returns whether anything changed.
    fn hover(&mut self, root: Id, depth: usize, item: Id, owns: Owns) -> bool {
        let Some(open) = self.submenu(root, depth) else {
            return false;
        };
        match owns {
            Owns::Submenu if open == item => false,
            Owns::Submenu => {
                self.open_submenu(root, depth, item);
                true
            }
            Owns::Nothing => {
                self.close_submenu(root, depth);
                true
            }
        }
    }
}

/// What every part of one open menu chain shares.
#[derive(Clone)]
struct Shared {
    root: Id,
    style: Arc<Style>,
    kept: Handle,
    ctx: Context,
}

impl Shared {
    fn of<A>(b: &Builder<'_, '_, A>, root: Id) -> Self {
        Self {
            root,
            style: Style::of(b.ctx()),
            kept: b.kept().clone(),
            ctx: b.ctx().clone(),
        }
    }

    fn change(&self, change: impl FnOnce(&mut Menus)) {
        self.kept.with(|kept| change(&mut kept.menus));
        self.ctx.request_repaint();
    }

    fn close(&self) {
        self.change(|menus| menus.close(self.root));
    }

    fn text(&self) -> &TextSpec {
        self.style.role(&self.style.menu.text)
    }
}

/// How an item is coloured.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Plain,
    Destructive,
}

impl MenuStyle {
    /// The classes of the box that holds the items.
    pub(crate) fn box_classes(&self, text: &TextSpec) -> Tw {
        let colors = self.popup.paint.states.rest;
        let classes = tw()
            .flex()
            .flex_col()
            .min_h_0()
            .p(self.padding / 4.0)
            .border(self.popup.border_width)
            .border_color(colors.border)
            .bg(colors.fill)
            .text(colors.text)
            .rounded_px(f32::from(self.radius))
            .role(text.clone());
        match self.popup.paint.shadow {
            Some(shadow) => classes.shadow(shadow),
            None => classes,
        }
    }

    /// The height of one item.
    pub(crate) fn row_height(&self, text: &TextSpec) -> f32 {
        text.line_height + 2.0 * self.item_padding.y
    }

    /// The classes of one item: the highlighted one takes the `hover`
    /// colours.
    pub(crate) fn row_classes(&self, tone: Tone, enablement: Enablement, text: &TextSpec) -> Tw {
        let states = match tone {
            Tone::Plain => self.item.paint.states,
            Tone::Destructive => self.destructive,
        };
        let base = tw()
            .flex()
            .items_center()
            .gap(self.item_gap / 4.0)
            .px(self.item_padding.x / 4.0)
            .h_px(self.row_height(text))
            .rounded_px(f32::from(self.item_radius))
            .whitespace_nowrap();
        match enablement {
            Enablement::Disabled => painted(base, self.item.paint.states.disabled).inert(),
            Enablement::Enabled => {
                let hover = states.hover;
                painted(base.sense_click(), states.rest)
                    .hover(|p| p.bg(hover.fill).text(hover.text))
            }
        }
    }
}

/// What an item shows before its label.
#[derive(Clone, Copy)]
enum Lead {
    Nothing,
    Check(Mark),
    Dot(Mark),
}

/// What an item shows after its label.
enum Trail {
    Nothing,
    Shortcut(String),
    Chevron,
}

/// What an item does when it is activated.
enum Reaction<A> {
    Run(Box<dyn FnOnce(&mut A)>),
    OpenSubmenu,
}

struct Row<A> {
    slot: Slot,
    label: String,
    role: Role,
    lead: Lead,
    trail: Trail,
    tone: Tone,
    enablement: Enablement,
    reaction: Reaction<A>,
}

/// The content of an open menu, which a menu's `items` body adds to. Each
/// item is a node under the key it is given, which must be unique in the
/// menu.
pub struct Menu<'m, 'f, A> {
    b: Builder<'m, 'f, A>,
    shared: Shared,
    depth: usize,
}

impl<'f, A: 'static> Menu<'_, 'f, A> {
    /// An item that runs a handler when it is activated, which closes the
    /// whole menu. Finish it with [`on_click`](ItemChain::on_click).
    pub fn item<K: crate::Key>(
        &mut self,
        key: K,
        label: impl Into<String>,
    ) -> ItemChain<'_, 'f, A, Action> {
        self.chain(key, label.into(), Action, Role::MenuItem, Lead::Nothing)
    }

    /// An item with a check mark where `current` is on, for a setting that
    /// is on or off. Finish it with [`on_change`](ItemChain::on_change).
    pub fn checkbox_item<K: crate::Key>(
        &mut self,
        key: K,
        label: impl Into<String>,
        current: impl Into<Mark>,
    ) -> ItemChain<'_, 'f, A, Checkbox> {
        let mark = current.into();
        self.chain(
            key,
            label.into(),
            Checkbox(mark),
            Role::MenuItemCheckBox,
            Lead::Check(mark),
        )
    }

    /// An item with a dot where `current` is on, for one choice among
    /// several. Finish it with [`on_click`](ItemChain::on_click).
    pub fn radio_item<K: crate::Key>(
        &mut self,
        key: K,
        label: impl Into<String>,
        current: impl Into<Mark>,
    ) -> ItemChain<'_, 'f, A, Radio> {
        self.chain(
            key,
            label.into(),
            Radio,
            Role::MenuItemRadio,
            Lead::Dot(current.into()),
        )
    }

    fn chain<K: crate::Key, Kind>(
        &mut self,
        key: K,
        label: String,
        kind: Kind,
        role: Role,
        lead: Lead,
    ) -> ItemChain<'_, 'f, A, Kind> {
        ItemChain {
            slot: self.b.slot(&key),
            b: self.b.reborrow(),
            shared: self.shared.clone(),
            depth: self.depth,
            label,
            role,
            lead,
            shortcut: None,
            tone: Tone::Plain,
            enablement: Enablement::Enabled,
            kind,
        }
    }

    /// A line between groups of items.
    pub fn separator(&mut self) {
        let menu = &self.shared.style.menu;
        self.b.div(
            tw().h_px(1.0)
                .mx(-menu.padding / 4.0)
                .my(menu.separator_margin / 4.0)
                .shrink_0()
                .bg(menu.separator),
            |_| {},
        );
    }

    /// A heading over a group of items. It takes no input.
    pub fn label(&mut self, text: impl Into<String>) {
        let (menu, text_role) = (&self.shared.style.menu, self.shared.text());
        let classes = tw()
            .flex()
            .items_center()
            .px(menu.item_padding.x / 4.0)
            .h_px(menu.row_height(text_role))
            .text(menu.heading)
            .select_none()
            .whitespace_nowrap();
        let text = text.into();
        self.b.div(classes, |b| b.text(tw(), text));
    }

    /// An item with a chevron that opens a submenu holding what `items`
    /// adds. A click, Enter or Right opens it; Left or Esc closes it and
    /// focuses the item. While a submenu is open, the pointer on a sibling
    /// item that owns a submenu switches to it, and on one that does not
    /// closes it.
    pub fn submenu<K: crate::Key + Clone>(
        &mut self,
        key: K,
        label: impl Into<String>,
        items: impl for<'m> FnOnce(&mut Menu<'m, 'f, A>),
    ) {
        let label = label.into();
        let overlay = self.b.slot(&(key.clone(), Part::Submenu));
        let slot = self.b.slot(&key);
        let item = slot.id;
        self.add_row(Row {
            slot,
            label: label.clone(),
            role: Role::MenuItem,
            lead: Lead::Nothing,
            trail: Trail::Chevron,
            tone: Tone::Plain,
            enablement: Enablement::Enabled,
            reaction: Reaction::OpenSubmenu,
        });
        let (root, depth) = (self.shared.root, self.depth);
        if self
            .shared
            .kept
            .with(|kept| kept.menus.submenu(root, depth))
            != Some(item)
        {
            return;
        }
        let shared = self.shared.clone();
        let close = self.shared.clone();
        OverlayChain::new(self.b.reborrow(), overlay)
            .anchor(Anchor::Node(item))
            .placement(Placement::submenu())
            .nav(Nav::Submenu)
            .outside(Outside::PassThrough)
            .role(Role::Menu)
            .named(label)
            .on_dismiss(move |_| close.change(|menus| menus.close_submenu(root, depth)))
            .children(|s| menu_box(s, &shared, depth + 1, items));
    }

    fn add_row(&mut self, row: Row<A>) {
        let Row {
            slot,
            label,
            role,
            lead,
            trail,
            tone,
            enablement,
            reaction,
        } = row;
        let enablement = self.b.enablement_of(enablement);
        let shared = self.shared.clone();
        let (text, menu) = (shared.text(), &shared.style.menu);
        let classes = menu.row_classes(tone, enablement, text);
        let (indicator, dot) = (menu.indicator, menu.dot);
        let (shortcut_text, shortcut, tracking) =
            (&menu.shortcut_text, menu.shortcut, menu.shortcut_tracking);
        let shown = label.clone();
        let (root, depth, id) = (shared.root, self.depth, slot.id);
        let owns = match reaction {
            Reaction::Run(_) => Owns::Nothing,
            Reaction::OpenSubmenu => Owns::Submenu,
        };
        let mark = match lead {
            Lead::Nothing => None,
            Lead::Check(mark) | Lead::Dot(mark) => Some(mark),
        };
        let expanded = shared.kept.with(|kept| kept.menus.submenu(root, depth)) == Some(id);
        self.b
            .div_id_with(id, &slot.label, classes, |b| {
                match lead {
                    Lead::Nothing => {}
                    Lead::Check(Mark::On) => {
                        b.paint(tw().size_px(indicator).shrink_0(), paint_check);
                    }
                    Lead::Dot(Mark::On) => b.paint(
                        tw().size_px(indicator).shrink_0(),
                        move |painter: &Painter, rect: Rect, color: Color32| {
                            painter.circle_filled(rect.center(), dot / 2.0, color);
                        },
                    ),
                    Lead::Check(Mark::Off) | Lead::Dot(Mark::Off) => {
                        b.div(tw().size_px(indicator).shrink_0(), |_| {});
                    }
                }
                b.text(tw(), shown);
                match trail {
                    Trail::Nothing => {}
                    Trail::Shortcut(keys) => b.text(
                        tw().ml_auto()
                            .role(shortcut_text.clone())
                            .text(shortcut)
                            .tracking(tracking),
                        keys,
                    ),
                    Trail::Chevron => b.paint(
                        tw().size_px(indicator).shrink_0().ml_auto(),
                        paint_chevron(Chevron::Right),
                    ),
                }
            })
            .role(role)
            .named(label.clone())
            .draw(move |app, ui| {
                let response = ui.node_response();
                describe(&response, ui.is_enabled(), role, mark, &label);
                if owns == Owns::Submenu {
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_has_popup(HasPopup::Menu);
                        node.set_expanded(expanded);
                    });
                }
                if enablement == Enablement::Disabled {
                    return;
                }
                super::number::hold_arrows(ui);
                let pointer_moved = ui.input(|input| input.pointer.delta() != Vec2::ZERO);
                if pointer_moved && response.hovered() {
                    shared.change_if(|menus| menus.hover(root, depth, id, owns));
                }
                let activated = response.clicked() || nav::take_activation(ui.ctx(), id);
                match reaction {
                    Reaction::Run(act) => {
                        if activated {
                            act(app);
                            shared.close();
                        }
                    }
                    Reaction::OpenSubmenu => {
                        let right =
                            response.has_focus() && super::number::pressed(ui, EguiKey::ArrowRight);
                        if activated || right {
                            shared.change(|menus| menus.open_submenu(root, depth, id));
                        }
                    }
                }
            });
    }
}

impl Shared {
    /// Like [`change`](Self::change), for a change that reports whether it
    /// did anything.
    fn change_if(&self, change: impl FnOnce(&mut Menus) -> bool) {
        if self.kept.with(|kept| change(&mut kept.menus)) {
            self.ctx.request_repaint();
        }
    }
}

/// The box of a menu at `depth`, with `items` in it.
fn menu_box<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    shared: &Shared,
    depth: usize,
    items: impl for<'m> FnOnce(&mut Menu<'m, 'f, A>),
) {
    let menu = &shared.style.menu;
    let classes = menu.box_classes(shared.text()).min_w_px(menu.min_width);
    let shared = shared.clone();
    s.div_id(Part::Box, classes, |s| {
        items(&mut Menu {
            b: s.reborrow(),
            shared,
            depth,
        });
    });
}

/// An item of a menu, started with [`Menu::item`], [`Menu::checkbox_item`] or
/// [`Menu::radio_item`] and finished with the handler of its kind.
#[must_use = "an item is not added until you call `.on_click(..)` or `.on_change(..)`"]
pub struct ItemChain<'m, 'f, A, Kind> {
    b: Builder<'m, 'f, A>,
    shared: Shared,
    depth: usize,
    slot: Slot,
    label: String,
    role: Role,
    lead: Lead,
    shortcut: Option<String>,
    tone: Tone,
    enablement: Enablement,
    kind: Kind,
}

/// The kind of an item that runs a handler.
pub struct Action;
/// The kind of an item with a check mark, on or off as it was built.
pub struct Checkbox(Mark);
/// The kind of an item with a dot.
pub struct Radio;

impl<A: 'static, Kind> ItemChain<'_, '_, A, Kind> {
    /// The keys that do the same thing, shown after the label. Display only:
    /// the application handles the keys.
    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }

    /// Draws the item for an action that cannot be undone.
    pub fn destructive(mut self) -> Self {
        self.tone = Tone::Destructive;
        self
    }

    /// Whether the item takes input. A disabled item is skipped by the arrow
    /// keys.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    #[track_caller]
    fn finish(self, handler: impl FnOnce(&mut A) + 'static) {
        let Self {
            b,
            shared,
            depth,
            slot,
            label,
            role,
            lead,
            shortcut,
            tone,
            enablement,
            kind: _,
        } = self;
        Menu { b, shared, depth }.add_row(Row {
            slot,
            label,
            role,
            lead,
            trail: shortcut.map_or(Trail::Nothing, Trail::Shortcut),
            tone,
            enablement,
            reaction: Reaction::Run(Box::new(handler)),
        });
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Action> {
    /// Adds the item. `act` runs when it is clicked or activated with Enter
    /// or Space, and then the menu closes.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) {
        self.finish(act);
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Radio> {
    /// Adds the item. `act` runs when it is activated, and then the menu
    /// closes.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) {
        self.finish(act);
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Checkbox> {
    /// Adds the item. `change` runs with the value the item moves to when it
    /// is activated, and then the menu closes.
    #[track_caller]
    pub fn on_change(self, change: impl FnOnce(&mut A, Mark) + 'static) {
        let moves_to = match self.kind.0 {
            Mark::On => Mark::Off,
            Mark::Off => Mark::On,
        };
        self.finish(move |app| change(app, moves_to));
    }
}

/// The least a trigger needs to say what it opens.
fn hold_vertical_arrows(ctx: &Context, trigger: Id) {
    let filter = EventFilter {
        vertical_arrows: true,
        ..EventFilter::default()
    };
    ctx.memory_mut(|memory| memory.set_focus_lock_filter(trigger, filter));
}

/// A button that opens a menu under it. Start one with
/// [`Builder::dropdown_menu`] and finish it with [`items`](Self::items).
///
/// Clicking the trigger, or Enter, Space or Down on it, opens the menu. In
/// the open menu the arrows, Home and End move between the items, Enter
/// activates one, and Esc, a press outside or Tab closes it. Activating an
/// item closes it.
///
/// ```html
/// <DropdownMenu>
///   <DropdownMenuTrigger asChild><Button variant="outline">Open</Button></DropdownMenuTrigger>
///   <DropdownMenuContent class="min-w-[8rem] rounded-md border bg-popover p-1 shadow-md">
///     <DropdownMenuItem>Copy <DropdownMenuShortcut>Ctrl+C</DropdownMenuShortcut></DropdownMenuItem>
///   </DropdownMenuContent>
/// </DropdownMenu>
/// ```
#[must_use = "a dropdown menu is not added until you call `.items(..)`"]
pub struct DropdownChain<'a, 'f, A, K> {
    b: Builder<'a, 'f, A>,
    key: K,
    face: Face,
    options: ButtonOptions,
    name: Option<String>,
    enablement: Enablement,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, K: crate::Key + Clone> DropdownChain<'a, 'f, A, K> {
    pub(super) fn new(b: Builder<'a, 'f, A>, key: K, face: Face) -> Self {
        Self {
            b,
            key,
            face,
            options: ButtonOptions::new(),
            name: None,
            enablement: Enablement::Enabled,
            on_response: OnResponse::default(),
        }
    }

    /// Applies shared button `options` to the trigger.
    pub fn with_options(mut self, options: ButtonOptions) -> Self {
        self.options = self.options.merge(options);
        self
    }

    /// Draws the trigger as the style's button variant `variant`.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.options = self.options.variant(variant);
        self
    }

    /// Puts `icon` before the trigger's label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.options = self.options.icon(icon);
        self
    }

    /// Classes merged over the styled ones of the trigger ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.options = self.options.classes(classes);
        self
    }

    /// The accessible name of the trigger and of its menu, when it is not
    /// the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether the trigger takes input. A trigger that becomes disabled
    /// closes its menu.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Runs `then` with the trigger's response each frame.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Adds the trigger and, while it is open, its menu, which `items` fills.
    #[track_caller]
    pub fn items(self, items: impl for<'m> FnOnce(&mut Menu<'m, 'f, A>)) {
        let Self {
            mut b,
            key,
            face,
            options,
            name,
            enablement,
            on_response,
        } = self;
        let shared = Shared::of(&b, b.slot(&key).id);
        let trigger = shared.root;
        let list = b.slot(&(key.clone(), Part::Menu));
        let enablement = b.enablement_of(enablement);
        if enablement == Enablement::Disabled {
            shared.close();
        }
        let open = shared.kept.with(|kept| kept.menus.is_open(trigger));
        let name = name.or(match &face {
            Face::Label(label) => Some(label.clone()),
            _ => None,
        });
        let mut button = ButtonChain::new(b.reborrow(), key, face)
            .with_options(options)
            .opens_popup()
            .enablement(enablement);
        if let Some(name) = &name {
            button = button.named(name.clone());
        }
        let (on_key, on_click) = (shared.clone(), shared.clone());
        button
            .on_response(move |app, response| {
                on_response.run(app, response);
                let ctx = &on_key.ctx;
                let open = on_key.kept.with(|kept| kept.menus.is_open(trigger));
                ctx.accesskit_node_builder(response.id, |node| {
                    node.set_expanded(open);
                    node.set_has_popup(HasPopup::Menu);
                });
                if response.has_focus() {
                    hold_vertical_arrows(ctx, response.id);
                    let down = ctx.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, EguiKey::ArrowDown)
                    });
                    if down && !open {
                        on_key.change(|menus| menus.open(trigger, None));
                    }
                }
            })
            .on_click(move |_| on_click.change(|menus| menus.toggle(trigger)));
        if !open {
            return;
        }
        let close = shared.clone();
        OverlayChain::new(b.reborrow(), list)
            .anchor(Anchor::Node(trigger))
            .placement(Placement::below().gap(Distance::new(4.0)))
            .nav(Nav::Menu)
            .role(Role::Menu)
            .named(name.unwrap_or_default())
            .on_dismiss(move |_| close.close())
            .children(|s| menu_box(s, &shared, 0, items));
    }
}

/// Opens the context menu of the node `owner` where the pointer is when it
/// is secondary-clicked, and shows it while it is open.
pub(crate) fn context<'f, A: 'static>(
    mut b: Builder<'_, 'f, A>,
    owner: Id,
    owner_path: &str,
    items: impl for<'m> FnOnce(&mut Menu<'m, 'f, A>),
) {
    let shared = Shared::of(&b, owner);
    let Some(at) = shared.kept.with(|kept| kept.menus.position(owner)) else {
        return;
    };
    let slot = Slot::labelled(
        b.slot(&(Part::Context, owner)).id,
        format!("ContextMenu({owner_path})"),
    );
    let close = shared.clone();
    OverlayChain::new(b.reborrow(), slot)
        .anchor(Anchor::Point(at))
        .placement(Placement::menu())
        .nav(Nav::Menu)
        .role(Role::Menu)
        .named("Context menu")
        .on_dismiss(move |_| close.close())
        .children(|s| menu_box(s, &shared, 0, items));
}

/// Opens the context menu of `owner` at the pointer on a secondary click.
pub(crate) fn open_on_secondary_click(ui: &egui::Ui, shared_kept: &Handle, owner: Id) {
    let response = ui.node_response();
    if let Some(at) = response
        .secondary_clicked()
        .then(|| response.interact_pointer_pos())
        .flatten()
    {
        shared_kept.with(|kept| kept.menus.open(owner, Some(at)));
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    fn open_at_depth_zero(item: &str) -> Menus {
        let mut menus = Menus::default();
        menus.open(id("root"), None);
        menus.open_submenu(id("root"), 0, id(item));
        menus
    }

    #[test]
    fn hover_changes_nothing_while_no_submenu_is_open() {
        let mut menus = Menus::default();
        menus.open(id("root"), None);
        assert!(!menus.hover(id("root"), 0, id("a"), Owns::Submenu));
        assert_eq!(menus.submenu(id("root"), 0), None);
    }

    #[test]
    fn hover_on_a_sibling_that_owns_a_submenu_switches_to_it() {
        let mut menus = open_at_depth_zero("a");
        assert!(menus.hover(id("root"), 0, id("b"), Owns::Submenu));
        assert_eq!(menus.submenu(id("root"), 0), Some(id("b")));
    }

    #[test]
    fn hover_on_the_item_that_owns_the_open_submenu_changes_nothing() {
        let mut menus = open_at_depth_zero("a");
        assert!(!menus.hover(id("root"), 0, id("a"), Owns::Submenu));
        assert_eq!(menus.submenu(id("root"), 0), Some(id("a")));
    }

    #[test]
    fn hover_on_a_plain_item_closes_the_submenu_and_those_inside_it() {
        let mut menus = open_at_depth_zero("a");
        menus.open_submenu(id("root"), 1, id("deep"));
        assert!(menus.hover(id("root"), 0, id("plain"), Owns::Nothing));
        assert_eq!(menus.submenu(id("root"), 0), None);
        assert_eq!(menus.submenu(id("root"), 1), None);
    }

    #[test]
    fn hover_inside_a_submenu_leaves_the_one_above_it_open() {
        let mut menus = open_at_depth_zero("a");
        menus.open_submenu(id("root"), 1, id("deep"));
        assert!(menus.hover(id("root"), 1, id("plain"), Owns::Nothing));
        assert_eq!(menus.submenu(id("root"), 0), Some(id("a")));
        assert_eq!(menus.submenu(id("root"), 1), None);
    }

    #[test]
    fn closing_a_menu_forgets_its_submenus() {
        let mut menus = open_at_depth_zero("a");
        menus.close(id("root"));
        assert!(!menus.is_open(id("root")));
        menus.open(id("root"), None);
        assert_eq!(menus.submenu(id("root"), 0), None);
    }
}
