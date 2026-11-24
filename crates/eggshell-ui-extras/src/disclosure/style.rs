//! How section headers and disclosure buttons look.

use std::time::Duration;

use eggshell_ui::contract::Focus;
use eggshell_ui::{Angle, Icon, Opacity, Style, TextSpec, Weight, icons};
use egui::Color32;

/// TODO(prose): the look of [`section`](super::SectionExt::section) headers
/// and [`disclosure`](super::DisclosureExt::disclosure) buttons, one
/// family: the button is a section header's chevron on its own. Lengths are
/// in points.
///
/// ```
/// use eggshell_ui_extras::disclosure::DisclosureStyle;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = (*style.extension::<DisclosureStyle>()).clone();
/// look.header.padding_y = 8.0;
/// style.set_extension(look);
/// assert_eq!(style.extension::<DisclosureStyle>().header.padding_y, 8.0);
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DisclosureStyle {
    /// A section header's chevron.
    pub section: Glyph,
    /// A disclosure button's chevron.
    pub tree: Glyph,
    /// The chevrons' colour; `None` is the header's text colour.
    pub tone: Option<Color32>,
    /// How much of that colour the chevrons keep.
    pub tone_opacity: Opacity,
    /// How far a section's chevron sits below the top of its header's
    /// first line.
    pub nudge: f32,
    /// A section's header.
    pub header: HeaderStyle,
    /// Bottom padding of a section's content.
    pub content_padding_bottom: f32,
    /// Side of a disclosure button.
    pub button: f32,
    /// A disclosure button's fill under the pointer, if it has one.
    pub button_hover: Option<Color32>,
    /// The line between an accordion's items, if there is one.
    pub separator: Option<Color32>,
}

/// TODO(prose): a chevron, which turns by `turn` times how far open it is.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Glyph {
    /// The icon while closed.
    pub icon: Icon,
    /// Side of the icon.
    pub size: f32,
    /// How far it turns once open.
    pub turn: Angle,
}

impl Glyph {
    /// TODO(prose): a chevron drawing `icon`, `size` points square, turned
    /// by `turn` once open.
    pub fn new(icon: Icon, size: f32, turn: Angle) -> Self {
        Self { icon, size, turn }
    }
}

/// TODO(prose): a section header.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HeaderStyle {
    /// The focus ring.
    pub ring: Focus,
    /// The border while focused, if any: the box then has a clear border
    /// at rest, one point wide.
    pub focus_border: Option<Color32>,
    /// Corner radius, for the focus ring and a hover tint.
    pub radius: u8,
    /// Top and bottom padding.
    pub padding_y: f32,
    /// Gap between the title and the chevron.
    pub gap: f32,
    /// The title's text role.
    pub text: TextSpec,
    /// What the pointer over the header does besides the look's hover.
    pub hover: Hover,
    /// The opacity of a disabled header.
    pub disabled: Opacity,
    /// How long state changes animate.
    pub transition: Duration,
}

/// TODO(prose): what the pointer over a section header does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    /// `hover:underline`.
    Underline,
    /// A fill behind the header.
    Tint(Color32),
    /// Nothing.
    None,
}

impl eggshell_ui::contract::Extension for DisclosureStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl DisclosureStyle {
    /// TODO(prose): headers in the style's text, medium, with a muted
    /// `chevron-down` that turns over trailing a section's title and a
    /// `chevron-right` that turns down in a disclosure button.
    pub fn from_style(style: &Style) -> Self {
        let glyph = style.select.chevron;
        Self {
            section: Glyph::new(Icon::new(icons::CHEVRON_DOWN), glyph, Angle::degrees(180.0)),
            tree: Glyph::new(Icon::new(icons::CHEVRON_RIGHT), glyph, Angle::degrees(90.0)),
            tone: None,
            tone_opacity: style.select.chevron_opacity,
            nudge: 2.0,
            header: HeaderStyle {
                ring: style.tokens.focus(),
                focus_border: None,
                radius: 6,
                padding_y: 16.0,
                gap: 16.0,
                text: TextSpec {
                    weight: Weight::Medium,
                    ..style.text.clone()
                },
                hover: Hover::Underline,
                disabled: Opacity::new(0.5),
                transition: style.toggle.transition,
            },
            content_padding_bottom: 16.0,
            button: glyph,
            button_hover: None,
            separator: Some(style.table.rule),
        }
    }
}
