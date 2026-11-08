//! Every stock widget and primitive in its own leaf, with the size the library
//! computed for it outlined against the size egui gave it.
//!
//! A visual cross-check of the size estimates in `bullseye::widget`: each cell
//! outlines the leaf's rect (what layout reserved from the estimate) in blue
//! and the rect egui returned when the widget was drawn into it in orange.
//! They agree to within a point, the slack of snapping a rect's edges to
//! physical pixels one at a time; beyond that the cell is outlined in red and
//! says by how much.
//! Drag a cell's handle or use the slider to change the width the widgets are
//! given, and vary the text size, `button_padding`, `interact_size` and the
//! label text, which is how the estimates drift when they are wrong.
//!
//! The widgets the layout sizes (a slider, a text field) take the cell's
//! width; the others are as wide as their content, wrapping at the cell's edge.

#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeMap;

use bullseye::prim::segmented::segmented;
use bullseye::prim::select::{self, Options};
use bullseye::prim::switch::Switch;
use bullseye::prim::text_field::{self, Field};
use bullseye::style::ButtonVariant;
use bullseye::widget::{
    Button, Checkbox, ComboBox, DragValue, Lines, RadioButton, Slider, TextEdit,
};
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

/// The widgets and primitives the gallery shows, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Control {
    Button,
    /// A button of a variant, in a `Ui` that is enabled or not.
    Variant(ButtonVariant, Enablement),
    Checkbox,
    RadioButton,
    Slider,
    DragValue,
    ComboBox,
    TextEdit,
    TextEditMulti,
    TextField,
    TextFieldMulti,
    Switch,
    Select,
    Segmented,
}

/// Whether a cell's widget is usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Enablement {
    Enabled,
    Disabled,
}

impl Enablement {
    fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}

const fn variant(variant: ButtonVariant, enablement: Enablement) -> Control {
    Control::Variant(variant, enablement)
}

impl Control {
    pub const ALL: [Self; 23] = [
        Self::Button,
        variant(ButtonVariant::Primary, Enablement::Enabled),
        variant(ButtonVariant::Primary, Enablement::Disabled),
        variant(ButtonVariant::Secondary, Enablement::Enabled),
        variant(ButtonVariant::Secondary, Enablement::Disabled),
        variant(ButtonVariant::Destructive, Enablement::Enabled),
        variant(ButtonVariant::Destructive, Enablement::Disabled),
        variant(ButtonVariant::Outline, Enablement::Enabled),
        variant(ButtonVariant::Outline, Enablement::Disabled),
        variant(ButtonVariant::Ghost, Enablement::Enabled),
        variant(ButtonVariant::Ghost, Enablement::Disabled),
        Self::Checkbox,
        Self::RadioButton,
        Self::Slider,
        Self::DragValue,
        Self::ComboBox,
        Self::TextEdit,
        Self::TextEditMulti,
        Self::TextField,
        Self::TextFieldMulti,
        Self::Switch,
        Self::Select,
        Self::Segmented,
    ];

    /// The controls that wrap an egui widget in a leaf, whose rect is
    /// checked. A switch and a segmented control are drawn by the library, so
    /// egui has no widget to compare them with.
    pub fn is_checked(self) -> bool {
        !matches!(self, Self::Switch | Self::Segmented)
    }

    fn name(self) -> String {
        let name = match self {
            Self::Variant(variant, enablement) => {
                let enablement = match enablement {
                    Enablement::Enabled => "",
                    Enablement::Disabled => ", disabled",
                };
                return format!("widget::Button, {variant:?}{enablement}");
            }
            Self::Button => "widget::Button",
            Self::Checkbox => "widget::Checkbox",
            Self::RadioButton => "widget::RadioButton",
            Self::Slider => "widget::Slider",
            Self::DragValue => "widget::DragValue",
            Self::ComboBox => "widget::ComboBox",
            Self::TextEdit => "widget::TextEdit",
            Self::TextEditMulti => "widget::TextEdit, multi-line",
            Self::TextField => "prim::text_field",
            Self::TextFieldMulti => "prim::text_field, multi-line",
            Self::Switch => "prim::switch",
            Self::Select => "prim::select",
            Self::Segmented => "prim::segmented",
        };
        name.to_owned()
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
    pub checked: bool,
    pub selected: bool,
    pub level: f64,
    pub amount: f64,
    pub pick: usize,
    pub single: String,
    pub multi: String,
    pub field_single: String,
    pub field_multi: String,
    pub on: bool,
    pub choice: usize,
    pub unit: Unit,
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
            checked: true,
            selected: true,
            level: 30.0,
            amount: 12.5,
            pick: 0,
            single: "Some text".into(),
            multi: "Several lines of text that wrap\nonto more than one row".into(),
            field_single: String::new(),
            field_multi: "Grows with its content, up to a cap".into(),
            on: true,
            choice: 0,
            unit: Unit::Metric,
        }
    }
}

/// Sets the egui style the widgets are sized and drawn with.
pub fn prepare(ctx: &Context, state: &State) {
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
        Control::Button => b.widget(control, tw(), Button::new(label), move |s, ui, w| {
            observed(
                s,
                ui,
                control,
                Never,
                |_, ui| {
                    w.show(ui);
                },
                |ui, _| ui.add(egui::Button::new(label)).rect,
            );
        }),
        Control::Variant(variant, enablement) => b.widget(
            control,
            tw(),
            Button::new(label).variant(variant),
            move |s, ui, w| {
                observed(
                    s,
                    ui,
                    control,
                    Never,
                    |_, ui| {
                        ui.add_enabled_ui(enablement.is_enabled(), |ui| w.show(ui));
                    },
                    |ui, _| ui.add(egui::Button::new(label)).rect,
                );
            },
        ),
        Control::Checkbox => b.widget(control, tw(), Checkbox::new(label), move |s, ui, w| {
            observed(
                s,
                ui,
                control,
                Never,
                |s, ui| {
                    w.show(ui, &mut s.checked);
                },
                |ui, _| ui.add(egui::Checkbox::new(&mut false, label)).rect,
            );
        }),
        Control::RadioButton => {
            b.widget(control, tw(), RadioButton::new(label), move |s, ui, w| {
                observed(
                    s,
                    ui,
                    control,
                    Never,
                    |s, ui| {
                        if w.show(ui, s.selected).clicked() {
                            s.selected = !s.selected;
                        }
                    },
                    |ui, _| ui.add(egui::RadioButton::new(false, label)).rect,
                );
            });
        }
        Control::Slider => b.widget(
            control,
            fill,
            Slider::new().named(label),
            move |s, ui, w| {
                observed(
                    s,
                    ui,
                    control,
                    Never,
                    |s, ui| {
                        w.show(ui, &mut s.level, 0.0..=100.0);
                    },
                    |ui, width| {
                        ui.spacing_mut().slider_width = width;
                        ui.add(egui::Slider::new(&mut 0.0, 0.0..=100.0).show_value(false))
                            .rect
                    },
                );
            },
        ),
        Control::DragValue => {
            b.widget(
                control,
                tw(),
                DragValue::new(DRAG_TEMPLATE).named(label),
                move |s, ui, w| {
                    observed(
                        s,
                        ui,
                        control,
                        Never,
                        |s, ui| {
                            w.show(ui, &mut s.amount, -100.0..=100.0);
                        },
                        |ui, _| {
                            ui.add(egui::DragValue::new(&mut -100.0).fixed_decimals(2))
                                .rect
                        },
                    );
                },
            );
        }
        Control::ComboBox => b.widget(
            control,
            tw(),
            ComboBox::new(label).named(label),
            move |s, ui, w| {
                observed(
                    s,
                    ui,
                    control,
                    Never,
                    |_, ui| {
                        w.show(ui, |ui| {
                            ui.label(label);
                        });
                    },
                    |ui, _| combo(ui, label).rect,
                );
            },
        ),
        Control::TextEdit => {
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
        Control::TextEditMulti => {
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
        Control::TextField => {
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
        Control::TextFieldMulti => {
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
        Control::Switch => b.widget(control, tw(), Switch::new(label), move |s, ui, w| {
            w.show(ui, &mut s.on);
        }),
        Control::Select => {
            let options = Options::new(select_labels(label)).named(label);
            let measured = options.clone();
            b.leaf(
                control,
                tw(),
                move |m| select::measure(m, &measured),
                move |s, ui| {
                    observed(
                        s,
                        ui,
                        control,
                        Never,
                        |s, ui| {
                            select::show(ui, &mut s.choice, &options);
                        },
                        |ui, _| widest_combo(ui, &select_labels(label)),
                    );
                },
            );
        }
        Control::Segmented => {
            let classes = common::segmented_classes(b.ctx());
            segmented(
                b,
                control,
                classes.container(),
                state.unit,
                &[(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")],
                |cell| classes.cell(cell),
                |s| &mut s.unit,
            )
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

/// The text of the widest value a drag value is sized for.
const DRAG_TEMPLATE: &str = "-100.00";

/// Rows after which the multi-line field scrolls.
const FIELD_ROWS: usize = 4;

fn select_labels(label: &'static str) -> [&'static str; 3] {
    [label, "Second option", "Third"]
}

/// A combo box header showing `selected`, as egui sizes it on its own.
fn combo(ui: &mut Ui, selected: &str) -> Response {
    ui.scope(|ui| {
        input_padded(ui);
        egui::ComboBox::from_id_salt(selected)
            .selected_text(selected)
            .show_ui(ui, |_| {})
            .response
    })
    .inner
}

/// The rect of the header of each of `labels`, each added at the same place:
/// the select is as large as its largest.
fn widest_combo(ui: &mut Ui, labels: &[&str]) -> Rect {
    let room = ui.max_rect();
    let size = labels
        .iter()
        .map(|label| {
            ui.scope_builder(UiBuilder::new().id_salt(label).max_rect(room), |ui| {
                combo(ui, label).rect.size()
            })
            .inner
        })
        .fold(egui::Vec2::ZERO, egui::Vec2::max);
    Rect::from_min_size(room.min, size)
}

/// What a cell's caption says about its report.
fn status(state: &State, control: Control, muted: Color32) -> (String, Color32) {
    match (control.is_checked(), state.reports.get(&control)) {
        (false, _) => ("drawn by the library".to_owned(), muted),
        (true, None) => (String::new(), muted),
        (true, Some(report)) => match report.mismatch() {
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
        |b| {
            b.div(
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
            );
        },
    );
}

/// The text style the gallery's layout starts from.
pub fn base_text() -> TextStyle {
    common::base_text(egui::Theme::Light)
}

fn main() -> eframe::Result {
    common::run_app_styled("controls_gallery", State::default(), prepare, layout)
}
