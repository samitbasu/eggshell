//! The application's palette and text roles, defined once as tokens indexed
//! into resolved values (the pattern of the design brief's Appendix A). The
//! palette is the shadcn preset's: each token names the preset colour it
//! takes, in light and in dark.

use core::ops::Index;

use bullseye::style::{SegmentedClasses, Shadcn, Tokens};
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

    /// The preset colour this token stands for.
    fn resolve(self, t: &Tokens) -> Color32 {
        match self {
            Self::Ground => t.background,
            Self::Panel => t.card,
            Self::Surface => t.muted,
            Self::Hover => t.accent,
            Self::Selected | Self::Line => t.border,
            Self::Ink => t.foreground,
            Self::InkMid | Self::InkDim => t.muted_foreground,
            Self::Accent => t.primary,
            Self::AccentHover => t.primary.gamma_multiply(0.9),
            Self::OnAccent => t.primary_foreground,
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
        Self::from_tokens(&preset_tokens(ctx))
    }

    /// The palette of the preset for `theme`.
    pub fn new(theme: egui::Theme) -> Self {
        Self::from_tokens(&Tokens::for_theme(theme))
    }

    fn from_tokens(tokens: &Tokens) -> Self {
        Self {
            colors: Tok::ALL.map(|token| token.resolve(tokens)),
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
    Shadcn {
        theme: ctx.theme(),
        tokens: preset_tokens(ctx),
    }
    .segmented()
}

fn preset_tokens(ctx: &Context) -> Tokens {
    bullseye::style::tokens(ctx).unwrap_or_else(|| Tokens::for_theme(ctx.theme()))
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
