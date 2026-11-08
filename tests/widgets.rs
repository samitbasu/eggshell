//! Stock widgets as leaves, through the full path: the builder sizes them
//! with the library's estimate, the draw phase places egui's widget in the
//! leaf's rect, and pointer events reach it.

use bullseye::widget::{Button, Checkbox, ComboBox, DragValue, RadioButton, Slider, Stock};
use bullseye::{Builder, TextSpec, TextStyle, Width, tw};
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

mod support;

use support::{Accessibility, Rig, click_node, drag, rect_of};

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
        b.widget("level", tw().w_px(200.0), Slider::new(), |app, ui, w| {
            w.show(ui, &mut app.level, 0.0..=100.0);
        });
        b.widget("amount", tw(), DragValue::new("000.0"), |app, ui, w| {
            w.show(ui, &mut app.amount, 0.0..=1000.0);
        });
        b.widget(
            "pick",
            tw(),
            ComboBox::new(OPTIONS[app.pick]),
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
    support::harness_with(
        vec2(400.0, 400.0),
        base_text(),
        App::default(),
        form,
        Accessibility::Unchecked,
    )
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
