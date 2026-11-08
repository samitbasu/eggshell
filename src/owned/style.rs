//! A draft of [`Style`]: the parts the owned button and checkbox read.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Context, Id, Shadow, Theme};

use super::Enablement;
use crate::style::{Mark, Shadcn};
use crate::{TextSpec, Tw, tw};

/// The names of shadcn's button variants.
pub mod variant {
    /// `default`.
    pub const PRIMARY: &str = "primary";
    /// `secondary`.
    pub const SECONDARY: &str = "secondary";
    /// `destructive`.
    pub const DESTRUCTIVE: &str = "destructive";
    /// `outline`.
    pub const OUTLINE: &str = "outline";
    /// `ghost`.
    pub const GHOST: &str = "ghost";
}

/// What a control draws in one state.
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
    fn faded(self) -> Self {
        let fade = |color: Color32| color.gamma_multiply(0.5);
        Self {
            fill: fade(self.fill),
            border: fade(self.border),
            text: fade(self.text),
        }
    }
}

/// A control's colours in each state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct States {
    /// At rest.
    pub rest: Colors,
    /// Under the pointer.
    pub hover: Colors,
    /// Held down.
    pub pressed: Colors,
    /// Not taking input.
    pub disabled: Colors,
}

impl States {
    fn new(rest: Colors, hover: Colors) -> Self {
        Self {
            rest,
            hover,
            pressed: hover,
            disabled: rest.faded(),
        }
    }
}

/// The keyboard focus indicator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    /// Ring width in points.
    pub ring_width: f32,
    /// Ring colour.
    pub ring: Color32,
    /// Border colour while focused.
    pub border: Color32,
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
    /// Focus indicator.
    pub focus: Focus,
}

/// Sizes of a control, in points.
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

/// Buttons: named variants, and the toggle form.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonStyle {
    /// Label text.
    pub text: TextSpec,
    /// Sizes of a button.
    pub metrics: Metrics,
    /// Sizes of a toggle.
    pub toggle_metrics: Metrics,
    /// The variants the style defines, by name.
    pub variants: BTreeMap<&'static str, Look>,
    /// A toggle that is off.
    pub toggle_off: Look,
    /// A toggle that is on.
    pub toggle_on: Look,
    /// How long state changes animate.
    pub transition: Duration,
}

impl ButtonStyle {
    /// The look of the variant `name`.
    ///
    /// # Panics
    /// If the style does not define it.
    pub fn variant(&self, name: &str) -> &Look {
        self.variants
            .get(name)
            .unwrap_or_else(|| panic!("the style has no button variant {name:?}"))
    }

    /// The look of a toggle in `mark`.
    pub fn toggle(&self, mark: Mark) -> &Look {
        match mark {
            Mark::Off => &self.toggle_off,
            Mark::On => &self.toggle_on,
        }
    }
}

/// Checkboxes.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckboxStyle {
    /// Label text.
    pub text: TextSpec,
    /// Side of the box.
    pub size: f32,
    /// Corner radius of the box.
    pub radius: u8,
    /// Side of the check glyph.
    pub glyph: f32,
    /// Gap between the box and its label.
    pub gap: f32,
    /// Unticked.
    pub unchecked: Look,
    /// Ticked.
    pub checked: Look,
    /// The label's colours.
    pub label: States,
    /// How long state changes animate.
    pub transition: Duration,
}

impl CheckboxStyle {
    /// The look of the box in `mark`.
    pub fn look(&self, mark: Mark) -> &Look {
        match mark {
            Mark::Off => &self.unchecked,
            Mark::On => &self.checked,
        }
    }
}

/// The draft of `bullseye::Style`: the button and checkbox parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// Buttons and toggles.
    pub button: ButtonStyle,
    /// Checkboxes.
    pub checkbox: CheckboxStyle,
}

impl Look {
    /// The classes that draw a box of `radius` in this look. A disabled box is
    /// inert and has no variants: its colours are the style's, not an opacity.
    pub(crate) fn classes(&self, radius: u8, enablement: Enablement, transition: Duration) -> Tw {
        let colors = |classes: Tw, colors: Colors| {
            classes
                .bg(colors.fill)
                .border_color(colors.border)
                .text(colors.text)
        };
        let base = tw()
            .border(self.border_width)
            .rounded_px(f32::from(radius))
            .sense_click();
        match enablement {
            Enablement::Enabled => {
                let States {
                    rest,
                    hover,
                    pressed,
                    ..
                } = self.states;
                let Focus {
                    ring_width,
                    ring,
                    border,
                } = self.focus;
                let classes = colors(base, rest)
                    .hover(|p| p.bg(hover.fill).border_color(hover.border).text(hover.text))
                    .active(|p| {
                        p.bg(pressed.fill)
                            .border_color(pressed.border)
                            .text(pressed.text)
                    })
                    .focus(|p| p.border_color(border).ring(ring_width, ring))
                    .transition(transition);
                self.shadow
                    .map_or(classes.clone(), |shadow| classes.shadow(shadow))
            }
            Enablement::Disabled => {
                let classes = colors(base, self.states.disabled).inert();
                self.shadow.map_or(classes.clone(), |shadow| {
                    classes.shadow(Shadow {
                        color: shadow.color.gamma_multiply(0.5),
                        ..shadow
                    })
                })
            }
        }
    }
}

fn style_id() -> Id {
    crate::key::root_id().with("owned-style")
}

impl Style {
    /// The style installed in `ctx`, or shadcn's (vega) for the context's
    /// current theme.
    pub fn of(ctx: &Context) -> Arc<Self> {
        ctx.data(|data| data.get_temp::<Arc<Self>>(style_id()))
            .unwrap_or_else(|| Arc::new(Self::shadcn(&Shadcn::of(ctx))))
    }

    /// Makes `self` the style of every owned control in `ctx`.
    pub fn install(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(style_id(), Arc::new(self)));
    }

    /// shadcn's vega style, from the preset's tokens.
    pub fn shadcn(preset: &Shadcn) -> Self {
        let t = &preset.tokens;
        let dark = preset.theme == Theme::Dark;
        let clear = Color32::TRANSPARENT;
        let ring = t.ring.gamma_multiply(0.5);
        let focus = Focus {
            ring_width: 3.0,
            ring,
            border: t.ring,
        };
        let colors = |fill, text| Colors {
            fill,
            border: clear,
            text,
        };
        let look = |states, border_width, shadow| Look {
            states,
            border_width,
            shadow,
            focus,
        };
        let filled = |rest: Colors, hover: Colors| look(States::new(rest, hover), 0.0, None);

        let outline_rest = Colors {
            fill: if dark { t.input_30() } else { t.background },
            border: t.input,
            text: t.foreground,
        };
        let outline_hover = Colors {
            fill: if dark { t.input_50() } else { t.accent },
            text: if dark {
                t.foreground
            } else {
                t.accent_foreground
            },
            ..outline_rest
        };
        let destructive_ring = t.destructive.gamma_multiply(if dark { 0.4 } else { 0.2 });
        let variants = BTreeMap::from([
            (
                variant::PRIMARY,
                filled(
                    colors(t.primary, t.primary_foreground),
                    colors(t.primary_90(), t.primary_foreground),
                ),
            ),
            (
                variant::SECONDARY,
                filled(
                    colors(t.secondary, t.secondary_foreground),
                    colors(t.secondary_80(), t.secondary_foreground),
                ),
            ),
            (
                variant::DESTRUCTIVE,
                Look {
                    focus: Focus {
                        ring: destructive_ring,
                        ..focus
                    },
                    ..filled(
                        colors(
                            if dark {
                                t.destructive_60()
                            } else {
                                t.destructive
                            },
                            t.destructive_foreground,
                        ),
                        colors(t.destructive_90(), t.destructive_foreground),
                    )
                },
            ),
            (
                variant::OUTLINE,
                look(
                    States::new(outline_rest, outline_hover),
                    1.0,
                    Some(t.shadow_xs),
                ),
            ),
            (
                variant::GHOST,
                filled(
                    colors(clear, t.foreground),
                    colors(
                        if dark { t.accent_50() } else { t.accent },
                        t.accent_foreground,
                    ),
                ),
            ),
        ]);

        let toggle_rest = colors(clear, t.foreground);
        let toggle_on = colors(t.accent, t.accent_foreground);
        let transition = Duration::from_millis(150);
        let box_rest = Colors {
            fill: if dark { t.input_30() } else { clear },
            border: t.input,
            text: t.primary_foreground,
        };
        let box_on = Colors {
            fill: t.primary,
            border: t.primary,
            text: t.primary_foreground,
        };
        let label = colors(clear, t.foreground);
        let text = t.text_role();
        Self {
            button: ButtonStyle {
                text: text.clone(),
                metrics: Metrics {
                    height: t.control_height,
                    min_width: 0.0,
                    padding_x: t.button_padding.x,
                    gap: 8.0,
                    radius: t.radius_md(),
                },
                toggle_metrics: Metrics {
                    height: t.control_height,
                    min_width: t.control_height,
                    padding_x: 8.0,
                    gap: 8.0,
                    radius: t.radius_md(),
                },
                variants,
                toggle_off: filled(toggle_rest, colors(t.muted, t.muted_foreground)),
                toggle_on: filled(toggle_on, toggle_on),
                transition,
            },
            checkbox: CheckboxStyle {
                text,
                size: 16.0,
                radius: 4,
                glyph: 14.0,
                gap: 8.0,
                unchecked: look(States::new(box_rest, box_rest), 1.0, Some(t.shadow_xs)),
                checked: look(States::new(box_on, box_on), 1.0, Some(t.shadow_xs)),
                label: States::new(label, label),
                transition,
            },
        }
    }
}
