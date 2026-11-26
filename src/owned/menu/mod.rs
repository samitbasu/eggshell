//! Stock menus: a dropdown that owns its trigger, a context menu on any
//! node, and the items, separators, headings and submenus they hold. Every
//! menu is an overlay of [`Nav::Menu`], and a submenu one of
//! [`Nav::Submenu`]. The widgets own whether they are open, in the
//! `LayoutState` (`Kept`), and close the whole chain when an item is
//! activated.

use std::collections::HashMap;
use std::sync::Arc;

use egui::accesskit::HasPopup;
use egui::{Context, EventFilter, Id, Key as EguiKey, Pos2, Role, Vec2};

use super::button::{self, Face};
mod sheet;

use super::options::{Configure, Interactive, Options, Resolved, Respond};
use super::{Chevron, Enablement, Mark, OnResponse, check, describe, tipped};
use crate::builder::{Scope, Slot};
use crate::commands::Bound;
use crate::contract::{MenuStyle, Style, Variant};
use crate::kept::Handle;
use crate::overlay::{
    Anchor, Distance, Fit, Nav, Outside, Placement, PointerOpen, ReturnsTo, Shown, Wrap,
};
use crate::overlay_chain::OverlayChain;
use crate::sheet::Key;
use crate::trigger_group::{Groups, Member, Switch};
use crate::{Builder, CommandState, Icon, NodeUi as _, TextSpec, Tw, nav, tw};
use sheet::INSET;
pub use sheet::Part;
pub(crate) use sheet::write_exceptions;

/// The parts of a menu.
#[derive(Hash, Debug, Clone, Copy)]
enum Node {
    Menu,
    Box,
    Rows,
    Submenu,
    Context,
}

/// The open menus, by the id of the node that opened each: the trigger of a
/// dropdown, the node of a context menu; and the dropdowns linked into bars.
#[derive(Default)]
pub(crate) struct Menus {
    open: HashMap<Id, Open>,
    links: Groups,
}

struct Open {
    /// Where a context menu was opened.
    at: Option<Pos2>,
    /// The item that owns the open submenu at each depth, outermost first.
    submenus: Vec<Id>,
    by: OpenedBy,
}

/// What opened a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenedBy {
    /// The pointer moving onto its trigger while a linked menu was open.
    Hover,
    /// Anything else: a press, a key.
    Asking,
}

/// A link between dropdown menus that makes them one menu bar. Get one with
/// [`Builder::menu_link`] and join each dropdown to it with
/// [`Chain::linked`]; the menus' order is the order they are declared in.
///
/// Opening a linked menu closes the others. While one is open, the pointer on
/// another's trigger opens that one, and Left and Right in the open menu open
/// the previous and next enabled one, wrapping as [`wrap`](Self::wrap) says.
/// A press outside a linked menu reaches what is under it, so a press on
/// another trigger opens that one too. A linked menu takes the focus however
/// it is opened, and gives it back to its own trigger when it closes. A
/// linked trigger is a `Role::MenuItem`, an item of a
/// [`Nav::Row`] group around the triggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuLink {
    id: Id,
    wrap: Wrap,
}

impl MenuLink {
    /// What Left and Right do in the first and last menu. [`Wrap::Around`]
    /// unless set.
    #[must_use]
    pub fn wrap(self, wrap: Wrap) -> Self {
        Self { wrap, ..self }
    }
}

/// What an item does to the submenu open beside it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owns {
    Nothing,
    Submenu,
}

impl Menus {
    pub(crate) fn is_open(&self, root: Id) -> bool {
        self.open.contains_key(&root)
    }

    fn open(&mut self, root: Id, at: Option<Pos2>) {
        self.open_by(root, at, OpenedBy::Asking);
    }

    /// Opens the menu of `root`, closing the others linked to it.
    fn open_by(&mut self, root: Id, at: Option<Pos2>, by: OpenedBy) {
        for sibling in self.links.siblings(root) {
            self.close(sibling);
        }
        self.open.insert(
            root,
            Open {
                at,
                submenus: Vec::new(),
                by,
            },
        );
    }

    fn close(&mut self, root: Id) {
        self.open.remove(&root);
    }

    /// Opens the linked menu `switch` asks for, if any, with its submenus
    /// closed. Returns whether anything changed.
    fn switch(&mut self, switch: Switch) -> bool {
        let Some(to) = self.links.target(switch, |trigger| self.is_open(trigger)) else {
            return false;
        };
        let by = match switch {
            Switch::Hover(_) => OpenedBy::Hover,
            Switch::Step { .. } => OpenedBy::Asking,
        };
        self.open_by(to, None, by);
        true
    }

    /// A click on the trigger `root`. It keeps open a menu the pointer opened
    /// on its way to the trigger, so that a press on another trigger of a bar
    /// switches to that menu rather than closing it.
    fn toggle(&mut self, root: Id) {
        match self.open.get_mut(&root) {
            Some(open) if open.by == OpenedBy::Hover => open.by = OpenedBy::Asking,
            Some(_) => self.close(root),
            None => self.open(root, None),
        }
    }

    fn position(&self, root: Id) -> Option<Pos2> {
        self.open.get(&root)?.at
    }

    fn submenu(&self, root: Id, depth: usize) -> Option<Id> {
        self.open.get(&root)?.submenus.get(depth).copied()
    }

    fn open_submenu(&mut self, root: Id, depth: usize, item: Id) {
        if let Some(open) = self.open.get_mut(&root) {
            open.submenus.truncate(depth);
            open.submenus.push(item);
        }
    }

    fn close_submenu(&mut self, root: Id, depth: usize) {
        if let Some(open) = self.open.get_mut(&root) {
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

/// Whose sheet parts a menu's nodes are drawn as: the dropdown menu's, or a
/// menu bar's, whose items shadcn styles apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Anatomy {
    Dropdown,
    #[cfg(feature = "menubar")]
    Menubar,
}

impl Anatomy {
    /// The classes of `part` for a node with `keys`, as this anatomy draws it.
    #[track_caller]
    fn part<A: 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        part: Part,
        keys: &[Key],
        call_site: Tw,
    ) -> Tw {
        match self {
            Self::Dropdown => b.part(part, keys, call_site),
            #[cfg(feature = "menubar")]
            Self::Menubar => b.part(crate::owned::menubar::Part::from(part), keys, call_site),
        }
    }

    /// How long the menu takes to leave, where it is not the style's: a menu
    /// bar's vanishes at once, as shadcn's `menubar-content` has no exit
    /// animation, so a switch never shows two menus.
    fn exit(self) -> Option<std::time::Duration> {
        match self {
            Self::Dropdown => None,
            #[cfg(feature = "menubar")]
            Self::Menubar => Some(std::time::Duration::ZERO),
        }
    }
}

/// What Left or Right on a menu item asks for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sideways {
    Nothing,
    OpenSubmenu,
    Switch(nav::Step),
}

/// What every part of one open menu chain shares.
#[derive(Clone)]
struct Shared {
    root: Id,
    link: Option<MenuLink>,
    anatomy: Anatomy,
    style: Arc<Style>,
    kept: Handle,
    ctx: Context,
}

impl Shared {
    fn of<A>(b: &Builder<'_, '_, A>, root: Id) -> Self {
        Self {
            root,
            link: None,
            anatomy: Anatomy::Dropdown,
            style: b.style().clone(),
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
}

/// How an item is coloured.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Plain,
    Destructive,
}

impl MenuStyle {
    /// The height of one item in the text role `text`.
    pub fn row_height(&self, text: &TextSpec) -> f32 {
        text.line_height + 2.0 * self.item_padding.y
    }
}

/// What an item shows before its label.
#[derive(Clone)]
enum Lead {
    Nothing,
    Inset,
    Icon(Icon),
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
    Run(Act<A>),
    OpenSubmenu,
}

struct Row<A> {
    slot: Slot,
    label: String,
    role: Role,
    lead: Lead,
    trail: Trail,
    tone: Tone,
    opts: Options,
    on_response: OnResponse<A>,
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

    /// An item for the command `id` names in the [`Commands`](crate::Commands)
    /// declared on the builder: its label is the command's title, it shows the
    /// command's shortcut as the platform writes it, it is disabled while the
    /// command is, and activating it runs the command, which closes the menu.
    /// A [`checked`](crate::Command::checked) command's item has a check mark
    /// and a [`chosen`](crate::Command::chosen) one's a dot. Finish it with
    /// [`add`](ItemChain::add).
    ///
    /// If no live table declares `id`, the item is disabled, says `id`, and
    /// is reported as [`UnknownCommand`](crate::Problem::UnknownCommand).
    #[track_caller]
    pub fn command<K: crate::Key>(&mut self, id: K) -> ItemChain<'_, 'f, A, CommandItem<A>> {
        let Some(bound) = self.b.command(&id) else {
            let readable = format!("{id:?}");
            let item = self.chain(
                id,
                readable.clone(),
                CommandItem::new(|_| {}),
                Role::MenuItem,
                Lead::Nothing,
            );
            crate::report::report(
                item.b.ctx(),
                crate::Problem::UnknownCommand,
                Some(item.slot.id),
                || {
                    format!(
                        "menu item for {readable}: no live command table declares it; it is \
                         disabled"
                    )
                },
            );
            return item.allowed(Enablement::Disabled);
        };
        let Bound {
            command,
            enablement,
            state,
        } = bound;
        let keys = command.shortcut_text(self.b.ctx());
        let shell = self.b.shell();
        let (role, lead) = match state {
            CommandState::Plain => (Role::MenuItem, Lead::Nothing),
            CommandState::Checked(mark) => (Role::MenuItemCheckBox, Lead::Check(mark)),
            CommandState::Chosen(mark) => (Role::MenuItemRadio, Lead::Dot(mark)),
        };
        let run = CommandItem::new(move |app| command.run(app, &shell));
        let mut item = self
            .chain(id, command.title().to_owned(), run, role, lead)
            .allowed(enablement);
        item.shortcut = keys;
        item
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
            opts: Options::new(),
            on_response: OnResponse::default(),
            allowed: Enablement::Enabled,
            kind,
        }
    }

    /// A line between groups of items.
    pub fn separator(&mut self) {
        let classes = self
            .shared
            .anatomy
            .part(&mut self.b, Part::Separator, &[], tw());
        self.b.div(classes, |_| {});
    }

    /// A heading over a group of items. It takes no input.
    pub fn label(&mut self, text: impl Into<String>) {
        let classes = self
            .shared
            .anatomy
            .part(&mut self.b, Part::Label, &[], tw());
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
        let overlay = self.b.slot(&(key.clone(), Node::Submenu));
        let slot = self.b.slot(&key);
        let item = slot.id;
        self.add_row(Row {
            slot,
            label: label.clone(),
            role: Role::MenuItem,
            lead: Lead::Nothing,
            trail: Trail::Chevron,
            tone: Tone::Plain,
            opts: Options::new(),
            on_response: OnResponse::default(),
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
            .nav(Nav::Submenu(Wrap::Around))
            .outside(Outside::PassThrough)
            .role(Role::Menu)
            .named(label)
            .on_dismiss(move |_| close.change(|menus| menus.close_submenu(root, depth)))
            .children_fitted(|s, fit| menu_box(s, &shared, ((depth + 1, tw()), fit), items));
    }

    fn add_row(&mut self, row: Row<A>) {
        let Row {
            slot,
            label,
            role,
            lead,
            trail,
            tone,
            opts,
            on_response,
            reaction,
        } = row;
        let Resolved {
            name,
            classes: caller,
            enablement,
            tooltip: tip,
            autofocus,
            ..
        } = opts.resolve(&mut self.b, slot.id);
        let name = name.unwrap_or_else(|| label.clone());
        let shared = self.shared.clone();
        let part = match (&lead, &trail) {
            (_, Trail::Chevron) => Part::SubTrigger,
            (Lead::Check(_), _) => Part::CheckboxItem,
            (Lead::Dot(_), _) => Part::RadioItem,
            _ => Part::Item,
        };
        let (root, depth, id) = (shared.root, self.depth, slot.id);
        let expanded = shared.kept.with(|kept| kept.menus.submenu(root, depth)) == Some(id);
        let mut keys = Part::item_keys(tone, enablement);
        if matches!(lead, Lead::Inset) {
            keys.push(Key::Attr(INSET.0, INSET.1));
        }
        keys.extend(super::expanded_key(expanded));
        let own = enablement.senses();
        let anatomy = shared.anatomy;
        let classes = anatomy.part(&mut self.b, part, &keys, own.merge(caller));
        let shown = label;
        let owns = match reaction {
            Reaction::Run(_) => Owns::Nothing,
            Reaction::OpenSubmenu => Owns::Submenu,
        };
        let mark = match &lead {
            Lead::Nothing | Lead::Inset | Lead::Icon(_) => None,
            Lead::Check(mark) | Lead::Dot(mark) => Some(*mark),
        };
        self.b
            .div_id_with(&slot, classes, |b| {
                match lead {
                    Lead::Nothing | Lead::Inset => {}
                    Lead::Icon(icon) => {
                        b.icon("icon", tw(), icon);
                    }
                    Lead::Check(mark) | Lead::Dot(mark) => {
                        let classes = anatomy.part(b, Part::ItemIndicator, &[], tw());
                        b.div(classes, |b| {
                            if mark == Mark::On {
                                b.glyph(tw(), check());
                            }
                        });
                    }
                }
                b.text(tw(), shown);
                match trail {
                    Trail::Nothing => {}
                    Trail::Shortcut(keys) => {
                        let classes = anatomy.part(b, Part::Shortcut, &[], tw());
                        b.text(classes, keys);
                    }
                    Trail::Chevron => {
                        b.glyph(tw().ml_auto(), Chevron::Right.icon());
                    }
                }
            })
            .role(role)
            .named(name.clone())
            .open_focus(autofocus.open_focus())
            .draw(move |app, ui| {
                let response = tipped(ui, ui.node_response(), &tip);
                describe(&response, ui.is_enabled().into(), role, mark, &name);
                autofocus.apply(ui.ctx(), &response);
                on_response.run(app, &response);
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
                let activated = response.clicked() || nav::take_activation(ui, id);
                let sideways = shared.sideways(ui, &response, (depth, owns));
                if let Sideways::Switch(step) = sideways {
                    shared.change_if(|menus| menus.switch(Switch::Step { from: root, step }));
                }
                match reaction {
                    Reaction::Run(act) => {
                        if activated {
                            act(app, ui.ctx());
                            shared.close();
                        }
                    }
                    Reaction::OpenSubmenu => {
                        if activated || sideways == Sideways::OpenSubmenu {
                            shared.change(|menus| menus.open_submenu(root, depth, id));
                        }
                    }
                }
            });
    }
}

impl Shared {
    /// What Left or Right on the focused item at `depth` asks for, taking the
    /// key. Right opens an item's own submenu; elsewhere in a linked menu it
    /// opens the next menu of the bar, at any depth, as Left at the top level
    /// opens the previous one. Left in a submenu is the submenu's.
    fn sideways(
        &self,
        ui: &egui::Ui,
        response: &egui::Response,
        (depth, owns): (usize, Owns),
    ) -> Sideways {
        if !response.has_focus() {
            return Sideways::Nothing;
        }
        let linked = self.link.is_some();
        let pressed = |key| super::number::pressed(ui, key);
        if (owns == Owns::Submenu || linked) && pressed(EguiKey::ArrowRight) {
            return match owns {
                Owns::Submenu => Sideways::OpenSubmenu,
                Owns::Nothing => Sideways::Switch(nav::Step::Next),
            };
        }
        if linked && depth == 0 && pressed(EguiKey::ArrowLeft) {
            return Sideways::Switch(nav::Step::Previous);
        }
        Sideways::Nothing
    }

    /// Like [`change`](Self::change), for a change that reports whether it
    /// did anything.
    fn change_if(&self, change: impl FnOnce(&mut Menus) -> bool) {
        if self.kept.with(|kept| change(&mut kept.menus)) {
            self.ctx.request_repaint();
        }
    }
}

/// The box of a menu at `depth`, with `items` in it and `caller` merged over
/// its classes.
fn menu_box<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    shared: &Shared,
    ((depth, caller), fit): ((usize, Tw), Fit),
    items: impl for<'m> FnOnce(&mut Menu<'m, 'f, A>),
) {
    let part = if depth == 0 {
        Part::Content
    } else {
        Part::SubContent
    };
    let classes = shared.anatomy.part(&mut s.reborrow(), part, &[], caller);
    let shared = shared.clone();
    super::fitted_box(s, fit, (Node::Box, Node::Rows), classes, |s| {
        items(&mut Menu {
            b: s.reborrow(),
            shared,
            depth,
        });
    });
}

/// An item of a menu, started with [`Menu::item`], [`Menu::checkbox_item`],
/// [`Menu::radio_item`] or [`Menu::command`] and finished with the handler of
/// its kind.
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its classes are the item's row's, and its
/// accessible name is its label unless it is [`named`](Configure::named). A
/// command's item takes input only while its command is enabled too, whatever
/// its enablement or refusal say.
#[must_use = "an item is not added until you call `.on_click(..)`, `.on_change(..)` or `.add()`"]
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
    opts: Options,
    on_response: OnResponse<A>,
    /// Whether the item may take input at all, whatever its own enablement
    /// says: a command item's command.
    allowed: Enablement,
    kind: Kind,
}

/// What activating an item runs.
type Act<A> = Box<dyn FnOnce(&mut A, &Context)>;

/// The kind of an item that runs a handler.
pub struct Action;
/// The kind of an item with a check mark, on or off as it was built.
pub struct Checkbox(Mark);
/// The kind of an item with a dot.
pub struct Radio;
/// TODO(prose): the kind of an item for a command, which runs the command.
pub struct CommandItem<A>(Box<dyn FnOnce(&mut A)>);

impl<A> CommandItem<A> {
    fn new(run: impl FnOnce(&mut A) + 'static) -> Self {
        Self(Box::new(run))
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Action> {
    /// TODO(prose): leaves the width of an icon before the label, to line up with items that have one.
    pub fn inset(self) -> Self {
        self.lead_unless_marked(Lead::Inset)
    }

    /// TODO(prose): `icon` before the label, in the item's text colour.
    pub fn icon(self, icon: Icon) -> Self {
        self.lead_unless_marked(Lead::Icon(icon))
    }

    /// The keys that do the same thing, shown after the label. Display only:
    /// the application handles the keys.
    pub fn shortcut(self, keys: impl Into<String>) -> Self {
        self.keys(keys.into())
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Checkbox> {
    /// The keys that do the same thing, as [`ItemChain::shortcut`] of an
    /// action item.
    pub fn shortcut(self, keys: impl Into<String>) -> Self {
        self.keys(keys.into())
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Radio> {
    /// The keys that do the same thing, as [`ItemChain::shortcut`] of an
    /// action item.
    pub fn shortcut(self, keys: impl Into<String>) -> Self {
        self.keys(keys.into())
    }
}

impl<A: 'static> ItemChain<'_, '_, A, CommandItem<A>> {
    /// TODO(prose): leaves the width of an icon before the label; a checked or chosen command's item keeps its mark there.
    pub fn inset(self) -> Self {
        self.lead_unless_marked(Lead::Inset)
    }

    /// TODO(prose): `icon` before the label; a checked or chosen command's item keeps its mark there.
    pub fn icon(self, icon: Icon) -> Self {
        self.lead_unless_marked(Lead::Icon(icon))
    }

    /// TODO(prose): adds the item; activating it runs the command, and then the menu closes.
    #[track_caller]
    pub fn add(self) {
        self.finish_with(|CommandItem(run)| move |app: &mut A, _: &Context| run(app));
    }
}

impl<A: 'static, Kind> ItemChain<'_, '_, A, Kind> {
    fn lead_unless_marked(mut self, lead: Lead) -> Self {
        if !matches!(self.lead, Lead::Check(_) | Lead::Dot(_)) {
            self.lead = lead;
        }
        self
    }

    fn keys(mut self, keys: String) -> Self {
        self.shortcut = Some(keys);
        self
    }

    fn allowed(mut self, allowed: Enablement) -> Self {
        self.allowed = allowed;
        self
    }

    /// Draws the item for an action that cannot be undone.
    pub fn destructive(mut self) -> Self {
        self.tone = Tone::Destructive;
        self
    }

    #[track_caller]
    fn finish(self, handler: impl FnOnce(&mut A, &Context) + 'static) {
        self.finish_with(|_| handler);
    }

    #[track_caller]
    fn finish_with<H: FnOnce(&mut A, &Context) + 'static>(self, handler: impl FnOnce(Kind) -> H) {
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
            mut opts,
            on_response,
            allowed,
            kind,
        } = self;
        if allowed == Enablement::Disabled {
            opts.set_enablement(Enablement::Disabled);
        }
        Menu { b, shared, depth }.add_row(Row {
            slot,
            label,
            role,
            lead,
            trail: shortcut.map_or(Trail::Nothing, Trail::Shortcut),
            tone,
            opts,
            on_response,
            reaction: Reaction::Run(Box::new(handler(kind))),
        });
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Action> {
    /// Adds the item. `act` runs when it is clicked or activated with Enter
    /// or Space, and then the menu closes.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) {
        self.finish(move |app, _| act(app));
    }

    /// Like [`on_click`](Self::on_click), with a handler that also gets the
    /// egui context, for an effect the context performs, such as copying
    /// text.
    #[track_caller]
    pub fn on_click_with(self, act: impl FnOnce(&mut A, &Context) + 'static) {
        self.finish(act);
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Radio> {
    /// Adds the item. `act` runs when it is activated, and then the menu
    /// closes.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) {
        self.finish(move |app, _| act(app));
    }

    /// Like [`on_click`](Self::on_click), with a handler that also gets the
    /// egui context, for an effect the context performs, such as copying
    /// text.
    #[track_caller]
    pub fn on_click_with(self, act: impl FnOnce(&mut A, &Context) + 'static) {
        self.finish(act);
    }
}

impl<A: 'static> ItemChain<'_, '_, A, Checkbox> {
    /// Adds the item. `change` runs with the value the item moves to when it
    /// is activated, and then the menu closes.
    #[track_caller]
    pub fn on_change(self, change: impl FnOnce(&mut A, Mark) + 'static) {
        self.on_change_with(move |app, _, mark| change(app, mark));
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, change: impl FnOnce(&mut A, &Context, Mark) + 'static) {
        let moves_to = match self.kind.0 {
            Mark::On => Mark::Off,
            Mark::Off => Mark::On,
        };
        self.finish(move |app, ctx| change(app, ctx, moves_to));
    }
}

impl<A, Kind> Configure for ItemChain<'_, '_, A, Kind> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, Kind> Interactive for ItemChain<'_, '_, A, Kind> {}

impl<A, Kind> Respond for ItemChain<'_, '_, A, Kind> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
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
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its options are the trigger's; a trigger that
/// becomes disabled or refused closes its menu.
#[must_use = "a dropdown menu is not added until you call `.items(..)`"]
pub struct Chain<'a, 'f, A, K> {
    b: Builder<'a, 'f, A>,
    key: K,
    face: Face,
    options: button::Options,
    offset: Option<Distance>,
    align_shift: Distance,
    link: Option<MenuLink>,
    bar_trigger: Option<BarTrigger<'a, 'f, A>>,
    on_response: OnResponse<A>,
    children: Option<Body<'a, 'f, A>>,
    trail: TriggerTrail,
    menu_classes: Tw,
}

/// How a menu bar draws a dropdown: its trigger as the bar's sheet's `part`,
/// inside the bar's node `owner` for assistive technology, and its menu as
/// `anatomy` says.
struct BarTrigger<'a, 'f, A> {
    part: button::PartOf<'a, 'f, A>,
    owner: Id,
    anatomy: Anatomy,
}

/// What a dropdown trigger's [`children`](Chain::children) build.
type Body<'a, 'f, A> = Box<dyn for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'a>;

/// What a dropdown trigger shows after its content.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TriggerTrail {
    Nothing,
    Chevron,
}

impl<'a, 'f, A: 'static, K: crate::Key + Clone> Chain<'a, 'f, A, K> {
    pub(super) fn new(b: Builder<'a, 'f, A>, key: K, face: Face) -> Self {
        Self {
            b,
            key,
            face,
            options: button::Options::new(),
            offset: None,
            align_shift: Distance::ZERO,
            link: None,
            bar_trigger: None,
            on_response: OnResponse::default(),
            children: None,
            trail: TriggerTrail::Nothing,
            menu_classes: tw(),
        }
    }

    /// TODO(prose): builds `body`'s nodes inside the trigger, after its icon and label, as [`button::Chain::children`] does.
    pub fn children(mut self, body: impl for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'a) -> Self {
        self.children = Some(Box::new(body));
        self
    }

    /// TODO(prose): a down chevron after the trigger's content, sized and toned as a select's.
    pub fn chevron(mut self) -> Self {
        self.trail = TriggerTrail::Chevron;
        self
    }

    /// TODO(prose): classes merged over the styled ones of the menu's box ([`Tw::merge`]), such as its width.
    pub fn menu_classes(mut self, classes: Tw) -> Self {
        self.menu_classes = self.menu_classes.merge(classes);
        self
    }

    /// Applies shared button `options` to the trigger.
    pub fn with_options(mut self, options: button::Options) -> Self {
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

    /// The gap between the trigger and its menu, in place of the style's
    /// [`MenuStyle::offset`](crate::contract::MenuStyle::offset).
    pub fn offset(mut self, offset: Distance) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Moves the menu along the trigger's edge, away from the edge they are
    /// aligned at, as Radix's `alignOffset` does; a negative shift moves it
    /// the other way. None unless set.
    pub fn align_shift(mut self, shift: Distance) -> Self {
        self.align_shift = shift;
        self
    }

    /// Joins the dropdown to the menu bar `link`, after the dropdowns joined
    /// before it this frame. The trigger becomes a `Role::MenuItem`; see
    /// [`MenuLink`] for what the link does.
    pub fn linked(mut self, link: MenuLink) -> Self {
        self.link = Some(link);
        self
    }

    /// Draws the dropdown as a menu bar's: the trigger's box as `part` of the
    /// sheet, in place of a button, inside the node `owner` for assistive
    /// technology, and the menu with the bar's parts.
    #[cfg(feature = "menubar")]
    pub(crate) fn trigger_of<P: crate::sheet::Part>(mut self, part: P, owner: Id) -> Self {
        self.bar_trigger = Some(BarTrigger {
            part: button::part_of(part),
            owner,
            anatomy: Anatomy::Menubar,
        });
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
            offset,
            align_shift,
            link,
            bar_trigger,
            on_response,
            children,
            trail,
            menu_classes,
        } = self;
        let shared = Shared {
            link,
            anatomy: bar_trigger
                .as_ref()
                .map_or(Anatomy::Dropdown, |bar| bar.anatomy),
            ..Shared::of(&b, b.slot(&key).id)
        };
        let trigger = shared.root;
        let list = b.slot(&(key.clone(), Node::Menu));
        let enablement = b.enablement_of(options.own_enablement().and(options.refusal()));
        if let Some(link) = link {
            let pass = b.ctx().cumulative_pass_nr();
            let member = Member {
                trigger,
                enablement,
            };
            shared.kept.with(|kept| {
                kept.menus.links.join((link.id, link.wrap), pass, member);
            });
        }
        if enablement == Enablement::Disabled {
            shared.close();
        }
        let open = shared.kept.with(|kept| kept.menus.is_open(trigger));
        let name = options.name_or(&face).to_owned();
        let select = &shared.style.select;
        let (chevron, opacity) = (select.chevron, select.chevron_opacity);
        let button = button::Chain::new(b.reborrow(), key, face)
            .with_options(options)
            .opens_popup()
            .expanded(open)
            .enablement(enablement);
        let button = match link {
            Some(_) => button.role(Role::MenuItem),
            None => button,
        };
        let button = match bar_trigger {
            Some(BarTrigger { part, owner, .. }) => button.drawn_as(part).owned_by(owner),
            None => button,
        };
        let button = match (children, trail) {
            (None, TriggerTrail::Nothing) => button,
            (children, trail) => button.children(move |s| {
                if let Some(children) = children {
                    children(s);
                }
                if trail == TriggerTrail::Chevron {
                    s.glyph(
                        tw().size_px(chevron).shrink_0().opacity(opacity),
                        Chevron::Down.icon(),
                    );
                }
            }),
        };
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
                let pointer_moved = ctx.input(|input| input.pointer.delta() != Vec2::ZERO);
                if on_key.link.is_some() && pointer_moved && response.hovered() {
                    on_key.change_if(|menus| menus.switch(Switch::Hover(trigger)));
                }
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
        let close = shared.clone();
        let placement = Placement::below()
            .gap(offset.unwrap_or(shared.style.menu.offset))
            .shift(align_shift);
        let menu = OverlayChain::new(b.reborrow(), list)
            .anchor(Anchor::Node(trigger))
            .placement(placement)
            .shown(if open { Shown::Open } else { Shown::Closed })
            .nav(Nav::Menu(Wrap::Around))
            .role(Role::Menu)
            .named(name)
            .on_dismiss(move |_| close.close())
            .when_some(shared.anatomy.exit(), |menu, exit| {
                menu.exit_transition(exit)
            });
        let menu = match link {
            Some(_) => menu
                .outside(Outside::PassThrough)
                .pointer_open(PointerOpen::Enters)
                .returns_to(ReturnsTo::Widget(trigger)),
            None => menu,
        };
        menu.children_fitted(|s, fit| menu_box(s, &shared, ((0, menu_classes), fit), items));
    }
}

impl crate::testing::PlacedFrame {
    /// TODO(prose): whether the menu of the dropdown declared with `key` at the top of the view is open; false from the frame it closes, while it fades out.
    pub fn menu_open<K: crate::Key>(&self, key: K) -> bool {
        self.menu_open_with(crate::child_id(self.root, &key))
    }

    /// TODO(prose): whether the menu the node `id` opens is open, a dropdown's by its trigger or a context menu's by its node.
    pub fn menu_open_with(&self, id: Id) -> bool {
        self.kept.with(|kept| kept.menus.is_open(id))
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
    let slot = b
        .slot(&(Node::Context, owner))
        .relabelled(format!("ContextMenu({owner_path})"));
    let close = shared.clone();
    OverlayChain::new(b.reborrow(), slot)
        .anchor(Anchor::Point(at))
        .placement(Placement::menu())
        .nav(Nav::Menu(Wrap::Around))
        .role(Role::Menu)
        .named("Context menu")
        .on_dismiss(move |_| close.close())
        .children_fitted(|s, fit| menu_box(s, &shared, ((0, tw()), fit), items));
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

impl<A, K> Configure for Chain<'_, '_, A, K> {
    fn options(&mut self) -> &mut Options {
        self.options.options()
    }
}

impl<A, K> Interactive for Chain<'_, '_, A, K> {}

impl<A, K> Respond for Chain<'_, '_, A, K> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A button that opens a menu under it, which the chain's `items` fills.
    /// Assistive technology sees the trigger as a `Role::Button`, the menu as a
    /// `Role::Menu` and its items as `Role::MenuItem`, `MenuItemCheckBox` or
    /// `MenuItemRadio`.
    pub fn dropdown_menu<K: crate::Key + Clone>(
        &mut self,
        key: K,
        face: impl Into<Face>,
    ) -> Chain<'_, 'f, A, K> {
        Chain::new(self.reborrow(), key, face.into())
    }

    /// A link under `key` that makes the dropdown menus joined to it with
    /// [`Chain::linked`] one menu bar. Declare it each frame, before its
    /// menus.
    pub fn menu_link<K: crate::Key>(&self, key: K) -> MenuLink {
        MenuLink {
            id: self.id_of(key),
            wrap: Wrap::Around,
        }
    }

    /// Whether the menu the node `id` opens is open, a dropdown's by its
    /// trigger or a context menu's by its node, as
    /// [`PlacedFrame::menu_open_with`](crate::testing::PlacedFrame::menu_open_with)
    /// says after the frame: for a trigger's open look.
    pub fn menu_open_with(&self, id: Id) -> bool {
        self.kept().with(|kept| kept.menus.is_open(id))
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
