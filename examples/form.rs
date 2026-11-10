//! A settings-style form: text fields, switches, selects and a segmented
//! control, then a toggle-button row and a swatch grid, the patterns that
//! are built from sensing nodes rather than widgets.
//!
//! The controls are the owned ones of `bullseye::owned` (select, switch,
//! button), and the text fields are `text_field` leaves. A control marked
//! disabled is drawn in the style's disabled colours: the beta switch, the
//! region select and Save until the form can be saved.
//! Tab visits the controls top to bottom, skipping what is inert; Enter or
//! Space presses a focused toggle or swatch.

#[path = "common/mod.rs"]
mod common;

use bullseye::owned::Enablement;
use bullseye::prim::text_field::Field;
use bullseye::{Builder, NodeUi as _, Tw, tw};
use bullseye_shadcn::variant;
use egui::{Color32, Role, WidgetInfo};

type B<'b, 'f> = Builder<'b, 'f, State>;

const FIELD_WIDTH: f32 = 240.0;

/// A text style a toggle button turns on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    Bold,
    Italic,
    Underline,
}

impl Mark {
    pub const ALL: [Self; 3] = [Self::Bold, Self::Italic, Self::Underline];

    fn letter(self) -> &'static str {
        match self {
            Self::Bold => "B",
            Self::Italic => "I",
            Self::Underline => "U",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Underline => "Underline",
        }
    }

    /// The letter as the style it turns on looks.
    fn styled(self) -> Tw {
        match self {
            Self::Bold => tw(),
            Self::Italic => tw().italic(),
            Self::Underline => tw().underline(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// A colour the accent can be set to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Swatch {
    pub name: &'static str,
    pub color: Color32,
}

impl Swatch {
    const fn new(name: &'static str, r: u8, g: u8, b: u8) -> Self {
        Self {
            name,
            color: Color32::from_rgb(r, g, b),
        }
    }
}

pub const SWATCHES: [Swatch; 8] = [
    Swatch::new("Slate", 0x5f, 0x6b, 0x7a),
    Swatch::new("Stone", 0x8a, 0x84, 0x7c),
    Swatch::new("Sand", 0xc9, 0xb8, 0x96),
    Swatch::new("Sky", 0x7f, 0xa8, 0xd0),
    Swatch::new("Sage", 0x8f, 0xae, 0x8b),
    Swatch::new("Rose", 0xd0, 0x9a, 0xa0),
    Swatch::new("Clay", 0xbf, 0x80, 0x66),
    Swatch::new("Plum", 0x8e, 0x70, 0x9a),
];

pub const LANGUAGES: [&str; 4] = ["English", "Deutsch", "Español", "Português"];
pub const REGIONS: [&str; 3] = ["Europe", "North America", "Asia Pacific"];

/// The identified nodes of the form, in Tab order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Name,
    Email,
    Bio,
    EmailAlerts,
    PushAlerts,
    Beta,
    Language,
    Region,
    Theme,
    Mark(Mark),
    Accent(&'static str),
    Save,
}

pub struct State {
    pub name: String,
    pub email: String,
    pub bio: String,
    pub email_alerts: bool,
    pub push_alerts: bool,
    pub beta: bool,
    pub language: usize,
    pub region: usize,
    pub theme: Theme,
    pub marks: Vec<Mark>,
    pub accent: Swatch,
    pub saves: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: String::new(),
            email: String::new(),
            bio: String::new(),
            email_alerts: true,
            push_alerts: false,
            beta: false,
            language: 0,
            region: 0,
            theme: Theme::System,
            marks: Vec::new(),
            accent: SWATCHES[3],
            saves: 0,
        }
    }
}

impl State {
    /// A name and an address that could be one.
    pub fn can_save(&self) -> bool {
        !self.name.trim().is_empty() && self.email.contains('@')
    }

    fn toggle(&mut self, mark: Mark) {
        match self.marks.iter().position(|&on| on == mark) {
            Some(at) => {
                self.marks.remove(at);
            }
            None => self.marks.push(mark),
        }
    }
}

/// A form row: the label on the left, the control beside it.
///
/// ```html
/// <div class="flex items-start gap-4">
///   <label class="w-28 shrink-0 pt-1 text-muted-foreground">Name</label>
///   <div class="grow min-w-0">…</div>
/// </div>
/// ```
fn row<'f>(b: &mut B<'_, 'f>, label: &str, control: impl FnOnce(&mut B<'_, 'f>)) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().items_start().gap(4.0), |b| {
        b.text(tw().w_px(112.0).shrink_0().pt(1.0).text(muted), label);
        b.div(tw().grow().min_w_0(), control);
    });
}

/// A titled group of rows.
///
/// ```html
/// <section class="flex flex-col gap-3">
///   <h2 class="pb-1 border-b border-border text-[18px]">Profile</h2>…
/// </section>
/// ```
fn section<'f>(b: &mut B<'_, 'f>, title: &str, rows: impl FnOnce(&mut B<'_, 'f>)) {
    let border = common::tokens(b.ctx()).border;
    b.div(tw().flex().flex_col().gap(3.0), |b| {
        b.div(tw().pb(1.0).border_b(1.0).border_color(border), |b| {
            b.text(tw().role(common::sized(18.0)), title)
        });
        rows(b);
    });
}

/// A select over `options`, `FIELD_WIDTH` wide.
fn choose(
    b: &mut B<'_, '_>,
    key: Part,
    enablement: Enablement,
    name: &'static str,
    options: &[&'static str],
    current: usize,
    lens: fn(&mut State) -> &mut usize,
) {
    b.select(key, current, options.iter().copied().enumerate())
        .named(name)
        .enablement(enablement)
        .classes(tw().w_px(FIELD_WIDTH))
        .on_change(move |state: &mut State, chosen| *lens(state) = chosen);
}

fn switch(
    b: &mut B<'_, '_>,
    key: Part,
    enablement: Enablement,
    label: &str,
    on: bool,
    set: fn(&mut State, bool),
) {
    b.switch(key, label, on)
        .enablement(enablement)
        .on_change(set);
}

/// A toggle button: pressed while its mark is on, independent of the others.
///
/// ```html
/// <button aria-pressed="true" class="flex items-center justify-center size-8 rounded-md border border-input
///   bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50">B</button>
/// ```
fn toggle(b: &mut B<'_, '_>, state: &State, mark: Mark) {
    let on = state.marks.contains(&mark);
    let t = common::tokens(b.ctx());
    let look = tw()
        .flex()
        .items_center()
        .justify_center()
        .size_px(32.0)
        .rounded_px(f32::from(t.radius_md()))
        .border(1.0)
        .border_color(t.input)
        .focus(|p| p.ring(3.0, common::ring(&t)));
    let look = if on {
        look.bg(t.accent).text(t.accent_foreground)
    } else {
        look.hover(|p| p.bg(t.accent))
    };
    b.div_id(
        Part::Mark(mark),
        look,
        move |state: &mut State, ui| {
            let response = ui.node_response();
            if response.clicked() {
                state.toggle(mark);
            }
            response.widget_info(|| {
                WidgetInfo::selected(Role::CheckBox, ui.is_enabled(), on, mark.name())
            });
        },
        |b| b.text(mark.styled(), mark.letter()),
    );
}

/// A swatch: a coloured square that is ringed while it is the accent.
///
/// ```html
/// <div role="radio" aria-checked="true" class="h-8 rounded-md border-2 border-foreground bg-[#7fa8d0]
///   focus-visible:ring-[3px] focus-visible:ring-ring/50"></div>
/// ```
fn swatch(b: &mut B<'_, '_>, state: &State, swatch: Swatch) {
    let chosen = state.accent == swatch;
    let t = common::tokens(b.ctx());
    let edge = if chosen {
        t.foreground
    } else {
        Color32::TRANSPARENT
    };
    b.div_id(
        Part::Accent(swatch.name),
        tw().h_px(32.0)
            .rounded_px(f32::from(t.radius_md()))
            .bg(swatch.color)
            .border(2.0)
            .border_color(edge)
            .focus(|p| p.ring(3.0, common::ring(&t))),
        move |state: &mut State, ui| {
            let response = ui.node_response();
            if response.clicked() {
                state.accent = swatch;
            }
            response.widget_info(|| {
                WidgetInfo::selected(Role::RadioButton, ui.is_enabled(), chosen, swatch.name)
            });
        },
        |_| {},
    );
}

/// ```html
/// <section class="flex flex-col gap-3">
///   <h2>Profile</h2>
///   <div class="flex items-start gap-4"><label>Name</label><input class="w-full" placeholder="Your name"></div>
///   <div class="flex items-start gap-4"><label>Email</label><input class="w-full" placeholder="you@example.com"></div>
///   <div class="flex items-start gap-4"><label>Bio</label><textarea class="w-full" rows="3" placeholder="A few words about you"></textarea></div>
/// </section>
/// ```
fn profile(b: &mut B<'_, '_>, state: &State) {
    section(b, "Profile", |b| {
        row(b, "Name", |b| {
            let one = Field::singleline(&state.name).hint("Your name");
            common::text_box(b, Part::Name, ("Name", Role::TextInput), one, |s| {
                &mut s.name
            });
        });
        row(b, "Email", |b| {
            let one = Field::singleline(&state.email).hint("you@example.com");
            common::text_box(b, Part::Email, ("Email", Role::TextInput), one, |s| {
                &mut s.email
            });
        });
        row(b, "Bio", |b| {
            let many = Field::multiline(&state.bio, 3)
                .max_rows(6)
                .hint("A few words about you");
            common::text_box(b, Part::Bio, ("Bio", Role::MultilineTextInput), many, |s| {
                &mut s.bio
            });
        });
    });
}

/// ```html
/// <section class="flex flex-col gap-3">
///   <h2>Notifications</h2>
///   <div class="flex items-start gap-4"><label>Alerts</label>
///     <div class="flex flex-col gap-2">three switches, the last one disabled</div>
///   </div>
/// </section>
/// ```
fn notifications(b: &mut B<'_, '_>, state: &State) {
    section(b, "Notifications", |b| {
        row(b, "Alerts", |b| {
            b.div(tw().flex().flex_col().gap(2.0), |b| {
                switch(
                    b,
                    Part::EmailAlerts,
                    Enablement::Enabled,
                    "Email alerts",
                    state.email_alerts,
                    |s, on| s.email_alerts = on,
                );
                switch(
                    b,
                    Part::PushAlerts,
                    Enablement::Enabled,
                    "Push notifications",
                    state.push_alerts,
                    |s, on| s.push_alerts = on,
                );
                switch(
                    b,
                    Part::Beta,
                    Enablement::Disabled,
                    "Beta features (unavailable)",
                    state.beta,
                    |s, on| s.beta = on,
                );
            });
        });
    });
}

/// ```html
/// <section class="flex flex-col gap-3">
///   <h2>Preferences</h2>
///   <div class="flex items-start gap-4"><label>Language</label><select class="w-[240px]">…</select></div>
///   <div class="flex items-start gap-4"><label>Region</label><select class="w-[240px] opacity-50" disabled>…</select></div>
///   <div class="flex items-start gap-4"><label>Theme</label><div class="segmented">Light Dark System</div></div>
///   <div class="flex items-start gap-4"><label>Text style</label><div class="flex gap-1">B I U</div></div>
///   <div class="flex items-start gap-4"><label>Accent</label><div class="grid grid-cols-4 gap-2 w-[152px]">…8 swatches…</div></div>
/// </section>
/// ```
fn preferences(b: &mut B<'_, '_>, state: &State) {
    section(b, "Preferences", |b| {
        row(b, "Language", |b| {
            choose(
                b,
                Part::Language,
                Enablement::Enabled,
                "Language",
                &LANGUAGES,
                state.language,
                |s| &mut s.language,
            );
        });
        row(b, "Region", |b| {
            choose(
                b,
                Part::Region,
                Enablement::Disabled,
                "Region",
                &REGIONS,
                state.region,
                |s| &mut s.region,
            );
        });
        row(b, "Theme", |b| {
            common::choice(
                b,
                "theme",
                state.theme,
                &[
                    (Theme::Light, "Light"),
                    (Theme::Dark, "Dark"),
                    (Theme::System, "System"),
                ],
                |s| &mut s.theme,
            );
        });
        row(b, "Text style", |b| {
            b.div(tw().flex().gap(1.0), |b| {
                for mark in Mark::ALL {
                    toggle(b, state, mark);
                }
            });
        });
        row(b, "Accent", |b| {
            b.div(tw().grid().grid_cols(4).gap(2.0).w_px(152.0), |b| {
                for color in SWATCHES {
                    swatch(b, state, color);
                }
            });
        });
    });
}

/// ```html
/// <div class="flex items-center gap-3">
///   <button class="self-start opacity-50" disabled>Save</button>
///   <span class="text-muted-foreground">Enter a name and an email address to save.</span>
/// </div>
/// ```
fn save(b: &mut B<'_, '_>, state: &State) {
    let enablement = if state.can_save() {
        Enablement::Enabled
    } else {
        Enablement::Disabled
    };
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().items_center().gap(3.0), |b| {
        b.button(Part::Save, "Save")
            .variant(variant::PRIMARY)
            .enablement(enablement)
            .on_click(|state: &mut State| state.saves += 1);
        let note = match (state.saves, state.can_save()) {
            (_, false) => "Enter a name and an email address to save.".to_owned(),
            (0, true) => String::new(),
            (1, true) => "Saved once.".to_owned(),
            (times, true) => format!("Saved {times} times."),
        };
        b.text(tw().text(muted), note);
    });
}

/// ```html
/// <main class="flex justify-center size-full p-6 bg-background text-foreground">
///   <form class="flex flex-col gap-5 w-full max-w-[560px]">…</form>
/// </main>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .justify_center()
            .size_full()
            .p(6.0)
            .bg(t.background),
        |b| {
            b.div(
                tw().flex()
                    .flex_col()
                    .gap(5.0)
                    .w_full()
                    .max_w_px(560.0)
                    .text(t.foreground),
                |b| {
                    profile(b, state);
                    notifications(b, state);
                    preferences(b, state);
                    save(b, state);
                },
            );
        },
    );
}

/// Tints the primary button and the focus with the examples' accent.
pub fn prepare(ctx: &egui::Context, state: &State) {
    common::use_accent(ctx, state);
}

fn main() -> eframe::Result {
    common::run_app_styled("form", State::default(), prepare, layout)
}
