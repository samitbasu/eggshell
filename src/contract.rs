//! [`Style`]: everything the owned widgets draw with.
//!
//! Core defines what its widgets need and a preset (such as the `eggshell-ui-shadcn` crate) fills it, so a design
//! system is data: an app installs a [`Style`] with [`Style::install`] and the
//! widgets follow, with no code change. [`Style::default`] is derived from
//! egui's visuals, so core looks coherent with no preset at all.
//!
//! Every field is exact: the owned widgets draw exactly these values. Only the
//! text input, which stays a wrapped egui `TextEdit`, is approximated (egui
//! paints it from fixed widget slots): see [`InputStyle`].
//!
//! The style is one per context (per theme if installed with
//! [`Style::install_for`]). Scoping a different style to a subtree is future
//! work; [`Style::of`] is the single place that would change.

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
    /// This paint's own focus ring, where it is not [`Style::focus`].
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

/// Buttons: open sets of variants and sizes, and the toggle form.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ButtonStyle {
    /// The variants the style defines. The default entry is the variant of
    /// [`Builder::button`](crate::Builder::button).
    pub variants: Keyed<Variant, Look>,
    /// Metrics of a button per size.
    pub sizes: Keyed<Size, Metrics>,
    /// Metrics of a toggle per size.
    pub toggle_sizes: Keyed<Size, Metrics>,
    /// Border width of a toggle, off or on.
    pub toggle_border_width: f32,
    /// The looks of a toggle that is off, by variant. Variants are chosen
    /// with [`ButtonChain::variant`](crate::owned::ButtonChain::variant) for toggles as
    /// for buttons.
    pub toggle_off: Keyed<Variant, Paint>,
    /// The looks of a toggle that is on.
    pub toggle_on: Keyed<Variant, Paint>,
    /// The text role of buttons, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// How far a pressed button moves down, in points; zero for none.
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
    /// Side of a radio item's dot.
    pub dot: f32,
}

/// A tooltip's arrow: a square turned 45 degrees, filled like the box, half
/// of it outside the box and pointing at the trigger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arrow {
    /// Side of the square, in points. The gap between the trigger and the
    /// box grows by it, as Radix's does.
    pub size: f32,
}

/// Tooltips: the box, the text it wraps, the optional arrow and the timing.
/// How one enters and leaves is [`MotionStyle::tooltip`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TooltipStyle {
    /// The box: its `rest` colours, border and shadow.
    pub popup: Look,
    /// Corner radius of the box.
    pub radius: u8,
    /// Left-right and top-bottom padding of the box.
    pub padding: Vec2,
    /// The text role, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// The widest the box gets; longer text wraps. A pointed arrow needs a
    /// borderless box.
    pub max_width: f32,
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

/// Tables: the header, the rule between rows, and a row's fills.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TableStyle {
    /// Behind the header row.
    pub header_fill: Color32,
    /// The weight of the header's text.
    pub header_weight: Weight,
    /// The height of the header row.
    pub header_height: f32,
    /// Padding around a body or footer cell's content, and beside a header
    /// cell's.
    pub cell_padding: f32,
    /// TODO(prose): padding above and below a body or footer cell's content,
    /// in place of `cell_padding`.
    pub cell_padding_y: Option<f32>,
    /// TODO(prose): padding above and below a header cell's content, which
    /// then sizes the header row in place of `header_height`.
    pub header_padding_y: Option<f32>,
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
    /// Behind every second body row, starting with the second.
    pub stripe: Color32,
    /// Behind a hovered body row.
    pub hover: Color32,
    /// Behind the selected row.
    pub selected: Color32,
    /// Behind the footer row.
    pub footer_fill: Color32,
    /// The weight of the footer's text.
    pub footer_weight: Weight,
    /// How long a row's fill takes to change.
    pub transition: Duration,
}

impl TableStyle {
    /// Rows ruled in egui's separator colour, hovered and selected in its
    /// widget and selection fills.
    pub fn from_egui(egui: &egui::Style) -> Self {
        let (v, s) = (&egui.visuals, &egui.spacing);
        Self {
            header_fill: Color32::TRANSPARENT,
            header_weight: Weight::Normal,
            header_height: s.interact_size.y + 2.0 * s.button_padding.y,
            cell_padding: s.button_padding.x,
            cell_padding_y: None,
            header_padding_y: None,
            first_column_inset: None,
            rule_below_last: false,
            rule: v.widgets.noninteractive.bg_stroke.color,
            rule_width: v.widgets.noninteractive.bg_stroke.width.max(1.0),
            stripe: Color32::TRANSPARENT,
            hover: v.widgets.hovered.weak_bg_fill,
            selected: v.selection.bg_fill,
            footer_fill: v.faint_bg_color,
            footer_weight: Weight::Normal,
            transition: Duration::from_millis(100),
        }
    }
}

/// The look of every widget eggshell draws.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// The text role of controls.
    pub text: TextSpec,
    /// The registered family that draws each weight of a family.
    pub faces: Faces,
    /// The default focus indicator.
    pub focus: Focus,
    /// How disabled controls are drawn.
    pub disabled: Disabled,
    /// The text beside a checkbox, radio button or switch; only `rest` and
    /// `disabled` apply.
    pub label: States,
    /// Buttons and toggles.
    pub button: ButtonStyle,
    /// Checkboxes.
    pub checkbox: CheckboxStyle,
    /// Radio buttons.
    pub radio: RadioStyle,
    /// Switches.
    pub switch: SwitchStyle,
    /// Segmented controls.
    pub segmented: SegmentedStyle,
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
}

#[derive(Clone)]
struct Cached {
    source: Arc<egui::Style>,
    derived: Arc<Style>,
}

fn style_id(theme: Theme) -> Id {
    crate::key::root_id().with(("style", theme))
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

    /// The text role of a widget section: its own, or [`Style::text`].
    pub fn role<'a>(&'a self, section: &'a Option<TextSpec>) -> &'a TextSpec {
        section.as_ref().unwrap_or(&self.text)
    }

    /// The style installed in `ctx` for its theme, else [`Style::from_egui`]
    /// of the context's egui style.
    ///
    /// Cheap to call: the installed style is shared, and the derived one is
    /// kept until the context's egui style changes.
    pub fn of(ctx: &Context) -> Arc<Self> {
        if let Some(style) = Self::installed(ctx) {
            return style;
        }
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

    /// The style installed in `ctx` for its theme, if one is.
    pub(crate) fn installed(ctx: &Context) -> Option<Arc<Self>> {
        ctx.data(|data| data.get_temp::<Arc<Self>>(style_id(ctx.theme())))
    }

    /// Makes `self` the style of every eggshell widget in `ctx`, in both
    /// themes.
    ///
    /// Installing a style equal to the installed one does nothing, so an app
    /// may install its style every frame for the price of a comparison.
    pub fn install(self, ctx: &Context) {
        self.install_in(ctx, &[Theme::Light, Theme::Dark]);
    }

    /// Makes `self` the style of every eggshell widget in `ctx` while its
    /// theme is `theme`, and like [`install`](Self::install) does nothing if
    /// that is the installed style already.
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        self.install_in(ctx, &[theme]);
    }

    fn install_in(self, ctx: &Context, themes: &[Theme]) {
        ctx.data_mut(|data| {
            let stale: Vec<Theme> = themes
                .iter()
                .copied()
                .filter(|&theme| {
                    data.get_temp::<Arc<Self>>(style_id(theme))
                        .is_none_or(|installed| *installed != self)
                })
                .collect();
            if stale.is_empty() {
                return;
            }
            let shared = Arc::new(self);
            for theme in stale {
                data.insert_temp(style_id(theme), shared.clone());
            }
        });
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
        let focus = Focus {
            width: 2.0,
            color: v.selection.bg_fill,
        };
        Self {
            text,
            faces: Faces::default(),
            focus,
            disabled: Disabled::Colors,
            label,
            button: ButtonStyle {
                variants: Keyed::new(
                    Variant::DEFAULT,
                    Look::new(button_states).bordered(border_width),
                ),
                sizes: Keyed::new(Size::DEFAULT, metrics),
                toggle_sizes: Keyed::new(Size::DEFAULT, toggle_metrics),
                toggle_border_width: 0.0,
                toggle_off: Keyed::new(
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
                toggle_on: Keyed::new(
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
                dot: s.icon_width_inner / 2.0,
            },
            tooltip: TooltipStyle {
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
                text: None,
                max_width: s.tooltip_width,
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
                bare_padding: focus.clearance(),
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
        }
    }

    /// Writes the [`InputStyle`] and the selection colours into `egui`, the
    /// style of the `Ui` a text field is drawn in, so that egui's `TextEdit`
    /// draws as the style says, at `size`. See [`InputStyle`] for what is
    /// approximated.
    pub fn map_input(&self, egui: &mut egui::Style, size: Size) {
        let input = &self.input;
        let metrics = input.sizes.get(size);
        let states = &input.look.paint.states;
        let width = input.look.border_width;
        let v = &mut egui.visuals;
        v.text_edit_bg_color = Some(states.rest.fill);
        v.weak_text_color = input.hint;
        let radius = egui::CornerRadius::same(metrics.radius);
        for (widget, border) in [
            (&mut v.widgets.inactive, states.rest.border),
            (&mut v.widgets.hovered, states.hover.border),
            (&mut v.widgets.active, states.focused.border),
        ] {
            widget.bg_stroke = egui::Stroke::new(width, border);
            widget.fg_stroke.color = states.rest.text;
            widget.corner_radius = radius;
            widget.expansion = 0.0;
        }
        v.selection.bg_fill = self.selection.fill;
        v.selection.stroke = egui::Stroke::new(width.max(1.0), states.focused.border);
        egui.spacing.interact_size.y = metrics.height;
        egui.spacing.button_padding = metrics.padding;
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::from_egui(&egui::Style::default())
    }
}
