//! The one list a command list, a combobox and an autosuggest show: a search
//! field, and the items that match its query in a virtual list it drives.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::owned::{Enablement, Mark, input, menu};
use crate::sheet::Key as SheetKey;
use crate::vlist::{ListNav, RowKind};
use crate::{
    Builder, Command, CommandState, Commands, Icon, Key, Scope, Shell, Tw, child_id, icons, tw,
};
use egui::{Color32, Context, Id, Role};

use crate::owned::styles::{Popover, menu_part};

use super::{ComboboxPart, Part};

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

/// TODO(prose): the items of a list, in groups, as [`List::items`](super::List::items) is given them.
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
        list.with("eggshell-ui-query")
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
    /// Whose parts the list draws.
    pub(crate) looks: Looks,
    pub(crate) first: First,
    /// What runs after an item's own pick: closing what holds the list.
    pub(crate) after: Option<Pick<A>>,
    /// The search field, or the app's own field, and what typing in it does.
    pub(crate) field: Field,
}

/// Whose parts a list draws: a combobox's or a command palette's, from the
/// style's sheet with the list's own look as their defaults, or the list's
/// own look alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Looks {
    Combobox,
    Command,
}

/// The parts a combobox's list and a command palette's list share.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListPart {
    List,
    Item,
    Empty,
    Heading,
}

impl ListPart {
    fn combobox(self) -> ComboboxPart {
        match self {
            Self::List => ComboboxPart::List,
            Self::Item => ComboboxPart::Item,
            Self::Empty => ComboboxPart::Empty,
            Self::Heading => ComboboxPart::Label,
        }
    }

    fn command(self) -> Part {
        match self {
            Self::List => Part::List,
            Self::Item => Part::Item,
            Self::Empty => Part::Empty,
            Self::Heading => Part::GroupHeading,
        }
    }
}

impl Looks {
    /// The classes of `part` for a node with `keys`: `own` as the part's
    /// defaults, under the sheet, for a combobox or a palette; `own` alone
    /// otherwise.
    pub(crate) fn part<A>(
        self,
        b: &mut Builder<'_, '_, A>,
        (part, keys): (ListPart, &[SheetKey]),
        call_site: Tw,
    ) -> Tw {
        match self {
            Self::Combobox => b.part(part.combobox(), keys, call_site),
            Self::Command => b.part(part.command(), keys, call_site),
        }
    }

    /// The classes of a part only a palette has: `own` under the sheet for
    /// a palette, `own` alone otherwise.
    fn command_part<A>(
        self,
        b: &mut Builder<'_, '_, A>,
        (part, keys): (Part, &[SheetKey]),
        own: Tw,
        call_site: Tw,
    ) -> Tw {
        match self {
            Self::Command => b.part(part, keys, call_site),
            Self::Combobox => own.merge(call_site),
        }
    }
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
            looks: Looks::Command,
            first: First::Highlighted,
            after,
            field: Field::Own,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
enum Node {
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
    /// The heading rows, each before its group's items: the group's heading
    /// and whether it is the list's first row.
    headings: HashMap<Row, (String, Place)>,
}

/// Where a group's heading is: first in the list, or after another group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    First,
    After,
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
        let mut headings = HashMap::new();
        for (at, group) in by_group.into_iter().enumerate() {
            let ranked = filter::ranked(group);
            if let (Some(Some(heading)), false) = (groups.get(at), ranked.is_empty()) {
                let row = Row(Id::NULL.with(("heading", at)));
                let place = match rows.is_empty() {
                    true => Place::First,
                    false => Place::After,
                };
                headings.insert(row, (heading.clone(), place));
                rows.push(row);
            }
            for entry in ranked {
                rows.push(entry.row);
                kept.insert(entry.row, entry);
            }
        }
        Self {
            rows,
            entries: kept,
            headings,
        }
    }

    fn kind(&self, row: Row) -> RowKind {
        match self.headings.contains_key(&row) {
            true => RowKind::Heading,
            false => RowKind::Option,
        }
    }
}

/// What the list's rows are drawn with, as `style`'s menu items are: the
/// colours from the tokens, the item's radius and the heading's weight
/// from the sheet's menu parts.
#[derive(Clone, Copy)]
struct Rows {
    muted: Color32,
    separator: Color32,
    heading_weight: crate::Weight,
}

impl Rows {
    fn of(style: &crate::Style) -> Self {
        let k = &style.tokens;
        let heading_role = style.role(&Some(style.menu.shortcut_text.clone())).clone();
        let heading = menu_part(style, menu::Part::Label, &[])
            .text_classes()
            .over(&crate::TextStyle::new(heading_role, Color32::PLACEHOLDER));
        Self {
            muted: k.muted_foreground,
            separator: menu_part(style, menu::Part::Separator, &[])
                .paint_classes()
                .background
                .unwrap_or(k.border),
            heading_weight: heading.spec.weight,
        }
    }
}

/// The gap between a combobox's field and its list: shadcn's `sideOffset 6`.
pub(crate) fn list_offset() -> crate::overlay::Distance {
    crate::overlay::Distance::new(6.0)
}

/// The classes of a command dialog's box, as `popover` draws.
fn dialog_box(popover: &Popover) -> Tw {
    let rest = popover.colors;
    tw().flex()
        .flex_col()
        .w_px(512.0)
        .max_w_full()
        .border(popover.border_width)
        .border_color(rest.border)
        .bg(rest.fill)
        .text(rest.text)
        .rounded_px(f32::from(popover.radius) + 2.0)
        .overflow_hidden()
        .shadow(popover.shadow)
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
        error: _,
    } = options;
    let listing = Rc::new(Listing::filtered(items, &query, filter.as_ref(), filtering));
    let looks = shape.looks;
    let root = looks.command_part(
        b,
        (Part::Root, &[]),
        tw(),
        tw().flex().flex_col().min_h_0().merge(classes),
    );
    b.div_id(key, root, |s| {
        let field = match field {
            Some(field) => field,
            None => search_field(s, key, &query, placeholder, name.clone(), on_query),
        };
        let rows = listing.rows.clone();
        if rows.is_empty() {
            match filtering {
                Filtering::App(Pending::Loading) => {
                    let busy = [SheetKey::Aria(crate::Aria::Busy)];
                    let classes = looks.part(s, (ListPart::Empty, &busy), tw());
                    s.div_id(Node::Loading, classes, |s| s.text(tw(), "Loading…"));
                }
                Filtering::App(Pending::Settled) | Filtering::Ours => {
                    let classes = looks.part(s, (ListPart::Empty, &[]), tw());
                    s.div_id(Node::Empty, classes, |s| match empty {
                        Some(empty) => empty(s),
                        None => s.text(tw(), "No results found."),
                    });
                }
            }
            return;
        }
        let text = style.role(&style.menu.text).clone();
        let row_height = style.menu.row_height(&text);
        let list_classes = looks.part(
            s,
            (ListPart::List, &[]),
            match looks {
                // `scroll-py-1`, which the importer does not map.
                Looks::Command => tw().scroll_py(1.0),
                Looks::Combobox => tw(),
            },
        );
        let mut list = s
            .virtual_list(Node::Results, list_classes, rows)
            .nav(ListNav::Listbox)
            .keys_from(field)
            .estimate_px(row_height);
        if shape.first == First::Highlighted {
            list = list.highlight_first();
        }
        {
            let rows_of = Rc::clone(&listing);
            list = list.row_part(move |b, row| {
                if rows_of.kind(row) == RowKind::Heading {
                    return tw();
                }
                let enablement = rows_of
                    .entries
                    .get(&row)
                    .map_or(Enablement::Disabled, |entry| entry.enablement);
                let keys: &[SheetKey] = match enablement {
                    Enablement::Enabled => &[],
                    Enablement::Disabled => &[SheetKey::Aria(crate::Aria::Disabled)],
                };
                looks.part(b, (ListPart::Item, keys), tw())
            });
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
        let kinds = Rc::clone(&listing);
        list.row_kind(move |row| kinds.kind(row))
            .row_name(move |row| {
                labels
                    .entries
                    .get(&row)
                    .map(|entry| entry.label.clone())
                    .or_else(|| {
                        labels
                            .headings
                            .get(&row)
                            .map(|(heading, _)| heading.clone())
                    })
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
                if let Some((heading, place)) = rows_of.headings.get(&row) {
                    match looks {
                        Looks::Command => palette_heading(s, *place, heading),
                        Looks::Combobox => {
                            let classes = looks.part(s, (ListPart::Heading, &[]), tw());
                            s.div(classes, |s| s.text(tw(), heading.clone()));
                        }
                    }
                }
                if let Some(entry) = rows_of.entries.get(&row) {
                    match looks {
                        Looks::Combobox => combobox_item(s, entry),
                        Looks::Command => palette_item(s, entry),
                    }
                }
            });
    });
}

fn search_field<A: 'static>(
    s: &mut Scope<'_, '_, A>,
    list: Id,
    query: &str,
    placeholder: Option<String>,
    name: Option<String>,
    on_query: Option<super::Heard<A>>,
) -> Id {
    let wrapper = s.part(Part::InputWrapper, &[], tw());
    let classes = s.part(Part::InputGroup, &[], tw().flex().items_center());
    let icon = s.part(Part::InputIcon, &[], tw());
    // shadcn's `CommandInput` is an `InputGroup` holding its icon in an
    // `InputGroupAddon` and its field as the group's input: core's parts.
    let start = SheetKey::Attr("align", "inline-start");
    let addon = s.part(
        input::GroupPart::Addon,
        &[start],
        tw().attr("align", "inline-start"),
    );
    let input = s
        .part(
            input::GroupPart::Input,
            &[SheetKey::Attr("addon", "inline-start")],
            tw().grow().min_w_0(),
        )
        .merge(s.part(Part::Input, &[], tw()));
    let ctx = s.ctx().clone();
    let mut field = Id::NULL;
    s.div_id(Node::Wrapper, wrapper, |s| {
        s.div_id(Node::Search, classes, |s| {
            field = s.id_of(Node::Query);
            s.div(addon, |s| {
                s.icon("icon", icon, Icon::new(icons::SEARCH));
            });
            let mut chain = s
                .text_field(Node::Query, query)
                .bare()
                .role(Role::ComboBox)
                .classes(input);
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

/// A heading's classes, inset by `padding_x`, in `role`, as `rows` draw.
fn heading_from(padding_x: f32, role: crate::TextSpec, rows: &Rows, place: Place) -> Tw {
    let classes = tw()
        .px_px(padding_x)
        .py(1.5)
        .role(role)
        .font_weight(rows.heading_weight)
        .text(rows.muted);
    match place {
        Place::First => classes,
        Place::After => classes.mt(1.0).border_t(1.0).border_color(rows.separator),
    }
}

/// A palette's group heading, with a `CommandSeparator` before every group
/// but the first.
fn palette_heading<A: 'static>(s: &mut Scope<'_, '_, A>, place: Place, heading: &str) {
    if place == Place::After {
        let separator = s.part(Part::Separator, &[], tw());
        s.div(separator, |_| {});
    }
    let classes = s.part(Part::GroupHeading, &[], tw());
    s.div(classes, |s| s.text(tw(), heading.to_owned()));
}

/// A palette's command, in a row that is its `Part::Item`: its
/// label, its shortcut and its check.
fn palette_item<A: 'static>(s: &mut Scope<'_, '_, A>, entry: &Entry<A>) {
    s.div(tw().grow().min_w_0(), |s| s.text(tw(), entry.label.clone()));
    if let Some(shortcut) = &entry.shortcut {
        let classes = s.part(Part::Shortcut, &[], tw());
        s.text(classes, shortcut.clone());
    }
    if let CommandState::Checked(Mark::On) | CommandState::Chosen(Mark::On) = entry.state {
        let indicator = s.part(
            Part::ItemIndicator,
            &[SheetKey::Attr(CHECKED.0, CHECKED.1)],
            tw(),
        );
        s.glyph(indicator, Icon::new(icons::CHECK));
    }
}

/// The key a checked command's mark carries, as shadcn's item does.
const CHECKED: (&str, &str) = ("checked", "true");

/// A combobox's option, in a row that is its `ComboboxPart::Item`: its
/// label, its shortcut and the room for its check, shadcn's item indicator.
fn combobox_item<A: 'static>(s: &mut Scope<'_, '_, A>, entry: &Entry<A>) {
    s.div(tw().grow().min_w_0(), |s| s.text(tw(), entry.label.clone()));
    if let Some(shortcut) = &entry.shortcut {
        s.text(tw().ml_auto(), shortcut.clone());
    }
    let indicator = s.part(ComboboxPart::ItemIndicator, &[], tw());
    let check = s.part(ComboboxPart::ItemIndicatorIcon, &[], tw());
    s.div(indicator, |s| {
        if let CommandState::Checked(Mark::On) | CommandState::Chosen(Mark::On) = entry.state {
            s.glyph(check, Icon::new(icons::CHECK));
        }
    });
}

/// Writes the command palette's recorded exceptions: its dialog's box and
/// its group headings, which shadcn draws through classes the generated
/// look does not carry, in the palette's colours from `style`.
pub(crate) fn write_exceptions(style: &mut crate::Style) {
    let popover = Popover::of(style);
    let (padding_x, rows) = (style.menu.item_padding.x, Rows::of(style));
    let role = style.role(&Some(style.menu.shortcut_text.clone())).clone();
    style
        .sheet
        .rule_under(Part::Dialog, move |_, _| dialog_box(&popover))
        .rule_under(Part::GroupHeading, move |_, _| {
            heading_from(padding_x, role.clone(), &rows, Place::First)
        });
}
