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

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Context, FontFamily, Id, Shadow, Theme, Vec2, vec2};

use crate::{Opacity, Progress, TextSpec};

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

/// Entries by key with one that stands in for any key the style lacks. In a
/// debug build, asking for an unknown key is a bug and asserts; a release
/// build quietly uses the default entry.
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
        self.others.get(&key).unwrap_or_else(|| {
            debug_assert!(false, "the style defines no {key:?}");
            &self.default
        })
    }

    /// The entry under `key`, if the style defines it.
    pub fn get_mut(&mut self, key: K) -> Option<&mut T> {
        if key == self.default_key {
            Some(&mut self.default)
        } else {
            self.others.get_mut(&key)
        }
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
}

impl Paint {
    /// A shadowless paint with the style's focus ring.
    pub fn new(states: States) -> Self {
        Self {
            states,
            shadow: None,
            focus: None,
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
/// **Faded is not CSS group opacity** (issue #40). CSS draws the element into
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

/// Checkboxes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CheckboxStyle {
    /// Side of the box.
    pub size: f32,
    /// Corner radius of the box.
    pub radius: u8,
    /// Side of the check glyph.
    pub glyph: f32,
    /// Gap between the box and its label.
    pub gap: f32,
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

/// Radio buttons.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RadioStyle {
    /// Diameter of the circle.
    pub size: f32,
    /// Diameter of the dot in the chosen one.
    pub dot: f32,
    /// Gap between the circle and its label.
    pub gap: f32,
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

/// Segmented controls: a track of cells, one of them selected.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SegmentedStyle {
    /// The track's background.
    pub track_fill: Color32,
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
    /// Side of the icon in a cell that has one. The icon takes the cell's
    /// `text` colour.
    pub icon_size: f32,
    /// Gap between a cell's icon and its label.
    pub icon_gap: f32,
    /// The text role of the cells, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// Border width of a cell, chosen or not.
    pub cell_border_width: f32,
    /// The chosen cell.
    pub selected: Paint,
    /// Any other cell.
    pub unselected: Paint,
    /// How long state changes animate.
    pub transition: Duration,
}

/// Sliders. A track holds a range that ends under a thumb.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderStyle {
    /// Thickness of the track.
    pub track_height: f32,
    /// Diameter of the thumb, which is also the slider's height.
    pub thumb_size: f32,
    /// Corner radius of the track, its range and the thumb; `u8::MAX` makes
    /// them round.
    pub radius: u8,
    /// The track's fill; only `rest` and `disabled` apply.
    pub track: States,
    /// The range's fill; only `rest` and `disabled` apply.
    pub range: States,
    /// The thumb. Its focus ring is the keyboard focus indicator.
    pub thumb: Look,
    /// How long state changes animate.
    pub transition: Duration,
}

/// Select triggers and the list of options they open.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectStyle {
    /// The trigger, an input-like box.
    pub trigger: Look,
    /// Size of the trigger: height, padding, gap between value and chevron,
    /// and corner radius.
    pub metrics: Metrics,
    /// The text role of the trigger, or `None` for [`Style::text`].
    pub text: Option<TextSpec>,
    /// Side of the chevron glyph.
    pub chevron: f32,
    /// How much of the trigger's text colour the chevron keeps.
    pub chevron_opacity: Opacity,
    /// The list's box: its `rest` colours, border and shadow.
    pub popup: Look,
    /// Corner radius of the list.
    pub popup_radius: u8,
    /// Padding between the list and its items.
    pub popup_padding: f32,
    /// An item; its `hover` colours mark the highlighted one.
    pub item: Look,
    /// Corner radius of an item.
    pub item_radius: u8,
    /// Left-right and top-bottom padding of an item.
    pub item_padding: Vec2,
    /// How long state changes animate.
    pub transition: Duration,
}

/// Steppers: a number field and two buttons, one above the other, in a single
/// bordered group.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StepperStyle {
    /// The group's box. Its focus ring and border show while the text has
    /// focus.
    pub group: Look,
    /// Height of the group.
    pub height: f32,
    /// Left and right padding of the text.
    pub padding_x: f32,
    /// Corner radius of the group; the buttons round their outer corners to
    /// fit inside its border.
    pub radius: u8,
    /// The two buttons, which have no border of their own.
    pub button: Look,
    /// Width of the buttons.
    pub button_width: f32,
    /// The hairlines between the text and the buttons, and between the buttons.
    pub divider: Color32,
    /// Side of the chevron glyph.
    pub glyph: f32,
    /// How long state changes animate.
    pub transition: Duration,
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
/// | `radius`, `padding`, `height` | exact |
/// | `hint` | exact when set; otherwise egui's own grey-out of the text colour |
/// | [`Style::selection`] fill | exact |
///
/// The text cursor keeps the colour of the `Ui`'s visuals.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct InputStyle {
    /// Colours per state and the border width.
    pub look: Look,
    /// Corner radius.
    pub radius: u8,
    /// Space between the border and the text.
    pub padding: Vec2,
    /// Height of a one-line field; a taller field's text sets its height.
    pub height: f32,
    /// The colour of hint text, or `None` for egui's grey-out of the text.
    pub hint: Option<Color32>,
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

/// The look of every widget eggshell draws.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// The text role of controls.
    pub text: TextSpec,
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
    /// Select triggers.
    pub select: SelectStyle,
    /// Steppers.
    pub stepper: StepperStyle,
    /// Text fields.
    pub input: InputStyle,
    /// Selected text.
    pub selection: Selection,
}

#[derive(Clone)]
struct Cached {
    source: Arc<egui::Style>,
    derived: Arc<Style>,
}

fn style_id(theme: Theme) -> Id {
    crate::key::root_id().with(("style", theme))
}

impl Style {
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
        if let Some(style) = ctx.data(|data| data.get_temp::<Arc<Self>>(style_id(ctx.theme()))) {
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
        };
        let metrics = Metrics {
            height: s.interact_size.y,
            min_width: 0.0,
            padding_x: s.button_padding.x,
            gap: s.item_spacing.x,
            radius,
            glyph: text.size,
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
        Self {
            text,
            focus: Focus {
                width: 2.0,
                color: v.selection.bg_fill,
            },
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
                size: s.icon_width,
                radius: 2,
                glyph: s.icon_width_inner,
                gap: s.icon_spacing,
                border_width: choice_border,
                text: None,
                off: choice_off,
                on: choice_on,
                transition,
            },
            radio: RadioStyle {
                size: s.icon_width,
                dot: s.icon_width_inner,
                gap: s.icon_spacing,
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
            slider: SliderStyle {
                track_height: s.slider_rail_height,
                thumb_size: s.icon_width,
                radius: u8::MAX,
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
                transition,
            },
            select: SelectStyle {
                trigger: Look::new(button_states).bordered(border_width.max(1.0)),
                metrics,
                text: None,
                chevron: s.icon_width,
                chevron_opacity: Opacity::new(0.5),
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
                popup_radius: radius,
                popup_padding: 2.0,
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
                transition,
            },
            stepper: StepperStyle {
                group: choice_off.bordered(choice_border),
                height: s.interact_size.y,
                padding_x: s.button_padding.x,
                radius,
                button: Look::new(button_states),
                button_width: s.icon_width * 1.5,
                divider: choice_off.states.rest.border,
                glyph: s.icon_width_inner,
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
                radius,
                padding: s.button_padding,
                height: s.interact_size.y,
                hint: None,
            },
            selection: Selection {
                fill: v.selection.bg_fill,
                text: Some(v.selection.stroke.color),
            },
        }
    }

    /// Writes the [`InputStyle`] and the selection colours into `egui`, the
    /// style of the `Ui` a text field is drawn in, so that egui's `TextEdit`
    /// draws as the style says. See [`InputStyle`] for what is approximated.
    pub fn map_input(&self, egui: &mut egui::Style) {
        let input = &self.input;
        let states = &input.look.paint.states;
        let width = input.look.border_width;
        let v = &mut egui.visuals;
        v.text_edit_bg_color = Some(states.rest.fill);
        v.weak_text_color = input.hint;
        let radius = egui::CornerRadius::same(input.radius);
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
        v.selection.stroke = egui::Stroke::new(
            width.max(1.0),
            self.selection.text.unwrap_or(states.focused.border),
        );
        egui.spacing.interact_size.y = input.height;
        egui.spacing.button_padding = input.padding;
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::from_egui(&egui::Style::default())
    }
}
