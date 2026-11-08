//! A select: a stock `egui::ComboBox`, whose popup state egui keeps inside
//! the leaf.

use egui::{Context, Response, Ui, Vec2};

use crate::widget::{ComboBox, Stock};
use crate::{Measure, TextStyle, Width};

/// The options of a select and which one is shown.
///
/// The header is as large as the widest option's, so choosing does not move
/// the layout.
#[derive(Clone, Debug)]
pub struct Options {
    labels: Vec<String>,
    name: Option<String>,
}

impl Options {
    /// Options labelled `labels`, in order.
    pub fn new<S: Into<String>>(labels: impl IntoIterator<Item = S>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            name: None,
        }
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn label(&self, index: usize) -> &str {
        self.labels.get(index).map_or("", String::as_str)
    }
}

impl Stock for Options {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        self.labels
            .iter()
            .map(|label| ComboBox::new(label.as_str()).size(ctx, text, width))
            .fold(ComboBox::new("").size(ctx, text, width), Vec2::max)
    }
}

/// A leaf's measure for a select over `options`.
pub fn measure(m: &Measure<'_>, options: &Options) -> taffy::Size<f32> {
    options.measure(m)
}

/// Shows the header with option `*selected` and a popup that changes it.
///
/// Disable it with [`inert`](crate::Tw::inert) on the leaf.
pub fn show(ui: &mut Ui, selected: &mut usize, options: &Options) -> Response {
    let chosen = options.label(*selected).to_owned();
    let combo = ComboBox::new(chosen);
    let combo = match &options.name {
        Some(name) => combo.named(name.as_str()),
        None => combo,
    };
    combo.show(ui, |ui| {
        for (index, label) in options.labels.iter().enumerate() {
            ui.selectable_value(selected, index, label.as_str());
        }
    })
}
