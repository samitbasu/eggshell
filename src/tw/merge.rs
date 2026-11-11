//! Combining class lists, the later one winning: `tailwind-merge`'s job.
//!
//! A class list does not remember which classes were written, only what they
//! resolved to, so the properties a list set are recorded as it is built:
//! a class writes layout through [`Layout`] and the merged box paint through
//! [`Shape`], whose accessors mark what they hand out. A class that sets a
//! property to its default, such as `flex-nowrap` over `flex-wrap`, still
//! counts as set.

use egui::CornerRadius;
use taffy::style::{
    AlignContent, AlignItems, BoxSizing, Dimension, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTemplateComponent, LengthPercentage, LengthPercentageAuto, Overflow,
    Position,
};
use taffy::{Line, Rect, Size, Style};

use super::{BoxPaint, ChWidths, Opacity, Tw};

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
    fn has(self, property: Property) -> bool {
        self.0 >> property as u32 & 1 == 1
    }

    fn mark(&mut self, property: Property) {
        self.0 |= 1 << property as u32;
    }
}

/// Declares the layout properties: the bit each has in [`Touched`], the
/// accessor of [`Layout`] that marks it, and its copy in a merge. This list is
/// the one place that names them.
macro_rules! layout_properties {
    ($($name:ident: $($field:ident).+ => $ty:ty),+ $(,)?) => {
        #[derive(Clone, Copy)]
        #[expect(non_camel_case_types, reason = "named for the style field they stand for")]
        enum Property {
            $($name,)+
            RadiusNw,
            RadiusNe,
            RadiusSw,
            RadiusSe,
            Opacity,
        }

        const _: () = assert!((Property::Opacity as u32) < 64);

        impl Layout<'_> {
            $(
                /// The property, marked as set by the class being written.
                pub(super) fn $name(&mut self) -> &mut $ty {
                    self.touched.mark(Property::$name);
                    self.ch.supersede(Property::$name);
                    &mut self.style.$($field).+
                }
            )+
        }

        #[cfg(test)]
        mod properties {
            use super::*;

            #[test]
            fn each_accessor_marks_its_own_property_only() {
                $(
                    let mut style = Style::default();
                    let mut touched = Touched::default();
                    Layout { style: &mut style, ch: &mut ChWidths::default(), touched: &mut touched }.$name();
                    assert_eq!(
                        touched.0,
                        1 << Property::$name as u32,
                        stringify!($name)
                    );
                )+
            }
        }

        /// `into` with the properties `touched` marks taken from `from`.
        fn copy_layout(into: &mut Style, from: &Style, touched: Touched) {
            $(
                if touched.has(Property::$name) {
                    into.$($field).+ = from.$($field).+.clone();
                }
            )+
        }
    };
}

layout_properties! {
    display: display => Display,
    box_sizing: box_sizing => BoxSizing,
    overflow_x: overflow.x => Overflow,
    overflow_y: overflow.y => Overflow,
    position: position => Position,
    inset_left: inset.left => LengthPercentageAuto,
    inset_right: inset.right => LengthPercentageAuto,
    inset_top: inset.top => LengthPercentageAuto,
    inset_bottom: inset.bottom => LengthPercentageAuto,
    width: size.width => Dimension,
    height: size.height => Dimension,
    min_width: min_size.width => LengthPercentageAuto,
    min_height: min_size.height => LengthPercentageAuto,
    max_width: max_size.width => LengthPercentageAuto,
    max_height: max_size.height => LengthPercentageAuto,
    aspect_ratio: aspect_ratio => Option<f32>,
    margin_left: margin.left => LengthPercentageAuto,
    margin_right: margin.right => LengthPercentageAuto,
    margin_top: margin.top => LengthPercentageAuto,
    margin_bottom: margin.bottom => LengthPercentageAuto,
    padding_left: padding.left => LengthPercentage,
    padding_right: padding.right => LengthPercentage,
    padding_top: padding.top => LengthPercentage,
    padding_bottom: padding.bottom => LengthPercentage,
    border_left: border.left => LengthPercentage,
    border_right: border.right => LengthPercentage,
    border_top: border.top => LengthPercentage,
    border_bottom: border.bottom => LengthPercentage,
    align_items: align_items => AlignItems,
    align_self: align_self => Option<AlignItems>,
    justify_items: justify_items => AlignItems,
    justify_self: justify_self => Option<AlignItems>,
    align_content: align_content => AlignContent,
    justify_content: justify_content => AlignContent,
    gap_width: gap.width => LengthPercentage,
    gap_height: gap.height => LengthPercentage,
    flex_direction: flex_direction => FlexDirection,
    flex_wrap: flex_wrap => FlexWrap,
    flex_basis: flex_basis => Dimension,
    flex_grow: flex_grow => f32,
    flex_shrink: flex_shrink => f32,
    grid_template_rows: grid_template_rows => Vec<GridTemplateComponent<String>>,
    grid_template_columns: grid_template_columns => Vec<GridTemplateComponent<String>>,
    grid_auto_flow: grid_auto_flow => GridAutoFlow,
    grid_row: grid_row => Line<GridPlacement<String>>,
    grid_column: grid_column => Line<GridPlacement<String>>,
}

/// The layout a class writes. Every property it writes is reached through
/// the accessor named for it, which records the write for [`Tw::merge`].
pub(super) struct Layout<'a> {
    style: &'a mut Style,
    pub(super) ch: &'a mut ChWidths,
    touched: &'a mut Touched,
}

impl ChWidths {
    /// A class that writes `property` in points or as a fraction replaces a
    /// width pending in `ch`.
    fn supersede(&mut self, property: Property) {
        match property {
            Property::width => self.width = None,
            Property::min_width => self.min_width = None,
            Property::max_width => self.max_width = None,
            _ => {}
        }
    }
}

impl Layout<'_> {
    pub(super) fn set_padding(&mut self, sides: Rect<LengthPercentage>) {
        *self.padding_left() = sides.left;
        *self.padding_right() = sides.right;
        *self.padding_top() = sides.top;
        *self.padding_bottom() = sides.bottom;
    }

    pub(super) fn set_margin(&mut self, sides: Rect<LengthPercentageAuto>) {
        *self.margin_left() = sides.left;
        *self.margin_right() = sides.right;
        *self.margin_top() = sides.top;
        *self.margin_bottom() = sides.bottom;
    }

    pub(super) fn set_border(&mut self, sides: Rect<LengthPercentage>) {
        *self.border_left() = sides.left;
        *self.border_right() = sides.right;
        *self.border_top() = sides.top;
        *self.border_bottom() = sides.bottom;
    }

    pub(super) fn set_inset(&mut self, sides: Rect<LengthPercentageAuto>) {
        *self.inset_left() = sides.left;
        *self.inset_right() = sides.right;
        *self.inset_top() = sides.top;
        *self.inset_bottom() = sides.bottom;
    }

    pub(super) fn set_size(&mut self, size: Size<Dimension>) {
        *self.width() = size.width;
        *self.height() = size.height;
    }
}

/// The box paint a class writes that merging tracks property by property:
/// the corner radii and the opacity.
pub(super) struct Shape<'a> {
    paint: &'a mut BoxPaint,
    touched: &'a mut Touched,
}

impl Shape<'_> {
    pub(super) fn set_radius(&mut self, radius: CornerRadius) {
        self.touched.mark(Property::RadiusNw);
        self.touched.mark(Property::RadiusNe);
        self.touched.mark(Property::RadiusSw);
        self.touched.mark(Property::RadiusSe);
        self.paint.radius = radius;
    }

    pub(super) fn radius_ne(&mut self) -> &mut u8 {
        self.touched.mark(Property::RadiusNe);
        &mut self.paint.radius.ne
    }

    pub(super) fn radius_se(&mut self) -> &mut u8 {
        self.touched.mark(Property::RadiusSe);
        &mut self.paint.radius.se
    }

    pub(super) fn set_opacity(&mut self, opacity: Opacity) {
        self.touched.mark(Property::Opacity);
        self.paint.opacity = opacity;
    }
}

impl Tw {
    /// Writes layout with `set`, which records each property it writes.
    pub(super) fn write_layout(&mut self, set: impl FnOnce(&mut Layout<'_>)) {
        set(&mut Layout {
            style: &mut self.style,
            ch: &mut self.ch,
            touched: &mut self.touched,
        });
    }

    /// Writes the box paint that merging tracks with `set`.
    pub(super) fn write_shape(&mut self, set: impl FnOnce(&mut Shape<'_>)) {
        set(&mut Shape {
            paint: &mut self.paint,
            touched: &mut self.touched,
        });
    }

    /// These classes with `other` applied on top, as `tailwind-merge` does:
    /// for each property `other` sets, its value replaces this list's, and
    /// the properties it leaves alone keep theirs.
    ///
    /// | What | Merged |
    /// |---|---|
    /// | Layout | per property: `p-2` over `px-4` leaves the top and bottom padding; each side, each axis of a size, a gap or an overflow is its own property; `w_ch` and `w_px` are the same property |
    /// | Box paint | per property: the background, each border side's colour, ring, shadow, each corner radius, opacity and each divider |
    /// | Text classes | per property, as in layout |
    /// | State variants | per state, then per property within it: `hover:bg-red` over `hover:text-white` keeps both |
    /// | `transition`, `animate_layout`, `ease-*` | `other`'s when it sets one |
    /// | `z` | `other`'s when it sets one |
    /// | `sense_*`, `group`, `focusable_none`, `inert`, `pointer_events_none` | combined: a node that either list makes sensing, a group, unfocusable, inert or pointer-transparent is, because no class takes them back |
    ///
    /// ```
    /// use eggshell_ui::tw;
    /// let card = tw().flex().flex_wrap().p(4.0).gap(2.0);
    /// let placed = card.merge(tw().flex_nowrap().px(1.0));
    /// let expected = tw().flex().flex_nowrap().p(4.0).px(1.0).gap(2.0);
    /// assert_eq!(placed.style(), expected.style());
    /// ```
    #[must_use]
    pub fn merge(mut self, other: Self) -> Self {
        copy_layout(&mut self.style, &other.style, other.touched);
        for (property, mine, theirs) in [
            (Property::width, &mut self.ch.width, other.ch.width),
            (
                Property::min_width,
                &mut self.ch.min_width,
                other.ch.min_width,
            ),
            (
                Property::max_width,
                &mut self.ch.max_width,
                other.ch.max_width,
            ),
        ] {
            if other.touched.has(property) {
                *mine = theirs;
            }
        }
        let (mine, theirs) = (&mut self.paint, &other.paint);
        let corners = |own: u8, over: u8, property| {
            if other.touched.has(property) {
                over
            } else {
                own
            }
        };
        mine.radius = CornerRadius {
            nw: corners(mine.radius.nw, theirs.radius.nw, Property::RadiusNw),
            ne: corners(mine.radius.ne, theirs.radius.ne, Property::RadiusNe),
            sw: corners(mine.radius.sw, theirs.radius.sw, Property::RadiusSw),
            se: corners(mine.radius.se, theirs.radius.se, Property::RadiusSe),
        };
        if other.touched.has(Property::Opacity) {
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
        mine.ring_placement = mine.ring_placement.merge(theirs.ring_placement);
        mine.shadow = theirs.shadow.or(mine.shadow);
        mine.divide_x = theirs.divide_x.or(mine.divide_x);
        mine.divide_y = theirs.divide_y.or(mine.divide_y);
        self.text = self.text.merge(other.text);
        self.z = other.z.or(self.z);
        self.inert |= other.inert;
        self.pointer_events = self.pointer_events.merge(other.pointer_events);
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
        let merged = tw().pointer_events_none().merge(tw());
        assert_eq!(merged.pointer_events, crate::tw::PointerEvents::None);
        let merged = tw().merge(tw().pointer_events_none());
        assert_eq!(merged.pointer_events, crate::tw::PointerEvents::None);
    }

    #[test]
    fn ring_inset_survives_a_later_ring_and_a_later_ring_wins_its_colour() {
        let merged = tw()
            .ring(1.0, Color32::RED)
            .ring_inset()
            .merge(tw().ring(2.0, Color32::BLUE));
        assert_eq!(
            merged.paint.ring_placement,
            crate::tw::RingPlacement::Inside
        );
        assert_eq!(
            merged.paint.ring.map(|ring| ring.color),
            Some(Color32::BLUE)
        );
        let merged = tw().ring(1.0, Color32::RED).merge(tw().ring_inset());
        assert_eq!(
            merged.paint.ring_placement,
            crate::tw::RingPlacement::Inside
        );
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
