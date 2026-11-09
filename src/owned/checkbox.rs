//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Color32, Painter, Pos2, Rect, Role, Shape, Stroke, WidgetInfo, pos2};

use super::{Enablement, choice_label, choice_row};
use crate::style::Mark;
use crate::{Builder, NodeUi as _, Style, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Label,
}

/// A checkbox drawn from the [`Style`], with an optional label.
#[derive(Clone, Debug)]
pub struct Checkbox {
    label: Option<String>,
    name: Option<String>,
    mark: Mark,
    enablement: Enablement,
}

/// lucide's `check`, on its 24 point grid.
const CHECK: [(f32, f32); 3] = [(4.0, 12.0), (9.0, 17.0), (20.0, 6.0)];

fn paint_check(painter: &Painter, rect: Rect, color: Color32) {
    let scale = rect.width() / 24.0;
    let at = |(x, y): (f32, f32)| -> Pos2 { rect.min + pos2(x, y).to_vec2() * scale };
    let width = 2.0 * scale;
    let points = CHECK.map(at);
    painter.add(Shape::line(points.to_vec(), Stroke::new(width, color)));
    for point in points {
        painter.circle_filled(point, width / 2.0, color);
    }
}

impl Checkbox {
    /// A checkbox labelled `label`, ticked when `mark` is on.
    pub fn new(label: impl Into<String>, mark: Mark) -> Self {
        Self {
            label: Some(label.into()),
            name: None,
            mark,
            enablement: Enablement::Enabled,
        }
    }

    /// A checkbox without a label.
    pub fn unlabelled(mark: Mark) -> Self {
        Self {
            label: None,
            ..Self::new("", mark)
        }
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether the checkbox takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the checkbox to `b` under `key`. `on_toggle` runs when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        on_toggle: fn(&mut A),
    ) {
        let style = Style::of(b.ctx());
        let checkbox = &style.checkbox;
        let look = match self.mark {
            Mark::Off => &checkbox.off,
            Mark::On => &checkbox.on,
        };
        let enablement = self.enablement;
        let box_classes = look
            .classes(&style, checkbox.radius, enablement, checkbox.transition)
            .flex()
            .items_center()
            .justify_center()
            .shrink_0()
            .size_px(checkbox.size);
        let (glyph, mark) = (checkbox.glyph, self.mark);
        let name = self.name.unwrap_or_default();
        b.scope(key, |b| {
            let control = b.id_of(Part::Box);
            b.div(choice_row(&style, enablement, checkbox.gap), |b| {
                b.div_id(
                    Part::Box,
                    box_classes,
                    move |app, ui| {
                        let response = ui.node_response();
                        if response.clicked() {
                            on_toggle(app);
                        }
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                Role::CheckBox,
                                ui.is_enabled(),
                                mark == Mark::On,
                                &name,
                            )
                        });
                    },
                    |b| {
                        if mark == Mark::On {
                            b.paint(tw().size_px(glyph), paint_check);
                        }
                    },
                );
                if let Some(label) = self.label {
                    choice_label(b, Part::Label, control, label, enablement, on_toggle);
                }
            });
        });
    }
}
