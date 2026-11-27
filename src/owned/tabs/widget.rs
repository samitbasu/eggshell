//! TODO(prose): tabs after shadcn/ui's Tabs: a tab list that the arrow keys
//! move through, and the panel of the selected tab.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::prelude::*;
//!
//! #[derive(Clone, Copy, PartialEq)]
//! enum Lang { Rust, Html }
//!
//! struct App { lang: Lang }
//!
//! fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
//!     b.tabs("lang", app.lang, [(Lang::Rust, "Rust"), (Lang::Html, "HTML")])
//!         .named("Language")
//!         .on_change(|app: &mut App, lang| app.lang = lang)
//!         .panel(|b, lang| match lang {
//!             Lang::Rust => b.text(eggshell_ui::tw(), "fn main() {}"),
//!             Lang::Html => b.text(eggshell_ui::tw(), "<main></main>"),
//!         });
//! }
//! ```

use std::rc::Rc;

use crate::contract::Size;
use crate::overlay::{Nav, Wrap};
use crate::owned::options::{Configure, Interactive, Options, Resolved, Respond};
use crate::owned::{
    Enablement, Mark, OnResponse,
    choice::{self, Choice, IntoChoice},
    segmented::{self, Cell},
    tabs::Part,
    tip_describes, tipped, tipped_whole,
};
use crate::{Builder, Key, NodeUi as _, Tw, child_id, tw};
use egui::{Context, Role};

/// TODO(prose): adds [`tabs`](Self::tabs) to the builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): tabs over `options`, of which `current` is selected. Give
    /// it an [`on_change`](Chain::on_change), then add it with
    /// [`panel`](Chain::panel).
    fn tabs<'s, K: Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &'s mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Chain<'s, 'b, 'f, A, K, T, Unset>;
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn tabs<'s, K: Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &'s mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Chain<'s, 'b, 'f, A, K, T, Unset> {
        Chain {
            b: self,
            key,
            current,
            options: choice::collect(options),
            opts: Options::new(),
            size: None,
            activation: Activation::OnFocus,
            list: tw(),
            panel: tw(),
            handler: Unset,
            on_response: OnResponse::default(),
        }
    }
}

/// TODO(prose): tabs that have no [`on_change`](Chain::on_change) yet.
pub struct Unset;

/// TODO(prose): what [`on_change`](Chain::on_change) gave the tabs.
pub struct Changes<A, T>(OnChange<A, T>);

type OnChange<A, T> = Rc<dyn Fn(&mut A, &Context, T)>;

/// When the keyboard selects the tab it reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Activation {
    /// As an arrow key focuses it.
    OnFocus,
    /// When Enter or Space is pressed on it.
    OnEnter,
}

/// The nodes of a tabs control, under its key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Node {
    List,
    Tab(usize),
    Panel(usize),
}

/// TODO(prose): tabs being declared; [`panel`](Self::panel) adds them.
///
/// Its shared options come from [`Configure`], [`Interactive`] and
/// [`Respond`], in scope through the [`prelude`](crate::prelude). Its name
/// is the tab list's, its classes the whole control's, holding the list and
/// the panel ([`list_classes`](Self::list_classes) and
/// [`panel_classes`](Self::panel_classes) style those), and its tooltip
/// and refusal the list's; a tab's own tooltip shows instead. Focus goes to
/// the selected tab, and the response is each tab's.
#[must_use = "tabs are not added until you call `.on_change(..)` and `.panel(..)`"]
pub struct Chain<'s, 'b, 'f, A, K, T, H> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    current: T,
    options: Vec<Choice<T>>,
    opts: Options,
    size: Option<Size>,
    activation: Activation,
    list: Tw,
    panel: Tw,
    handler: H,
    on_response: OnResponse<A>,
}

impl<A, K, T, H> Configure for Chain<'_, '_, '_, A, K, T, H> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, K, T, H> Interactive for Chain<'_, '_, '_, A, K, T, H> {}

impl<A, K, T, H> Respond for Chain<'_, '_, '_, A, K, T, H> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'s, 'b, 'f, A: 'static, K: Key, T: Copy + PartialEq + 'static, H>
    Chain<'s, 'b, 'f, A, K, T, H>
{
    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): selects a tab the arrow keys reach only when Enter or
    /// Space is pressed on it, for panels that are slow to build. By default
    /// the arrows select as they move.
    pub fn activate_on_enter(mut self) -> Self {
        self.activation = Activation::OnEnter;
        self
    }

    /// TODO(prose): classes merged over the styled tab list's.
    pub fn list_classes(mut self, classes: Tw) -> Self {
        self.list = self.list.merge(classes);
        self
    }

    /// TODO(prose): classes merged over the panel's.
    pub fn panel_classes(mut self, classes: Tw) -> Self {
        self.panel = self.panel.merge(classes);
        self
    }
}

impl<'s, 'b, 'f, A: 'static, K: Key, T: Copy + PartialEq + 'static>
    Chain<'s, 'b, 'f, A, K, T, Unset>
{
    /// TODO(prose): runs `on_change` with the value of the tab that was
    /// selected, by a click or the keyboard.
    pub fn on_change(
        self,
        on_change: impl Fn(&mut A, T) + 'static,
    ) -> Chain<'s, 'b, 'f, A, K, T, Changes<A, T>> {
        self.on_change_with(move |app, _, value| on_change(app, value))
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    pub fn on_change_with(
        self,
        on_change: impl Fn(&mut A, &Context, T) + 'static,
    ) -> Chain<'s, 'b, 'f, A, K, T, Changes<A, T>> {
        Chain {
            b: self.b,
            key: self.key,
            current: self.current,
            options: self.options,
            opts: self.opts,
            size: self.size,
            activation: self.activation,
            list: self.list,
            panel: self.panel,
            handler: Changes(Rc::new(on_change)),
            on_response: self.on_response,
        }
    }
}

impl<'s, 'b, 'f, A: 'static, K: Key, T: Copy + PartialEq + 'static>
    Chain<'s, 'b, 'f, A, K, T, Changes<A, T>>
{
    /// TODO(prose): adds the tab list, and below it the panel of the selected
    /// tab, which `build` builds; the panels of the other tabs are not built.
    #[track_caller]
    pub fn panel(self, build: impl FnOnce(&mut Builder<'_, 'f, A>, T)) {
        let Self {
            b,
            key,
            current,
            options,
            opts,
            size,
            activation,
            list,
            panel,
            handler: Changes(on_change),
            on_response,
        } = self;
        let size = Part::size_key(b, size.unwrap_or(b.control_size()));
        let slot = b.slot(&key);
        let Resolved {
            name,
            classes,
            enablement,
            tooltip,
            refused,
            autofocus,
        } = opts.resolve(b, child_id(slot.id, &Node::List));
        let selected = options.iter().position(|option| option.value == current);
        let target = choice::focus_target(&options, &current, enablement);
        let rows = segmented::Rows::One;
        let root = b.part(Part::Root, &[], classes);
        b.div_id_with(&slot, root, |b| {
            let panel_id = selected.map(|index| b.id_of(Node::Panel(index)));
            let track = b.part(
                Part::List,
                &Part::list_keys(size, rows, enablement),
                tw().sense_hover().merge(list),
            );
            let control_tip = tooltip.clone();
            let tabs = b
                .div_id(Node::List, track, |b| {
                    for (index, option) in options.iter().enumerate() {
                        let (own, tip) = option.gated(enablement, refused);
                        let cell = if Some(index) == selected {
                            Cell::Pressed
                        } else {
                            Cell::Released
                        };
                        let classes = b.part(
                            Part::Trigger,
                            &Part::trigger_keys(size, rows, cell, own),
                            match own {
                                Enablement::Enabled => tw().sense_click().text_center(),
                                Enablement::Disabled => tw().sense_click().text_center().inert(),
                            },
                        );
                        let tab = match &option.icon {
                            None => b.span(Node::Tab(index), classes, option.label.clone()),
                            Some(glyph) => b.div_id(Node::Tab(index), classes, |b| {
                                b.glyph(tw(), glyph.clone());
                                b.text(tw(), option.label.clone());
                            }),
                        };
                        let autofocus = match Some(index) == target {
                            true => autofocus,
                            false => crate::focus::Autofocus::Never,
                        };
                        let tab = tab
                            .role(Role::Tab)
                            .named(option.label.clone())
                            .selected(Mark::from(cell == Cell::Pressed))
                            .open_focus(autofocus.open_focus());
                        let tab = match (Some(index) == selected, panel_id) {
                            (true, Some(panel)) => tab.controls(panel),
                            _ => tab,
                        };
                        let (control_tip, on_response) = (control_tip.clone(), on_response.clone());
                        let tab = tab.draw(move |app, ui| {
                            let response = tipped(ui, ui.node_response(), &tip);
                            autofocus.apply(ui.ctx(), &response);
                            if tip.is_none() {
                                tip_describes(&response, &control_tip);
                            }
                            on_response.run(app, &response);
                        });
                        let value = option.value;
                        let on_change = Rc::clone(&on_change);
                        tab.on_click_with(move |app: &mut A, ctx| on_change(app, ctx, value));
                    }
                })
                .role(Role::TabList)
                .nav(Nav::Row(Wrap::Around))
                .draw(move |_, ui| tipped_whole(ui, &tooltip));
            let tabs = match &name {
                Some(name) => tabs.named(name.clone()),
                None => tabs,
            };
            if activation == Activation::OnFocus {
                tabs.selection_follows_focus();
            }
            if let Some(index) = selected {
                let label = options
                    .get(index)
                    .map(|option| option.label.clone())
                    .unwrap_or_default();
                let content = b.part(Part::Content, &[], tw().sense_hover().merge(panel));
                b.div_id(Node::Panel(index), content, |b| build(b, current))
                    .role(Role::TabPanel)
                    .named(label);
            }
        });
    }
}
