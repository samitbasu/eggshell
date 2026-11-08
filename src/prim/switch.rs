//! A switch: a labelled on/off track that slides a knob.

use egui::{
    Context, Frame, NumExt as _, Pos2, Rect, Response, Role, Sense, TextWrapMode, Ui, Vec2,
    WidgetInfo, WidgetText, epaint::CircleShape, pos2, vec2,
};

use crate::sizing::{Laid, Measured, Piece, Row, Sizer};
use crate::widget::Stock;
use taffy::Size;

use crate::{Measure, TextStyle, Width};

/// A switch, with an optional label to the right of the track.
///
/// The track is shadcn's 32 by 18.4 points under the [`style`](crate::style)
/// preset and `2 * icon_width` by `icon_width` otherwise. An unlabelled
/// switch is its track alone and follows the leaf: `w` and `h` classes size
/// it (one of them sizes the other axis in proportion), and the thumb and its
/// travel scale with the track. A labelled switch keeps the default track,
/// centred in a row at least `interact_size.y` tall; for a larger one, put an
/// unlabelled switch and a text node in a flex row.
#[derive(Clone, Debug)]
pub struct Switch {
    label: Option<String>,
    name: Option<String>,
}

impl Switch {
    /// A switch labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            name: None,
        }
    }

    /// A switch without a label.
    pub fn unlabelled() -> Self {
        Self {
            label: None,
            name: None,
        }
    }

    /// The accessible name, for a switch whose label is not text beside it.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn default_track(sizer: &Sizer<'_>) -> Vec2 {
        crate::style::tokens(sizer.ctx).map_or_else(
            || {
                vec2(
                    2.0 * sizer.style.spacing.icon_width,
                    sizer.style.spacing.icon_width,
                )
            },
            |tokens| tokens.switch_track,
        )
    }

    /// The track an unlabelled switch takes when layout fixes some of its size.
    fn track_for(sizer: &Sizer<'_>, known: Size<Option<f32>>) -> Vec2 {
        let default = Self::default_track(sizer);
        match (known.width, known.height) {
            (Some(width), Some(height)) => vec2(width, height),
            (Some(width), None) => vec2(width, width * default.y / default.x),
            (None, Some(height)) => vec2(height * default.x / default.y, height),
            (None, None) => default,
        }
    }

    /// The one sizing both `size` and `show` use.
    fn lay_out(&self, sizer: &Sizer<'_>, width: Option<Width>, track: Vec2) -> Laid {
        let spacing = &sizer.style.spacing;
        let floor = if self.label.is_some() {
            spacing.interact_size.y
        } else {
            0.0
        };
        let min_size = vec2(0.0, floor);
        let row = Row {
            pieces: std::iter::once(Piece::Fixed(vec2(track.x, track.y.at_least(floor))))
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
        let track = match self.label {
            Some(_) => Self::default_track(&sizer),
            None => rect.size(),
        };
        let laid = self.lay_out(&sizer, Some(Width::new(rect.width())), track);
        let id = ui.make_persistent_id("switch");
        let mut response = ui.interact(rect, id, Sense::click());
        if response.clicked() {
            *on = !*on;
            response.mark_changed();
        }
        let label = self.name.or(self.label).unwrap_or_default();
        response.widget_info(|| WidgetInfo::selected(Role::Switch, ui.is_enabled(), *on, &label));

        if !ui.is_rect_visible(rect) {
            return response;
        }
        let visuals = *ui.style().interact(&response);
        let colors = Colors::of(ui, &visuals, *on, track);
        let slide = ui.ctx().animate_bool_responsive(id, *on);
        let mut cursor = rect.left();
        for piece in &laid.pieces {
            let size = piece.size();
            let top_left = pos2(cursor, rect.center().y - size.y / 2.0);
            match piece {
                Measured::Fixed(_) => {
                    let track =
                        Rect::from_center_size(Rect::from_min_size(top_left, size).center(), track)
                            .expand(visuals.expansion);
                    paint_track(ui, track, slide, &colors);
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

/// What a switch is painted with: the preset's colours when one is applied,
/// otherwise egui's visuals.
struct Colors {
    track: egui::Color32,
    outline: egui::Stroke,
    thumb: egui::Color32,
    thumb_radius: f32,
}

impl Colors {
    fn of(ui: &Ui, visuals: &egui::style::WidgetVisuals, on: bool, track: Vec2) -> Self {
        match crate::style::tokens(ui.ctx()) {
            Some(tokens) => {
                let (track_color, thumb) = if on {
                    (tokens.switch_track_on, tokens.switch_thumb_on)
                } else {
                    (tokens.switch_track_off, tokens.switch_thumb_off)
                };
                Self {
                    track: track_color,
                    outline: egui::Stroke::NONE,
                    thumb,
                    thumb_radius: tokens.switch_thumb_for(track) / 2.0,
                }
            }
            None => Self {
                track: if on {
                    ui.visuals().selection.bg_fill
                } else {
                    visuals.bg_fill
                },
                outline: visuals.bg_stroke,
                thumb: visuals.fg_stroke.color,
                thumb_radius: track.y / 2.0 - 2.0,
            },
        }
    }
}

fn paint_track(ui: &Ui, track: Rect, slide: f32, colors: &Colors) {
    let radius = track.height() / 2.0;
    ui.painter().rect(
        track,
        radius,
        colors.track,
        colors.outline,
        egui::StrokeKind::Inside,
    );
    let travel = track.width() - 2.0 * radius;
    let centre: Pos2 = pos2(track.left() + radius + travel * slide, track.center().y);
    ui.painter().add(CircleShape {
        center: centre,
        radius: colors.thumb_radius,
        fill: colors.thumb,
        stroke: egui::Stroke::NONE,
    });
}

impl Stock for Switch {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        self.lay_out(&sizer, width, Self::default_track(&sizer))
            .outer
    }

    fn measure(&self, m: &Measure<'_>) -> Size<f32> {
        let sizer = Sizer::new(m.ctx(), m.text());
        let track = match self.label {
            Some(_) => Self::default_track(&sizer),
            None => Self::track_for(&sizer, m.known()),
        };
        let size = self.lay_out(&sizer, m.width().map(Width::new), track).outer;
        Size {
            width: size.x,
            height: size.y,
        }
    }
}
