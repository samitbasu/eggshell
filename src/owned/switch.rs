//! The owned switch: a track node that senses clicks and slides a thumb,
//! beside a label node that flips it too.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Context, Role, Vec2};

use super::Mark;
use super::options::{Configure, Interactive, Options, Resolved, Respond, Validity};
use super::{
    Card, Form, OnResponse, Unset, choice_card, choice_label, choice_row, described, flip,
};
use crate::builder::Slot;
use crate::contract::Size;
use crate::transition;
use crate::{Builder, Easing, Progress, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Node {
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
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its classes are the whole row's,
/// and its response is the track's.
#[must_use = "a switch is not added until you call `.on_change(..)`"]
pub struct Chain<'a, 'f, A, T, V = Unset> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: Option<String>,
    description: Option<String>,
    form: Form,
    opts: Options,
    validity: Validity,
    current: T,
    values: V,
    size: Option<Size>,
    track: Option<Vec2>,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T> {
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
            opts: Options::new(),
            current,
            values: Unset,
            size: None,
            track: None,
            on_response: OnResponse::default(),
            validity: Validity::Valid,
        }
    }

    /// The values the switch moves between: `off` when off and `on` when on.
    pub fn values(self, off: T, on: T) -> Chain<'a, 'f, A, T, [T; 2]> {
        Chain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            description: self.description,
            form: self.form,
            opts: self.opts,
            current: self.current,
            values: [off, on],
            size: self.size,
            track: self.track,
            on_response: self.on_response,
            validity: self.validity,
        }
    }
}

impl<'a, 'f, A: 'static, T, V> Chain<'a, 'f, A, T, V> {
    /// TODO(prose): marks the switch invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::owned::text_field::TextFieldChain::invalid)
    /// does: its track's border and ring turn destructive, and so does its field's label.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.validity = Validity::of(error);
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
}

impl<'a, 'f, A: 'static> Chain<'a, 'f, A, bool> {
    /// Adds the switch. `on_change` runs with the new value when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, on_change: impl Fn(&mut A, &Context, bool) + 'static) {
        self.values(false, true).on_change_with(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T, [T; 2]> {
    /// Adds the switch. `on_change` runs with the new value when the track or
    /// the label is clicked, or the track is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        self.on_change_with(move |app, _, value| on_change(app, value));
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, on_change: impl Fn(&mut A, &Context, T) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            description,
            form,
            opts,
            current,
            values: [off, on_value],
            size,
            track,
            on_response,
            validity,
        } = self;
        let Resolved {
            name,
            classes: caller,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let (mark, toggled) = flip([off, on_value], current);
        let on_toggle = Rc::new(move |app: &mut A, ctx: &Context| on_change(app, ctx, toggled));
        let style = b.style().clone();
        let switch = &style.switch;
        let size = b.size_of(size);
        let metrics = *switch.sizes.lookup(b.ctx(), size);
        let size = switch.sizes.key_for(size);
        let default_track = super::field::switch_track(&style.tokens, size);
        let thumb_scale = track.map_or(1.0, |track| track.y / default_track.y);
        let thumb = |b: &mut Builder<'_, '_, A>, mark| {
            let classes =
                b.resolved_part(Part::Thumb, &form.keys(Part::keys(size, mark, enablement)));
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
            let control = b.id_of(Node::Track);
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
                            Part::Root,
                            &form.keys(Part::keys(size, mark, enablement)),
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
            let mut keys = form.keys(Part::keys(size, mark, enablement));
            keys.extend(validity.key(enablement));
            let classes = b.part(Part::Root, &keys, own);
            let row = choice_row(
                b,
                &style.role_sized(&switch.text, metrics.text),
                (enablement, &validity),
                switch.gap,
            );
            let on_press = on_toggle.clone();
            let build_track = move |b: &mut Builder<'_, '_, A>| {
                b.div_id(Node::Track, classes, |b| {
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
                        on_press(app, ui.ctx());
                    }
                    on_response.run(app, &response);
                });
            };
            match form {
                Form::Row => b.div(row.merge(caller), |b| {
                    build_track(b);
                    if let Some(label) = label {
                        choice_label(
                            b,
                            Node::Label,
                            control,
                            label,
                            enablement,
                            move |app, ctx| on_toggle(app, ctx),
                        );
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
                        Node::Label,
                        card,
                        row,
                        move |app, ctx| on_toggle(app, ctx),
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
pub enum Part {
    /// TODO(prose): the track.
    Root,
    /// TODO(prose): the thumb.
    Thumb,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "switch.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "switch",
            Self::Thumb => "switch-thumb",
        }
    }
}

impl Part {
    /// TODO(prose): the keys of a switch of `size` and its thumb, on if `mark` is, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, Mark, switch::Part};
    ///
    /// let keys = Part::keys(Size::DEFAULT, Mark::Off, Enablement::Enabled);
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

impl<A, T, V> Configure for Chain<'_, '_, A, T, V> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, T, V> Interactive for Chain<'_, '_, A, T, V> {}

impl<A, T, V> Respond for Chain<'_, '_, A, T, V> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A switch labelled `label` that shows `current`: a `bool`, or with
    /// [`values`](Chain::values) any two-valued type. Assistive
    /// technology sees a `Role::Switch`.
    pub fn switch<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        label: impl Into<String>,
        current: T,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, Some(label.into()), current)
    }

    /// A switch without a label.
    pub fn switch_unlabelled<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        current: T,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, None, current)
    }
}
