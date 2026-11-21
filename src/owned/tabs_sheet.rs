//! The tabs' parts, which the segmented control draws too, and the sheet rules
//! that give them today's [`SegmentedStyle`] look.
//!
//! TODO(#446): these rules are written from the values `SegmentedStyle` holds,
//! as the step that moves the mechanism onto the sheet; the generated rules of
//! shadcn's `cn-tabs*` replace them.

use crate::TextSpec;
use crate::contract::{Disabled, Focus, SegmentedStyle, Size};
use crate::sheet::{Key, Part, Sheet, When};
use crate::{Aria, Builder, Match};

use super::{Cell, Enablement, SegmentedRows, centered};

/// TODO(prose): the anatomy of tabs: shadcn's `tabs`. A segmented control is
/// a list of triggers alone.
///
/// TODO(prose): the keys its rules see: the size, `aria-disabled`, a trigger's
/// `aria-selected` (shadcn's `data-active`), and `data-[rows=wrapping]` on a
/// list and its triggers when they wrap, a key of eggshell's own: shadcn's
/// lists do not wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TabsPart {
    /// TODO(prose): the tabs, holding the list and the panel.
    Root,
    /// TODO(prose): the track of triggers.
    List,
    /// TODO(prose): one trigger: a tab, or a segment.
    Trigger,
    /// TODO(prose): the panel of the selected tab.
    Content,
}

impl Part for TabsPart {
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

impl TabsPart {
    /// TODO(prose): the size whose rules draw triggers of `size` here: `size`, or the style's default reported as missing.
    #[track_caller]
    pub fn size_key<A>(b: &Builder<'_, '_, A>, size: Size) -> Size {
        let sizes = &b.style().segmented.sizes;
        sizes.lookup(b.ctx(), size);
        sizes.key_for(size)
    }

    /// TODO(prose): the keys of a list of `size` laid out in `rows`, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, SegmentedRows, TabsPart};
    ///
    /// let keys = TabsPart::list_keys(Size::DEFAULT, SegmentedRows::One, Enablement::Enabled);
    /// assert_eq!(keys.len(), 1);
    /// ```
    pub fn list_keys(size: Size, rows: SegmentedRows, enablement: Enablement) -> Vec<Key> {
        let mut keys = vec![Key::Size(size)];
        if rows == SegmentedRows::Wrapping {
            keys.push(Key::Attr(WRAPPING.0, WRAPPING.1));
        }
        if enablement == Enablement::Disabled {
            keys.push(Key::Aria(Aria::Disabled));
        }
        keys
    }

    /// TODO(prose): the keys of a trigger in such a list, the selected one if `cell` is pressed.
    ///
    /// ```
    /// use eggshell_ui::Aria;
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Cell, Enablement, SegmentedRows, TabsPart};
    /// use eggshell_ui::sheet::Key;
    ///
    /// let keys = TabsPart::trigger_keys(
    ///     Size::DEFAULT,
    ///     SegmentedRows::One,
    ///     Cell::Pressed,
    ///     Enablement::Enabled,
    /// );
    /// assert!(keys.contains(&Key::Aria(Aria::Selected)));
    /// ```
    pub fn trigger_keys(
        size: Size,
        rows: SegmentedRows,
        cell: Cell,
        enablement: Enablement,
    ) -> Vec<Key> {
        let mut keys = Self::list_keys(size, rows, enablement);
        if cell == Cell::Pressed {
            keys.push(Key::Aria(Aria::Selected));
        }
        keys
    }
}

/// The text roles of a size's triggers: unselected, and selected.
pub(crate) struct Roles {
    pub(crate) size: Size,
    pub(crate) unselected: TextSpec,
    pub(crate) selected: TextSpec,
}

/// Writes the rules that draw `segmented`'s look at the sheet's current layer.
pub(crate) fn write_rules(
    sheet: &mut Sheet,
    segmented: &SegmentedStyle,
    roles: Vec<Roles>,
    (focus, disabled): (Focus, Disabled),
) {
    let wrapping = When::always().attr(WRAPPING.0, WRAPPING.1);
    let one_row = When::always().not_attr(WRAPPING.0, WRAPPING.1);
    sheet.rule(TabsPart::Root, |t, _| t.flex().flex_col().gap(2.0));
    sheet.rule(TabsPart::Content, |t, _| t.flex_1());
    sheet.rule_when(TabsPart::Trigger, one_row, |t, _| t.h_full());
    sheet.rule_when(TabsPart::Trigger, wrapping, |t, _| t.h_auto());
    if let Disabled::Faded(opacity) = disabled {
        sheet.rule_when(
            TabsPart::List,
            When::always().aria(Aria::Disabled),
            move |t, _| t.opacity(opacity),
        );
    }
    let fill = segmented.track_fill;
    let (width, transition) = (segmented.cell_border_width, segmented.transition);
    for Roles {
        size,
        unselected,
        selected,
    } in roles
    {
        let metrics = *segmented.sizes.get(size);
        sheet.rule_when(TabsPart::List, When::always().size(size), move |t, _| {
            t.flex()
                .items_center()
                .w_fit()
                .px(metrics.track_padding / 4.0)
                .py((metrics.track_padding + metrics.cell_trim / 2.0) / 4.0)
                .rounded_px(f32::from(metrics.track_radius))
                .bg(fill)
        });
        sheet.rule_when(TabsPart::List, one_row.size(size), move |t, _| {
            t.h_px(metrics.height).gap_x(metrics.column_gap / 4.0)
        });
        sheet.rule_when(TabsPart::List, wrapping.size(size), move |t, _| {
            t.flex_wrap().h_auto().gap(metrics.row_gap / 4.0)
        });
        for (chosen, paint, role) in [
            (true, segmented.selected, selected),
            (false, segmented.unselected, unselected),
        ] {
            for enablement in [Enablement::Enabled, Enablement::Disabled] {
                let when = When::always().size(size);
                let when = match chosen {
                    true => when.aria(Aria::Selected),
                    false => when.not_aria(Aria::Selected),
                };
                let when = match enablement {
                    Enablement::Enabled => when.not_aria(Aria::Disabled),
                    Enablement::Disabled => when.aria(Aria::Disabled),
                };
                let role = role.clone();
                sheet.rule_when(TabsPart::Trigger, when, move |t, _| {
                    let look = paint.classes_in(
                        width,
                        (focus, disabled),
                        metrics.cell_radius,
                        enablement,
                        transition,
                    );
                    centered(t.merge(look))
                        .grow()
                        .text_center()
                        .px(metrics.cell_padding.x / 4.0)
                        .py(metrics.cell_padding.y / 4.0)
                        .gap(metrics.icon_gap / 4.0)
                        .role(role.clone())
                        .descendants(Match::glyph(), |g| g.size_px(metrics.icon_size).shrink_0())
                });
            }
        }
    }
}
