//! Grid track lists and placement.

use taffy::geometry::{Line, MinMax};
use taffy::style::{GridPlacement, GridTemplateComponent, RepetitionCount, TrackSizingFunction};
use taffy::style_helpers::{
    TaffyAuto as _, TaffyMaxContent as _, TaffyMinContent as _, flex, fr, length, repeat,
};

use super::Tw;

/// One grid track: `48`, `1fr`, `minmax(0, 1fr)`, ...
///
/// ```
/// use eggshell_ui::{tw, Track};
/// let columns = tw().grid().grid_cols_tracks(&[
///     Track::px(48.0),
///     Track::minmax(Track::px(0.0), Track::fr(1.0)),
/// ]);
/// # let _ = columns;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track(TrackSizingFunction);

impl Track {
    /// A fixed length in points.
    pub fn px(points: f32) -> Self {
        Self(length(points))
    }

    /// A share of the free space.
    pub fn fr(fraction: f32) -> Self {
        Self(fr(fraction))
    }

    /// Sized by its content and the free space.
    pub fn auto() -> Self {
        Self(TrackSizingFunction::AUTO)
    }

    /// As narrow as its content allows.
    pub fn min_content() -> Self {
        Self(TrackSizingFunction::MIN_CONTENT)
    }

    /// As wide as its content wants.
    pub fn max_content() -> Self {
        Self(TrackSizingFunction::MAX_CONTENT)
    }

    /// `minmax(min, max)`: the minimum is taken from `min`'s lower bound and
    /// the maximum from `max`'s upper bound.
    pub fn minmax(min: Self, max: Self) -> Self {
        Self(MinMax {
            min: min.0.min_sizing_function(),
            max: max.0.max_sizing_function(),
        })
    }

    /// `minmax(0, <fraction>fr)`: tracks that share space evenly.
    pub fn flex(fraction: f32) -> Self {
        Self(flex(fraction))
    }
}

fn template(tracks: &[Track]) -> Vec<GridTemplateComponent<String>> {
    tracks
        .iter()
        .map(|track| GridTemplateComponent::Single(track.0))
        .collect()
}

fn even(count: u16) -> Vec<GridTemplateComponent<String>> {
    vec![repeat(
        RepetitionCount::Count(count),
        vec![Track::flex(1.0).0],
    )]
}

fn span(line: &mut Line<GridPlacement>, count: u16) {
    if !matches!(line.start, GridPlacement::Line(_)) {
        line.start = GridPlacement::Span(count);
    }
    line.end = GridPlacement::Span(count);
}

fn full(line: &mut Line<GridPlacement>) {
    line.start = GridPlacement::from_line_index(1);
    line.end = GridPlacement::from_line_index(-1);
}

macro_rules! placement {
    ($($span:ident, $span_full:ident, $start:ident, $end:ident, $field:ident, $axis:literal;)*) => {$(
        #[doc = concat!("Spans `count` ", $axis, "s (`", stringify!($span), "-<count>`).")]
        pub fn $span(self, count: u16) -> Self {
            self.styled(|s| span(s.$field(), count))
        }

        #[doc = concat!("Spans every ", $axis, " of the explicit grid.")]
        pub fn $span_full(self) -> Self {
            self.styled(|s| full(s.$field()))
        }

        #[doc = concat!("Starts at grid line `line` (negative counts from the end).")]
        pub fn $start(self, line: i16) -> Self {
            self.styled(|s| s.$field().start = GridPlacement::from_line_index(line))
        }

        #[doc = concat!("Ends at grid line `line` (negative counts from the end).")]
        pub fn $end(self, line: i16) -> Self {
            self.styled(|s| s.$field().end = GridPlacement::from_line_index(line))
        }
    )*};
}

impl Tw {
    /// `grid-cols-<count>`: `count` columns of `minmax(0, 1fr)`.
    pub fn grid_cols(self, count: u16) -> Self {
        self.styled(|s| *s.grid_template_columns() = even(count))
    }

    /// `grid-rows-<count>`: `count` rows of `minmax(0, 1fr)`.
    pub fn grid_rows(self, count: u16) -> Self {
        self.styled(|s| *s.grid_template_rows() = even(count))
    }

    /// An arbitrary column track list, as in `grid-cols-[48px_minmax(0,1fr)_...]`.
    pub fn grid_cols_tracks(self, tracks: &[Track]) -> Self {
        self.styled(|s| *s.grid_template_columns() = template(tracks))
    }

    /// An arbitrary row track list.
    pub fn grid_rows_tracks(self, tracks: &[Track]) -> Self {
        self.styled(|s| *s.grid_template_rows() = template(tracks))
    }

    /// `grid-flow-row`: auto-placed items fill rows.
    pub fn grid_flow_row(self) -> Self {
        self.styled(|s| *s.grid_auto_flow() = taffy::GridAutoFlow::Row)
    }

    /// `grid-flow-col`: auto-placed items fill columns.
    pub fn grid_flow_col(self) -> Self {
        self.styled(|s| *s.grid_auto_flow() = taffy::GridAutoFlow::Column)
    }

    /// `grid-flow-dense`: back-fills holes, keeping the flow direction.
    pub fn grid_flow_dense(self) -> Self {
        self.styled(|s| {
            let flow = s.grid_auto_flow();
            *flow = match *flow {
                taffy::GridAutoFlow::Row | taffy::GridAutoFlow::RowDense => {
                    taffy::GridAutoFlow::RowDense
                }
                taffy::GridAutoFlow::Column | taffy::GridAutoFlow::ColumnDense => {
                    taffy::GridAutoFlow::ColumnDense
                }
            }
        })
    }

    placement! {
        col_span, col_span_full, col_start, col_end, grid_column, "column";
        row_span, row_span_full, row_start, row_end, grid_row, "row";
    }
}

use taffy::style_helpers::TaffyGridLine as _;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tw::layout::tests::expect;
    use crate::tw::tw;
    use taffy::style::GridAutoFlow;
    use taffy::style_helpers::minmax;

    #[test]
    fn tracks_map_to_taffy() {
        assert_eq!(Track::px(48.0).0, length::<_, TrackSizingFunction>(48.0));
        assert_eq!(Track::fr(2.0).0, fr::<_, TrackSizingFunction>(2.0));
        assert_eq!(
            Track::minmax(Track::px(0.0), Track::fr(1.0)).0,
            minmax::<TrackSizingFunction>(length(0.0), fr(1.0))
        );
        assert_eq!(
            Track::flex(1.0),
            Track::minmax(Track::px(0.0), Track::fr(1.0))
        );
    }

    #[test]
    fn evenly_sized_columns() {
        expect(tw().grid_cols(3), |s| {
            s.grid_template_columns = taffy::style_helpers::evenly_sized_tracks(3)
        });
        expect(tw().grid_rows(2), |s| {
            s.grid_template_rows = taffy::style_helpers::evenly_sized_tracks(2)
        });
    }

    #[test]
    fn arbitrary_track_list() {
        let tracks = [
            Track::px(48.0),
            Track::minmax(Track::px(0.0), Track::fr(1.0)),
            Track::px(68.0),
            Track::px(73.6),
            Track::px(48.0),
            Track::px(20.0),
        ];
        expect(tw().grid_cols_tracks(&tracks), |s| {
            s.grid_template_columns = vec![
                GridTemplateComponent::Single(length(48.0)),
                GridTemplateComponent::Single(minmax(length(0.0), fr(1.0))),
                GridTemplateComponent::Single(length(68.0)),
                GridTemplateComponent::Single(length(73.6)),
                GridTemplateComponent::Single(length(48.0)),
                GridTemplateComponent::Single(length(20.0)),
            ]
        });
        expect(tw().grid_rows_tracks(&[Track::auto()]), |s| {
            s.grid_template_rows = vec![GridTemplateComponent::Single(TrackSizingFunction::AUTO)]
        });
    }

    #[test]
    fn flow() {
        expect(tw().grid_flow_col(), |s| {
            s.grid_auto_flow = GridAutoFlow::Column
        });
        expect(tw().grid_flow_row(), |s| {
            s.grid_auto_flow = GridAutoFlow::Row
        });
        expect(tw().grid_flow_dense(), |s| {
            s.grid_auto_flow = GridAutoFlow::RowDense
        });
        expect(tw().grid_flow_col().grid_flow_dense(), |s| {
            s.grid_auto_flow = GridAutoFlow::ColumnDense
        });
    }

    #[test]
    fn placement() {
        use taffy::style_helpers::{line, span};
        expect(tw().col_span(2), |s| {
            s.grid_column = Line {
                start: span(2),
                end: span(2),
            }
        });
        expect(tw().col_span_full(), |s| {
            s.grid_column = Line {
                start: line(1),
                end: line(-1),
            }
        });
        expect(tw().row_start(2).row_end(4), |s| {
            s.grid_row = Line {
                start: line(2),
                end: line(4),
            }
        });
    }

    #[test]
    fn a_definite_start_survives_a_span_in_either_order() {
        use taffy::style_helpers::{line, span};
        let want = |s: &mut taffy::Style| {
            s.grid_column = Line {
                start: line(2),
                end: span(3),
            }
        };
        expect(tw().col_start(2).col_span(3), want);
        expect(tw().col_span(3).col_start(2), want);
    }
}
