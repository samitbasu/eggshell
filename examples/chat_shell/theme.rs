//! The application's palette and text roles, defined once as tokens indexed
//! into resolved values (the pattern of the design brief's Appendix A). The
//! palette is the shadcn preset's: each token names the preset colour it
//! takes, in light and in dark.

use core::ops::Index;

use bullseye::style::{SegmentedClasses, Shadcn};
use bullseye::{TextSpec, TextStyle};
use egui::{Color32, Context, FontFamily};

/// A colour by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    /// The centre pane.
    Ground,
    /// The side panes.
    Panel,
    /// Bubbles, the composer and chips.
    Surface,
    /// A row under the pointer: half the strength of the selected row.
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
    /// The accent as text or a small marker: the accent itself in light, its
    /// lighter ring shade in dark, where the fill shade is too dim to read.
    AccentText,
    /// The accent under the pointer.
    AccentHover,
    /// Text on the accent.
    OnAccent,
    /// Additions.
    Success,
    /// Removals.
    Danger,
}

impl Tok {
    const ALL: [Self; 15] = [
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
        Self::AccentText,
        Self::AccentHover,
        Self::OnAccent,
        Self::Success,
        Self::Danger,
    ];

    /// The preset colour this token stands for.
    fn resolve(self, preset: &Shadcn) -> Color32 {
        let t = &preset.tokens;
        match self {
            Self::Ground => t.background,
            Self::Panel => t.card,
            Self::Surface => t.muted,
            Self::Hover => t.accent_50(),
            Self::Selected => t.accent,
            Self::Line => t.border,
            Self::Ink => t.foreground,
            Self::InkMid | Self::InkDim => t.muted_foreground,
            Self::Accent => t.primary,
            Self::AccentText => match preset.theme {
                egui::Theme::Light => t.primary,
                egui::Theme::Dark => t.ring,
            },
            Self::AccentHover => t.primary_90(),
            Self::OnAccent => t.primary_foreground,
            Self::Success => t.success,
            Self::Danger => t.destructive,
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
    /// The palette of the preset applied to `ctx` for its current theme, or
    /// the preset's own if none was.
    pub fn of(ctx: &Context) -> Self {
        Self::from_preset(&Shadcn::of(ctx))
    }

    /// The palette of the preset for `theme`.
    pub fn new(theme: egui::Theme) -> Self {
        Self::from_preset(&Shadcn::for_theme(theme))
    }

    fn from_preset(preset: &Shadcn) -> Self {
        Self {
            colors: Tok::ALL.map(|token| token.resolve(preset)),
            roles: Role::ALL.map(Role::spec),
        }
    }

    /// The text every layout starts from.
    pub fn base_text(&self) -> TextStyle {
        TextStyle::new(self[Role::Body].clone(), self[Tok::Ink])
    }
}

/// The preset's tab list classes for the context's current theme.
pub fn tab_list(ctx: &Context) -> SegmentedClasses {
    Shadcn::of(ctx).segmented()
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
