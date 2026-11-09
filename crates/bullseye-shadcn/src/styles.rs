//! shadcn's styles: parameter sets that differ in a handful of values.

use bullseye::Opacity;
use egui::vec2;

use crate::Tokens;

/// One of shadcn's styles, from `apps/v4/registry/styles/style-*.css`. A
/// style is a parameter set (radii, control height, text size, ring width and
/// a few choices of surface); it is independent of the [`ThemeColor`](crate::ThemeColor)
/// and of light or dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShadcnStyle {
    /// `style-vega`, the classic look: `rounded-md`, `h-9`, `text-sm`, a 3 pt
    /// focus ring and `shadow-xs` on inputs.
    #[default]
    Vega,
    /// `style-lyra`: square corners, `h-8`, `text-xs`, a 1 pt focus ring, flat
    /// controls and a tinted destructive button.
    Lyra,
}

impl ShadcnStyle {
    /// Every style, in the order of the type.
    pub const ALL: [Self; 2] = [Self::Vega, Self::Lyra];

    /// The style's name, as shadcn spells it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Vega => "Vega",
            Self::Lyra => "Lyra",
        }
    }

    /// Sets the style-dependent fields of `tokens`; the colours are untouched.
    pub(crate) fn shape(self, tokens: &mut Tokens) {
        let t = tokens;
        t.style = self;
        match self {
            Self::Vega => {
                t.radius = 10;
                t.text_size = 14.0;
                t.text_line_height = 20.0;
                t.control_height = 36.0;
                t.button_padding = vec2(16.0, 8.0);
                t.input_padding = vec2(12.0, 4.0);
                t.control_gap = 8.0;
                t.toggle_padding_x = 8.0;
                t.ring_width = 3.0;
                t.checkbox_radius = 4;
                t.slider_track = 6.0;
                t.slider_thumb = 16.0;
                t.slider_radius = u8::MAX;
                t.list_padding = 3.0;
                t.cell_padding = vec2(8.0, 4.0);
                t.popup_padding = 4.0;
                t.item_padding = vec2(8.0, 6.0);
                t.primary_hover = Opacity::new(0.9);
                t.secondary_hover = SecondaryHover::Fade(Opacity::new(0.8));
                t.hover_surface = HoverSurface::Accent;
                t.destructive_look = DestructiveLook::Solid;
                t.elevation = Elevation::Raised;
            }
            Self::Lyra => {
                t.radius = 0;
                t.text_size = 12.0;
                t.text_line_height = 16.0;
                t.control_height = 32.0;
                t.button_padding = vec2(10.0, 6.0);
                t.input_padding = vec2(10.0, 4.0);
                t.control_gap = 6.0;
                t.toggle_padding_x = 10.0;
                t.ring_width = 1.0;
                t.checkbox_radius = 0;
                t.slider_track = 4.0;
                t.slider_thumb = 12.0;
                t.slider_radius = 0;
                t.list_padding = 3.0;
                t.cell_padding = vec2(6.0, 2.0);
                t.popup_padding = 4.0;
                t.item_padding = vec2(8.0, 8.0);
                t.primary_hover = Opacity::new(0.8);
                t.secondary_hover = SecondaryHover::Mix(Opacity::new(0.05));
                t.hover_surface = HoverSurface::Muted;
                t.destructive_look = DestructiveLook::Tinted;
                t.elevation = Elevation::Flat;
            }
        }
    }
}

/// Whether controls carry `shadow-xs` and `shadow-sm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elevation {
    /// Outline buttons, boxes and the slider thumb are shadowed.
    Raised,
    /// They are not.
    Flat,
}

impl Elevation {
    pub(crate) fn shadow(self, shadow: egui::Shadow) -> Option<egui::Shadow> {
        match self {
            Self::Raised => Some(shadow),
            Self::Flat => None,
        }
    }
}

/// The surface a ghost or outline button takes under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverSurface {
    /// `hover:bg-accent hover:text-accent-foreground`.
    Accent,
    /// `hover:bg-muted hover:text-foreground`.
    Muted,
}

/// How a secondary button looks under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SecondaryHover {
    /// `hover:bg-secondary/80`: the fill at this opacity.
    Fade(Opacity),
    /// `hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]`:
    /// this much of `foreground` mixed into the fill.
    Mix(Opacity),
}

/// How the destructive button is filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestructiveLook {
    /// `bg-destructive text-white`.
    Solid,
    /// `bg-destructive/10 text-destructive`: a tint of the colour, in the
    /// colour.
    Tinted,
}
