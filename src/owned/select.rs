//! The owned select: an input-like trigger that opens a list of options.
//! The list is an overlay anchored to the trigger, and the select owns whether
//! it is open: in egui's temp data, keyed by the trigger.

use std::rc::Rc;
use std::sync::Arc;

use egui::accesskit::HasPopup;
use egui::{Context, Id, Key, Role, WidgetInfo, vec2};

use super::choice::{self, Choice, IntoChoice};
use super::menu::Tone;
use super::number::{hold_arrows, pressed};
use super::{
    Chevron, Enablement, Mark, OnResponse, Tip, describe, paint_check, paint_chevron, tipped,
};
use crate::builder::Slot;
use crate::overlay::{Anchor, Distance, Focus, MatchWidth, Nav, Outside, Placement, Shown, Wrap};
use crate::overlay_chain::OverlayChain;
use crate::pipeline::Room;
use crate::scroll::{Axis, Focusable};
use crate::{Builder, NodeUi as _, Scope, Style, Tw, nav, tw};

/// The parts of a select's list.
#[derive(Hash, Debug, Clone, Copy)]
enum Part {
    List,
    Box,
    Rows,
    Option(usize),
}

/// Whether the list is shown.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Listing {
    #[default]
    Closed,
    Open,
}

impl Listing {
    fn of(ctx: &Context, trigger: Id) -> Self {
        ctx.data(|data| data.get_temp(trigger.with("select-listing")))
            .unwrap_or_default()
    }

    fn store(self, ctx: &Context, trigger: Id) {
        ctx.data_mut(|data| data.insert_temp(trigger.with("select-listing"), self));
        ctx.request_repaint();
    }

    fn toggled(self) -> Self {
        match self {
            Self::Closed => Self::Open,
            Self::Open => Self::Closed,
        }
    }
}

/// A trigger that shows the chosen option and opens the list of `options`,
/// drawn from the [`Style`]. Start one with [`Builder::select`] and finish it
/// with [`on_change`](Self::on_change).
///
/// With the trigger focused, Space, Enter, ArrowDown and ArrowUp open the
/// list. While it is open the arrows (and Home and End) move the highlight,
/// Enter or Space choose the highlighted option, and Escape closes it.
/// Clicking an option chooses it. A list taller than the room it has scrolls.
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
    list: Slot,
    current: T,
    options: Vec<Choice<T>>,
    name: Option<String>,
    offset: Option<Distance>,
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
            list: b.slot(&(key, Part::List)),
            b,
            current,
            options: choice::collect(options),
            name: None,
            offset: None,
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

    /// The gap between the trigger and its list, in place of the style's
    /// [`MenuStyle::offset`](crate::contract::MenuStyle::offset).
    pub fn offset(mut self, offset: Distance) -> Self {
        self.offset = Some(offset);
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
            list,
            current: chosen,
            options,
            name,
            offset,
            tooltip,
            classes: caller,
            enablement,
            on_response,
        } = self;
        let enablement = b.enablement_of(enablement);
        let ctx = b.ctx().clone();
        let trigger = slot.id;
        if enablement == Enablement::Disabled && Listing::of(&ctx, trigger) == Listing::Open {
            Listing::Closed.store(&ctx, trigger);
        }
        let style = Style::of(&ctx);
        let offset = offset.unwrap_or(style.menu.offset);
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
        let value = shown.clone();
        let label = name.clone();
        b.div_id_with(trigger, &slot.label, classes, |b| {
            b.text(tw(), shown);
            b.paint(
                tw().size_px(chevron).shrink_0().opacity(opacity),
                paint_chevron(Chevron::Down),
            );
        })
        .draw(move |app, ui| {
            let response = tipped(ui.node_response(), &tooltip);
            let ctx = ui.ctx();
            hold_arrows(ui);
            let listing = Listing::of(ctx, trigger);
            let arrowed = response.has_focus()
                && listing == Listing::Closed
                && (pressed(ui, Key::ArrowDown) | pressed(ui, Key::ArrowUp));
            if response.clicked() || arrowed {
                listing.toggled().store(ctx, trigger);
            }
            on_response.run(app, &response);
            response.widget_info(|| WidgetInfo {
                current_text_value: Some(value.clone()),
                ..WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &label)
            });
            let open = Listing::of(ctx, trigger) == Listing::Open;
            ctx.accesskit_node_builder(response.id, |node| {
                node.set_expanded(open);
                node.set_has_popup(HasPopup::Listbox);
            });
        });
        {
            let shown = match Listing::of(&ctx, trigger) {
                Listing::Open => Shown::Open,
                Listing::Closed => Shown::Closed,
            };
            let options = List {
                style,
                options,
                current,
                trigger,
                list: list.id,
                offset,
            };
            let choose: Choose<A, T> = Rc::new(choose);
            let close = ctx.clone();
            OverlayChain::new(b.reborrow(), list)
                .anchor(Anchor::Node(trigger))
                .placement(options.placement())
                .shown(shown)
                .match_width(MatchWidth::Anchor)
                .focus(Focus::Stay)
                .nav(Nav::Listbox)
                .wrap(Wrap::Stop)
                .outside(Outside::Swallow)
                .role(Role::ListBox)
                .named(name)
                .on_dismiss(move |_| Listing::Closed.store(&close, trigger))
                .children(|s| options.show(s, &choose));
        }
    }
}

type Choose<A, T> = Rc<dyn Fn(&mut A, &Context, T)>;

/// The options of an open list.
struct List<T> {
    style: Arc<Style>,
    options: Vec<Choice<T>>,
    current: Option<usize>,
    trigger: Id,
    list: Id,
    offset: Distance,
}

impl<T: Copy + 'static> List<T> {
    fn row_height(&self) -> f32 {
        self.style
            .menu
            .row_height(self.style.role(&self.style.menu.text))
    }

    /// The box around the rows.
    fn box_classes(&self) -> Tw {
        let text = self.style.role(&self.style.menu.text);
        self.style.menu.box_classes(text)
    }

    /// Where the list goes: under the trigger, flipping above it for want of room.
    fn placement(&self) -> Placement {
        Placement::below().gap(self.offset)
    }

    /// One row: the highlighted one takes the item's `hover` colours.
    fn row_classes(&self, enablement: Enablement) -> Tw {
        let text = self.style.role(&self.style.menu.text);
        self.style
            .menu
            .row_classes(Tone::Plain, enablement, text)
            .justify_between()
            .focusable_none()
    }

    fn show<'f, A: 'static>(&self, s: &mut Scope<'_, 'f, A>, choose: &Choose<A, T>) {
        let menu = &self.style.menu;
        let rows = self.options.len() as f32 * self.row_height();
        let chrome = 2.0 * (menu.padding + menu.popup.border_width);
        let natural = vec2(0.0, rows + chrome);
        let room = s.room_for(self.trigger, &self.placement(), natural);
        s.div_id(Part::Box, self.box_classes(), |s| match room {
            Room::Enough => self.rows(s, choose),
            Room::Short => {
                let scroller = tw().flex().flex_col().h_px(rows).min_h_0().focusable_none();
                s.scroll(Part::Rows, scroller)
                    .axis(Axis::Y)
                    .focusable(Focusable::Auto)
                    .children(|s| self.rows(s, choose));
            }
        });
    }

    fn rows<'f, A: 'static>(&self, s: &mut Scope<'_, 'f, A>, choose: &Choose<A, T>) {
        let enabled = |at: usize| self.options[at].enablement == Enablement::Enabled;
        let start = self
            .current
            .filter(|&at| enabled(at))
            .or_else(|| (0..self.options.len()).find(|&at| enabled(at)));
        if let Some(at) = start {
            let item = s.id_of(Part::Option(at));
            nav::highlight_initially(s.ctx(), s.kept(), self.list, item, self.trigger);
        }
        let ctx = s.ctx().clone();
        for (index, option) in self.options.iter().enumerate() {
            let enablement = self.options[index].enablement;
            let marked = Some(index) == self.current;
            let (label, value) = (option.label.clone(), option.value);
            let side = self.style.select.chevron;
            let row = s
                .div_id(Part::Option(index), self.row_classes(enablement), |s| {
                    s.text(tw(), label.clone());
                    if marked {
                        s.paint(tw().size_px(side).shrink_0(), paint_check);
                    } else {
                        s.div(tw().size_px(side).shrink_0(), |_| {});
                    }
                })
                .role(Role::ListBoxOption)
                .named(label.clone());
            let mut row = row.draw(move |_, ui| {
                describe(
                    &ui.node_response(),
                    enablement == Enablement::Enabled,
                    Role::ListBoxOption,
                    Some(Mark::from(marked)),
                    &label,
                );
            });
            if let Some(tip) = &option.tooltip {
                row = row.tooltip(tip.clone());
            }
            if enablement == Enablement::Enabled {
                let (choose, close) = (Rc::clone(choose), ctx.clone());
                let trigger = self.trigger;
                row.on_click(move |app| {
                    choose(app, &close, value);
                    Listing::Closed.store(&close, trigger);
                });
            }
        }
    }
}
