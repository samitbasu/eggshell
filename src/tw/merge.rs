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

use super::{BoxPaint, Opacity, Sides, TextWidths, Tw};

/// A layout value as the Computed-style table writes it.
#[cfg(debug_assertions)]
trait Show {
    fn show(&self) -> String;
}

#[cfg(debug_assertions)]
macro_rules! show_debug {
    ($($ty:ty),+ $(,)?) => {$(
        impl Show for $ty {
            fn show(&self) -> String {
                format!("{self:?}")
            }
        }
    )+};
}

#[cfg(debug_assertions)]
show_debug!(
    Display,
    BoxSizing,
    Overflow,
    Position,
    AlignItems,
    Option<AlignItems>,
    AlignContent,
    FlexDirection,
    FlexWrap,
    GridAutoFlow,
    Option<f32>,
    f32,
    Vec<GridTemplateComponent<String>>,
    Line<GridPlacement<String>>,
);

#[cfg(debug_assertions)]
fn length_text(raw: taffy::style::CompactLength) -> String {
    use taffy::style::CompactLength;
    match raw.tag() {
        CompactLength::LENGTH_TAG => format!("{}px", raw.value()),
        CompactLength::PERCENT_TAG => format!("{}%", raw.value() * 100.0),
        _ if raw.is_auto() => "auto".to_owned(),
        _ => format!("{raw:?}"),
    }
}

#[cfg(debug_assertions)]
macro_rules! show_length {
    ($($ty:ty),+ $(,)?) => {$(
        impl Show for $ty {
            fn show(&self) -> String {
                length_text(self.into_raw())
            }
        }
    )+};
}

#[cfg(debug_assertions)]
show_length!(Dimension, LengthPercentage, LengthPercentageAuto);

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

    fn clear(&mut self, property: Property) {
        self.0 &= !(1 << property as u32);
    }
}

/// How a class gives the lengths it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Given {
    /// In points, a fraction, or a keyword.
    Points,
    /// In spacing units, which become points with the theme's spacing.
    Units,
}

/// Declares the layout properties: the bit each has in [`Touched`], the
/// accessor of [`Layout`] that marks it, and its copy in a merge. This list is
/// the one place that names them.
macro_rules! layout_properties {
    ($($name:ident: $($field:ident).+ => $ty:ty),+ $(,)?) => {
        /// TODO(prose): a property of a class list, named for the style field it sets.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        #[expect(non_camel_case_types, reason = "named for the style field they stand for")]
        #[cfg_attr(debug_assertions, expect(missing_docs, reason = "named for the style field they stand for"))]
        #[cfg_attr(not(debug_assertions), allow(dead_code))]
        pub enum Property {
            $($name,)+
            RadiusNw,
            RadiusNe,
            RadiusSw,
            RadiusSe,
            Opacity,
            // The properties below are not merged through `Touched`: their
            // classes are `Option`s, set or not. They are named so that a
            // winner can be too.
            Background,
            BorderColorTop,
            BorderColorRight,
            BorderColorBottom,
            BorderColorLeft,
            Ring,
            Shadow,
            DivideX,
            DivideY,
            TextSize,
            TextLineHeight,
            TextFamily,
            TextWeight,
            TextColor,
            TextFontStyle,
            TextDecoration,
            TextWhiteSpace,
            TextAlign,
            TextTracking,
            TextCase,
            TextSelect,
            TextNumerals,
            VariantGroupHover,
            VariantHover,
            VariantFocusWithin,
            VariantFocus,
            VariantHighlighted,
            VariantActive,
            VariantDragAccepts,
            VariantDragOver,
            VariantDragLifted,
        }

        impl Property {
            /// The value layout `style` has for the property, written out.
            #[cfg(debug_assertions)]
            fn layout_value(self, style: &Style) -> Option<String> {
                match self {
                    $(Property::$name => Some(Show::show(&style.$($field).+)),)+
                    _ => None,
                }
            }

            /// Every property that [`Touched`] tracks.
            #[cfg(debug_assertions)]
            const TOUCHED: &[Property] = &[
                $(Property::$name,)+
                Property::RadiusNw,
                Property::RadiusNe,
                Property::RadiusSw,
                Property::RadiusSe,
                Property::Opacity,
            ];
        }

        #[expect(clippy::disallowed_macros, reason = "a const assertion fails the build, not a run")]
        const _: () = assert!((Property::Opacity as u32) < 64);

        impl Layout<'_> {
            $(
                /// The property, marked as set by the class being written.
                pub(super) fn $name(&mut self) -> &mut $ty {
                    self.touched.mark(Property::$name);
                    self.ch.supersede(Property::$name);
                    match self.given {
                        Given::Points => self.spaced.clear(Property::$name),
                        Given::Units => self.spaced.mark(Property::$name),
                    }
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
                    Layout {
                        style: &mut style,
                        ch: &mut TextWidths::default(),
                        touched: &mut touched,
                        spaced: &mut Touched::default(),
                        given: Given::Points,
                    }.$name();
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

/// A length a class may give in spacing units.
trait Spaceable: Copy {
    fn scaled(self, points_per_unit: f32) -> Self;
}

macro_rules! spaceable_types {
    ($($ty:ty),+) => {$(
        impl Spaceable for $ty {
            fn scaled(self, points_per_unit: f32) -> Self {
                let raw = self.into_raw();
                match raw.tag() == taffy::CompactLength::LENGTH_TAG {
                    true => <$ty>::length(raw.value() * points_per_unit),
                    false => self,
                }
            }
        }
    )+};
}

spaceable_types!(Dimension, LengthPercentage, LengthPercentageAuto);

/// The properties a class may give in spacing units: each takes a length.
macro_rules! spaceable_properties {
    ($($name:ident: $($field:ident).+),+ $(,)?) => {
        /// `style` with the lengths `spaced` marks turned from spacing units
        /// into points.
        pub(super) fn in_points(style: &mut Style, spaced: Touched, points_per_unit: f32) {
            $(
                if spaced.has(Property::$name) {
                    style.$($field).+ = style.$($field).+.scaled(points_per_unit);
                }
            )+
        }
    };
}

spaceable_properties! {
    inset_left: inset.left,
    inset_right: inset.right,
    inset_top: inset.top,
    inset_bottom: inset.bottom,
    width: size.width,
    height: size.height,
    min_width: min_size.width,
    min_height: min_size.height,
    max_width: max_size.width,
    max_height: max_size.height,
    margin_left: margin.left,
    margin_right: margin.right,
    margin_top: margin.top,
    margin_bottom: margin.bottom,
    padding_left: padding.left,
    padding_right: padding.right,
    padding_top: padding.top,
    padding_bottom: padding.bottom,
    gap_width: gap.width,
    gap_height: gap.height,
    flex_basis: flex_basis,
}

/// The layout a class writes. Every property it writes is reached through
/// the accessor named for it, which records the write for [`Tw::merge`].
pub(super) struct Layout<'a> {
    style: &'a mut Style,
    pub(super) ch: &'a mut TextWidths,
    touched: &'a mut Touched,
    spaced: &'a mut Touched,
    given: Given,
}

impl TextWidths {
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

    pub(super) fn radius_nw(&mut self) -> &mut u8 {
        self.touched.mark(Property::RadiusNw);
        &mut self.paint.radius.nw
    }

    pub(super) fn radius_sw(&mut self) -> &mut u8 {
        self.touched.mark(Property::RadiusSw);
        &mut self.paint.radius.sw
    }

    pub(super) fn set_opacity(&mut self, opacity: Opacity) {
        self.touched.mark(Property::Opacity);
        self.paint.opacity = opacity;
    }
}

#[cfg(debug_assertions)]
impl Tw {
    /// The value the classes have for `property`, written out.
    pub(super) fn value_of(&self, property: Property) -> String {
        if let Some(value) = property.layout_value(&self.style) {
            return match self.spaced.has(property) {
                true => value.replace("px", " units"),
                false => value,
            };
        }
        let paint = &self.paint;
        let text = &self.text;
        match property {
            Property::RadiusNw => format!("{}", paint.radius.nw),
            Property::RadiusNe => format!("{}", paint.radius.ne),
            Property::RadiusSw => format!("{}", paint.radius.sw),
            Property::RadiusSe => format!("{}", paint.radius.se),
            Property::Opacity => format!("{:?}", paint.opacity),
            Property::Background => format!("{:?}", paint.background),
            Property::BorderColorTop => format!("{:?}", paint.border_colors.top),
            Property::BorderColorRight => format!("{:?}", paint.border_colors.right),
            Property::BorderColorBottom => format!("{:?}", paint.border_colors.bottom),
            Property::BorderColorLeft => format!("{:?}", paint.border_colors.left),
            Property::Ring => format!("{:?} {:?}", paint.ring, paint.ring_placement),
            Property::Shadow => format!("{:?}", paint.shadow),
            Property::DivideX => format!("{:?}", paint.divide_x),
            Property::DivideY => format!("{:?}", paint.divide_y),
            Property::TextSize => format!("{:?}", text.size),
            Property::TextLineHeight => format!("{:?}", text.line_height),
            Property::TextFamily => format!("{:?}", text.family),
            Property::TextWeight => format!("{:?}", text.weight),
            Property::TextColor => format!("{:?}", text.color),
            Property::TextFontStyle => format!("{:?}", text.font_style),
            Property::TextDecoration => format!("{:?}", text.decoration),
            Property::TextWhiteSpace => format!("{:?}", text.white_space),
            Property::TextAlign => format!("{:?}", text.align),
            Property::TextTracking => format!("{:?}", text.tracking),
            Property::TextCase => format!("{:?}", text.case),
            Property::TextSelect => format!("{:?}", text.select),
            Property::TextNumerals => format!("{:?}", text.numerals),
            Property::VariantGroupHover => self.variant_value(super::State::GroupHover),
            Property::VariantHover => self.variant_value(super::State::Hover),
            Property::VariantFocusWithin => self.variant_value(super::State::FocusWithin),
            Property::VariantFocus => self.variant_value(super::State::Focus),
            Property::VariantHighlighted => self.variant_value(super::State::Highlighted),
            Property::VariantActive => self.variant_value(super::State::Active),
            Property::VariantDragAccepts => self.variant_value(super::State::DragAccepts),
            Property::VariantDragOver => self.variant_value(super::State::DragOver),
            Property::VariantDragLifted => self.variant_value(super::State::DragLifted),
            _ => String::new(),
        }
    }

    fn variant_value(&self, state: super::State) -> String {
        format!("{:?}", self.variants.get(state))
    }
}

impl Tw {
    /// Writes layout with `set`, which records each property it writes.
    #[cfg_attr(debug_assertions, track_caller)]
    pub(super) fn write_layout(&mut self, given: Given, set: impl FnOnce(&mut Layout<'_>)) {
        let (style, ch, spaced) = (self.style.clone(), self.ch.clone(), self.spaced);
        let mut written = Touched::default();
        set(&mut Layout {
            style: &mut self.style,
            ch: &mut self.ch,
            touched: &mut written,
            spaced: &mut self.spaced,
            given,
        });
        if super::layout::is_finite(&self.style, &self.ch) {
            self.touched = self.touched | written;
            #[cfg(debug_assertions)]
            self.note_all(
                Property::TOUCHED
                    .iter()
                    .copied()
                    .filter(|&p| written.has(p)),
            );
        } else {
            (self.style, self.ch, self.spaced) = (style, ch, spaced);
            self.lengths = super::Lengths::Rejected;
        }
    }

    /// Writes the box paint that merging tracks with `set`.
    #[cfg_attr(debug_assertions, track_caller)]
    pub(super) fn write_shape(&mut self, set: impl FnOnce(&mut Shape<'_>)) {
        let mut written = Touched::default();
        set(&mut Shape {
            paint: &mut self.paint,
            touched: &mut written,
        });
        self.touched = self.touched | written;
        #[cfg(debug_assertions)]
        self.note_all(
            Property::TOUCHED
                .iter()
                .copied()
                .filter(|&p| written.has(p)),
        );
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
                Property::max_width,
                &mut self.ch.max_width,
                other.ch.max_width,
            ),
        ] {
            if other.touched.has(property) {
                *mine = theirs;
            }
        }
        if other.touched.has(Property::min_width) {
            self.ch.min_width = other.ch.min_width.clone();
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
        self.control_size = other.control_size.or(self.control_size);
        self.pointer_events = self.pointer_events.merge(other.pointer_events);
        self.visibility = other.visibility.or(self.visibility);
        self.sensing = self.sensing.or(other.sensing);
        self.focusability = self.focusability.merge(other.focusability);
        self.scrollbar = self.scrollbar.merge(other.scrollbar);
        let (mine, theirs) = (self.scroll_padding, other.scroll_padding);
        let later = |before: f32, after: f32| if after == 0.0 { before } else { after };
        self.scroll_padding = Sides {
            top: later(mine.top, theirs.top),
            right: later(mine.right, theirs.right),
            bottom: later(mine.bottom, theirs.bottom),
            left: later(mine.left, theirs.left),
        };
        self.grouping = self.grouping.merge(other.grouping);
        self.variants = self.variants.merge(other.variants);
        self.spaced = Touched(self.spaced.0 & !other.touched.0 | other.spaced.0);
        self.touched = self.touched | other.touched;
        self.lengths = self.lengths.or(other.lengths);
        #[cfg(debug_assertions)]
        {
            self.origins = match (self.origins, other.origins) {
                (Some(own), Some(later)) => Some(Box::new(own.over(*later))),
                (own, later) => later.or(own),
            };
        }
        self.rules = match (self.rules, other.rules) {
            (Some(own), Some(later)) => Some(Box::new(own.merge(*later))),
            (own, later) => later.or(own),
        };
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
        assert_eq!(kept.variants.transition(), Some(Duration::from_millis(100)));
        assert_eq!(kept.variants.easing(), crate::tw::Easing::In);
        assert_eq!(kept.z, Some(2));
        let replaced = base.merge(tw().transition(Duration::from_millis(30)).ease_out().z(0));
        assert_eq!(
            replaced.variants.transition(),
            Some(Duration::from_millis(30))
        );
        assert_eq!(replaced.variants.easing(), crate::tw::Easing::Out);
        assert_eq!(replaced.z, Some(0), "z(0) is still a z");
    }

    #[test]
    fn the_later_visibility_class_wins_and_none_keeps_the_earlier() {
        use crate::tw::Visibility;
        assert_eq!(
            tw().invisible().merge(tw().visible()).visibility,
            Some(Visibility::Visible)
        );
        assert_eq!(
            tw().visible().merge(tw().invisible()).visibility,
            Some(Visibility::Hidden)
        );
        assert_eq!(
            tw().invisible().merge(tw()).visibility,
            Some(Visibility::Hidden)
        );
        assert_eq!(tw().merge(tw()).visibility, None);
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

#[cfg(test)]
mod spacing {
    use crate::contract::Spacing;
    use crate::tw;
    use taffy::style::{CompactLength, Dimension, LengthPercentage};

    fn left_padding(classes: &super::Tw, points_per_unit: f32) -> f32 {
        classes
            .style_at(Spacing::new(points_per_unit))
            .padding
            .left
            .into_raw()
            .value()
    }

    #[test]
    fn a_class_in_units_holds_the_units_until_resolved() {
        let classes = tw().p(4.0);
        assert_eq!(classes.style().padding.left, LengthPercentage::length(4.0));
        assert_eq!(left_padding(&classes, 3.5), 14.0);
        assert_eq!(left_padding(&classes, 4.0), 16.0);
    }

    #[test]
    fn a_class_in_points_never_scales() {
        assert_eq!(left_padding(&tw().p_px(4.0), 3.5), 4.0);
        assert_eq!(left_padding(&tw().w_full().p_px(4.0), 3.5), 4.0);
    }

    #[test]
    fn the_later_class_decides_whether_a_property_scales() {
        assert_eq!(left_padding(&tw().p(4.0).p_px(3.0), 3.5), 3.0);
        assert_eq!(left_padding(&tw().p_px(3.0).p(4.0), 3.5), 14.0);
        assert_eq!(left_padding(&tw().p(4.0).merge(tw().p_px(3.0)), 3.5), 3.0);
        assert_eq!(left_padding(&tw().p_px(3.0).merge(tw().p(4.0)), 3.5), 14.0);
        assert_eq!(
            left_padding(&tw().p(4.0).merge(tw().pt(1.0)), 3.5),
            14.0,
            "a merge that leaves a side alone keeps its units"
        );
        assert_eq!(
            left_padding(&tw().p(4.0).merge(tw().gap_px(1.0)), 3.5),
            14.0
        );
    }

    #[test]
    fn a_fraction_or_keyword_over_units_does_not_scale() {
        let width = |classes: super::Tw| classes.style_at(Spacing::new(3.5)).size.width;
        assert_eq!(width(tw().w(4.0).w_full()), Dimension::percent(1.0));
        assert_eq!(width(tw().w(4.0).w_auto()), Dimension::auto());
        assert_eq!(width(tw().w_full().w(4.0)), Dimension::length(14.0));
    }

    #[test]
    fn a_rejected_class_keeps_the_units_it_replaced() {
        let classes = tw().p(4.0).p_px(f32::NAN);
        assert_eq!(left_padding(&classes, 3.5), 14.0);
    }

    #[test]
    fn every_spacing_method_scales_what_it_writes() {
        let style = tw()
            .m(1.0)
            .gap(2.0)
            .inset(3.0)
            .min_w(4.0)
            .max_h(5.0)
            .basis(6.0)
            .style_at(Spacing::new(0.5));
        let raw = |length: CompactLength| length.value();
        assert_eq!(raw(style.margin.top.into_raw()), 0.5);
        assert_eq!(raw(style.gap.width.into_raw()), 1.0);
        assert_eq!(raw(style.inset.bottom.into_raw()), 1.5);
        assert_eq!(raw(style.min_size.width.into_raw()), 2.0);
        assert_eq!(raw(style.max_size.height.into_raw()), 2.5);
        assert_eq!(raw(style.flex_basis.into_raw()), 3.0);
    }

    #[test]
    fn scroll_padding_is_in_units() {
        let padding = tw().scroll_py(2.0).scroll_padding_points(Spacing::new(3.5));
        assert_eq!((padding.top, padding.bottom, padding.left), (7.0, 7.0, 0.0));
    }
}
