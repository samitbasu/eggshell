//! [`Style`]: everything the owned widgets draw with.
//!
//! Core defines what its widgets need and a preset (such as the `eggshell-ui-shadcn` crate) fills it, so a design
//! system is data: an app gives a layout its [`Style`] with
//! [`LayoutState::new`](crate::LayoutState::new) and the widgets
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

use egui::{Color32, Context, FontFamily, Id, Shadow, Theme, Vec2};

pub use crate::presence::{Entry, KindMotion, MotionStyle};
use crate::theme::{HoverSurface, Tokens};
use crate::{Faces, Opacity, Progress, TextSpec, Weight};

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

/// TODO(prose): toggle buttons: their sizes and their looks off and on. A
/// plain button's look is the [`sheet`](Style::sheet)'s
/// ([`button::Part`](crate::owned::button::Part)); a toggle's moves there with
/// its own shadcn source, `toggle.tsx`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToggleStyle {
    // #497 stage 5: each size's text is read nowhere (the type is shared with the select's).
    /// Metrics of a toggle per size.
    pub sizes: Keyed<Size, Metrics>,
    /// Border width of a toggle, off or on.
    pub border_width: f32,
    /// The looks of a toggle that is off, by variant. Variants are chosen
    /// with [`button::Chain::variant`](crate::owned::button::Chain::variant) for toggles as
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
    /// The text role of the label, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
}

/// A radio button's circle at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadioMetrics {
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
    /// The text role of the label, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
}

/// A switch's track and thumb at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchMetrics {
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
    /// The text role of the label, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// How long the thumb takes to slide.
    pub slide: Duration,
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
    /// The label above the track; only `text` of `rest` and `disabled` applies.
    pub label: States,
    /// The readout above the track; only `text` of `rest` and `disabled` applies.
    pub readout: States,
}

/// Determinate progress bars: a track filled up to a fraction.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProgressStyle {
    /// Corner radius of the track and its range; `u8::MAX` makes them round.
    pub radius: u8,
    /// The range's colours; only `fill` applies.
    pub range: Colors,
}

/// TODO(prose): Select triggers: each size's metrics and the chevron. The trigger's look is the sheet's [`select::Part::Trigger`](crate::owned::select::Part::Trigger); the list it opens is drawn from [`MenuStyle`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectStyle {
    // #497 stage 5: height, gap, radius and text feed only write_select_exceptions; padding_x is read only by the command list; min_width and glyph nowhere.
    /// Size of the trigger per size: height, padding, gap between value and
    /// chevron, and corner radius.
    pub sizes: Keyed<Size, Metrics>,
    // #497 stage 5: under the generated look neither chevron takes this size (their triggers size the svg); read by extras' disclosure and command list.
    /// Side of the chevron glyph.
    pub chevron: f32,
    /// How much of the trigger's text colour the chevron keeps.
    pub chevron_opacity: Opacity,
}

/// TODO(prose): Menus: the metrics of a dropdown's, a context menu's and a submenu's box, items and shortcuts, and of the list a select opens. Their look is the sheet's [`menu::Part`](crate::owned::menu::Part)s and [`select::Part`](crate::owned::select::Part)s.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MenuStyle {
    // #497 stage 5: read only by the command list.
    /// Padding between the box and its items.
    pub padding: f32,
    /// The gap between a trigger and its dropdown menu or select list.
    pub offset: Distance,
    // #497 stage 5: read only by the command list.
    /// Left-right and top-bottom padding of an item.
    pub item_padding: Vec2,
    // #497 stage 5: read only by the command list.
    /// Space between the parts of an item: its indicator, label and shortcut.
    pub item_gap: f32,
    // #497 stage 5: read only by the command list.
    /// The text role of items and headings, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    // #497 stage 5: read only by the command list.
    /// The text role of a shortcut.
    pub shortcut_text: TextSpec,
    // #497 stage 5: read only by the command list.
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

/// TODO(prose): Tooltips: the optional arrow, where the box sits and the timing. The box's look is the sheet's [`tooltip::Part::Content`](crate::owned::tooltip::Part::Content); how one enters and leaves is [`MotionStyle::tooltip`].
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
    /// The group's dimensions per size.
    pub sizes: Keyed<Size, StepperMetrics>,
    /// The two buttons, which have no border of their own.
    pub button: Look,
}

/// A text field's dimensions at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputMetrics {
    /// The text's size, or `None` for the text role's own.
    pub text: Option<TextSize>,
}

/// TODO(prose): Text fields, which are egui's `TextEdit` wrapped: egui paints them from
/// fixed widget slots, so [`Style::map_input`] approximates the sheet's
/// [`input::Part::Root`](crate::owned::input::Part::Root) and this section on
/// the field's own `Ui`, and the library's size estimate reads the same
/// values.
///
/// | From | On the `TextEdit` |
/// |---|---|
/// | the part's fill | exact: the field's background in every state |
/// | the part's border colour, at rest and hovered, and its width | exact; the focus border is at least 1 pt wide so that focus always shows |
/// | the part's text colour | exact: the colour of typed text in every state |
/// | the part's other states and variants | not drawn: egui has one fill and one text colour |
/// | the part's focused border | approximated: egui draws the focus border in the colour of selected text, so [`Selection::text`] wins when set |
/// | the part's shadow and focus ring | not drawn: egui's frame has no shadow and its ring is the 1 pt border |
/// | the part's radius, padding and height | exact |
/// | `hint` | exact when set; otherwise egui's own grey-out of the text colour |
/// | [`Style::selection`] fill | exact |
///
/// The text cursor keeps the colour of the `Ui`'s visuals.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct InputStyle {
    /// TODO(prose): the text and icons per size; the sheet gives the radius, padding and height.
    pub sizes: Keyed<Size, InputMetrics>,
    /// The colour of hint text, or `None` for egui's grey-out of the text.
    pub hint: Option<Color32>,
    // #497 stage 5: read only by the command list.
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

/// TODO(prose): Tables: a group's title and end. The rest of a table's look is the sheet's [`table::Part`](crate::owned::table::Part)s.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TableStyle {
    /// TODO(prose): the size of a group row's title; `None` keeps the
    /// table's.
    pub group_text_size: Option<TextSize>,
    /// TODO(prose): the weight of a group row's title.
    pub group_weight: Weight,
    /// TODO(prose): what shows where a group ends before rows in no group.
    pub group_end: GroupEnd,
}

/// TODO(prose): the severity of a status a control shows: a badge, an
/// alert, a notification. Its colours come from [`Style::tones`].
///
/// ```
/// use eggshell_ui::{Style, Tone};
///
/// let style = Style::default();
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

/// What an opt-in widget's style is built from: the tokens and the core
/// sections it follows.
#[cfg(any(
    feature = "badge",
    feature = "hover-card",
    feature = "notify",
    feature = "skeleton",
    feature = "spinner"
))]
pub(crate) struct Base<'a> {
    #[cfg(any(
        feature = "badge",
        feature = "hover-card",
        feature = "notify",
        feature = "skeleton"
    ))]
    pub tokens: &'a Tokens,
    #[cfg(any(feature = "badge", feature = "notify"))]
    pub text: &'a TextSpec,
    #[cfg(any(feature = "badge", feature = "spinner"))]
    pub toggle: &'a ToggleStyle,
    #[cfg(any(feature = "hover-card", feature = "notify"))]
    pub menu: &'a MenuStyle,
    #[cfg(feature = "hover-card")]
    pub motion: &'a MotionStyle,
}

/// The look of every widget eggshell draws.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Style {
    /// The text role of controls.
    pub text: TextSpec,
    /// The registered family that draws each weight of a family.
    pub faces: Faces,
    /// TODO(prose): toggle buttons.
    pub toggle: ToggleStyle,
    /// Checkboxes.
    pub checkbox: CheckboxStyle,
    /// Radio buttons.
    pub radio: RadioStyle,
    /// Switches.
    pub switch: SwitchStyle,
    /// Sliders.
    pub slider: SliderStyle,
    /// Progress bars.
    pub progress: ProgressStyle,
    /// Select triggers.
    pub select: SelectStyle,
    /// Menus, and the lists selects open.
    pub menu: MenuStyle,
    /// Tooltips.
    pub tooltip: TooltipStyle,
    /// Key chips.
    pub kbd: KbdStyle,
    /// TODO(prose)
    pub mount: MountStyle,
    /// Rich text: links and inline code.
    pub rich: RichTextStyle,
    /// Steppers.
    pub stepper: StepperStyle,
    /// Text fields.
    pub input: InputStyle,
    /// Selected text.
    pub selection: Selection,
    /// Scroll containers.
    pub scroll: ScrollStyle,
    /// TODO(prose): splitters.
    pub splitter: SplitterStyle,
    /// Tables.
    pub table: TableStyle,
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
    /// TODO(prose): the fonts the style is drawn in, added to egui as a
    /// layout first builds with the style.
    pub fonts: Vec<egui::FontInsert>,
    /// TODO(prose): badges and chips.
    #[cfg(feature = "badge")]
    pub badge: crate::owned::badge::Style,
    /// TODO(prose): disclosures and accordions.
    #[cfg(feature = "disclosure")]
    pub disclosure: crate::owned::disclosure::Style,
    /// TODO(prose): hover cards.
    #[cfg(feature = "hover-card")]
    pub hover_card: crate::owned::hover_card::Style,
    /// TODO(prose): the typeset Markdown documents are spaced by; its
    /// parts' rules are in the sheet, which [`Typeset::apply`] writes with it.
    ///
    /// [`Typeset::apply`]: crate::owned::markdown::Typeset::apply
    #[cfg(feature = "markdown")]
    pub markdown: crate::owned::markdown::Typeset,
    /// TODO(prose): navigation menus' delays and panels.
    #[cfg(feature = "navigation-menu")]
    pub navigation_menu: crate::owned::navigation_menu::Style,
    /// TODO(prose): notifications.
    #[cfg(feature = "notify")]
    pub notify: crate::owned::notify::Style,
    /// TODO(prose): skeletons.
    #[cfg(feature = "skeleton")]
    pub skeleton: crate::owned::skeleton::Style,
    /// TODO(prose): spinners.
    #[cfg(feature = "spinner")]
    pub spinner: crate::owned::spinner::Style,
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

    /// The text role of a widget section: its own, or [`Style::text`].
    pub fn role<'a>(&'a self, section: &'a Option<TextSpec>) -> &'a TextSpec {
        section.as_ref().unwrap_or(&self.text)
    }

    /// [`Style::default`], built once: the style of a `Ui` that is no node's.
    pub(crate) fn shared_default() -> Arc<Self> {
        static DEFAULT: std::sync::LazyLock<Arc<Style>> =
            std::sync::LazyLock::new(|| Arc::new(Style::default()));
        DEFAULT.clone()
    }

    /// TODO(prose): every section filled from `tokens` the way shadcn draws
    /// it, in the tokens' theme, and the widgets' look in the sheet: the
    /// generated sheets of the tokens' style and the recorded exceptions.
    pub fn new(tokens: &Tokens) -> Self {
        use crate::theme::size;
        let t = tokens;
        let dark = t.theme == Theme::Dark;
        let clear = Color32::TRANSPARENT;
        let focused = |rest: Colors| Colors {
            border: t.ring,
            ..rest
        };
        let flat = |fill, text| Colors {
            fill,
            border: clear,
            text,
        };
        let states = |rest: Colors, hover: Colors| States {
            focused: focused(rest),
            ..States::new(rest, hover, t.background)
        };
        let filled = |rest: Colors, hover: Colors| Look::new(states(rest, hover));

        let (hover_fill, hover_text, hover_fill_dark) = match t.hover_surface {
            HoverSurface::Accent => (t.accent, t.accent_foreground, t.accent_50()),
            HoverSurface::Muted => (t.muted, t.foreground, t.muted_50()),
        };
        let ghost_button = filled(
            flat(clear, t.foreground),
            flat(if dark { hover_fill_dark } else { hover_fill }, hover_text),
        );
        let control = Metrics {
            height: t.control_height(),
            min_width: 0.0,
            padding_x: t.points(t.button_padding.x),
            gap: t.points(t.control_gap),
            radius: t.radius_md(),
            glyph: t.points(t.icon_size),
            text: None,
        };
        let xs_text = Some(t.text.xs);
        let u = |units: f32| t.points(units);
        let small = |m: Metrics| Metrics {
            height: m.height - u(1.0),
            min_width: if m.min_width == 0.0 {
                0.0
            } else {
                m.height - u(1.0)
            },
            padding_x: (m.padding_x - u(1.0)).max(0.0),
            gap: m.gap.min(u(1.5)),
            ..m
        };
        let tiny = |m: Metrics| Metrics {
            height: m.height - u(3.0),
            min_width: if m.min_width == 0.0 {
                0.0
            } else {
                m.height - u(3.0)
            },
            padding_x: (m.padding_x - u(2.0)).max(0.0),
            gap: m.gap.min(u(1.0)),
            glyph: u(3.0),
            text: xs_text,
            ..m
        };
        let toggle_metrics = Metrics {
            min_width: t.control_height(),
            padding_x: t.points(t.toggle_padding_x),
            ..control
        };
        let square = |metrics: Metrics| Metrics {
            min_width: t.control_height(),
            padding_x: 0.0,
            ..metrics
        };
        let sized = |metrics: Metrics| {
            Keyed::new(Size::DEFAULT, metrics)
                .with(size::ICON, square(metrics))
                .with(size::SM, small(metrics))
                .with(size::XS, tiny(metrics))
                .with(size::ICON_SM, square(small(metrics)))
                .with(size::ICON_XS, square(tiny(metrics)))
        };
        let accented = flat(t.accent, t.accent_foreground);
        let transition = Duration::from_millis(150);

        let toggle = ToggleStyle {
            sizes: sized(toggle_metrics),
            border_width: 0.0,
            off: Keyed::new(
                Variant::DEFAULT,
                Paint::new(states(
                    flat(clear, t.foreground),
                    flat(t.muted, t.muted_foreground),
                )),
            ),
            on: Keyed::new(Variant::DEFAULT, Paint::new(states(accented, accented))),
            text: Some(t.text_role_at(t.control_weight)),
            press_shift: 1.0,
            transition,
        };
        let checkbox = {
            let box_at = |gap: f32, text| CheckboxMetrics { gap, text };
            CheckboxStyle {
                sizes: Keyed::new(Size::DEFAULT, box_at(8.0, None))
                    .with(size::SM, box_at(u(1.5), None))
                    .with(size::XS, box_at(u(1.5), xs_text)),
                text: Some(t.text_role_at(t.label_weight)),
            }
        };
        let radio = {
            let circle = |gap: f32, text| RadioMetrics { gap, text };
            RadioStyle {
                sizes: Keyed::new(Size::DEFAULT, circle(8.0, None))
                    .with(size::SM, circle(u(1.5), None))
                    .with(size::XS, circle(u(1.5), xs_text)),
                text: Some(t.text_role_at(t.label_weight)),
            }
        };
        let switch = {
            let switch_at = |text| SwitchMetrics { text };
            SwitchStyle {
                sizes: Keyed::new(Size::DEFAULT, switch_at(None))
                    .with(size::SM, switch_at(None))
                    .with(size::XS, switch_at(xs_text)),
                gap: 8.0,
                text: Some(t.text_role_at(t.label_weight)),
                slide: Duration::from_millis(150),
            }
        };
        let slider = {
            let slider_at = |track_height: f32,
                             thumb_size: f32,
                             text: Option<TextSize>,
                             header_gap: f32| SliderMetrics {
                track_height,
                thumb_size,
                radius: t.slider_radius,
                text,
                header_gap,
            };
            SliderStyle {
                sizes: Keyed::new(
                    Size::DEFAULT,
                    slider_at(
                        t.points(t.slider_track),
                        t.points(t.slider_thumb),
                        None,
                        8.0,
                    ),
                )
                .with(
                    size::SM,
                    slider_at(
                        t.points(t.slider_track).min(u(1.0)),
                        t.points(t.slider_thumb) - u(0.5),
                        None,
                        u(1.5),
                    ),
                )
                .with(
                    size::XS,
                    slider_at(
                        t.points(t.slider_track).min(u(0.75)),
                        t.points(t.slider_thumb) - u(1.0),
                        xs_text,
                        u(1.0),
                    ),
                ),
                label: States::still(flat(clear, t.foreground), t.background),
                readout: States::still(flat(clear, t.muted_foreground), t.background),
            }
        };
        let stepper = {
            let group_at = |(button_width, glyph): (f32, f32), text| StepperMetrics {
                radius: t.radius_md(),
                button_width,
                glyph,
                text,
            };
            StepperStyle {
                sizes: Keyed::new(Size::DEFAULT, group_at((u(6.0), u(3.0)), None))
                    .with(size::SM, group_at((u(5.0), u(3.0)), None))
                    .with(size::XS, group_at((u(4.0), u(2.5)), xs_text)),
                button: ghost_button,
            }
        };
        let input = {
            let field_at = |text| InputMetrics { text };
            InputStyle {
                sizes: Keyed::new(Size::DEFAULT, field_at(None))
                    .with(size::SM, field_at(None))
                    .with(size::XS, field_at(xs_text)),
                // input.tsx: `placeholder:text-muted-foreground`.
                hint: Some(t.muted_foreground),
                icon: t.muted_foreground,
            }
        };
        // shadcn's `animate-in` (150 ms) for popovers, dropdown menus and
        // selects: `zoom-in-95 slide-in-from-*-2`. dialog::Content:
        // `zoom-in-95 duration-200`. SheetContent:
        // `slide-in-from-*`, `ease-in-out`, 500 ms open and 300 ms close.
        let animate = KindMotion {
            entry: Entry::Pop,
            enter: Duration::from_millis(150),
            exit: Duration::from_millis(150),
            easing: crate::Easing::Default,
        };
        let motion = MotionStyle {
            popover: animate,
            menu: animate,
            tooltip: KindMotion {
                entry: Entry::Zoom,
                ..animate
            },
            dialog: KindMotion {
                entry: Entry::Zoom,
                enter: Duration::from_millis(200),
                exit: Duration::from_millis(200),
                easing: crate::Easing::Default,
            },
            sheet: KindMotion {
                entry: Entry::Slide,
                enter: Duration::from_millis(500),
                exit: Duration::from_millis(300),
                easing: crate::Easing::InOut,
            },
        };
        let text = t.text_role();
        let select = SelectStyle {
            sizes: Keyed::new(Size::DEFAULT, control)
                .with(size::SM, small(control))
                .with(size::XS, tiny(control)),
            chevron: 16.0,
            chevron_opacity: Opacity::new(0.5),
        };
        // shadcn's DropdownMenu: `p-1`, items `px-2 py-1.5 gap-2`, a 16 pt
        // check and chevron, and a `text-xs` shortcut. The look is the
        // generated sheet's.
        let menu = MenuStyle {
            padding: t.points(t.popup_padding),
            item_padding: t.points_vec(t.item_padding),
            item_gap: t.points(t.item_padding.x),
            offset: Distance::new(4.0),
            text: None,
            shortcut_text: TextSpec {
                size: 12.0,
                line_height: 16.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            indicator: 16.0,
        };
        // shadcn's typography: a link is `font-medium text-primary underline`,
        // inline code `rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm`.
        let rich = RichTextStyle {
            link: t.primary,
            link_underline: None,
            link_weight: Weight::Medium,
            code: InlineCode {
                fill: t.muted,
                radius: t.radius_sm(),
                padding_x: 4.8,
                padding_y: 3.2,
                size: Em::new(0.875),
            },
        };
        #[cfg(any(
            feature = "badge",
            feature = "hover-card",
            feature = "notify",
            feature = "skeleton",
            feature = "spinner"
        ))]
        let base = Base {
            #[cfg(any(
                feature = "badge",
                feature = "hover-card",
                feature = "notify",
                feature = "skeleton"
            ))]
            tokens: t,
            #[cfg(any(feature = "badge", feature = "notify"))]
            text: &text,
            #[cfg(any(feature = "badge", feature = "spinner"))]
            toggle: &toggle,
            #[cfg(any(feature = "hover-card", feature = "notify"))]
            menu: &menu,
            #[cfg(feature = "hover-card")]
            motion: &motion,
        };
        let fonts = t
            .font
            .as_ref()
            .map(crate::theme::FontSet::inserts)
            .unwrap_or_default();
        #[cfg(feature = "badge")]
        let badge = crate::theme::Shadcn::badge(t, &base);
        #[cfg(feature = "disclosure")]
        let disclosure = crate::theme::Shadcn::disclosure();
        #[cfg(feature = "hover-card")]
        let hover_card = crate::theme::Shadcn::hover_card(t, &base);
        #[cfg(feature = "markdown")]
        let markdown = crate::owned::markdown::Typeset::default();
        #[cfg(feature = "navigation-menu")]
        let navigation_menu = crate::owned::navigation_menu::Style::default();
        #[cfg(feature = "notify")]
        let notify = crate::theme::Shadcn::notify(t, &base);
        #[cfg(feature = "skeleton")]
        let skeleton = crate::theme::Shadcn::skeleton(t, &base);
        #[cfg(feature = "spinner")]
        let spinner = crate::owned::spinner::Style::from_base(&base);
        let mut style = Self {
            text,
            faces: t
                .font
                .as_ref()
                .map_or_else(Faces::egui_regular, crate::theme::FontSet::declared),
            toggle,
            checkbox,
            radio,
            switch,
            slider,
            progress: ProgressStyle {
                radius: u8::MAX,
                range: flat(t.primary, t.primary_foreground),
            },
            select,
            menu,
            // shadcn's TooltipContent: a 10 pt arrow, flush with the trigger.
            // Radix's default delay, deliberately not shadcn's 0 ms, which
            // assumes tooltips are rare (desktop apps use them densely); the
            // grace is Radix's `skipDelayDuration`.
            tooltip: TooltipStyle {
                arrow: Some(Arrow { size: 10.0 }),
                offset: Distance::new(0.0),
                delay: Duration::from_millis(700),
                grace: Duration::from_millis(300),
            },
            // shadcn's Kbd: `bg-muted text-muted-foreground h-5 min-w-5 rounded-sm
            // px-1 text-xs font-medium`, and inside a tooltip
            // `bg-background/20 text-background` (`/10` in the dark theme).
            kbd: KbdStyle {
                page: flat(t.muted, t.muted_foreground),
                tooltip: flat(
                    if dark {
                        t.background_10()
                    } else {
                        t.background_20()
                    },
                    t.background,
                ),
                border_width: 0.0,
                radius: t.radius_sm(),
                padding_x: 4.0,
                height: 20.0,
                text: Some(TextSpec {
                    size: 12.0,
                    line_height: 16.0,
                    family: FontFamily::Proportional,
                    weight: t.control_weight,
                }),
            },
            // A captioned mount: `text-muted-foreground` above `border rounded-md
            // p-4`, as the docs page's examples were rendered for the pick.
            mount: MountStyle {
                caption: t.muted_foreground,
                caption_text: None,
                gap: 6.0,
                border: t.border,
                border_width: 1.0,
                radius: t.radius_md(),
                padding: 16.0,
                bare_padding: 8.0,
                selected_ring: t.primary,
            },
            rich,
            stepper,
            input,
            selection: Selection {
                fill: t.primary.gamma_multiply(0.3),
                text: Some(t.foreground),
            },
            // shadcn's ScrollArea: `w-2.5` bar, `bg-border` `rounded-full` thumb;
            // Radix's keeps its thumb at least 18 px long.
            scroll: ScrollStyle {
                width: 10.0,
                thumb: t.border,
                thumb_hover: t.border,
                thumb_active: t.border,
                track: clear,
                radius: egui::CornerRadius::same(u8::MAX),
                min_thumb_length: 18.0,
                fade: EdgeFade::OFF,
            },
            // shadcn's ResizableHandle: a `w-px bg-border` line in a wider hit area,
            // which a faint band fills while hot.
            splitter: SplitterStyle {
                thickness: 6.0,
                line_width: 1.0,
                rest: SplitterColors {
                    band: clear,
                    line: t.border,
                    grip: clear,
                },
                hot: SplitterColors {
                    band: t.primary.gamma_multiply(0.15),
                    line: t.primary,
                    grip: clear,
                },
                grip: None,
                transition: Duration::from_millis(150),
            },
            // shadcn's Table: `border-b` rows, none under the last; the stripes
            // and groups are eggshell's own.
            table: TableStyle {
                group_text_size: Some(TextSize {
                    size: 12.0,
                    line_height: 16.0,
                }),
                group_weight: Weight::Medium,
                group_end: GroupEnd::Gap(12.0),
            },
            // shadcn's overlays are `bg-black/50`; with no backdrop blur (#40) the
            // scrim alone separates a dialog from the page, so it is darker.
            scrim: Color32::from_black_alpha(204),
            motion,
            tones: t.tones(),
            tokens: t.clone(),
            sheet: crate::sheet::Sheet::new(),
            fonts,
            #[cfg(feature = "badge")]
            badge,
            #[cfg(feature = "disclosure")]
            disclosure,
            #[cfg(feature = "hover-card")]
            hover_card,
            #[cfg(feature = "markdown")]
            markdown,
            #[cfg(feature = "navigation-menu")]
            navigation_menu,
            #[cfg(feature = "notify")]
            notify,
            #[cfg(feature = "skeleton")]
            skeleton,
            #[cfg(feature = "spinner")]
            spinner,
        };
        style.write_look();
        style
    }

    /// What an opt-in widget's style is built from, from this style.
    /// Writes the widgets' look: the generated sheets of the tokens' style
    /// and theme, eggshell's own rules around them, and the recorded
    /// exceptions over them.
    fn write_look(&mut self) {
        if self.tokens.theme == Theme::Dark {
            crate::owned::field::write_dark_card_label(&mut self.sheet);
        }
        crate::owned::table::write_group_band(&mut self.sheet);
        crate::sheets::write(&mut self.sheet, self.tokens.style, self.tokens.theme);
        crate::owned::input::write_field_sizes(&mut self.sheet, &self.tokens);
        self.write_exceptions();
        #[cfg(feature = "disclosure")]
        {
            // The section's own look, not its preset's: what the generated
            // look's exceptions have always been written from.
            let disclosure = crate::owned::disclosure::Style::shadcn();
            crate::owned::disclosure::write_exceptions(&mut self.sheet, &disclosure);
            crate::owned::disclosure::write_collapsible_exceptions(
                &mut self.sheet,
                self.toggle.transition,
            );
        }
        #[cfg(feature = "command")]
        crate::owned::command::write_exceptions(self);
        #[cfg(feature = "notify")]
        crate::owned::notify::write_exceptions(&self.notify, &mut self.sheet);
        #[cfg(feature = "alert")]
        crate::owned::alert::write_exceptions(&mut self.sheet);
        #[cfg(feature = "menubar")]
        crate::owned::menubar::write_exceptions(&mut self.sheet);
        #[cfg(feature = "navigation-menu")]
        crate::owned::navigation_menu::write_exceptions(&mut self.sheet);
        #[cfg(feature = "badge")]
        crate::owned::badge::write_exceptions(&mut self.sheet);
        #[cfg(feature = "form")]
        crate::owned::form::write_exceptions(&mut self.sheet);
        #[cfg(feature = "markdown")]
        crate::owned::markdown::Typeset::default().write_rules(&mut self.sheet);
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
        use crate::owned::input::{self, Frame, keys as input_keys};
        let input = &self.input;
        let size = input.sizes.key_for(size);
        let mut keys = input_keys(size, false);
        keys.extend(aria.map(crate::sheet::Key::Aria));
        let classes = self.sheet.resolve(input::Part::Root, &keys, &self.tokens);
        let frame = Frame::of(&classes, self.tokens.spacing);
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
        egui.spacing.interact_size.y = frame.height;
        egui.spacing.button_padding = frame.padding;
    }

    /// TODO(prose): writes the recorded exceptions over the rules at the sheet's current layer. The input's: a disabled group fades as a whole, and the sizes other than the default keep their metrics. The select's: a size the trigger's rules leave out keeps its metrics. The tabs': the track keeps its fill, the trigger's 1 px trim comes out of its padding, the sizes other than the default keep their metrics, and wrapping lists keep their rows. The menu's: an item of the default variant keeps the generated look. The table's: its bands' borders, the last row's, a selected row's fill under the pointer, stripes and the first column's inset. The choice controls': a radio's dot is centred in its circle, and a size their rules leave out keeps its metrics. A preset calls this after its generated look.
    fn write_exceptions(&mut self) {
        crate::owned::input::write_exceptions(&mut self.sheet, &self.input);
        crate::owned::select::write_select_exceptions(&mut self.sheet, &self.select);
        crate::owned::tabs::write_exceptions(&mut self.sheet);
        crate::owned::menu::write_exceptions(&mut self.sheet);
        // A disabled slider's blends go over its track's.
        crate::owned::write_track_exceptions(&mut self.sheet);
        crate::owned::slider::write_exceptions(&mut self.sheet);
        crate::owned::stepper::write_exceptions(&mut self.sheet, &self.stepper);
        crate::owned::table::write_exceptions(&mut self.sheet);
        crate::owned::field::write_exceptions(
            &mut self.sheet,
            (&self.checkbox, &self.radio, &self.switch),
        );
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new(&Tokens::light())
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
