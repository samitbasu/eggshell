//! TODO(prose): grouped forms: sections of labelled rows, after shadcn/ui's
//! `FieldSet` in a `Card`'s frame and SwiftUI's `LabeledContent`.
//!
//! ```
//! use eggshell_ui::Builder;
//! use eggshell_ui_extras::form::FormExt;
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

mod style;

use eggshell_ui::owned::FieldPart;
use eggshell_ui::sheet::{Defaults, Part};
use eggshell_ui::{Builder, Key, Match, tw};
use egui::{Context, Id, Role};

pub use style::FormStyle;

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

/// TODO(prose): adds [`form_section`](Self::form_section),
/// [`labeled`](Self::labeled) and [`labeled_text`](Self::labeled_text) to
/// the builder.
pub trait FormExt<'b, 'f, A> {
    /// TODO(prose): a section of a form: a box of rows, under an optional
    /// title. Add it with [`children`](FormSectionChain::children).
    fn form_section<K: Key>(&mut self, key: K) -> FormSectionChain<'_, 'b, 'f, A, K>;

    /// TODO(prose): a row with `label` leading and the value `value` builds
    /// at its trailing edge. Inside a section, it takes the section's inset.
    fn labeled(&mut self, label: impl Into<String>, value: impl FnOnce(&mut Builder<'_, 'f, A>));

    /// TODO(prose): a [`labeled`](Self::labeled) row whose value is `text`,
    /// in the secondary colour.
    fn labeled_text(&mut self, label: impl Into<String>, text: impl Into<String>);
}

/// TODO(prose): a form section being configured; see
/// [`FormExt::form_section`].
#[must_use = "a form section is not added until you call `.children(..)`"]
pub struct FormSectionChain<'s, 'b, 'f, A, K> {
    b: &'s mut Builder<'b, 'f, A>,
    key: K,
    title: Option<String>,
    rules: Rules,
}

/// Whether a section draws a rule between its rows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rules {
    None,
    Between,
}

/// How many sections enclose what is being built now, so a row inside one
/// takes its inset. Building is synchronous, so the count is exact.
fn depth_id() -> Id {
    Id::NULL.with("eggshell-ui-extras-form-section-depth")
}

fn depth(ctx: &Context) -> u32 {
    ctx.data(|data| data.get_temp::<u32>(depth_id()))
        .unwrap_or(0)
}

fn set_depth(ctx: &Context, depth: u32) {
    ctx.data_mut(|data| data.insert_temp(depth_id(), depth));
}

/// The ordinal of the next keyless row under the parent `b` builds into,
/// counted afresh each pass: a row's positional key.
fn next_row<A>(b: &Builder<'_, '_, A>) -> u32 {
    let id = b.id_of("eggshell-ui-extras-labeled-rows");
    let pass = b.ctx().cumulative_pass_nr();
    b.ctx().data_mut(|data| {
        let (seen, next) = data.get_temp::<(u64, u32)>(id).unwrap_or((pass, 0));
        let next = if seen == pass { next } else { 0 };
        data.insert_temp(id, (pass, next + 1));
        next
    })
}

impl<'s, 'b, 'f, A: 'static, K: Key> FormSectionChain<'s, 'b, 'f, A, K> {
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
        let style = b.style().extension::<FormStyle>();
        let surface = tw()
            .bg(style.fill)
            .border(style.border_width)
            .border_color(style.border)
            .rounded_px(f32::from(style.radius));
        // Recorded exception (#446): the section keeps its own padding and
        // row spacing, not the card's `py`/`gap`, which space a card's
        // header, content and footer.
        let mut own = tw()
            .flex()
            .flex_col()
            .py(style.padding_y / 4.0)
            // A form's choice fields are shadcn's `Field`, `w-full`.
            .descendants(Match::slot(FieldPart::Root.slot()), |t| t.w_full());
        if rules == Rules::Between {
            own = own.divide_y(style.rule_width, style.rule);
        }
        let frame = b.part_over(CardPart::Root, &[], Defaults(surface), own);
        let section = b.div_id(key, tw().flex().flex_col().sense_hover(), |b| {
            if let Some(title) = &title {
                b.text(
                    tw().role(style.title.clone())
                        .text(style.title_color)
                        .mb(style.title_gap / 4.0),
                    title.clone(),
                );
            }
            b.div_id("Box", frame, |b| {
                let ctx = b.ctx().clone();
                let outer = depth(&ctx);
                set_depth(&ctx, outer + 1);
                rows(b);
                set_depth(&ctx, outer);
            });
        });
        let section = section.role(Role::Group);
        if let Some(title) = title {
            section.named(title);
        }
    }
}

impl<'b, 'f, A: 'static> FormExt<'b, 'f, A> for Builder<'b, 'f, A> {
    fn form_section<K: Key>(&mut self, key: K) -> FormSectionChain<'_, 'b, 'f, A, K> {
        FormSectionChain {
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
        let color = self.style().extension::<FormStyle>().value_color;
        let shown = text.clone();
        row(self, label.into(), Some(text), |b| {
            b.text(tw().text(color), shown)
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
    let style = b.style().extension::<FormStyle>();
    // The least height holds a control, inside the row's padding, so a
    // text row is as tall as a control row.
    let (classes, padding_y) = match depth(b.ctx()) {
        0 => (tw(), 0.0),
        _ => (
            tw().px(style.row_padding_x / 4.0)
                .py(style.row_padding_y / 4.0),
            style.row_padding_y,
        ),
    };
    let classes = classes
        .flex()
        .items_center()
        .gap(style.row_gap / 4.0)
        .min_h_px(style.row_min_height + 2.0 * padding_y)
        .sense_hover();
    let key = ("eggshell-labeled", next_row(b));
    let node = b
        .div_id(key, classes, |b| {
            b.text(
                tw().flex_1().min_w(0.0).text(style.label_color),
                label.clone(),
            );
            b.div_id("Value", tw().flex().items_center().shrink_0(), |b| value(b));
        })
        .role(Role::Group)
        .named(label);
    if let Some(description) = description {
        node.description(description);
    }
}
