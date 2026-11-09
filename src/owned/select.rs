//! The owned select: an input-like trigger that opens a list of options.
//! The list is egui's popup until bullseye has overlays of its own.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{
    Align, Context, FontId, Frame, Key, Layout, Popup, Role, Sense, SetOpenCommand, Stroke, Ui,
    Vec2, WidgetInfo, vec2,
};

use super::number::{hold_keys, pressed};
use super::{Chevron, Enablement, paint_check, paint_chevron};
use crate::{Builder, NodeUi as _, Style, Tw, tw};

/// A trigger that shows the chosen option and opens the list of `options`,
/// drawn from the [`Style`].
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
#[derive(Clone, Debug)]
pub struct Select<T> {
    current: T,
    options: Vec<(T, String)>,
    name: Option<String>,
    classes: Tw,
    enablement: Enablement,
}

impl<T: Copy + PartialEq + Hash + Debug + 'static> Select<T> {
    /// A select over `(value, label)` options of which `current` is chosen.
    pub fn new(current: T, options: &[(T, &str)]) -> Self {
        Self {
            current,
            options: options
                .iter()
                .map(|&(value, label)| (value, label.to_owned()))
                .collect(),
            name: None,
            classes: tw(),
            enablement: Enablement::Enabled,
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

    /// Whether the select takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the select to `b` under `key`. `choose` gets the application and
    /// the context, for a value the context holds.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        choose: impl Fn(&mut A, &Context, T) + Copy + 'static,
    ) {
        let style = Style::of(b.ctx());
        let select = &style.select;
        let metrics = select.metrics;
        let classes = select
            .trigger
            .classes(&style, metrics.radius, self.enablement, select.transition)
            .flex()
            .items_center()
            .justify_between()
            .h_px(metrics.height)
            .px(metrics.padding_x / 4.0)
            .gap(metrics.gap / 4.0)
            .role(style.role(&select.text).clone())
            .whitespace_nowrap()
            .w_fit();
        let classes = self.enablement.fade(classes, &style).merge(self.classes);
        let current = self
            .options
            .iter()
            .position(|(value, _)| *value == self.current);
        let shown = current.map_or_else(String::new, |at| self.options[at].1.clone());
        let name = self.name.unwrap_or_default();
        let (chevron, opacity) = (select.chevron, select.chevron_opacity);
        let list = List {
            style: style.clone(),
            options: self.options,
            current,
            name: name.clone(),
        };
        b.div_id(
            key,
            classes,
            move |app, ui| {
                let response = ui.node_response();
                if let Some(chosen) = list.show(ui, &response) {
                    choose(app, ui.ctx(), list.options[chosen].0);
                }
                let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
                response.widget_info(|| WidgetInfo {
                    current_text_value: Some(list.shown().to_owned()),
                    ..WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &name)
                });
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_expanded(open));
            },
            |b| {
                b.text(tw(), shown);
                b.paint(
                    tw().size_px(chevron).shrink_0().opacity(opacity),
                    paint_chevron(Chevron::Down),
                );
            },
        );
    }
}

/// The options and what drives their popup.
struct List<T> {
    style: std::sync::Arc<Style>,
    options: Vec<(T, String)>,
    current: Option<usize>,
    name: String,
}

impl<T> List<T> {
    fn shown(&self) -> &str {
        self.current.map_or("", |at| self.options[at].1.as_str())
    }

    /// Opens, drives and draws the popup under `trigger`; the option chosen
    /// with the pointer or the keyboard this frame, if any.
    fn show(&self, ui: &Ui, trigger: &egui::Response) -> Option<usize> {
        let ctx = ui.ctx();
        let popup_id = Popup::default_response_id(trigger);
        let highlight_id = popup_id.with("highlight");
        let open = Popup::is_id_open(ctx, popup_id);
        let last = self.options.len().saturating_sub(1);
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
                    highlight = (highlight + 1).min(last);
                }
                if pressed(ui, Key::ArrowUp) {
                    highlight = highlight.saturating_sub(1);
                }
                if pressed(ui, Key::Home) {
                    highlight = 0;
                }
                if pressed(ui, Key::End) {
                    highlight = last;
                }
            } else if pressed(ui, Key::ArrowDown) {
                command = Some(SetOpenCommand::Bool(true));
                highlight = self.current.unwrap_or(0);
            }
        }
        let confirmed = open
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
        let width = trigger.rect.width();
        let inner = Popup::from_response(trigger)
            .open_memory(command)
            .gap(4.0)
            .frame(frame)
            .layout(Layout::top_down_justified(Align::Min))
            .accessible_name(&self.name)
            .show(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.set_min_width(width - 2.0 * (select.popup_padding + popup.border_width));
                self.rows(ui, &mut highlight)
            })
            .and_then(|popup| popup.inner);
        ctx.data_mut(|data| data.insert_temp(highlight_id, highlight));
        chosen.or(inner)
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
        for (index, (_, label)) in self.options.iter().enumerate() {
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
            let moved = ui.input(|input| input.pointer.delta() != Vec2::ZERO);
            if response.hovered() && moved {
                *highlight = index;
            }
            let chosen = Some(index) == self.current;
            let colors = if *highlight == index {
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
            response.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, chosen, label));
            if response.clicked() {
                clicked = Some(index);
            }
        }
        clicked
    }
}
