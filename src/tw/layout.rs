//! Layout classes: everything that writes a [`taffy::Style`].

use taffy::style::{
    AlignContent, AlignItems, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
    LengthPercentage, LengthPercentageAuto, Overflow, Position,
};
use taffy::style_helpers::{FromLength, FromPercent, TaffyAuto, TaffyZero as _};

use super::{SPACE_UNIT, Tw};

/// A length as written in a class: `p-4`, `w-px`, `w-1/2`, `w-auto`.
#[derive(Clone, Copy)]
enum Len {
    Points(f32),
    Fraction(f32),
    Auto,
}

impl Len {
    fn space(units: f32) -> Self {
        Self::Points(units * SPACE_UNIT)
    }

    fn fraction(numerator: f32, denominator: f32) -> Self {
        Self::Fraction(numerator / denominator)
    }

    fn to<T: FromLength + FromPercent + TaffyAuto>(self) -> T {
        match self {
            Self::Points(points) => T::from_length(points),
            Self::Fraction(fraction) => T::from_percent(fraction),
            Self::Auto => T::AUTO,
        }
    }
}

fn space(units: f32) -> LengthPercentage {
    LengthPercentage::length(units * SPACE_UNIT)
}

/// Methods taking spacing units; `$v` is the length in points.
macro_rules! spacing {
    ($($name:ident, $doc:literal, |$s:ident, $v:ident| $set:expr;)*) => {$(
        #[doc = $doc]
        pub fn $name(self, units: f32) -> Self {
            self.styled(|$s| { let $v = units * SPACE_UNIT; $set })
        }
    )*};
}

macro_rules! auto_margins {
    ($($name:ident, $doc:literal, |$s:ident| $set:expr;)*) => {$(
        #[doc = $doc]
        pub fn $name(self) -> Self {
            self.styled(|$s| $set)
        }
    )*};
}

/// The five spellings of a length class: units, `_px`, `_frac`, `_full`, `_auto`.
macro_rules! lengths {
    ($($name:ident, $px:ident, $frac:ident, $full:ident, $auto:ident, $what:literal,
       |$s:ident, $l:ident| $set:expr;)*) => {$(
        #[doc = concat!("Sets ", $what, " in spacing units.")]
        pub fn $name(self, units: f32) -> Self {
            self.styled(|$s| { let $l = Len::space(units); $set })
        }

        #[doc = concat!("Sets ", $what, " in points.")]
        pub fn $px(self, points: f32) -> Self {
            self.styled(|$s| { let $l = Len::Points(points); $set })
        }

        #[doc = concat!("Sets ", $what, " to the fraction `numerator / denominator` of the parent.")]
        pub fn $frac(self, numerator: f32, denominator: f32) -> Self {
            self.styled(|$s| { let $l = Len::fraction(numerator, denominator); $set })
        }

        #[doc = concat!("Sets ", $what, " to 100% of the parent.")]
        pub fn $full(self) -> Self {
            self.styled(|$s| { let $l = Len::Fraction(1.0); $set })
        }

        #[doc = concat!("Sets ", $what, " to `auto`.")]
        pub fn $auto(self) -> Self {
            self.styled(|$s| { let $l = Len::Auto; $set })
        }
    )*};
}

macro_rules! keywords {
    ($($name:ident, $doc:literal, |$s:ident| $set:expr;)*) => {$(
        #[doc = $doc]
        pub fn $name(self) -> Self {
            self.styled(|$s| $set)
        }
    )*};
}

impl Tw {
    keywords! {
        flex, "`flex`: a flex container.", |s| s.display = Display::Flex;
        block, "`block`: a block container.", |s| s.display = Display::Block;
        grid, "`grid`: a grid container.", |s| s.display = Display::Grid;
        hidden, "`hidden`: takes no part in layout.", |s| s.display = Display::None;

        flex_row, "`flex-row`.", |s| s.flex_direction = FlexDirection::Row;
        flex_col, "`flex-col`.", |s| s.flex_direction = FlexDirection::Column;
        flex_row_reverse, "`flex-row-reverse`.", |s| s.flex_direction = FlexDirection::RowReverse;
        flex_col_reverse, "`flex-col-reverse`.", |s| s.flex_direction = FlexDirection::ColumnReverse;
        flex_wrap, "`flex-wrap`.", |s| s.flex_wrap = FlexWrap::Wrap;
        flex_wrap_reverse, "`flex-wrap-reverse`.", |s| s.flex_wrap = FlexWrap::WrapReverse;
        flex_nowrap, "`flex-nowrap`.", |s| s.flex_wrap = FlexWrap::NoWrap;

        flex_1, "`flex-1`: grow and shrink from a zero basis.", |s| {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = Dimension::percent(0.0);
        };
        flex_auto, "`flex-auto`: grow and shrink from the content size.", |s| {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = Dimension::AUTO;
        };
        flex_initial, "`flex-initial`: shrink but do not grow.", |s| {
            s.flex_grow = 0.0;
            s.flex_shrink = 1.0;
            s.flex_basis = Dimension::AUTO;
        };
        flex_none, "`flex-none`: neither grow nor shrink.", |s| {
            s.flex_grow = 0.0;
            s.flex_shrink = 0.0;
            s.flex_basis = Dimension::AUTO;
        };
        grow, "`grow`.", |s| s.flex_grow = 1.0;
        grow_0, "`grow-0`.", |s| s.flex_grow = 0.0;
        shrink, "`shrink`.", |s| s.flex_shrink = 1.0;
        shrink_0, "`shrink-0`.", |s| s.flex_shrink = 0.0;

        aspect_square, "`aspect-square`.", |s| s.aspect_ratio = Some(1.0);
        aspect_video, "`aspect-video`: 16 by 9.", |s| s.aspect_ratio = Some(16.0 / 9.0);
        aspect_auto, "`aspect-auto`: no preferred ratio.", |s| s.aspect_ratio = None;

        items_start, "`items-start`.", |s| s.align_items = AlignItems::FLEX_START;
        items_end, "`items-end`.", |s| s.align_items = AlignItems::FLEX_END;
        items_center, "`items-center`.", |s| s.align_items = AlignItems::CENTER;
        items_baseline, "`items-baseline`.", |s| s.align_items = AlignItems::BASELINE;
        items_stretch, "`items-stretch`.", |s| s.align_items = AlignItems::STRETCH;

        justify_start, "`justify-start`.", |s| s.justify_content = AlignContent::FLEX_START;
        justify_end, "`justify-end`.", |s| s.justify_content = AlignContent::FLEX_END;
        justify_center, "`justify-center`.", |s| s.justify_content = AlignContent::CENTER;
        justify_between, "`justify-between`.", |s| s.justify_content = AlignContent::SPACE_BETWEEN;
        justify_around, "`justify-around`.", |s| s.justify_content = AlignContent::SPACE_AROUND;
        justify_evenly, "`justify-evenly`.", |s| s.justify_content = AlignContent::SPACE_EVENLY;
        justify_stretch, "`justify-stretch`.", |s| s.justify_content = AlignContent::STRETCH;

        content_start, "`content-start`.", |s| s.align_content = AlignContent::FLEX_START;
        content_end, "`content-end`.", |s| s.align_content = AlignContent::FLEX_END;
        content_center, "`content-center`.", |s| s.align_content = AlignContent::CENTER;
        content_between, "`content-between`.", |s| s.align_content = AlignContent::SPACE_BETWEEN;
        content_around, "`content-around`.", |s| s.align_content = AlignContent::SPACE_AROUND;
        content_evenly, "`content-evenly`.", |s| s.align_content = AlignContent::SPACE_EVENLY;
        content_stretch, "`content-stretch`.", |s| s.align_content = AlignContent::STRETCH;

        self_auto, "`self-auto`.", |s| s.align_self = None;
        self_start, "`self-start`.", |s| s.align_self = Some(AlignItems::FLEX_START);
        self_end, "`self-end`.", |s| s.align_self = Some(AlignItems::FLEX_END);
        self_center, "`self-center`.", |s| s.align_self = Some(AlignItems::CENTER);
        self_stretch, "`self-stretch`.", |s| s.align_self = Some(AlignItems::STRETCH);
        self_baseline, "`self-baseline`.", |s| s.align_self = Some(AlignItems::BASELINE);

        justify_items_start, "`justify-items-start`.", |s| s.justify_items = AlignItems::START;
        justify_items_end, "`justify-items-end`.", |s| s.justify_items = AlignItems::END;
        justify_items_center, "`justify-items-center`.", |s| s.justify_items = AlignItems::CENTER;
        justify_items_stretch, "`justify-items-stretch`.", |s| s.justify_items = AlignItems::STRETCH;

        justify_self_auto, "`justify-self-auto`.", |s| s.justify_self = None;
        justify_self_start, "`justify-self-start`.", |s| s.justify_self = Some(AlignItems::START);
        justify_self_end, "`justify-self-end`.", |s| s.justify_self = Some(AlignItems::END);
        justify_self_center, "`justify-self-center`.", |s| s.justify_self = Some(AlignItems::CENTER);
        justify_self_stretch, "`justify-self-stretch`.", |s| s.justify_self = Some(AlignItems::STRETCH);

        relative, "`relative`.", |s| s.position = Position::Relative;
        absolute, "`absolute`.", |s| s.position = Position::Absolute;
        static_, "`static`.", |s| s.position = Position::Static;

        overflow_visible, "`overflow-visible`.", |s| { s.overflow.x = Overflow::Visible; s.overflow.y = Overflow::Visible };
        overflow_hidden, "`overflow-hidden`.", |s| { s.overflow.x = Overflow::Hidden; s.overflow.y = Overflow::Hidden };
        overflow_clip, "`overflow-clip`.", |s| { s.overflow.x = Overflow::Clip; s.overflow.y = Overflow::Clip };
        overflow_x_visible, "`overflow-x-visible`.", |s| s.overflow.x = Overflow::Visible;
        overflow_x_hidden, "`overflow-x-hidden`.", |s| s.overflow.x = Overflow::Hidden;
        overflow_x_clip, "`overflow-x-clip`.", |s| s.overflow.x = Overflow::Clip;
        overflow_y_visible, "`overflow-y-visible`.", |s| s.overflow.y = Overflow::Visible;
        overflow_y_hidden, "`overflow-y-hidden`.", |s| s.overflow.y = Overflow::Hidden;
        overflow_y_clip, "`overflow-y-clip`.", |s| s.overflow.y = Overflow::Clip;

        box_border, "`box-border`: sizes include padding and border.", |s| s.box_sizing = BoxSizing::BorderBox;
        box_content, "`box-content`: sizes exclude padding and border.", |s| s.box_sizing = BoxSizing::ContentBox;

        min_w_0, "`min-w-0`.", |s| s.min_size.width = LengthPercentageAuto::ZERO;
        min_h_0, "`min-h-0`.", |s| s.min_size.height = LengthPercentageAuto::ZERO;
        w_fit, "`w-fit`: the content's width, not stretched.", |s| s.size.width = Dimension::fit_content();
        h_fit, "`h-fit`: the content's height, not stretched.", |s| s.size.height = Dimension::fit_content();
    }

    /// `gap-<units>`: spacing between children on both axes.
    pub fn gap(self, units: f32) -> Self {
        self.styled(|s| {
            s.gap.width = space(units);
            s.gap.height = space(units);
        })
    }

    /// `gap-x-<units>`: spacing between columns.
    pub fn gap_x(self, units: f32) -> Self {
        self.styled(|s| s.gap.width = space(units))
    }

    /// `gap-y-<units>`: spacing between rows.
    pub fn gap_y(self, units: f32) -> Self {
        self.styled(|s| s.gap.height = space(units))
    }

    /// `grow-<factor>`.
    pub fn grow_by(self, factor: f32) -> Self {
        self.styled(|s| s.flex_grow = factor)
    }

    /// `aspect-[<ratio>]`: width divided by height.
    pub fn aspect(self, ratio: f32) -> Self {
        self.styled(|s| s.aspect_ratio = Some(ratio))
    }

    /// `border-<width>`: border width in points on every side.
    pub fn border(self, points: f32) -> Self {
        self.styled(|s| s.border = taffy::Rect::length(points))
    }

    /// `border-x-<width>`: left and right border widths in points.
    pub fn border_x(self, points: f32) -> Self {
        self.styled(|s| {
            s.border.left = LengthPercentage::length(points);
            s.border.right = LengthPercentage::length(points);
        })
    }

    /// `border-y-<width>`: top and bottom border widths in points.
    pub fn border_y(self, points: f32) -> Self {
        self.styled(|s| {
            s.border.top = LengthPercentage::length(points);
            s.border.bottom = LengthPercentage::length(points);
        })
    }

    /// `border-t-<width>`: top border width in points.
    pub fn border_t(self, points: f32) -> Self {
        self.styled(|s| s.border.top = LengthPercentage::length(points))
    }

    /// `border-r-<width>`: right border width in points.
    pub fn border_r(self, points: f32) -> Self {
        self.styled(|s| s.border.right = LengthPercentage::length(points))
    }

    /// `border-b-<width>`: bottom border width in points.
    pub fn border_b(self, points: f32) -> Self {
        self.styled(|s| s.border.bottom = LengthPercentage::length(points))
    }

    /// `border-l-<width>`: left border width in points.
    pub fn border_l(self, points: f32) -> Self {
        self.styled(|s| s.border.left = LengthPercentage::length(points))
    }

    /// `basis-<units>`: flex basis in spacing units.
    pub fn basis(self, units: f32) -> Self {
        self.styled(|s| s.flex_basis = Len::space(units).to())
    }

    /// `basis-px`: flex basis in points.
    pub fn basis_px(self, points: f32) -> Self {
        self.styled(|s| s.flex_basis = Len::Points(points).to())
    }

    /// `basis-1/2`: flex basis as a fraction of the container.
    pub fn basis_frac(self, numerator: f32, denominator: f32) -> Self {
        self.styled(|s| s.flex_basis = Len::fraction(numerator, denominator).to())
    }

    /// `basis-full`.
    pub fn basis_full(self) -> Self {
        self.styled(|s| s.flex_basis = Len::Fraction(1.0).to())
    }

    /// `basis-auto`.
    pub fn basis_auto(self) -> Self {
        self.styled(|s| s.flex_basis = Dimension::AUTO)
    }

    spacing! {
        p, "`p-<units>`: padding on every side.", |s, v| s.padding = taffy::Rect::length(v);
        px, "`px-<units>`: left and right padding.", |s, v| {
            s.padding.left = LengthPercentage::length(v);
            s.padding.right = LengthPercentage::length(v);
        };
        py, "`py-<units>`: top and bottom padding.", |s, v| {
            s.padding.top = LengthPercentage::length(v);
            s.padding.bottom = LengthPercentage::length(v);
        };
        pt, "`pt-<units>`: top padding.", |s, v| s.padding.top = LengthPercentage::length(v);
        pr, "`pr-<units>`: right padding.", |s, v| s.padding.right = LengthPercentage::length(v);
        pb, "`pb-<units>`: bottom padding.", |s, v| s.padding.bottom = LengthPercentage::length(v);
        pl, "`pl-<units>`: left padding.", |s, v| s.padding.left = LengthPercentage::length(v);

        m, "`m-<units>`: margin on every side; negative values pull outward.", |s, v| s.margin = taffy::Rect::length(v);
        mx, "`mx-<units>`: left and right margin.", |s, v| {
            s.margin.left = LengthPercentageAuto::length(v);
            s.margin.right = LengthPercentageAuto::length(v);
        };
        my, "`my-<units>`: top and bottom margin.", |s, v| {
            s.margin.top = LengthPercentageAuto::length(v);
            s.margin.bottom = LengthPercentageAuto::length(v);
        };
        mt, "`mt-<units>`: top margin.", |s, v| s.margin.top = LengthPercentageAuto::length(v);
        mr, "`mr-<units>`: right margin.", |s, v| s.margin.right = LengthPercentageAuto::length(v);
        mb, "`mb-<units>`: bottom margin.", |s, v| s.margin.bottom = LengthPercentageAuto::length(v);
        ml, "`ml-<units>`: left margin.", |s, v| s.margin.left = LengthPercentageAuto::length(v);
    }

    auto_margins! {
        m_auto, "`m-auto`.", |s| s.margin = taffy::Rect::AUTO;
        mx_auto, "`mx-auto`.", |s| { s.margin.left = LengthPercentageAuto::AUTO; s.margin.right = LengthPercentageAuto::AUTO };
        my_auto, "`my-auto`.", |s| { s.margin.top = LengthPercentageAuto::AUTO; s.margin.bottom = LengthPercentageAuto::AUTO };
        mt_auto, "`mt-auto`.", |s| s.margin.top = LengthPercentageAuto::AUTO;
        mr_auto, "`mr-auto`.", |s| s.margin.right = LengthPercentageAuto::AUTO;
        mb_auto, "`mb-auto`.", |s| s.margin.bottom = LengthPercentageAuto::AUTO;
        ml_auto, "`ml-auto`.", |s| s.margin.left = LengthPercentageAuto::AUTO;
    }

    lengths! {
        w, w_px, w_frac, w_full, w_auto, "the width", |s, l| s.size.width = l.to();
        h, h_px, h_frac, h_full, h_auto, "the height", |s, l| s.size.height = l.to();
        size, size_px, size_frac, size_full, size_auto, "the width and height", |s, l| s.size = taffy::Size { width: l.to(), height: l.to() };
        min_w, min_w_px, min_w_frac, min_w_full, min_w_auto, "the minimum width", |s, l| s.min_size.width = l.to();
        min_h, min_h_px, min_h_frac, min_h_full, min_h_auto, "the minimum height", |s, l| s.min_size.height = l.to();
        max_w, max_w_px, max_w_frac, max_w_full, max_w_auto, "the maximum width", |s, l| s.max_size.width = l.to();
        max_h, max_h_px, max_h_frac, max_h_full, max_h_auto, "the maximum height", |s, l| s.max_size.height = l.to();
        inset, inset_px, inset_frac, inset_full, inset_auto, "the inset on every side", |s, l| s.inset = taffy::Rect { left: l.to(), right: l.to(), top: l.to(), bottom: l.to() };
        inset_x, inset_x_px, inset_x_frac, inset_x_full, inset_x_auto, "the left and right inset", |s, l| { s.inset.left = l.to(); s.inset.right = l.to() };
        inset_y, inset_y_px, inset_y_frac, inset_y_full, inset_y_auto, "the top and bottom inset", |s, l| { s.inset.top = l.to(); s.inset.bottom = l.to() };
        top, top_px, top_frac, top_full, top_auto, "the top inset", |s, l| s.inset.top = l.to();
        right, right_px, right_frac, right_full, right_auto, "the right inset", |s, l| s.inset.right = l.to();
        bottom, bottom_px, bottom_frac, bottom_full, bottom_auto, "the bottom inset", |s, l| s.inset.bottom = l.to();
        left, left_px, left_frac, left_full, left_auto, "the left inset", |s, l| s.inset.left = l.to();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::tw::tw;
    use taffy::{Rect, Size, Style};

    /// Asserts that `tw` produces the base style with exactly `change` applied.
    #[track_caller]
    pub(crate) fn expect(tw: Tw, change: impl FnOnce(&mut Style)) {
        let mut want = super::super::tw().style;
        change(&mut want);
        assert_eq!(tw.style, want);
    }

    fn px(points: f32) -> Dimension {
        Dimension::length(points)
    }

    #[test]
    fn display() {
        expect(tw().flex(), |s| s.display = Display::Flex);
        expect(tw().grid(), |s| s.display = Display::Grid);
        expect(tw().hidden(), |s| s.display = Display::None);
        expect(tw().flex().block(), |s| s.display = Display::Block);
    }

    #[test]
    fn flex_direction_and_wrap() {
        expect(tw().flex_row(), |s| s.flex_direction = FlexDirection::Row);
        expect(tw().flex_col(), |s| {
            s.flex_direction = FlexDirection::Column
        });
        expect(tw().flex_row_reverse(), |s| {
            s.flex_direction = FlexDirection::RowReverse
        });
        expect(tw().flex_col_reverse(), |s| {
            s.flex_direction = FlexDirection::ColumnReverse
        });
        expect(tw().flex_wrap(), |s| s.flex_wrap = FlexWrap::Wrap);
        expect(tw().flex_wrap_reverse(), |s| {
            s.flex_wrap = FlexWrap::WrapReverse
        });
        expect(tw().flex_wrap().flex_nowrap(), |s| {
            s.flex_wrap = FlexWrap::NoWrap
        });
    }

    #[test]
    fn flex_item_shorthands() {
        expect(tw().flex_1(), |s| {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = Dimension::percent(0.0);
        });
        expect(tw().flex_auto(), |s| {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
        });
        expect(tw().flex_initial(), |s| s.flex_shrink = 1.0);
        expect(tw().flex_1().flex_none(), |s| {
            s.flex_grow = 0.0;
            s.flex_shrink = 0.0;
        });
        expect(tw().grow(), |s| s.flex_grow = 1.0);
        expect(tw().grow().grow_0(), |s| s.flex_grow = 0.0);
        expect(tw().grow_by(2.5), |s| s.flex_grow = 2.5);
        expect(tw().shrink_0(), |s| s.flex_shrink = 0.0);
        expect(tw().shrink_0().shrink(), |s| s.flex_shrink = 1.0);
    }

    #[test]
    fn flex_basis() {
        expect(tw().basis(2.0), |s| s.flex_basis = px(8.0));
        expect(tw().basis_px(37.0), |s| s.flex_basis = px(37.0));
        expect(tw().basis_frac(1.0, 4.0), |s| {
            s.flex_basis = Dimension::percent(0.25)
        });
        expect(tw().basis_full(), |s| {
            s.flex_basis = Dimension::percent(1.0)
        });
        expect(tw().basis(2.0).basis_auto(), |_| {});
    }

    #[test]
    fn gap() {
        expect(tw().gap(2.5), |s| {
            s.gap = Size {
                width: LengthPercentage::length(10.0),
                height: LengthPercentage::length(10.0),
            }
        });
        expect(tw().gap_x(1.0), |s| {
            s.gap.width = LengthPercentage::length(4.0)
        });
        expect(tw().gap_y(1.0), |s| {
            s.gap.height = LengthPercentage::length(4.0)
        });
    }

    #[test]
    fn padding() {
        let l = LengthPercentage::length;
        expect(tw().p(4.0), |s| s.padding = Rect::length(16.0));
        expect(tw().px(1.0), |s| {
            s.padding.left = l(4.0);
            s.padding.right = l(4.0);
        });
        expect(tw().py(1.0), |s| {
            s.padding.top = l(4.0);
            s.padding.bottom = l(4.0);
        });
        expect(tw().pt(1.5), |s| s.padding.top = l(6.0));
        expect(tw().pr(1.5), |s| s.padding.right = l(6.0));
        expect(tw().pb(1.5), |s| s.padding.bottom = l(6.0));
        expect(tw().pl(1.5), |s| s.padding.left = l(6.0));
    }

    #[test]
    fn margin() {
        let l = LengthPercentageAuto::length;
        let auto = LengthPercentageAuto::AUTO;
        expect(tw().m(1.0), |s| s.margin = Rect::length(4.0));
        expect(tw().mx(1.0), |s| {
            s.margin.left = l(4.0);
            s.margin.right = l(4.0);
        });
        expect(tw().my(1.0), |s| {
            s.margin.top = l(4.0);
            s.margin.bottom = l(4.0);
        });
        expect(tw().mt(-2.0), |s| s.margin.top = l(-8.0));
        expect(tw().mr(1.0), |s| s.margin.right = l(4.0));
        expect(tw().mb(1.0), |s| s.margin.bottom = l(4.0));
        expect(tw().ml(1.0), |s| s.margin.left = l(4.0));
        expect(tw().m_auto(), |s| s.margin = Rect::AUTO);
        expect(tw().mx_auto(), |s| {
            s.margin.left = auto;
            s.margin.right = auto;
        });
        expect(tw().my_auto(), |s| {
            s.margin.top = auto;
            s.margin.bottom = auto;
        });
        expect(tw().mt_auto(), |s| s.margin.top = auto);
        expect(tw().mr_auto(), |s| s.margin.right = auto);
        expect(tw().mb_auto(), |s| s.margin.bottom = auto);
        expect(tw().ml_auto(), |s| s.margin.left = auto);
    }

    #[test]
    fn sizes_in_every_spelling() {
        expect(tw().w(12.0), |s| s.size.width = px(48.0));
        expect(tw().w_px(37.0), |s| s.size.width = px(37.0));
        expect(tw().w_frac(1.0, 2.0), |s| {
            s.size.width = Dimension::percent(0.5)
        });
        expect(tw().w_full(), |s| s.size.width = Dimension::percent(1.0));
        expect(tw().w_px(1.0).w_auto(), |_| {});
        expect(tw().h(2.0), |s| s.size.height = px(8.0));
        expect(tw().h_px(100.0), |s| s.size.height = px(100.0));
        expect(tw().h_frac(1.0, 3.0), |s| {
            s.size.height = Dimension::percent(1.0 / 3.0)
        });
        expect(tw().h_full(), |s| s.size.height = Dimension::percent(1.0));
        expect(tw().h_px(1.0).h_auto(), |_| {});
        expect(tw().size(11.0), |s| {
            s.size = Size {
                width: px(44.0),
                height: px(44.0),
            }
        });
        expect(tw().size_px(150.0), |s| {
            s.size = Size {
                width: px(150.0),
                height: px(150.0),
            }
        });
        expect(tw().size_full(), |s| s.size = Size::from_percent(1.0, 1.0));
        expect(tw().size_frac(1.0, 2.0), |s| {
            s.size = Size::from_percent(0.5, 0.5)
        });
        expect(tw().size_px(1.0).size_auto(), |_| {});
    }

    #[test]
    fn min_and_max_sizes() {
        let l = LengthPercentageAuto::length;
        let pct = LengthPercentageAuto::percent;
        expect(tw().min_w(2.0), |s| s.min_size.width = l(8.0));
        expect(tw().min_w_px(120.0), |s| s.min_size.width = l(120.0));
        expect(tw().min_w_frac(1.0, 2.0), |s| s.min_size.width = pct(0.5));
        expect(tw().min_w_full(), |s| s.min_size.width = pct(1.0));
        expect(tw().min_w_0(), |s| s.min_size.width = l(0.0));
        expect(tw().min_h(2.0), |s| s.min_size.height = l(8.0));
        expect(tw().min_h_px(5.0), |s| s.min_size.height = l(5.0));
        expect(tw().min_h_0(), |s| s.min_size.height = l(0.0));
        expect(tw().max_w(2.0), |s| s.max_size.width = l(8.0));
        expect(tw().max_w_px(320.0), |s| s.max_size.width = l(320.0));
        expect(tw().max_w_frac(3.0, 4.0), |s| s.max_size.width = pct(0.75));
        expect(tw().max_w_full(), |s| s.max_size.width = pct(1.0));
        expect(tw().max_h(2.0), |s| s.max_size.height = l(8.0));
        expect(tw().max_h_px(9.0), |s| s.max_size.height = l(9.0));
        expect(tw().max_h_full(), |s| s.max_size.height = pct(1.0));
        expect(tw().max_w_px(9.0).max_w_auto(), |_| {});
        expect(tw().min_h_px(9.0).min_h_auto(), |_| {});
    }

    #[test]
    fn fit_sizes() {
        expect(tw().w_fit(), |s| s.size.width = Dimension::fit_content());
        expect(tw().h_fit(), |s| s.size.height = Dimension::fit_content());
    }

    #[test]
    fn aspect() {
        expect(tw().aspect_square(), |s| s.aspect_ratio = Some(1.0));
        expect(tw().aspect_video(), |s| s.aspect_ratio = Some(16.0 / 9.0));
        expect(tw().aspect(2.0), |s| s.aspect_ratio = Some(2.0));
        expect(tw().aspect_square().aspect_auto(), |_| {});
    }

    #[test]
    fn alignment() {
        expect(tw().items_start(), |s| {
            s.align_items = AlignItems::FLEX_START
        });
        expect(tw().items_end(), |s| s.align_items = AlignItems::FLEX_END);
        expect(tw().items_center(), |s| s.align_items = AlignItems::CENTER);
        expect(tw().items_baseline(), |s| {
            s.align_items = AlignItems::BASELINE
        });
        expect(tw().items_stretch(), |s| {
            s.align_items = AlignItems::STRETCH
        });

        expect(tw().justify_start(), |s| {
            s.justify_content = AlignContent::FLEX_START
        });
        expect(tw().justify_end(), |s| {
            s.justify_content = AlignContent::FLEX_END
        });
        expect(tw().justify_center(), |s| {
            s.justify_content = AlignContent::CENTER
        });
        expect(tw().justify_between(), |s| {
            s.justify_content = AlignContent::SPACE_BETWEEN
        });
        expect(tw().justify_around(), |s| {
            s.justify_content = AlignContent::SPACE_AROUND
        });
        expect(tw().justify_evenly(), |s| {
            s.justify_content = AlignContent::SPACE_EVENLY
        });
        expect(tw().justify_stretch(), |s| {
            s.justify_content = AlignContent::STRETCH
        });

        expect(tw().content_start(), |s| {
            s.align_content = AlignContent::FLEX_START
        });
        expect(tw().content_end(), |s| {
            s.align_content = AlignContent::FLEX_END
        });
        expect(tw().content_center(), |s| {
            s.align_content = AlignContent::CENTER
        });
        expect(tw().content_between(), |s| {
            s.align_content = AlignContent::SPACE_BETWEEN
        });
        expect(tw().content_around(), |s| {
            s.align_content = AlignContent::SPACE_AROUND
        });
        expect(tw().content_evenly(), |s| {
            s.align_content = AlignContent::SPACE_EVENLY
        });
        expect(tw().content_stretch(), |s| {
            s.align_content = AlignContent::STRETCH
        });

        expect(tw().self_start(), |s| {
            s.align_self = Some(AlignItems::FLEX_START)
        });
        expect(tw().self_end(), |s| {
            s.align_self = Some(AlignItems::FLEX_END)
        });
        expect(tw().self_center(), |s| {
            s.align_self = Some(AlignItems::CENTER)
        });
        expect(tw().self_stretch(), |s| {
            s.align_self = Some(AlignItems::STRETCH)
        });
        expect(tw().self_baseline(), |s| {
            s.align_self = Some(AlignItems::BASELINE)
        });
        expect(tw().self_center().self_auto(), |_| {});

        expect(tw().justify_items_start(), |s| {
            s.justify_items = AlignItems::START
        });
        expect(tw().justify_items_end(), |s| {
            s.justify_items = AlignItems::END
        });
        expect(tw().justify_items_center(), |s| {
            s.justify_items = AlignItems::CENTER
        });
        expect(tw().justify_items_stretch(), |s| {
            s.justify_items = AlignItems::STRETCH
        });
        expect(tw().justify_self_start(), |s| {
            s.justify_self = Some(AlignItems::START)
        });
        expect(tw().justify_self_end(), |s| {
            s.justify_self = Some(AlignItems::END)
        });
        expect(tw().justify_self_center(), |s| {
            s.justify_self = Some(AlignItems::CENTER)
        });
        expect(tw().justify_self_stretch(), |s| {
            s.justify_self = Some(AlignItems::STRETCH)
        });
        expect(tw().justify_self_end().justify_self_auto(), |_| {});
    }

    #[test]
    fn position_and_inset() {
        let l = LengthPercentageAuto::length;
        let pct = LengthPercentageAuto::percent;
        expect(tw().relative(), |s| s.position = Position::Relative);
        expect(tw().absolute(), |s| s.position = Position::Absolute);
        expect(tw().absolute().static_(), |_| {});
        expect(tw().inset(0.0), |s| s.inset = Rect::length(0.0));
        expect(tw().inset_px(3.0), |s| s.inset = Rect::length(3.0));
        expect(tw().inset_full(), |s| {
            s.inset = Rect::<LengthPercentageAuto>::from_percent(1.0)
        });
        expect(tw().inset_frac(1.0, 2.0), |s| {
            s.inset = Rect::<LengthPercentageAuto>::from_percent(0.5)
        });
        expect(tw().inset(1.0).inset_auto(), |s| s.inset = Rect::AUTO);
        expect(tw().inset_x(1.0), |s| {
            s.inset.left = l(4.0);
            s.inset.right = l(4.0);
        });
        expect(tw().inset_y_px(2.0), |s| {
            s.inset.top = l(2.0);
            s.inset.bottom = l(2.0);
        });
        expect(tw().top(1.0), |s| s.inset.top = l(4.0));
        expect(tw().right_px(5.0), |s| s.inset.right = l(5.0));
        expect(tw().bottom_frac(1.0, 4.0), |s| s.inset.bottom = pct(0.25));
        expect(tw().left_full(), |s| s.inset.left = pct(1.0));
        expect(tw().top(1.0).top_auto(), |_| {});
        expect(tw().inset_x_frac(1.0, 2.0), |s| {
            s.inset.left = pct(0.5);
            s.inset.right = pct(0.5);
        });
    }

    #[test]
    fn overflow() {
        expect(tw().overflow_hidden(), |s| {
            s.overflow = taffy::Point {
                x: Overflow::Hidden,
                y: Overflow::Hidden,
            }
        });
        expect(tw().overflow_clip(), |s| {
            s.overflow = taffy::Point {
                x: Overflow::Clip,
                y: Overflow::Clip,
            }
        });
        expect(tw().overflow_hidden().overflow_visible(), |_| {});
        expect(tw().overflow_x_hidden(), |s| {
            s.overflow.x = Overflow::Hidden
        });
        expect(tw().overflow_y_clip(), |s| s.overflow.y = Overflow::Clip);
        expect(tw().overflow_x_clip().overflow_x_visible(), |_| {});
        expect(tw().overflow_y_hidden().overflow_y_visible(), |_| {});
        expect(tw().overflow_x_hidden(), |s| {
            s.overflow.x = Overflow::Hidden
        });
    }

    #[test]
    fn border_widths() {
        let l = LengthPercentage::length;
        expect(tw().border(2.0), |s| s.border = Rect::length(2.0));
        expect(tw().border_x(1.0), |s| {
            s.border.left = l(1.0);
            s.border.right = l(1.0);
        });
        expect(tw().border_y(1.0), |s| {
            s.border.top = l(1.0);
            s.border.bottom = l(1.0);
        });
        expect(tw().border_t(3.0), |s| s.border.top = l(3.0));
        expect(tw().border_r(3.0), |s| s.border.right = l(3.0));
        expect(tw().border_b(3.0), |s| s.border.bottom = l(3.0));
        expect(tw().border_l(3.0), |s| s.border.left = l(3.0));
    }

    #[test]
    fn box_sizing() {
        expect(tw().box_content(), |s| s.box_sizing = BoxSizing::ContentBox);
        expect(tw().box_content().box_border(), |_| {});
    }
}
