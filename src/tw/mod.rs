//! The `Tw` style builder, in Tailwind vocabulary.
//!
//! A [`Tw`] holds fully resolved values: points, [`egui::Color32`] and
//! [`TextSpec`](crate::TextSpec). Theming is the application's concern.
//!
//! Layout classes write a [`taffy::Style`]. One spacing unit is 4 points, so
//! `.p(1.5)` is `p-1.5` is 6 points. Methods suffixed `_px` take points
//! directly, `_frac(n, d)` is the fraction `n/d`, `_full` is 100%, and `_auto`
//! is the `auto` keyword, and `w_ch`, `min_w_ch` and `max_w_ch` take widths in
//! `ch`, resolved when the builder knows the node's font. Layout methods take bare floats because the method
//! name carries the unit, exactly as in Tailwind.

mod ch;
mod easing;
mod grid;
mod layout;
mod merge;
mod paint;
mod rules;
mod text;
mod units;
mod variants;

use taffy::{Display, Style};

use crate::contract::Size;
use crate::text::TextClasses;

pub use ch::Reserve;
pub(crate) use ch::{MinWidth, TextWidths};
pub use easing::Easing;
pub use grid::Track;
use merge::{Layout, Touched};
pub use paint::{BoxPaint, Divide, Ring, RingPlacement, Sides};
pub use rules::{Aria, Match};
pub(crate) use rules::{Link, Ruled, Rules};
pub use units::{Angle, Mirror, Opacity, Progress};
pub use variants::Paint;
pub(crate) use variants::{
    Focusability, Grouping, Look, PointerEvents, Presence, Scrollbar, Sensing, State, Variants,
    Visibility,
};

/// One Tailwind space unit, in points.
const SPACE_UNIT: f32 = 4.0;

/// A node's classes: layout, box paint, inherited text and behaviour.
#[derive(Clone, Debug)]
pub struct Tw {
    pub(crate) style: Style,
    pub(crate) ch: TextWidths,
    pub(crate) paint: BoxPaint,
    pub(crate) text: TextClasses,
    pub(crate) z: Option<i16>,
    pub(crate) inert: bool,
    pub(crate) control_size: Option<Size>,
    pub(crate) pointer_events: PointerEvents,
    /// `None` inherits the parent's.
    pub(crate) visibility: Option<Visibility>,
    pub(crate) sensing: Sensing,
    pub(crate) focusability: Focusability,
    pub(crate) scrollbar: Scrollbar,
    /// How far scrolling a node into view keeps it from the edges.
    pub(crate) scroll_padding: Sides<f32>,
    pub(crate) grouping: Grouping,
    pub(crate) variants: Variants,
    pub(crate) lengths: Lengths,
    /// What the node says of itself and the rules it declares; `None` for
    /// most nodes, which keeps a `Tw` small.
    pub(crate) rules: Option<Box<Ruled>>,
    touched: Touched,
}

/// Whether a class was ignored because a length it was given is infinite or
/// not a number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Lengths {
    #[default]
    Finite,
    Rejected,
}

impl Lengths {
    /// `after` if `finite` holds for it, else `before` with the rejection noted.
    fn keep<T>(&mut self, before: T, after: T, finite: impl FnOnce(&T) -> bool) -> T {
        if finite(&after) {
            after
        } else {
            *self = Self::Rejected;
            before
        }
    }

    fn or(self, other: Self) -> Self {
        if self == Self::Rejected { self } else { other }
    }
}

/// A fresh class list. Like CSS, it starts as `display: block`.
pub fn tw() -> Tw {
    Tw {
        style: Style {
            display: Display::Block,
            ..Style::default()
        },
        ch: TextWidths::default(),
        paint: BoxPaint::default(),
        text: TextClasses::default(),
        z: None,
        inert: false,
        control_size: None,
        pointer_events: PointerEvents::Auto,
        visibility: None,
        sensing: Sensing::None,
        focusability: Focusability::Tab,
        scrollbar: Scrollbar::Gutter,
        scroll_padding: Sides::default(),
        grouping: Grouping::Ungrouped,
        variants: Variants::default(),
        lengths: Lengths::Finite,
        rules: None,
        touched: Touched::default(),
    }
}

impl Tw {
    /// Classes that say nothing.
    pub(crate) fn blank() -> Self {
        tw()
    }

    /// The layout style the classes have produced so far.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// The box paint the classes have produced so far.
    pub fn paint_classes(&self) -> &BoxPaint {
        &self.paint
    }

    /// The text classes set so far.
    pub fn text_classes(&self) -> &TextClasses {
        &self.text
    }

    /// Sibling paint and hit order; never affects layout.
    pub fn z(mut self, z: i16) -> Self {
        self.z = Some(z);
        self
    }

    /// Makes the subtree non-interactive, like HTML's `inert` attribute: its
    /// widgets are disabled and take no input or focus.
    ///
    /// The owned controls inside it (buttons, checkboxes, sliders, ...) are
    /// drawn in their disabled look, as inside a `<fieldset disabled>`. Other
    /// inert content looks unchanged; to dim it, add opacity, which fades
    /// boxes, text and widgets together (Tailwind's `inert:opacity-50`):
    ///
    /// ```
    /// use eggshell_ui::{Opacity, tw};
    /// let dimmed = tw().inert().opacity(Opacity::new(0.5));
    /// ```
    pub fn inert(mut self) -> Self {
        self.inert = true;
        self
    }

    /// TODO(prose): the size the owned controls inside take unless they set their own.
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::tw;
    /// let dense = tw().control_size(Size("sm"));
    /// ```
    pub fn control_size(mut self, size: Size) -> Self {
        self.control_size = Some(size);
        self
    }

    /// Lets the pointer through this node and its descendants to what lies
    /// beneath them, like CSS `pointer-events: none`.
    ///
    /// By default a node that paints a background or a border takes the
    /// pointer from the sensing nodes painted before it that it covers (CSS
    /// `pointer-events: auto`): a panel over a canvas keeps the canvas from
    /// hovering, clicking or dragging through it. Use this on a decoration
    /// that must not take the pointer. It only stops boxes from blocking: a
    /// node inside that senses the pointer still does, and there is no way to
    /// turn blocking back on below it.
    pub fn pointer_events_none(mut self) -> Self {
        self.pointer_events = PointerEvents::None;
        self
    }

    /// TODO(prose): hides the node and its subtree but keeps its box in the
    /// layout, like Tailwind's `invisible` (CSS `visibility: hidden`):
    /// nothing of it is painted, takes the pointer or the keyboard focus,
    /// shows a tooltip or reaches assistive technology, and its neighbours
    /// stay where they are. A descendant with [`visible`](Self::visible) is
    /// shown again. [`hidden`](Self::hidden) removes the box instead.
    ///
    /// ```
    /// use eggshell_ui::{Builder, tw};
    ///
    /// fn view(b: &mut Builder<'_, '_, ()>, saving: bool) {
    ///     let spinner = if saving { tw() } else { tw().invisible() };
    ///     b.div(tw().flex().gap(2.0), |b| {
    ///         b.span("status", spinner, "Saving…");
    ///         b.button("save", "Save").add();
    ///     });
    /// }
    /// ```
    pub fn invisible(mut self) -> Self {
        self.visibility = Some(Visibility::Hidden);
        self
    }

    /// TODO(prose): shows the node again inside an [`invisible`](Self::invisible)
    /// subtree, like Tailwind's `visible` (CSS `visibility: visible`).
    pub fn visible(mut self) -> Self {
        self.visibility = Some(Visibility::Visible);
        self
    }

    fn styled(mut self, set: impl FnOnce(&mut Layout<'_>)) -> Self {
        self.write_layout(set);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_as_a_block_with_default_taffy_style() {
        let tw = tw();
        assert_eq!(
            *tw.style(),
            Style {
                display: Display::Block,
                ..Style::default()
            }
        );
        assert_eq!(
            taffy::Style::<String>::default().display,
            Display::Flex,
            "precondition: taffy defaults to flex, so tw() must set block itself"
        );
    }

    #[test]
    fn a_class_given_a_number_that_is_not_finite_is_ignored_and_noted() {
        use egui::Color32;

        let width = tw().w_px(4.0);
        assert_eq!(width.lengths, Lengths::Finite);
        let rejected = [
            width.clone().w_px(f32::NAN),
            width.clone().p(f32::INFINITY),
            width.clone().gap(f32::NEG_INFINITY),
            width.clone().w_frac(1.0, 0.0),
            width.clone().w_ch(f32::NAN),
            width.clone().grow_by(f32::NAN),
            width.clone().aspect(f32::INFINITY),
            width.clone().grid_cols_tracks(&[Track::px(f32::NAN)]),
        ];
        for classes in rejected {
            assert_eq!(*classes.style(), *width.style());
            assert_eq!(classes.ch, width.ch);
            assert_eq!(classes.lengths, Lengths::Rejected);
        }
        let ringed = tw().ring(2.0, Color32::RED);
        let unringed = ringed.clone().ring(f32::NAN, Color32::BLUE);
        assert_eq!(unringed.paint, ringed.paint);
        assert_eq!(unringed.lengths, Lengths::Rejected);
        let divided = tw().divide_x(f32::INFINITY, Color32::RED);
        assert_eq!(divided.paint, tw().paint);
        let hovered = tw().hover(|p| p.bg(Color32::RED).translate(f32::NAN, 0.0));
        assert_eq!(hovered.variants, tw().hover(|p| p).variants);
        assert_eq!(hovered.lengths, Lengths::Rejected);
        let texted = tw().role(crate::text::TextSpec {
            size: f32::NAN,
            line_height: 20.0,
            family: egui::FontFamily::Proportional,
            weight: crate::text::Weight::Normal,
        });
        assert_eq!(texted.text, tw().text);
        assert_eq!(texted.lengths, Lengths::Rejected);
    }

    #[test]
    fn merging_keeps_the_note_and_a_rejected_class_overrides_nothing() {
        let merged = tw().w_px(4.0).merge(tw().w_px(f32::NAN));
        assert_eq!(*merged.style(), *tw().w_px(4.0).style());
        assert_eq!(merged.lengths, Lengths::Rejected);
        let merged = tw().w_px(f32::NAN).merge(tw().p(1.0));
        assert_eq!(merged.lengths, Lengths::Rejected);
    }

    #[test]
    fn z_and_inert_do_not_touch_layout() {
        let styled = tw().z(30).inert();
        assert_eq!(styled.z, Some(30));
        assert!(styled.inert);
        assert_eq!(*styled.style(), *tw().style());
    }
}
