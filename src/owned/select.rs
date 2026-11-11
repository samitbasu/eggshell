//! The owned select: an input-like trigger that opens a list of options.
//! The list is egui's popup until eggshell has overlays of its own.

use egui::{
    Align, Color32, Context, FontId, Frame, Key, Layout, Popup, Role, Sense, SetOpenCommand,
    Stroke, Ui, Vec2, WidgetInfo, vec2,
};

use super::choice::{self, Choice, IntoChoice};
use super::number::{hold_keys, pressed};
use super::{Chevron, Enablement, OnResponse, Tip, paint_check, paint_chevron, tipped};
use crate::builder::Slot;
use crate::{Builder, NodeUi as _, Style, Tw, tw};

/// A trigger that shows the chosen option and opens the list of `options`,
/// drawn from the [`Style`]. Start one with [`Builder::select`] and finish it
/// with [`on_change`](Self::on_change).
///
/// With the trigger focused, Space, Enter and ArrowDown open the list. While
/// it is open the arrows (and Home and End) move the highlight, Enter or
/// Space choose the highlighted option, and Escape closes it. Clicking an
/// option chooses it.
///
/// ```html
/// <button role="combobox" class="flex items-center justify-between gap-2 h-9 w-fit px-3 rounded-md
///   border border-input shadow-xs focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50">
///   Europe <svg class="size-4 opacity-50">chevron-down</svg>
/// </button>
/// ```
#[must_use = "a select is not added until you call `.on_change(..)`"]
pub struct SelectChain<'a, 'f, A, T> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    current: T,
    options: Vec<Choice<T>>,
    name: Option<String>,
    tooltip: Tip,
    classes: Tw,
    enablement: Enablement,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> SelectChain<'a, 'f, A, T> {
    /// A select over `options` of which `current` is chosen.
    pub(super) fn new<K: crate::Key, O: IntoChoice<T>>(
        b: Builder<'a, 'f, A>,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            current,
            options: choice::collect(options),
            name: None,
            tooltip: None,
            classes: tw(),
            enablement: Enablement::Enabled,
            on_response: OnResponse::default(),
        }
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the trigger, such as its width,
    /// which is `w-fit` until then; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Shown while the pointer hovers the control, as egui's `on_hover_text`
    /// shows it: with egui's timing and style, and also when the control is
    /// disabled, as an HTML `title` is. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Runs `then` with the trigger's response each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Whether the select takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the select. `choose` runs with the chosen value.
    #[track_caller]
    pub fn on_change(self, choose: impl Fn(&mut A, T) + 'static) {
        self.on_change_with(move |app, _, value| choose(app, value));
    }

    /// Adds the select. `choose` gets the application and the egui context,
    /// for a value the context holds.
    #[track_caller]
    pub fn on_change_with(self, choose: impl Fn(&mut A, &Context, T) + 'static) {
        let Self {
            mut b,
            slot,
            current: chosen,
            options,
            name,
            tooltip,
            classes: caller,
            enablement,
            on_response,
        } = self;
        let enablement = b.enablement_of(enablement);
        let style = Style::of(b.ctx());
        let select = &style.select;
        let metrics = select.metrics;
        let classes = select
            .trigger
            .classes(&style, metrics.radius, enablement, select.transition)
            .flex()
            .items_center()
            .justify_between()
            .h_px(metrics.height)
            .px(metrics.padding_x / 4.0)
            .gap(metrics.gap / 4.0)
            .role(style.role(&select.text).clone())
            .whitespace_nowrap()
            .w_fit();
        let classes = enablement.fade(classes, &style).merge(caller);
        let current = options.iter().position(|option| option.value == chosen);
        let shown = current.map_or_else(String::new, |at| options[at].label.clone());
        let name = name.unwrap_or_default();
        let (chevron, opacity) = (select.chevron, select.chevron_opacity);
        let list = List {
            style: style.clone(),
            options,
            current,
            name: name.clone(),
        };
        b.div_id_with(slot.id, &slot.label, classes, |b| {
            b.text(tw(), shown);
            b.paint(
                tw().size_px(chevron).shrink_0().opacity(opacity),
                paint_chevron(Chevron::Down),
            );
        })
        .draw(move |app, ui| {
            let response = tipped(ui.node_response(), &tooltip);
            if let Some(chosen) = list.show(ui, &response) {
                choose(app, ui.ctx(), list.options[chosen].value);
            }
            on_response.run(app, &response);
            let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
            response.widget_info(|| WidgetInfo {
                current_text_value: Some(list.shown().to_owned()),
                ..WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &name)
            });
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_expanded(open));
        });
    }
}

/// The options and what drives their popup.
struct List<T> {
    style: std::sync::Arc<Style>,
    options: Vec<Choice<T>>,
    current: Option<usize>,
    name: String,
}

impl<T> List<T> {
    fn enabled(&self, at: usize) -> bool {
        self.options[at].enablement == Enablement::Enabled
    }

    /// The nearest enabled option from `at` in the direction of `forward`,
    /// or `at` when there is none.
    fn step(&self, at: usize, forward: bool) -> usize {
        let candidates: Vec<usize> = if forward {
            (at + 1..self.options.len()).collect()
        } else {
            (0..at).rev().collect()
        };
        candidates
            .into_iter()
            .find(|&next| self.enabled(next))
            .unwrap_or(at)
    }

    fn edge(&self, forward: bool) -> usize {
        let mut all = 0..self.options.len();
        let found = if forward {
            all.rfind(|&at| self.enabled(at))
        } else {
            all.find(|&at| self.enabled(at))
        };
        found.unwrap_or(0)
    }

    fn shown(&self) -> &str {
        self.current
            .map_or("", |at| self.options[at].label.as_str())
    }

    /// Opens, drives and draws the popup under `trigger`; the option chosen
    /// with the pointer or the keyboard this frame, if any.
    fn show(&self, ui: &Ui, trigger: &egui::Response) -> Option<usize> {
        let ctx = ui.ctx();
        let popup_id = Popup::default_response_id(trigger);
        let highlight_id = popup_id.with("highlight");
        let open = Popup::is_id_open(ctx, popup_id);
        let mut highlight = ctx
            .data(|data| data.get_temp::<usize>(highlight_id))
            .filter(|_| open)
            .or(self.current)
            .unwrap_or(0);
        let mut command = None;
        let mut chosen = None;
        if trigger.has_focus() {
            hold_keys(ui, open);
            if open {
                if pressed(ui, Key::ArrowDown) {
                    highlight = self.step(highlight, true);
                }
                if pressed(ui, Key::ArrowUp) {
                    highlight = self.step(highlight, false);
                }
                if pressed(ui, Key::Home) {
                    highlight = self.edge(false);
                }
                if pressed(ui, Key::End) {
                    highlight = self.edge(true);
                }
            } else if pressed(ui, Key::ArrowDown) {
                command = Some(SetOpenCommand::Bool(true));
                highlight = self.current.unwrap_or(0);
            }
        }
        let confirmed = open
            && self
                .options
                .get(highlight)
                .is_some_and(|_| self.enabled(highlight))
            && trigger.has_focus()
            && ui.input(|input| input.key_pressed(Key::Enter) || input.key_pressed(Key::Space));
        if confirmed {
            chosen = Some(highlight);
            command = Some(SetOpenCommand::Bool(false));
        } else if trigger.clicked() {
            command = Some(SetOpenCommand::Toggle);
            highlight = self.current.unwrap_or(0);
        }
        let select = &self.style.select;
        let popup = &select.popup;
        let colors = popup.paint.states.rest;
        let frame = Frame::new()
            .fill(colors.fill)
            .stroke(Stroke::new(popup.border_width, colors.border))
            .corner_radius(select.popup_radius)
            .inner_margin(select.popup_padding)
            .shadow(popup.paint.shadow.unwrap_or(egui::Shadow::NONE));
        let widest = self.widest_label(ctx);
        let width = trigger.rect.width();
        let inner = Popup::from_response(trigger)
            .open_memory(command)
            .gap(4.0)
            .frame(frame)
            .layout(Layout::top_down_justified(Align::Min))
            .accessible_name(&self.name)
            .show(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let chrome = 2.0 * (select.popup_padding + popup.border_width);
                let row = 3.0 * select.item_padding.x + select.chevron + widest;
                ui.set_width((width - chrome).max(row));
                self.rows(ui, &mut highlight)
            })
            .and_then(|popup| popup.inner);
        ctx.data_mut(|data| data.insert_temp(highlight_id, highlight));
        chosen.or(inner)
    }

    fn widest_label(&self, ctx: &Context) -> f32 {
        let text = &self.style.text;
        let font = FontId::new(text.size, text.family.clone());
        ctx.fonts_mut(|fonts| {
            self.options
                .iter()
                .map(|option| {
                    fonts
                        .layout_no_wrap(option.label.clone(), font.clone(), Color32::PLACEHOLDER)
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
        })
    }

    /// The option rows; the one clicked.
    fn rows(&self, ui: &mut Ui, highlight: &mut usize) -> Option<usize> {
        let select = &self.style.select;
        let item = select.item;
        let text = &self.style.text;
        let font = FontId::new(text.size, text.family.clone());
        let pad = select.item_padding;
        let height = text.line_height + 2.0 * pad.y;
        let mut clicked = None;
        for (index, option) in self.options.iter().enumerate() {
            let label = &option.label;
            let enabled = self.enabled(index);
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), sense);
            let response = tipped(response, &option.tooltip);
            let moved = ui.input(|input| input.pointer.delta() != Vec2::ZERO);
            if enabled && response.hovered() && moved {
                *highlight = index;
            }
            let chosen = Some(index) == self.current;
            let colors = if !enabled {
                item.paint.states.disabled
            } else if *highlight == index {
                item.paint.states.hover
            } else {
                item.paint.states.rest
            };
            let painter = ui.painter();
            painter.rect_filled(rect, select.item_radius, colors.fill);
            let galley = painter.layout_no_wrap(label.clone(), font.clone(), colors.text);
            let at = egui::pos2(rect.left() + pad.x, rect.center().y - galley.size().y / 2.0);
            painter.galley(at, galley, colors.text);
            if chosen {
                let side = select.chevron;
                let mark = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - pad.x - side / 2.0, rect.center().y),
                    Vec2::splat(side),
                );
                paint_check(painter, mark, colors.text);
            }
            response
                .widget_info(|| WidgetInfo::selected(Role::ListBoxOption, enabled, chosen, label));
            if enabled && response.clicked() {
                clicked = Some(index);
            }
        }
        clicked
    }
}
