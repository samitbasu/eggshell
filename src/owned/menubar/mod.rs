//! TODO(prose): a menu bar, after shadcn/ui's Menubar: a row of dropdown
//! menus that the pointer and the arrow keys move between while one is open.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::prelude::*;
//!
//! fn view(b: &mut Builder<'_, '_, ()>) {
//!     b.menubar("menubar").menus(|bar| {
//!         bar.menu("file", "File").items(|m| {
//!             m.item("new", "New").on_click(|_| {});
//!             m.separator();
//!             m.item("quit", "Quit").on_click(|_| {});
//!         });
//!         bar.menu("edit", "Edit").items(|m| {
//!             m.item("undo", "Undo").on_click(|_| {});
//!         });
//!     });
//! }
//! ```

use egui::Role;
use egui::accesskit::Orientation;

use crate::contract::Variant;
use crate::overlay::{Distance, Nav, Wrap};
use crate::owned::button::Face;
use crate::owned::menu::{self, MenuLink};
use crate::owned::options::{Configure, Options, Resolved};
use crate::owned::tipped;
use crate::sheet::{Sheet, When};
use crate::theme::ShadcnStyle;
use crate::{Builder, Key, NodeUi as _, tw};

/// TODO(prose): a menu bar's anatomy for the style's sheet: shadcn's
/// `menubar`. The bar draws its menus' items as the dropdown menu's
/// ([`menu::Part`]); its own item parts are here for the look review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the bar's box.
    Root,
    /// TODO(prose): a menu's trigger.
    Trigger,
    /// TODO(prose): the box of an open menu.
    Content,
    /// TODO(prose): an item that runs a handler.
    Item,
    /// TODO(prose): an item with a check mark.
    CheckboxItem,
    /// TODO(prose): the square a check item keeps before its label.
    CheckboxItemIndicator,
    /// TODO(prose): an item with a dot.
    RadioItem,
    /// TODO(prose): the square a radio item keeps before its label.
    RadioItemIndicator,
    /// TODO(prose): a heading over a group of items.
    Label,
    /// TODO(prose): a line between groups.
    Separator,
    /// TODO(prose): the keys an item shows after its label.
    Shortcut,
    /// TODO(prose): an item that opens a submenu.
    SubTrigger,
    /// TODO(prose): the box of a submenu.
    SubContent,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "menubar.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "menubar",
            Self::Trigger => "menubar-trigger",
            Self::Content => "menubar-content",
            Self::Item => "menubar-item",
            Self::CheckboxItem => "menubar-checkbox-item",
            Self::CheckboxItemIndicator => "menubar-checkbox-item-indicator",
            Self::RadioItem => "menubar-radio-item",
            Self::RadioItemIndicator => "menubar-radio-item-indicator",
            Self::Label => "menubar-label",
            Self::Separator => "menubar-separator",
            Self::Shortcut => "menubar-shortcut",
            Self::SubTrigger => "menubar-sub-trigger",
            Self::SubContent => "menubar-sub-content",
        }
    }
}

/// Writes the menu bar's recorded exceptions over its generated look.
///
/// Lyra's trigger is `py-[calc(--spacing(0.8))]`, which the importer cannot
/// map yet. An item of the default variant keeps the generated look, which
/// names only the destructive one; naming it says so.
pub(crate) fn write_exceptions(sheet: &mut Sheet) {
    sheet.rule(Part::Trigger, |t, k| match k.style {
        ShadcnStyle::Vega => t,
        ShadcnStyle::Lyra => t.py(0.8),
    });
    sheet.rule_when(
        Part::Item,
        When::always().variant(Variant::DEFAULT),
        |t, _| t,
    );
}

/// Where a bar's menu opens: shadcn's `MenubarContent` defaults,
/// `sideOffset={8}` below the trigger and `alignOffset={-4}` along it.
const SIDE_OFFSET: f32 = 8.0;
const ALIGN_OFFSET: f32 = -4.0;

/// TODO(prose): adds [`menubar`](Self::menubar) to the builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): a menu bar under `key`. Add its menus with
    /// [`menus`](Chain::menus).
    fn menubar<'s, K: Key>(&'s mut self, key: K) -> Chain<'s, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn menubar<'s, K: Key>(&'s mut self, key: K) -> Chain<'s, 'b, 'f, A, K> {
        Chain {
            b: self,
            key,
            opts: Options::new(),
        }
    }
}

/// TODO(prose): a menu bar, after shadcn/ui's Menubar. Start one with
/// [`Ext::menubar`] and finish it with [`menus`](Self::menus).
///
/// The bar is one Tab stop: Left and Right move between its triggers, and
/// Enter, Space, Down or a click opens a menu, with the focus on its first
/// item. While a menu is open, the pointer on another trigger opens that
/// one, and Left and Right open the neighbouring menus; Esc closes the menu
/// and focuses its trigger. Assistive technology sees a `Role::MenuBar`,
/// named "Menu bar" unless [`named`](Configure::named), and each trigger as
/// a `Role::MenuItem`.
///
/// The bar takes no input of its own, so of the shared options it has only
/// [`Configure`]'s, which are the bar's box's; each menu's are its trigger's,
/// set on the chain [`Bar::menu`] returns.
///
/// ```html
/// <Menubar class="flex h-9 items-center gap-1 rounded-md border p-1 shadow-xs">
///   <MenubarMenu>
///     <MenubarTrigger class="rounded-sm px-2 py-1 text-sm font-medium hover:bg-muted aria-expanded:bg-muted">File</MenubarTrigger>
///     <MenubarContent align="start" alignOffset={-4} sideOffset={8}>…</MenubarContent>
///   </MenubarMenu>
/// </Menubar>
/// ```
#[must_use = "a menu bar is not added until you call `.menus(..)`"]
pub struct Chain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    opts: Options,
}

impl<A, K> Configure for Chain<'_, '_, '_, A, K> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

/// The parts of a menu bar that are not nodes.
#[derive(Hash, Debug)]
enum Node {
    Link,
}

impl<'f, A: 'static, K: Key + Clone> Chain<'_, '_, 'f, A, K> {
    /// TODO(prose): adds the bar, with the menus `menus` adds with
    /// [`Bar::menu`], in order.
    #[track_caller]
    pub fn menus(self, menus: impl for<'m, 'c> FnOnce(&mut Bar<'m, 'c, 'f, A>)) {
        let Self { b, key, opts } = self;
        let link = b.menu_link((key.clone(), Node::Link));
        let slot = b.slot(&key);
        let bar = slot.id;
        let Resolved {
            name,
            classes,
            tooltip,
            ..
        } = opts.resolve(b, bar);
        // A node shows its role to assistive technology only if it senses.
        let own = tw().sense_hover();
        let classes = b.part(Part::Root, &[], own.merge(classes));
        b.div_id_with(&slot, classes, |b| menus(&mut Bar { b, link, bar }))
            .role(Role::MenuBar)
            .named(name.unwrap_or_else(|| "Menu bar".to_owned()))
            .nav(Nav::Row(Wrap::Around))
            .draw(move |_, ui| {
                let id = tipped(ui, ui.node_response(), &tooltip).id;
                ui.ctx().accesskit_node_builder(id, |node| {
                    node.set_orientation(Orientation::Horizontal);
                });
            });
    }
}

/// TODO(prose): the menus of a menu bar, which the body of
/// [`Chain::menus`] adds.
pub struct Bar<'m, 'c, 'f, A> {
    b: &'m mut Builder<'c, 'f, A>,
    link: MenuLink,
    /// The bar's node.
    bar: egui::Id,
}

impl<'f, A: 'static> Bar<'_, '_, 'f, A> {
    /// TODO(prose): the next menu of the bar, under `key`, its trigger
    /// showing `face`: the dropdown menu, linked to the bar and drawn as its
    /// trigger, which [`items`](menu::Chain::items) fills and adds.
    pub fn menu<K: Key + Clone>(
        &mut self,
        key: K,
        face: impl Into<Face>,
    ) -> menu::Chain<'_, 'f, A, K> {
        let content = self.b.resolved_part(Part::Content, &[]);
        self.b
            .dropdown_menu(key, face)
            .linked(self.link)
            .trigger_of(Part::Trigger, self.bar)
            .offset(Distance::new(SIDE_OFFSET))
            .align_shift(Distance::new(ALIGN_OFFSET))
            .menu_classes(content)
    }
}
