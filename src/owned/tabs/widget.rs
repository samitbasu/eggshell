//! TODO(prose): tabs after shadcn/ui's Tabs: a tab list that the arrow keys
//! move through, and the panel of the selected tab.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::owned::tabs::TabsExt;
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
use crate::owned::{Cell, Choice, Enablement, IntoChoice, Mark, SegmentedRows, TabsPart};
use crate::{Builder, Key, Tw, tw};
use egui::Role;

/// TODO(prose): adds [`tabs`](Self::tabs) to the builder.
pub trait TabsExt<'b, 'f, A> {
    /// TODO(prose): tabs over `options`, of which `current` is selected. Give
    /// it an [`on_change`](TabsChain::on_change), then add it with
    /// [`panel`](TabsChain::panel).
    fn tabs<'s, K: Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &'s mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> TabsChain<'s, 'b, 'f, A, K, T, Unset>;
}

impl<'b, 'f, A: 'static> TabsExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn tabs<'s, K: Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &'s mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> TabsChain<'s, 'b, 'f, A, K, T, Unset> {
        TabsChain {
            b: self,
            key,
            current,
            options: options.into_iter().map(IntoChoice::into_choice).collect(),
            name: None,
            size: None,
            enablement: Enablement::Enabled,
            activation: Activation::OnFocus,
            list: tw(),
            panel: tw(),
            handler: Unset,
        }
    }
}

/// TODO(prose): tabs that have no [`on_change`](TabsChain::on_change) yet.
pub struct Unset;

/// TODO(prose): what [`on_change`](TabsChain::on_change) gave the tabs.
pub struct Changes<A, T>(OnChange<A, T>);

type OnChange<A, T> = Rc<dyn Fn(&mut A, T)>;

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
enum Part {
    List,
    Tab(usize),
    Panel(usize),
}

/// TODO(prose): tabs being declared; [`panel`](Self::panel) adds them.
#[must_use = "tabs are not added until you call `.on_change(..)` and `.panel(..)`"]
pub struct TabsChain<'s, 'b, 'f, A, K, T, H> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    current: T,
    options: Vec<Choice<T>>,
    name: Option<String>,
    size: Option<Size>,
    enablement: Enablement,
    activation: Activation,
    list: Tw,
    panel: Tw,
    handler: H,
}

impl<'s, 'b, 'f, A: 'static, K: Key, T: Copy + PartialEq + 'static, H>
    TabsChain<'s, 'b, 'f, A, K, T, H>
{
    /// TODO(prose): the tab list's accessible name, as `aria-label` is.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// TODO(prose): whether the tabs take input, on top of the subtree's.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        self.enablement = enablement.into();
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
    TabsChain<'s, 'b, 'f, A, K, T, Unset>
{
    /// TODO(prose): runs `on_change` with the value of the tab that was
    /// selected, by a click or the keyboard.
    pub fn on_change(
        self,
        on_change: impl Fn(&mut A, T) + 'static,
    ) -> TabsChain<'s, 'b, 'f, A, K, T, Changes<A, T>> {
        TabsChain {
            b: self.b,
            key: self.key,
            current: self.current,
            options: self.options,
            name: self.name,
            size: self.size,
            enablement: self.enablement,
            activation: self.activation,
            list: self.list,
            panel: self.panel,
            handler: Changes(Rc::new(on_change)),
        }
    }
}

impl<'s, 'b, 'f, A: 'static, K: Key, T: Copy + PartialEq + 'static>
    TabsChain<'s, 'b, 'f, A, K, T, Changes<A, T>>
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
            name,
            size,
            enablement,
            activation,
            list,
            panel,
            handler: Changes(on_change),
        } = self;
        let size = TabsPart::size_key(b, size.unwrap_or(b.control_size()));
        let enablement = enablement.and(b.enablement());
        let selected = options.iter().position(|option| option.value == current);
        let rows = SegmentedRows::One;
        let root = b.part(TabsPart::Root, &[], tw());
        b.div_id(key, root, |b| {
            let panel_id = selected.map(|index| b.id_of(Part::Panel(index)));
            let track = b.part(
                TabsPart::List,
                &TabsPart::list_keys(size, rows, enablement),
                tw().sense_hover().merge(list),
            );
            let tabs = b
                .div_id(Part::List, track, |b| {
                    for (index, option) in options.iter().enumerate() {
                        let own = enablement.and(option.enablement).and(match option.refusal {
                            Some(_) => Enablement::Disabled,
                            None => Enablement::Enabled,
                        });
                        let cell = if Some(index) == selected {
                            Cell::Pressed
                        } else {
                            Cell::Released
                        };
                        let classes = b.part(
                            TabsPart::Trigger,
                            &TabsPart::trigger_keys(size, rows, cell, own),
                            match own {
                                Enablement::Enabled => tw().sense_click().text_center(),
                                Enablement::Disabled => tw().sense_click().text_center().inert(),
                            },
                        );
                        let tab = match &option.icon {
                            None => b.span(Part::Tab(index), classes, option.label.clone()),
                            Some(glyph) => b.div_id(Part::Tab(index), classes, |b| {
                                b.glyph(tw(), glyph.clone());
                                b.text(tw(), option.label.clone());
                            }),
                        };
                        let tab = tab
                            .role(Role::Tab)
                            .named(option.label.clone())
                            .selected(Mark::from(cell == Cell::Pressed));
                        let tab = match (Some(index) == selected, panel_id) {
                            (true, Some(panel)) => tab.controls(panel),
                            _ => tab,
                        };
                        let tab = match option.refusal.clone().or_else(|| option.tooltip.clone()) {
                            Some(tip) => tab.tooltip(tip),
                            None => tab,
                        };
                        let value = option.value;
                        let on_change = Rc::clone(&on_change);
                        tab.on_click(move |app: &mut A| on_change(app, value));
                    }
                })
                .role(Role::TabList)
                .nav(Nav::Row(Wrap::Around));
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
                let content = b.part(TabsPart::Content, &[], tw().sense_hover().merge(panel));
                b.div_id(Part::Panel(index), content, |b| build(b, current))
                    .role(Role::TabPanel)
                    .named(label);
            }
        });
    }
}
