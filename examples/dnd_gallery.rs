//! A telemetry gallery: square panels in 1, 2 or 3 columns inside a scroll
//! area, each with a live trace. Dragging a panel reorders in two dimensions,
//! the panels reflowing across rows as the pointer crosses them; the drag
//! image is the panel itself, or a chip. Image files dropped from the OS
//! insert a panel at the pointer. Near the top or bottom of the viewport a
//! drag scrolls. 1/2/3 set the columns, G swaps the drag image, Esc cancels.
//!
//! The gallery is a scroll container (`b.scroll`), which scrolls itself when a
//! drag is near its edge.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;

use eggshell_ui::{File, Files, LayoutState, Opacity, tw};
use eggshell_ui_shadcn::DragChip as _;
use egui::{Context, Key, Ui, vec2};

type B<'b, 'f, A> = eggshell_ui::Builder<'b, 'f, A>;

const SLIDE: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PanelKey(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Image {
    Chip,
    Live,
}

pub struct State {
    pub panels: Vec<(PanelKey, String)>,
    pub cols: usize,
    pub image: Image,
    /// What the last drag or drop did, shown in the toolbar.
    pub status: String,
    /// A panel to scroll into view: one dropped with no position, which went
    /// to the end.
    pub reveal: Option<PanelKey>,
    next: u32,
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            panels: Vec::new(),
            cols: 3,
            image: Image::Live,
            status: String::new(),
            reveal: None,
            next: 0,
        };
        for _ in 0..12 {
            state.add_panel(None, format!("w{}", state.next));
        }
        state
    }
}

impl State {
    pub fn add_panel(&mut self, at: Option<usize>, label: String) -> PanelKey {
        let key = PanelKey(self.next);
        self.next += 1;
        let at = at.unwrap_or(self.panels.len()).min(self.panels.len());
        self.panels.insert(at, (key, label));
        key
    }

    fn move_panel(&mut self, key: PanelKey, to: Option<usize>) {
        if let (Some(from), Some(to)) = (self.panels.iter().position(|p| p.0 == key), to) {
            let last = self.panels.len() - 1;
            common::dnd::move_to(&mut self.panels, from, to.min(last));
        }
    }

    fn insert_files(&mut self, files: &Files, at: Option<usize>) {
        let mut at = at;
        for file in files.iter() {
            let name = file_name(file);
            let key = self.add_panel(at, name.clone());
            self.status = format!("accepted {name}");
            at = at.map(|i| i + 1);
            if at.is_none() {
                self.reveal = Some(key);
            }
        }
    }
}

fn file_name(file: &File) -> String {
    file.path()
        .and_then(|path| path.file_name())
        .map_or(String::new(), |name| name.to_string_lossy().into())
}

/// Whether the gallery takes a file: images only.
pub fn accepts(file: &File) -> bool {
    matches!(file.extension().as_deref(), Some("png" | "jpg" | "jpeg"))
}

/// The panel in its slot: the slot is outlined while the panel is carried
/// away by a live image. The face is the drag source.
fn cell(b: &mut B<'_, '_, State>, key: PanelKey, label: &str, state: &State, lifted: bool) {
    let t = common::tokens(b.ctx());
    let outer = tw()
        .w_frac(1.0, state.cols as f32)
        .aspect_square()
        .p(1.0)
        .animate_layout(SLIDE);
    let outer = match (lifted, state.image) {
        (true, Image::Live) => outer.rounded_md().border(2.0).border_color(t.ring),
        _ => outer,
    };
    let face = tw()
        .flex()
        .items_center()
        .justify_center()
        .w_full()
        .h_full()
        .rounded_md()
        .border(1.0)
        .border_color(t.input)
        .bg(t.muted)
        .text(t.foreground);
    // With a chip, the lifted face stays in its slot and is dimmed there. The
    // live image is the face itself, so it keeps its look at rest.
    let face = match state.image {
        Image::Chip => face.drag_lifted(|p| p.opacity(Opacity::new(0.35))),
        Image::Live => face,
    };
    b.div_id(key, outer, |b| {
        let source = b
            .div_id("face", face, |b| {
                b.div(
                    tw().flex().flex_col().gap(1.0).p(2.0).w_full().h_full(),
                    |b| {
                        let now = b.ctx().input(|i| i.time);
                        b.div(tw().flex().justify_between().items_center(), |b| {
                            b.text(tw().role(common::sized(12.0)), label);
                            b.text(
                                tw().role(common::sized(12.0)).text(t.muted_foreground),
                                format!("{:+.2}", channel(key, now, 1.0)),
                            );
                        });
                        trace(b, key, t.primary, t.border);
                    },
                );
            })
            .drag_source(key);
        if state.image == Image::Chip {
            source.with_drag_chip(label);
        }
    });
}

/// A telemetry channel's value at `time` seconds, `x` from 0 (oldest) to 1
/// (now): a few sines whose frequencies differ per panel.
fn channel(key: PanelKey, time: f64, x: f32) -> f32 {
    let seed = key.0 as f32;
    let t = time as f32 - (1.0 - x) * 2.0;
    0.55 * (t * (1.3 + 0.37 * seed)).sin()
        + 0.30 * (t * (3.1 + 0.53 * seed) + seed).sin()
        + 0.15 * (t * (7.7 + 0.11 * seed) + 2.0 * seed).sin()
}

/// The last two seconds of the channel, scrolling left, drawn as a 2 pt line
/// over a hairline zero baseline.
fn trace(b: &mut B<'_, '_, State>, key: PanelKey, line: egui::Color32, baseline: egui::Color32) {
    const POINTS: usize = 64;
    b.custom("trace", tw().w_full().grow())
        .ui(move |_: &mut State, ui| {
            let rect = ui.max_rect();
            let now = ui.input(|i| i.time);
            let painter = ui.painter();
            painter.hline(
                rect.x_range(),
                rect.center().y,
                egui::Stroke::new(1.0, baseline),
            );
            let points: Vec<egui::Pos2> = (0..POINTS)
                .map(|i| {
                    let x = i as f32 / (POINTS - 1) as f32;
                    let y = channel(key, now, x);
                    egui::pos2(
                        rect.lerp_inside(vec2(x, 0.5)).x,
                        rect.center().y - y * rect.height() * 0.45,
                    )
                })
                .collect();
            painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, line)));
            ui.ctx().request_repaint();
        });
}

fn placeholder(b: &mut B<'_, '_, State>, cols: usize) {
    let t = common::tokens(b.ctx());
    b.div_id(
        "insert",
        tw().w_frac(1.0, cols as f32)
            .aspect_square()
            .p(1.0)
            .animate_layout(SLIDE),
        |b| {
            b.div(
                tw().w_full()
                    .h_full()
                    .rounded_md()
                    .border(2.0)
                    .border_color(t.ring),
                |_| {},
            )
        },
    );
}

pub const GRID: &str = "/\"page\"/\"grid\"";

pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    let carried = b.dragging::<PanelKey>();
    let arriving = b.dragging::<Files>();
    b.on_drag_end::<PanelKey>(|app, key, end| app.status = format!("{key:?}: {end:?}"));

    b.scroll("page", tw().size_full())
        .children(|b| {
            let grid = b.id_of("grid");
            let mut order: Vec<&(PanelKey, String)> = state.panels.iter().collect();
            if let Some(drag) = &carried
                && let Some(to) = drag.index_hint_in(grid)
                && let Some(from) = order.iter().position(|p| p.0 == *drag.payload)
            {
                common::dnd::move_to(&mut order, from, to);
            }
            let gap = arriving.and_then(|files| files.index_hint_in(grid));
            let lifted = carried.as_ref().map(|drag| *drag.payload);
            b.div_id(
                "grid",
                tw().flex().flex_row().flex_wrap().w_full().p(2.0),
                |b| {
                    for (slot, (key, label)) in order.into_iter().enumerate() {
                        if gap == Some(slot) {
                            placeholder(b, state.cols);
                        }
                        cell(b, *key, label, state, lifted == Some(*key));
                    }
                    if gap.is_some_and(|at| at >= state.panels.len()) {
                        placeholder(b, state.cols);
                    }
                },
            )
            .drop_target::<PanelKey>(|app, drop| {
                app.move_panel(*drop.payload, drop.index_hint);
            })
            .drop_target_if::<Files>(
                |files| files.iter().all(accepts),
                |app, drop| app.insert_files(&drop.payload, drop.index_hint),
            )
            .drop_fallback();
        })
        .drop_target::<Files>(|app, drop| {
            let names: Vec<String> = drop.payload.iter().map(file_name).collect();
            app.status = format!("refused {}", names.join(", "));
        })
        .drop_fallback();
}

/// One frame: the build, layout and draw over the central area, then the
/// reveal of a panel dropped with no position. Shared by `main` and the
/// tests.
pub fn frame(ui: &mut Ui, app: &mut State, layout_state: &mut LayoutState) {
    let ctx = ui.ctx().clone();
    let placed = layout_state
        .build(&ctx, |b| layout(b, app))
        .layout(ui.max_rect());
    if let Some(key) = app.reveal.take() {
        let name = format!("/{key:?}");
        if let Some(id) = placed
            .nodes()
            .find(|node| node.path().ends_with(&name))
            .and_then(|panel| panel.id())
        {
            eggshell_ui::scroll::scroll_into_view(&ctx, id, Some(egui::Align::Center));
        } else {
            app.reveal = Some(key);
        }
    }
    placed.draw(app, ui);
}

/// The strip above the gallery: the column count, the drag image, and what
/// the last drag did.
///
/// ```html
/// <div class="flex items-center gap-3 px-3 h-11 border-b">
///   <span>Columns</span> <div role="radiogroup">1 2 3</div>
///   <span>Drag image</span> <div role="radiogroup">Live Chip</div>
///   <span>status</span>
/// </div>
/// ```
pub fn toolbar(b: &mut B<'_, '_, State>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .items_center()
            .gap(3.0)
            .px(3.0)
            .size_full()
            .border_b(1.0)
            .border_color(t.border)
            .text(t.foreground),
        |b| {
            b.text(tw().text(t.muted_foreground), "Columns");
            b.segmented("columns", state.cols, [(1, "1"), (2, "2"), (3, "3")])
                .on_change(|state: &mut State, cols| state.cols = cols);
            b.text(tw().text(t.muted_foreground), "Drag image");
            b.segmented(
                "image",
                state.image,
                [(Image::Live, "Live"), (Image::Chip, "Chip")],
            )
            .on_change(|state: &mut State, image| state.image = image);
            b.text(tw().text(t.muted_foreground), state.status.clone());
        },
    );
}

fn shortcuts(ctx: &Context, state: &mut State) {
    for (key, cols) in [(Key::Num1, 1), (Key::Num2, 2), (Key::Num3, 3)] {
        if ctx.input(|i| i.key_pressed(key)) {
            state.cols = cols;
        }
    }
    if ctx.input(|i| i.key_pressed(Key::G)) {
        state.image = match state.image {
            Image::Chip => Image::Live,
            Image::Live => Image::Chip,
        };
    }
}

/// The window below the theme bar: the toolbar's own eggshell tree, then the
/// gallery's. `main`, the tests and the fuzzer all draw it, so they exercise
/// the same arrangement of trees.
pub fn window(
    ui: &mut Ui,
    state: &mut State,
    gallery: &mut LayoutState,
    toolbar_layout: &mut LayoutState,
) {
    let ctx = ui.ctx().clone();
    egui::Panel::top("gallery_toolbar")
        .exact_size(44.0)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
        .show(ui, |ui| {
            let area = ui.max_rect();
            toolbar_layout.set_base_text(Some(common::base_text_in(&ctx)));
            toolbar_layout
                .build(&ctx, |b| toolbar(b, state))
                .layout(area)
                .draw(state, ui);
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(common::tokens(&ctx).background))
        .show(ui, |ui| frame(ui, state, gallery));
}

fn main() -> eframe::Result {
    let mut state = State::default();
    let mut layout_state = LayoutState::new();
    let mut toolbar_state = LayoutState::new();
    let mut theme_bar = common::ThemeBar::new();
    common::launch("dnd_gallery", None, move |ui| {
        let ctx = ui.ctx().clone();
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout_state.set_base_text(Some(common::base_text_in(&ctx)));
        shortcuts(&ctx, &mut state);
        window(ui, &mut state, &mut layout_state, &mut toolbar_state);
    })
}
