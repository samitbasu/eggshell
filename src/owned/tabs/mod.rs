//! TODO(prose): the tabs' parts, which the segmented control draws too, and
//! their recorded exceptions over the generated `cn-tabs*` look.

#[cfg(feature = "tabs")]
mod widget;

#[cfg(feature = "tabs")]
pub use widget::{Chain, Changes, Ext, Unset};

use crate::contract::{Keyed, Size, Variant};
use crate::sheet::{Key, Sheet, When};
use crate::theme::{Tokens, size};
use crate::{Aria, Builder, Match};

use super::{Enablement, segmented, segmented::Cell};

/// TODO(prose): the anatomy of tabs: shadcn's `tabs`. A segmented control is
/// a list of triggers alone.
///
/// TODO(prose): the keys its rules see: the size, `aria-disabled`, a list's
/// variant (shadcn's `default`, the only one eggshell draws), a trigger's
/// `aria-selected` (shadcn's `data-active`), and `data-[rows=wrapping]` on a
/// list and its triggers when they wrap, a key of eggshell's own: shadcn's
/// lists do not wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the tabs, holding the list and the panel.
    Root,
    /// TODO(prose): the track of triggers.
    List,
    /// TODO(prose): one trigger: a tab, or a segment.
    Trigger,
    /// TODO(prose): the panel of the selected tab.
    Content,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "tabs.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "tabs",
            Self::List => "tabs-list",
            Self::Trigger => "tabs-trigger",
            Self::Content => "tabs-content",
        }
    }
}

/// The attribute of a list whose triggers wrap onto more rows.
const WRAPPING: (&str, &str) = ("rows", "wrapping");

/// The keys a list and its triggers share.
fn shared_keys(size: Size, rows: segmented::Rows, enablement: Enablement) -> Vec<Key> {
    let mut keys = vec![Key::Size(size)];
    if rows == segmented::Rows::Wrapping {
        keys.push(Key::Attr(WRAPPING.0, WRAPPING.1));
    }
    if enablement == Enablement::Disabled {
        keys.push(Key::Aria(Aria::Disabled));
    }
    keys
}

impl Part {
    /// TODO(prose): the size whose rules draw triggers of `size` here: `size`, or the style's default reported as missing.
    #[track_caller]
    pub fn size_key<A>(b: &Builder<'_, '_, A>, size: Size) -> Size {
        let sizes = sizes();
        sizes.lookup(b.ctx(), size);
        sizes.key_for(size)
    }

    /// TODO(prose): the keys of a list of `size` laid out in `rows`, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, segmented, tabs::Part};
    ///
    /// let keys = Part::list_keys(Size::DEFAULT, segmented::Rows::One, Enablement::Enabled);
    /// assert_eq!(keys.len(), 2);
    /// ```
    pub fn list_keys(size: Size, rows: segmented::Rows, enablement: Enablement) -> Vec<Key> {
        let mut keys = shared_keys(size, rows, enablement);
        keys.push(Key::Variant(Variant::DEFAULT));
        keys
    }

    /// TODO(prose): the keys of a trigger in such a list, the selected one if `cell` is pressed.
    ///
    /// ```
    /// use eggshell_ui::Aria;
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{segmented::{Cell, self}, Enablement, tabs::Part};
    /// use eggshell_ui::sheet::Key;
    ///
    /// let keys = Part::trigger_keys(
    ///     Size::DEFAULT,
    ///     segmented::Rows::One,
    ///     Cell::Pressed,
    ///     Enablement::Enabled,
    /// );
    /// assert!(keys.contains(&Key::Aria(Aria::Selected)));
    /// ```
    pub fn trigger_keys(
        size: Size,
        rows: segmented::Rows,
        cell: Cell,
        enablement: Enablement,
    ) -> Vec<Key> {
        let mut keys = shared_keys(size, rows, enablement);
        if cell == Cell::Pressed {
            keys.push(Key::Aria(Aria::Selected));
        }
        keys
    }
}

/// The sizes the tabs and the segmented control are drawn at: shadcn's one
/// and eggshell's own smaller two.
fn sizes() -> Keyed<Size, ()> {
    Keyed::new(Size::DEFAULT, ())
        .with(size::SM, ())
        .with(size::XS, ())
}

/// A tab list's and its triggers' metrics at a size.
struct Metrics {
    height: f32,
    track_padding: f32,
    cell_padding: egui::Vec2,
    row_gap: f32,
    icon_size: f32,
    text: Option<crate::contract::TextSize>,
}

/// tabs.tsx, trigger: `h-[calc(100%-1px)]`.
const CELL_TRIM: f32 = 1.0;

fn metrics(k: &Tokens, size: Size) -> Metrics {
    let u = |units: f32| k.points(units);
    let at = |height: f32, padding_x: f32, icon_size: f32, text| Metrics {
        height,
        track_padding: k.list_padding,
        // tabs.tsx, trigger: `px-2 py-1` in a box 1 px shorter than the list's
        // inside, which overflows its `text-sm` by 1 px; the trimmed point
        // comes out of the padding instead.
        cell_padding: egui::vec2(padding_x, k.points(k.cell_padding.y) - CELL_TRIM / 2.0),
        row_gap: 2.0,
        icon_size,
        text,
    };
    let (padding_x, icon) = (k.points(k.cell_padding.x), k.points(k.icon_size));
    match size {
        s if s == size::XS => at(
            k.control_height() - u(3.0),
            (padding_x - u(1.0)).max(0.0),
            u(3.0),
            Some(k.text.xs),
        ),
        s if s == size::SM => at(k.control_height() - u(1.0), padding_x, icon, None),
        _ => at(k.control_height(), padding_x, icon, None),
    }
}

/// Writes the tabs' recorded exceptions over the rules at the sheet's current
/// layer: the list's track keeps the `input` fill, which shadcn's `bg-muted`
/// would lose on a muted panel (#346, revisit with #443); a trigger's
/// `h-[calc(100%-1px)]` is the list's full height inside a list padded half a
/// point more, with the point its `text-sm` would overflow taken out of its
/// own padding; the sizes other than the default keep their metrics; and
/// wrapping lists, eggshell's own, keep their rows, with each trigger growing
/// from its content's width where `flex-1` would share a row equally.
pub(crate) fn write_exceptions(sheet: &mut Sheet) {
    sheet.rule(Part::List, |t, k| t.bg(k.input));
    let one_row = When::always().not_attr(WRAPPING.0, WRAPPING.1);
    let wrapping = When::always().attr(WRAPPING.0, WRAPPING.1);
    let sizes = sizes();
    let default = sizes.default_key();
    for size in sizes.keys() {
        let when = When::always().size(size);
        sheet.rule_when(Part::Trigger, when, move |t, k| {
            t.py_px(metrics(k, size).cell_padding.y)
        });
        sheet.rule_when(Part::List, one_row.size(size), move |t, k| {
            t.py_px(metrics(k, size).track_padding + CELL_TRIM / 2.0)
        });
        sheet.rule_when(Part::List, wrapping.size(size), move |t, k| {
            t.flex_wrap().h_auto().gap_px(metrics(k, size).row_gap)
        });
        if size == default {
            sheet.rule_when(Part::List, when, |t, _| t);
            continue;
        }
        sheet.rule_when(Part::List, one_row.size(size), move |t, k| {
            t.h_px(metrics(k, size).height)
        });
        sheet.rule_when(Part::Trigger, when, move |t, k| {
            let m = metrics(k, size);
            let t = t
                .px_px(m.cell_padding.x)
                .descendants(Match::glyph(), |g| g.size_px(m.icon_size));
            match m.text {
                Some(text) => t.text_size(text.size, text.line_height),
                None => t,
            }
        });
    }
    sheet.rule_when(Part::Trigger, one_row, |t, _| t.h_full());
    sheet.rule_when(Part::Trigger, wrapping, |t, _| t.h_auto().basis_auto());
}
