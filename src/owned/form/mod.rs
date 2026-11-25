//! TODO(prose): grouped forms: sections of labelled rows, after shadcn/ui's
//! `FieldSet` in a `Card`'s frame and SwiftUI's `LabeledContent`.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui::prelude::*;
//!
//! struct App { gain: f64, mute: bool }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.form_section("audio")
//!         .title("Audio")
//!         .divided()
//!         .children(|b| {
//!             b.labeled("Gain", |b| {
//!                 b.slider("gain", app.gain, 0.0..=100.0)
//!                     .on_change(|app: &mut App, v| app.gain = v);
//!             });
//!             b.labeled_text("Sample rate", "48 kHz");
//!             b.labeled("Mute", |b| {
//!                 b.switch_unlabelled("mute", app.mute)
//!                     .named("Mute")
//!                     .on_change(|app: &mut App, on| app.mute = on);
//!             });
//!         });
//! }
//! ```

use crate::owned::field;
use crate::sheet::{Key as SheetKey, Part};
use crate::{Builder, Key, Match, tw};
use egui::Role;

/// TODO(prose): a form section's frame for the style's sheet: shadcn's
/// `card`, whose surface the section takes. The section's rows and title are
/// its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardPart {
    /// TODO(prose): the frame.
    Root,
}

impl Part for CardPart {
    const SOURCE: &'static str = "card.tsx";

    fn slot(self) -> &'static str {
        "card"
    }
}

/// TODO(prose): a form row's anatomy for the style's sheet: shadcn's
/// `item`, its title leading and its actions trailing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemPart {
    /// TODO(prose): the row.
    Root,
    /// TODO(prose): what holds the title.
    Content,
    /// TODO(prose): the row's label.
    Title,
    /// TODO(prose): a row's text value.
    Description,
    /// TODO(prose): what holds the row's value.
    Actions,
}

impl Part for ItemPart {
    const SOURCE: &'static str = "item.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "item",
            Self::Content => "item-content",
            Self::Title => "item-title",
            Self::Description => "item-description",
            Self::Actions => "item-actions",
        }
    }
}

/// TODO(prose): adds [`form_section`](Self::form_section),
/// [`labeled`](Self::labeled) and [`labeled_text`](Self::labeled_text) to
/// the builder.
pub trait Ext<'b, 'f, A> {
    /// TODO(prose): a section of a form: a box of rows, under an optional
    /// title. Add it with [`children`](SectionChain::children).
    fn form_section<K: Key>(&mut self, key: K) -> SectionChain<'_, 'b, 'f, A, K>;

    /// TODO(prose): a row with `label` leading and the value `value` builds
    /// at its trailing edge. Inside a section, it takes the section's inset.
    fn labeled(&mut self, label: impl Into<String>, value: impl FnOnce(&mut Builder<'_, 'f, A>));

    /// TODO(prose): a [`labeled`](Self::labeled) row whose value is `text`,
    /// in the secondary colour.
    fn labeled_text(&mut self, label: impl Into<String>, text: impl Into<String>);
}

/// TODO(prose): a form section being configured; see
/// [`Ext::form_section`].
#[must_use = "a form section is not added until you call `.children(..)`"]
pub struct SectionChain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    title: Option<String>,
    rules: Rules,
}

/// The attribute of a section that draws a rule between its rows.
const DIVIDED: (&str, &str) = ("divided", "true");

/// Writes the form section's recorded exceptions over the card's generated
/// look (#446): the box keeps its own padding, not the card's `py`/`gap`,
/// which space a card's header, content and footer, and a divided section
/// rules its rows off with `divide-y` in `border`.
pub(crate) fn write_exceptions(sheet: &mut crate::sheet::Sheet) {
    use crate::sheet::When;
    sheet.rule(CardPart::Root, |t, _| t.py_px(8.0));
    sheet.rule_when(
        CardPart::Root,
        When::always().attr(DIVIDED.0, DIVIDED.1),
        |t, k| t.divide_y(1.0, k.border),
    );
}

/// Whether a section draws a rule between its rows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rules {
    None,
    Between,
}

/// The ordinal of the next keyless row under the parent `b` builds into,
/// counted afresh each pass: a row's positional key.
fn next_row<A>(b: &Builder<'_, '_, A>) -> u32 {
    let id = b.id_of("eggshell-ui-labeled-rows");
    let pass = b.ctx().cumulative_pass_nr();
    b.ctx().data_mut(|data| {
        let (seen, next) = data.get_temp::<(u64, u32)>(id).unwrap_or((pass, 0));
        let next = if seen == pass { next } else { 0 };
        data.insert_temp(id, (pass, next + 1));
        next
    })
}

impl<'s, 'b, 'f, A: 'static, K: Key> SectionChain<'s, 'b, 'f, A, K> {
    /// TODO(prose): the title over the box; it names the section's group.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// TODO(prose): a rule between the rows, from edge to edge.
    pub fn divided(mut self) -> Self {
        self.rules = Rules::Between;
        self
    }

    /// TODO(prose): adds the section, its rows built by `rows`.
    pub fn children(self, rows: impl FnOnce(&mut Builder<'_, 'f, A>)) {
        let Self {
            b,
            key,
            title,
            rules,
        } = self;
        let divided: &[SheetKey] = match rules {
            Rules::Between => &[SheetKey::Attr(DIVIDED.0, DIVIDED.1)],
            Rules::None => &[],
        };
        // A form's choice fields are shadcn's `Field`, `w-full`.
        let own = tw()
            .flex()
            .flex_col()
            .descendants(Match::slot(field::Part::Root.slot()), |t| t.w_full());
        let frame = b.part(CardPart::Root, divided, own);
        let legend = b.part(
            field::Part::Legend,
            &[SheetKey::Variant(crate::contract::Variant("label"))],
            tw(),
        );
        let section = b.div_id(key, tw().flex().flex_col().sense_hover(), |b| {
            if let Some(title) = &title {
                b.text(legend, title.clone());
            }
            b.div_id("Box", frame, |b| rows(b));
        });
        let section = section.role(Role::Group);
        if let Some(title) = title {
            section.named(title);
        }
    }
}

impl<'b, 'f, A: 'static> Ext<'b, 'f, A> for Builder<'b, 'f, A> {
    fn form_section<K: Key>(&mut self, key: K) -> SectionChain<'_, 'b, 'f, A, K> {
        SectionChain {
            b: self,
            key,
            title: None,
            rules: Rules::None,
        }
    }

    fn labeled(&mut self, label: impl Into<String>, value: impl FnOnce(&mut Builder<'_, 'f, A>)) {
        row(self, label.into(), None, value);
    }

    fn labeled_text(&mut self, label: impl Into<String>, text: impl Into<String>) {
        let text = text.into();
        let shown = text.clone();
        row(self, label.into(), Some(text), |b| {
            let classes = b.part(ItemPart::Description, &[], tw());
            b.text(classes, shown)
        });
    }
}

/// A labelled row: `label` leading, `value` trailing, a group named by the
/// label. A text value is also the group's description, because core does
/// not put plain text in the accessibility tree.
fn row<'f, A: 'static>(
    b: &mut Builder<'_, 'f, A>,
    label: String,
    description: Option<String>,
    value: impl FnOnce(&mut Builder<'_, 'f, A>),
) {
    let item = [
        SheetKey::Variant(crate::contract::Variant("default")),
        SheetKey::Size(crate::contract::Size::DEFAULT),
    ];
    let classes = b.part(ItemPart::Root, &item, tw().sense_hover());
    let (content, title) = (
        b.part(ItemPart::Content, &[], tw().min_w(0.0)),
        b.part(ItemPart::Title, &[], tw()),
    );
    let actions = b.part(ItemPart::Actions, &[], tw().shrink_0());
    let key = ("eggshell-labeled", next_row(b));
    let node = b
        .div_id(key, classes, |b| {
            b.div(content, |b| b.text(title, label.clone()));
            b.div_id("Value", actions, |b| value(b));
        })
        .role(Role::Group)
        .named(label);
    if let Some(description) = description {
        node.description(description);
    }
}
