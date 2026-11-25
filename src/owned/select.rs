//! The owned select: an input-like trigger that opens a list of options.
//! The list is an overlay anchored to the trigger, and the select owns whether
//! it is open: in egui's temp data, keyed by the trigger.

use std::rc::Rc;

use egui::accesskit::HasPopup;
use egui::{Context, Id, Key, Role, WidgetInfo};

use super::choice::{self, Choice, IntoChoice};
use super::number::{hold_arrows, pressed};
use super::options::Refused;
use super::{
    Chevron, Enablement, Mark, OnResponse, check, describe, options, options::Gate,
    options::IntoTip, options::Validity, tipped,
};
use crate::Aria;
use crate::builder::Slot;
use crate::contract::{SelectStyle, Size};
use crate::focus::{Autofocus, Select};
use crate::overlay::Fit;
use crate::overlay::{Anchor, Distance, Focus, MatchWidth, Nav, Outside, Placement, Shown, Wrap};
use crate::overlay_chain::OverlayChain;
use crate::sheet::{Key as SheetKey, Part as SheetPart, Sheet, When};
use crate::{Builder, NodeUi as _, Scope, Tw, nav, tw};

/// The parts of a select's list.
#[derive(Hash, Debug, Clone, Copy)]
enum Part {
    List,
    Box,
    Rows,
    Group,
    Option(usize),
}

/// TODO(prose): a select's anatomy for the [`Sheet`],
/// as shadcn's `select.tsx` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectPart {
    /// TODO(prose): the box that shows the chosen option.
    Trigger,
    /// TODO(prose): the chosen option's label in the trigger.
    Value,
    /// TODO(prose): the chevron in the trigger.
    TriggerIcon,
    /// TODO(prose): the box of the open list.
    Content,
    /// TODO(prose): the options of the list, together.
    Group,
    /// TODO(prose): one option of the list.
    Item,
    /// TODO(prose): the room an option keeps for its check mark.
    ItemIndicator,
    /// TODO(prose): the check mark of the chosen option.
    ItemIndicatorIcon,
}

impl SheetPart for SelectPart {
    const SOURCE: &'static str = "select.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Trigger => "select-trigger",
            Self::Value => "select-value",
            Self::TriggerIcon => "select-trigger-icon",
            Self::Content => "select-content",
            Self::Group => "select-group",
            Self::Item => "select-item",
            Self::ItemIndicator => "select-item-indicator",
            Self::ItemIndicatorIcon => "select-item-indicator-icon",
        }
    }
}

/// Writes the select's recorded exception over the rules at the sheet's
/// current layer: a size the trigger's rules leave out keeps its metrics.
pub(crate) fn write_select_exceptions(sheet: &mut Sheet, select: &SelectStyle) {
    for size in select.sizes.keys() {
        if sheet
            .unnamed(SelectPart::Trigger, &[SheetKey::Size(size)])
            .is_none()
        {
            continue;
        }
        let m = *select.sizes.get(size);
        sheet.rule_when(
            SelectPart::Trigger,
            When::always().size(size),
            move |t, _| {
                let t = t
                    .h_px(m.height)
                    .px_px(m.padding_x)
                    .gap_px(m.gap)
                    .rounded_px(f32::from(m.radius));
                match m.text {
                    Some(text) => t.text_size(text.size, text.line_height),
                    None => t,
                }
            },
        );
    }
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
/// drawn from the [`Style`](crate::Style). Start one with [`Builder::select`] and finish it
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
    gate: Gate,
    classes: Tw,
    enablement: Enablement,
    size: Option<Size>,
    on_response: OnResponse<A>,
    autofocus: Autofocus,
    validity: Validity,
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
            gate: Gate::default(),
            classes: tw(),
            enablement: Enablement::Enabled,
            size: None,
            on_response: OnResponse::default(),
            autofocus: Autofocus::Never,
            validity: Validity::Valid,
        }
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): marks the select invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::prim::text_field::TextFieldChain::invalid)
    /// does: its trigger's border and ring turn destructive.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.validity = Validity::of(error);
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
    /// The style's state looks still apply over them in their states: see
    /// [the module docs](crate::owned#classes-and-the-styles-state-looks).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control is
    /// disabled, as an HTML `title` is. Rich text keeps egui's formatting. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.gate.tooltip(text);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box, such as text and a [`kbd`](crate::Builder::kbd) chip.
    /// The content takes no input, and its text describes the control to
    /// assistive technology.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut crate::Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.gate.tooltip_with(build);
        self
    }

    /// Runs `then` with the trigger's response each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Whether the select takes input.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.enablement = enablement;
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the last
    /// call wins.
    pub fn refused(mut self, reason: impl IntoTip) -> Self {
        self.gate.refuse(reason);
        self
    }

    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    pub fn focus_on_open(mut self) -> Self {
        self.autofocus = Autofocus::OnOverlayOpen;
        self
    }

    /// TODO(prose): takes focus on every frame this is set.
    pub fn focus_now(mut self) -> Self {
        self.autofocus = Autofocus::Now(Select::None);
        self
    }

    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        options::when_some(self, value, then)
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
            gate,
            classes: caller,
            enablement,
            size,
            on_response,
            autofocus,
            validity,
        } = self;
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let ctx = b.ctx().clone();
        let trigger = slot.id;
        if enablement == Enablement::Disabled && Listing::of(&ctx, trigger) == Listing::Open {
            Listing::Closed.store(&ctx, trigger);
        }
        let style = b.style().clone();
        let offset = offset.unwrap_or(style.menu.offset);
        let mut keys = vec![SheetKey::Size(b.size_of(size))];
        if enablement == Enablement::Disabled {
            keys.push(SheetKey::Aria(Aria::Disabled));
        }
        keys.extend(validity.key(enablement));
        keys.extend(super::expanded_key(
            Listing::of(&ctx, trigger) == Listing::Open,
        ));
        let own = tw().sense_click();
        let own = match enablement {
            Enablement::Enabled => own,
            Enablement::Disabled => own.inert(),
        };
        let classes = b.part(SelectPart::Trigger, &keys, own.merge(caller));
        let value_classes = b.part(SelectPart::Value, &[], tw());
        let icon = b.part(SelectPart::TriggerIcon, &[], tw());
        let current = options.iter().position(|option| option.value == chosen);
        let shown = current
            .and_then(|at| options.get(at))
            .map_or_else(String::new, |option| option.label.clone());
        let name = name.unwrap_or_default();
        let value = shown.clone();
        let label = name.clone();
        b.div_id_with(&slot, classes, |b| {
            b.text(value_classes, shown);
            b.glyph(icon, Chevron::Down.icon());
        })
        .open_focus(autofocus.open_focus())
        .draw(move |app, ui| {
            let response = tipped(ui, ui.node_response(), &tooltip);
            validity.describe(ui.ctx(), response.id);
            autofocus.apply(ui.ctx(), &response);
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
                .nav(Nav::Listbox(Wrap::Stop))
                .outside(Outside::Swallow)
                .role(Role::ListBox)
                .named(name)
                .on_dismiss(move |_| Listing::Closed.store(&close, trigger))
                .children_fitted(|s, fit| options.show(s, &choose, fit));
        }
    }
}

type Choose<A, T> = Rc<dyn Fn(&mut A, &Context, T)>;

/// The options of an open list.
struct List<T> {
    options: Vec<Choice<T>>,
    current: Option<usize>,
    trigger: Id,
    list: Id,
    offset: Distance,
}

impl<T: Copy + 'static> List<T> {
    /// Where the list goes: under the trigger, flipping above it for want of room.
    fn placement(&self) -> Placement {
        Placement::below().gap(self.offset)
    }

    fn show<'f, A: 'static>(&self, s: &mut Scope<'_, 'f, A>, choose: &Choose<A, T>, fit: Fit) {
        let content = s.part(SelectPart::Content, &[], tw().flex().flex_col().min_h_0());
        let group = s.part(SelectPart::Group, &[], tw().flex().flex_col());
        super::fitted_box(s, fit, (Part::Box, Part::Rows), content, |s| {
            s.div_id(Part::Group, group, |s| self.rows(s, choose, fit));
        });
    }

    fn rows<'f, A: 'static>(&self, s: &mut Scope<'_, 'f, A>, choose: &Choose<A, T>, fit: Fit) {
        let enabled = |at: usize| {
            self.options.get(at).is_some_and(|option| {
                option.gated(Enablement::Enabled, Refused::No).0 == Enablement::Enabled
            })
        };
        let start = self
            .current
            .filter(|&at| enabled(at))
            .or_else(|| (0..self.options.len()).find(|&at| enabled(at)));
        // Scrolled to once the list is drawn, in the container it is drawn in.
        if let Some(at) = start.filter(|_| fit != Fit::Measuring) {
            let item = s.id_of(Part::Option(at));
            nav::highlight_initially(s.ctx(), s.kept(), self.list, item, self.trigger);
        }
        let ctx = s.ctx().clone();
        for (index, option) in self.options.iter().enumerate() {
            let (enablement, tooltip) = option.gated(Enablement::Enabled, Refused::No);
            let marked = Some(index) == self.current;
            let (label, value) = (option.label.clone(), option.value);
            let keys: &[SheetKey] = match enablement {
                Enablement::Enabled => &[],
                Enablement::Disabled => &[SheetKey::Aria(Aria::Disabled)],
            };
            let own = enablement.senses();
            let item = s.part(SelectPart::Item, keys, own.focusable_none());
            let indicator = s.part(SelectPart::ItemIndicator, &[], tw());
            let mark = s.part(SelectPart::ItemIndicatorIcon, &[], tw());
            let row = s
                .div_id(Part::Option(index), item, |s| {
                    s.text(tw(), label.clone());
                    s.div(indicator, |s| {
                        if marked {
                            s.glyph(mark, check());
                        }
                    });
                })
                .role(Role::ListBoxOption)
                .named(label.clone());
            let mut row = row.draw(move |_, ui| {
                describe(
                    &ui.node_response(),
                    enablement,
                    Role::ListBoxOption,
                    Some(Mark::from(marked)),
                    &label,
                );
            });
            if let Some(tip) = tooltip {
                row = row.tip(tip);
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
