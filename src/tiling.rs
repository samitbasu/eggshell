//! Hit areas that tile a container: each cell takes the space around it up to
//! the midpoint of the gap to its neighbour, or to the container's edge where
//! it has none, so that no point of the container belongs to no cell. Cells
//! are read in rows, as flex lays them out and wraps them: a neighbour across
//! is one beside the cell in its row, and one above or below is in another
//! row, wherever it is across.

use egui::{Rangef, Rect};

/// Whether a container's sensing children tile its padding box for the
/// pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tiling {
    #[default]
    Untiled,
    Tiled,
}

/// The hit area of each of `cells` within `area`.
pub(crate) fn tile(area: Rect, cells: &[Rect]) -> Vec<Rect> {
    cells
        .iter()
        .map(|&cell| {
            let beside = |other: &&Rect| other.y_range().intersects(cell.y_range());
            let left = cells
                .iter()
                .filter(beside)
                .filter(|other| other.max.x <= cell.min.x)
                .fold(area.min.x, |edge, other| {
                    edge.max(midpoint(other.max.x, cell.min.x))
                });
            let right = cells
                .iter()
                .filter(beside)
                .filter(|other| other.min.x >= cell.max.x)
                .fold(area.max.x, |edge, other| {
                    edge.min(midpoint(cell.max.x, other.min.x))
                });
            let top = cells
                .iter()
                .filter(|other| other.max.y <= cell.min.y)
                .fold(area.min.y, |edge, other| {
                    edge.max(midpoint(other.max.y, cell.min.y))
                });
            let bottom = cells
                .iter()
                .filter(|other| other.min.y >= cell.max.y)
                .fold(area.max.y, |edge, other| {
                    edge.min(midpoint(cell.max.y, other.min.y))
                });
            Rect::from_x_y_ranges(
                Rangef::new(left.min(cell.min.x), right.max(cell.max.x)),
                Rangef::new(top.min(cell.min.y), bottom.max(cell.max.y)),
            )
        })
        .collect()
}

fn midpoint(a: f32, b: f32) -> f32 {
    (a + b) / 2.0
}

#[cfg(test)]
mod tests {
    use egui::{Rect, pos2};

    use super::tile;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    }

    #[test]
    fn a_row_of_cells_tiles_its_track_to_the_gaps_midpoints() {
        let area = rect(0.0, 0.0, 100.0, 20.0);
        let cells = [rect(3.0, 3.0, 30.0, 17.0), rect(34.0, 3.0, 97.0, 17.0)];
        assert_eq!(
            tile(area, &cells),
            [rect(0.0, 0.0, 32.0, 20.0), rect(32.0, 0.0, 100.0, 20.0)]
        );
    }

    #[test]
    fn a_lone_cell_takes_the_whole_area() {
        let area = rect(0.0, 0.0, 50.0, 20.0);
        assert_eq!(tile(area, &[rect(3.0, 3.0, 47.0, 17.0)]), [area]);
    }

    #[test]
    fn wrapped_rows_split_at_the_row_gap_and_a_short_rows_last_cell_reaches_the_edge() {
        let area = rect(0.0, 0.0, 100.0, 50.0);
        let cells = [
            rect(2.0, 2.0, 48.0, 22.0),
            rect(52.0, 2.0, 98.0, 22.0),
            rect(2.0, 26.0, 48.0, 48.0),
        ];
        assert_eq!(
            tile(area, &cells),
            [
                rect(0.0, 0.0, 50.0, 24.0),
                rect(50.0, 0.0, 100.0, 24.0),
                rect(0.0, 24.0, 100.0, 50.0),
            ]
        );
    }

    #[test]
    fn a_cell_never_shrinks_below_its_own_box() {
        let area = rect(10.0, 10.0, 20.0, 20.0);
        let cell = rect(0.0, 0.0, 30.0, 30.0);
        assert_eq!(tile(area, &[cell]), [cell]);
    }
}
