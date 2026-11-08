//! The application's palette and text roles, defined once as tokens indexed
//! into resolved values (the pattern of the design brief's Appendix A). Swap
//! the tables to restyle the whole example.

use core::ops::Index;

use bullseye::{TextSpec, TextStyle};
use egui::{Color32, FontFamily};

/// A colour by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    /// The centre pane.
    Ground,
    /// The side panes.
    Panel,
    /// Bubbles, the composer and chips.
    Surface,
    /// A row under the pointer.
    Hover,
    /// The selected row.
    Selected,
    /// Borders and dividers.
    Line,
    /// Primary text.
    Ink,
    /// Secondary text.
    InkMid,
    /// Placeholders and hints.
    InkDim,
    /// The one accent: splitter, meter, send button, busy state.
    Accent,
    /// The accent under the pointer.
    AccentHover,
    /// Text on the accent.
    OnAccent,
}

impl Tok {
    const ALL: [Self; 12] = [
        Self::Ground,
        Self::Panel,
        Self::Surface,
        Self::Hover,
        Self::Selected,
        Self::Line,
        Self::Ink,
        Self::InkMid,
        Self::InkDim,
        Self::Accent,
        Self::AccentHover,
        Self::OnAccent,
    ];

    fn dark(self) -> Color32 {
        match self {
            Self::Ground => Color32::from_rgb(0x1f, 0x1f, 0x1f),
            Self::Panel => Color32::from_rgb(0x25, 0x25, 0x25),
            Self::Surface => Color32::from_rgb(0x2b, 0x2b, 0x2b),
            Self::Hover => Color32::from_rgb(0x2e, 0x2e, 0x2e),
            Self::Selected => Color32::from_rgb(0x33, 0x3a, 0x44),
            Self::Line => Color32::from_rgb(0x38, 0x38, 0x38),
            Self::Ink => Color32::from_rgb(0xec, 0xec, 0xec),
            Self::InkMid => Color32::from_rgb(0xa8, 0xa8, 0xa8),
            Self::InkDim => Color32::from_rgb(0x7c, 0x7c, 0x7c),
            Self::Accent => Color32::from_rgb(0x6f, 0x8f, 0xb3),
            Self::AccentHover => Color32::from_rgb(0x84, 0xa2, 0xc4),
            Self::OnAccent => Color32::from_rgb(0x14, 0x18, 0x1d),
        }
    }
}

/// A text style by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Body,
    Small,
    Meta,
    Title,
    Message,
    Code,
}

impl Role {
    const ALL: [Self; 6] = [
        Self::Body,
        Self::Small,
        Self::Meta,
        Self::Title,
        Self::Message,
        Self::Code,
    ];

    fn spec(self) -> TextSpec {
        let (size, line_height, family) = match self {
            Self::Body => (14.0, 20.0, FontFamily::Proportional),
            Self::Small => (13.0, 18.0, FontFamily::Proportional),
            Self::Meta => (11.0, 16.0, FontFamily::Proportional),
            Self::Title => (15.0, 20.0, FontFamily::Proportional),
            Self::Message => (15.0, 24.0, FontFamily::Proportional),
            Self::Code => (13.0, 18.0, FontFamily::Monospace),
        };
        TextSpec {
            size,
            line_height,
            family,
        }
    }
}

pub struct Theme {
    colors: [Color32; Tok::ALL.len()],
    roles: [TextSpec; Role::ALL.len()],
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            colors: Tok::ALL.map(Tok::dark),
            roles: Role::ALL.map(Role::spec),
        }
    }

    /// The text every layout starts from.
    pub fn base_text(&self) -> TextStyle {
        TextStyle::new(self[Role::Body].clone(), self[Tok::Ink])
    }
}

impl Index<Tok> for Theme {
    type Output = Color32;

    fn index(&self, token: Tok) -> &Color32 {
        &self.colors[token as usize]
    }
}

impl Index<Role> for Theme {
    type Output = TextSpec;

    fn index(&self, role: Role) -> &TextSpec {
        &self.roles[role as usize]
    }
}
