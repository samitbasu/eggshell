//! Stock widgets as leaves, through the full path: the builder sizes them
//! with the library's estimate, the draw phase places egui's widget in the
//! leaf's rect, and pointer events reach it.

use bullseye::widget::{
    Button, Checkbox, ComboBox, DragValue, RadioButton, Slider, Stock, TextEdit,
};
use bullseye::{Builder, TextSpec, TextStyle, Width, tw};
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support;

use support::{Rig, click_node, drag, rect_of};

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::WHITE,
    )
}

#[derive(Default)]
struct App {
    clicks: u32,
    checked: bool,
    choice: u8,
    level: f64,
    amount: f64,
    pick: usize,
}

const OPTIONS: [&str; 3] = ["Alpha", "Beta", "Gamma"];

/// ```html
/// <div class="flex flex-col items-start gap-2 p-2">
///   <button class="px-2">Save</button>
///   <label><input type="checkbox"> Enabled</label>
///   <label><input type="radio"> One</label> <label><input type="radio"> Two</label>
///   <input type="range" class="w-[200px]">
///   <input type="number" class="w-[80px]">
///   <select><option>Alpha</option>…</select>
/// </div>
/// ```
fn form(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0).p(2.0), |b| {
        b.widget("save", tw(), Button::new("Save"), |app, ui, button| {
            if button.show(ui).clicked() {
                app.clicks += 1;
            }
        });
        b.widget("enabled", tw(), Checkbox::new("Enabled"), |app, ui, w| {
            w.show(ui, &mut app.checked);
        });
        for choice in 1..=2u8 {
            b.widget(
                ("radio", choice),
                tw(),
                RadioButton::new(format!("Choice {choice}")),
                move |app, ui, w| {
                    if w.show(ui, app.choice == choice).clicked() {
                        app.choice = choice;
                    }
                },
            );
        }
        b.widget(
            "level",
            tw().w_px(200.0),
            Slider::new().named("Level"),
            |app, ui, w| {
                w.show(ui, &mut app.level, 0.0..=100.0);
            },
        );
        b.widget(
            "amount",
            tw(),
            DragValue::new("000.0").named("Amount"),
            |app, ui, w| {
                w.show(ui, &mut app.amount, 0.0..=1000.0);
            },
        );
        b.widget(
            "pick",
            tw(),
            ComboBox::new(OPTIONS[app.pick]).named("Pick"),
            |app, ui, w| {
                w.show(ui, |ui| {
                    for (index, option) in OPTIONS.iter().enumerate() {
                        ui.selectable_value(&mut app.pick, index, *option);
                    }
                });
            },
        );
    });
}

fn harness() -> Harness<'static, Rig<App>> {
    support::harness(vec2(400.0, 400.0), base_text(), App::default(), form)
}

#[test]
fn a_leaf_is_as_large_as_the_estimate() {
    let mut harness = harness();
    harness.run();
    let ctx = harness.ctx.clone();
    let text = base_text();
    for (path, estimate) in [
        ("/\"save\"", Button::new("Save").size(&ctx, &text, None)),
        (
            "/\"enabled\"",
            Checkbox::new("Enabled").size(&ctx, &text, None),
        ),
        (
            "/\"amount\"",
            DragValue::new("000.0").size(&ctx, &text, None),
        ),
        (
            "/\"level\"",
            Slider::new().size(&ctx, &text, Some(Width::new(200.0))),
        ),
    ] {
        assert_eq!(rect_of(&harness, path).size(), estimate, "{path}");
    }
}

#[test]
fn widgets_respond_to_the_pointer() {
    let mut harness = harness();
    harness.run();

    click_node(&mut harness, "/\"save\"");
    assert_eq!(harness.state().app.clicks, 1);

    click_node(&mut harness, "/\"enabled\"");
    assert!(harness.state().app.checked);

    click_node(&mut harness, "/(\"radio\", 2)");
    assert_eq!(harness.state().app.choice, 2);

    let level = rect_of(&harness, "/\"level\"");
    drag(
        &mut harness,
        level.left_center() + vec2(2.0, 0.0),
        level.right_center() - vec2(2.0, 0.0),
    );
    assert!(
        harness.state().app.level > 90.0,
        "dragged to the end: {}",
        harness.state().app.level
    );
}

#[test]
fn a_combo_box_opens_and_selects() {
    let mut harness = harness();
    harness.run();
    click_node(&mut harness, "/\"pick\"");
    harness.run_steps(3);
    assert_eq!(harness.state().app.pick, 0, "precondition");
    harness.get_by_label("Gamma").click();
    harness.run_steps(3);
    assert_eq!(harness.state().app.pick, 2);
    assert_eq!(
        rect_of(&harness, "/\"pick\"").size(),
        ComboBox::new("Gamma").size(&harness.ctx, &base_text(), None),
        "the header follows the selection"
    );
}

#[derive(Default)]
struct Secret {
    password: String,
}

fn secret(b: &mut Builder<'_, '_, Secret>, app: &Secret) {
    b.widget(
        "password",
        tw().w_px(200.0),
        TextEdit::singleline(&app.password)
            .masked()
            .named("Password"),
        |app, ui, field| {
            field.show(ui, &mut app.password);
        },
    );
}

/// The text of every galley painted in the last frame.
fn painted_text<A>(harness: &Harness<'_, Rig<A>>) -> Vec<String> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_masked_field_edits_the_text_and_paints_dots() {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), Secret::default(), secret);
    harness.run_steps(2);
    let unmasked = rect_of(&harness, "/\"password\"");
    click_node(&mut harness, "/\"password\"");
    harness.run_steps(2);
    harness.event(egui::Event::Text("hunter2".to_owned()));
    harness.run_steps(3);

    assert_eq!(harness.state().app.password, "hunter2");
    let painted = painted_text(&harness).concat();
    assert!(!painted.contains("hunter2"), "painted {painted:?}");
    assert!(painted.contains('•'), "painted {painted:?}");
    assert_eq!(rect_of(&harness, "/\"password\""), unmasked);
}

#[test]
fn a_drag_value_shows_the_decimals_of_its_template() {
    let mut harness = harness();
    harness.state_mut().app.amount = 7.46;
    harness.run_steps(3);
    let painted = painted_text(&harness);
    assert!(
        painted.iter().any(|text| text == "7.5"),
        "a one-decimal template shows one decimal: {painted:?}"
    );
}

const NARROW_LABEL: &str = "A label that has to wrap";

#[derive(Default)]
struct Wide {
    seen: Vec<(&'static str, egui::Rect)>,
}

fn wide(b: &mut Builder<'_, '_, Wide>, _: &Wide) {
    b.div(tw().flex().flex_col().gap(2.0), |b| {
        b.widget(
            "check",
            tw().w_px(300.0),
            Checkbox::new("Wide"),
            |app, ui, w| {
                let response = w.show(ui, &mut false);
                app.seen.push(("check", response.rect));
            },
        );
        b.widget(
            "radio",
            tw().w_px(300.0),
            RadioButton::new("Wide"),
            |app, ui, w| {
                let response = w.show(ui, false);
                app.seen.push(("radio", response.rect));
            },
        );
        b.widget(
            "narrow",
            tw().w_px(90.0),
            Checkbox::new(NARROW_LABEL),
            |app, ui, w| {
                let response = w.show(ui, &mut false);
                app.seen.push(("narrow", response.rect));
            },
        );
    });
}

#[test]
fn a_checkbox_in_a_wider_leaf_stays_at_the_left_at_its_own_size() {
    let mut harness = support::harness(vec2(400.0, 120.0), base_text(), Wide::default(), wide);
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    let text = base_text();
    for (name, estimate) in [
        ("check", Checkbox::new("Wide").size(&ctx, &text, None)),
        ("radio", RadioButton::new("Wide").size(&ctx, &text, None)),
    ] {
        let leaf = rect_of(&harness, &format!("/\"{name}\""));
        let (_, drawn) = *harness
            .state()
            .app
            .seen
            .iter()
            .rfind(|(seen, _)| *seen == name)
            .expect("drawn");
        assert_eq!(leaf.width(), 300.0, "precondition: a wide leaf");
        assert!(estimate.x < 200.0, "precondition: the widget is narrower");
        assert_eq!(drawn.min.x, leaf.min.x, "{name} starts at the left edge");
        assert_eq!(
            drawn.size(),
            estimate,
            "{name} keeps the size egui gives it"
        );
    }
}

#[test]
fn a_checkbox_label_still_wraps_in_a_narrow_leaf() {
    let mut harness = support::harness(vec2(400.0, 200.0), base_text(), Wide::default(), wide);
    harness.run_steps(2);
    let estimate =
        Checkbox::new(NARROW_LABEL).size(&harness.ctx, &base_text(), Some(Width::new(90.0)));
    let one_line = Checkbox::new(NARROW_LABEL).size(&harness.ctx, &base_text(), None);
    let (_, drawn) = *harness
        .state()
        .app
        .seen
        .iter()
        .rfind(|(seen, _)| *seen == "narrow")
        .expect("drawn");
    assert!(estimate.y > one_line.y, "precondition: the label wraps");
    assert_eq!(drawn.size(), estimate);
}
