//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Role, Vec2};

use super::Mark;
use super::{
    Card, Enablement, Form, OnResponse, Unset, choice_card, choice_label, choice_row, described,
    flip, options, options::Gate, options::IntoTip, options::Validity,
};
use crate::builder::Slot;
use crate::contract::Size;
use crate::focus::{Autofocus, Select};
use crate::transition;
use crate::{Builder, Easing, Progress, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Track,
    Label,
}

/// A switch drawn from the [`Style`](crate::Style), with an optional label. Start one with
/// [`Builder::switch`] and finish it with [`on_change`](Self::on_change).
///
/// It shows `current` and is on when that is its `on` value: `true` for a
/// `bool`, or the second of [`values`](Self::values). A click passes the other
/// value to `on_change`.
///
/// The thumb scales with the track's height and travels its width, so
/// [`track`](Self::track) can resize the track of a size without redrawing
/// anything.
#[must_use = "a switch is not added until you call `.on_change(..)`"]
pub struct SwitchChain<'a, 'f, A, T, V = Unset> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: Option<String>,
    description: Option<String>,
    form: Form,
    name: Option<String>,
    gate: Gate,
    validity: Validity,
    current: T,
    values: V,
    size: Option<Size>,
    track: Option<Vec2>,
    classes: Tw,
    enablement: Enablement,
    on_response: OnResponse<A>,
    autofocus: Autofocus,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> SwitchChain<'a, 'f, A, T> {
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        label: Option<String>,
        current: T,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            label,
            description: None,
            form: Form::Row,
            name: None,
            gate: Gate::default(),
            current,
            values: Unset,
            size: None,
            track: None,
            classes: tw(),
            enablement: Enablement::Enabled,
            on_response: OnResponse::default(),
            autofocus: Autofocus::Never,
            validity: Validity::Valid,
        }
    }

    /// The values the switch moves between: `off` when off and `on` when on.
    pub fn values(self, off: T, on: T) -> SwitchChain<'a, 'f, A, T, [T; 2]> {
        SwitchChain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            description: self.description,
            form: self.form,
            name: self.name,
            gate: self.gate,
            current: self.current,
            values: [off, on],
            size: self.size,
            track: self.track,
            classes: self.classes,
            enablement: self.enablement,
            on_response: self.on_response,
            autofocus: self.autofocus,
            validity: self.validity,
        }
    }
}

impl<'a, 'f, A: 'static, T, V> SwitchChain<'a, 'f, A, T, V> {
    /// TODO(prose): marks the switch invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::owned::text_field::TextFieldChain::invalid)
    /// does: its track's border and ring turn destructive, and so does its field's label.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.validity = Validity::of(error);
        self
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Draws the switch at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Overrides the track's width and height, in points.
    pub fn track(mut self, track: Vec2) -> Self {
        self.track = Some(track);
        self
    }

    /// TODO(prose): draws the switch as a choice card: the label holds it, the track after the label and the description, and a click anywhere on the card flips it.
    pub fn card(mut self) -> Self {
        self.form = Form::Card;
        self
    }

    /// TODO(prose): the text under a card's label.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Classes merged over the styled ones of the whole row, such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
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

    /// Whether the switch takes input.
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

    /// Runs `then` with the track's response each frame: the escape hatch for
    /// what the builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }
}

impl<'a, 'f, A: 'static> SwitchChain<'a, 'f, A, bool> {
    /// Adds the switch. `on_change` runs with the new value when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> SwitchChain<'a, 'f, A, T, [T; 2]> {
    /// Adds the switch. `on_change` runs with the new value when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            description,
            form,
            name,
            gate,
            current,
            values: [off, on_value],
            size,
            track,
            classes: caller,
            enablement,
            on_response,
            autofocus,
            validity,
        } = self;
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let (mark, toggled) = flip([off, on_value], current);
        let on_toggle = Rc::new(move |app: &mut A| on_change(app, toggled));
        let style = b.style().clone();
        let switch = &style.switch;
        let size = b.size_of(size);
        let metrics = *switch.sizes.lookup(b.ctx(), size);
        let size = switch.sizes.key_for(size);
        let default_track = super::field::switch_track(&style.tokens, size);
        let thumb_scale = track.map_or(1.0, |track| track.y / default_track.y);
        let thumb = |b: &mut Builder<'_, '_, A>, mark| {
            let classes = b.resolved_part(
                SwitchPart::Thumb,
                &form.keys(SwitchPart::keys(size, mark, enablement)),
            );
            let diameter = b.layout_of(&classes).size.width.into_raw().value() * thumb_scale;
            let fill = classes
                .paint
                .background
                .unwrap_or(egui::Color32::TRANSPARENT);
            (diameter, fill)
        };
        let ((thumb_off, fill_off), (thumb_on, fill_on)) =
            (thumb(&mut b, Mark::Off), thumb(&mut b, Mark::On));
        let slide = switch.slide;
        let target = if mark == Mark::On {
            Progress::END
        } else {
            Progress::START
        };
        // A card's label is drawn before the control it holds, so the
        // control names itself by the card's title.
        let name = match form {
            Form::Row => name.unwrap_or_default(),
            Form::Card => name.or_else(|| label.clone()).unwrap_or_default(),
        };
        b.claim(&slot);
        b.scope_with(&slot, |b| {
            let control = b.id_of(Part::Track);
            let tint = transition::animate(
                b.ctx(),
                control.with("slide"),
                target,
                slide,
                Easing::Default,
            );
            let own = match track {
                Some(track) => enablement.senses().w_px(track.x).h_px(track.y),
                None => enablement.senses(),
            };
            let own = match tint == target {
                true => own,
                false => {
                    let [off, on] = [Mark::Off, Mark::On].map(|mark| {
                        let classes = b.resolved_part(
                            SwitchPart::Root,
                            &form.keys(SwitchPart::keys(size, mark, enablement)),
                        );
                        (classes.paint.background, classes.paint.border_colors.top)
                    });
                    let t = f32::from(tint);
                    let mix = |off: Option<egui::Color32>, on: Option<egui::Color32>| {
                        let clear = egui::Color32::TRANSPARENT;
                        off.unwrap_or(clear).lerp_to_gamma(on.unwrap_or(clear), t)
                    };
                    let (fill, border) = (mix(off.0, on.0), mix(off.1, on.1));
                    own.bg(fill)
                        .border_color(border)
                        .hover(|p| p.bg(fill).border_color(border))
                        .active(|p| p.bg(fill).border_color(border))
                }
            };
            let mut keys = form.keys(SwitchPart::keys(size, mark, enablement));
            keys.extend(validity.key(enablement));
            let classes = b.part(SwitchPart::Root, &keys, own);
            let row = choice_row(
                b,
                &style.role_sized(&switch.text, metrics.text),
                (enablement, &validity),
                switch.gap,
            );
            let on_press = on_toggle.clone();
            let build_track = move |b: &mut Builder<'_, '_, A>| {
                b.div_id(Part::Track, classes, |b| {
                    b.paint(tw().w_full().h_full(), move |painter, rect, _| {
                        let t = f32::from(transition::animate(
                            painter.ctx(),
                            control.with("slide"),
                            target,
                            slide,
                            Easing::Default,
                        ));
                        let diameter = thumb_off + (thumb_on - thumb_off) * t;
                        let thumb_color = fill_off.lerp_to_gamma(fill_on, t);
                        let inset = rect.height() / 2.0;
                        let x = egui::lerp(rect.left() + inset..=rect.right() - inset, t);
                        painter.circle_filled(
                            egui::pos2(x, rect.center().y),
                            diameter / 2.0,
                            thumb_color,
                        );
                    });
                })
                .open_focus(autofocus.open_focus())
                .draw(move |app, ui| {
                    let response = described(ui, Role::Switch, Some(mark), &name, &tooltip);
                    validity.describe(ui.ctx(), response.id);
                    autofocus.apply(ui.ctx(), &response);
                    if response.clicked() {
                        on_press(app);
                    }
                    on_response.run(app, &response);
                });
            };
            match form {
                Form::Row => b.div(row.merge(caller), |b| {
                    build_track(b);
                    if let Some(label) = label {
                        choice_label(b, Part::Label, control, label, enablement, move |app| {
                            on_toggle(app)
                        });
                    }
                }),
                Form::Card => {
                    let card = Card {
                        control,
                        title: label.unwrap_or_default(),
                        description,
                        enablement,
                        classes: caller,
                    };
                    choice_card(
                        b,
                        Part::Label,
                        card,
                        row,
                        move |app| on_toggle(app),
                        build_track,
                    );
                }
            }
        });
    }
}

/// TODO(prose): the anatomy of a switch: shadcn's `switch`.
///
/// TODO(prose): the keys its rules see: the size (shadcn's `data-size`),
/// `aria-checked` (shadcn's `data-checked`) and `aria-disabled`. The thumb is
/// painted, sliding between its two places: its rules give the size and fill
/// it is painted at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwitchPart {
    /// TODO(prose): the track.
    Root,
    /// TODO(prose): the thumb.
    Thumb,
}

impl crate::sheet::Part for SwitchPart {
    const SOURCE: &'static str = "switch.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "switch",
            Self::Thumb => "switch-thumb",
        }
    }
}

impl SwitchPart {
    /// TODO(prose): the keys of a switch of `size` and its thumb, on if `mark` is, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, Mark, switch::SwitchPart};
    ///
    /// let keys = SwitchPart::keys(Size::DEFAULT, Mark::Off, Enablement::Enabled);
    /// assert_eq!(keys.len(), 1);
    /// ```
    pub fn keys(
        size: crate::contract::Size,
        mark: super::Mark,
        enablement: super::Enablement,
    ) -> Vec<crate::sheet::Key> {
        super::field::choice_keys(size, mark, enablement)
    }
}
