//! Port of hello_egui's signup form: a centred form of text fields, a drag
//! value and checkboxes, with a validation error under each field that has
//! one.
//!
//! After the fancy-example in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT) by Lucas Meurer, rebuilt with bullseye rather than copied: the
//! `egui_form` and `validator` crates become a few functions on the state. An
//! error shows for a field the user has changed, and for every field once
//! Submit has been pressed with something invalid. The error appears as text
//! under its field, on a line reserved for it, so the form does not move. Two-column rows are
//! wrapping flex rows, which stack when the window is narrow.

#[path = "common/mod.rs"]
mod common;

use bullseye::prim::text_field::Field;
use bullseye::widget::{Button, Checkbox, DragValue};
use bullseye::{Builder, Tw, tw};
use egui::{Role, WidgetInfo};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// The fields of the form, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    FirstName,
    LastName,
    Email,
    Password,
    Repeat,
    Age,
    Terms,
    Newsletter,
    Submit,
    Reset,
}

#[derive(Clone, Debug, Default)]
pub struct Fields {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub repeat: String,
    pub age: f64,
    pub terms: bool,
    pub newsletter: bool,
}

/// How far the user has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Editing,
    /// Submit was pressed with something invalid: every error shows.
    Rejected,
    Submitted,
}

#[derive(Debug)]
pub struct State {
    pub fields: Fields,
    pub progress: Progress,
}

impl Default for State {
    fn default() -> Self {
        Self {
            fields: Fields::default(),
            progress: Progress::Editing,
        }
    }
}

fn name_error(name: &str) -> Option<&'static str> {
    (!(2..=50).contains(&name.chars().count()))
        .then_some("Must be between 2 and 50 characters long")
}

fn strong(password: &str) -> bool {
    password.chars().any(char::is_lowercase)
        && password.chars().any(char::is_uppercase)
        && password.chars().any(|c| c.is_ascii_digit())
        && password.chars().any(|c| !c.is_alphanumeric())
}

impl Fields {
    fn error(&self, part: Part) -> Option<&'static str> {
        match part {
            Part::FirstName => name_error(&self.first_name),
            Part::LastName => name_error(&self.last_name),
            Part::Email => {
                let valid = self
                    .email
                    .split_once('@')
                    .is_some_and(|(user, host)| !user.is_empty() && host.contains('.'));
                (!valid).then_some("Invalid email")
            }
            Part::Password if self.password.chars().count() < 8 => {
                Some("Must be at least 8 characters long")
            }
            Part::Password => (!strong(&self.password)).then_some(
                "Password must contain at least one uppercase letter, one lowercase letter, \
                 one digit, and one special character",
            ),
            Part::Repeat if self.repeat.chars().count() < 8 => {
                Some("Must be at least 8 characters long")
            }
            Part::Repeat => (self.repeat != self.password).then_some("Passwords do not match"),
            Part::Age => {
                (!(18.0..=120.0).contains(&self.age)).then_some("You must be at least 18 years old")
            }
            Part::Terms => (!self.terms).then_some("You must agree to the terms"),
            Part::Newsletter | Part::Submit | Part::Reset => None,
        }
    }

    /// Whether the user has changed the field from its starting value.
    fn touched(&self, part: Part) -> bool {
        let start = Self::default();
        match part {
            Part::FirstName => self.first_name != start.first_name,
            Part::LastName => self.last_name != start.last_name,
            Part::Email => self.email != start.email,
            Part::Password => self.password != start.password,
            Part::Repeat => self.repeat != start.repeat,
            Part::Age => self.age != start.age,
            Part::Terms => self.terms != start.terms,
            Part::Newsletter | Part::Submit | Part::Reset => false,
        }
    }

    pub const REQUIRED: [Part; 7] = [
        Part::FirstName,
        Part::LastName,
        Part::Email,
        Part::Password,
        Part::Repeat,
        Part::Age,
        Part::Terms,
    ];

    pub fn is_valid(&self) -> bool {
        Self::REQUIRED
            .iter()
            .all(|&part| self.error(part).is_none())
    }
}

impl State {
    /// The error to show under `part`.
    pub fn visible_error(&self, part: Part) -> Option<&'static str> {
        let show = self.progress == Progress::Rejected || self.fields.touched(part);
        self.fields.error(part).filter(|_| show)
    }

    fn submit(&mut self) {
        self.progress = if self.fields.is_valid() {
            Progress::Submitted
        } else {
            Progress::Rejected
        };
    }
}

/// A field with its label above and its error below.
///
/// ```html
/// <div class="flex flex-col gap-1">
///   <label>First Name</label>
///   <input>
///   <span class="text-destructive">Must be between 2 and 50 characters long</span>
/// </div>
/// ```
/// The line under a field that holds its error. It keeps its height while
/// the field is valid, so a field's error appearing does not move the rest
/// of the form.
///
/// ```html
/// <p class="min-h-[16.25px] text-[13px]/[16.25px] text-destructive">Invalid email</p>
/// ```
fn error_line(b: &mut B<'_, '_>, error: Option<&'static str>) {
    let t = common::tokens(b.ctx());
    let role = common::sized(13.0);
    b.div(tw().min_h_px(role.line_height), |b| {
        if let Some(error) = error {
            b.text(tw().text(t.destructive).role(role), error);
        }
    });
}

fn labelled<'f>(
    b: &mut B<'_, 'f>,
    state: &State,
    part: Part,
    label: &str,
    control: impl FnOnce(&mut B<'_, 'f>),
) {
    b.div(
        tw().flex()
            .flex_col()
            .gap(1.0)
            .grow()
            .basis_px(180.0)
            .min_w_0(),
        |b| {
            b.text(tw(), label);
            control(b);
            error_line(b, state.visible_error(part));
        },
    );
}

fn text(
    b: &mut B<'_, '_>,
    state: &State,
    part: Part,
    (label, hint): (&'static str, &'static str),
    lens: fn(&mut State) -> &mut String,
    field: fn(&State) -> Field,
) {
    labelled(b, state, part, label, |b| {
        common::text_box(
            b,
            part,
            (label, Role::TextInput),
            field(state).hint(hint),
            lens,
        );
    });
}

/// ```html
/// <div class="flex flex-wrap gap-3">
///   <div class="flex flex-col gap-1 grow basis-[180px]">…First Name…</div>
///   <div class="flex flex-col gap-1 grow basis-[180px]">…Last Name…</div>
/// </div>
/// ```
fn pair<'f>(b: &mut B<'_, 'f>, columns: impl FnOnce(&mut B<'_, 'f>)) {
    b.div(tw().flex().flex_wrap().gap(3.0), columns);
}

fn check(
    b: &mut B<'_, '_>,
    state: &State,
    part: Part,
    label: &'static str,
    lens: fn(&mut State) -> &mut bool,
) {
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        b.widget(
            part,
            tw().self_start(),
            Checkbox::new(label),
            move |state, ui, checkbox| {
                checkbox.show(ui, lens(state));
            },
        );
        error_line(b, state.visible_error(part));
    });
}

/// ```html
/// <form class="flex flex-col gap-3 w-full max-w-[480px]">
///   <p>This is an example signup form…</p>
///   <div class="flex flex-wrap gap-3">First Name, Last Name</div>
///   <div class="flex flex-col gap-1">Email</div>
///   <div class="flex flex-wrap gap-3">Password, Repeat Password</div>
///   <div class="flex flex-col gap-1">Age <input type="number" class="w-[48px]"></div>
///   <label><input type="checkbox"> I agree to the terms</label>
///   <label><input type="checkbox"> Subscribe to newsletter</label>
///   <button class="w-full h-10">Submit</button>
/// </form>
/// ```
fn form(b: &mut B<'_, '_>, state: &State) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.text(
        tw().text(muted),
        "This is an example signup form. Try signing up with invalid data to see the \
         validation errors. Errors show up after editing a field or after trying to submit.",
    );
    pair(b, |b| {
        text(
            b,
            state,
            Part::FirstName,
            ("First Name", "Ada"),
            |s| &mut s.fields.first_name,
            |s| Field::singleline(&s.fields.first_name),
        );
        text(
            b,
            state,
            Part::LastName,
            ("Last Name", "Lovelace"),
            |s| &mut s.fields.last_name,
            |s| Field::singleline(&s.fields.last_name),
        );
    });
    b.div(tw().flex(), |b| {
        text(
            b,
            state,
            Part::Email,
            ("Email", "ada@example.com"),
            |s| &mut s.fields.email,
            |s| Field::singleline(&s.fields.email),
        );
    });
    pair(b, |b| {
        text(
            b,
            state,
            Part::Password,
            ("Password", ""),
            |s| &mut s.fields.password,
            |s| Field::singleline(&s.fields.password).masked(),
        );
        text(
            b,
            state,
            Part::Repeat,
            ("Repeat Password", ""),
            |s| &mut s.fields.repeat,
            |s| Field::singleline(&s.fields.repeat).masked(),
        );
    });
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        b.text(tw(), "Age");
        b.widget(
            Part::Age,
            tw().self_start(),
            DragValue::new("120"),
            |state, ui, age| {
                let response = age.show(ui, &mut state.fields.age, 0.0..=150.0);
                response
                    .widget_info(|| WidgetInfo::slider(ui.is_enabled(), state.fields.age, "Age"));
            },
        );
        error_line(b, state.visible_error(Part::Age));
    });
    check(b, state, Part::Terms, "I agree to the terms", |s| {
        &mut s.fields.terms
    });
    check(b, state, Part::Newsletter, "Subscribe to newsletter", |s| {
        &mut s.fields.newsletter
    });
    b.widget(
        Part::Submit,
        tw().w_full().h_px(40.0),
        Button::new("Submit"),
        |state, ui, submit| {
            if submit.show(ui).clicked() {
                state.submit();
            }
        },
    );
}

/// What replaces the form once it is submitted.
///
/// ```html
/// <div class="flex flex-col items-start gap-3">
///   <strong>Thank you for signing up!</strong>
///   <p>Here are the details you submitted:</p>
///   <pre class="font-mono p-2 bg-muted rounded">…</pre>
///   <button>Reset</button>
/// </div>
/// ```
fn thanks(b: &mut B<'_, '_>, state: &State) {
    b.text(tw().role(common::sized(20.0)), "Thank you for signing up!");
    b.text(tw(), "Here are the details you submitted:");
    let t = common::tokens(b.ctx());
    b.div(tw().p(2.0).rounded().bg(t.muted), |b| {
        b.text(
            tw().role(common::mono(13.0)),
            format!("{:#?}", state.fields),
        );
    });
    b.widget(
        Part::Reset,
        tw().self_start(),
        Button::new("Reset"),
        |state, ui, reset| {
            if reset.show(ui).clicked() {
                *state = State::default();
            }
        },
    );
}

/// ```html
/// <main class="flex justify-center size-full p-4 bg-background text-foreground">
///   <div class="flex flex-col gap-3 w-full max-w-[480px]">…</div>
/// </main>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .justify_center()
            .size_full()
            .p(4.0)
            .bg(t.background),
        |b| {
            let column: Tw = tw()
                .flex()
                .flex_col()
                .items_stretch()
                .gap(3.0)
                .w_full()
                .max_w_px(480.0)
                .text(t.foreground);
            b.div(column, |b| match state.progress {
                Progress::Submitted => thanks(b, state),
                Progress::Editing | Progress::Rejected => form(b, state),
            });
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("signup_form", State::default(), layout)
}
