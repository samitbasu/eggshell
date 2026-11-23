//! Controls built from eggshell nodes instead of wrapped egui widgets. A
//! control is an identified node with a sense, `hover`, `active` and `focus`
//! variants and text or paint children, styled from [`Style`] and measured
//! by taffy, so there is no size estimate to drift.
//!
//! Every control sets its accessibility role, label and state, is a Tab stop
//! (Space and Enter activate it), and is drawn in the style's explicit
//! disabled colours when disabled. Each builder method names the role a test
//! queries the control by; [`testing`](crate::testing) has the table.
//!
//! # Classes and the style's state looks
//!
//! A control's `classes(..)` merge over the style's **base** classes, the
//! later winning ([`Tw::merge`]). The style's looks for a state (hover,
//! pressed, focus) are variants: they apply on top of the base, and so on
//! top of the caller's classes, while the control is in that state. A
//! caller's `border_color` therefore holds at rest but gives way to a hover
//! look that sets a border, as in CSS, where shadcn's
//! `cn(buttonVariants(..), "border-red-500")` keeps the variant's
//! `hover:border-transparent`. To hold a colour in a state, say it for that
//! state as well:
//!
//! ```html
//! <button class="border border-red-500 hover:border-red-500">Card</button>
//! ```
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use egui::Color32;
//!
//! fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
//!     let edge = Color32::from_rgb(0xef, 0x44, 0x44);
//!     b.button("card", "Card")
//!         .classes(tw().border(1.0).border_color(edge).hover(|p| p.border_color(edge)))
//!         .on_click(|_| {});
//! }
//! ```
//!
//! When the colour follows only whether the control is chosen, carry it in
//! the style variant's on and off looks instead.

mod button;
mod checkbox;
mod choice;
mod choices_sheet;
mod custom;
mod field_box;
mod input_sheet;
mod menu;
mod menu_sheet;
mod number;
mod number_field;
pub(crate) mod options;
mod progress;
mod radio;
mod readout;
mod segmented;
mod select;
mod slider;
mod splitter;
mod stepper;
mod switch;
mod table;
mod table_sheet;
mod tabs_sheet;
mod toggle;
mod toggle_group;
mod tooltip_sheet;

use crate::overlay_input::Blur;
use crate::widget::Enter;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

use egui::{Id, Response, Role, Ui, WidgetInfo};

pub use button::{ButtonChain, ButtonOptions, ButtonPart, Face};
pub(crate) use button::{ButtonDefaults, button_defaults};
pub use checkbox::CheckboxChain;
pub use choice::{Choice, IntoChoice};
pub use choices_sheet::{CheckboxPart, FieldPart, RadioGroupPart, SwitchPart};
pub(crate) use choices_sheet::{
    ChoiceDefaults, Painted as ChoicePaint, choice_defaults, write_checkbox_rules,
    write_exceptions as write_choice_exceptions, write_field_rules, write_radio_rules,
    write_switch_rules,
};
pub use custom::CustomChain;
pub use field_box::FieldBoxChain;
pub(crate) use input_sheet::{
    Frame, keys as input_keys, write_exceptions as write_input_exceptions,
    write_rules as write_input_rules,
};
pub(crate) use input_sheet::{INLINE_END, INLINE_START};
pub use input_sheet::{InputGroupPart, InputPart};
pub use menu::{Action, Checkbox, CommandItem, DropdownChain, ItemChain, Menu, Radio};
pub(crate) use menu::{Menus, context, open_on_secondary_click};
pub use menu_sheet::MenuPart;
pub(crate) use menu_sheet::{
    write_exceptions as write_menu_exceptions, write_rules as write_menu_rules,
};
pub use number::{Edit, Step, ValueRange};
pub use options::IntoTip;
pub(crate) use options::{Gate, Validity};
pub use progress::ProgressChain;
pub use radio::RadioGroupChain;
pub use readout::NumberChain;
pub use segmented::{Cell, SegmentedChain, SegmentedRows};
pub use select::{SelectChain, SelectPart};
pub(crate) use select::{write_select_exceptions, write_select_rules};
pub(crate) use slider::write_exceptions as write_slider_exceptions;
pub(crate) use stepper::write_exceptions as write_stepper_exceptions;

/// Writes the recorded exception of the tracks over a preset's look (#132):
/// a slider's and a progress bar's track are the style's fill (the input
/// token), not shadcn's `bg-muted`, which vanishes on a muted panel; the
/// segmented control's is written with the tabs'.
pub(crate) fn write_track_exceptions(
    sheet: &mut crate::sheet::Sheet,
    (slider, progress): (
        &crate::contract::SliderStyle,
        &crate::contract::ProgressStyle,
    ),
) {
    let (slider, progress) = (slider.track.rest.fill, progress.track.fill);
    sheet.rule(SliderPart::Track, move |t, _| t.bg(slider));
    sheet.rule(ProgressPart::Root, move |t, _| t.bg(progress));
}
pub use progress::ProgressPart;
pub use slider::{SliderChain, SliderPart};
pub(crate) use splitter::Side;
pub use splitter::{PaneSize, SplitterChain, SplitterStart};
pub use stepper::StepperChain;
pub use switch::SwitchChain;
pub use table::{
    Align, Cells, Column, Rows, RowsChain, Selectable, Selected, Stripes, TableChain, Unselectable,
};
pub use table_sheet::TablePart;
pub(crate) use table_sheet::{
    TableDefaults, table_defaults, write_exceptions as write_table_exceptions,
    write_rules as write_table_rules,
};
pub use tabs_sheet::TabsPart;
pub(crate) use tabs_sheet::{
    TabsDefaults, tabs_defaults, write_exceptions as write_tabs_exceptions,
    write_rules as write_tabs_rules,
};
pub use toggle::{ToggleChain, ToggleItemChain};
pub use toggle_group::{Member, ToggleGroup};
pub use tooltip_sheet::TooltipPart;
pub(crate) use tooltip_sheet::{TooltipDefaults, tooltip_defaults};

use crate::contract::{Colors, Disabled, Focus, Look, Paint, Size, States, Style};
pub(crate) use crate::tooltip::{Content, Tooltip};
use crate::{Builder, Icon, NodeUi as _, TextSpec, Tw, icons, tw};

/// Whether a checkbox is ticked, a radio button chosen or a selectable
/// button selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Unticked, unchosen or unselected.
    Off,
    /// Ticked, chosen or selected.
    On,
}

impl From<bool> for Mark {
    fn from(on: bool) -> Self {
        if on { Self::On } else { Self::Off }
    }
}

/// The two values of a checkbox or switch, set with `values(off, on)`. A
/// control over a `bool` that has not set them has [`Unset`] and means
/// `false` and `true`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unset;

/// The mark of `current` among the two `[off, on]` values of a checkbox,
/// switch or toggle, and the value a click moves to.
fn flip<T: Copy + PartialEq>([off, on]: [T; 2], current: T) -> (Mark, T) {
    if current == on {
        (Mark::On, off)
    } else {
        (Mark::Off, on)
    }
}

/// Whether a control takes input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Enablement {
    /// Responds to the pointer and the keyboard.
    #[default]
    Enabled,
    /// Inert, and drawn in the style's explicit disabled colours.
    Disabled,
}

/// TODO(prose): `true` is [`Enabled`](Enablement::Enabled).
///
/// ```
/// use eggshell_ui::owned::Enablement;
///
/// assert_eq!(Enablement::from(true), Enablement::Enabled);
/// assert_eq!(Enablement::from(false), Enablement::Disabled);
/// ```
impl From<bool> for Enablement {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

impl Enablement {
    /// What a control's own node senses: a click, or nothing at all while
    /// disabled. A generated look sets paint only, so the control says this
    /// at its call site.
    pub(crate) fn senses(self) -> Tw {
        match self {
            Self::Enabled => tw().sense_click(),
            Self::Disabled => tw().inert(),
        }
    }

    /// `classes`, inert when disabled and, if the style fades disabled
    /// controls, faded. Apply it to the root of a control: what is inside
    /// follows the root's opacity.
    pub(crate) fn apply(self, classes: Tw, style: &Style) -> Tw {
        self.apply_in(classes, style.tokens.disabled)
    }

    /// [`apply`](Self::apply) where disabled controls draw as `disabled` says.
    fn apply_in(self, classes: Tw, disabled: Disabled) -> Tw {
        match (self, disabled) {
            (Self::Enabled, _) => classes,
            (Self::Disabled, Disabled::Faded(opacity)) => classes.inert().opacity(opacity),
            (Self::Disabled, Disabled::Colors) => classes.inert(),
        }
    }

    /// `classes` at the opacity the style gives a disabled control, if it
    /// fades them.
    fn fade(self, classes: Tw, style: &Style) -> Tw {
        match (self, style.tokens.disabled) {
            (Self::Disabled, Disabled::Faded(opacity)) => classes.opacity(opacity),
            _ => classes,
        }
    }

    /// TODO(prose): disabled if either is.
    ///
    /// ```
    /// use eggshell_ui::owned::Enablement;
    ///
    /// assert_eq!(Enablement::Enabled.and(Enablement::Enabled), Enablement::Enabled);
    /// assert_eq!(Enablement::Enabled.and(Enablement::Disabled), Enablement::Disabled);
    /// assert_eq!(Enablement::Disabled.and(Enablement::Enabled), Enablement::Disabled);
    /// ```
    #[must_use]
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Enabled, Self::Enabled) => Self::Enabled,
            _ => Self::Disabled,
        }
    }

    /// The colours of `states` at rest, or the disabled ones if the style
    /// has them (a style that fades has the rest colours, faded by the root).
    fn colors(self, states: States, style: &Style) -> Colors {
        self.colors_in(states, style.tokens.disabled)
    }

    /// [`colors`](Self::colors) where disabled controls draw as `disabled`
    /// says.
    fn colors_in(self, states: States, disabled: Disabled) -> Colors {
        match (self, disabled) {
            (Self::Disabled, Disabled::Colors) => states.disabled,
            _ => states.rest,
        }
    }
}

/// `classes` with their content centred in both directions.
fn centered(classes: Tw) -> Tw {
    classes.flex().items_center().justify_center()
}

/// What a control's [`on_response`](ButtonChain::on_response) runs with its
/// response each frame, if it has one.
pub(crate) struct OnResponse<A>(Option<Rc<ResponseFn<A>>>);

type ResponseFn<A> = dyn Fn(&mut A, &Response);
type Hook<A> = Option<Rc<dyn Fn(&mut A)>>;
type BlurHook<A> = Option<Rc<dyn Fn(&mut A, Blur)>>;

impl<A> Clone for OnResponse<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<A> Default for OnResponse<A> {
    fn default() -> Self {
        Self(None)
    }
}

impl<A> OnResponse<A> {
    pub(crate) fn set(&mut self, then: impl Fn(&mut A, &Response) + 'static) {
        self.0 = Some(Rc::new(then));
    }

    pub(crate) fn run(&self, app: &mut A, response: &Response) {
        if let Some(then) = &self.0 {
            then(app, response);
        }
    }
}

/// The hooks of a control that holds text, run after its field is shown.
pub(crate) struct FieldHooks<A> {
    pub(crate) response: OnResponse<A>,
    submitted: Hook<A>,
    blurred: BlurHook<A>,
}

impl<A> Clone for FieldHooks<A> {
    fn clone(&self) -> Self {
        Self {
            response: self.response.clone(),
            submitted: self.submitted.clone(),
            blurred: self.blurred.clone(),
        }
    }
}

impl<A> Default for FieldHooks<A> {
    fn default() -> Self {
        Self {
            response: OnResponse::default(),
            submitted: None,
            blurred: None,
        }
    }
}

impl<A> FieldHooks<A> {
    pub(crate) fn submitted(&mut self, then: impl Fn(&mut A) + 'static) {
        self.submitted = Some(Rc::new(then));
    }

    pub(crate) fn blurred(&mut self, then: impl Fn(&mut A, Blur) + 'static) {
        self.blurred = Some(Rc::new(then));
    }

    /// Runs the hooks for the frame in which `response`, the field's, was
    /// shown. On Enter `submitted` runs before `blurred`. egui reports a lost focus for two frames, so a blur is the
    /// frame in which the focus the field had last frame is gone.
    pub(crate) fn run(&self, app: &mut A, ui: &Ui, response: &Response, enter: Enter) {
        self.response.run(app, response);
        if let Some(then) = &self.submitted
            && entered(ui, response, enter)
        {
            then(app);
        }
        let has_focus = response.has_focus();
        let had_focus = ui.data_mut(|data| {
            let slot = data.get_temp_mut_or_default::<bool>(response.id.with("had-focus"));
            std::mem::replace(slot, has_focus)
        });
        if let Some(then) = &self.blurred
            && had_focus
            && !has_focus
        {
            then(app, crate::draw::blur_cause(ui));
        }
    }
}

/// Whether the frame's Enter submitted `response`'s field. Under
/// `KeepFocus` the field still has focus, and an Enter that comes in the frame
/// of IME input or the one after belongs to the IME: some platforms send the
/// Enter that confirms a composition after its commit.
fn entered(ui: &Ui, response: &Response, enter: Enter) -> bool {
    let settled = match enter {
        Enter::Submit => response.lost_focus(),
        Enter::KeepFocus => response.has_focus(),
    };
    let ime = enter == Enter::KeepFocus && after_ime(ui, response);
    let pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
    settled && pressed && !ime
}

/// Whether IME input came in this frame or the last, noting this frame's.
fn after_ime(ui: &Ui, response: &Response) -> bool {
    let now = ui.ctx().cumulative_frame_nr();
    let slot = response.id.with("eggshell-ime");
    let last = ui.data(|data| data.get_temp::<u64>(slot));
    if ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)))
    }) {
        ui.data_mut(|data| data.insert_temp(slot, now));
        return true;
    }
    last.is_some_and(|last| now <= last.saturating_add(1))
}

/// What a control's tooltip shows, if it has one, until the control is
/// built.
pub(crate) type TipContent = Option<Content>;

/// A built control's tooltip, if it has one.
pub(crate) type Tip = Option<Tooltip>;

/// The tooltip of the control `trigger` that `b` builds, showing `content`.
pub(crate) fn tip<A: 'static>(b: &mut Builder<'_, '_, A>, trigger: Id, content: TipContent) -> Tip {
    content.map(|content| Tooltip::new(b, trigger, content))
}

/// `response` with `tip` shown on hover, also when the control is disabled.
pub(crate) fn tipped(ui: &Ui, response: Response, tip: &Tip) -> Response {
    if let Some(tip) = tip {
        crate::tooltip::register(ui, &response, tip);
    }
    response
}

/// `tip` shown while the pointer is anywhere over the node of `ui` or focus
/// is on anything inside it, anchored to the node: a control's own tooltip,
/// whatever part of it is hovered or focused.
pub(crate) fn tipped_whole(ui: &Ui, tip: &Tip) {
    if let Some(tip) = tip {
        crate::tooltip::register_whole(ui, tip);
    }
}

/// `response`, a part of a control [`tipped_whole`], described by the
/// control's `tip`, which it does not trigger.
pub(crate) fn tip_describes(ui: &Ui, response: &Response, tip: &Tip) {
    if let Some(tip) = tip {
        crate::tooltip::describes(ui, response, tip);
    }
}

/// The node's response, described to assistive technology as a `role` named
/// `name`, and as on or off when it is `selected`, with the tooltip `tip`.
fn described(ui: &Ui, role: Role, selected: Option<Mark>, name: &str, tip: &Tip) -> Response {
    let response = tipped(ui, ui.node_response(), tip);
    describe(&response, ui.is_enabled().into(), role, selected, name);
    response
}

/// Announces the node of `ui` to assistive technology as a group of `role`,
/// named `name` if it has one.
pub(crate) fn announce_group(ui: &Ui, role: Role, name: Option<&str>) {
    let id = ui.node_response().id;
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(role);
        if let Some(name) = name {
            node.set_label(name);
        }
    });
}

/// Describes `response` to assistive technology as a `role` named `name`,
/// and as on or off when it is `selected`.
pub(crate) fn describe(
    response: &Response,
    enablement: Enablement,
    role: Role,
    selected: Option<Mark>,
    name: &str,
) {
    let enabled = enablement == Enablement::Enabled;
    response.widget_info(|| match selected {
        Some(mark) => WidgetInfo::selected(role, enabled, mark == Mark::On, name),
        None => WidgetInfo::labeled(role, enabled, name),
    });
}

/// The classes that paint a box with `colors`. A control's text is not
/// selectable: pressing it operates the control.
fn painted(classes: Tw, colors: Colors) -> Tw {
    classes
        .select_none()
        .bg(colors.fill)
        .border_color(colors.border)
        .text(colors.text)
}

impl Look {
    /// The classes that draw a box of `radius` in this look, as the owned
    /// button does: the rest colours, the hover, pressed and focused looks
    /// over them, the focus ring and the shadow, or when `enablement` is
    /// disabled the disabled form. The box senses clicks and is a Tab stop.
    ///
    /// TODO(prose)
    ///
    /// ```
    /// use std::time::Duration;
    /// use eggshell_ui::owned::Enablement;
    /// use eggshell_ui::Builder;
    ///
    /// fn tile(b: &mut Builder<'_, '_, ()>) {
    ///     let style = b.style().clone();
    ///     let look = style.select().trigger;
    ///     let classes = look.classes(&style, 6, b.enablement(), Duration::from_millis(150));
    ///     b.div_id("tile", classes.p(2.0), |b| b.text(eggshell_ui::tw(), "Tile"))
    ///         .role(egui::Role::Button)
    ///         .on_click(|_| {});
    /// }
    /// ```
    pub fn classes(
        &self,
        style: &Style,
        radius: u8,
        enablement: Enablement,
        transition: Duration,
    ) -> Tw {
        self.paint
            .classes(self.border_width, style, radius, enablement, transition)
    }
}

impl Paint {
    /// The classes that draw a box of `radius` with a border `border_width`
    /// wide in this paint: [`Look::classes`] for a paint and a border width
    /// apart. A disabled box is inert and has no variants: its colours are
    /// the style's, not an opacity.
    ///
    /// TODO(prose)
    ///
    /// ```
    /// use std::time::Duration;
    /// use eggshell_ui::owned::Enablement;
    /// use eggshell_ui::Builder;
    ///
    /// fn cell(b: &mut Builder<'_, '_, ()>) {
    ///     let style = b.style().clone();
    ///     let toggles = &style.toggle.off;
    ///     let paint = *toggles.get(toggles.default_key());
    ///     let classes = paint.classes(1.0, &style, 4, Enablement::Enabled, Duration::ZERO);
    ///     b.div_id("cell", classes.size_px(16.0), |_| {});
    /// }
    /// ```
    pub fn classes(
        &self,
        border_width: f32,
        style: &Style,
        radius: u8,
        enablement: Enablement,
        transition: Duration,
    ) -> Tw {
        self.classes_in(
            border_width,
            (style.tokens.focus, style.tokens.disabled),
            radius,
            enablement,
            transition,
        )
    }

    /// [`classes`](Self::classes) with the style's focus ring and disabled
    /// look given apart, for a sheet rule, which has no style to read.
    pub(crate) fn classes_in(
        &self,
        border_width: f32,
        (focus, disabled): (Focus, Disabled),
        radius: u8,
        enablement: Enablement,
        transition: Duration,
    ) -> Tw {
        let base = tw()
            .border(border_width)
            .rounded_px(f32::from(radius))
            .sense_click();
        let shadow = |classes: Tw, fade: f32| match self.shadow {
            Some(shadow) => classes.shadow(egui::Shadow {
                color: shadow.color.gamma_multiply(fade),
                ..shadow
            }),
            None => classes,
        };
        let states = self.states;
        match enablement {
            Enablement::Disabled => {
                let fade = match disabled {
                    Disabled::Colors => 0.5,
                    Disabled::Faded(_) => 1.0,
                };
                shadow(
                    painted(base, enablement.colors_in(states, disabled)).inert(),
                    fade,
                )
            }
            Enablement::Enabled => {
                let ring = self.focus.unwrap_or(focus);
                let (rest, focused) = (states.rest, states.focused);
                let halo = |p: crate::tw::Paint| match self.hover_ring {
                    Some(ring) => p.ring(ring.width, ring.color),
                    None => p,
                };
                let classes = painted(base, rest)
                    .hover(|p| {
                        halo(
                            p.bg(states.hover.fill)
                                .border_color(states.hover.border)
                                .text(states.hover.text),
                        )
                    })
                    .active(|p| {
                        halo(
                            p.bg(states.pressed.fill)
                                .border_color(states.pressed.border)
                                .text(states.pressed.text),
                        )
                    })
                    .focus(|p| {
                        let p = if focused.fill == rest.fill {
                            p
                        } else {
                            p.bg(focused.fill)
                        };
                        let p = if focused.border == rest.border {
                            p
                        } else {
                            p.border_color(focused.border)
                        };
                        let p = if focused.text == rest.text {
                            p
                        } else {
                            p.text(focused.text)
                        };
                        p.ring(ring.width, ring.color)
                    })
                    .transition(transition);
                shadow(classes, 1.0)
            }
        }
    }
}

/// `size`'s text laid over the role the subtree inherits.
pub(crate) fn text_sized<A>(b: &Builder<'_, '_, A>, size: Option<crate::contract::TextSize>) -> Tw {
    match size {
        Some(crate::contract::TextSize { size, line_height }) => tw().role(TextSpec {
            size,
            line_height,
            ..b.text_style().spec.clone()
        }),
        None => tw(),
    }
}

/// The key of a trigger whose popup is `open`, the web's `aria-expanded`:
/// for its own open look, and for its containers' `has-aria-expanded`.
pub(crate) fn expanded_key(open: bool) -> Option<crate::sheet::Key> {
    open.then_some(crate::sheet::Key::Aria(crate::Aria::Expanded))
}

/// The classes of a framed text field's leaf in the ARIA state `aria`, if
/// any: the ring around the box the field's own frame draws, from the
/// field's rules, at rest (an invalid field's) and on focus. The frame's
/// border is egui's, set by [`Style::map_input`].
pub(crate) fn text_field_classes<A: 'static>(
    b: &mut Builder<'_, '_, A>,
    size: Size,
    aria: Option<crate::Aria>,
) -> Tw {
    let style = b.style().clone();
    let mut keys = input_keys(style.input().sizes.key_for(size), false);
    keys.extend(aria.map(crate::sheet::Key::Aria));
    let root = b.resolved_part(InputPart::Root, &keys);
    let leaf = tw().rounded_px(f32::from(root.paint.radius.nw));
    let own_text = style
        .input()
        .sizes
        .get(style.input().sizes.key_for(size))
        .text;
    let leaf = match (own_text, root.text.size, root.text.line_height) {
        (None, Some(size), Some(line_height)) => leaf.text_size(size, line_height),
        _ => leaf,
    };
    let leaf = match aria {
        Some(aria) => leaf.aria(aria),
        None => leaf,
    };
    let leaf = match root.paint.ring {
        Some(ring) => leaf.ring(ring.width, ring.color),
        None => leaf,
    };
    match root.variant_ring(crate::tw::State::Focus) {
        Some(ring) => leaf.focus_within(|p| p.ring(ring.width, ring.color)),
        None => leaf,
    }
}

fn check() -> Icon {
    Icon::new(icons::CHECK)
}

/// Which way a chevron points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chevron {
    Up,
    Down,
    Right,
}

impl Chevron {
    fn icon(self) -> Icon {
        Icon::new(match self {
            Self::Down => icons::CHEVRON_DOWN,
            Self::Up => icons::CHEVRON_UP,
            Self::Right => icons::CHEVRON_RIGHT,
        })
    }
}

/// The classes of the row that holds a checkbox, radio button or switch and
/// its label.
///
/// TODO(#446): the gap and the text role are the control's own section's,
/// given here until the field's rules hold them.
fn choice_row<A>(
    b: &mut Builder<'_, '_, A>,
    text: &TextSpec,
    (enablement, validity): (Enablement, &Validity),
    gap: f32,
) -> Tw {
    let own = tw().gap_px(gap).role(TextSpec {
        line_height: text.size,
        ..text.clone()
    });
    let own = match enablement {
        Enablement::Enabled => own,
        Enablement::Disabled => own.inert(),
    };
    let mut keys = FieldPart::keys(enablement);
    keys.extend(validity.field_key(enablement));
    b.part(FieldPart::Root, &keys, own)
}

/// The visible label of the control `control`: clicking it runs `on_click`,
/// and assistive technology names the control by it.
fn choice_label<A: 'static, K: crate::Key>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    control: Id,
    text: String,
    enablement: Enablement,
    on_click: impl FnOnce(&mut A) + 'static,
) {
    let classes = b.part(
        FieldPart::Label,
        &FieldPart::keys(enablement),
        tw().sense_click().focusable_none(),
    );
    let name = text.clone();
    b.span(key, classes, text).draw(move |app, ui| {
        if ui.label_for(control, &name) {
            on_click(app);
        }
    });
}

/// How a choice control is laid out with its label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Form {
    /// The control beside its label.
    #[default]
    Row,
    /// A card: the label holds the field, the control after the title and
    /// description, and the whole card toggles it.
    Card,
}

impl Form {
    /// `keys` with the card's, for a control in a card.
    fn keys(self, mut keys: Vec<crate::sheet::Key>) -> Vec<crate::sheet::Key> {
        if self == Self::Card {
            keys.push(crate::sheet::Key::Attr(
                choices_sheet::CARD.0,
                choices_sheet::CARD.1,
            ));
        }
        keys
    }
}

/// What a choice card says of the control it holds.
struct Card {
    /// The control the card names and toggles.
    control: Id,
    title: String,
    description: Option<String>,
    enablement: Enablement,
    /// The caller's classes, over the card's.
    classes: Tw,
}

/// A choice card under `key`: a label that holds the field `row`, with the
/// content and then what `build_control` builds. Clicking the card runs
/// `on_click`.
fn choice_card<A: 'static, K: crate::Key>(
    b: &mut Builder<'_, '_, A>,
    key: K,
    card: Card,
    row: Tw,
    on_click: impl FnOnce(&mut A) + 'static,
    build_control: impl FnOnce(&mut Builder<'_, '_, A>),
) {
    let Card {
        control,
        title,
        description,
        enablement,
        classes,
    } = card;
    let keys = FieldPart::card_keys(enablement);
    let own = enablement.senses();
    let label = b.part(FieldPart::Label, &keys, own.focusable_none().merge(classes));
    let name = title.clone();
    b.div_id(key, label, |b| {
        b.div(row, |b| {
            let content = b.part(FieldPart::Content, &[], tw());
            let keys = FieldPart::keys(enablement);
            b.div(content, |b| {
                let title_classes = b.part(FieldPart::Title, &keys, tw());
                b.text(title_classes, title);
                if let Some(description) = description {
                    let classes = b.part(FieldPart::Description, &keys, tw());
                    b.text(classes, description);
                }
            });
            build_control(b);
        });
    })
    .draw(move |app, ui| {
        if ui.label_for(control, &name) {
            on_click(app);
        }
    });
}

/// The owned controls, each shown under a key like [`div_id`](Builder::div_id)
/// and [`widget`](Builder::widget).
///
/// A control starts from the builder and ends in a terminating call that adds
/// it and says what the user's input changes in the application's state:
///
/// ```
/// # use eggshell_ui::Builder;
/// # #[derive(Hash, Debug)]
/// # enum Part { Save, Terms }
/// # struct App { saves: u32, terms: bool }
/// # fn view(b: &mut Builder<'_, '_, App>, s: &App) {
/// b.button(Part::Save, "Save").on_click(|s: &mut App| s.saves += 1);
/// b.checkbox(Part::Terms, "I agree", s.terms).on_change(|s, agreed| s.terms = agreed);
/// # }
/// ```
///
/// Configuration and hooks (`on_response`, and `when_submitted` and
/// `when_blurred` on a text field and a stepper) come before the terminating
/// call. A control that is started and dropped without one is not added, and
/// the compiler warns. Anything that only triggers finishes with `on_click` (a button), and
/// anything that holds a value with `on_change`: it is passed the value
/// the user chose, and the control shows the value it was built with.
impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// A button showing `face`: a label, or an [`Icon`](crate::Icon) alone.
    /// Assistive technology sees a `Role::Button`.
    pub fn button<K: crate::Key>(
        &mut self,
        key: K,
        face: impl Into<Face>,
    ) -> ButtonChain<'_, 'f, A> {
        ButtonChain::new(self.reborrow(), key, face.into())
    }

    /// A toggle button showing `face` and `current`: a `bool`, or with
    /// [`values`](ToggleChain::values) any two-valued type. Assistive
    /// technology sees a `Role::Button`, toggled on or off.
    pub fn toggle<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        face: impl Into<Face>,
        current: T,
    ) -> ToggleChain<'_, 'f, A, T> {
        ToggleChain::new(self.reborrow(), key, face.into(), current)
    }

    /// A member of `group` that is on when the group's current value is
    /// `value`, showing `face`. Members may sit anywhere below the builder
    /// that places them, with other nodes between them. Assistive technology
    /// sees each member as a `Role::RadioButton`.
    pub fn toggle_item<T: Copy + PartialEq + Hash + Debug + 'static>(
        &mut self,
        group: &ToggleGroup<A, T>,
        value: T,
        face: impl Into<Face>,
    ) -> ToggleItemChain<'_, 'f, A, T> {
        group.item(self.reborrow(), value, face.into())
    }

    /// A node that hosts custom egui content, sized by classes or by
    /// [`measure`](CustomChain::measure).
    pub fn custom<K: crate::Key>(&mut self, key: K, classes: Tw) -> CustomChain<'_, 'f, A> {
        CustomChain::new(self.reborrow(), key, classes)
    }

    /// A checkbox labelled `label` that shows `current`: a `bool`, or with
    /// [`values`](CheckboxChain::values) any two-valued type. Assistive
    /// technology sees a `Role::CheckBox`.
    pub fn checkbox<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        label: impl Into<String>,
        current: T,
    ) -> CheckboxChain<'_, 'f, A, T> {
        CheckboxChain::new(self.reborrow(), key, Some(label.into()), current)
    }

    /// A checkbox without a label.
    pub fn checkbox_unlabelled<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        current: T,
    ) -> CheckboxChain<'_, 'f, A, T> {
        CheckboxChain::new(self.reborrow(), key, None, current)
    }

    /// A switch labelled `label` that shows `current`: a `bool`, or with
    /// [`values`](SwitchChain::values) any two-valued type. Assistive
    /// technology sees a `Role::Switch`.
    pub fn switch<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        label: impl Into<String>,
        current: T,
    ) -> SwitchChain<'_, 'f, A, T> {
        SwitchChain::new(self.reborrow(), key, Some(label.into()), current)
    }

    /// A switch without a label.
    pub fn switch_unlabelled<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        current: T,
    ) -> SwitchChain<'_, 'f, A, T> {
        SwitchChain::new(self.reborrow(), key, None, current)
    }

    /// Radio buttons for `options` (anything [`IntoChoice`]), of which
    /// `current` is chosen. Values must be distinct. Assistive technology
    /// sees each option as a `Role::RadioButton`.
    pub fn radio_group<K: crate::Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> RadioGroupChain<'_, 'f, A, T> {
        RadioGroupChain::new(self.reborrow(), key, current, options)
    }

    /// A segmented control over `options` (anything [`IntoChoice`]), of which
    /// `current` is selected. Values must be distinct. Assistive technology
    /// sees each cell as a `Role::RadioButton`.
    pub fn segmented<K: crate::Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> SegmentedChain<'_, 'f, A, T> {
        SegmentedChain::new(self.reborrow(), key, current, options)
    }

    /// A slider at `value` within `range`. Assistive technology sees a
    /// `Role::Slider`.
    pub fn slider<K: crate::Key>(
        &mut self,
        key: K,
        value: f64,
        range: impl Into<ValueRange>,
    ) -> SliderChain<'_, 'f, A> {
        SliderChain::new(self.reborrow(), key, value, range.into())
    }

    /// A select over `options` (anything [`IntoChoice`]), of which `current`
    /// is chosen; the trigger shows the label of the chosen one. Assistive
    /// technology sees the trigger as a `Role::ComboBox`, the open list as a
    /// `Role::ListBox` and each option as a `Role::ListBoxOption`.
    pub fn select<K: crate::Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> SelectChain<'_, 'f, A, T> {
        SelectChain::new(self.reborrow(), key, current, options)
    }

    /// A button that opens a menu under it, which the chain's `items` fills.
    /// Assistive technology sees the trigger as a `Role::Button`, the menu as a
    /// `Role::Menu` and its items as `Role::MenuItem`, `MenuItemCheckBox` or
    /// `MenuItemRadio`.
    pub fn dropdown_menu<K: crate::Key + Clone>(
        &mut self,
        key: K,
        face: impl Into<Face>,
    ) -> DropdownChain<'_, 'f, A, K> {
        DropdownChain::new(self.reborrow(), key, face.into())
    }

    /// A progress bar filled to `progress`, drawn like a slider's track.
    /// Assistive technology sees a `Role::ProgressIndicator`.
    pub fn progress<K: crate::Key>(
        &mut self,
        key: K,
        progress: crate::Progress,
    ) -> ProgressChain<'_, 'f, A> {
        ProgressChain::new(self.reborrow(), key, progress)
    }

    /// TODO(prose): a splitter between two panes of the flex container it is
    /// built in.
    pub fn splitter<K: crate::Key>(&mut self, key: K) -> SplitterStart<'_, 'f, A> {
        SplitterStart::new(self.reborrow(), key)
    }

    /// TODO(prose): a table, one grid whose columns line up across its rows.
    /// Assistive technology sees a `Role::Table` of `Role::Row`s, each of
    /// `Role::ColumnHeader`s or `Role::Cell`s.
    pub fn table<K: crate::Key>(&mut self, key: K) -> TableChain<'_, 'f, A> {
        TableChain::new(self.reborrow(), key)
    }

    /// A stepper at `value` within `range`. Assistive technology sees its
    /// field as a `Role::SpinButton`, and its buttons as `Role::Button`s named
    /// `"Increase <name>"` and `"Decrease <name>"`.
    pub fn stepper<K: crate::Key>(
        &mut self,
        key: K,
        value: f64,
        range: impl Into<ValueRange>,
    ) -> StepperChain<'_, 'f, A> {
        StepperChain::new(self.reborrow(), key, value, range.into())
    }
}

/// Builds an open overlay's rows in a box of `classes` under `key`. When the
/// last layout cut the overlay short, the rows are in a scroll container
/// under `scroller`, as tall as they want, which the box shrinks to its
/// room: shadcn's `max-h-(--available-height) overflow-y-auto`, with the
/// gutter only when it scrolls.
pub(crate) fn fitted_box<'f, A: 'static>(
    s: &mut crate::builder::Scope<'_, 'f, A>,
    fit: crate::overlay::Fit,
    (key, scroller): (impl crate::Key, impl crate::Key),
    classes: Tw,
    rows: impl FnOnce(&mut crate::builder::Scope<'_, 'f, A>),
) {
    use crate::overlay::Fit;
    use crate::scroll::{Axis, Focusable};
    match fit {
        Fit::Short { natural } => {
            let height = natural.y - crate::builder::block_insets(&s.layout_of(&classes));
            s.div_id(key, classes.flex().flex_col().min_h_0(), |s| {
                let viewport = tw()
                    .flex()
                    .flex_col()
                    .h_px(height)
                    .min_h_0()
                    .focusable_none();
                s.scroll(scroller, viewport)
                    .axis(Axis::Y)
                    .focusable(Focusable::Auto)
                    .children(rows);
            });
        }
        Fit::Measuring | Fit::Fits => {
            s.div_id(key, classes, rows);
        }
    }
}
