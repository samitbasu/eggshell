//! The custom node: a leaf that hosts egui content of the application's own.

use egui::{Role, Ui, WidgetInfo};
use taffy::Size;

use super::{OnResponse, Tip, tipped};
use crate::builder::Slot;
use crate::{Builder, Measure, NodeUi as _, Tw, fill};

type MeasureFn<'f> = Box<dyn Fn(&Measure<'_>) -> Size<f32> + 'f>;

/// An identified leaf that runs egui code in its own [`Ui`]. Start one with
/// [`Builder::custom`] and finish it with [`ui`](Self::ui).
///
/// Its size comes from its classes unless [`measure`](Self::measure) gives
/// the size of its content.
///
/// ```
/// # use eggshell_ui::{Builder, tw};
/// struct App { angle: f32 }
///
/// fn view(b: &mut Builder<'_, '_, App>, _: &App) {
///     b.custom("angle", tw().w_px(160.0).h_px(24.0))
///         .named("Angle")
///         .ui(|app, ui| {
///             ui.add(egui::DragValue::new(&mut app.angle).suffix(" deg"));
///         });
/// }
/// ```
#[must_use = "a custom node is not added until you call `.ui(..)`"]
pub struct CustomChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    classes: Tw,
    measure: Option<MeasureFn<'f>>,
    name: Option<String>,
    role: Role,
    tooltip: Tip,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static> CustomChain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K, classes: Tw) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            classes,
            measure: None,
            name: None,
            role: Role::Unknown,
            tooltip: None,
            on_response: OnResponse::default(),
        }
    }

    /// Sizes the node from its content. `measure` may borrow application
    /// data for the frame.
    pub fn measure(mut self, measure: impl Fn(&Measure<'_>) -> Size<f32> + 'f) -> Self {
        self.measure = Some(Box::new(measure));
        self
    }

    /// The accessible name of the node.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Announces the node as a `role`, with the semantics of
    /// [`NodeHandle::role`](crate::NodeHandle::role): a node with an item role
    /// (`MenuItem`, `MenuItemCheckBox`, `MenuItemRadio`, `ListBoxOption`) is
    /// an item of the menu or listbox it is in, found by the arrow keys. The
    /// node must sense, and needs a [`named`](Self::named) name.
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Shown while the pointer hovers the node, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)).
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Runs `then` with the node's response each frame.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Adds the node. `draw` runs at draw time in the node's own `Ui`, with
    /// the application's state.
    #[track_caller]
    pub fn ui(self, draw: impl FnOnce(&mut A, &mut Ui) + 'static) {
        let Self {
            mut b,
            slot,
            classes,
            measure,
            name,
            role,
            tooltip,
            on_response,
        } = self;
        let measure: MeasureFn<'f> = measure.unwrap_or_else(|| Box::new(fill));
        let mut node = b.leaf_with(slot.id, &slot.label, classes, measure);
        if role != Role::Unknown {
            node = node.role(role);
            if let Some(name) = &name {
                node = node.named(name.clone());
            }
        }
        node.draw(move |app, ui| {
            let response = tipped(ui, ui.node_response(), &tooltip);
            if let Some(name) = &name {
                response.widget_info(|| WidgetInfo::labeled(role, ui.is_enabled(), name));
            }
            on_response.run(app, &response);
            draw(app, ui);
        });
    }
}
