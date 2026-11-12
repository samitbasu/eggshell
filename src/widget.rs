//! The text edit: `egui::TextEdit` as a leaf, with a size the library computes.
//!
//! [`TextEdit`] holds the content that decides its size (the text, a hint, the
//! row count) and has a pure `size` from that content, the egui
//! [`Style`](egui::Style) and the text galley: it never runs the widget.
//! `show` places the real egui widget in exactly the leaf's content rect.
//! Use it through [`Builder::widget`](crate::Builder::widget), which keeps the
//! measure and the draw on the same value, or through
//! [`prim::text_field`](crate::prim::text_field):
//!
//! ```
//! # use eggshell_ui::{Builder, tw, widget::TextEdit};
//! # fn view(b: &mut Builder<'_, '_, String>) {
//! b.widget("name", tw(), TextEdit::singleline(""), |name, ui, field| {
//!     field.show(ui, name);
//! });
//! # }
//! ```
//!
//! The field is drawn from the [`Style`]'s input section, mapped onto the
//! node's `Ui` by [`Style::map_input`]; the size reads the same values.
//! `tests/it/sizing.rs` asserts that the size equals what egui allocates when
//! it draws the widget.

use egui::{Context, Frame, Margin, NumExt as _, Response, RichText, TextWrapMode, Ui, Vec2, vec2};
use taffy::Size;

use crate::sizing::Sizer;
use crate::{Measure, Style, TextStyle, Width};

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

/// Gives the widget its accessible name, the `aria-label` of the web.
fn announce(response: &Response, name: Option<&str>) {
    if let Some(name) = name {
        response
            .ctx
            .accesskit_node_builder(response.id, |node| node.set_label(name));
    }
}

fn place(ui: &mut Ui, widget: impl egui::Widget) -> Response {
    ui.put(ui.max_rect(), widget)
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

/// Whether a text edit draws its frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Framing {
    Framed,
    Bare,
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
    framing: Framing,
    name: Option<String>,
}

impl TextEdit {
    /// The text the field was built with.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// A one-line field showing `text`.
    pub fn singleline(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hint: None,
            lines: Lines::Single,
            masking: Masking::Shown,
            framing: Framing::Framed,
            name: None,
        }
    }

    /// A wrapping field showing `text`, `min` rows tall at least.
    pub fn multiline(text: impl Into<String>, min: usize) -> Self {
        Self {
            text: text.into(),
            hint: None,
            lines: Lines::Multi { min, max: None },
            masking: Masking::Shown,
            framing: Framing::Framed,
            name: None,
        }
    }

    /// Makes the field wrap its text over at least `min` rows.
    pub fn multiline_rows(mut self, min: usize) -> Self {
        self.lines = Lines::Multi { min, max: None };
        self
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

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Shows dots for what is typed, like a password field. A one-line
    /// field's size does not depend on its text, so the size is unchanged.
    pub fn masked(mut self) -> Self {
        self.masking = Masking::Hidden;
        self
    }

    /// Draws no frame, margin or background: the field is its text, for a
    /// field that sits inside a box of the application's own.
    pub fn bare(mut self) -> Self {
        self.framing = Framing::Bare;
        self
    }

    /// Greyed text shown while the field is empty.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The egui id of the field a leaf's `ui` holds.
    pub fn id_in(ui: &Ui) -> egui::Id {
        ui.make_persistent_id("text")
    }

    fn rows(&self) -> usize {
        match self.lines {
            Lines::Single => 1,
            Lines::Multi { min, .. } => min.at_least(1),
        }
    }

    /// Adds the field to `ui`, filling its rect, editing `text`. Focus and
    /// the cursor follow the leaf's identity.
    ///
    /// The leaf was sized from the text as it was when the frame was built.
    /// A keystroke that changes the field's height asks egui for another
    /// pass, so the frame that is painted is laid out for the new text
    /// instead of showing one row too many or too few for a frame.
    pub fn show(&self, ui: &mut Ui, text: &mut String) -> Response {
        Style::of(ui.ctx()).map_input(ui.style_mut());
        if self.framing == Framing::Bare {
            // egui floors a single-line field at interact_size.y, which
            // map_input set to input.height; a bare field is its leaf's height.
            ui.spacing_mut().interact_size.y = 0.0;
        }
        let rect = ui.max_rect();
        let id = Self::id_in(ui);
        let width = Width::new(rect.width());
        let before = self.fit(&Sizer::for_ui(ui), text, Some(width));
        let before_text = text.clone();
        let mut edit = match self.lines {
            Lines::Single => egui::TextEdit::singleline(text),
            Lines::Multi { .. } => egui::TextEdit::multiline(text).desired_rows(self.rows()),
        }
        .id(id)
        .password(self.masking == Masking::Hidden)
        .desired_width(rect.width());
        match (self.framing, self.lines) {
            (Framing::Bare, _) => edit = edit.frame(Frame::NONE),
            (Framing::Framed, _) => {
                edit = edit.margin(Margin::from(Sizer::for_ui(ui).contract.input.padding));
            }
        }
        if self.lines == Lines::Single {
            edit = edit
                .align(egui::Align2::LEFT_CENTER)
                .min_size(vec2(0.0, rect.height()));
        }
        if let Some(hint) = &self.hint {
            edit = edit.hint_text(hint.as_str());
        }
        let response = match self.lines {
            Lines::Multi { max: Some(_), .. } if before.scrolls() => {
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(id.with("scroll"))
                        .auto_shrink(false)
                        .min_scrolled_height(0.0)
                        .show(ui, |ui| ui.add(edit))
                        .inner
                })
                .inner
            }
            _ => place(ui, edit),
        };
        announce(&response, self.name.as_deref());
        if response.changed() && *text != before_text {
            let after = self.fit(&Sizer::for_ui(ui), text, Some(width));
            if after.size.y != before.size.y {
                ui.ctx().request_discard("text field changed height");
            }
        }
        response
    }

    /// The size the field takes with `text` in it, and how tall its text
    /// would be if the cap did not stop it.
    fn fit(&self, sizer: &Sizer<'_>, text: &str, width: Option<Width>) -> Fit {
        let spacing = &sizer.style.spacing;
        let input = &sizer.contract.input;
        let (frame, floor) = match self.framing {
            Framing::Framed => (
                Frame::new().inner_margin(Margin::from(input.padding)),
                input.height,
            ),
            Framing::Bare => (
                Frame::NONE,
                match self.lines {
                    Lines::Single => sizer.role_line_height.unwrap_or(0.0),
                    Lines::Multi { .. } => 0.0,
                },
            ),
        };
        let margin = frame.total_margin().sum();
        let line = sizer.line_height();
        let allocate = width.map_or(spacing.text_edit_width, f32::from);
        let inner_width = allocate - margin.x;
        let min_inner_height = self.rows() as f32 * line;
        let min_size = vec2(allocate, (min_inner_height + margin.y).at_least(floor));

        let inner = match (&self.hint, text.is_empty()) {
            (Some(hint), true) => {
                let wrap = match self.lines {
                    Lines::Single => TextWrapMode::Truncate,
                    Lines::Multi { .. } => TextWrapMode::Wrap,
                };
                let hint = RichText::new(hint.as_str());
                sizer.galley(hint.into(), wrap, inner_width).size()
            }
            _ => {
                let galley = text_edit_galley(sizer, text, self.lines, inner_width, line);
                let mut size = galley.size();
                size.y = size.y.at_least(min_inner_height);
                if self.lines == Lines::Single {
                    size.x = size.x.at_most(inner_width);
                }
                size
            }
        };
        let natural = (inner + margin).at_least(min_size);
        let size = match self.lines {
            Lines::Multi { max: Some(max), .. } => natural.at_most(vec2(
                f32::INFINITY,
                (max as f32 * line + margin.y).at_least(min_size.y),
            )),
            _ => natural,
        };
        Fit { natural, size }
    }
}

/// What a text edit takes, and what it would take without a cap.
struct Fit {
    natural: Vec2,
    size: Vec2,
}

impl Fit {
    /// Whether the cap hides some of the text, which then scrolls.
    fn scrolls(&self) -> bool {
        self.natural.y > self.size.y
    }
}

impl Stock for TextEdit {
    fn size(&self, ctx: &Context, text: &TextStyle, width: Option<Width>) -> Vec2 {
        self.fit(&Sizer::new(ctx, text), &self.text, width).size
    }
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
