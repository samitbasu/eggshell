//! The owned progress bar: a determinate fraction, drawn as a slider's track
//! and range.

use egui::{Role, WidgetInfo};

use super::slider::track_and_range;
use super::{Enablement, Tip, tipped};
use crate::builder::Slot;
use crate::{Builder, NodeUi as _, Progress, Style, Tw, tw};

/// A bar filled to `progress`, drawn from the [`Style`] with the slider's
/// track and range, and announced to assistive technology as a progress
/// indicator whose value is the percentage. Start one with
/// [`Builder::progress`] and finish it with [`add`](Self::add).
///
/// ```html
/// <div role="progressbar" aria-valuenow="40" class="w-full h-1.5 rounded-full bg-muted overflow-hidden">
///   <div class="h-full w-[40%] rounded-full bg-primary"></div>
/// </div>
/// ```
#[must_use = "a progress bar is not added until you call `.add()`"]
pub struct ProgressChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    progress: Progress,
    name: Option<String>,
    tooltip: Tip,
    classes: Tw,
}

impl<'a, 'f, A: 'static> ProgressChain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K, progress: Progress) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            progress,
            name: None,
            tooltip: None,
            classes: tw(),
        }
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the bar, such as its width,
    /// which is the space it is given; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Shown while the pointer hovers the bar, as egui's `on_hover_text`
    /// shows it. It adds to the accessible name and does not replace it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Adds the bar.
    pub fn add(self) {
        let Self {
            mut b,
            slot,
            progress,
            name,
            tooltip,
            classes,
        } = self;
        let style = Style::of(b.ctx());
        let (track, range) = track_and_range(&style, Enablement::Enabled, progress.into());
        let percent = f64::from(f32::from(progress) * 100.0);
        let name = name.unwrap_or_default();
        b.div_id_with(slot.id, &slot.label, track.merge(classes), |b| {
            b.div(range, |_| {});
        })
        .draw(move |_, ui| {
            let response = tipped(ui.node_response(), &tooltip);
            response.widget_info(|| WidgetInfo {
                value: Some(percent),
                ..WidgetInfo::labeled(Role::ProgressIndicator, ui.is_enabled(), &name)
            });
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(100.0);
            });
        });
    }
}
