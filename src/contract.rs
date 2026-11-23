//! [`Style`]: everything the owned widgets draw with.
//!
//! Core defines what its widgets need and a preset (such as the `eggshell-ui-shadcn` crate) fills it, so a design
//! system is data: an app gives a layout its [`Style`] with
//! [`LayoutState::with_style`](crate::LayoutState::with_style) and the widgets
//! follow, with no code change. [`Style::default`] is derived from egui's
//! visuals, so core looks coherent with no preset at all.
//!
//! Every field is exact: the owned widgets draw exactly these values. Only the
//! text input, which stays a wrapped egui `TextEdit`, is approximated (egui
//! paints it from fixed widget slots): see [`InputStyle`].

use crate::overlay::Distance;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Context, FontFamily, Id, Shadow, Theme, Vec2, vec2};

pub use crate::presence::{Entry, KindMotion, MotionStyle};
use crate::{Faces, Opacity, Progress, TextSpec, Tracking, Weight};

/// A button variant, named by the style that defines it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant(pub &'static str);

impl Variant {
    /// The name [`Style::default`] gives its one variant.
    pub const DEFAULT: Self = Self("default");
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// A control size, named by the style that defines it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Size(pub &'static str);

impl Size {
    /// The name [`Style::default`] gives its one size.
    pub const DEFAULT: Self = Self("default");
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Entries by key with one that stands in for any key the style lacks. A
/// widget drawn with a key the style lacks uses the default entry and is
/// reported as [`Problem::MissingStyleEntry`](crate::Problem::MissingStyleEntry).
#[derive(Clone, Debug, PartialEq)]
pub struct Keyed<K, T> {
    default_key: K,
    default: T,
    others: BTreeMap<K, T>,
}

impl<K: Ord + Copy + Debug, T> Keyed<K, T> {
    /// A set whose default entry is `default` under `key`.
    pub fn new(key: K, default: T) -> Self {
        Self {
            default_key: key,
            default,
            others: BTreeMap::new(),
        }
    }

    /// Adds or replaces the entry under `key`.
    #[must_use]
    pub fn with(mut self, key: K, entry: T) -> Self {
        self.insert(key, entry);
        self
    }

    /// Adds or replaces the entry under `key`.
    pub fn insert(&mut self, key: K, entry: T) {
        if key == self.default_key {
            self.default = entry;
        } else {
            self.others.insert(key, entry);
        }
    }

    /// The key of the entry unknown keys fall back to.
    pub fn default_key(&self) -> K {
        self.default_key
    }

    /// `key` if the style defines it, else the key of the default entry.
    pub fn key_for(&self, key: K) -> K {
        if key == self.default_key || self.others.contains_key(&key) {
            key
        } else {
            self.default_key
        }
    }

    /// The entry under `key`, or the default entry if there is none.
    pub fn get(&self, key: K) -> &T {
        if key == self.default_key {
            return &self.default;
        }
        self.others.get(&key).unwrap_or(&self.default)
    }

    /// [`Self::get`], reporting a key the style does not define.
    #[track_caller]
    pub(crate) fn lookup(&self, ctx: &Context, key: K) -> &T {
        if key != self.default_key && !self.others.contains_key(&key) {
            crate::report::report(
                ctx,
                crate::Problem::MissingStyleEntry,
                Some(Id::NULL.with(("style-entry", format!("{key:?}")))),
                || format!("the style defines no {key:?}; drawing its default entry"),
            );
        }
        self.get(key)
    }

    /// The entry under `key`, if the style defines it.
    pub fn get_mut(&mut self, key: K) -> Option<&mut T> {
        if key == self.default_key {
            Some(&mut self.default)
        } else {
            self.others.get_mut(&key)
        }
    }

    /// TODO(prose): the default entry, to change.
    pub fn default_entry_mut(&mut self) -> &mut T {
        &mut self.default
    }

    /// The keys the style defines.
    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        std::iter::once(self.default_key).chain(self.others.keys().copied())
    }
}

/// What a box draws in one state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    /// Background.
    pub fill: Color32,
    /// Border.
    pub border: Color32,
    /// Text and glyphs.
    pub text: Color32,
}

impl Colors {
    /// These colours as CSS `opacity` shows them on an opaque `surface`: each
    /// channel is composited over `surface` and blended `opacity` of the way
    /// from it, so the result is opaque. A transparent channel stays
    /// transparent. Unlike a per-shape alpha, the result hides what lies
    /// beneath it.
    #[must_use]
    pub fn faded_over(self, opacity: Opacity, surface: Color32) -> Self {
        let t = f32::from(opacity);
        let fade = |color: Color32| {
            if color.a() == 0 {
                return Color32::TRANSPARENT;
            }
            let [r, g, b, a] = color.to_srgba_unmultiplied();
            let a = f32::from(a) / 255.0;
            let over = |c: u8, s: u8| f32::from(c) * a + f32::from(s) * (1.0 - a);
            let [sr, sg, sb, _] = surface.to_array();
            let covered = Color32::from_rgb(
                over(r, sr).round() as u8,
                over(g, sg).round() as u8,
                over(b, sb).round() as u8,
            );
            Color32::from_rgb(surface.r(), surface.g(), surface.b()).lerp_to_gamma(covered, t)
        };
        Self {
            fill: fade(self.fill),
            border: fade(self.border),
            text: fade(self.text),
        }
    }
}

impl Colors {
    /// These colours `progress` of the way to `other`.
    #[must_use]
    pub fn towards(self, other: Self, progress: Progress) -> Self {
        let t = progress.into();
        Self {
            fill: self.fill.lerp_to_gamma(other.fill, t),
            border: self.border.lerp_to_gamma(other.border, t),
            text: self.text.lerp_to_gamma(other.text, t),
        }
    }
}

/// A control's colours in each state.
///
/// `focused` only differs from `rest` in the channels the keyboard focus
/// changes: a channel equal to `rest` is left to the other states, so a
/// focused, hovered control keeps its hover fill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct States {
    /// At rest.
    pub rest: Colors,
    /// Under the pointer.
    pub hover: Colors,
    /// Held down.
    pub pressed: Colors,
    /// Holding keyboard focus.
    pub focused: Colors,
    /// Not taking input.
    pub disabled: Colors,
}

impl States {
    /// `rest` and `hover`, with the other states derived: pressed is hover,
    /// focused is rest, disabled is rest at half opacity over `surface`, the
    /// opaque colour the control sits on.
    pub fn new(rest: Colors, hover: Colors, surface: Color32) -> Self {
        Self {
            rest,
            hover,
            pressed: hover,
            focused: rest,
            disabled: rest.faded_over(Opacity::new(0.5), surface),
        }
    }

    /// The same colours in every state but disabled.
    pub fn still(rest: Colors, surface: Color32) -> Self {
        Self::new(rest, rest, surface)
    }
}

impl States {
    /// These colours, state by state, `progress` of the way to `other`.
    #[must_use]
    pub fn towards(self, other: Self, progress: Progress) -> Self {
        Self {
            rest: self.rest.towards(other.rest, progress),
            hover: self.hover.towards(other.hover, progress),
            pressed: self.pressed.towards(other.pressed, progress),
            focused: self.focused.towards(other.focused, progress),
            disabled: self.disabled.towards(other.disabled, progress),
        }
    }
}

/// The keyboard focus indicator: a ring outside the border box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    /// Ring width in points.
    pub width: f32,
    /// Ring colour.
    pub color: Color32,
}

impl Focus {
    /// TODO(prose): the inset that keeps this ring a ring's width clear of the box's edge.
    pub fn clearance(&self) -> f32 {
        2.0 * self.width
    }
}

/// How a box is painted: colours per state, shadow and focus ring. Nothing in
/// it changes the box's size, so the two [`Paint`]s of an on/off widget can
/// differ freely without moving anything.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    /// Colours per state.
    pub states: States,
    /// Drop shadow at rest.
    pub shadow: Option<Shadow>,
    /// This paint's own focus ring, where it is not [`Tokens::focus`].
    pub focus: Option<Focus>,
    /// A ring while the pointer is over the box or presses it, as the focus
    /// ring is drawn: shadcn's slider thumb halo.
    pub hover_ring: Option<Focus>,
}

impl Paint {
    /// A shadowless paint with the style's focus ring.
    pub fn new(states: States) -> Self {
        Self {
            states,
            shadow: None,
            focus: None,
            hover_ring: None,
        }
    }

    /// This paint with a border `width` points wide, making it a [`Look`].
    #[must_use]
    pub fn bordered(self, width: f32) -> Look {
        Look {
            paint: self,
            border_width: width,
        }
    }

    /// This paint's colours `progress` of the way to those of `other`, which
    /// supplies everything else.
    #[must_use]
    pub fn towards(self, other: Self, progress: Progress) -> Self {
        Self {
            states: self.states.towards(other.states, progress),
            ..other
        }
    }

    /// This paint with `ring` as its focus ring and as its ring under the
    /// pointer.
    #[must_use]
    pub fn haloed(self, ring: Focus) -> Self {
        Self {
            focus: Some(ring),
            hover_ring: Some(ring),
            ..self
        }
    }

    /// This paint with a drop shadow.
    #[must_use]
    pub fn shadowed(self, shadow: Shadow) -> Self {
        Self {
            shadow: Some(shadow),
            ..self
        }
    }
}

/// Everything that makes one kind of box look the way it does: its [`Paint`]
/// and the border width, which takes up room. A widget whose two states swap
/// paints holds one border width and two [`Paint`]s, so that switching states
/// cannot move anything.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// How the box is painted.
    pub paint: Paint,
    /// Border width in points.
    pub border_width: f32,
}

impl Look {
    /// A borderless, shadowless look with the style's focus ring.
    pub fn new(states: States) -> Self {
        Paint::new(states).bordered(0.0)
    }

    /// This look with a border `width` points wide.
    #[must_use]
    pub fn bordered(self, width: f32) -> Self {
        Self {
            border_width: width,
            ..self
        }
    }

    /// This look with a drop shadow.
    #[must_use]
    pub fn shadowed(self, shadow: Shadow) -> Self {
        Self {
            paint: self.paint.shadowed(shadow),
            ..self
        }
    }
}

/// A text size and line height, laid over a section's text role at one
/// control size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSize {
    /// Font size in points.
    pub size: f32,
    /// Height of one line in points.
    pub line_height: f32,
}

/// TODO(prose): the points one spacing unit is, Tailwind's `--spacing`: `p-4` is four of them.
///
/// Construction clamps to zero and above; NaN becomes [`Spacing::DEFAULT`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing(f32);

impl Spacing {
    /// TODO(prose): Tailwind's, 4 points.
    pub const DEFAULT: Self = Self(4.0);

    /// TODO(prose): `points` to a unit, clamped to zero and above; NaN is [`Spacing::DEFAULT`].
    pub fn new(points: f32) -> Self {
        if points.is_nan() {
            Self::DEFAULT
        } else {
            Self(crate::finite::clamp(points, 0.0, f32::MAX))
        }
    }

    /// TODO(prose): `units` of spacing, in points.
    pub fn points(self, units: f32) -> f32 {
        units * self.0
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<Spacing> for f32 {
    fn from(spacing: Spacing) -> f32 {
        spacing.0
    }
}

/// TODO(prose): a theme's corner radii, shadcn's `--radius` and the steps its `globals.css` derives from it.
///
/// Construction clamps to zero and above; NaN becomes [`Radius::DEFAULT`].
/// Every step is floored at zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radius(f32);

impl Radius {
    /// TODO(prose): shadcn's, `0.625rem`.
    pub const DEFAULT: Self = Self(10.0);

    /// TODO(prose): a scale whose `--radius` is `base` points.
    pub fn new(base: f32) -> Self {
        if base.is_nan() {
            Self::DEFAULT
        } else {
            Self(crate::finite::clamp(base, 0.0, f32::MAX))
        }
    }

    fn step(self, offset: f32) -> f32 {
        (self.0 + offset).max(0.0)
    }

    /// TODO(prose): `--radius-sm`, four points less than the base.
    pub fn sm(self) -> f32 {
        self.step(-4.0)
    }

    /// TODO(prose): `--radius-md`, two points less.
    pub fn md(self) -> f32 {
        self.step(-2.0)
    }

    /// TODO(prose): `--radius-lg`, the base.
    pub fn lg(self) -> f32 {
        self.step(0.0)
    }

    /// TODO(prose): `--radius-xl`, four points more.
    pub fn xl(self) -> f32 {
        self.step(4.0)
    }

    /// TODO(prose): `--radius-2xl`, eight points more.
    pub fn xl2(self) -> f32 {
        self.step(8.0)
    }

    /// TODO(prose): `--radius-3xl`, twelve points more.
    pub fn xl3(self) -> f32 {
        self.step(12.0)
    }

    /// TODO(prose): `--radius-4xl`, sixteen points more.
    pub fn xl4(self) -> f32 {
        self.step(16.0)
    }
}

/// TODO(prose): `points` as one of egui's corner radii: whole points, at most 255.
pub fn corner(points: f32) -> u8 {
    crate::finite::clamp(points.round(), 0.0, 255.0) as u8
}

impl Default for Radius {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO(prose): Tailwind's text sizes, each with the line height that goes with it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextScale {
    /// TODO(prose): `text-xs`.
    pub xs: TextSize,
    /// TODO(prose): `text-sm`.
    pub sm: TextSize,
    /// TODO(prose): `text-base`.
    pub base: TextSize,
    /// TODO(prose): `text-lg`.
    pub lg: TextSize,
}

/// TODO(prose): a step of the text scale, as a class names it: `text-sm` is [`TextStep::Sm`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStep {
    /// `text-xs`.
    Xs,
    /// `text-sm`.
    Sm,
    /// `text-base`.
    Base,
    /// `text-lg`.
    Lg,
}

impl TextScale {
    /// TODO(prose): the size and line height of `step`.
    pub fn step(&self, step: TextStep) -> TextSize {
        match step {
            TextStep::Xs => self.xs,
            TextStep::Sm => self.sm,
            TextStep::Base => self.base,
            TextStep::Lg => self.lg,
        }
    }
}

impl Default for TextScale {
    fn default() -> Self {
        let at = |size, line_height| TextSize { size, line_height };
        Self {
            xs: at(12.0, 16.0),
            sm: at(14.0, 20.0),
            base: at(16.0, 24.0),
            lg: at(18.0, 28.0),
        }
    }
}

/// Sizes of a button, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Height.
    pub height: f32,
    /// Smallest width.
    pub min_width: f32,
    /// Left and right padding.
    pub padding_x: f32,
    /// Gap between children.
    pub gap: f32,
    /// Corner radius.
    pub radius: u8,
    /// Side of an icon in the control: the icon of an icon button, with or
    /// without a label.
    pub glyph: f32,
    /// The text size of the control's label at this size, or `None` for the
    /// text role's own.
    pub text: Option<TextSize>,
}

/// How a disabled control is drawn.
///
/// Both forms make the control inert. They differ in what they paint.
///
/// * [`Colors`](Self::Colors) draws every box in the `disabled` colours of its
///   [`States`]. The colours are the style's, so a disabled control can be
///   anything, and each shape is painted once: nothing shows through a fill
///   that [`Colors::faded_over`] made opaque. The result is exactly CSS
///   `opacity` only if the style computed those colours against the surface
///   the control really sits on.
/// * [`Faded`](Self::Faded) draws the control in its `rest` colours with the
///   node opacity of its root, which every shape inside multiplies into its own
///   alpha. It needs no knowledge of the surface, so it is the form for a
///   translucent one, which no opaque colour can stand in for.
///
/// **Faded is not CSS group opacity.** CSS draws the element into
/// a group, composites its shapes there, and fades the group over the page
/// once. This library fades each shape separately, so where one shape lies on
/// another, such as text on its own fill, the lower one shows through the
/// upper by `1 - opacity`: a faded fill under faded text mixes a little of the
/// fill into the glyphs. [`Faded`](Self::Faded) is exact when no shapes
/// overlap: text or glyphs over the backdrop, a border-only or fill-only box,
/// a ghost button. Over an opaque fill, [`Colors`](Self::Colors) with colours
/// from [`Colors::faded_over`] is the exact form.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Disabled {
    /// The `disabled` colours of each state.
    #[default]
    Colors,
    /// The `rest` colours at this node opacity.
    Faded(Opacity),
}

/// TODO(prose): toggle buttons: their sizes and their looks off and on. A
/// plain button's look is the [`sheet`](Style::sheet)'s
/// ([`ButtonPart`](crate::owned::ButtonPart)); a toggle's moves there with
/// its own shadcn source, `toggle.tsx`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToggleStyle {
    /// Metrics of a toggle per size.
    pub sizes: Keyed<Size, Metrics>,
    /// Border width of a toggle, off or on.
    pub border_width: f32,
    /// The looks of a toggle that is off, by variant. Variants are chosen
    /// with [`ButtonChain::variant`](crate::owned::ButtonChain::variant) for toggles as
    /// for buttons.
    pub off: Keyed<Variant, Paint>,
    /// The looks of a toggle that is on.
    pub on: Keyed<Variant, Paint>,
    /// The text role of toggles, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// How far a pressed toggle moves down, in points; zero for none.
    pub press_shift: f32,
    /// How long state changes animate.
    pub transition: Duration,
}

/// A checkbox's box at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxMetrics {
    /// Side of the box.
    pub size: f32,
    /// Corner radius of the box.
    pub radius: u8,
    /// Side of the check glyph.
    pub glyph: f32,
    /// Gap between the box and its label.
    pub gap: f32,
    /// The label's text size, or `None` for the text role's own.
    pub text: Option<TextSize>,
}

/// Checkboxes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CheckboxStyle {
    /// The box and its label's gap and text per size.
    pub sizes: Keyed<Size, CheckboxMetrics>,
    /// Border width of the box, ticked or not.
    pub border_width: f32,
    /// The text role of the label, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// Unticked. The glyph takes the `text` colour of the ticked paint.
    pub off: Paint,
    /// Ticked.
    pub on: Paint,
    /// How long state changes animate.
    pub transition: Duration,
}

/// A radio button's circle at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadioMetrics {
    /// Diameter of the circle.
    pub size: f32,
    /// Diameter of the dot in the chosen one.
    pub dot: f32,
    /// Gap between the circle and its label.
    pub gap: f32,
    /// The label's text size, or `None` for the text role's own.
    pub text: Option<TextSize>,
}

/// Radio buttons.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RadioStyle {
    /// The circle and its label's gap and text per size.
    pub sizes: Keyed<Size, RadioMetrics>,
    /// Border width of the circle, chosen or not.
    pub border_width: f32,
    /// The text role of the label, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// Unchosen.
    pub off: Paint,
    /// Chosen. The dot takes the `text` colour.
    pub on: Paint,
    /// How long state changes animate.
    pub transition: Duration,
}

/// A switch's track and thumb at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchMetrics {
    /// Track width and height.
    pub track: Vec2,
    /// Thumb diameter while off.
    pub thumb_off: f32,
    /// Thumb diameter while on.
    pub thumb_on: f32,
    /// The label's text size, or `None` for the text role's own.
    pub text: Option<TextSize>,
}

/// Switches. The thumb takes the `text` colour of the track's look.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwitchStyle {
    /// Track and thumb per size.
    pub sizes: Keyed<Size, SwitchMetrics>,
    /// Gap between the track and its label.
    pub gap: f32,
    /// Border width of the track, off or on.
    pub border_width: f32,
    /// The text role of the label, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// Off.
    pub off: Paint,
    /// On.
    pub on: Paint,
    /// How long the thumb takes to slide.
    pub slide: Duration,
    /// How long state changes animate.
    pub transition: Duration,
}

/// A segmented control's dimensions at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentedMetrics {
    /// The track's corner radius.
    pub track_radius: u8,
    /// Padding between the track and its cells.
    pub track_padding: f32,
    /// Height of a one-row track.
    pub height: f32,
    /// Corner radius of a cell.
    pub cell_radius: u8,
    /// Left-right and top-bottom padding of a cell.
    pub cell_padding: Vec2,
    /// Gap between the cells of a row.
    pub column_gap: f32,
    /// Gap between the rows of a wrapping track.
    pub row_gap: f32,
    /// Side of the icon in a cell that has one.
    pub icon_size: f32,
    /// Gap between a cell's icon and its label.
    pub icon_gap: f32,
    /// The cells' text size, or `None` for the text role's own.
    pub text: Option<TextSize>,
    /// How much shorter than the inside of a track a cell is, split above
    /// and below it; zero fills it.
    pub cell_trim: f32,
}

/// Segmented controls: a track of cells, one of them selected.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SegmentedStyle {
    /// The track's background.
    pub track_fill: Color32,
    /// Track and cell dimensions per size.
    pub sizes: Keyed<Size, SegmentedMetrics>,
    /// The text role of the cells, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// The weight of the chosen cell's text, or `None` for the text role's
    /// own. A face of another weight has other widths, so when this differs
    /// from the cells' weight, choosing a cell resizes it and shifts the
    /// cells after it, and a fit-width track with them.
    pub selected_weight: Option<Weight>,
    /// Border width of a cell, chosen or not.
    pub cell_border_width: f32,
    /// The chosen cell.
    pub selected: Paint,
    /// Any other cell.
    pub unselected: Paint,
    /// How long state changes animate.
    pub transition: Duration,
}

/// A slider's dimensions at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderMetrics {
    /// Thickness of the track.
    pub track_height: f32,
    /// Diameter of the thumb, which is also the slider's height.
    pub thumb_size: f32,
    /// Corner radius of the track, its range and the thumb; `u8::MAX` makes
    /// them round.
    pub radius: u8,
    /// The text of the label and the readout above the track, over the
    /// role the subtree inherits.
    pub text: Option<TextSize>,
    /// The space between the label row and the track.
    pub header_gap: f32,
}

/// Sliders. A track holds a range that ends under a thumb.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderStyle {
    /// Track and thumb dimensions per size.
    pub sizes: Keyed<Size, SliderMetrics>,
    /// The track's fill; only `rest` and `disabled` apply.
    pub track: States,
    /// The range's fill; only `rest` and `disabled` apply.
    pub range: States,
    /// The thumb. Its focus ring is the keyboard focus indicator.
    pub thumb: Look,
    /// The label above the track; only `text` of `rest` and `disabled` applies.
    pub label: States,
    /// The readout above the track; only `text` of `rest` and `disabled` applies.
    pub readout: States,
    /// How long state changes animate.
    pub transition: Duration,
}

/// Determinate progress bars: a track filled up to a fraction.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProgressStyle {
    /// Thickness of the bar.
    pub height: f32,
    /// Corner radius of the track and its range; `u8::MAX` makes them round.
    pub radius: u8,
    /// The track's colours; only `fill` applies.
    pub track: Colors,
    /// The range's colours; only `fill` applies.
    pub range: Colors,
}

/// Select triggers. The list they open is drawn from [`MenuStyle`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectStyle {
    /// The trigger, an input-like box.
    pub trigger: Look,
    /// Size of the trigger per size: height, padding, gap between value and
    /// chevron, and corner radius.
    pub sizes: Keyed<Size, Metrics>,
    /// The text role of the trigger, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// Side of the chevron glyph.
    pub chevron: f32,
    /// How much of the trigger's text colour the chevron keeps.
    pub chevron_opacity: Opacity,
    /// How long state changes animate.
    pub transition: Duration,
}

/// Menus: the box, items, separators, headings, shortcuts and indicators of a
/// dropdown, a context menu and a submenu, and the list a select opens.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MenuStyle {
    /// The box: its `rest` colours, border and shadow.
    pub popup: Look,
    /// Corner radius of the box.
    pub radius: u8,
    /// Padding between the box and its items.
    pub padding: f32,
    /// The least width of a menu.
    pub min_width: f32,
    /// The gap between a trigger and its dropdown menu or select list.
    pub offset: Distance,
    /// An item; its `hover` colours mark the highlighted one, the one the
    /// pointer is on or that has the keyboard focus.
    pub item: Look,
    /// Corner radius of an item.
    pub item_radius: u8,
    /// Left-right and top-bottom padding of an item.
    pub item_padding: Vec2,
    /// Space between the parts of an item: its indicator, label and shortcut.
    pub item_gap: f32,
    /// A destructive item: `rest` and `hover` colours.
    pub destructive: States,
    /// A separator's colour.
    pub separator: Color32,
    /// The space above and below a separator.
    pub separator_margin: f32,
    /// The colour of a group heading.
    pub heading: Color32,
    /// The weight of a group heading, over the items' text role.
    pub heading_weight: Weight,
    /// The text role of items and headings, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// The colour of a shortcut.
    pub shortcut: Color32,
    /// The text role of a shortcut.
    pub shortcut_text: TextSpec,
    /// The letter spacing of a shortcut.
    pub shortcut_tracking: Tracking,
    /// Side of the square an item keeps for its check or radio mark, and of
    /// the submenu chevron.
    pub indicator: f32,
}

/// A tooltip's arrow: a square turned 45 degrees, filled like the box, half
/// of it outside the box and pointing at the trigger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arrow {
    /// Side of the square, in points. The gap between the trigger and the
    /// box grows by it, as Radix's does.
    pub size: f32,
}

/// TODO(prose): Tooltips: the optional arrow, where the box sits and the timing. The box's look is the sheet's [`TooltipPart::Content`](crate::owned::TooltipPart::Content); how one enters and leaves is [`MotionStyle::tooltip`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TooltipStyle {
    /// The arrow, if there is one.
    pub arrow: Option<Arrow>,
    /// The space between the trigger and the box (or its arrow).
    pub offset: Distance,
    /// How long the pointer rests on a trigger before its tooltip opens.
    pub delay: Duration,
    /// How long after a tooltip closes the next one opens at once.
    pub grace: Duration,
}

/// Key chips (`kbd`): a key or a chord, shown as a small box.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct KbdStyle {
    /// A chip on the page.
    pub page: Colors,
    /// A chip inside a tooltip, which is drawn on the tooltip's box.
    pub tooltip: Colors,
    /// Border width in points; zero for none.
    pub border_width: f32,
    /// Corner radius.
    pub radius: u8,
    /// Left-right padding.
    pub padding_x: f32,
    /// Height, and the least width, so one key is a square.
    pub height: f32,
    /// The text role, or `None` for the text around the chip.
    pub text: Option<TextSpec>,
}

/// TODO(prose): how a mount with a caption is framed.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MountStyle {
    /// TODO(prose)
    pub caption: Color32,
    /// TODO(prose)
    pub caption_text: Option<TextSpec>,
    /// TODO(prose)
    pub gap: f32,
    /// TODO(prose)
    pub border: Color32,
    /// TODO(prose)
    pub border_width: f32,
    /// TODO(prose)
    pub radius: u8,
    /// TODO(prose)
    pub padding: f32,
    /// TODO(prose): the inset of an uncaptioned mount, which keeps its controls' focus rings clear of the selected ring.
    pub bare_padding: f32,
    /// TODO(prose): the ring around the selected mount, drawn as the focus ring is.
    pub selected_ring: Color32,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RichTextStyle {
    /// Link text and underline.
    pub link: Color32,
    /// TODO(prose): a link's underline, in place of `link`.
    pub link_underline: Option<Color32>,
    /// The least weight of link text.
    pub link_weight: Weight,
    /// Inline code.
    pub code: InlineCode,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct InlineCode {
    /// The tint behind the code.
    pub fill: Color32,
    /// Corner radius of the tint.
    pub radius: u8,
    /// Left-right padding of the tint, taken as layout space.
    pub padding_x: f32,
    /// Top-bottom padding of the tint around the font's box; not layout space.
    pub padding_y: f32,
    /// The monospace size, relative to the text around it.
    pub size: Em,
}

/// A size relative to the surrounding text's, as CSS `em`: positive and
/// finite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Em(f32);

impl Em {
    /// The surrounding text's size.
    pub const ONE: Self = Self(1.0);

    /// `em` times the surrounding size; one where `em` is not positive and
    /// finite.
    pub fn new(em: f32) -> Self {
        if em.is_finite() && em > 0.0 {
            Self(em)
        } else {
            Self::ONE
        }
    }
}

impl From<Em> for f32 {
    fn from(em: Em) -> f32 {
        em.0
    }
}

/// A stepper's dimensions at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepperMetrics {
    /// Height of the group.
    pub height: f32,
    /// Left and right padding of the text.
    pub padding_x: f32,
    /// Corner radius of the group; the buttons round their outer corners to
    /// fit inside its border.
    pub radius: u8,
    /// Width of the buttons.
    pub button_width: f32,
    /// Side of the chevron glyph.
    pub glyph: f32,
    /// The text's size, or `None` for the text role's own.
    pub text: Option<TextSize>,
}

/// Steppers: a number field and two buttons, one above the other, in a single
/// bordered group.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StepperStyle {
    /// The group's box. Its focus ring and border show while the text has
    /// focus.
    pub group: Look,
    /// The group's dimensions per size.
    pub sizes: Keyed<Size, StepperMetrics>,
    /// The two buttons, which have no border of their own.
    pub button: Look,
    /// The hairlines between the text and the buttons, and between the buttons.
    pub divider: Color32,
    /// How long state changes animate.
    pub transition: Duration,
}

/// A text field's dimensions at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputMetrics {
    /// Corner radius.
    pub radius: u8,
    /// Space between the border and the text.
    pub padding: Vec2,
    /// Height of a one-line field; a taller field's text sets its height.
    pub height: f32,
    /// The text's size, or `None` for the text role's own.
    pub text: Option<TextSize>,
    /// Side of a leading or trailing icon.
    pub icon_size: f32,
    /// Space between an icon and the text.
    pub icon_gap: f32,
}

/// Text fields, which are egui's `TextEdit` wrapped: egui paints them from
/// fixed widget slots, so [`Style::map_input`] approximates this section on
/// the field's own `Ui` and the library's size estimate reads the same
/// values.
///
/// | Field | On the `TextEdit` |
/// |---|---|
/// | `look` rest `fill` | exact: the field's background in every state |
/// | `look` rest and hover `border`, `border_width` | exact; the focus border is at least 1 pt wide so that focus always shows |
/// | `look` rest `text` | exact: the colour of typed text in every state |
/// | `look` hover `fill`, `pressed`, disabled colours | not drawn: egui has one fill and one text colour |
/// | `look` focused `border` | approximated: egui draws the focus border in the colour of selected text, so [`Selection::text`] wins when set |
/// | `look` shadow and focus ring | not drawn: egui's frame has no shadow and its ring is the 1 pt border |
/// | `sizes`: radius, padding, height | exact |
/// | `hint` | exact when set; otherwise egui's own grey-out of the text colour |
/// | [`Style::selection`] fill | exact |
///
/// The text cursor keeps the colour of the `Ui`'s visuals.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct InputStyle {
    /// Colours per state and the border width.
    pub look: Look,
    /// Radius, padding and height per size.
    pub sizes: Keyed<Size, InputMetrics>,
    /// The colour of hint text, or `None` for egui's grey-out of the text.
    pub hint: Option<Color32>,
    /// The colour of a leading or trailing icon.
    pub icon: Color32,
}

/// Selected text.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Selection {
    /// The highlight behind selected text.
    pub fill: Color32,
    /// The colour of selected text, if it is not the text's own.
    pub text: Option<Color32>,
}

/// The scroll bars of scroll containers.
///
/// A container always reserves a gutter `width` wide on the axis it scrolls,
/// and draws the bar in it when the content overflows. The bar is egui's: it
/// handles the drag, and is painted with these colours.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollStyle {
    /// The gutter's width, and the bar's: the width of the vertical bar and
    /// the height of the horizontal one, in points.
    pub width: f32,
    /// The thumb at rest.
    pub thumb: Color32,
    /// The thumb under the pointer.
    pub thumb_hover: Color32,
    /// The thumb while it is dragged.
    pub thumb_active: Color32,
    /// The track behind the thumb.
    pub track: Color32,
    /// The corner radius of the thumb and the track.
    pub radius: egui::CornerRadius,
    /// The shortest the thumb gets, in points.
    pub min_thumb_length: f32,
    /// Fades the content into the container's background at an edge it can
    /// still scroll past. [`EdgeFade::OFF`] unless a container opts in with
    /// [`ScrollChain::fade_edges`](crate::scroll::ScrollChain::fade_edges).
    pub fade: EdgeFade,
}

/// The gradient egui paints over the edges of a scroll container that has
/// more content beyond them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeFade {
    /// The opacity of the background colour at the very edge.
    /// [`Opacity::TRANSPARENT`] draws no fade.
    pub strength: Opacity,
    /// How far the fade reaches into the container, in points.
    pub size: f32,
}

impl EdgeFade {
    /// No fade.
    pub const OFF: Self = Self {
        strength: Opacity::TRANSPARENT,
        size: 0.0,
    };

    /// A fade `size` points deep that reaches `strength` at the edge.
    pub const fn new(strength: Opacity, size: f32) -> Self {
        Self { strength, size }
    }
}

impl Default for EdgeFade {
    fn default() -> Self {
        Self::OFF
    }
}

impl ScrollStyle {
    /// What egui's own scroll bars look like in `egui`.
    pub fn from_egui(egui: &egui::Style) -> Self {
        let (scroll, widgets) = (egui.spacing.scroll, &egui.visuals.widgets);
        Self {
            width: scroll.bar_width,
            thumb: widgets.inactive.bg_fill,
            thumb_hover: widgets.hovered.bg_fill,
            thumb_active: widgets.active.bg_fill,
            track: egui.visuals.extreme_bg_color,
            radius: widgets.inactive.corner_radius,
            min_thumb_length: scroll.handle_min_length,
            fade: EdgeFade::OFF,
        }
    }

    /// The gutter's width, never negative.
    pub(crate) fn gutter(&self) -> f32 {
        self.width.max(0.0)
    }
}

/// TODO(prose): the colours of a splitter at rest or hot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterColors {
    /// TODO(prose): the hit area's fill.
    pub band: Color32,
    /// TODO(prose): the line along the middle.
    pub line: Color32,
    /// TODO(prose): the grip, where there is one.
    pub grip: Color32,
}

impl SplitterColors {
    /// These colours `progress` of the way to `other`.
    #[must_use]
    pub fn towards(self, other: Self, progress: Progress) -> Self {
        let t = progress.into();
        Self {
            band: self.band.lerp_to_gamma(other.band, t),
            line: self.line.lerp_to_gamma(other.line, t),
            grip: self.grip.lerp_to_gamma(other.grip, t),
        }
    }
}

/// TODO(prose): a pill at the middle of a splitter's line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grip {
    /// TODO(prose): along the line.
    pub length: f32,
    /// TODO(prose): across the line.
    pub thickness: f32,
}

/// TODO(prose): splitters.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitterStyle {
    /// TODO(prose): the hit area across the axis.
    pub thickness: f32,
    /// TODO(prose): the line's width.
    pub line_width: f32,
    /// TODO(prose): at rest.
    pub rest: SplitterColors,
    /// TODO(prose): hovered, dragged or focused.
    pub hot: SplitterColors,
    /// TODO(prose): the grip, if any.
    pub grip: Option<Grip>,
    /// TODO(prose): how long the change to and from hot takes.
    pub transition: Duration,
}

impl SplitterStyle {
    /// TODO(prose): a splitter in egui's colours.
    pub fn from_egui(egui: &egui::Style) -> Self {
        let v = &egui.visuals;
        let clear = Color32::TRANSPARENT;
        Self {
            thickness: 6.0,
            line_width: 1.0,
            rest: SplitterColors {
                band: clear,
                line: v.widgets.noninteractive.bg_stroke.color,
                grip: clear,
            },
            hot: SplitterColors {
                band: clear,
                line: v.selection.bg_fill,
                grip: clear,
            },
            grip: None,
            transition: Duration::ZERO,
        }
    }
}

/// TODO(prose): what a table shows where a group ends and rows in no group
/// follow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroupEnd {
    /// TODO(prose): an empty band this many points tall.
    Gap(f32),
    /// TODO(prose): a rule `width` points wide in `color`.
    Rule {
        /// TODO(prose)
        width: f32,
        /// TODO(prose)
        color: Color32,
    },
}

/// TODO(prose): Tables: the rules between rows, stripes, the first column's inset and a group's title and end. The rest of a table's look is the sheet's [`TablePart`](crate::owned::TablePart)s.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TableStyle {
    /// TODO(prose): padding before the first column's content, in place of
    /// `cell_padding`; zero sets it flush with the table's edge.
    pub first_column_inset: Option<f32>,
    /// TODO(prose): whether a rule also runs under the last body row,
    /// closing the table.
    pub rule_below_last: bool,
    /// The line under each row but the last body row, and over the footer.
    pub rule: Color32,
    /// The rule's width.
    pub rule_width: f32,
    /// TODO(prose): behind every second body row of a table with
    /// [`Stripes::Alternate`](crate::owned::Stripes::Alternate), starting
    /// with the second, and again after each group row.
    pub stripe: Color32,
    /// TODO(prose): the size of a group row's title; `None` keeps the
    /// table's.
    pub group_text_size: Option<TextSize>,
    /// TODO(prose): the weight of a group row's title.
    pub group_weight: Weight,
    /// TODO(prose): what shows where a group ends before rows in no group.
    pub group_end: GroupEnd,
}

impl TableStyle {
    /// Rows ruled in egui's separator colour.
    pub fn from_egui(egui: &egui::Style) -> Self {
        let (v, s) = (&egui.visuals, &egui.spacing);
        Self {
            first_column_inset: None,
            rule_below_last: false,
            rule: v.widgets.noninteractive.bg_stroke.color,
            rule_width: v.widgets.noninteractive.bg_stroke.width.max(1.0),
            stripe: v.faint_bg_color,
            group_text_size: None,
            group_weight: Weight::Normal,
            group_end: GroupEnd::Gap(s.item_spacing.y * 3.0),
        }
    }
}

/// TODO(prose): the severity of a status a control shows: a badge, an
/// alert, a notification. Its colours come from [`Style::tones`].
///
/// ```
/// use eggshell_ui::{Style, Tone};
///
/// let style = Style::from_egui(&egui::Style::default());
/// for tone in Tone::ALL {
///     let colours = style.tones.get(tone);
///     assert_ne!(colours.tone, colours.foreground);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tone {
    /// TODO(prose): neutral news.
    Info,
    /// TODO(prose): a good outcome.
    Success,
    /// TODO(prose): a caution.
    Warning,
}

impl Tone {
    /// Every tone, least severe first.
    pub const ALL: [Self; 3] = [Self::Info, Self::Success, Self::Warning];
}

/// TODO(prose): the colours of one tone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToneColors {
    /// TODO(prose): the tone itself: its text or icon on the page, a solid
    /// fill, or the base of a tint.
    pub tone: Color32,
    /// TODO(prose): text on a solid fill of `tone`.
    pub foreground: Color32,
}

/// TODO(prose): the colours of every tone, one entry each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tones {
    /// TODO(prose)
    pub info: ToneColors,
    /// TODO(prose)
    pub success: ToneColors,
    /// TODO(prose)
    pub warning: ToneColors,
}

impl Tones {
    /// TODO(prose): the colours of `tone`.
    pub fn get(&self, tone: Tone) -> ToneColors {
        *self.slot(tone)
    }

    /// TODO(prose): sets the colours of `tone`.
    pub fn set(&mut self, tone: Tone, colors: ToneColors) {
        *self.slot_mut(tone) = colors;
    }

    fn slot(&self, tone: Tone) -> &ToneColors {
        match tone {
            Tone::Info => &self.info,
            Tone::Success => &self.success,
            Tone::Warning => &self.warning,
        }
    }

    fn slot_mut(&mut self, tone: Tone) -> &mut ToneColors {
        match tone {
            Tone::Info => &mut self.info,
            Tone::Success => &mut self.success,
            Tone::Warning => &mut self.warning,
        }
    }

    /// egui's warning colour, and Tailwind's sky and green for the tones
    /// egui has none of, in `visuals`' theme: the 700 shades in light, the
    /// 400 in dark.
    fn from_egui(visuals: &egui::Visuals) -> Self {
        let rgb = |hex: u32| Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
        let on = |tone: Color32, foreground: Color32| ToneColors { tone, foreground };
        let (light, dark) = (rgb(0xfafafa), rgb(0x0a0a0a));
        match visuals.dark_mode {
            false => Self {
                info: on(rgb(0x0069a8), light),
                success: on(rgb(0x008236), light),
                warning: on(visuals.warn_fg_color, dark),
            },
            true => Self {
                info: on(rgb(0x00bcff), dark),
                success: on(rgb(0x05df72), dark),
                warning: on(visuals.warn_fg_color, dark),
            },
        }
    }
}

/// TODO(prose): one value for each egui theme, such as a layout's style in light and dark.
///
/// ```
/// use eggshell_ui::contract::Themed;
///
/// let gaps = Themed { light: 4.0, dark: 6.0 };
/// assert_eq!(*gaps.get(egui::Theme::Dark), 6.0);
/// assert_eq!(gaps.map(|gap| gap * 2.0).light, 8.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Themed<T> {
    /// TODO(prose): the value while the theme is light.
    pub light: T,
    /// TODO(prose): the value while the theme is dark.
    pub dark: T,
}

impl<T: Clone> Themed<T> {
    /// TODO(prose): `value` in both themes.
    ///
    /// ```
    /// use eggshell_ui::contract::Themed;
    ///
    /// assert_eq!(Themed::both(4.0), Themed { light: 4.0, dark: 4.0 });
    /// ```
    pub fn both(value: T) -> Self {
        Self {
            light: value.clone(),
            dark: value,
        }
    }
}

impl<T> Themed<T> {
    /// TODO(prose): the value for `theme`.
    pub fn get(&self, theme: Theme) -> &T {
        match theme {
            Theme::Light => &self.light,
            Theme::Dark => &self.dark,
        }
    }

    /// TODO(prose): both values through `f`.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Themed<U> {
        Themed {
            light: f(self.light),
            dark: f(self.dark),
        }
    }
}

/// TODO(prose): a widget's own look, kept in a [`Style`] beside the core-painted fields and scoped with it.
///
/// ```
/// use eggshell_ui::Style;
/// use eggshell_ui::contract::Extension;
///
/// #[derive(Clone)]
/// struct Gap(f32);
///
/// impl Extension for Gap {
///     fn derive(style: &Style) -> Self {
///         Gap(style.table().rule_width)
///     }
/// }
///
/// let mut style = Style::default();
/// assert_eq!(style.extension::<Gap>().0, style.table().rule_width);
/// style.set_extension(Gap(12.0));
/// assert_eq!(style.extension::<Gap>().0, 12.0);
/// ```
pub trait Extension: Clone + Send + Sync + 'static {
    /// TODO(prose): the look a style that sets none gives the widget.
    fn derive(style: &Style) -> Self;
}

/// The extensions of a style, by type.
#[derive(Clone, Debug, Default)]
struct Extensions {
    map: egui::util::IdTypeMap,
}

fn extensions_id() -> egui::Id {
    crate::key::root_id().with("extensions")
}

/// TODO(prose): a design system's named values, neutral of any one: its colours and the few sizes egui's own widgets are given.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Tokens {
    /// TODO(prose): the page.
    pub background: Color32,
    /// TODO(prose): text on the page.
    pub foreground: Color32,
    /// TODO(prose): a card's fill.
    pub card: Color32,
    /// TODO(prose): text on a card.
    pub card_foreground: Color32,
    /// TODO(prose): a popover's fill: menus, windows.
    pub popover: Color32,
    /// TODO(prose): text on a popover.
    pub popover_foreground: Color32,
    /// TODO(prose): the primary action's fill.
    pub primary: Color32,
    /// TODO(prose): text on the primary fill.
    pub primary_foreground: Color32,
    /// TODO(prose): a secondary action's fill.
    pub secondary: Color32,
    /// TODO(prose): text on the secondary fill.
    pub secondary_foreground: Color32,
    /// TODO(prose): a muted surface.
    pub muted: Color32,
    /// TODO(prose): muted text.
    pub muted_foreground: Color32,
    /// TODO(prose): a hovered or active item's fill.
    pub accent: Color32,
    /// TODO(prose): text on the accent fill.
    pub accent_foreground: Color32,
    /// TODO(prose): a destructive action's colour.
    pub destructive: Color32,
    /// TODO(prose): text on the destructive fill.
    pub destructive_foreground: Color32,
    /// TODO(prose): borders.
    pub border: Color32,
    /// TODO(prose): an input's border and fill tint.
    pub input: Color32,
    /// TODO(prose): the focus ring.
    pub ring: Color32,
    /// TODO(prose): the keyboard focus indicator every control draws unless its look names its own.
    pub focus: Focus,
    /// TODO(prose): how disabled controls are drawn.
    pub disabled: Disabled,
    /// TODO(prose): the points one spacing unit is.
    pub spacing: Spacing,
    /// TODO(prose): the corner radii.
    pub radius: Radius,
    /// TODO(prose): the text sizes.
    pub text: TextScale,
    /// TODO(prose): a control's height.
    pub control_height: f32,
    /// TODO(prose): a button's padding.
    pub button_padding: Vec2,
    /// TODO(prose): the body text size.
    pub text_size: f32,
    /// TODO(prose): a switch's thumb, egui's checkbox and radio size.
    pub switch_thumb: f32,
    /// TODO(prose): the smallest shadow: a control at rest.
    pub shadow_xs: Shadow,
    /// TODO(prose): a small shadow: a slider's thumb, a tab.
    pub shadow_sm: Shadow,
    /// TODO(prose): a medium shadow: a menu, a popover.
    pub shadow_md: Shadow,
    /// TODO(prose): a large shadow: a dialog.
    pub shadow_lg: Shadow,
}

impl Tokens {
    /// Tokens read off egui's own style.
    pub fn from_egui(egui: &egui::Style) -> Self {
        let v = &egui.visuals;
        let w = &v.widgets;
        Self {
            background: v.panel_fill,
            foreground: v.text_color(),
            card: v.panel_fill,
            card_foreground: v.text_color(),
            popover: v.window_fill,
            popover_foreground: v.text_color(),
            primary: v.selection.bg_fill,
            primary_foreground: v.selection.stroke.color,
            secondary: w.inactive.weak_bg_fill,
            secondary_foreground: w.inactive.fg_stroke.color,
            muted: v.faint_bg_color,
            muted_foreground: v.weak_text_color(),
            accent: w.active.weak_bg_fill,
            accent_foreground: w.active.fg_stroke.color,
            destructive: v.error_fg_color,
            destructive_foreground: v.text_color(),
            border: v.window_stroke.color,
            input: w.inactive.bg_stroke.color,
            ring: w.active.bg_stroke.color,
            focus: Focus {
                width: 2.0,
                color: v.selection.bg_fill,
            },
            disabled: Disabled::Colors,
            spacing: Spacing::DEFAULT,
            radius: Radius::new(f32::from(w.inactive.corner_radius.nw)),
            text: TextScale::default(),
            control_height: egui.spacing.interact_size.y,
            button_padding: egui.spacing.button_padding,
            text_size: egui
                .text_styles
                .get(&egui::TextStyle::Body)
                .map_or(14.0, |font| font.size),
            switch_thumb: egui.spacing.icon_width,
            shadow_xs: Shadow::NONE,
            shadow_sm: Shadow::NONE,
            shadow_md: v.popup_shadow,
            shadow_lg: v.window_shadow,
        }
    }
}

/// TODO(prose): the fonts a style is drawn in, added to egui as a layout first builds with the style.
#[derive(Clone, Debug, Default)]
pub struct Fonts(pub Vec<egui::FontInsert>);

impl Extension for Fonts {
    fn derive(_: &Style) -> Self {
        Self::default()
    }
}

/// The look of every widget eggshell draws.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Style {
    /// The text role of controls.
    pub text: TextSpec,
    /// The registered family that draws each weight of a family.
    pub faces: Faces,
    /// The text beside a checkbox, radio button or switch; only `rest` and
    /// `disabled` apply.
    label: States,
    /// TODO(prose): toggle buttons.
    pub toggle: ToggleStyle,
    /// Checkboxes.
    checkbox: CheckboxStyle,
    /// Radio buttons.
    radio: RadioStyle,
    /// Switches.
    switch: SwitchStyle,
    /// Segmented controls.
    segmented: SegmentedStyle,
    /// Sliders.
    pub slider: SliderStyle,
    /// Progress bars.
    pub progress: ProgressStyle,
    /// Select triggers.
    select: SelectStyle,
    /// Menus, and the lists selects open.
    menu: MenuStyle,
    /// Tooltips.
    tooltip: TooltipStyle,
    /// Key chips.
    pub kbd: KbdStyle,
    /// TODO(prose)
    pub mount: MountStyle,
    /// Rich text: links and inline code.
    pub rich: RichTextStyle,
    /// Steppers.
    pub stepper: StepperStyle,
    /// Text fields.
    input: InputStyle,
    /// Selected text.
    pub selection: Selection,
    /// Scroll containers.
    pub scroll: ScrollStyle,
    /// TODO(prose): splitters.
    pub splitter: SplitterStyle,
    /// Tables.
    table: TableStyle,
    /// What a modal overlay paints over the screen behind its box. Usually
    /// translucent.
    pub scrim: Color32,
    /// How overlays that declare when they are shown enter and leave.
    pub motion: MotionStyle,
    /// TODO(prose): the colours of each [`Tone`].
    pub tones: Tones,
    /// TODO(prose): the design system's named values, from which egui's own look is set.
    pub tokens: Tokens,
    /// TODO(prose): the classes of every widget's parts, by layer.
    pub sheet: crate::sheet::Sheet,
    /// The widgets' own looks, beside the core-painted fields.
    extensions: Extensions,
}

#[derive(Clone)]
struct Cached {
    source: Arc<egui::Style>,
    derived: Arc<Style>,
}

/// A time egui gives in seconds: none when negative or NaN, and the longest
/// duration when it is longer.
fn seconds(seconds: f32) -> Duration {
    Duration::try_from_secs_f32(seconds.max(0.0)).unwrap_or(Duration::MAX)
}

impl Style {
    /// [`role`](Self::role) with a control size's text size laid over it.
    pub fn role_sized(&self, section: &Option<TextSpec>, size: Option<TextSize>) -> TextSpec {
        let role = self.role(section).clone();
        match size {
            Some(TextSize { size, line_height }) => TextSpec {
                size,
                line_height,
                ..role
            },
            None => role,
        }
    }

    /// TODO(prose): the widget look `T` this style sets, else the one `T` derives from it.
    pub fn extension<T: Extension>(&self) -> Arc<T> {
        self.extensions
            .map
            .get_temp::<Arc<T>>(extensions_id())
            .unwrap_or_else(|| Arc::new(T::derive(self)))
    }

    /// TODO(prose): sets the widget look `T`, in place of any set before.
    pub fn set_extension<T: Extension>(&mut self, value: T) {
        self.extensions
            .map
            .insert_temp(extensions_id(), Arc::new(value));
    }

    /// The text role of a widget section: its own, or [`Style::text`].
    pub fn role<'a>(&'a self, section: &'a Option<TextSpec>) -> &'a TextSpec {
        section.as_ref().unwrap_or(&self.text)
    }

    /// [`Style::from_egui`] of the context's egui style, kept until that
    /// style changes: the style of a layout given none.
    pub(crate) fn derived(ctx: &Context) -> Arc<Self> {
        let source = ctx.global_style();
        let id = crate::key::root_id().with("derived-style");
        match ctx.data(|data| data.get_temp::<Cached>(id)) {
            Some(cached) if Arc::ptr_eq(&cached.source, &source) => cached.derived,
            _ => {
                let derived = Arc::new(Self::from_egui(&source));
                ctx.data_mut(|data| {
                    data.insert_temp(
                        id,
                        Cached {
                            source,
                            derived: derived.clone(),
                        },
                    );
                });
                derived
            }
        }
    }

    /// A neutral style from egui's visuals and spacing.
    pub fn from_egui(egui: &egui::Style) -> Self {
        let (v, s) = (&egui.visuals, &egui.spacing);
        let colors = |w: &egui::style::WidgetVisuals| Colors {
            fill: w.weak_bg_fill,
            border: w.bg_stroke.color,
            text: w.fg_stroke.color,
        };
        let w = &v.widgets;
        let (inactive, hovered, active) =
            (colors(&w.inactive), colors(&w.hovered), colors(&w.active));
        let focus_border = v.selection.stroke.color;
        let faded = Opacity::new(v.disabled_alpha);
        let surface = v.panel_fill;
        let states = |rest: Colors, hover: Colors, pressed: Colors| States {
            rest,
            hover,
            pressed,
            focused: Colors {
                border: focus_border,
                ..rest
            },
            disabled: rest.faded_over(faded, surface),
        };
        let button_states = states(inactive, hovered, active);
        let radius = w.inactive.corner_radius.nw;
        let border_width = w.inactive.bg_stroke.width;
        let font = egui
            .text_styles
            .get(&egui::TextStyle::Button)
            .cloned()
            .unwrap_or_default();
        let text = TextSpec {
            size: font.size,
            line_height: (font.size * 1.25).round(),
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        };
        let metrics = Metrics {
            height: s.interact_size.y,
            min_width: 0.0,
            padding_x: s.button_padding.x,
            gap: s.item_spacing.x,
            radius,
            glyph: text.size,
            text: None,
        };
        let clear = Color32::TRANSPARENT;
        let selected = Colors {
            fill: v.selection.bg_fill,
            border: v.selection.bg_fill,
            text: v.selection.stroke.color,
        };
        let choice_border = border_width.max(1.0);
        let choice_off = Paint::new(states(
            Colors {
                fill: v.extreme_bg_color,
                border: w.inactive.bg_stroke.color,
                text: v.text_color(),
            },
            hovered,
            hovered,
        ));
        let choice_on = Paint::new(states(selected, selected, selected));
        let transition = Duration::from_millis(100);
        let label = States::new(
            Colors {
                fill: clear,
                border: clear,
                text: v.text_color(),
            },
            Colors {
                fill: clear,
                border: clear,
                text: v.text_color(),
            },
            surface,
        );
        let toggle_metrics = Metrics {
            min_width: metrics.height,
            ..metrics
        };
        let track = vec2(2.0 * s.icon_width, s.icon_width);
        let tokens = Tokens::from_egui(egui);
        let mut sheet = crate::sheet::Sheet::defaults();
        crate::owned::table_defaults(&mut sheet, crate::owned::TableDefaults::from_egui(egui));
        crate::owned::tooltip_defaults(
            &mut sheet,
            crate::owned::TooltipDefaults {
                popup: Look::new(States::still(
                    Colors {
                        fill: v.window_fill,
                        border: v.window_stroke.color,
                        text: v.text_color(),
                    },
                    v.window_fill,
                ))
                .bordered(v.window_stroke.width.max(1.0))
                .shadowed(v.popup_shadow),
                radius,
                padding: vec2(f32::from(s.menu_margin.left), f32::from(s.menu_margin.top)),
                text: text.clone(),
                max_width: s.tooltip_width,
            },
        );
        crate::owned::button_defaults(
            &mut sheet,
            crate::owned::ButtonDefaults {
                look: Look::new(button_states).bordered(border_width),
                sizes: Keyed::new(Size::DEFAULT, metrics),
                text: text.clone(),
                transition,
                press_shift: 0.0,
            },
        );
        let mut style = Self {
            text,
            faces: Faces::default(),
            label,
            toggle: ToggleStyle {
                sizes: Keyed::new(Size::DEFAULT, toggle_metrics),
                border_width: 0.0,
                off: Keyed::new(
                    Variant::DEFAULT,
                    Paint::new(states(
                        Colors {
                            fill: clear,
                            border: clear,
                            text: inactive.text,
                        },
                        hovered,
                        active,
                    )),
                ),
                on: Keyed::new(
                    Variant::DEFAULT,
                    Paint::new(states(selected, selected, selected)),
                ),
                text: None,
                press_shift: 0.0,
                transition,
            },
            checkbox: CheckboxStyle {
                sizes: Keyed::new(
                    Size::DEFAULT,
                    CheckboxMetrics {
                        size: s.icon_width,
                        radius: 2,
                        glyph: s.icon_width_inner,
                        gap: s.icon_spacing,
                        text: None,
                    },
                ),
                border_width: choice_border,
                text: None,
                off: choice_off,
                on: choice_on,
                transition,
            },
            radio: RadioStyle {
                sizes: Keyed::new(
                    Size::DEFAULT,
                    RadioMetrics {
                        size: s.icon_width,
                        dot: s.icon_width_inner,
                        gap: s.icon_spacing,
                        text: None,
                    },
                ),
                border_width: choice_border,
                text: None,
                off: choice_off,
                on: choice_on,
                transition,
            },
            switch: SwitchStyle {
                sizes: Keyed::new(
                    Size::DEFAULT,
                    SwitchMetrics {
                        track,
                        thumb_off: track.y * 0.75,
                        thumb_on: track.y * 0.75,
                        text: None,
                    },
                ),
                gap: s.icon_spacing,
                border_width: choice_border,
                text: None,
                off: choice_off,
                on: choice_on,
                slide: Duration::from_millis(150),
                transition,
            },
            segmented: SegmentedStyle {
                track_fill: v.faint_bg_color,
                sizes: Keyed::new(
                    Size::DEFAULT,
                    SegmentedMetrics {
                        track_radius: radius,
                        track_padding: 2.0,
                        height: s.interact_size.y,
                        cell_radius: radius,
                        cell_padding: vec2(s.button_padding.x, s.button_padding.y),
                        column_gap: 0.0,
                        row_gap: 2.0,
                        icon_size: metrics.glyph,
                        icon_gap: s.icon_spacing,
                        text: None,
                        cell_trim: 0.0,
                    },
                ),
                text: None,
                selected_weight: None,
                cell_border_width: 0.0,
                selected: Paint::new(States::still(
                    Colors {
                        fill: w.active.weak_bg_fill,
                        border: clear,
                        text: v.strong_text_color(),
                    },
                    surface,
                )),
                unselected: Paint::new(States::new(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: v.weak_text_color(),
                    },
                    hovered,
                    surface,
                )),
                transition,
            },
            progress: ProgressStyle {
                height: s.slider_rail_height,
                radius: u8::MAX,
                track: Colors {
                    fill: w.inactive.bg_fill,
                    border: clear,
                    text: clear,
                },
                range: Colors {
                    fill: selected.fill,
                    border: clear,
                    text: clear,
                },
            },
            slider: SliderStyle {
                sizes: Keyed::new(
                    Size::DEFAULT,
                    SliderMetrics {
                        track_height: s.slider_rail_height,
                        thumb_size: s.icon_width,
                        radius: u8::MAX,
                        text: None,
                        header_gap: 8.0,
                    },
                ),
                track: States::still(
                    Colors {
                        fill: w.inactive.bg_fill,
                        border: clear,
                        text: clear,
                    },
                    surface,
                ),
                range: States::still(
                    Colors {
                        fill: selected.fill,
                        border: clear,
                        text: clear,
                    },
                    surface,
                ),
                thumb: Look::new(states(
                    Colors {
                        fill: w.inactive.weak_bg_fill,
                        border: selected.fill,
                        text: clear,
                    },
                    hovered,
                    active,
                ))
                .bordered(border_width.max(1.0)),
                label: States::still(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: v.text_color(),
                    },
                    surface,
                ),
                readout: States::still(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: v.weak_text_color(),
                    },
                    surface,
                ),
                transition,
            },
            select: SelectStyle {
                trigger: Look::new(button_states).bordered(border_width.max(1.0)),
                sizes: Keyed::new(Size::DEFAULT, metrics),
                text: None,
                chevron: s.icon_width,
                chevron_opacity: Opacity::new(0.5),
                transition,
            },
            menu: MenuStyle {
                popup: Look::new(States::still(
                    Colors {
                        fill: v.window_fill,
                        border: v.window_stroke.color,
                        text: v.text_color(),
                    },
                    v.window_fill,
                ))
                .bordered(v.window_stroke.width.max(1.0))
                .shadowed(v.popup_shadow),
                radius,
                padding: 2.0,
                min_width: s.combo_width,
                item: Look::new(States::new(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: v.text_color(),
                    },
                    selected,
                    v.window_fill,
                )),
                item_radius: radius,
                item_padding: vec2(s.button_padding.x, s.button_padding.y),
                item_gap: s.item_spacing.x,
                destructive: States::new(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: v.error_fg_color,
                    },
                    Colors {
                        fill: v.error_fg_color.gamma_multiply(0.15),
                        border: clear,
                        text: v.error_fg_color,
                    },
                    v.window_fill,
                ),
                separator: w.noninteractive.bg_stroke.color,
                separator_margin: s.item_spacing.y,
                offset: Distance::new(4.0),
                heading: v.weak_text_color(),
                heading_weight: Weight::Normal,
                text: None,
                shortcut: v.weak_text_color(),
                shortcut_text: {
                    let small = egui
                        .text_styles
                        .get(&egui::TextStyle::Small)
                        .cloned()
                        .unwrap_or_default();
                    TextSpec {
                        size: small.size,
                        line_height: (small.size * 1.25).round(),
                        family: FontFamily::Proportional,
                        weight: Weight::Normal,
                    }
                },
                shortcut_tracking: Tracking::ZERO,
                indicator: s.icon_width,
            },
            tooltip: TooltipStyle {
                arrow: None,
                offset: Distance::new(4.0),
                delay: seconds(egui.interaction.tooltip_delay),
                grace: seconds(egui.interaction.tooltip_grace_time),
            },
            kbd: KbdStyle {
                page: Colors {
                    fill: v.code_bg_color,
                    border: Color32::TRANSPARENT,
                    text: v.text_color(),
                },
                tooltip: Colors {
                    fill: v.code_bg_color,
                    border: Color32::TRANSPARENT,
                    text: v.text_color(),
                },
                border_width: 0.0,
                radius,
                padding_x: 4.0,
                height: s.interact_size.y.min(20.0),
                text: None,
            },
            mount: MountStyle {
                caption: v.weak_text_color(),
                caption_text: None,
                gap: s.item_spacing.y,
                border: v.widgets.noninteractive.bg_stroke.color,
                border_width: v.widgets.noninteractive.bg_stroke.width.max(1.0),
                radius,
                padding: f32::from(s.window_margin.left),
                bare_padding: tokens.focus.clearance(),
                selected_ring: v.selection.stroke.color,
            },
            rich: RichTextStyle {
                link: v.hyperlink_color,
                link_underline: None,
                link_weight: Weight::Normal,
                code: InlineCode {
                    fill: v.code_bg_color,
                    radius,
                    padding_x: 2.0,
                    padding_y: 1.0,
                    size: Em::ONE,
                },
            },
            stepper: StepperStyle {
                group: choice_off.bordered(choice_border),
                sizes: Keyed::new(
                    Size::DEFAULT,
                    StepperMetrics {
                        height: s.interact_size.y,
                        padding_x: s.button_padding.x,
                        radius,
                        button_width: s.icon_width * 1.5,
                        glyph: s.icon_width_inner,
                        text: None,
                    },
                ),
                button: Look::new(button_states),
                divider: choice_off.states.rest.border,
                transition,
            },
            input: InputStyle {
                look: Look::new(States {
                    rest: Colors {
                        fill: v.text_edit_bg_color(),
                        border: w.inactive.bg_stroke.color,
                        text: w.inactive.fg_stroke.color,
                    },
                    hover: Colors {
                        fill: v.text_edit_bg_color(),
                        border: w.hovered.bg_stroke.color,
                        text: w.inactive.fg_stroke.color,
                    },
                    pressed: Colors {
                        fill: v.text_edit_bg_color(),
                        border: focus_border,
                        text: w.inactive.fg_stroke.color,
                    },
                    focused: Colors {
                        fill: v.text_edit_bg_color(),
                        border: focus_border,
                        text: w.inactive.fg_stroke.color,
                    },
                    disabled: Colors {
                        fill: v.text_edit_bg_color(),
                        border: w.inactive.bg_stroke.color,
                        text: w.inactive.fg_stroke.color,
                    }
                    .faded_over(faded, surface),
                })
                .bordered(border_width),
                sizes: Keyed::new(
                    Size::DEFAULT,
                    InputMetrics {
                        radius,
                        padding: s.button_padding,
                        height: s.interact_size.y,
                        text: None,
                        icon_size: 16.0,
                        icon_gap: 8.0,
                    },
                ),
                hint: None,
                icon: v.weak_text_color(),
            },
            selection: Selection {
                fill: v.selection.bg_fill,
                text: Some(v.selection.stroke.color),
            },
            scroll: ScrollStyle::from_egui(egui),
            splitter: SplitterStyle::from_egui(egui),
            table: TableStyle::from_egui(egui),
            scrim: Color32::from_black_alpha(100),
            motion: MotionStyle::INSTANT,
            tones: Tones::from_egui(v),
            tokens,
            sheet,
            extensions: Extensions::default(),
        };
        style.rewrite_rules();
        style
    }

    /// TODO(prose): writes every family's rules from its section, at the widgets' defaults layer, and the recorded exceptions again over a preset's look. A preset writes its own rules over them.
    pub fn rewrite_rules(&mut self) {
        self.restyle(|style| {
            style.write_input_rules();
            style.write_segmented_rules();
            style.write_menu_rules();
            style.write_table_rules();
            style.write_choice_rules();
            crate::owned::write_select_rules(style);
        });
    }

    /// Runs `write`, which writes a section's rules, at the widgets'
    /// defaults layer, then writes the recorded exceptions again over a
    /// preset's look, if a preset wrote one, so a changed section reaches
    /// them. The sheet's layer is as it was after.
    fn restyle(&mut self, write: impl FnOnce(&mut Self)) {
        let layer = self.sheet.layer();
        self.sheet.set_layer(crate::sheet::Layer::Defaults);
        write(self);
        if self.sheet.has_rules_at(crate::sheet::Layer::Preset) {
            self.sheet.set_layer(crate::sheet::Layer::Preset);
            self.write_exceptions();
        }
        self.sheet.set_layer(layer);
    }

    /// Writes the [`InputStyle`] and the selection colours into `egui`, the
    /// style of the `Ui` a text field is drawn in, so that egui's `TextEdit`
    /// draws as the style says, at `size`. See [`InputStyle`] for what is
    /// approximated.
    pub fn map_input(&self, egui: &mut egui::Style, size: Size) {
        self.map_input_as(egui, size, None);
    }

    /// [`map_input`](Self::map_input) for a field in the ARIA state `aria`,
    /// such as an invalid one.
    pub(crate) fn map_input_as(
        &self,
        egui: &mut egui::Style,
        size: Size,
        aria: Option<crate::Aria>,
    ) {
        use crate::owned::{Frame, InputPart, input_keys};
        let input = &self.input;
        let size = input.sizes.key_for(size);
        let metrics = input.sizes.get(size);
        let mut keys = input_keys(size, false);
        keys.extend(aria.map(crate::sheet::Key::Aria));
        let classes = self.sheet.resolve(InputPart::Root, &keys, &self.tokens);
        let frame = Frame::of(&classes);
        let v = &mut egui.visuals;
        v.text_edit_bg_color = frame.fill;
        v.weak_text_color = input.hint;
        let radius = egui::CornerRadius::same(frame.radius);
        let clear = Color32::TRANSPARENT;
        let width = frame.border_width;
        for (widget, border) in [
            (&mut v.widgets.inactive, frame.rest),
            (&mut v.widgets.hovered, frame.hover),
            (&mut v.widgets.active, frame.focused),
        ] {
            widget.bg_stroke = egui::Stroke::new(width, border.unwrap_or(clear));
            if let Some(text) = frame.text {
                widget.fg_stroke.color = text;
            }
            widget.corner_radius = radius;
            widget.expansion = 0.0;
        }
        v.selection.bg_fill = self.selection.fill;
        v.selection.stroke = egui::Stroke::new(width.max(1.0), frame.focused.unwrap_or(clear));
        egui.spacing.interact_size.y = metrics.height;
        egui.spacing.button_padding = metrics.padding;
    }

    /// Selects: the trigger, and the list it opens.
    pub fn select(&self) -> &SelectStyle {
        &self.select
    }

    /// TODO(prose): sets the select section, and writes the rules that draw a select from it and the menu section at the widgets' defaults layer, under a preset's look. A later change to either does not reach the select until this is called again.
    pub fn set_select(&mut self, select: SelectStyle) {
        self.select = select;
        self.restyle(crate::owned::write_select_rules);
    }

    /// The text beside a checkbox, radio button or switch; only `rest` and
    /// `disabled` apply.
    pub fn label(&self) -> States {
        self.label
    }

    /// Checkboxes.
    pub fn checkbox(&self) -> &CheckboxStyle {
        &self.checkbox
    }

    /// Radio buttons.
    pub fn radio(&self) -> &RadioStyle {
        &self.radio
    }

    /// Switches.
    pub fn switch(&self) -> &SwitchStyle {
        &self.switch
    }

    /// Segmented controls, and the tabs' list and triggers.
    pub fn segmented(&self) -> &SegmentedStyle {
        &self.segmented
    }

    /// The input section: text fields and the boxes that hold them.
    pub fn input(&self) -> &InputStyle {
        &self.input
    }

    /// TODO(prose): sets the input section, and writes the rules that draw it at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_input(&mut self, input: InputStyle) {
        self.input = input;
        self.restyle(Self::write_input_rules);
    }

    /// Menus, and the lists selects open.
    pub fn menu(&self) -> &MenuStyle {
        &self.menu
    }

    /// TODO(prose): sets the menu section, and writes the rules that draw it, a menu's box, items and the rest, at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_menu(&mut self, menu: MenuStyle) {
        self.menu = menu;
        self.restyle(Self::write_menu_rules);
    }

    /// Tooltips.
    pub fn tooltip(&self) -> &TooltipStyle {
        &self.tooltip
    }

    /// TODO(prose): sets the tooltip section: its arrow, offset and timing.
    pub fn set_tooltip(&mut self, tooltip: TooltipStyle) {
        self.tooltip = tooltip;
    }

    /// Tables.
    pub fn table(&self) -> &TableStyle {
        &self.table
    }

    /// TODO(prose): sets the table section, and writes its rules (between rows, stripes, the first column's inset) at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_table(&mut self, table: TableStyle) {
        self.table = table;
        self.restyle(Self::write_table_rules);
    }

    fn write_table_rules(&mut self) {
        crate::owned::write_table_rules(&mut self.sheet, &self.table);
    }

    fn write_menu_rules(&mut self) {
        let text = self.role(&self.menu.text).clone();
        crate::owned::write_menu_rules(&mut self.sheet, &self.menu, &text);
    }

    /// TODO(prose): writes the recorded exceptions over the rules at the sheet's current layer. The input's: a disabled group fades as a whole, and the sizes other than the default keep their metrics. The select's: a size the trigger's rules leave out keeps its metrics. The tabs': the track keeps its fill, the trigger's 1 px trim comes out of its padding, the sizes other than the default keep their metrics, and wrapping lists keep their rows. The menu's: an item of the default variant keeps the generated look. The table's: its bands' borders, the last row's, a selected row's fill under the pointer, stripes and the first column's inset. The choice controls': a radio's dot is centred in its circle, and a size their rules leave out keeps its metrics. A preset calls this after its generated look.
    pub fn write_exceptions(&mut self) {
        crate::owned::write_input_exceptions(&mut self.sheet, &self.input);
        crate::owned::write_select_exceptions(&mut self.sheet, &self.select);
        crate::owned::write_tabs_exceptions(&mut self.sheet, &self.segmented);
        crate::owned::write_menu_exceptions(&mut self.sheet);
        // A disabled slider's blends go over its track's.
        crate::owned::write_track_exceptions(&mut self.sheet, (&self.slider, &self.progress));
        crate::owned::write_slider_exceptions(&mut self.sheet, &self.slider);
        crate::owned::write_stepper_exceptions(&mut self.sheet, &self.stepper);
        crate::owned::write_table_exceptions(&mut self.sheet, &self.table);
        crate::owned::write_choice_exceptions(
            &mut self.sheet,
            (&self.checkbox, &self.radio, &self.switch),
        );
    }

    fn write_input_rules(&mut self) {
        crate::owned::write_input_rules(&mut self.sheet, &self.input);
    }

    /// TODO(prose): sets the colours of a choice control's label, and writes the rules that draw it at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_label(&mut self, label: States) {
        self.label = label;
        self.restyle(|style| crate::owned::write_field_rules(&mut style.sheet, style.label));
    }

    /// TODO(prose): sets the checkbox section, and writes the rules that draw it at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_checkbox(&mut self, checkbox: CheckboxStyle) {
        self.checkbox = checkbox;
        self.restyle(|style| crate::owned::write_checkbox_rules(&mut style.sheet, &style.checkbox));
    }

    /// TODO(prose): sets the radio section, and writes the rules that draw it at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_radio(&mut self, radio: RadioStyle) {
        self.radio = radio;
        self.restyle(|style| crate::owned::write_radio_rules(&mut style.sheet, &style.radio));
    }

    /// TODO(prose): sets the switch section, and writes the rules that draw it at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_switch(&mut self, switch: SwitchStyle) {
        self.switch = switch;
        self.restyle(|style| crate::owned::write_switch_rules(&mut style.sheet, &style.switch));
    }

    fn write_choice_rules(&mut self) {
        crate::owned::write_field_rules(&mut self.sheet, self.label);
        crate::owned::write_checkbox_rules(&mut self.sheet, &self.checkbox);
        crate::owned::write_radio_rules(&mut self.sheet, &self.radio);
        crate::owned::write_switch_rules(&mut self.sheet, &self.switch);
    }

    /// TODO(prose): sets the segmented section, and writes the rules that draw it, the tabs' list and triggers, at the widgets' defaults layer, under a preset's look; the recorded exceptions are written again over it.
    pub fn set_segmented(&mut self, segmented: SegmentedStyle) {
        self.segmented = segmented;
        self.restyle(Self::write_segmented_rules);
    }

    fn write_segmented_rules(&mut self) {
        let seg = &self.segmented;
        let roles = seg
            .sizes
            .keys()
            .map(|size| {
                let unselected = self.role_sized(&seg.text, seg.sizes.get(size).text);
                let selected = match seg.selected_weight {
                    Some(weight) => TextSpec {
                        weight,
                        ..unselected.clone()
                    },
                    None => unselected.clone(),
                };
                crate::owned::TabsRoles {
                    size,
                    unselected,
                    selected,
                }
            })
            .collect();
        crate::owned::write_tabs_rules(&mut self.sheet, &self.segmented, roles);
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::from_egui(&egui::Style::default())
    }
}

#[cfg(test)]
mod metric_tokens {
    use super::{Radius, Spacing, TextScale};

    #[test]
    fn spacing_clamps_to_zero_and_takes_nan_as_the_default() {
        assert_eq!(Spacing::new(-1.0).points(4.0), 0.0);
        assert_eq!(Spacing::new(f32::NAN), Spacing::DEFAULT);
        assert_eq!(Spacing::new(3.5).points(9.0), 31.5);
    }

    #[test]
    fn the_radius_steps_are_shadcns_offsets_floored_at_zero() {
        let r = Radius::new(10.0);
        let steps = [r.sm(), r.md(), r.lg(), r.xl(), r.xl2(), r.xl3(), r.xl4()];
        assert_eq!(steps, [6.0, 8.0, 10.0, 14.0, 18.0, 22.0, 26.0]);
        assert_eq!((Radius::new(3.0).sm(), Radius::new(3.0).md()), (0.0, 1.0));
        assert_eq!(Radius::new(f32::NAN), Radius::DEFAULT);
        assert_eq!(Radius::new(-5.0).lg(), 0.0);
    }

    #[test]
    fn the_text_scale_is_tailwinds() {
        let t = TextScale::default();
        let pairs = [t.xs, t.sm, t.base, t.lg].map(|s| (s.size, s.line_height));
        assert_eq!(
            pairs,
            [(12.0, 16.0), (14.0, 20.0), (16.0, 24.0), (18.0, 28.0)]
        );
    }
}
