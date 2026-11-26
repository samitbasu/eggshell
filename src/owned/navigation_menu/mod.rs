//! TODO(prose): a navigation menu, after shadcn/ui's NavigationMenu: a row of
//! a site's top-level links and triggers, each trigger opening a panel of
//! links under it.
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui::prelude::*;
//!
//! #[derive(Clone, Copy, PartialEq)]
//! enum Page { Intro, Install, Docs }
//!
//! struct App { page: Page }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     let page = app.page;
//!     b.navigation_menu("site")
//!         .named("Main")
//!         .panel("start", "Getting started", move |p| {
//!             p.div(tw().grid().grid_cols(2).gap(2.0).w_px(500.0), |g| {
//!                 g.nav_link("intro", "Introduction")
//!                     .description("Re-usable components built with Tailwind.")
//!                     .current(page == Page::Intro)
//!                     .on_click(|app: &mut App| app.page = Page::Intro);
//!                 g.nav_link("install", "Installation")
//!                     .description("How to install dependencies.")
//!                     .current(page == Page::Install)
//!                     .on_click(|app: &mut App| app.page = Page::Install);
//!             });
//!         })
//!         .link("docs", "Docs", move |link| {
//!             link.current(page == Page::Docs)
//!                 .on_click(|app: &mut App| app.page = Page::Docs);
//!         })
//!         .add();
//! }
//! ```
//!
//! The panels are the library's, as a hover card's is: one opens when the
//! pointer rests on its trigger, or when the trigger is pressed, and while
//! one is open the pointer or the arrow keys on another trigger open that
//! one instead ([`TriggerGroup`]). Every trigger and link is a Tab stop, and
//! the arrow keys move along the row as well. Down on a trigger opens its
//! panel and moves focus into it; Esc closes it and gives the focus back to
//! the trigger. A link pressed in a panel closes it.

use core::time::Duration;

use egui::{Painter, Pos2, Rect, Role, Shape, pos2};

use crate::overlay::{Align, Distance, HoverIntent, Nav, Placement, TabStops, TriggerGroup, Wrap};
use crate::sheet::{Key as SheetKey, Sheet};
use crate::{Builder, Current, Icon, Key, NodeUi as _, Tw, icons, tw};

/// TODO(prose): a navigation menu's anatomy for the style's sheet: shadcn's
/// `navigation-menu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the menu: the `nav` landmark around the row.
    Root,
    /// TODO(prose): the row of items.
    List,
    /// TODO(prose): one item of the row: a trigger and its panel, or a link.
    Item,
    /// TODO(prose): a trigger, and a top-level link, which looks like one.
    Trigger,
    /// TODO(prose): the chevron after a trigger's label.
    TriggerIcon,
    /// TODO(prose): a panel's box.
    Content,
    /// TODO(prose): a link in a panel: [`nav_link`](Ext::nav_link).
    Link,
    /// TODO(prose): the box under the open trigger that holds the indicator.
    Indicator,
    /// TODO(prose): the indicator's arrow.
    IndicatorArrow,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "navigation-menu.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "navigation-menu",
            Self::List => "navigation-menu-list",
            Self::Item => "navigation-menu-item",
            Self::Trigger => "navigation-menu-trigger",
            Self::TriggerIcon => "navigation-menu-trigger-icon",
            Self::Content => "navigation-menu-content",
            Self::Link => "navigation-menu-link",
            Self::Indicator => "navigation-menu-indicator",
            Self::IndicatorArrow => "navigation-menu-indicator-arrow",
        }
    }
}

/// The key of a link marked current: shadcn's `data-active`.
const ACTIVE: SheetKey = SheetKey::Attr("active", "true");

/// Writes the navigation menu's recorded exceptions over its generated look.
///
/// The panel's box is an overlay the placement puts `mt-1.5` under its
/// trigger, so its own positioning classes (`absolute top-full left-0 w-full`,
/// `mt-1.5`) do not apply. `shadow` (Tailwind's, which is its `shadow-sm`)
/// has no `Tw` method of its own. A link in a panel is `rounded-sm` in Vega
/// (`in-data-[slot=navigation-menu-content]:rounded-sm`). The `focus:bg-muted`
/// of a trigger and a link is their keyboard focus's.
pub(crate) fn write_exceptions(sheet: &mut Sheet) {
    sheet.rule(Part::Content, |t, k| {
        t.relative()
            .top(0.0)
            .mt(0.0)
            .w_auto()
            .shadow(k.shadow_sm)
            .flex()
            .flex_col()
    });
    sheet.rule(Part::Trigger, |t, k| t.focus(|p| p.bg(k.muted)));
    sheet.rule(Part::Link, |t, k| {
        let inside = match k.style {
            crate::theme::ShadcnStyle::Vega => k.radius.sm(),
            crate::theme::ShadcnStyle::Lyra => 0.0,
        };
        t.focus(|p| p.bg(k.muted)).rounded_px(inside)
    });
}

/// TODO(prose): what the chevron after a trigger's label does while its
/// panel is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chevron {
    /// TODO(prose): it points up, as shadcn's turns 180°.
    #[default]
    Turns,
    /// TODO(prose): it keeps pointing down.
    Still,
}

/// TODO(prose): when navigation menus' panels open and close, and how they
/// enter: a [`Style`](crate::Style)'s extension.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// TODO(prose): how long the pointer rests on a trigger before its panel
    /// opens: Radix's `delayDuration`.
    pub open_delay: Duration,
    /// TODO(prose): how long after a panel closes another still opens at
    /// once: Radix's `skipDelayDuration`.
    pub skip_delay: Duration,
    /// TODO(prose): how long a panel stays once neither its trigger nor its
    /// box is hovered.
    pub close_delay: Duration,
    /// TODO(prose): the gap between a trigger and its panel: shadcn's
    /// `mt-1.5`.
    pub offset: Distance,
    /// TODO(prose): how long a panel takes to enter and to leave.
    pub transition: Duration,
    /// TODO(prose): the trigger's chevron while its panel is open.
    pub chevron: Chevron,
}

impl Default for Style {
    /// TODO(prose): Radix's delays (200 ms, 300 ms, and its 150 ms close
    /// timer), and shadcn's panel 6 pt under its trigger.
    fn default() -> Self {
        Self {
            open_delay: Duration::from_millis(200),
            skip_delay: Duration::from_millis(300),
            close_delay: Duration::from_millis(150),
            offset: Distance::new(6.0),
            transition: Duration::from_millis(150),
            chevron: Chevron::Turns,
        }
    }
}

/// TODO(prose): adds [`navigation_menu`](Self::navigation_menu) and
/// [`nav_link`](Self::nav_link) to the builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): a navigation menu named `key`: give it its items with
    /// [`panel`](Chain::panel) and [`link`](Chain::link), then
    /// [`add`](Chain::add) it.
    fn navigation_menu<K: Key>(&mut self, key: K) -> Chain<'_, 'b, 'f, A, K>;

    /// TODO(prose): a link reading `title`, shadcn's `NavigationMenuLink`,
    /// for a panel's content. Finish it with [`on_click`](Link::on_click).
    fn nav_link<K: Key>(&mut self, key: K, title: impl Into<String>) -> Link<'_, 'b, 'f, A, K>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn navigation_menu<K: Key>(&mut self, key: K) -> Chain<'_, 'b, 'f, A, K> {
        Chain {
            b: self,
            key,
            name: None,
            items: Vec::new(),
            indicator: Indicator::Hidden,
            open_delay: None,
            skip_delay: None,
            close_delay: None,
            classes: tw(),
        }
    }

    fn nav_link<K: Key>(&mut self, key: K, title: impl Into<String>) -> Link<'_, 'b, 'f, A, K> {
        Link {
            b: self,
            key,
            title: title.into(),
            description: None,
            icon: None,
            current: None,
            look: LinkLook::Panel,
        }
    }
}

/// Whether the menu draws the indicator under the open trigger.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Indicator {
    Shown,
    Hidden,
}

/// The parts of a menu the library names.
#[derive(Hash, Debug, Clone, Copy)]
enum Node {
    List,
    Trigger,
    Panel,
    Content,
}

/// What every item of a menu is built with.
struct Menu {
    group: TriggerGroup,
    open_delay: Duration,
    close_delay: Duration,
    indicator: Indicator,
    style: Style,
}

type Declare<'s, 'f, A> = Box<dyn for<'x> FnOnce(&mut Builder<'x, 'f, A>, &Menu) + 's>;

/// TODO(prose): a navigation menu being declared. Start one with
/// [`Ext::navigation_menu`] and finish it with [`add`](Self::add).
///
/// ```html
/// <nav data-slot="navigation-menu" data-viewport="false" class="relative flex max-w-max flex-1 items-center justify-center">
///   <ul data-slot="navigation-menu-list" class="flex flex-1 list-none items-center justify-center gap-0">…</ul>
/// </nav>
/// ```
#[must_use = "a navigation menu is not added until you call `.add()`"]
pub struct Chain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    name: Option<String>,
    items: Vec<Declare<'s, 'f, A>>,
    indicator: Indicator,
    open_delay: Option<Duration>,
    skip_delay: Option<Duration>,
    close_delay: Option<Duration>,
    classes: Tw,
}

impl<'s, 'f, A: 'static, K: Key> Chain<'s, '_, 'f, A, K> {
    /// TODO(prose): the menu's accessible name, as the landmark's: "Main".
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): a trigger reading `label` whose panel `body` fills: free
    /// content, built while the panel is shown, usually of
    /// [`nav_link`](Ext::nav_link)s.
    pub fn panel<I: Key>(
        mut self,
        key: I,
        label: impl Into<String>,
        body: impl for<'x> FnOnce(&mut Builder<'x, 'f, A>) + 's,
    ) -> Self {
        let label = label.into();
        self.items.push(Box::new(move |b, menu| {
            panel_item(b, key, label, Box::new(body), menu);
        }));
        self
    }

    /// TODO(prose): a top-level link reading `label`, drawn as a trigger
    /// without a panel. `link` gets the link and finishes it with its
    /// handler.
    pub fn link<I: Key>(
        mut self,
        key: I,
        label: impl Into<String>,
        link: impl for<'c, 'x> FnOnce(Link<'c, 'x, 'f, A, &'static str>) + 's,
    ) -> Self {
        let label = label.into();
        self.items.push(Box::new(move |b, _| {
            let item = b.part(Part::Item, &[], tw().sense_hover());
            b.div_id(key, item, |b| {
                link(Link {
                    b,
                    key: "Link",
                    title: label,
                    description: None,
                    icon: None,
                    current: None,
                    look: LinkLook::Bar,
                });
            })
            .role(Role::ListItem);
        }));
        self
    }

    /// TODO(prose): draws an arrow under the open trigger, pointing at it:
    /// shadcn's `NavigationMenuIndicator`. Off unless set.
    pub fn indicator(mut self) -> Self {
        self.indicator = Indicator::Shown;
        self
    }

    /// TODO(prose): the style's `open_delay` for this menu.
    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.open_delay = Some(delay);
        self
    }

    /// TODO(prose): the style's `skip_delay` for this menu.
    pub fn skip_delay(mut self, delay: Duration) -> Self {
        self.skip_delay = Some(delay);
        self
    }

    /// TODO(prose): the style's `close_delay` for this menu.
    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.close_delay = Some(delay);
        self
    }

    /// TODO(prose): classes merged over the root's styled ones; the later
    /// class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = std::mem::replace(&mut self.classes, tw()).merge(classes);
        self
    }

    /// TODO(prose): adds the menu.
    #[track_caller]
    pub fn add(self) {
        let Self {
            b,
            key,
            name,
            items,
            indicator,
            open_delay,
            skip_delay,
            close_delay,
            classes,
        } = self;
        let style = b.style().navigation_menu.clone();
        // A node shows its role to assistive technology only if it senses.
        let own = tw().sense_hover();
        let root = b.part(Part::Root, &[], own.clone().merge(classes));
        let node = b.div_id(key, root, |b| {
            let menu = Menu {
                group: TriggerGroup::new(
                    b.id_of(Node::List),
                    skip_delay.unwrap_or(style.skip_delay),
                ),
                open_delay: open_delay.unwrap_or(style.open_delay),
                close_delay: close_delay.unwrap_or(style.close_delay),
                indicator,
                style,
            };
            let list = b.part(Part::List, &[], own);
            b.div_id(Node::List, list, |b| {
                for item in items {
                    item(b, &menu);
                }
            })
            .role(Role::List)
            .nav(Nav::Row(Wrap::Stop))
            .tab_stops(TabStops::Every);
        });
        let node = node.role(Role::Navigation);
        if let Some(name) = name {
            node.named(name);
        }
    }
}

type Body<'s, 'f, A> = Box<dyn for<'x> FnOnce(&mut Builder<'x, 'f, A>) + 's>;

/// A trigger, its panel and, while it is open, the indicator under it.
fn panel_item<'f, A: 'static, K: Key>(
    b: &mut Builder<'_, 'f, A>,
    key: K,
    label: String,
    body: Body<'_, 'f, A>,
    menu: &Menu,
) {
    let item = b.part(Part::Item, &[], tw().sense_hover());
    b.div_id(key, item, |b| {
        let trigger = b.id_of(Node::Trigger);
        let content = crate::child_id(b.id_of(Node::Panel), &Node::Content);
        let open = crate::hover::is_open(b, menu.group.id, trigger);
        let keys: Vec<SheetKey> = crate::owned::expanded_key(open).into_iter().collect();
        let classes = b.part(Part::Trigger, &keys, tw().sense_click());
        let chevron = match (open, menu.style.chevron) {
            (true, Chevron::Turns) => icons::CHEVRON_UP,
            _ => icons::CHEVRON_DOWN,
        };
        b.div_id(Node::Trigger, classes, |b| {
            b.text(tw(), label.clone());
            let icon = b.part(Part::TriggerIcon, &[], tw());
            b.glyph(icon, Icon::new(chevron));
        })
        .role(Role::Button)
        .named(label)
        .expanded(open)
        .controls(content)
        .nav_item();
        if open && menu.indicator == Indicator::Shown {
            let fill = b
                .resolved_part(Part::IndicatorArrow, &[])
                .paint
                .background
                .unwrap_or(egui::Color32::TRANSPARENT);
            let place = b.part(Part::Indicator, &[], tw().absolute().left(0.0).w_full());
            b.paint(place, move |painter, rect, _| {
                paint_indicator(painter, rect, fill)
            });
        }
        let intent = HoverIntent::new(menu.open_delay, menu.close_delay).group(menu.group);
        let placement = Placement::below()
            .align(Align::Start)
            .gap(menu.style.offset);
        b.overlay(Node::Panel)
            .nav(Nav::Grid)
            .hover_anchored(trigger, intent)
            .transition(menu.style.transition)
            .exit_transition(menu.style.transition)
            .placement(placement)
            .children(|c| {
                let classes = c.part(Part::Content, &[], tw().sense_hover());
                // No role, as Radix's; a container still, so that the
                // trigger's `controls` can name it.
                c.div_id(Node::Content, classes, |c| body(c))
                    .role(Role::GenericContainer);
            });
    })
    .role(Role::ListItem);
}

/// The side of the indicator's square: shadcn's `h-2 w-2`.
const INDICATOR_SIDE: f32 = 8.0;

/// Paints the indicator in its box `rect`: shadcn's square turned 45°, its
/// top `60%` down the box, which shows only the corner pointing up.
fn paint_indicator(painter: &Painter, rect: Rect, fill: egui::Color32) {
    let reach = INDICATOR_SIDE * std::f32::consts::FRAC_1_SQRT_2;
    let centre: Pos2 = pos2(
        rect.center().x,
        rect.top() + rect.height() * 0.6 + INDICATOR_SIDE / 2.0,
    );
    let points = vec![
        pos2(centre.x - reach, centre.y),
        pos2(centre.x, centre.y - reach),
        pos2(centre.x + reach, centre.y),
        pos2(centre.x, centre.y + reach),
    ];
    painter
        .with_clip_rect(rect)
        .add(Shape::convex_polygon(points, fill, egui::Stroke::NONE));
}

/// Where a link is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LinkLook {
    /// In a panel: shadcn's `NavigationMenuLink`.
    Panel,
    /// In the row: the link with the trigger's look,
    /// `navigationMenuTriggerStyle()`.
    Bar,
}

/// TODO(prose): a link being declared, in a panel or in the row. Finish it
/// with [`on_click`](Self::on_click).
#[must_use = "a link is not added until you call `.on_click(..)`"]
pub struct Link<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    title: String,
    description: Option<String>,
    icon: Option<Icon>,
    current: Option<Current>,
    look: LinkLook,
}

impl<'f, A: 'static, K: Key> Link<'_, '_, 'f, A, K> {
    /// TODO(prose): a muted line under the title, cut to two lines; also the
    /// link's accessible description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// TODO(prose): draws `icon` before the title.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// TODO(prose): whether the link is to the page on screen: drawn as the
    /// active one, and announced as the current page.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current.then_some(Current::Page);
        self
    }

    /// TODO(prose): adds the link, which runs `act` when pressed. A link
    /// pressed in a panel closes the panel.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) {
        let Self {
            b,
            key,
            title,
            description,
            icon,
            current,
            look,
        } = self;
        let keys: Vec<SheetKey> = current.map(|_| ACTIVE).into_iter().collect();
        let link = b.part(Part::Link, &keys, tw().sense_click());
        let classes = match look {
            LinkLook::Panel => link,
            LinkLook::Bar => link.merge(b.part(Part::Trigger, &[], tw())),
        };
        let muted = b.style().tokens.muted_foreground;
        let node = b.div_id(key, classes, |b| {
            if let Some(icon) = icon {
                b.glyph(tw(), icon);
            }
            match (&description, look) {
                (Some(description), LinkLook::Panel) => {
                    b.div(tw().flex().flex_col().gap(1.0), |b| {
                        b.text(tw().leading_none().font_medium(), title.clone());
                        b.text(tw().line_clamp(2).text(muted), description.clone());
                    });
                }
                _ => b.text(tw(), title.clone()),
            }
        });
        let node = node.role(Role::Link).named(title).nav_item();
        let node = match current {
            Some(current) => node.current(current),
            None => node,
        };
        let node = match description {
            Some(description) => node.description(description),
            None => node,
        };
        node.draw(move |app, ui| {
            let id = ui.node_response().id;
            if ui.node_response().clicked() || crate::nav::take_activation(ui, id) {
                act(app);
                if let Some(shell) = crate::shell::shell_or_report(ui, "a navigation link") {
                    crate::actions::note_closer(ui.ctx(), &shell.kept, id);
                }
            }
        });
    }
}
