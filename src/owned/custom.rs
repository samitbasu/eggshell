//! The custom node: a leaf that hosts egui content of the application's own.

use egui::{Role, Ui, WidgetInfo};
use taffy::Size;

use super::options::{Configure, Interactive, Options, Resolved, Respond};
use super::{Enablement, OnResponse, tipped};
use crate::builder::Slot;
use crate::{Builder, Measure, NodeUi as _, Tw, fill};

type MeasureFn<'f> = Box<dyn Fn(&Measure<'_>) -> Size<f32> + 'f>;

/// An identified leaf that runs egui code in its own [`Ui`]. Start one with
/// [`Builder::custom`] and finish it with [`ui`](Self::ui).
///
/// Its size comes from its classes unless [`measure`](Self::measure) gives
/// the size of its content.
///
/// Its shared options come from [`Configure`], [`Interactive`] and
/// [`Respond`]. A disabled node's `Ui` is disabled: its draw closure sees
/// `ui.is_enabled()` false, and egui's widgets in it take no input. Focus
/// goes to the node, which must sense to take it.
///
/// ```
/// # use eggshell_ui::prelude::*;
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
pub struct Chain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    opts: Options,
    measure: Option<MeasureFn<'f>>,
    role: Role,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static> Chain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K, classes: Tw) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            measure: None,
            opts: Options::with_classes(classes),
            role: Role::Unknown,
            on_response: OnResponse::default(),
        }
    }

    /// Sizes the node from its content. `measure` may borrow application
    /// data for the frame.
    pub fn measure(mut self, measure: impl Fn(&Measure<'_>) -> Size<f32> + 'f) -> Self {
        self.measure = Some(Box::new(measure));
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

    /// Adds the node. `draw` runs at draw time in the node's own `Ui`, with
    /// the application's state.
    #[track_caller]
    pub fn ui(self, draw: impl FnOnce(&mut A, &mut Ui) + 'static) {
        let Self {
            mut b,
            slot,
            opts,
            measure,
            role,
            on_response,
        } = self;
        let Resolved {
            name,
            classes,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let classes = match enablement {
            Enablement::Enabled => classes,
            Enablement::Disabled => classes.inert(),
        };
        let measure: MeasureFn<'f> = measure.unwrap_or_else(|| Box::new(fill));
        let mut node = b
            .leaf_with(&slot, classes, measure)
            .open_focus(autofocus.open_focus());
        if role != Role::Unknown {
            node = node.role(role);
            if let Some(name) = &name {
                node = node.named(name.clone());
            }
        }
        node.draw(move |app, ui| {
            let response = tipped(ui, ui.node_response(), &tooltip);
            autofocus.apply(ui.ctx(), &response);
            if let Some(name) = &name {
                response.widget_info(|| WidgetInfo::labeled(role, ui.is_enabled(), name));
            }
            on_response.run(app, &response);
            draw(app, ui);
        });
    }
}

impl<A> Configure for Chain<'_, '_, A> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A> Interactive for Chain<'_, '_, A> {}

impl<A> Respond for Chain<'_, '_, A> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A node that hosts custom egui content, sized by classes or by
    /// [`measure`](Chain::measure).
    pub fn custom<K: crate::Key>(&mut self, key: K, classes: Tw) -> Chain<'_, 'f, A> {
        Chain::new(self.reborrow(), key, classes)
    }
}
