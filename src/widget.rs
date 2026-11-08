//! Stock egui widgets as leaves, with sizes the library computes.
//!
//! Each type holds the content that decides its size (a label, a template for
//! the widest value) and has a pure `size` from that content, the egui
//! [`Style`](egui::Style) and the text galley: it never runs the widget.
//! `show` places the real egui widget in exactly the leaf's content rect.
//! Use them through [`Builder::widget`](crate::Builder::widget), which keeps
//! the measure and the draw on the same value:
//!
//! ```
//! # use bullseye::{Builder, tw, widget::Button};
//! # fn view(b: &mut Builder<'_, '_, u32>) {
//! b.widget("save", tw(), Button::new("Save"), |clicks, ui, button| {
//!     if button.show(ui).clicked() {
//!         *clicks += 1;
//!     }
//! });
//! # }
//! ```
//!
//! A widget the layout sizes (a slider that fills its row) takes the width
//! the leaf is given. `tests/sizing.rs` asserts, for every widget here, that
//! the size equals what egui allocates when it draws the widget.

use std::ops::RangeInclusive;

use egui::{
    Context, Frame, NumExt as _, Response, RichText, TextWrapMode, Ui, Vec2, WidgetText, vec2,
};
use taffy::Size;

use crate::sizing::{Piece, Row, Sizer};
use crate::{Measure, TextStyle, Width};

/// A stock widget the builder can size.
pub trait Stock: Clone + 'static {
    /// The widget's size in `ctx` for text in `text`, when `width` is the
    /// width it will be given. `None` is the natural width.
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2;

    /// The size as a leaf's measure: the known or definite width is the
    /// width the widget is given.
    fn measure(&self, m: &Measure<'_>) -> Size<f32> {
        let size = self.size(m.ctx(), m.text(), m.width().map(Width::new));
        Size {
            width: size.x,
            height: size.y,
        }
    }
}

fn place(ui: &mut Ui, widget: impl egui::Widget) -> Response {
    ui.put(ui.max_rect(), widget)
}

/// Adds `widget` at the top left of the leaf, at the size egui gives it.
/// `place` would justify it to the whole rect, which centres a checkbox's box
/// and label in a leaf that is wider than they are. The layout is a column
/// because egui wraps labels only in vertical layouts, and the estimate
/// wraps them.
fn place_start(ui: &mut Ui, widget: impl egui::Widget) -> Response {
    let layout = egui::Layout::top_down(egui::Align::Min);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(ui.max_rect())
            .layout(layout),
        |ui| ui.add(widget),
    )
    .inner
}

fn icon_row(sizer: &Sizer<'_>, icon: Vec2, label: Option<&String>, min_size: Vec2) -> Row {
    let spacing = &sizer.style.spacing;
    Row {
        pieces: std::iter::once(Piece::Fixed(icon))
            .chain(label.map(|label| Piece::Text(WidgetText::from(label.as_str()))))
            .collect(),
        gap: spacing.icon_spacing,
        frame: Frame::new(),
        min_size,
        wrap: TextWrapMode::Wrap,
    }
}

/// `egui::Button` with a text label.
#[derive(Clone, Debug)]
pub struct Button {
    label: String,
}

impl Button {
    /// A button labelled `label`. Long labels wrap at the leaf's width.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    /// Adds the button to `ui`, filling its rect.
    pub fn show(self, ui: &mut Ui) -> Response {
        place(ui, egui::Button::new(self.label))
    }
}

impl Stock for Button {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let spacing = &sizer.style.spacing;
        let row = Row {
            pieces: vec![Piece::Text(WidgetText::from(self.label.as_str()))],
            gap: spacing.icon_spacing,
            frame: sizer.button_frame(),
            min_size: vec2(0.0, spacing.interact_size.y),
            wrap: TextWrapMode::Wrap,
        };
        sizer.lay_out(row, width).outer
    }
}

/// `egui::Checkbox`, with an optional label.
#[derive(Clone, Debug)]
pub struct Checkbox {
    label: Option<String>,
}

impl Checkbox {
    /// A checkbox labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
        }
    }

    /// A checkbox without a label.
    pub fn unlabelled() -> Self {
        Self { label: None }
    }

    /// Adds the checkbox to `ui`, at the top left of its rect. A click
    /// toggles `checked`.
    pub fn show(self, ui: &mut Ui, checked: &mut bool) -> Response {
        match self.label {
            Some(label) => place_start(ui, egui::Checkbox::new(checked, label)),
            None => place_start(ui, egui::Checkbox::without_text(checked)),
        }
    }
}

impl Stock for Checkbox {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let spacing = &sizer.style.spacing;
        let min_size = vec2(
            spacing.interact_size.y,
            spacing.interact_size.y.at_least(spacing.icon_width),
        );
        let icon = vec2(spacing.icon_width, spacing.icon_width.at_least(min_size.y));
        sizer
            .lay_out(icon_row(&sizer, icon, self.label.as_ref(), min_size), width)
            .outer
    }
}

/// `egui::RadioButton`, with an optional label.
#[derive(Clone, Debug)]
pub struct RadioButton {
    label: Option<String>,
}

impl RadioButton {
    /// A radio button labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
        }
    }

    /// A radio button without a label.
    pub fn unlabelled() -> Self {
        Self { label: None }
    }

    /// Adds the radio button to `ui`, at the top left of its rect.
    pub fn show(self, ui: &mut Ui, selected: bool) -> Response {
        match self.label {
            Some(label) => place_start(ui, egui::RadioButton::new(selected, label)),
            None => place_start(ui, egui::RadioButton::new(selected, ())),
        }
    }
}

impl Stock for RadioButton {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let spacing = &sizer.style.spacing;
        let side = spacing.interact_size.y;
        let min_size = vec2(side, side.at_least(spacing.icon_width));
        let icon = vec2(spacing.icon_width, spacing.icon_width.at_least(min_size.y));
        sizer
            .lay_out(icon_row(&sizer, icon, self.label.as_ref(), min_size), width)
            .outer
    }
}

/// A horizontal `egui::Slider` without a value readout or label.
///
/// The rail takes the width the leaf is given and `Spacing::slider_width`
/// otherwise; show the value with a text node beside it.
#[derive(Clone, Debug, Default)]
pub struct Slider;

impl Slider {
    /// A slider.
    pub fn new() -> Self {
        Self
    }

    /// Adds the slider to `ui`, its rail as wide as the rect. Dragging
    /// changes `value` within `range`.
    pub fn show(self, ui: &mut Ui, value: &mut f64, range: RangeInclusive<f64>) -> Response {
        let rect = ui.max_rect();
        ui.spacing_mut().slider_width = rect.width();
        place(ui, egui::Slider::new(value, range).show_value(false))
    }
}

impl Stock for Slider {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let spacing = &sizer.style.spacing;
        let body = egui::TextStyle::Body.resolve(&sizer.style);
        let thickness = sizer.row_height(&body).at_least(spacing.interact_size.y);
        vec2(width.map_or(spacing.slider_width, f32::from), thickness)
    }
}

/// `egui::DragValue` in its resting, button-like state.
#[derive(Clone, Debug)]
pub struct DragValue {
    template: String,
}

impl DragValue {
    /// A drag value sized for `template`, the text of the widest value it
    /// will show (`"-100.00"`). The field never resizes to the value, and
    /// shows as many decimals as the template does, so `"100"` shows whole
    /// numbers and `"-100.00"` two decimals.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }

    /// Adds the drag value to `ui`, filling its rect. `range` bounds `value`.
    pub fn show(self, ui: &mut Ui, value: &mut f64, range: RangeInclusive<f64>) -> Response {
        let decimals = self.template.split('.').nth(1).map_or(0, str::len);
        place(
            ui,
            egui::DragValue::new(value)
                .range(range)
                .fixed_decimals(decimals),
        )
    }
}

impl Stock for DragValue {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let row = Row {
            pieces: vec![Piece::Text(RichText::new(self.template.as_str()).into())],
            gap: 0.0,
            frame: sizer.button_frame(),
            min_size: sizer.style.spacing.interact_size,
            wrap: TextWrapMode::Extend,
        };
        sizer.lay_out(row, width).outer
    }
}

/// The header of an `egui::ComboBox`: the button that shows the selection.
#[derive(Clone, Debug)]
pub struct ComboBox {
    selected: String,
}

impl ComboBox {
    /// A combo box whose header shows `selected`.
    pub fn new(selected: impl Into<String>) -> Self {
        Self {
            selected: selected.into(),
        }
    }

    /// Adds the combo box to `ui`, its header filling the rect. `items`
    /// fills the popup.
    pub fn show(self, ui: &mut Ui, items: impl FnOnce(&mut Ui)) -> Response {
        let rect = ui.max_rect();
        let combo = egui::ComboBox::from_id_salt(ui.make_persistent_id("combo"))
            .selected_text(self.selected)
            .width(rect.width());
        place(ui, |ui: &mut Ui| combo.show_ui(ui, items).response)
    }
}

impl Stock for ComboBox {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let spacing = &sizer.style.spacing;
        let row = Row {
            pieces: vec![
                Piece::Text(WidgetText::from(self.selected.as_str())),
                Piece::Fixed(Vec2::splat(spacing.icon_width)),
            ],
            gap: spacing.icon_spacing,
            frame: sizer.button_frame(),
            min_size: vec2(spacing.combo_width, spacing.interact_size.y),
            wrap: TextWrapMode::Wrap,
        };
        sizer.lay_out(row, width).outer
    }
}

/// How many lines a text edit shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lines {
    /// One line that scrolls sideways.
    Single,
    /// Wrapped text, at least `min` rows tall. With `max`, the field stops
    /// growing at that many rows and scrolls instead.
    Multi {
        /// Rows shown when the text is shorter.
        min: usize,
        /// Rows after which the field scrolls; unbounded when `None`.
        max: Option<usize>,
    },
}

/// Whether a one-line field shows what is typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Masking {
    Shown,
    Hidden,
}

/// `egui::TextEdit`, single- or multi-line, with optional hint text.
///
/// Holds the text as it is when the frame is built: the height of a
/// multi-line field follows its content.
#[derive(Clone, Debug)]
pub struct TextEdit {
    text: String,
    hint: Option<String>,
    lines: Lines,
    masking: Masking,
}

impl TextEdit {
    /// A one-line field showing `text`.
    pub fn singleline(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hint: None,
            lines: Lines::Single,
            masking: Masking::Shown,
        }
    }

    /// A wrapping field showing `text`, `min` rows tall at least.
    pub fn multiline(text: impl Into<String>, min: usize) -> Self {
        Self {
            text: text.into(),
            hint: None,
            lines: Lines::Multi { min, max: None },
            masking: Masking::Shown,
        }
    }

    /// Stops a multi-line field growing at the height of a field `max` rows
    /// tall (egui's `desired_rows`). It scrolls instead.
    pub fn max_rows(mut self, max: usize) -> Self {
        if let Lines::Multi { min, .. } = self.lines {
            self.lines = Lines::Multi {
                min,
                max: Some(max.at_least(min)),
            };
        }
        self
    }

    /// Shows dots for what is typed, like a password field. A one-line
    /// field's size does not depend on its text, so the size is unchanged.
    pub fn masked(mut self) -> Self {
        self.masking = Masking::Hidden;
        self
    }

    /// Greyed text shown while the field is empty.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    fn rows(&self) -> usize {
        match self.lines {
            Lines::Single => 1,
            Lines::Multi { min, .. } => min.at_least(1),
        }
    }

    /// Adds the field to `ui`, filling its rect, editing `text`. Focus and
    /// the cursor follow the leaf's identity.
    pub fn show(&self, ui: &mut Ui, text: &mut String) -> Response {
        let rect = ui.max_rect();
        let id = ui.make_persistent_id("text");
        let mut edit = match self.lines {
            Lines::Single => egui::TextEdit::singleline(text),
            Lines::Multi { .. } => egui::TextEdit::multiline(text).desired_rows(self.rows()),
        }
        .id(id)
        .password(self.masking == Masking::Hidden)
        .desired_width(rect.width());
        if let Some(hint) = &self.hint {
            edit = edit.hint_text(hint.as_str());
        }
        match self.lines {
            Lines::Multi { max: Some(_), .. } => {
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(id.with("scroll"))
                        .auto_shrink(false)
                        .show(ui, |ui| ui.add(edit))
                        .inner
                })
                .inner
            }
            _ => place(ui, edit),
        }
    }
}

impl Stock for TextEdit {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        let sizer = Sizer::new(ctx, text);
        let spacing = &sizer.style.spacing;
        let frame = text_edit_frame(&sizer);
        let margin = frame.total_margin().sum();
        let line = sizer.line_height();
        let allocate = width.map_or(spacing.text_edit_width, f32::from);
        let inner_width = allocate - margin.x;
        let min_inner_height = self.rows() as f32 * line;
        let min_size = vec2(
            allocate,
            (min_inner_height + margin.y).at_least(spacing.interact_size.y),
        );

        let inner = match (&self.hint, self.text.is_empty()) {
            (Some(hint), true) => {
                let wrap = match self.lines {
                    Lines::Single => TextWrapMode::Truncate,
                    Lines::Multi { .. } => TextWrapMode::Wrap,
                };
                let hint = RichText::new(hint.as_str());
                sizer.galley(hint.into(), wrap, inner_width).size()
            }
            _ => {
                let galley = text_edit_galley(&sizer, &self.text, self.lines, inner_width, line);
                let mut size = galley.size();
                size.y = size.y.at_least(min_inner_height);
                if self.lines == Lines::Single {
                    size.x = size.x.at_most(inner_width);
                }
                size
            }
        };
        let outer = (inner + margin).at_least(min_size);
        match self.lines {
            Lines::Multi { max: Some(max), .. } => outer.at_most(vec2(
                f32::INFINITY,
                (max as f32 * line + margin.y).at_least(min_size.y),
            )),
            _ => outer,
        }
    }
}

/// The frame of a `TextEdit` in its inactive state.
fn text_edit_frame(sizer: &Sizer<'_>) -> Frame {
    let visuals = sizer.style.visuals.widgets.inactive;
    Frame {
        inner_margin: sizer.style.spacing.button_padding.into(),
        ..Frame::default()
    }
    .apply_stroke_and_expansion_without_layout_shift(visuals.bg_stroke, visuals.expansion)
}

/// The galley egui's default text edit layouter makes.
fn text_edit_galley(
    sizer: &Sizer<'_>,
    text: &str,
    lines: Lines,
    wrap_width: f32,
    line_height: f32,
) -> std::sync::Arc<egui::Galley> {
    let font = sizer.font.clone();
    let mut job = match lines {
        Lines::Single => egui::text::LayoutJob::simple_singleline(
            text.to_owned(),
            font,
            egui::Color32::PLACEHOLDER,
        ),
        Lines::Multi { .. } => egui::text::LayoutJob::simple(
            text.to_owned(),
            font,
            egui::Color32::PLACEHOLDER,
            wrap_width,
        ),
    };
    job.keep_trailing_whitespace = true;
    for section in &mut job.sections {
        section.format.line_height = Some(line_height);
    }
    sizer.ctx.fonts_mut(|fonts| fonts.layout_job(job))
}
