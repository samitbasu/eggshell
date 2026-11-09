//! Combining class lists, the later one winning: `tailwind-merge`'s job.
//!
//! A class list does not remember which classes were written, only what they
//! resolved to, so the properties a list set are recorded as it is built. A
//! class that sets a property to its default, such as `flex-nowrap` over
//! `flex-wrap`, still counts as set.

use egui::CornerRadius;
use taffy::style::{
    AlignContent, AlignItems, BoxSizing, Dimension, Display, FlexDirection, FlexWrap, GridAutoFlow,
    LengthPercentage, LengthPercentageAuto, Overflow, Position,
};
use taffy::style_helpers::TaffyGridLine as _;
use taffy::{Line, Rect, Size, Style};

use super::{BoxPaint, Opacity, Tw};

/// The properties a class list has set: a bit for each layout property that
/// can be merged on its own, then the corner radii and the opacity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Touched(u64);

impl std::ops::BitOr for Touched {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl Touched {
    fn has(self, bit: u32) -> bool {
        self.0 >> bit & 1 == 1
    }
}

macro_rules! layout_properties {
    ($($($field:ident).+),+ $(,)?) => {
        const LAYOUT_PROPERTIES: u32 = [$(stringify!($($field).+)),+].len() as u32;

        impl Touched {
            /// The layout properties that differ between `before` and `after`.
            fn between(before: &Style, after: &Style) -> Self {
                let mut touched = 0u64;
                let mut bit = 0;
                $(
                    if before.$($field).+ != after.$($field).+ {
                        touched |= 1 << bit;
                    }
                    bit += 1;
                )+
                let _ = bit;
                Self(touched)
            }
        }

        /// `into` with the properties `touched` marks taken from `from`.
        fn copy_layout(into: &mut Style, from: &Style, touched: Touched) {
            let mut bit = 0;
            $(
                if touched.has(bit) {
                    into.$($field).+ = from.$($field).+.clone();
                }
                bit += 1;
            )+
            let _ = bit;
        }
    };
}

layout_properties! {
    display, box_sizing, overflow.x, overflow.y, position,
    inset.left, inset.right, inset.top, inset.bottom,
    size.width, size.height, min_size.width, min_size.height,
    max_size.width, max_size.height, aspect_ratio,
    margin.left, margin.right, margin.top, margin.bottom,
    padding.left, padding.right, padding.top, padding.bottom,
    border.left, border.right, border.top, border.bottom,
    align_items, align_self, justify_items, justify_self, align_content, justify_content,
    gap.width, gap.height,
    flex_direction, flex_wrap, flex_basis, flex_grow, flex_shrink,
    grid_template_rows, grid_template_columns, grid_auto_flow, grid_row, grid_column,
}

const RADIUS_NW: u32 = LAYOUT_PROPERTIES;
const RADIUS_NE: u32 = LAYOUT_PROPERTIES + 1;
const RADIUS_SW: u32 = LAYOUT_PROPERTIES + 2;
const RADIUS_SE: u32 = LAYOUT_PROPERTIES + 3;
const OPACITY: u32 = LAYOUT_PROPERTIES + 4;
const _: () = assert!(OPACITY < 64);

macro_rules! pick {
    ($first:expr, $a:expr, $b:expr) => {
        if $first { $a } else { $b }
    };
}

// Two layouts that differ in every property a class can set, so that
// whatever a class writes differs from at least one of them.
thread_local! {
    static LAYOUT_PROBES: ([Style; 2], std::cell::RefCell<[Style; 2]>) =
        (layout_probes(), std::cell::RefCell::new(layout_probes()));
}

fn layout_probes() -> [Style; 2] {
    let probe = |n: f32, first: bool| {
        let length = |n: f32| LengthPercentage::length(n);
        let auto_length = |n: f32| LengthPercentageAuto::length(n);
        let sides = |n: f32| Rect {
            left: length(n),
            right: length(n + 0.5),
            top: length(n + 0.25),
            bottom: length(n + 0.75),
        };
        let auto_sides = |n: f32| Rect {
            left: auto_length(n),
            right: auto_length(n + 0.5),
            top: auto_length(n + 0.25),
            bottom: auto_length(n + 0.75),
        };
        let line = |n: i16| Line {
            start: taffy::GridPlacement::from_line_index(n),
            end: taffy::GridPlacement::from_line_index(n + 1),
        };
        Style {
            display: pick!(first, Display::Flex, Display::Grid),
            box_sizing: pick!(first, BoxSizing::BorderBox, BoxSizing::ContentBox),
            overflow: taffy::Point {
                x: pick!(first, Overflow::Visible, Overflow::Hidden),
                y: pick!(first, Overflow::Hidden, Overflow::Visible),
            },
            position: pick!(first, Position::Relative, Position::Absolute),
            inset: auto_sides(n),
            size: Size {
                width: Dimension::length(-n),
                height: Dimension::length(-n - 0.5),
            },
            min_size: Size {
                width: auto_length(-n),
                height: auto_length(-n - 0.5),
            },
            max_size: Size {
                width: auto_length(-n - 1.0),
                height: auto_length(-n - 1.5),
            },
            aspect_ratio: pick!(first, Some(n), None),
            margin: auto_sides(n),
            padding: sides(n),
            border: sides(n),
            align_items: pick!(first, AlignItems::BASELINE, AlignItems::CENTER),
            align_self: pick!(first, Some(AlignItems::BASELINE), None),
            justify_items: pick!(first, AlignItems::BASELINE, AlignItems::CENTER),
            justify_self: pick!(first, Some(AlignItems::BASELINE), None),
            align_content: pick!(first, AlignContent::CENTER, AlignContent::START),
            justify_content: pick!(first, AlignContent::CENTER, AlignContent::START),
            gap: Size {
                width: length(n),
                height: length(n + 0.5),
            },
            flex_direction: pick!(first, FlexDirection::Row, FlexDirection::Column),
            flex_wrap: pick!(first, FlexWrap::Wrap, FlexWrap::NoWrap),
            flex_basis: Dimension::length(n),
            flex_grow: n,
            flex_shrink: n + 0.5,
            grid_auto_flow: pick!(first, GridAutoFlow::Row, GridAutoFlow::Column),
            grid_row: line(pick!(first, 3, -3)),
            grid_column: line(pick!(first, 4, -4)),
            ..Style::default()
        }
    };
    [probe(7.0, true), probe(11.0, false)]
}

fn paint_probes() -> [BoxPaint; 2] {
    let probe = |radius: u8, opacity: f32| BoxPaint {
        radius: CornerRadius::same(radius),
        opacity: Opacity::new(opacity),
        ..BoxPaint::default()
    };
    [probe(77, 0.25), probe(78, 0.75)]
}

impl Tw {
    /// Writes layout with `set`, noting which properties it wrote.
    pub(super) fn write_layout(&mut self, set: impl Fn(&mut Style)) {
        set(&mut self.style);
        let wrote = LAYOUT_PROBES.with(|(originals, work)| {
            let mut work = work.borrow_mut();
            let mut touched = Touched::default();
            for (original, probe) in originals.iter().zip(work.iter_mut()) {
                set(probe);
                let wrote = Touched::between(original, probe);
                copy_layout(probe, original, wrote);
                touched = touched | wrote;
            }
            touched
        });
        self.touched = self.touched | wrote;
    }

    /// Writes box paint with `set`, noting the radii and opacity it wrote.
    pub(super) fn write_paint(&mut self, set: impl Fn(&mut BoxPaint)) {
        set(&mut self.paint);
        let probes = paint_probes();
        let [mut a, mut b] = probes;
        set(&mut a);
        set(&mut b);
        let wrote = |before: &BoxPaint, after: &BoxPaint| {
            let (r, s) = (before.radius, after.radius);
            let mut touched = 0u64;
            for (bit, changed) in [
                (RADIUS_NW, r.nw != s.nw),
                (RADIUS_NE, r.ne != s.ne),
                (RADIUS_SW, r.sw != s.sw),
                (RADIUS_SE, r.se != s.se),
                (OPACITY, before.opacity != after.opacity),
            ] {
                touched |= u64::from(changed) << bit;
            }
            Touched(touched)
        };
        self.touched = self.touched | wrote(&probes[0], &a) | wrote(&probes[1], &b);
    }

    /// These classes with `other` applied on top, as `tailwind-merge` does:
    /// for each property `other` sets, its value replaces this list's, and
    /// the properties it leaves alone keep theirs.
    ///
    /// | What | Merged |
    /// |---|---|
    /// | Layout | per property: `p-2` over `px-4` leaves the top and bottom padding; each side, each axis of a size, a gap or an overflow is its own property |
    /// | Box paint | per property: the background, each border side's colour, ring, shadow, each corner radius, opacity and each divider |
    /// | Text classes | per property, as in layout |
    /// | State variants | per state, then per property within it: `hover:bg-red` over `hover:text-white` keeps both |
    /// | `transition`, `ease-*` | `other`'s when it sets one |
    /// | `z` | `other`'s when it sets one |
    /// | `sense_*`, `group`, `focusable_none`, `inert` | combined: a node that either list makes sensing, a group, unfocusable or inert is, because no class takes them back |
    ///
    /// ```
    /// use bullseye::tw;
    /// let card = tw().flex().flex_wrap().p(4.0).gap(2.0);
    /// let placed = card.merge(tw().flex_nowrap().px(1.0));
    /// let expected = tw().flex().flex_nowrap().p(4.0).px(1.0).gap(2.0);
    /// assert_eq!(placed.style(), expected.style());
    /// ```
    #[must_use]
    pub fn merge(mut self, other: Self) -> Self {
        copy_layout(&mut self.style, &other.style, other.touched);
        let (mine, theirs) = (&mut self.paint, &other.paint);
        let corners = |own: u8, over: u8, bit: u32| if other.touched.has(bit) { over } else { own };
        mine.radius = CornerRadius {
            nw: corners(mine.radius.nw, theirs.radius.nw, RADIUS_NW),
            ne: corners(mine.radius.ne, theirs.radius.ne, RADIUS_NE),
            sw: corners(mine.radius.sw, theirs.radius.sw, RADIUS_SW),
            se: corners(mine.radius.se, theirs.radius.se, RADIUS_SE),
        };
        if other.touched.has(OPACITY) {
            mine.opacity = theirs.opacity;
        }
        mine.background = theirs.background.or(mine.background);
        let (top, right) = (mine.border_colors.top, mine.border_colors.right);
        let (bottom, left) = (mine.border_colors.bottom, mine.border_colors.left);
        mine.border_colors = super::Sides {
            top: theirs.border_colors.top.or(top),
            right: theirs.border_colors.right.or(right),
            bottom: theirs.border_colors.bottom.or(bottom),
            left: theirs.border_colors.left.or(left),
        };
        mine.ring = theirs.ring.or(mine.ring);
        mine.shadow = theirs.shadow.or(mine.shadow);
        mine.divide_x = theirs.divide_x.or(mine.divide_x);
        mine.divide_y = theirs.divide_y.or(mine.divide_y);
        self.text = self.text.merge(other.text);
        self.z = other.z.or(self.z);
        self.inert |= other.inert;
        self.sensing = self.sensing.or(other.sensing);
        self.focusability = self.focusability.merge(other.focusability);
        self.grouping = self.grouping.merge(other.grouping);
        self.variants = self.variants.merge(other.variants);
        self.touched = self.touched | other.touched;
        self
    }
}

#[cfg(test)]
mod tests {
    use egui::Color32;

    use super::*;
    use crate::tw::{Paint, tw};

    #[test]
    fn the_probes_differ_in_every_property() {
        LAYOUT_PROBES.with(|(probes, _)| {
            let differing = Touched::between(&probes[0], &probes[1]);
            let track_lists = 0b11 << (LAYOUT_PROPERTIES - 5);
            assert_eq!(
                differing.0,
                ((1 << LAYOUT_PROPERTIES) - 1) & !track_lists,
                "every property but the two track lists, which are empty in both probes \
                 so that cloning them allocates nothing"
            );
        });
        let [a, b] = paint_probes();
        assert!(a.radius.nw != b.radius.nw && a.opacity != b.opacity);
    }

    #[test]
    fn a_property_set_to_its_default_still_overrides() {
        let wrapped = tw().flex().flex_wrap().flex_row();
        let merged = wrapped.merge(tw().flex_nowrap().flex_row());
        assert_eq!(merged.style().flex_wrap, FlexWrap::NoWrap);
        let merged = tw().flex().merge(tw().block());
        assert_eq!(merged.style().display, Display::Block);
        let merged = tw()
            .opacity(Opacity::new(0.5))
            .merge(tw().opacity(Opacity::OPAQUE));
        assert_eq!(merged.paint.opacity, Opacity::OPAQUE);
        let merged = tw().rounded_lg().merge(tw().rounded_none());
        assert_eq!(merged.paint.radius, CornerRadius::ZERO);
    }

    #[test]
    fn a_property_the_later_list_leaves_alone_keeps_its_value() {
        let merged = tw().flex().gap(2.0).p(4.0).merge(tw().w_full());
        assert_eq!(
            *merged.style(),
            *tw().flex().gap(2.0).p(4.0).w_full().style()
        );
        let merged = tw().bg(Color32::RED).rounded_lg().merge(tw());
        assert_eq!(merged.paint.background, Some(Color32::RED));
        assert_eq!(merged.paint.radius, CornerRadius::same(8));
    }

    #[test]
    fn sides_and_axes_merge_one_at_a_time() {
        let merged = tw().p(4.0).gap(1.0).merge(tw().px(1.0).gap_x(3.0));
        assert_eq!(
            *merged.style(),
            *tw().p(4.0).gap(1.0).px(1.0).gap_x(3.0).style()
        );
        assert_ne!(merged.style().padding.top, merged.style().padding.left);
        let merged = tw().rounded_lg().merge(tw().rounded_tr_px(1.0));
        assert_eq!(
            (merged.paint.radius.ne, merged.paint.radius.nw),
            (1, 8),
            "one corner replaced, the others kept"
        );
        let merged = tw()
            .border_color(Color32::RED)
            .merge(tw().border_t_color(Color32::BLUE));
        let sides = merged.paint.border_colors;
        assert_eq!(
            (sides.top, sides.bottom),
            (Some(Color32::BLUE), Some(Color32::RED))
        );
    }

    #[test]
    fn merging_the_empty_list_changes_nothing() {
        let list = tw()
            .flex()
            .flex_col()
            .items_center()
            .size_px(10.0)
            .bg(Color32::RED)
            .text(Color32::BLUE)
            .z(3)
            .hover(|p| p.bg(Color32::GREEN))
            .transition(std::time::Duration::from_millis(80));
        let merged = list.clone().merge(tw());
        assert_eq!(*merged.style(), *list.style());
        assert_eq!(merged.paint, list.paint);
        assert_eq!(merged.text, list.text);
        assert_eq!(merged.z, list.z);
        assert_eq!(merged.variants, list.variants);
        assert_eq!(merged.sensing, list.sensing);
        let merged = tw().merge(list.clone());
        assert_eq!(*merged.style(), *list.style());
        assert_eq!(merged.variants, list.variants);
    }

    #[test]
    fn variants_merge_per_state_and_per_property() {
        let merged = tw()
            .hover(|p| p.bg(Color32::RED).text(Color32::WHITE))
            .active(|p| p.bg(Color32::BLUE))
            .merge(
                tw().hover(|p| p.bg(Color32::GREEN))
                    .focus(|p| p.underline()),
            );
        let hover = merged.variants.get(crate::tw::State::Hover).unwrap();
        assert_eq!(
            *hover,
            Paint::default().bg(Color32::GREEN).text(Color32::WHITE),
            "background replaced, text kept"
        );
        assert_eq!(
            *merged.variants.get(crate::tw::State::Active).unwrap(),
            Paint::default().bg(Color32::BLUE)
        );
        assert!(merged.variants.get(crate::tw::State::Focus).is_some());
    }

    #[test]
    fn transition_easing_and_z_take_the_later_one_when_set() {
        use std::time::Duration;
        let base = tw().transition(Duration::from_millis(100)).ease_in().z(2);
        let kept = base.clone().merge(tw());
        assert_eq!(kept.variants.transition, Some(Duration::from_millis(100)));
        assert_eq!(kept.variants.easing, crate::tw::Easing::In);
        assert_eq!(kept.z, Some(2));
        let replaced = base.merge(tw().transition(Duration::from_millis(30)).ease_out().z(0));
        assert_eq!(
            replaced.variants.transition,
            Some(Duration::from_millis(30))
        );
        assert_eq!(replaced.variants.easing, crate::tw::Easing::Out);
        assert_eq!(replaced.z, Some(0), "z(0) is still a z");
    }

    #[test]
    fn behaviour_combines_because_no_class_takes_it_back() {
        let merged = tw().sense_click().merge(tw().sense_drag().inert().group());
        assert_eq!(merged.sensing, crate::tw::Sensing::ClickDrag);
        assert!(merged.inert);
        assert_eq!(merged.grouping, crate::tw::Grouping::Group);
        let merged = tw().focusable_none().merge(tw());
        assert_eq!(merged.focusability, crate::tw::Focusability::Skip);
    }

    #[test]
    fn text_classes_merge_per_property() {
        let merged = tw()
            .text(Color32::RED)
            .italic()
            .merge(tw().text(Color32::BLUE).text_center());
        assert_eq!(merged.text.color, Some(Color32::BLUE));
        assert!(merged.text.font_style.is_some());
        assert!(merged.text.align.is_some());
    }
}
