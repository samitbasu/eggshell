//! A switch: a labelled on/off track that slides a knob.

use egui::{
    Context, Frame, NumExt as _, Pos2, Rect, Response, Role, Sense, TextWrapMode, Ui, Vec2,
    WidgetInfo, WidgetText, epaint::CircleShape, pos2, vec2,
};

use crate::sizing::{Laid, Measured, Piece, Row, Sizer};
use crate::widget::Stock;
use crate::{TextStyle, Width};

/// A switch, with an optional label to the right of the track.
///
/// The track is `2 * icon_width` by `icon_width`, the size of a checkbox's
/// box doubled, centred in a row at least `interact_size.y` tall.
#[derive(Clone, Debug)]
pub struct Switch {
    label: Option<String>,
}

impl Switch {
    /// A switch labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
        }
    }

    /// A switch without a label.
    pub fn unlabelled() -> Self {
        Self { label: None }
    }

    /// The one sizing both `size` and `show` use.
    fn lay_out(&self, sizer: &Sizer<'_>, width: Option<Width>) -> Laid {
        let spacing = &sizer.style.spacing;
        let track = vec2(2.0 * spacing.icon_width, spacing.icon_width);
        let min_size = vec2(0.0, spacing.interact_size.y);
        let row = Row {
            pieces: std::iter::once(Piece::Fixed(vec2(
                track.x,
                track.y.at_least(spacing.interact_size.y),
            )))
            .chain(
                self.label
                    .as_ref()
                    .map(|label| Piece::Text(WidgetText::from(label.as_str()))),
            )
            .collect(),
            gap: spacing.icon_spacing,
            frame: Frame::new(),
            min_size,
            wrap: TextWrapMode::Wrap,
        };
        sizer.lay_out(row, width)
    }

    /// Draws the switch in the leaf's content rect. A click flips `on`.
    ///
    /// Disable it with [`inert`](crate::Tw::inert) on the leaf.
    pub fn show(self, ui: &mut Ui, on: &mut bool) -> Response {
        let rect = ui.max_rect();
        let sizer = Sizer::for_ui(ui);
        let laid = self.lay_out(&sizer, Some(Width::new(rect.width())));
        let id = ui.make_persistent_id("switch");
        let mut response = ui.interact(rect, id, Sense::click());
        if response.clicked() {
            *on = !*on;
            response.mark_changed();
        }
        let label = self.label.unwrap_or_default();
        response.widget_info(|| WidgetInfo::selected(Role::Switch, ui.is_enabled(), *on, &label));

        if !ui.is_rect_visible(rect) {
            return response;
        }
        let visuals = *ui.style().interact(&response);
        let slide = ui.ctx().animate_bool_responsive(id, *on);
        let mut cursor = rect.left();
        for piece in &laid.pieces {
            let size = piece.size();
            let top_left = pos2(cursor, rect.center().y - size.y / 2.0);
            match piece {
                Measured::Fixed(_) => {
                    let track = Rect::from_center_size(
                        Rect::from_min_size(top_left, size).center(),
                        vec2(
                            2.0 * sizer.style.spacing.icon_width,
                            sizer.style.spacing.icon_width,
                        ),
                    )
                    .expand(visuals.expansion);
                    paint_track(ui, track, slide, &visuals, *on);
                }
                Measured::Text(galley) => {
                    ui.painter()
                        .galley(top_left, galley.clone(), visuals.text_color());
                }
            }
            cursor += size.x + sizer.style.spacing.icon_spacing;
        }
        response
    }
}

fn paint_track(ui: &Ui, track: Rect, slide: f32, visuals: &egui::style::WidgetVisuals, on: bool) {
    let radius = track.height() / 2.0;
    let fill = if on {
        ui.visuals().selection.bg_fill
    } else {
        visuals.bg_fill
    };
    ui.painter().rect(
        track,
        radius,
        fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let travel = track.width() - 2.0 * radius;
    let centre: Pos2 = pos2(track.left() + radius + travel * slide, track.center().y);
    ui.painter().add(CircleShape {
        center: centre,
        radius: radius - 2.0,
        fill: visuals.fg_stroke.color,
        stroke: egui::Stroke::NONE,
    });
}

impl Stock for Switch {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        self.lay_out(&sizer, width).outer
    }
}
