//! The one list a command list, a combobox and an autosuggest show: a search
//! field, and the items that match its query in a virtual list it drives.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::vlist::ListNav;
use eggshell_ui::{
    Builder, Command, CommandState, Commands, Icon, Key, Scope, Shell, Style, Tw, child_id, icons,
    tw,
};
use egui::{Context, Id, Role};

use super::{Filter, Filtering, Options, Pending, filter};

type Pick<A> = Rc<dyn Fn(&mut A)>;

/// The key of a row: the item's group and its own key, the same from frame
/// to frame while the query narrows the rows.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct Row(Id);

struct Entry<A> {
    row: Row,
    group: usize,
    label: String,
    keywords: Vec<String>,
    shortcut: Option<String>,
    state: CommandState,
    enablement: Enablement,
    pick: Option<Pick<A>>,
}

/// TODO(prose): the items of a list, in groups, as [`CommandList::items`](super::CommandList::items) is given them.
pub struct Items<A> {
    entries: Vec<Entry<A>>,
    /// Each group's heading; the first is the items outside every group.
    groups: Vec<Option<String>>,
    group: usize,
    shell: Shell,
}

impl<A: 'static> Items<A> {
    pub(crate) fn new(b: &Builder<'_, '_, A>) -> Self {
        Self {
            entries: Vec::new(),
            groups: vec![None],
            group: 0,
            shell: b.shell(),
        }
    }

    /// TODO(prose): the items `body` adds, under the heading `heading`.
    pub fn group(&mut self, heading: impl Into<String>, body: impl FnOnce(&mut Self)) {
        self.groups.push(Some(heading.into()));
        let outer = std::mem::replace(&mut self.group, self.groups.len() - 1);
        body(self);
        self.group = outer;
    }

    /// TODO(prose): an item labelled `label`, under its own key `key` in its group.
    pub fn item<K: Key>(&mut self, key: K, label: impl Into<String>) -> ItemChain<'_, A> {
        let row = Row(child_id(Id::NULL.with(self.group), &key));
        self.entries.push(Entry {
            row,
            group: self.group,
            label: label.into(),
            keywords: Vec::new(),
            shortcut: None,
            state: CommandState::Plain,
            enablement: Enablement::Enabled,
            pick: None,
        });
        let at = self.entries.len() - 1;
        ItemChain { items: self, at }
    }

    /// TODO(prose): each command of `table` under the heading `heading`: its title, its shortcut, whether it is enabled and its check or dot; a pick runs it.
    pub fn commands(&mut self, heading: impl Into<String>, table: &Commands<A>, app: &A) {
        let shell = self.shell.clone();
        self.group(heading, |items| {
            for (at, command) in table.iter().enumerate() {
                let command: Command<A> = *command;
                let shell = shell.clone();
                let shortcut = command.shortcut_text(shell.ctx());
                let mut item = items
                    .item(at, command.title())
                    .state(command.state(app))
                    .enablement(command.is_enabled(app).into());
                if let Some(shortcut) = shortcut {
                    item = item.shortcut(shortcut);
                }
                item.on_pick(move |app| command.run(app, &shell));
            }
        });
    }
}

/// TODO(prose): an item being added; its options.
pub struct ItemChain<'i, A> {
    items: &'i mut Items<A>,
    at: usize,
}

impl<A> ItemChain<'_, A> {
    fn entry(&mut self) -> Option<&mut Entry<A>> {
        self.items.entries.get_mut(self.at)
    }

    /// TODO(prose): more words the item is found by, not shown: "remove" finds Delete.
    pub fn keywords(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if let Some(entry) = self.entry() {
            entry.keywords = words.into_iter().map(Into::into).collect();
        }
        self
    }

    /// TODO(prose): the shortcut the row shows at its end, as a menu item does.
    pub fn shortcut(mut self, text: impl Into<String>) -> Self {
        if let Some(entry) = self.entry() {
            entry.shortcut = Some(text.into());
        }
        self
    }

    /// TODO(prose): a check or a dot before the label, as a command's menu item shows.
    pub fn state(mut self, state: CommandState) -> Self {
        if let Some(entry) = self.entry() {
            entry.state = state;
        }
        self
    }

    /// TODO(prose): a disabled item is shown greyed, and the arrows and picks pass it by.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        if let Some(entry) = self.entry() {
            entry.enablement = enablement;
        }
        self
    }

    /// TODO(prose): runs `then` when the item is picked: clicked, or Enter on its highlight.
    pub fn on_pick(mut self, then: impl Fn(&mut A) + 'static) {
        if let Some(entry) = self.entry() {
            entry.pick = Some(Rc::new(then));
        }
    }
}

/// The query of a list: the widget's own state, kept from frame to frame.
pub(crate) struct Query;

impl Query {
    fn id(list: Id) -> Id {
        list.with("eggshell-ui-extras-query")
    }

    pub(crate) fn get(ctx: &Context, list: Id) -> String {
        ctx.data(|data| data.get_temp::<String>(Self::id(list)))
            .unwrap_or_default()
    }

    pub(crate) fn set(ctx: &Context, list: Id, query: String) {
        ctx.data_mut(|data| data.insert_temp(Self::id(list), query));
    }

    pub(crate) fn clear(ctx: &Context, list: Id) {
        ctx.data_mut(|data| data.remove::<String>(Self::id(list)));
    }
}

/// Whether the first row is highlighted as the rows change.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum First {
    Highlighted,
    Left,
}

/// How the list shows for its shape.
pub(crate) struct ListOptions<A> {
    pub(crate) first: First,
    /// What runs after an item's own pick: closing what holds the list.
    pub(crate) after: Option<Pick<A>>,
    /// The search field, or the app's own field, and what typing in it does.
    pub(crate) field: Field,
}

/// Where the query comes from.
pub(crate) enum Field {
    /// The list's own search field.
    Own,
    /// A field of the caller's, `id`, whose text is `text`.
    Theirs { id: Id, text: String },
}

impl<A> ListOptions<A> {
    pub(crate) fn palette(after: Option<Pick<A>>) -> Self {
        Self {
            first: First::Highlighted,
            after,
            field: Field::Own,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Wrapper,
    Search,
    Query,
    Results,
    Empty,
    Loading,
}

/// How well `entry` matches `query`, by `filter` or the library's own, or
/// as the app gave it when the app filters.
fn score<A>(
    entry: &Entry<A>,
    query: &str,
    filter: Option<&Filter>,
    filtering: Filtering,
) -> Option<filter::Score> {
    let candidate = filter::Candidate {
        label: &entry.label,
        keywords: &entry.keywords,
    };
    match (filtering, filter) {
        (Filtering::App(_), _) => Some(filter::Score::BEST),
        (Filtering::Ours, Some(filter)) => filter(query, candidate),
        (Filtering::Ours, None) => filter::word_prefix(query, candidate),
    }
}

impl<A> Items<A> {
    /// Whether any item matches `query`.
    pub(crate) fn any_match(
        &self,
        query: &str,
        filter: Option<&Filter>,
        filtering: Filtering,
    ) -> bool {
        self.entries
            .iter()
            .any(|entry| score(entry, query, filter, filtering).is_some())
    }
}

/// The items that match `query`, ranked, as rows; the entries by their row.
pub(crate) struct Listing<A> {
    rows: Vec<Row>,
    entries: HashMap<Row, Entry<A>>,
    groups: Vec<Option<String>>,
}

impl<A> Listing<A> {
    /// The items of `items` that `query` matches, best first within each
    /// group, the groups in the order given, empty groups left out.
    fn filtered(
        items: Items<A>,
        query: &str,
        filter: Option<&Filter>,
        filtering: Filtering,
    ) -> Self {
        let Items {
            entries, groups, ..
        } = items;
        let mut by_group: Vec<Vec<(Option<filter::Score>, Entry<A>)>> =
            groups.iter().map(|_| Vec::new()).collect();
        for entry in entries {
            let scored = (score(&entry, query, filter, filtering), entry);
            if let Some(group) = by_group.get_mut(scored.1.group) {
                group.push(scored);
            }
        }
        let mut rows = Vec::new();
        let mut kept = HashMap::new();
        for group in by_group {
            for entry in filter::ranked(group) {
                rows.push(entry.row);
                kept.insert(entry.row, entry);
            }
        }
        Self {
            rows,
            entries: kept,
            groups,
        }
    }

    /// Whether `row` is the first of its group, which shows the heading.
    fn heading(&self, at: usize) -> Option<&str> {
        let row = self.rows.get(at)?;
        let group = self.entries.get(row)?.group;
        let first = at == 0
            || self
                .rows
                .get(at - 1)
                .and_then(|before| self.entries.get(before))
                .is_some_and(|before| before.group != group);
        first.then(|| self.groups.get(group)?.as_deref()).flatten()
    }
}

/// The classes of the box a combobox's or an autosuggest's list sits in.
pub(crate) fn popover_classes(style: &Style) -> Tw {
    style
        .menu
        .box_classes(style.role(&style.menu.text))
        .p(0.0)
        .overflow_hidden()
}

/// The gap between a combobox's field and its list: shadcn's `sideOffset 6`.
pub(crate) fn list_offset() -> eggshell_ui::overlay::Distance {
    eggshell_ui::overlay::Distance::new(6.0)
}

/// The classes of a combobox's box, shadcn's input group: the select's box
/// with the ring while the field inside has focus. The field's own padding
/// is the box's left, and the end button sits close to its right.
pub(crate) fn group_classes(style: &Style) -> Tw {
    let select = &style.select;
    let metrics = select.sizes.get(eggshell_ui::contract::Size::DEFAULT);
    let states = select.trigger.paint.states;
    let classes = tw()
        .flex()
        .items_center()
        .pl(metrics.padding_x / 4.0)
        .pr(1.0)
        .h_px(metrics.height)
        .rounded_px(f32::from(metrics.radius))
        .border(select.trigger.border_width)
        .border_color(states.rest.border)
        .bg(states.rest.fill)
        .text(states.rest.text)
        .role(style.role(&select.text).clone())
        .focus_within(|p| {
            p.border_color(states.focused.border)
                .ring(style.focus.width, style.focus.color)
        });
    match select.trigger.paint.shadow {
        Some(shadow) => classes.shadow(shadow),
        None => classes,
    }
}

/// The classes of the button at the end of a combobox's box, shadcn's ghost
/// `icon-xs`: the stepper's button.
pub(crate) fn end_button_classes(style: &Style) -> Tw {
    let stepper = &style.stepper;
    let metrics = stepper.sizes.get(eggshell_ui::contract::Size::DEFAULT);
    let states = stepper.button.paint.states;
    tw().flex()
        .items_center()
        .justify_center()
        .size_px(metrics.button_width)
        .shrink_0()
        .rounded_px(f32::from(style.menu.item_radius))
        .bg(states.rest.fill)
        .sense_click()
        .focusable_none()
        .hover(|p| p.bg(states.hover.fill))
}

/// The classes of the glyph in the end button: muted, `size-4`.
pub(crate) fn end_glyph_classes(style: &Style) -> Tw {
    tw().size_px(style.select.chevron)
        .shrink_0()
        .text(style.input.icon)
}

/// The classes of a command dialog's box.
pub(crate) fn dialog_classes(style: &Style) -> Tw {
    let menu = &style.menu;
    let rest = menu.popup.paint.states.rest;
    let classes = tw()
        .flex()
        .flex_col()
        .w_px(512.0)
        .max_w_full()
        .border(menu.popup.border_width)
        .border_color(rest.border)
        .bg(rest.fill)
        .text(rest.text)
        .rounded_px(f32::from(menu.radius) + 2.0)
        .overflow_hidden();
    match menu.popup.paint.shadow {
        Some(shadow) => classes.shadow(shadow),
        None => classes,
    }
}

/// Adds the list with its field: the items of `items` that match the query,
/// in a virtual list the field's keys move through.
pub(crate) fn show<'f, A: 'static>(
    b: &mut Builder<'_, 'f, A>,
    key: Id,
    items: Items<A>,
    options: Options<'f, A>,
    shape: ListOptions<A>,
) {
    let style = Arc::clone(b.style());
    let ctx = b.ctx().clone();
    let command = super::CommandStyle::of(&ctx);
    let (field, query) = match &shape.field {
        Field::Own => (None, Query::get(&ctx, key)),
        Field::Theirs { id, text } => (Some(*id), text.clone()),
    };
    let Options {
        placeholder,
        name,
        filter,
        filtering,
        on_query,
        empty,
        classes,
    } = options;
    let listing = Rc::new(Listing::filtered(items, &query, filter.as_ref(), filtering));
    b.div_id(key, tw().flex().flex_col().min_h_0().merge(classes), |s| {
        let field = match field {
            Some(field) => field,
            None => search_field(s, &style, key, &query, placeholder, name.clone(), on_query),
        };
        let rows = listing.rows.clone();
        if rows.is_empty() {
            match filtering {
                Filtering::App(Pending::Loading) => {
                    s.div_id(Part::Loading, note_classes(&style), |s| {
                        s.text(tw(), "Loading…")
                    });
                }
                Filtering::App(Pending::Settled) | Filtering::Ours => {
                    s.div_id(Part::Empty, note_classes(&style), |s| match empty {
                        Some(empty) => empty(s),
                        None => s.text(tw(), "No results found."),
                    });
                }
            }
            return;
        }
        let text = style.role(&style.menu.text).clone();
        let row_height = style.menu.row_height(&text);
        let mut list = s
            .virtual_list(
                Part::Results,
                tw().max_h_px(command.list_height)
                    .p(style.menu.padding / 4.0)
                    .scroll_py(1.0)
                    .scrollbar_none(),
                rows,
            )
            .nav(ListNav::Listbox)
            .keys_from(field)
            .estimate_px(row_height);
        if shape.first == First::Highlighted {
            list = list.highlight_first();
        }
        if let Some(name) = name {
            list = list.named(name);
        }
        let (labels, enabled, picks) = (
            Rc::clone(&listing),
            Rc::clone(&listing),
            Rc::clone(&listing),
        );
        let marks = Rc::clone(&listing);
        let after = shape.after.clone();
        let rows_of = Rc::clone(&listing);
        let style_of = Arc::clone(&style);
        let command_of = Arc::clone(&command);
        list.row_name(move |row| {
            labels
                .entries
                .get(&row)
                .map(|entry| entry.label.clone())
                .unwrap_or_default()
        })
        .row_mark(move |row| {
            marks.entries.get(&row).and_then(|entry| match entry.state {
                CommandState::Plain => None,
                CommandState::Checked(mark) | CommandState::Chosen(mark) => Some(mark),
            })
        })
        .row_enablement(move |row| {
            enabled
                .entries
                .get(&row)
                .map_or(Enablement::Disabled, |entry| entry.enablement)
        })
        .on_activate(move |app, row| {
            let Some(entry) = picks.entries.get(&row) else {
                return;
            };
            if entry.enablement == Enablement::Disabled {
                return;
            }
            if let Some(pick) = &entry.pick {
                pick(app);
            }
            if let Some(after) = &after {
                after(app);
            }
        })
        .items(move |s, row| {
            let at = rows_of.rows.iter().position(|&r| r == row).unwrap_or(0);
            if let Some(heading) = rows_of.heading(at) {
                s.div(heading_classes(&style_of, at), |s| {
                    s.text(tw(), heading.to_owned())
                });
            }
            if let Some(entry) = rows_of.entries.get(&row) {
                item(s, &style_of, &command_of, entry);
            }
        });
    });
}

fn search_field<A: 'static>(
    s: &mut Scope<'_, '_, A>,
    style: &Style,
    list: Id,
    query: &str,
    placeholder: Option<String>,
    name: Option<String>,
    on_query: Option<super::Heard<A>>,
) -> Id {
    let menu = &style.menu;
    let rest = menu.popup.paint.states.rest;
    let tint = style
        .input
        .look
        .paint
        .states
        .rest
        .border
        .gamma_multiply(0.3);
    let wrapper = tw().p(1.0).pb(0.0);
    let classes = tw()
        .flex()
        .items_center()
        .gap(2.0)
        .pl(2.0)
        .pr(2.5)
        .h_px(32.0)
        .rounded_px(f32::from(menu.radius) + 2.0)
        .border(1.0)
        .border_color(tint)
        .bg(tint);
    let ctx = s.ctx().clone();
    let mut field = Id::NULL;
    s.div_id(Part::Wrapper, wrapper, |s| {
        s.div_id(Part::Search, classes, |s| {
            field = s.id_of(Part::Query);
            s.icon(
                "icon",
                tw().size_px(16.0)
                    .shrink_0()
                    .text(rest.text.gamma_multiply(0.5)),
                Icon::new(icons::SEARCH),
            );
            let mut chain = s
                .text_field(Part::Query, query)
                .bare()
                .role(Role::ComboBox)
                .classes(tw().grow().min_w_0());
            if let Some(placeholder) = placeholder {
                chain = chain.hint(placeholder);
            }
            if let Some(name) = name {
                chain = chain.named(name);
            }
            chain.on_change(move |app, query| {
                if let Some(on_query) = &on_query {
                    on_query(app, &query);
                }
                Query::set(&ctx, list, query);
            });
        });
    });
    field
}

fn note_classes(style: &Style) -> Tw {
    tw().py(6.0).text_center().text(style.menu.shortcut)
}

fn heading_classes(style: &Style, at: usize) -> Tw {
    let menu = &style.menu;
    let classes = tw()
        .px(menu.item_padding.x / 4.0)
        .py(1.5)
        .role(style.role(&Some(menu.shortcut_text.clone())).clone())
        .font_weight(menu.heading_weight)
        .text(menu.shortcut);
    if at == 0 {
        classes
    } else {
        classes.mt(1.0).border_t(1.0).border_color(menu.separator)
    }
}

fn item<A: 'static>(
    s: &mut Scope<'_, '_, A>,
    style: &Style,
    command: &super::CommandStyle,
    entry: &Entry<A>,
) {
    let menu = &style.menu;
    let text = style.role(&menu.text);
    let states = menu.item.paint.states;
    let base = tw()
        .flex()
        .items_center()
        .gap(menu.item_gap / 4.0)
        .px(menu.item_padding.x / 4.0)
        .h_px(menu.row_height(text))
        .rounded_px(f32::from(menu.item_radius))
        .whitespace_nowrap();
    let classes = match entry.enablement {
        Enablement::Disabled => base.text(states.disabled.text),
        Enablement::Enabled => {
            let (fill, text) = (command.selected_fill, command.selected_text);
            base.bg(states.rest.fill)
                .text(states.rest.text)
                .group_hover(|p| p.bg(fill).text(text))
        }
    };
    let side = menu.indicator;
    s.div(classes, |s| {
        s.div(tw().grow().min_w_0(), |s| s.text(tw(), entry.label.clone()));
        if let Some(shortcut) = &entry.shortcut {
            s.text(tw().text(menu.shortcut).ml_auto(), shortcut.clone());
        }
        if let CommandState::Checked(Mark::On) | CommandState::Chosen(Mark::On) = entry.state {
            s.glyph(tw().size_px(side).shrink_0(), Icon::new(icons::CHECK));
        }
    });
}
