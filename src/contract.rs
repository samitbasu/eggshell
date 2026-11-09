//! [`Style`]: everything the owned widgets draw with.
//!
//! Core defines what its widgets need and a preset fills it, so a design
//! system is data: an app installs a [`Style`] with [`Style::install`] and the
//! widgets follow, with no code change. [`Style::default`] is derived from
//! egui's visuals, so core looks coherent with no preset at all.
//!
//! Every field is exact: the owned widgets draw exactly these values. Only the
//! text input, which stays a wrapped egui `TextEdit`, will be approximated
//! (egui paints it from fixed widget slots), and it is not part of the
//! contract yet.
//!
//! The style is one per context (per theme if installed with
//! [`Style::install_for`]). Scoping a different style to a subtree is future
//! work; [`Style::of`] is the single place that would change.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Context, FontFamily, Id, Shadow, Theme, Vec2, vec2};

use crate::{Opacity, TextSpec};

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
    /// These colours at `opacity`.
    #[must_use]
    pub fn faded(self, opacity: Opacity) -> Self {
        let fade = |color: Color32| color.gamma_multiply(opacity.into());
        Self {
            fill: fade(self.fill),
            border: fade(self.border),
            text: fade(self.text),
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
    /// focused is rest, disabled is rest at half opacity.
    pub fn new(rest: Colors, hover: Colors) -> Self {
        Self {
            rest,
            hover,
            pressed: hover,
            focused: rest,
            disabled: rest.faded(Opacity::new(0.5)),
        }
    }

    /// The same colours in every state but disabled.
    pub fn still(rest: Colors) -> Self {
        Self::new(rest, rest)
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

/// Everything that makes one kind of box look the way it does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Colours per state.
    pub states: States,
    /// Border width in points.
    pub border_width: f32,
    /// Drop shadow at rest.
    pub shadow: Option<Shadow>,
    /// This look's own focus ring, where it is not [`Style::focus`].
    pub focus: Option<Focus>,
}

impl Look {
    /// A borderless, shadowless look with the style's focus ring.
    pub fn new(states: States) -> Self {
        Self {
            states,
            border_width: 0.0,
            shadow: None,
            focus: None,
        }
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
            shadow: Some(shadow),
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
}

/// Buttons: open sets of variants and sizes, and the toggle form.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ButtonStyle {
    /// The variants the style defines. The default entry is the variant of
    /// [`Button::new`](crate::owned::Button::new).
    pub variants: Keyed<Variant, Look>,
    /// Metrics of a button per size.
    pub sizes: Keyed<Size, Metrics>,
    /// Metrics of a toggle per size.
    pub toggle_sizes: Keyed<Size, Metrics>,
    /// A toggle that is off.
    pub toggle_off: Look,
    /// A toggle that is on.
    pub toggle_on: Look,
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
    /// Unticked. The glyph takes the `text` colour of the ticked look.
    pub off: Look,
    /// Ticked.
    pub on: Look,
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
    /// Unchosen.
    pub off: Look,
    /// Chosen. The dot takes the `text` colour.
    pub on: Look,
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
    /// Off.
    pub off: Look,
    /// On.
    pub on: Look,
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
    /// Gap between the rows of a wrapping track.
    pub row_gap: f32,
    /// The chosen cell.
    pub selected: Look,
    /// Any other cell.
    pub unselected: Look,
    /// How long state changes animate.
    pub transition: Duration,
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

/// Links.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Link {
    /// At rest.
    pub rest: Color32,
    /// Under the pointer.
    pub hover: Color32,
}

/// The look of every widget bullseye draws.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// The text role of controls.
    pub text: TextSpec,
    /// The default focus indicator.
    pub focus: Focus,
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
    /// Selected text.
    pub selection: Selection,
    /// Links.
    pub link: Link,
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

    /// Makes `self` the style of every bullseye widget in `ctx`, in both themes.
    pub fn install(self, ctx: &Context) {
        let shared = Arc::new(self);
        ctx.data_mut(|data| {
            for theme in [Theme::Light, Theme::Dark] {
                data.insert_temp(style_id(theme), shared.clone());
            }
        });
    }

    /// Makes `self` the style of every bullseye widget in `ctx` while its
    /// theme is `theme`.
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        ctx.data_mut(|data| data.insert_temp(style_id(theme), Arc::new(self)));
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
        let states = |rest: Colors, hover: Colors, pressed: Colors| States {
            rest,
            hover,
            pressed,
            focused: Colors {
                border: focus_border,
                ..rest
            },
            disabled: rest.faded(faded),
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
        };
        let clear = Color32::TRANSPARENT;
        let selected = Colors {
            fill: v.selection.bg_fill,
            border: v.selection.bg_fill,
            text: v.selection.stroke.color,
        };
        let choice_off = Look::new(states(
            Colors {
                fill: v.extreme_bg_color,
                border: w.inactive.bg_stroke.color,
                text: v.text_color(),
            },
            hovered,
            hovered,
        ))
        .bordered(border_width.max(1.0));
        let choice_on =
            Look::new(states(selected, selected, selected)).bordered(border_width.max(1.0));
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
            label,
            button: ButtonStyle {
                variants: Keyed::new(
                    Variant::DEFAULT,
                    Look::new(button_states).bordered(border_width),
                ),
                sizes: Keyed::new(Size::DEFAULT, metrics),
                toggle_sizes: Keyed::new(Size::DEFAULT, toggle_metrics),
                toggle_off: Look::new(states(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: inactive.text,
                    },
                    hovered,
                    active,
                )),
                toggle_on: Look::new(states(selected, selected, selected)),
                press_shift: 0.0,
                transition,
            },
            checkbox: CheckboxStyle {
                size: s.icon_width,
                radius: 2,
                glyph: s.icon_width_inner,
                gap: s.icon_spacing,
                off: choice_off,
                on: choice_on,
                transition,
            },
            radio: RadioStyle {
                size: s.icon_width,
                dot: s.icon_width_inner,
                gap: s.icon_spacing,
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
                row_gap: 2.0,
                selected: Look::new(States::still(Colors {
                    fill: w.active.weak_bg_fill,
                    border: clear,
                    text: v.strong_text_color(),
                })),
                unselected: Look::new(States::new(
                    Colors {
                        fill: clear,
                        border: clear,
                        text: v.weak_text_color(),
                    },
                    hovered,
                )),
                transition,
            },
            selection: Selection {
                fill: v.selection.bg_fill,
                text: Some(v.selection.stroke.color),
            },
            link: Link {
                rest: v.hyperlink_color,
                hover: v.hyperlink_color,
            },
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::from_egui(&egui::Style::default())
    }
}
