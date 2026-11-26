//! TODO(prose): searchable lists: a command list (a search field over a filtered, virtual list of results, inline or as a Cmd/Ctrl+K dialog), a combobox and an autosuggest, sharing one list and one filter.

mod filter;
mod list;
pub(crate) use list::write_exceptions;
mod parts;

use std::rc::Rc;
use std::sync::Arc;

use crate::overlay::{Anchor, DismissOn, Dismissed, Focus, MatchWidth, Modal, Placement};
use crate::owned::Mark;
use crate::owned::options::{Configure as _, Interactive as _, Respond as _};

use crate::pipeline::DismissFn;
use crate::sheet::Key as SheetKey;
use crate::{Aria, Builder, CommandState, Icon, Key, Scope, Tw, icons, tw};
use egui::{Id, Role};

pub use filter::{Candidate, Score, word_prefix};
pub use list::{ItemChain, Items};
pub use parts::{ComboboxPart, Part};

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
type Cleared<A> = Rc<dyn Fn(&mut A)>;

/// TODO(prose): the builder methods of this module.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): an inline command list: a search field over the results.
    fn command_list<K: Key>(&mut self, key: K) -> List<'_, 'b, 'f, A>;

    /// TODO(prose): a command list in a centred modal dialog, shown while `open`; a pick or a dismissal runs `on_close`.
    fn command_dialog<K: Key>(
        &mut self,
        key: K,
        open: Open,
        on_close: impl Fn(&mut A) + 'static,
    ) -> List<'_, 'b, 'f, A>;

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
enum Node {
    Dialog,
    Box,
    Group,
    Chevron,
    Clear,
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
        dismiss_on: DismissOn,
        /// Runs instead of `close` when the dialog is dismissed.
        on_dismiss: Option<DismissFn<A>>,
    },
}

/// TODO(prose): a command list, started with [`Ext::command_list`] or [`Ext::command_dialog`] and finished with [`items`](Self::items).
#[must_use = "a command list is not added until you call `.items(..)`"]
pub struct List<'s, 'b, 'f, A> {
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
    error: Option<String>,
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
            error: None,
        }
    }
}

impl<'f, A: 'static> List<'_, '_, 'f, A> {
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

    /// TODO(prose): what dismisses a command dialog, as
    /// [`OverlayChain::dismiss_on`](crate::overlay::OverlayChain::dismiss_on)
    /// says. An inline list has no overlay, and ignores it.
    pub fn dismiss_on(mut self, dismiss_on: DismissOn) -> Self {
        if let Shape::Dialog { dismiss_on: on, .. } = &mut self.shape {
            *on = dismiss_on;
        }
        self
    }

    /// TODO(prose): runs `then` instead of the dialog's `on_close` when the
    /// library dismisses a command dialog, told why, as
    /// [`OverlayChain::on_dismiss_with`](crate::overlay::OverlayChain::on_dismiss_with)
    /// is. A pick still runs `on_close`. An inline list has no overlay, and
    /// ignores it.
    pub fn on_dismiss_with(mut self, then: impl FnOnce(&mut A, Dismissed) + 'static) -> Self {
        if let Shape::Dialog { on_dismiss, .. } = &mut self.shape {
            *on_dismiss = Some(Box::new(then));
        }
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
                dismiss_on,
                on_dismiss,
            } => {
                let name = options.name.clone().unwrap_or_default();
                let on_dismiss = on_dismiss.unwrap_or_else(|| {
                    let close = Rc::clone(&close);
                    Box::new(move |app, _| close(app))
                });
                b.overlay((key, Node::Dialog))
                    .modal(Modal::Center)
                    .role(Role::Dialog)
                    .named(name)
                    .dismiss_on(dismiss_on)
                    .on_dismiss_with(on_dismiss)
                    .children(|s| {
                        let classes = s.part(Part::Dialog, &[], tw());
                        s.div_id(Node::Box, classes, |s| {
                            list::show(s, key, given, options, ListOptions::palette(Some(close)));
                        });
                    });
            }
        }
    }
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn command_list<K: Key>(&mut self, key: K) -> List<'_, 'b, 'f, A> {
        List {
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
    ) -> List<'_, 'b, 'f, A> {
        List {
            key: self.id_of(key),
            b: self,
            shape: Shape::Dialog {
                open,
                close: Rc::new(on_close),
                dismiss_on: DismissOn::default(),
                on_dismiss: None,
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
            clear: None,
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

/// A combobox's own state: whether its list is open, and what was typed since
/// it opened. Until something is typed the field shows the chosen label and
/// the list shows every option.
#[derive(Clone, Debug, Default, PartialEq)]
struct Typing {
    open: bool,
    query: Option<String>,
    /// The field takes the focus on the next frame: a press on a button beside
    /// it asked for it.
    focus: bool,
}

impl Typing {
    const CLOSED: Self = Self {
        open: false,
        query: None,
        focus: false,
    };

    fn id(combobox: Id) -> Id {
        combobox.with("eggshell-ui-typing")
    }

    fn load(ctx: &egui::Context, combobox: Id) -> Self {
        ctx.data(|data| data.get_temp::<Self>(Self::id(combobox)))
            .unwrap_or_default()
    }

    fn store(self, ctx: &egui::Context, combobox: Id) {
        ctx.data_mut(|data| data.insert_temp(Self::id(combobox), self));
    }

    fn opened() -> Self {
        Self {
            open: true,
            query: None,
            focus: false,
        }
    }

    fn focusing(self) -> Self {
        Self {
            focus: true,
            ..self
        }
    }
}

/// TODO(prose): a combobox, started with [`Ext::combobox`] and finished with [`on_change`](Self::on_change).
#[must_use = "a combobox is not added until you call `.on_change(..)`"]
pub struct ComboboxChain<'s, 'b, 'f, A, V> {
    b: &'s mut Builder<'b, 'f, A>,
    key: Id,
    value: Option<V>,
    choices: Vec<(V, String)>,
    options: Options<'f, A>,
    clear: Option<Cleared<A>>,
}

impl<'f, A: 'static, V: Key + Copy + PartialEq> ComboboxChain<'_, '_, 'f, A, V> {
    /// TODO(prose): greyed text in the empty field.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.options.placeholder = Some(text.into());
        self
    }

    /// TODO(prose): the accessible name of the field and of the list.
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

    /// TODO(prose): classes of the field's box, such as its width.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.options.classes = classes;
        self
    }

    /// TODO(prose): marks the combobox invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::owned::text_field::TextFieldChain::invalid)
    /// does: its box's border and ring turn destructive, because its field
    /// is invalid.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.options.error = error.map(Into::into);
        self
    }

    /// TODO(prose): a clear button in place of the chevron while there is a
    /// value; it runs `on_clear`.
    pub fn on_clear(mut self, on_clear: impl Fn(&mut A) + 'static) -> Self {
        self.clear = Some(Rc::new(on_clear));
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
            clear,
        } = self;
        let ctx = b.ctx().clone();
        let state = Typing::load(&ctx, key);
        let chosen = value
            .and_then(|value| choices.iter().find(|(v, _)| *v == value))
            .map(|(_, label)| label.clone());
        let text = state
            .query
            .clone()
            .unwrap_or_else(|| chosen.clone().unwrap_or_default());
        let _style = Arc::clone(b.style());
        let group_id = b.id_of((key, Node::Group));
        let mut field_id = Id::NULL;
        let caller = std::mem::replace(&mut options.classes, tw());
        let group_classes = b.part(ComboboxPart::Input, &[], caller);
        let (placeholder, name) = (options.placeholder.clone(), options.name.clone());
        let error = options.error.take();
        let open = state.open;
        let clearing = clear.filter(|_| value.is_some());
        b.div_id((key, Node::Group), group_classes, |s| {
            field_id = s.id_of((key, Node::Field));
            let mut field = s
                .text_field((key, Node::Field), text.clone())
                .bare()
                .role(Role::ComboBox)
                .invalid(error)
                .classes(match open {
                    true => tw().grow().min_w_0().aria(Aria::Expanded),
                    false => tw().grow().min_w_0(),
                });
            if let Some(placeholder) = placeholder {
                field = field.hint(placeholder);
            }
            if let Some(name) = name {
                field = field.named(name);
            }
            if state.focus {
                Typing {
                    focus: false,
                    ..state.clone()
                }
                .store(&ctx, key);
                field = field.focus_now();
            }
            let (handled, shown) = (ctx.clone(), text.clone());
            field = field.on_response(move |_, response| {
                let (escape, down, tab) = response.ctx.input(|input| {
                    (
                        input.key_pressed(egui::Key::Escape),
                        input.key_pressed(egui::Key::ArrowDown),
                        input.key_pressed(egui::Key::Tab),
                    )
                });
                let focused = response.has_focus();
                let tabbed_away = tab && response.lost_focus();
                if (escape && (focused || response.lost_focus())) || tabbed_away {
                    Typing::CLOSED.store(&handled, key);
                } else if (down && focused && !open) || response.clicked() {
                    Typing::opened().store(&handled, key);
                }
                response.ctx.accesskit_node_builder(response.id, |node| {
                    node.set_has_popup(egui::accesskit::HasPopup::Listbox);
                    node.set_expanded(open);
                    node.set_value(shown.as_str());
                });
            });
            let typed = ctx.clone();
            field.on_change(move |_, text| {
                Typing {
                    open: true,
                    query: Some(text),
                    focus: false,
                }
                .store(&typed, key);
            });
            let toggled = ctx.clone();
            match clearing {
                Some(clear) => {
                    let cleared = ctx.clone();
                    let icon = s.part(ComboboxPart::ClearIcon, &[], tw());
                    let classes = end_button(s, ComboboxPart::Clear, &[]);
                    s.div_id((key, Node::Clear), classes, |s| {
                        s.glyph(icon, Icon::new(icons::CLOSE));
                    })
                    .role(Role::Button)
                    .named("Clear")
                    .on_click(move |app| {
                        clear(app);
                        Typing::CLOSED.focusing().store(&cleared, key);
                    });
                }
                None => {
                    let icon = s.part(ComboboxPart::TriggerIcon, &[], tw());
                    let expanded: &[SheetKey] = match open {
                        true => &[SheetKey::Aria(crate::Aria::Expanded)],
                        false => &[],
                    };
                    let classes = end_button(s, ComboboxPart::Trigger, expanded);
                    s.div_id((key, Node::Chevron), classes, |s| {
                        s.glyph(icon, Icon::new(icons::CHEVRON_DOWN))
                    })
                    .role(Role::Button)
                    .named("Show options")
                    .on_click(move |_| {
                        let next = if open {
                            Typing::CLOSED
                        } else {
                            Typing::opened()
                        };
                        next.focusing().store(&toggled, key);
                    });
                }
            }
        });
        if !open {
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
        let (close, dismiss) = (ctx.clone(), ctx.clone());
        let after: Rc<dyn Fn(&mut A)> = Rc::new(move |_| Typing::CLOSED.store(&close, key));
        b.overlay((key, Node::Popover))
            .anchor(Anchor::Node(group_id))
            .placement(Placement::below().gap(list::list_offset()))
            .match_width(MatchWidth::Anchor)
            .focus(Focus::Stay)
            .on_dismiss(move |_| Typing::CLOSED.store(&dismiss, key))
            .children(|s| {
                let content = s.part(ComboboxPart::Content, &[], tw().flex().flex_col().min_h_0());
                s.div_id(Node::Box, content, |s| {
                    list::show(
                        s,
                        key,
                        items,
                        options,
                        ListOptions {
                            looks: list::Looks::Combobox,
                            first: First::Highlighted,
                            after: Some(after),
                            field: list::Field::Theirs {
                                id: field_id,
                                text: state.query.unwrap_or_default(),
                            },
                        },
                    );
                });
            });
    }
}

/// TODO(prose): an autosuggest, started with [`Ext::autosuggest`] and finished with [`on_change`](Self::on_change).
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
        field.with("eggshell-ui-suggesting")
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

    /// TODO(prose): who filters; see [`List::filtering`].
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
        let field_id = b.id_of((key, Node::Field));
        let mut field = b
            .text_field((key, Node::Field), text.clone())
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
        let focused = b.has_focus_within((key, Node::Field));
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
        b.overlay((key, Node::Suggestions))
            .anchor(Anchor::Node(field_id))
            .placement(Placement::below().gap(style.menu.offset))
            .match_width(MatchWidth::Anchor)
            .focus(Focus::Stay)
            .on_dismiss(move |_| Suggesting::Dismissed.store(&dismiss, field_id))
            .children(|s| {
                let content = s.part(ComboboxPart::Content, &[], tw().flex().flex_col().min_h_0());
                s.div_id(Node::Box, content, |s| {
                    list::show(
                        s,
                        key,
                        items,
                        options,
                        list::ListOptions {
                            looks: list::Looks::Combobox,
                            first: First::Left,
                            after: Some(after),
                            field: list::Field::Theirs { id: field_id, text },
                        },
                    );
                });
            });
    }
}

/// The classes of a combobox's trigger or clear, `part`: shadcn's
/// `InputGroupButton size="icon-xs"`, a ghost `Button`, under the part's
/// own rules, with `keys` for every rule.
fn end_button<A>(s: &mut Builder<'_, '_, A>, part: ComboboxPart, keys: &[SheetKey]) -> Tw {
    use crate::contract::Size;
    use crate::theme::{size, variant};
    let button = [
        &[
            SheetKey::Variant(variant::GHOST),
            SheetKey::Size(Size::DEFAULT),
        ][..],
        keys,
    ]
    .concat();
    let group = [&[SheetKey::Size(size::ICON_XS)][..], keys].concat();
    s.part(crate::owned::button::Part::Root, &button, tw())
        .merge(s.part(crate::owned::input::GroupPart::Button, &group, tw()))
        .merge(s.part(part, keys, tw().sense_click().focusable_none()))
}
