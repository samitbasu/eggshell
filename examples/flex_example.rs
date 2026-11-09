//! Port of egui_flex's "flex_example": a wrapping row of items that grow to
//! fill each line, mixing text boxes, stock egui widgets and nested flex
//! containers.
//!
//! After the example in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT) by Lucas Meurer, rebuilt with bullseye rather than copied. The labels
//! are text boxes that wrap at their item's width; the buttons, checkbox,
//! slider and text field are stock egui widgets in leaves that take whatever
//! width their flex item grows to. The "toggle my size" button changes its
//! label and so its estimate, and the row re-wraps around it.

#[path = "common/mod.rs"]
mod common;

use std::convert::identity;

use bullseye::owned::{Button, Checkbox, Slider};
use bullseye::prim::text_field::{self, Field};
use bullseye::style::Mark;
use bullseye::{Builder, Tw, tw};
use egui::{Role, WidgetInfo};

type B<'b, 'f> = Builder<'b, 'f, State>;

const LABELS: [&str; 12] = [
    "I",
    "can have veeeeeeeeeeeeery long",
    "and",
    "very",
    "short",
    "and",
    "multi\nline",
    "or\neven\nmore\nlines",
    "and",
    "even",
    "some middle length",
    "items",
];

/// A class applied to an item.
type Class = fn(Tw) -> Tw;

const ALIGNS: [(&str, Class); 4] = [
    ("Start", Tw::self_start),
    ("End", Tw::self_end),
    ("Center", Tw::self_center),
    ("Stretch", Tw::self_stretch),
];

/// The identified nodes the tests name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Toggle,
    Text,
    Slider,
    Checkbox,
    Button(&'static str),
}

pub struct State {
    pub text: String,
    pub toggle: bool,
    pub level: f64,
    pub checked: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            text: "Hello, world!".into(),
            toggle: false,
            level: 0.0,
            checked: false,
        }
    }
}

/// A framed box that grows, with its content centred in it.
///
/// ```html
/// <div class="flex items-center justify-center grow p-1.5 border border-border rounded-md">…</div>
/// ```
fn framed<A>(b: &Builder<'_, '_, A>, base: Tw) -> Tw {
    base.flex()
        .items_center()
        .justify_center()
        .grow()
        .p(1.5)
        .border(1.0)
        .border_color(common::tokens(b.ctx()).border)
        .rounded_md()
}

fn text<A>(b: &mut Builder<'_, '_, A>, class: Tw, content: &str) {
    b.div(framed(b, class), |b| b.text(tw(), content));
}

/// A button that takes the classes `class` gives it.
fn button(b: &mut B<'_, '_>, key: Part, class: Class, label: &str) {
    Button::new(label)
        .classes(class)
        .show(b, key, |_: &mut State| {});
}

/// ```html
/// <div class="flex flex-col grow p-1.5 border … rounded-md">
///   <button class="grow">btn</button> <button>Very long button</button>
///   <div class="flex grow"><button class="grow">btn</button> <button>Very long button</button></div>
/// </div>
/// ```
fn stacked(b: &mut B<'_, '_>) {
    b.div(
        framed(b, tw())
            .flex_col()
            .items_stretch()
            .justify_start()
            .gap(1.0),
        |b| {
            button(b, Part::Button("stack btn"), Tw::grow, "btn");
            button(b, Part::Button("stack long"), identity, "Very long button");
            b.div(tw().flex().grow().items_stretch().gap(1.0), |b| {
                button(b, Part::Button("row btn"), Tw::grow, "btn");
                button(b, Part::Button("row long"), identity, "Very long button");
            });
        },
    );
}

/// ```html
/// <div class="flex flex-wrap items-stretch w-full gap-2">
///   <div class="grow …">Hello</div>
///   <!-- twelve wrapping labels, the toggling button, a text field with basis-[200px],
///        "I have flex basis 80", four align-self items, a slider, the stacked buttons,
///        more buttons and a checkbox -->
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    b.div(tw().p(2.0).w_full(), |b| {
        b.div(
            tw().flex().flex_wrap().items_stretch().gap(2.0).w_full(),
            |b| items(b, state),
        );
    });
}

fn items(b: &mut B<'_, '_>, state: &State) {
    text(b, tw(), "Hello");
    for content in LABELS {
        text(b, tw(), content);
    }

    let label = if state.toggle {
        "short"
    } else {
        "You can toggle my size"
    };
    Button::new(label)
        .classes(Tw::grow)
        .show(b, Part::Toggle, |state: &mut State| {
            state.toggle = !state.toggle;
        });

    let field = Field::singleline(&state.text);
    let measured = field.clone();
    b.div(framed(b, tw()).basis_px(200.0), |b| {
        b.leaf(
            Part::Text,
            tw().w_full(),
            move |m| text_field::measure(m, &measured),
            move |state, ui| {
                let response = text_field::show(ui, &mut state.text, &field);
                response
                    .widget_info(|| WidgetInfo::labeled(Role::TextInput, ui.is_enabled(), "Text"));
            },
        );
    });

    text(b, tw().basis_px(80.0), "I have flex basis 80");
    for (name, align) in ALIGNS {
        text(b, align(tw()), &format!("I have align-self: {name}"));
    }

    b.div(
        tw().flex().grow().basis_px(150.0).items_center().gap(2.0),
        |b| {
            Slider::new(state.level, 0.0..=1000.0).named("Level").show(
                b,
                Part::Slider,
                |state: &mut State, level| {
                    state.level = level;
                },
            );
            b.text(tw().w_px(40.0), format!("{:.0}", state.level));
        },
    );

    stacked(b);

    button(b, Part::Button("long"), Tw::grow, "Very long button");
    button(b, Part::Button("short"), Tw::grow, "Button");
    button(b, Part::Button("odd"), Tw::grow, "Button wefoijfgiweopjg");
    button(b, Part::Button("short again"), Tw::grow, "Button");
    button(b, Part::Button("plain"), identity, "Non-grow Button");
    Checkbox::new("Checkbox", Mark::from(state.checked)).show(
        b,
        Part::Checkbox,
        |state: &mut State| state.checked = !state.checked,
    );
}

fn main() -> eframe::Result {
    common::run_app("flex_example", State::default(), layout)
}
