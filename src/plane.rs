//! The two axes of the plane, read from egui's points, vectors and rects by
//! name rather than by egui's `[usize]` indexing, which panics past 1.

use egui::{Pos2, Rect, Vec2, Vec2b, pos2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    X,
    Y,
}

impl Axis {
    pub(crate) const BOTH: [Self; 2] = [Self::X, Self::Y];

    pub(crate) fn of(self, v: Vec2) -> f32 {
        match self {
            Self::X => v.x,
            Self::Y => v.y,
        }
    }

    pub(crate) fn at(self, p: Pos2) -> f32 {
        match self {
            Self::X => p.x,
            Self::Y => p.y,
        }
    }

    pub(crate) fn of_both(self, b: Vec2b) -> bool {
        match self {
            Self::X => b.x,
            Self::Y => b.y,
        }
    }

    /// A vector `amount` long along the axis.
    pub(crate) fn vec(self, amount: f32) -> Vec2 {
        match self {
            Self::X => Vec2::new(amount, 0.0),
            Self::Y => Vec2::new(0.0, amount),
        }
    }

    /// Where `rect` starts and ends along the axis.
    pub(crate) fn span(self, rect: Rect) -> (f32, f32) {
        (self.at(rect.min), self.at(rect.max))
    }

    pub(crate) fn other(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    /// The rect spanning `along` on the axis and `across` on the other.
    pub(crate) fn rect(self, along: (f32, f32), across: (f32, f32)) -> Rect {
        match self {
            Self::X => Rect::from_min_max(pos2(along.0, across.0), pos2(along.1, across.1)),
            Self::Y => Rect::from_min_max(pos2(across.0, along.0), pos2(across.1, along.1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_axis_reads_its_own_coordinate() {
        let rect = Rect::from_min_max(pos2(1.0, 2.0), pos2(3.0, 5.0));
        assert_eq!(Axis::X.span(rect), (1.0, 3.0));
        assert_eq!(Axis::Y.span(rect), (2.0, 5.0));
        assert_eq!(Axis::Y.of(rect.size()), 3.0);
        assert_eq!(Axis::X.other(), Axis::Y);
        assert_eq!(Axis::Y.vec(4.0), Vec2::new(0.0, 4.0));
        assert!(Axis::X.of_both(Vec2b::new(true, false)));
        assert_eq!(Axis::X.rect((1.0, 3.0), (2.0, 5.0)), rect);
        assert_eq!(Axis::Y.rect((2.0, 5.0), (1.0, 3.0)), rect);
    }
}
