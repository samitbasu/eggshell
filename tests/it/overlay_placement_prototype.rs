//! PROTOTYPE (M6 design pass, docs/overlays.md). The pure placement function
//! and the test matrix that pins its rules, so the design note's rules are
//! executable rather than prose. The build PR moves `place` into the library
//! (`src/overlay.rs`) and keeps these tests; nothing here is public API yet.

use egui::{Rect, Vec2, pos2, vec2};

/// A signed length in points: a gap, a shift or a margin.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Distance(f32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// Where the overlay's edge sits along the anchor's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Align {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flip {
    Never,
    ToOpposite,
}

#[derive(Clone, Copy, Debug)]
struct Placement {
    side: Side,
    align: Align,
    /// Between the anchor and the overlay along the side; negative overlaps.
    gap: Distance,
    /// Along the anchor's edge, away from the alignment.
    shift: Distance,
    flip: Flip,
    /// Kept clear of the bounds' edges.
    margin: Distance,
}

impl Placement {
    fn below() -> Self {
        Self {
            side: Side::Bottom,
            align: Align::Start,
            gap: Distance(0.0),
            shift: Distance(0.0),
            flip: Flip::ToOpposite,
            margin: Distance(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anchored {
    Attached,
    /// The anchor lies wholly outside the bounds (scrolled out of its
    /// viewport): the overlay is hidden, its open state untouched.
    Detached,
}

#[derive(Clone, Copy, Debug)]
struct Placed {
    rect: Rect,
    side: Side,
    /// The most the overlay may measure on the chosen side, so that a list
    /// taller than that scrolls instead of leaving the screen.
    available: Vec2,
    anchored: Anchored,
}

/// Where an overlay of `size` (its natural, unclamped size) goes.
fn place(anchor: Rect, size: Vec2, p: &Placement, bounds: Rect) -> Placed {
    let area = bounds.shrink(p.margin.0);
    let room = |side: Side| match side {
        Side::Bottom => area.bottom() - anchor.bottom() - p.gap.0,
        Side::Top => anchor.top() - area.top() - p.gap.0,
        Side::Right => area.right() - anchor.right() - p.gap.0,
        Side::Left => anchor.left() - area.left() - p.gap.0,
    };
    let need = |side: Side| if side.vertical() { size.y } else { size.x };
    let fits = |side: Side| need(side) <= room(side);
    let side = match p.flip {
        Flip::ToOpposite
            if !fits(p.side)
                && (fits(p.side.opposite()) || room(p.side.opposite()) > room(p.side)) =>
        {
            p.side.opposite()
        }
        _ => p.side,
    };
    let along = |lo: f32, hi: f32, len: f32| match p.align {
        Align::Start => lo + p.shift.0,
        Align::Center => (lo + hi - len) / 2.0 + p.shift.0,
        Align::End => hi - len - p.shift.0,
    };
    let mut min = if side.vertical() {
        pos2(
            along(anchor.left(), anchor.right(), size.x),
            if side == Side::Bottom {
                anchor.bottom() + p.gap.0
            } else {
                anchor.top() - p.gap.0 - size.y
            },
        )
    } else {
        pos2(
            if side == Side::Right {
                anchor.right() + p.gap.0
            } else {
                anchor.left() - p.gap.0 - size.x
            },
            along(anchor.top(), anchor.bottom(), size.y),
        )
    };
    // Clamp into the area; an overlay larger than the area starts at its edge.
    min.x = min.x.min(area.right() - size.x).max(area.left());
    min.y = min.y.min(area.bottom() - size.y).max(area.top());
    let main = room(side).max(0.0);
    let cross = if side.vertical() {
        area.width()
    } else {
        area.height()
    };
    let available = if side.vertical() {
        vec2(cross, main)
    } else {
        vec2(main, cross)
    };
    Placed {
        rect: Rect::from_min_size(min, size),
        side,
        available,
        anchored: if anchor.intersects(bounds) {
            Anchored::Attached
        } else {
            Anchored::Detached
        },
    }
}

const BOUNDS: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
const SIZE: Vec2 = vec2(100.0, 60.0);

fn anchor() -> Rect {
    Rect::from_min_size(pos2(150.0, 120.0), vec2(80.0, 30.0))
}

fn at(side: Side, align: Align) -> Placement {
    Placement {
        side,
        align,
        ..Placement::below()
    }
}

#[test]
fn every_side_and_alignment_with_room_to_spare() {
    let a = anchor();
    let cases = [
        (Side::Bottom, Align::Start, pos2(150.0, 150.0)),
        (Side::Bottom, Align::Center, pos2(140.0, 150.0)),
        (Side::Bottom, Align::End, pos2(130.0, 150.0)),
        (Side::Top, Align::Start, pos2(150.0, 60.0)),
        (Side::Top, Align::Center, pos2(140.0, 60.0)),
        (Side::Top, Align::End, pos2(130.0, 60.0)),
        (Side::Right, Align::Start, pos2(230.0, 120.0)),
        (Side::Right, Align::Center, pos2(230.0, 105.0)),
        (Side::Right, Align::End, pos2(230.0, 90.0)),
        (Side::Left, Align::Start, pos2(50.0, 120.0)),
        (Side::Left, Align::Center, pos2(50.0, 105.0)),
        (Side::Left, Align::End, pos2(50.0, 90.0)),
    ];
    for (side, align, min) in cases {
        let placed = place(a, SIZE, &at(side, align), BOUNDS);
        assert_eq!(
            placed.rect,
            Rect::from_min_size(min, SIZE),
            "{side:?} {align:?}"
        );
        assert_eq!(placed.side, side);
        assert_eq!(placed.anchored, Anchored::Attached);
    }
}

#[test]
fn gap_and_shift_move_the_overlay_and_a_negative_gap_overlaps() {
    let a = anchor();
    let p = Placement {
        gap: Distance(4.0),
        shift: Distance(2.0),
        ..at(Side::Bottom, Align::Start)
    };
    assert_eq!(place(a, SIZE, &p, BOUNDS).rect.min, pos2(152.0, 154.0));
    let sub = Placement {
        gap: Distance(-4.0),
        shift: Distance(-4.0),
        ..at(Side::Right, Align::Start)
    };
    assert_eq!(place(a, SIZE, &sub, BOUNDS).rect.min, pos2(226.0, 116.0));
}

#[test]
fn flips_to_the_opposite_side_only_when_the_preferred_does_not_fit() {
    let low = Rect::from_min_size(pos2(150.0, 250.0), vec2(80.0, 30.0));
    let flipped = place(low, SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(flipped.side, Side::Top);
    assert_eq!(flipped.rect.bottom(), low.top());
    let stay = place(anchor(), SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(stay.side, Side::Bottom);
    let never = Placement {
        flip: Flip::Never,
        ..at(Side::Bottom, Align::Start)
    };
    let clamped = place(low, SIZE, &never, BOUNDS);
    assert_eq!(clamped.side, Side::Bottom);
    assert_eq!(
        clamped.rect.bottom(),
        BOUNDS.bottom(),
        "kept on screen, covering the anchor"
    );
}

#[test]
fn neither_side_fits_picks_the_roomier_and_reports_what_is_available() {
    let a = Rect::from_min_size(pos2(150.0, 100.0), vec2(80.0, 30.0));
    let tall = vec2(100.0, 250.0);
    let placed = place(a, tall, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(placed.side, Side::Bottom, "170 below beats 100 above");
    assert_eq!(placed.available.y, 170.0);
    let a = Rect::from_min_size(pos2(150.0, 180.0), vec2(80.0, 30.0));
    let placed = place(a, tall, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(placed.side, Side::Top, "180 above beats 90 below");
    assert_eq!(placed.available.y, 180.0);
}

#[test]
fn the_cross_axis_is_shifted_into_the_bounds() {
    let near_right = Rect::from_min_size(pos2(360.0, 120.0), vec2(30.0, 30.0));
    let placed = place(near_right, SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(placed.rect.right(), BOUNDS.right());
    let near_left = Rect::from_min_size(pos2(5.0, 120.0), vec2(30.0, 30.0));
    let placed = place(near_left, SIZE, &at(Side::Bottom, Align::End), BOUNDS);
    assert_eq!(placed.rect.left(), BOUNDS.left());
}

#[test]
fn the_margin_keeps_clear_of_the_edges_and_counts_against_the_room() {
    let p = Placement {
        margin: Distance(8.0),
        ..at(Side::Bottom, Align::Start)
    };
    let near_right = Rect::from_min_size(pos2(380.0, 120.0), vec2(10.0, 30.0));
    assert_eq!(place(near_right, SIZE, &p, BOUNDS).rect.right(), 392.0);
    // Anchor bottom at 180: 120 below without the margin, 112 with it.
    let low = Rect::from_min_size(pos2(150.0, 150.0), vec2(80.0, 30.0));
    let size = vec2(100.0, 115.0);
    assert_eq!(
        place(low, size, &at(Side::Bottom, Align::Start), BOUNDS).side,
        Side::Bottom
    );
    assert_eq!(place(low, size, &p, BOUNDS).side, Side::Top);
}

#[test]
fn an_overlay_larger_than_the_bounds_starts_at_the_edge() {
    let huge = vec2(500.0, 400.0);
    let placed = place(anchor(), huge, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(placed.rect.min, BOUNDS.min);
}

#[test]
fn a_point_anchor_is_a_context_menu_that_flips_at_the_corner() {
    let point = |x, y| Rect::from_min_size(pos2(x, y), Vec2::ZERO);
    let menu = at(Side::Bottom, Align::Start);
    assert_eq!(
        place(point(50.0, 50.0), SIZE, &menu, BOUNDS).rect.min,
        pos2(50.0, 50.0)
    );
    let corner = place(point(390.0, 295.0), SIZE, &menu, BOUNDS);
    assert_eq!(corner.side, Side::Top);
    assert_eq!(corner.rect.right_bottom(), pos2(400.0, 295.0));
}

#[test]
fn an_anchor_outside_the_bounds_is_detached_and_the_overlay_is_still_inside() {
    let gone = Rect::from_min_size(pos2(150.0, -100.0), vec2(80.0, 30.0));
    let placed = place(gone, SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!(placed.anchored, Anchored::Detached);
    assert!(BOUNDS.contains_rect(placed.rect));
}

#[test]
fn placement_is_mirror_and_transpose_symmetric() {
    // Reflection in y maps Bottom to Top, in x maps Start to End: one table of
    // reflections covers what 12 hand cases would.
    let flipx = |r: Rect| {
        Rect::from_min_max(
            pos2(BOUNDS.right() - r.right(), r.top()),
            pos2(BOUNDS.right() - r.left(), r.bottom()),
        )
    };
    let flipy = |r: Rect| {
        Rect::from_min_max(
            pos2(r.left(), BOUNDS.bottom() - r.bottom()),
            pos2(r.right(), BOUNDS.bottom() - r.top()),
        )
    };
    let anchors = [
        anchor(),
        Rect::from_min_size(pos2(5.0, 5.0), vec2(30.0, 20.0)),
        Rect::from_min_size(pos2(350.0, 270.0), vec2(40.0, 25.0)),
        Rect::from_min_size(pos2(190.0, 140.0), Vec2::ZERO),
    ];
    for a in anchors {
        for size in [SIZE, vec2(300.0, 20.0), vec2(20.0, 290.0)] {
            for align in [Align::Start, Align::Center, Align::End] {
                let swap = |align| match align {
                    Align::Start => Align::End,
                    Align::End => Align::Start,
                    c => c,
                };
                // Reflecting y turns Bottom into Top and keeps the alignment.
                let below = place(a, size, &at(Side::Bottom, align), BOUNDS);
                let above = place(flipy(a), size, &at(Side::Top, align), BOUNDS);
                assert_eq!(flipy(below.rect), above.rect, "y {a:?} {size:?} {align:?}");
                // Reflecting x swaps Start and End.
                let mirrored = place(flipx(a), size, &at(Side::Bottom, swap(align)), BOUNDS);
                assert_eq!(
                    flipx(place(a, size, &at(Side::Bottom, align), BOUNDS).rect),
                    mirrored.rect,
                    "x {a:?} {size:?} {align:?}"
                );
            }
        }
    }
}

#[test]
fn every_result_stays_inside_the_bounds_that_can_hold_it() {
    for ax in [0.0, 5.0, 150.0, 330.0, 399.0] {
        for ay in [0.0, 5.0, 120.0, 270.0, 299.0] {
            let a = Rect::from_min_size(pos2(ax, ay), vec2(40.0, 25.0));
            for side in [Side::Top, Side::Bottom, Side::Left, Side::Right] {
                for align in [Align::Start, Align::Center, Align::End] {
                    let placed = place(a, SIZE, &at(side, align), BOUNDS);
                    assert!(
                        BOUNDS.contains_rect(placed.rect),
                        "{a:?} {side:?} {align:?} -> {:?}",
                        placed.rect
                    );
                }
            }
        }
    }
}

#[test]
fn the_choice_depends_on_the_natural_size_only_so_clamping_cannot_make_it_flicker() {
    // A list taller than the room is clamped to `available` and scrolls. The
    // next frame measures the natural size again and must choose the same side.
    let a = Rect::from_min_size(pos2(150.0, 100.0), vec2(80.0, 30.0));
    let natural = vec2(100.0, 250.0);
    let first = place(a, natural, &at(Side::Bottom, Align::Start), BOUNDS);
    let again = place(a, natural, &at(Side::Bottom, Align::Start), BOUNDS);
    assert_eq!((first.side, first.rect), (again.side, again.rect));
    // Fed back its own clamped size it fits where it is, and keeps the side.
    let clamped = vec2(natural.x, first.available.y);
    assert_eq!(
        place(a, clamped, &at(Side::Bottom, Align::Start), BOUNDS).side,
        first.side
    );
}
