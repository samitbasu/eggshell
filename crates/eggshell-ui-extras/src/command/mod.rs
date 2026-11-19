//! TODO(prose): searchable lists: a command list (a search field over a filtered, virtual list of results, inline or as a Cmd/Ctrl+K dialog), a combobox and an autosuggest, sharing one list and one filter.

mod filter;
mod list;

use std::rc::Rc;
use std::sync::Arc;

use eggshell_ui::overlay::{Anchor, Focus, MatchWidth, Modal, Placement};
use eggshell_ui::owned::Mark;
use eggshell_ui::{Builder, CommandState, Icon, Key, NodeUi as _, Scope, Tw, icons, tw};
use egui::{Id, Role};

pub use filter::{Candidate, Score, word_prefix};
pub use list::{ItemChain, Items};

use list::{First, ListOptions, Query};

/// TODO(prose): who filters the items: the list, by its filter, or the app, which hears the query and gives back the items to show as they are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filtering {
    /// TODO(prose): the list filters and ranks the items it is given.
    #[default]
    Ours,
    /// TODO(prose): the app does, as it is `pending`.
    App(Pending),
}

/// TODO(prose): whether the app's results for the query are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    /// TODO(prose): the items given are the results.
    Settled,
    /// TODO(prose): results are on their way: a loading row shows.
    Loading,
}

type Filter = Rc<dyn Fn(&str, Candidate<'_>) -> Option<Score>>;
type Heard<A> = Rc<dyn Fn(&mut A, &str)>;
type Empty<'f, A> = Box<dyn FnOnce(&mut Scope<'_, 'f, A>) + 'f>;
type Typed<A> = Rc<dyn Fn(&mut A, String)>;

/// TODO(prose): the builder methods of this module.
pub trait CommandExt<'b, 'f, A> {
    /// TODO(prose): an inline command list: a search field over the results.
    fn command_list<K: Key>(&mut self, key: K) -> CommandList<'_, 'b, 'f, A>;

    /// TODO(prose): a command list in a centred modal dialog, shown while `open`; a pick or a dismissal runs `on_close`.
    fn command_dialog<K: Key>(
        &mut self,
        key: K,
        open: Open,
        on_close: impl Fn(&mut A) + 'static,
    ) -> CommandList<'_, 'b, 'f, A>;

    /// TODO(prose): a combobox: a trigger showing `value`'s label that opens a searchable list of `options`; a pick runs `on_change`.
    fn combobox<K: Key, V: Key + Copy + PartialEq>(
        &mut self,
        key: K,
        value: Option<V>,
        options: impl IntoIterator<Item = (V, String)>,
    ) -> ComboboxChain<'_, 'b, 'f, A, V>;

    /// TODO(prose): a text field that suggests `suggestions` matching what is typed; the text is the value.
    fn autosuggest<K: Key>(
        &mut self,
        key: K,
        text: &str,
        suggestions: impl IntoIterator<Item = String>,
    ) -> AutosuggestChain<'_, 'b, 'f, A>;
}

/// TODO(prose): whether a dialog is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    /// TODO(prose): shown.
    Yes,
    /// TODO(prose): not shown.
    No,
}

impl From<bool> for Open {
    fn from(open: bool) -> Self {
        if open { Self::Yes } else { Self::No }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Dialog,
    Box,
    Trigger,
    Popover,
    Field,
    Suggestions,
}

/// How a command list sits, and what a pick does to it.
enum Shape<A> {
    Inline,
    Dialog {
        open: Open,
        close: Rc<dyn Fn(&mut A)>,
    },
}

/// TODO(prose): a command list, started with [`CommandExt::command_list`] or [`CommandExt::command_dialog`] and finished with [`items`](Self::items).
#[must_use = "a command list is not added until you call `.items(..)`"]
pub struct CommandList<'s, 'b, 'f, A> {
    b: &'s mut Builder<'b, 'f, A>,
    key: Id,
    shape: Shape<A>,
    options: Options<'f, A>,
}

/// What a command list, a combobox and an autosuggest all take.
struct Options<'f, A> {
    placeholder: Option<String>,
    name: Option<String>,
    filter: Option<Filter>,
    filtering: Filtering,
    on_query: Option<Heard<A>>,
    empty: Option<Empty<'f, A>>,
    classes: Tw,
}

impl<A> Default for Options<'_, A> {
    fn default() -> Self {
        Self {
            placeholder: None,
            name: None,
            filter: None,
            filtering: Filtering::Ours,
            on_query: None,
            empty: None,
            classes: tw(),
        }
    }
}

impl<'f, A: 'static> CommandList<'_, '_, 'f, A> {
    /// TODO(prose): greyed text in the empty search field.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.options.placeholder = Some(text.into());
        self
    }

    /// TODO(prose): the accessible name of the list, and of the dialog.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.options.name = Some(name.into());
        self
    }

    /// TODO(prose): ranks the items by `filter` instead of [`word_prefix`]; `None` leaves an item out.
    pub fn filter(
        mut self,
        filter: impl Fn(&str, Candidate<'_>) -> Option<Score> + 'static,
    ) -> Self {
        self.options.filter = Some(Rc::new(filter));
        self
    }

    /// TODO(prose): who filters: [`Filtering::App`] shows the items as given, for an app that fetches its own results; hear the query with [`on_query`](Self::on_query).
    pub fn filtering(mut self, filtering: Filtering) -> Self {
        self.options.filtering = filtering;
        self
    }

    /// TODO(prose): runs `then` with the query each time it is edited.
    pub fn on_query(mut self, then: impl Fn(&mut A, &str) + 'static) -> Self {
        self.options.on_query = Some(Rc::new(then));
        self
    }

    /// TODO(prose): what shows when nothing matches.
    pub fn empty(mut self, body: impl FnOnce(&mut Scope<'_, 'f, A>) + 'f) -> Self {
        self.options.empty = Some(Box::new(body));
        self
    }

    /// TODO(prose): classes of the list's box, such as its width.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.options.classes = classes;
        self
    }

    /// TODO(prose): adds the list, whose items `items` gives, in groups.
    pub fn items(self, items: impl FnOnce(&mut Items<A>)) {
        let Self {
            b,
            key,
            shape,
            options,
        } = self;
        let mut given = Items::new(b);
        items(&mut given);
        match shape {
            Shape::Inline => list::show(b, key, given, options, ListOptions::palette(None)),
            Shape::Dialog { open: Open::No, .. } => Query::clear(b.ctx(), key),
            Shape::Dialog {
                open: Open::Yes,
                close,
            } => {
                let name = options.name.clone().unwrap_or_default();
                let dismiss = Rc::clone(&close);
                b.overlay((key, Part::Dialog))
                    .modal(Modal::Center)
                    .role(Role::Dialog)
                    .named(name)
                    .on_dismiss(move |app| dismiss(app))
                    .children(|s| {
                        let classes = list::dialog_classes(s.style());
                        s.div_id(Part::Box, classes, |s| {
                            list::show(s, key, given, options, ListOptions::palette(Some(close)));
                        });
                    });
            }
        }
    }
}

impl<'b, 'f, A: 'static> CommandExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn command_list<K: Key>(&mut self, key: K) -> CommandList<'_, 'b, 'f, A> {
        CommandList {
            key: self.id_of(key),
            b: self,
            shape: Shape::Inline,
            options: Options::default(),
        }
    }

    fn command_dialog<K: Key>(
        &mut self,
        key: K,
        open: Open,
        on_close: impl Fn(&mut A) + 'static,
    ) -> CommandList<'_, 'b, 'f, A> {
        CommandList {
            key: self.id_of(key),
            b: self,
            shape: Shape::Dialog {
                open,
                close: Rc::new(on_close),
            },
            options: Options::default(),
        }
    }

    fn combobox<K: Key, V: Key + Copy + PartialEq>(
        &mut self,
        key: K,
        value: Option<V>,
        options: impl IntoIterator<Item = (V, String)>,
    ) -> ComboboxChain<'_, 'b, 'f, A, V> {
        ComboboxChain {
            key: self.id_of(key),
            b: self,
            value,
            choices: options.into_iter().collect(),
            options: Options::default(),
        }
    }

    fn autosuggest<K: Key>(
        &mut self,
        key: K,
        text: &str,
        suggestions: impl IntoIterator<Item = String>,
    ) -> AutosuggestChain<'_, 'b, 'f, A> {
        AutosuggestChain {
            key: self.id_of(key),
            b: self,
            text: text.to_owned(),
            suggestions: suggestions.into_iter().collect(),
            options: Options::default(),
        }
    }
}

/// Whether a combobox's popover is open: the widget's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Popover {
    #[default]
    Closed,
    Open,
}

impl Popover {
    fn id(combobox: Id) -> Id {
        combobox.with("eggshell-ui-extras-popover")
    }

    fn load(ctx: &egui::Context, combobox: Id) -> Self {
        ctx.data(|data| data.get_temp::<Self>(Self::id(combobox)))
            .unwrap_or_default()
    }

    fn store(self, ctx: &egui::Context, combobox: Id) {
        ctx.data_mut(|data| data.insert_temp(Self::id(combobox), self));
    }
}

/// TODO(prose): a combobox, started with [`CommandExt::combobox`] and finished with [`on_change`](Self::on_change).
#[must_use = "a combobox is not added until you call `.on_change(..)`"]
pub struct ComboboxChain<'s, 'b, 'f, A, V> {
    b: &'s mut Builder<'b, 'f, A>,
    key: Id,
    value: Option<V>,
    choices: Vec<(V, String)>,
    options: Options<'f, A>,
}

impl<'f, A: 'static, V: Key + Copy + PartialEq> ComboboxChain<'_, '_, 'f, A, V> {
    /// TODO(prose): what the trigger says with no value, and the search field's hint.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.options.placeholder = Some(text.into());
        self
    }

    /// TODO(prose): the accessible name of the trigger and of the list.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.options.name = Some(name.into());
        self
    }

    /// TODO(prose): ranks the options by `filter` instead of [`word_prefix`].
    pub fn filter(
        mut self,
        filter: impl Fn(&str, Candidate<'_>) -> Option<Score> + 'static,
    ) -> Self {
        self.options.filter = Some(Rc::new(filter));
        self
    }

    /// TODO(prose): what shows when nothing matches.
    pub fn empty(mut self, body: impl FnOnce(&mut Scope<'_, 'f, A>) + 'f) -> Self {
        self.options.empty = Some(Box::new(body));
        self
    }

    /// TODO(prose): classes of the trigger, such as its width.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.options.classes = classes;
        self
    }

    /// TODO(prose): adds the combobox; `on_change` runs with the option picked.
    pub fn on_change(self, on_change: impl Fn(&mut A, V) + 'static) {
        let Self {
            b,
            key,
            value,
            choices,
            mut options,
        } = self;
        let ctx = b.ctx().clone();
        let popover = Popover::load(&ctx, key);
        let shown = value
            .and_then(|value| choices.iter().find(|(v, _)| *v == value))
            .map(|(_, label)| label.clone());
        let placeholder = options.placeholder.clone().unwrap_or_default();
        let name = options.name.clone().unwrap_or_default();
        let trigger_classes = std::mem::replace(&mut options.classes, tw());
        let (open, toggle) = (ctx.clone(), popover);
        let label = shown.clone().unwrap_or(placeholder);
        let style = Arc::clone(b.style());
        let trigger_id = b.id_of((key, Part::Trigger));
        let trigger = b
            .div_id(
                (key, Part::Trigger),
                list::trigger_classes(&style).merge(trigger_classes),
                |s| {
                    s.div(tw().grow().min_w_0().truncate(), |s| s.text(tw(), label));
                    s.glyph(
                        list::chevron_classes(&style),
                        Icon::new(icons::CHEVRON_DOWN),
                    );
                },
            )
            .role(Role::ComboBox)
            .named(name)
            .on_click(move |_| {
                let next = match toggle {
                    Popover::Open => Popover::Closed,
                    Popover::Closed => Popover::Open,
                };
                next.store(&open, key);
            });
        let value_text = shown.unwrap_or_default();
        trigger.draw(move |_, ui| {
            let id = ui.node_response().id;
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_value(value_text.as_str());
                node.set_has_popup(egui::accesskit::HasPopup::Dialog);
                if popover == Popover::Closed {
                    node.set_expanded(false);
                }
            });
        });
        if popover == Popover::Closed {
            return;
        }
        let on_change = Rc::new(on_change);
        let mut items = Items::new(b);
        for (choice, label) in choices {
            let on_change = Rc::clone(&on_change);
            let mark = Mark::from(Some(choice) == value);
            items
                .item(choice, label)
                .state(CommandState::Checked(mark))
                .on_pick(move |app| on_change(app, choice));
        }
        let close = ctx.clone();
        let after: Rc<dyn Fn(&mut A)> = Rc::new(move |_| Popover::Closed.store(&close, key));
        let dismiss = ctx.clone();
        let style = Arc::clone(b.style());
        b.overlay((key, Part::Popover))
            .anchor(Anchor::Node(trigger_id))
            .placement(Placement::below().gap(style.menu.offset))
            .match_width(MatchWidth::Anchor)
            .focus(Focus::Enter)
            .role(Role::Dialog)
            .on_dismiss(move |_| Popover::Closed.store(&dismiss, key))
            .children(|s| {
                s.div_id(Part::Box, list::popover_classes(&style), |s| {
                    list::show(
                        s,
                        key,
                        items,
                        options,
                        list::ListOptions::palette(Some(after)),
                    );
                });
            });
    }
}

/// TODO(prose): an autosuggest, started with [`CommandExt::autosuggest`] and finished with [`on_change`](Self::on_change).
#[must_use = "an autosuggest is not added until you call `.on_change(..)`"]
pub struct AutosuggestChain<'s, 'b, 'f, A> {
    b: &'s mut Builder<'b, 'f, A>,
    key: Id,
    text: String,
    suggestions: Vec<String>,
    options: Options<'f, A>,
}

/// Whether Escape or a pick put an autosuggest's suggestions away until the
/// next edit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Suggesting {
    #[default]
    Yes,
    Dismissed,
}

impl Suggesting {
    fn id(field: Id) -> Id {
        field.with("eggshell-ui-extras-suggesting")
    }

    fn load(ctx: &egui::Context, field: Id) -> Self {
        ctx.data(|data| data.get_temp::<Self>(Self::id(field)))
            .unwrap_or_default()
    }

    fn store(self, ctx: &egui::Context, field: Id) {
        ctx.data_mut(|data| data.insert_temp(Self::id(field), self));
    }
}

impl<'f, A: 'static> AutosuggestChain<'_, '_, 'f, A> {
    /// TODO(prose): greyed text in the empty field.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.options.placeholder = Some(text.into());
        self
    }

    /// TODO(prose): the accessible name of the field and of the suggestions.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.options.name = Some(name.into());
        self
    }

    /// TODO(prose): ranks the suggestions by `filter` instead of [`word_prefix`].
    pub fn filter(
        mut self,
        filter: impl Fn(&str, Candidate<'_>) -> Option<Score> + 'static,
    ) -> Self {
        self.options.filter = Some(Rc::new(filter));
        self
    }

    /// TODO(prose): who filters; see [`CommandList::filtering`].
    pub fn filtering(mut self, filtering: Filtering) -> Self {
        self.options.filtering = filtering;
        self
    }

    /// TODO(prose): classes of the field, such as its width.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.options.classes = classes;
        self
    }

    /// TODO(prose): adds the field; `on_change` runs with the text as typed, or with a suggestion picked.
    pub fn on_change(self, on_change: impl Fn(&mut A, String) + 'static) {
        let Self {
            b,
            key,
            text,
            suggestions,
            mut options,
        } = self;
        let ctx = b.ctx().clone();
        let on_change: Typed<A> = Rc::new(on_change);
        let classes = std::mem::replace(&mut options.classes, tw());
        let field_id = b.id_of((key, Part::Field));
        let mut field = b
            .text_field((key, Part::Field), text.clone())
            .role(Role::ComboBox)
            .classes(classes);
        if let Some(placeholder) = options.placeholder.take() {
            field = field.hint(placeholder);
        }
        if let Some(name) = options.name.clone() {
            field = field.named(name);
        }
        let escaped = ctx.clone();
        field = field.on_response(move |_, response| {
            let escape = response
                .ctx
                .input(|input| input.key_pressed(egui::Key::Escape));
            if escape && (response.has_focus() || response.lost_focus()) {
                Suggesting::Dismissed.store(&escaped, field_id);
            }
        });
        let (typed, reset) = (Rc::clone(&on_change), ctx.clone());
        field.on_change(move |app, text| {
            Suggesting::Yes.store(&reset, field_id);
            typed(app, text);
        });
        let suggesting = Suggesting::load(&ctx, field_id);
        let focused = b.has_focus_within((key, Part::Field));
        if !focused || suggesting == Suggesting::Dismissed || text.trim().is_empty() {
            return;
        }
        let mut items = Items::new(b);
        for suggestion in suggestions {
            let picked = Rc::clone(&on_change);
            let label = suggestion.clone();
            items
                .item(suggestion.clone(), suggestion)
                .on_pick(move |app| picked(app, label.clone()));
        }
        if !items.any_match(&text, options.filter.as_ref(), options.filtering)
            && options.filtering == Filtering::Ours
        {
            return;
        }
        let (close, dismiss) = (ctx.clone(), ctx.clone());
        let after: Rc<dyn Fn(&mut A)> =
            Rc::new(move |_| Suggesting::Dismissed.store(&close, field_id));
        let style = Arc::clone(b.style());
        options.empty = Some(Box::new(|_| {}));
        b.overlay((key, Part::Suggestions))
            .anchor(Anchor::Node(field_id))
            .placement(Placement::below().gap(style.menu.offset))
            .match_width(MatchWidth::Anchor)
            .focus(Focus::Stay)
            .on_dismiss(move |_| Suggesting::Dismissed.store(&dismiss, field_id))
            .children(|s| {
                s.div_id(Part::Box, list::popover_classes(&style), |s| {
                    list::show(
                        s,
                        key,
                        items,
                        options,
                        list::ListOptions {
                            first: First::Left,
                            after: Some(after),
                            field: list::Field::Theirs { id: field_id, text },
                        },
                    );
                });
            });
    }
}
