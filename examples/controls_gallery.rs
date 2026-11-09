//! The text edits in their own leaves, with the size the library computed for
//! each outlined against the size egui gave it, and the owned controls below.
//!
//! A visual cross-check of the size computation in `bullseye::widget`: each
//! cell outlines the leaf's rect (what layout reserved) in blue and the rect
//! egui returned when the widget was drawn into it in orange. They agree to
//! within a point, the slack of snapping a rect's edges to physical pixels one
//! at a time; beyond that the cell is outlined in red and says by how much.
//! Drag a cell's handle or use the slider to change the width the fields are
//! given, and vary the text size, `button_padding`, `interact_size` and the
//! hint text, which is how the computation drifts when it is wrong.
//!
//! The buttons, choices and values sections show every control built from
//! bullseye nodes, which have no size computation to check.

#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeMap;

use bullseye::owned;
use bullseye::prim::text_field::{self, Field};
use bullseye::style::{Mark, variant};
use bullseye::widget::{Lines, TextEdit};
use bullseye::{Builder, NodeUi as _, TextStyle, Tw, tw};
use egui::{Color32, Context, Rect, Response, Stroke, StrokeKind, Ui, UiBuilder, vec2};

type B<'b, 'f> = Builder<'b, 'f, State>;

const COMPUTED: Color32 = Color32::from_rgb(0x2a, 0x6f, 0xdb);
const DRAWN: Color32 = Color32::from_rgb(0xe0, 0x8a, 0x00);
const FLAG: Color32 = Color32::from_rgb(0xd0, 0x20, 0x20);

/// How far the rect egui allocated may be from the leaf's, in points, before
/// a cell is flagged. Drawing snaps each edge of a layout rect to a physical
/// pixel: an edge moves by half a pixel, and so the rect's size by up to a
/// whole one, which egui's unsnapped widget does not follow.
pub const TOLERANCE: f32 = 1.0;

/// A cell's width, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellWidth(f32);

impl CellWidth {
    const MIN: f32 = 80.0;
    const MAX: f32 = 320.0;

    /// `points` within the range the controls offer; NaN gives the minimum.
    pub fn new(points: f32) -> Self {
        Self(points.clamp(Self::MIN, Self::MAX))
    }
}

impl State {
    /// The width a leaf in a stage is offered: the cell's less the stage's
    /// padding and border.
    fn offered_width(&self) -> f32 {
        f32::from(self.cell_width) - 2.0 * STAGE_INSET
    }
}

/// A stage's padding (`p-1`, 4 points) and border (1 point).
const STAGE_INSET: f32 = 5.0;

impl From<CellWidth> for f32 {
    fn from(width: CellWidth) -> f32 {
        width.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextSize {
    Small,
    Body,
    Large,
}

impl TextSize {
    pub const ALL: [(Self, &'static str); 3] =
        [(Self::Small, "12"), (Self::Body, "14"), (Self::Large, "22")];

    fn points(self) -> f32 {
        match self {
            Self::Small => 12.0,
            Self::Body => 14.0,
            Self::Large => 22.0,
        }
    }
}

/// `Spacing::button_padding`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Padding {
    Tight,
    Standard,
    Roomy,
}

impl Padding {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Tight, "0, 0"),
        (Self::Standard, "16, 8"),
        (Self::Roomy, "12, 6"),
    ];

    fn size(self) -> egui::Vec2 {
        match self {
            Self::Tight => vec2(0.0, 0.0),
            Self::Standard => vec2(16.0, 8.0),
            Self::Roomy => vec2(12.0, 6.0),
        }
    }
}

/// `Spacing::interact_size`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interact {
    Small,
    Standard,
    Tall,
}

impl Interact {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Small, "24, 14"),
        (Self::Standard, "40, 36"),
        (Self::Tall, "64, 48"),
    ];

    fn size(self) -> egui::Vec2 {
        match self {
            Self::Small => vec2(24.0, 14.0),
            Self::Standard => vec2(40.0, 36.0),
            Self::Tall => vec2(64.0, 48.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Label {
    Short,
    Long,
    Wrapping,
}

impl Label {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Short, "short"),
        (Self::Long, "long"),
        (Self::Wrapping, "wrapping"),
    ];

    fn text(self) -> &'static str {
        match self {
            Self::Short => "OK",
            Self::Long => "A label of moderate length",
            Self::Wrapping => {
                "A label long enough that it has to wrap onto several lines in a narrow cell"
            }
        }
    }
}

/// The text inputs the gallery checks, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Control {
    Edit,
    EditMulti,
    Field,
    FieldMulti,
}

impl Control {
    pub const ALL: [Self; 4] = [Self::Edit, Self::EditMulti, Self::Field, Self::FieldMulti];

    fn name(self) -> &'static str {
        match self {
            Self::Edit => "widget::TextEdit",
            Self::EditMulti => "widget::TextEdit, multi-line",
            Self::Field => "prim::text_field",
            Self::FieldMulti => "prim::text_field, multi-line",
        }
    }
}

/// What a cell's leaf reserved and what egui's widget took.
#[derive(Clone, Copy, Debug)]
pub struct Report {
    pub computed: Rect,
    pub drawn: Rect,
}

impl Report {
    /// How far the rects' edges are apart, if that is more than
    /// [`TOLERANCE`].
    pub fn mismatch(&self) -> Option<egui::Vec2> {
        let off = |a: Rect, b: Rect| {
            let min = (a.min - b.min).abs();
            let max = (a.max - b.max).abs();
            vec2(min.x.max(max.x), min.y.max(max.y))
        };
        let off = off(self.computed, self.drawn);
        (off.x > TOLERANCE || off.y > TOLERANCE).then_some(off)
    }
}

pub struct State {
    pub cell_width: CellWidth,
    pub text_size: TextSize,
    pub padding: Padding,
    pub interact: Interact,
    pub label: Label,
    /// The last frame's report for each cell.
    pub reports: BTreeMap<Control, Report>,
    pub section: Section,
    pub single: String,
    pub multi: String,
    pub field_single: String,
    pub field_multi: String,
    pub on: bool,
    pub unit: Unit,
    /// What the owned section's checkbox, toggle, radio group and switch hold.
    pub owned_checked: bool,
    pub owned_toggled: bool,
    pub owned_radio: Unit,
    /// How often an owned button was clicked.
    pub owned_clicks: u32,
    /// What the owned slider, select and stepper hold.
    pub owned_slider: f64,
    pub owned_select: usize,
    pub owned_stepper: f64,
}

/// The size of the window the example opens with.
pub const WINDOW: egui::Vec2 = egui::vec2(1280.0, 800.0);

/// What the gallery's main area shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    /// Each text input in a cell outlined with its computed and drawn rects.
    Cells,
    /// The owned buttons: every variant enabled and disabled, and the toggle.
    Buttons,
    /// The owned checkbox, radio group, switch and segmented control.
    Choices,
    /// The owned slider, select and stepper.
    Values,
}

impl Section {
    const ALL: [(Self, &'static str); 4] = [
        (Self::Cells, "cells"),
        (Self::Buttons, "buttons"),
        (Self::Choices, "choices"),
        (Self::Values, "values"),
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    Metric,
    Imperial,
}

impl Default for State {
    fn default() -> Self {
        Self {
            cell_width: CellWidth::new(200.0),
            text_size: TextSize::Body,
            padding: Padding::Standard,
            interact: Interact::Standard,
            label: Label::Long,
            reports: BTreeMap::new(),
            single: "Some text".into(),
            multi: "Several lines of text that wrap\nonto more than one row".into(),
            field_single: String::new(),
            field_multi: "Grows with its content, up to a cap".into(),
            on: true,
            unit: Unit::Metric,
            section: Section::Cells,
            owned_checked: true,
            owned_toggled: true,
            owned_radio: Unit::Metric,
            owned_clicks: 0,
            owned_slider: 40.0,
            owned_select: 0,
            owned_stepper: 3.0,
        }
    }
}

/// Tints the preset with the examples' accent, then sets the egui style the
/// widgets are sized and drawn with.
pub fn prepare(ctx: &Context, state: &State) {
    common::use_accent(ctx, state);
    ctx.all_styles_mut(|style| {
        style.spacing.button_padding = state.padding.size();
        style.spacing.interact_size = state.interact.size();
    });
}

/// The base text of the gallery's cells.
fn cell_text(state: &State) -> Tw {
    tw().role(common::sized(state.text_size.points()))
}

/// Whether a widget may be larger than its leaf.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Overflow {
    /// It may not: its rect is what egui allocated.
    Never,
    /// It scrolls inside the leaf, so egui allocates the whole content and
    /// the part inside the leaf is what is compared.
    Scrolls,
}

/// Draws the control with `show`, then asks egui what `probe` allocates when
/// the same widget is added to a `Ui` as it would be on its own, as wide as
/// the width the leaf was offered. `show` places the widget in the leaf,
/// which fills it whatever the widget's size is, so only the probe can say
/// how large egui makes it. The probe is invisible and keeps its ids to
/// itself. It is given the width offered, for the widgets that take one.
fn observed(
    state: &mut State,
    ui: &mut Ui,
    control: Control,
    overflow: Overflow,
    show: impl FnOnce(&mut State, &mut Ui),
    probe: impl FnOnce(&mut Ui, f32) -> Rect,
) {
    let computed = ui.max_rect();
    show(state, ui);
    let offered = state.offered_width();
    let room = Rect::from_min_size(computed.min, vec2(offered, computed.height()));
    let natural = ui
        .scope_builder(
            UiBuilder::new().id_salt("probe").max_rect(room).invisible(),
            |ui| probe(ui, offered),
        )
        .inner;
    let drawn = match overflow {
        Overflow::Never => natural,
        Overflow::Scrolls => natural.intersect(computed),
    };
    let report = Report { computed, drawn };
    let painter = ui.ctx().layer_painter(ui.layer_id());
    painter.rect_stroke(
        computed.expand(1.0),
        0.0,
        Stroke::new(1.0, COMPUTED),
        StrokeKind::Outside,
    );
    painter.rect_stroke(drawn, 0.0, Stroke::new(1.0, DRAWN), StrokeKind::Inside);
    if report.mismatch().is_some() {
        painter.rect_stroke(
            computed.union(drawn).expand(3.0),
            2.0,
            Stroke::new(2.0, FLAG),
            StrokeKind::Outside,
        );
    }
    state.reports.insert(control, report);
}

/// The leaf of `control`, built from the state's content.
fn leaf(b: &mut B<'_, '_>, state: &State, control: Control) {
    use Overflow::{Never, Scrolls};
    let label = state.label.text();
    let fill = tw().w_full();
    match control {
        Control::Edit => {
            let text = state.single.clone();
            let field = TextEdit::singleline(&text).hint(label).named(label);
            b.widget(control, fill, field, move |s, ui, w| {
                observed(
                    s,
                    ui,
                    control,
                    Never,
                    |s, ui| {
                        w.show(ui, &mut s.single);
                    },
                    |ui, width| natural_edit(ui, &text, Lines::Single, width, Some(label)).rect,
                );
            });
        }
        Control::EditMulti => {
            let text = state.multi.clone();
            let field = TextEdit::multiline(&text, 2).hint(label).named(label);
            b.widget(control, fill, field, move |s, ui, w| {
                observed(
                    s,
                    ui,
                    control,
                    Never,
                    |s, ui| {
                        w.show(ui, &mut s.multi);
                    },
                    |ui, width| natural_edit(ui, &text, MULTI, width, Some(label)).rect,
                );
            });
        }
        Control::Field => {
            let text = state.field_single.clone();
            let field = Field::singleline(&text).hint(label);
            let measured = field.clone();
            b.leaf(
                control,
                fill,
                move |m| text_field::measure(m, &measured),
                move |s, ui| {
                    observed(
                        s,
                        ui,
                        control,
                        Never,
                        |s, ui| {
                            text_field::show(ui, &mut s.field_single, &field);
                        },
                        |ui, width| natural_edit(ui, &text, Lines::Single, width, Some(label)).rect,
                    );
                },
            );
        }
        Control::FieldMulti => {
            let text = state.field_multi.clone();
            let field = Field::multiline(&text, 1).max_rows(FIELD_ROWS).named(label);
            let measured = field.clone();
            b.leaf(
                control,
                fill,
                move |m| text_field::measure(m, &measured),
                move |s, ui| {
                    observed(
                        s,
                        ui,
                        control,
                        Scrolls,
                        |s, ui| {
                            text_field::show(ui, &mut s.field_multi, &field);
                        },
                        |ui, width| natural_edit(ui, &text, ONE_ROW, width, None).rect,
                    );
                },
            );
        }
    }
}

const MULTI: Lines = Lines::Multi { min: 2, max: None };
const ONE_ROW: Lines = Lines::Multi { min: 1, max: None };

/// Pads an input as the preset says inputs are padded, which egui shares with
/// buttons.
fn input_padded(ui: &mut Ui) {
    if let Some(tokens) = bullseye::style::tokens(ui.ctx()) {
        ui.spacing_mut().button_padding = tokens.input_padding;
    }
}

/// A text edit as egui sizes it on its own, `width` wide.
fn natural_edit(ui: &mut Ui, text: &str, lines: Lines, width: f32, hint: Option<&str>) -> Response {
    let mut buffer = text.to_owned();
    let mut edit = match lines {
        Lines::Single => egui::TextEdit::singleline(&mut buffer),
        Lines::Multi { min, .. } => egui::TextEdit::multiline(&mut buffer).desired_rows(min),
    }
    .desired_width(width);
    if let Some(hint) = hint {
        edit = edit.hint_text(hint);
    }
    ui.scope(|ui| {
        input_padded(ui);
        ui.add(edit)
    })
    .inner
}

/// Rows after which the multi-line field scrolls.
const FIELD_ROWS: usize = 4;

/// What a cell's caption says about its report.
fn status(state: &State, control: Control, muted: Color32) -> (String, Color32) {
    match state.reports.get(&control) {
        None => (String::new(), muted),
        Some(report) => match report.mismatch() {
            None => ("ok".to_owned(), muted),
            Some(off) => (format!("MISMATCH {:.1} x {:.1}", off.x, off.y), FLAG),
        },
    }
}

/// A thin handle at a stage's right edge that drags the cell width.
///
/// ```html
/// <div class="w-2 rounded cursor-col-resize bg-border hover:bg-ring"></div>
/// ```
fn handle(b: &mut B<'_, '_>, control: Control) {
    let t = common::tokens(b.ctx());
    b.div_id(
        ("handle", control),
        tw().w_px(8.0)
            .rounded()
            .bg(t.border)
            .hover(|p| p.bg(t.ring))
            .sense_drag()
            .focusable_none(),
        |state: &mut State, ui| {
            let response = ui.node_response();
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if response.dragged() {
                let dx = response.drag_delta().x;
                state.cell_width = CellWidth::new(f32::from(state.cell_width) + dx);
            }
        },
        |_| {},
    );
}

/// One control: its name and status, then a stage of the cell width with the
/// control's leaf in it and a handle beside it.
///
/// ```html
/// <div class="flex flex-col gap-1 p-2 border border-border rounded-xl bg-card">
///   <div class="flex justify-between gap-2"><span>widget::Button</span><span class="text-muted-foreground">ok</span></div>
///   <div class="flex gap-1">
///     <div class="flex flex-col items-start w-[200px] p-1 bg-muted border border-border">…leaf…</div>
///     <div class="w-2 rounded bg-border"></div>
///   </div>
/// </div>
/// ```
fn cell(b: &mut B<'_, '_>, state: &State, control: Control) {
    let t = common::tokens(b.ctx());
    let (status, colour) = status(state, control, t.muted_foreground);
    b.div(
        tw().flex()
            .flex_col()
            .gap(1.0)
            .p(2.0)
            .border(1.0)
            .border_color(t.border)
            .rounded_px(f32::from(t.radius))
            .bg(t.card)
            .text(t.card_foreground),
        |b| {
            b.div(tw().flex().justify_between().gap(2.0), |b| {
                b.text(tw(), control.name());
                b.text(tw().text(colour), status);
            });
            b.div(tw().flex().gap(1.0), |b| {
                b.div(
                    cell_text(state)
                        .flex()
                        .flex_col()
                        .items_start()
                        .w_px(f32::from(state.cell_width))
                        .p(1.0)
                        .bg(t.muted)
                        .border(1.0)
                        .border_color(t.border),
                    |b| leaf(b, state, control),
                );
                handle(b, control);
            });
        },
    );
}

/// The controls that vary what the cells are given.
///
/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]">
///   <input type="range"> + four rows of choices, then the legend
/// </aside>
/// ```
fn controls(b: &mut B<'_, '_>, state: &State) {
    common::captioned(b, "Section", |b| {
        common::choice(b, "section", state.section, &Section::ALL, |s| {
            &mut s.section
        });
    });
    common::slider_row(
        b,
        "width",
        "Cell width (points)",
        f32::from(state.cell_width),
        CellWidth::MIN..=CellWidth::MAX,
        |s| &mut s.cell_width.0,
    );
    common::captioned(b, "Text size", |b| {
        common::choice(b, "text_size", state.text_size, &TextSize::ALL, |s| {
            &mut s.text_size
        });
    });
    common::captioned(b, "button_padding", |b| {
        common::choice(b, "padding", state.padding, &Padding::ALL, |s| {
            &mut s.padding
        });
    });
    common::captioned(b, "interact_size", |b| {
        common::choice(b, "interact", state.interact, &Interact::ALL, |s| {
            &mut s.interact
        });
    });
    common::captioned(b, "Label", |b| {
        common::choice(b, "label", state.label, &Label::ALL, |s| &mut s.label);
    });
    legend(b);
}

/// ```html
/// <div class="flex flex-col gap-1 text-muted-foreground">
///   <span><span class="text-[#2a6fdb]">blue</span>: the rect the library computed</span>
///   <span><span class="text-[#e08a00]">orange</span>: the rect egui returned</span>
///   <span><span class="text-[#d02020]">red</span>: they differ by more than 1 point</span>
/// </div>
/// ```
fn legend(b: &mut B<'_, '_>) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        for (colour, name, meaning) in [
            (COMPUTED, "Blue", "the rect the library computed"),
            (DRAWN, "Orange", "the rect egui returned"),
            (FLAG, "Red", "they differ by more than a point"),
        ] {
            b.div(tw().flex().items_start().gap(2.0), |b| {
                b.rule(tw().w_px(10.0).h_px(10.0).mt(1.5).bg(colour));
                b.text(tw().text(muted), format!("{name}: {meaning}"));
            });
        }
    });
}

/// ```html
/// <div class="flex size-full">
///   <aside>…controls…</aside>
///   <div class="flex flex-wrap content-start gap-3 p-3 grow">…one cell per control…</div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    common::with_controls(
        b,
        |b| controls(b, state),
        |b| match state.section {
            Section::Cells => b.div(
                tw().flex()
                    .flex_wrap()
                    .content_start()
                    .items_start()
                    .gap(3.0)
                    .p(3.0),
                |b| {
                    for control in Control::ALL {
                        cell(b, state, control);
                    }
                },
            ),
            Section::Buttons => buttons_section(b, state),
            Section::Choices => choices_section(b, state),
            Section::Values => values_section(b, state),
        },
    );
}

const VARIANTS: [(&str, bullseye::contract::Variant); 5] = [
    ("primary", variant::PRIMARY),
    ("secondary", variant::SECONDARY),
    ("destructive", variant::DESTRUCTIVE),
    ("outline", variant::OUTLINE),
    ("ghost", variant::GHOST),
];

const ENABLEMENTS: [owned::Enablement; 2] =
    [owned::Enablement::Enabled, owned::Enablement::Disabled];

/// `name`, with ", disabled" when `enablement` says so.
fn named(name: &str, enablement: owned::Enablement) -> String {
    match enablement {
        owned::Enablement::Enabled => name.to_owned(),
        owned::Enablement::Disabled => format!("{name}, disabled"),
    }
}

/// One row: a name, then the control.
fn row<'f>(b: &mut B<'_, 'f>, name: &str, control: impl FnOnce(&mut B<'_, 'f>)) {
    b.div(tw().flex().items_center().gap(4.0), |b| {
        b.div(
            tw().w_px(130.0)
                .shrink_0()
                .text(common::tokens(b.ctx()).muted_foreground),
            |b| {
                b.text(tw(), name);
            },
        );
        b.div(tw().w_px(170.0).shrink_0().flex().items_center(), control);
    });
}

/// A column of rows.
///
/// ```html
/// <div class="flex flex-col gap-3 p-3">
///   <div class="flex gap-4"><span class="w-[130px]">name</span> control</div>
///   …
/// </div>
/// ```
fn rows<'f>(b: &mut B<'_, 'f>, rows: impl FnOnce(&mut B<'_, 'f>)) {
    b.div(tw().flex().flex_col().gap(3.0).p(3.0), rows);
}

/// A row per button variant, enabled and disabled, then the toggle.
fn buttons_section(b: &mut B<'_, '_>, state: &State) {
    rows(b, |b| {
        for enablement in ENABLEMENTS {
            for (name, variant) in VARIANTS {
                row(b, &named(name, enablement), |b| {
                    owned::Button::new("Button")
                        .variant(variant)
                        .enablement(enablement)
                        .show(b, ("button", name, enablement), |s: &mut State| {
                            s.owned_clicks += 1;
                        });
                });
            }
        }
        row(b, "toggle", |b| {
            owned::Button::toggle("Toggle", Mark::from(state.owned_toggled)).show(
                b,
                "toggle",
                |s: &mut State| s.owned_toggled = !s.owned_toggled,
            );
        });
    });
}

/// The checkbox, radio group, switch and segmented control.
fn choices_section(b: &mut B<'_, '_>, state: &State) {
    rows(b, |b| {
        for enablement in ENABLEMENTS {
            row(b, &named("checkbox", enablement), |b| {
                owned::Checkbox::new("Accept terms", Mark::from(state.owned_checked))
                    .enablement(enablement)
                    .show(b, ("checkbox", enablement), |s: &mut State| {
                        s.owned_checked = !s.owned_checked;
                    });
            });
        }
        row(b, "radio group", |b| {
            owned::RadioGroup::new(
                state.owned_radio,
                &[(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
            )
            .show(b, "radio", |s: &mut State, unit| s.owned_radio = unit);
        });
        for enablement in ENABLEMENTS {
            row(b, &named("switch", enablement), |b| {
                owned::Switch::new("Airplane mode", Mark::from(state.on))
                    .enablement(enablement)
                    .show(b, ("switch", enablement), |s: &mut State| s.on = !s.on);
            });
        }
        row(b, "segmented", |b| {
            owned::Segmented::new(
                state.unit,
                &[(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
            )
            .show(b, "segmented", |s: &mut State, _, unit| s.unit = unit);
        });
    });
}

/// The slider and select, enabled and disabled, and the stepper.
fn values_section(b: &mut B<'_, '_>, state: &State) {
    rows(b, |b| {
        for enablement in ENABLEMENTS {
            row(b, &named("slider", enablement), |b| {
                owned::Slider::new(state.owned_slider, 0.0..=100.0)
                    .named("Volume")
                    .enablement(enablement)
                    .show(b, ("slider", enablement), |s: &mut State, value| {
                        s.owned_slider = value;
                    });
            });
        }
        for enablement in ENABLEMENTS {
            row(b, &named("select", enablement), |b| {
                owned::Select::new(state.owned_select, &REGION_OPTIONS)
                    .named("Region")
                    .classes(|classes| classes.w_px(150.0))
                    .enablement(enablement)
                    .show(b, ("select", enablement), |s: &mut State, _, region| {
                        s.owned_select = region;
                    });
            });
        }
        for enablement in ENABLEMENTS {
            row(b, &named("stepper", enablement), |b| {
                owned::Stepper::new(state.owned_stepper, 0.0..=10.0)
                    .named("Count")
                    .enablement(enablement)
                    .show(b, ("stepper", enablement), |s: &mut State, value| {
                        s.owned_stepper = value;
                    });
            });
        }
    });
}

const REGIONS: [&str; 3] = ["Europe", "North America", "Asia Pacific"];
const REGION_OPTIONS: [(usize, &str); 3] = [(0, REGIONS[0]), (1, REGIONS[1]), (2, REGIONS[2])];

/// The text style the gallery's layout starts from.
pub fn base_text() -> TextStyle {
    common::base_text(egui::Theme::Light)
}

fn main() -> eframe::Result {
    common::run_app_sized(
        "controls_gallery",
        WINDOW,
        State::default(),
        prepare,
        layout,
    )
}
