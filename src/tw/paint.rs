//! Box paint classes. Stored on the node; drawn by the library.

use egui::{Color32, CornerRadius, Shadow};

use super::{Opacity, Tw};

/// A value for each side of a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sides<T> {
    /// The top side.
    pub top: T,
    /// The right side.
    pub right: T,
    /// The bottom side.
    pub bottom: T,
    /// The left side.
    pub left: T,
}

/// A ring drawn outside the border box; it does not affect layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    /// Width in points.
    pub width: f32,
    /// Colour.
    pub color: Color32,
}

/// Hairlines drawn between consecutive in-flow children.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Divide {
    /// Width in points.
    pub width: f32,
    /// Colour.
    pub color: Color32,
}

/// Everything painted for a node's box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxPaint {
    /// Background fill.
    pub background: Option<Color32>,
    /// Border colour of each side.
    pub border_colors: Sides<Option<Color32>>,
    /// Corner radii.
    pub radius: CornerRadius,
    /// Ring outside the border box.
    pub ring: Option<Ring>,
    /// Drop shadow.
    pub shadow: Option<Shadow>,
    /// Opacity of the node and its subtree.
    pub opacity: Opacity,
    /// Dividers between children laid out in a row.
    pub divide_x: Option<Divide>,
    /// Dividers between children laid out in a column.
    pub divide_y: Option<Divide>,
}

fn radius(points: u8) -> CornerRadius {
    CornerRadius::same(points)
}

macro_rules! rounded {
    ($($name:ident, $doc:literal, $points:expr;)*) => {$(
        #[doc = $doc]
        pub fn $name(self) -> Self {
            self.shaped(|p| p.radius = radius($points))
        }
    )*};
}

macro_rules! side_colors {
    ($($name:ident, $doc:literal, |$p:ident, $c:ident| $set:expr;)*) => {$(
        #[doc = $doc]
        pub fn $name(self, color: impl Into<Color32>) -> Self {
            self.painted(|$p| { let $c = Some(color.into()); $set })
        }
    )*};
}

impl Tw {
    fn painted(mut self, set: impl FnOnce(&mut BoxPaint)) -> Self {
        set(&mut self.paint);
        self
    }

    /// [`painted`](Self::painted) for the properties that merging tracks:
    /// the corner radii and the opacity.
    fn shaped(mut self, set: impl Fn(&mut BoxPaint)) -> Self {
        self.write_paint(set);
        self
    }

    /// `bg-<color>`.
    pub fn bg(self, color: impl Into<Color32>) -> Self {
        self.painted(|p| p.background = Some(color.into()))
    }

    side_colors! {
        border_color, "`border-<color>`: every side.", |p, c| p.border_colors = Sides { top: c, right: c, bottom: c, left: c };
        border_x_color, "`border-x-<color>`: left and right.", |p, c| { p.border_colors.left = c; p.border_colors.right = c };
        border_y_color, "`border-y-<color>`: top and bottom.", |p, c| { p.border_colors.top = c; p.border_colors.bottom = c };
        border_t_color, "`border-t-<color>`.", |p, c| p.border_colors.top = c;
        border_r_color, "`border-r-<color>`.", |p, c| p.border_colors.right = c;
        border_b_color, "`border-b-<color>`.", |p, c| p.border_colors.bottom = c;
        border_l_color, "`border-l-<color>`.", |p, c| p.border_colors.left = c;
    }

    /// `divide-x-<width>`: hairlines between children laid out in a row.
    pub fn divide_x(self, width: f32, color: impl Into<Color32>) -> Self {
        let color = color.into();
        self.painted(|p| p.divide_x = Some(Divide { width, color }))
    }

    /// `divide-y-<width>`: hairlines between children laid out in a column.
    pub fn divide_y(self, width: f32, color: impl Into<Color32>) -> Self {
        let color = color.into();
        self.painted(|p| p.divide_y = Some(Divide { width, color }))
    }

    rounded! {
        rounded_none, "`rounded-none`.", 0;
        rounded_sm, "`rounded-sm`: 2 points.", 2;
        rounded, "`rounded`: 4 points.", 4;
        rounded_md, "`rounded-md`: 6 points.", 6;
        rounded_lg, "`rounded-lg`: 8 points.", 8;
        rounded_xl, "`rounded-xl`: 12 points.", 12;
        rounded_2xl, "`rounded-2xl`: 16 points.", 16;
        rounded_3xl, "`rounded-3xl`: 24 points.", 24;
        rounded_full, "`rounded-full`: as round as the box allows.", u8::MAX;
    }

    /// `rounded-[<r>px]`: corner radius in points, rounded to whole points.
    pub fn rounded_px(self, points: f32) -> Self {
        self.shaped(|p| p.radius = radius(points.round().clamp(0.0, 255.0) as u8))
    }

    /// `rounded-tr-[<r>px]`: top-right corner radius in points, rounded to
    /// whole points; the other corners keep theirs.
    pub fn rounded_tr_px(self, points: f32) -> Self {
        self.shaped(|p| p.radius.ne = points.round().clamp(0.0, 255.0) as u8)
    }

    /// `rounded-br-[<r>px]`: bottom-right corner radius in points, rounded to
    /// whole points; the other corners keep theirs.
    pub fn rounded_br_px(self, points: f32) -> Self {
        self.shaped(|p| p.radius.se = points.round().clamp(0.0, 255.0) as u8)
    }

    /// `ring-<width>`: a ring outside the border box.
    pub fn ring(self, width: f32, color: impl Into<Color32>) -> Self {
        let color = color.into();
        self.painted(|p| p.ring = Some(Ring { width, color }))
    }

    /// `shadow-*`.
    pub fn shadow(self, shadow: Shadow) -> Self {
        self.painted(|p| p.shadow = Some(shadow))
    }

    /// `opacity-*`.
    pub fn opacity(self, opacity: Opacity) -> Self {
        self.shaped(|p| p.opacity = opacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tw::tw;

    fn paint(tw: Tw) -> BoxPaint {
        tw.paint
    }

    #[test]
    fn paint_classes_leave_layout_alone() {
        let styled = tw()
            .bg(Color32::RED)
            .border_color(Color32::BLUE)
            .rounded_lg()
            .ring(2.0, Color32::GREEN)
            .opacity(Opacity::new(0.5));
        assert_eq!(*styled.style(), *tw().style());
    }

    #[test]
    fn background_and_borders() {
        let blue = Some(Color32::BLUE);
        assert_eq!(paint(tw().bg(Color32::RED)).background, Some(Color32::RED));
        assert_eq!(
            paint(tw().border_color(Color32::BLUE)).border_colors,
            Sides {
                top: blue,
                right: blue,
                bottom: blue,
                left: blue
            }
        );
        assert_eq!(
            paint(tw().border_t_color(Color32::BLUE)).border_colors,
            Sides {
                top: blue,
                ..Default::default()
            }
        );
        assert_eq!(
            paint(tw().border_x_color(Color32::BLUE)).border_colors,
            Sides {
                left: blue,
                right: blue,
                ..Default::default()
            }
        );
        assert_eq!(
            paint(tw().border_y_color(Color32::BLUE)).border_colors,
            Sides {
                top: blue,
                bottom: blue,
                ..Default::default()
            }
        );
        assert_eq!(
            paint(tw().border_r_color(Color32::BLUE))
                .border_colors
                .right,
            blue
        );
        assert_eq!(
            paint(tw().border_b_color(Color32::BLUE))
                .border_colors
                .bottom,
            blue
        );
        assert_eq!(
            paint(tw().border_l_color(Color32::BLUE)).border_colors.left,
            blue
        );
    }

    #[test]
    fn radii() {
        assert_eq!(paint(tw().rounded_none()).radius, radius(0));
        assert_eq!(paint(tw().rounded_sm()).radius, radius(2));
        assert_eq!(paint(tw().rounded()).radius, radius(4));
        assert_eq!(paint(tw().rounded_md()).radius, radius(6));
        assert_eq!(paint(tw().rounded_lg()).radius, radius(8));
        assert_eq!(paint(tw().rounded_xl()).radius, radius(12));
        assert_eq!(paint(tw().rounded_2xl()).radius, radius(16));
        assert_eq!(paint(tw().rounded_3xl()).radius, radius(24));
        assert_eq!(paint(tw().rounded_full()).radius, radius(255));
        assert_eq!(paint(tw().rounded_px(12.4)).radius, radius(12));
        assert_eq!(paint(tw().rounded_px(1e6)).radius, radius(255));
        let corners = paint(tw().rounded_tr_px(5.0).rounded_br_px(3.4)).radius;
        assert_eq!(
            (corners.ne, corners.se, corners.nw, corners.sw),
            (5, 3, 0, 0)
        );
    }

    #[test]
    fn ring_shadow_opacity_dividers() {
        assert_eq!(
            paint(tw().ring(2.0, Color32::RED)).ring,
            Some(Ring {
                width: 2.0,
                color: Color32::RED
            })
        );
        let shadow = Shadow {
            offset: [0, 2],
            blur: 4,
            spread: 0,
            color: Color32::BLACK,
        };
        assert_eq!(paint(tw().shadow(shadow)).shadow, Some(shadow));
        assert_eq!(
            paint(tw().opacity(Opacity::new(0.5))).opacity,
            Opacity::new(0.5)
        );
        assert_eq!(paint(tw()).opacity, Opacity::OPAQUE);
        assert_eq!(
            paint(tw().divide_x(1.0, Color32::GRAY)).divide_x,
            Some(Divide {
                width: 1.0,
                color: Color32::GRAY
            })
        );
        assert_eq!(
            paint(tw().divide_y(2.0, Color32::GRAY)).divide_y,
            Some(Divide {
                width: 2.0,
                color: Color32::GRAY
            })
        );
    }
}
