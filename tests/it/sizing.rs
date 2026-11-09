//! Drift guards: for the text edit, the size the library computes from
//! content, style and galley equals the size egui allocates when it draws the
//! widget. Each case varies the text, the font size and the style, and a
//! width that makes text wrap.

use bullseye::style::Shadcn;
use bullseye::widget::{Lines, Stock, TextEdit};
use bullseye::{TextSpec, TextStyle, Width};
use egui::{
    Color32, Context, FontFamily, Rect, Response, Style, Theme, Ui, UiBuilder, Vec2, pos2, vec2,
};

fn text_style(size: f32) -> TextStyle {
    TextStyle::new(
        TextSpec {
            size,
            line_height: size * 1.25,
            family: FontFamily::Proportional,
        },
        Color32::WHITE,
    )
}

/// A change to egui's default style that widget sizes depend on.
struct Tweak {
    name: &'static str,
    apply: fn(&mut Style),
    preset: Option<Theme>,
}

const TWEAKS: [Tweak; 8] = [
    Tweak {
        name: "shadcn light",
        apply: |_| {},
        preset: Some(Theme::Light),
    },
    Tweak {
        name: "shadcn dark",
        apply: |_| {},
        preset: Some(Theme::Dark),
    },
    Tweak {
        name: "default",
        apply: |_| {},
        preset: None,
    },
    Tweak {
        name: "roomy buttons",
        apply: |style| {
            style.spacing.button_padding = vec2(11.0, 5.0);
            style.spacing.interact_size = vec2(52.0, 31.0);
        },
        preset: None,
    },
    Tweak {
        name: "big icons",
        apply: |style| {
            style.spacing.icon_width = 19.0;
            style.spacing.icon_spacing = 7.0;
        },
        preset: None,
    },
    Tweak {
        name: "frameless buttons",
        apply: |style| style.visuals.button_frame = false,
        preset: None,
    },
    Tweak {
        name: "loose lines",
        apply: |style| style.spacing.extra_text_line_spacing = 3.0,
        preset: None,
    },
    Tweak {
        name: "wide controls",
        apply: |style| {
            style.spacing.slider_width = 170.0;
            style.spacing.combo_width = 140.0;
            style.spacing.text_edit_width = 210.0;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(2.0, Color32::GRAY);
            style.visuals.widgets.inactive.expansion = 2.0;
        },
        preset: None,
    },
];

const SIZES: [f32; 3] = [11.0, 14.0, 21.0];
const WIDTHS: [Option<f32>; 3] = [None, Some(150.0), Some(70.0)];

/// What egui allocates for `add`, drawn in a `Ui` `width` wide with the
/// node's font override, as the draw phase sets up its own.
fn drawn(
    ctx: &Context,
    text: &TextStyle,
    width: Option<f32>,
    mut add: impl FnMut(&mut Ui) -> Response,
) -> Vec2 {
    let mut size = Vec2::ZERO;
    for _ in 0..2 {
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.style_mut().override_font_id = Some(text.spec.font_id());
            let room = Rect::from_min_size(pos2(0.0, 0.0), vec2(width.unwrap_or(5000.0), 5000.0));
            ui.scope_builder(UiBuilder::new().max_rect(room), |ui| {
                size = add(ui).rect.size();
            });
        });
        output.drop_without_applying_deltas();
    }
    size
}

/// Pads an input as the preset says inputs are padded, which egui shares with
/// buttons; a no-op without a preset.
fn input_padded(ui: &mut Ui) {
    if let Some(tokens) = bullseye::style::tokens(ui.ctx()) {
        ui.spacing_mut().button_padding = tokens.input_padding;
    }
}

/// A context with the tweak applied and its fonts loaded, which sizes mean
/// nothing without.
fn context(tweak: &Tweak) -> Context {
    let ctx = Context::default();
    if let Some(theme) = tweak.preset {
        ctx.set_theme(theme);
        Shadcn::for_theme(theme).apply(&ctx);
    }
    ctx.global_style_mut(tweak.apply);
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

fn estimate(ctx: &Context, text: &TextStyle, width: Option<f32>, widget: &impl Stock) -> Vec2 {
    widget.size(ctx, text, width.map(Width::new))
}

type Add = Box<dyn Fn(&mut Ui, Option<f32>) -> Response>;

/// One case: the library's widget and the egui widget drawn for it.
struct Case<W> {
    widget: W,
    add: Add,
}

/// Asserts the estimate equals the drawn size over every tweak, font size and
/// width. Returns whether some case grew taller than at its natural width,
/// so callers can assert that wrapping was exercised.
fn agrees<W: Stock>(cases: impl Fn(&str) -> Case<W>, labels: &[&str]) -> bool {
    let mut wrapped = false;
    for tweak in &TWEAKS {
        let ctx = context(tweak);
        for size in SIZES {
            let text = text_style(size);
            for width in WIDTHS {
                for label in labels {
                    let case = cases(label);
                    let expected = drawn(&ctx, &text, width, |ui| (case.add)(ui, width));
                    let got = estimate(&ctx, &text, width, &case.widget);
                    assert_eq!(
                        got, expected,
                        "tweak {:?}, font {size}, width {width:?}, label {label:?}",
                        tweak.name
                    );
                    let natural = estimate(&ctx, &text, None, &case.widget);
                    wrapped |= got.y > natural.y;
                }
            }
        }
    }
    wrapped
}

const TEXTS: [&str; 5] = [
    "",
    "short",
    "two\nlines",
    "a paragraph of text long enough to wrap in a narrow field and more besides",
    "supercalifragilisticexpialidocious-and-unbreakable",
];

fn text_edit_case(text: &str, hint: Option<&'static str>, lines: Lines) -> Case<TextEdit> {
    let widget = match lines {
        Lines::Single => TextEdit::singleline(text),
        Lines::Multi { min, .. } => TextEdit::multiline(text, min),
    };
    let widget = match hint {
        Some(hint) => widget.hint(hint),
        None => widget,
    };
    let text = text.to_owned();
    Case {
        widget,
        add: Box::new(move |ui, width| {
            input_padded(ui);
            let mut buffer = text.clone();
            let mut edit = match lines {
                Lines::Single => egui::TextEdit::singleline(&mut buffer),
                Lines::Multi { min, .. } => {
                    egui::TextEdit::multiline(&mut buffer).desired_rows(min)
                }
            };
            if let Some(width) = width {
                edit = edit.desired_width(width);
            }
            if let Some(hint) = hint {
                edit = edit.hint_text(hint);
            }
            ui.add(edit)
        }),
    }
}

#[test]
fn single_line_text_edit() {
    for hint in [
        None,
        Some("Type here"),
        Some("a hint far too long to fit a narrow field"),
    ] {
        agrees(|text| text_edit_case(text, hint, Lines::Single), &TEXTS);
    }
}

#[test]
fn a_masked_one_line_field_is_the_size_of_a_shown_one() {
    agrees(
        |text| Case {
            widget: TextEdit::singleline(text).masked(),
            add: Box::new({
                let text = text.to_owned();
                move |ui, width| {
                    input_padded(ui);
                    let mut buffer = text.clone();
                    let mut edit = egui::TextEdit::singleline(&mut buffer).password(true);
                    if let Some(width) = width {
                        edit = edit.desired_width(width);
                    }
                    ui.add(edit)
                }
            }),
        },
        &TEXTS,
    );
}

fn bare_case(text: &str, lines: Lines) -> Case<TextEdit> {
    let widget = match lines {
        Lines::Single => TextEdit::singleline(text),
        Lines::Multi { min, .. } => TextEdit::multiline(text, min),
    };
    let text = text.to_owned();
    Case {
        widget: widget.hint("Reply").bare(),
        add: Box::new(move |ui, width| {
            let mut buffer = text.clone();
            let mut edit = match lines {
                Lines::Single => egui::TextEdit::singleline(&mut buffer),
                Lines::Multi { min, .. } => {
                    egui::TextEdit::multiline(&mut buffer).desired_rows(min)
                }
            };
            if let Some(width) = width {
                edit = edit.desired_width(width);
            }
            ui.add(edit.hint_text("Reply").frame(egui::Frame::NONE))
        }),
    }
}

#[test]
fn bare_text_edits_have_no_margin() {
    let mut wrapped = false;
    for lines in [
        Lines::Single,
        Lines::Multi { min: 1, max: None },
        Lines::Multi { min: 3, max: None },
    ] {
        wrapped |= agrees(|text| bare_case(text, lines), &TEXTS);
    }
    assert!(wrapped, "precondition: long text wraps in a narrow width");
    let ctx = context(&TWEAKS[0]);
    let text = text_style(14.0);
    let framed = estimate(&ctx, &text, Some(200.0), &TextEdit::multiline("x", 3));
    let bare = estimate(
        &ctx,
        &text,
        Some(200.0),
        &TextEdit::multiline("x", 3).bare(),
    );
    assert!(bare.y < framed.y, "the margin is gone: {bare:?} {framed:?}");
}

#[test]
fn multi_line_text_edit() {
    let mut wrapped = false;
    for lines in [
        Lines::Multi { min: 1, max: None },
        Lines::Multi { min: 3, max: None },
    ] {
        for hint in [
            None,
            Some("Type here"),
            Some("a hint far too long to fit a narrow field"),
        ] {
            wrapped |= agrees(|text| text_edit_case(text, hint, lines), &TEXTS);
        }
    }
    assert!(wrapped, "precondition: long text wraps in a narrow width");
}

#[test]
fn a_row_cap_stops_a_multi_line_field_growing() {
    let tweak = &TWEAKS[0];
    let ctx = context(tweak);
    let text = text_style(14.0);
    let long = "line\n".repeat(12);
    let free = estimate(&ctx, &text, Some(200.0), &TextEdit::multiline(&long, 2));
    let capped = estimate(
        &ctx,
        &text,
        Some(200.0),
        &TextEdit::multiline(&long, 2).max_rows(4),
    );
    let short = estimate(
        &ctx,
        &text,
        Some(200.0),
        &TextEdit::multiline("", 2).max_rows(4),
    );
    assert!(
        free.y > capped.y,
        "precondition: the text is taller than the cap"
    );
    assert!(
        capped.y > short.y,
        "four rows are taller than the two-row minimum"
    );
    assert_eq!(capped.x, free.x);
    let one_more = estimate(
        &ctx,
        &text,
        Some(200.0),
        &TextEdit::multiline(&long, 2).max_rows(5),
    );
    assert!(one_more.y > capped.y);
}

/// The estimates are for a resting widget. egui folds a hovered or pressed
/// widget's stroke and expansion into its margin, so the size must not move.
#[test]
fn hover_press_and_focus_do_not_change_the_size() {
    #[derive(Default)]
    struct Sizes {
        button: Vec2,
        edit: Vec2,
        text: String,
    }
    let mut harness = egui_kittest::Harness::builder()
        .with_size(vec2(400.0, 200.0))
        .build_ui_state(
            |ui, sizes: &mut Sizes| {
                sizes.button = ui.add(egui::Button::new("Press me")).rect.size();
                sizes.edit = ui
                    .add(egui::TextEdit::singleline(&mut sizes.text).hint_text("name"))
                    .rect
                    .size();
            },
            Sizes::default(),
        );
    harness.run();
    let resting = (harness.state().button, harness.state().edit);
    assert!(resting.0.x > 0.0 && resting.1.x > 0.0, "precondition");

    let button_at = pos2(30.0, 15.0);
    harness.hover_at(button_at);
    harness.run_steps(3);
    assert_eq!(harness.state().button, resting.0, "hovered");
    harness.drag_at(button_at);
    harness.run_steps(2);
    assert_eq!(harness.state().button, resting.0, "pressed");
    harness.drop_at(button_at);

    let edit_at = pos2(100.0, 45.0);
    harness.hover_at(edit_at);
    harness.run_steps(2);
    harness.drag_at(edit_at);
    harness.step();
    harness.drop_at(edit_at);
    harness.run_steps(3);
    assert!(
        harness.ctx.memory(|memory| memory.focused().is_some()),
        "precondition: the click focused the text edit"
    );
    assert_eq!(harness.state().edit, resting.1, "focused");
}
