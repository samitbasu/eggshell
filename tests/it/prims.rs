//! The interactive primitives through the full path: measured size against
//! the size egui uses, ids that follow keys through
//! reordering, inert subtrees, and a multi-line field that grows to a cap.

use std::collections::BTreeMap;

use bullseye::owned::{Cell, Segmented};
use bullseye::widget::{Stock, TextEdit};
use bullseye::{Builder, TextSpec, TextStyle, Width, tw};
use egui::epaint::Shape;
use egui::{Color32, Event, FontFamily, Id, Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support;

use support::{Rig, click_node, rect_of};

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
    one_line: String,
    notes: String,
    inert: bool,
    /// Keyed rows, in display order.
    rows: Vec<(u32, String)>,
    /// The egui id of each row's field, as `show` reported it last frame.
    field_ids: BTreeMap<u32, Id>,
    /// The size egui reported for each leaf's widget.
    used: BTreeMap<&'static str, egui::Vec2>,
}

/// ```html
/// <div class="flex flex-col items-start gap-2 p-2">
///   <input class="w-[220px]" placeholder="Name">
///   <textarea class="w-[220px]" placeholder="Notes"></textarea>
/// </div>
/// ```
fn controls(b: &mut Builder<'_, '_, App>, app: &App) {
    let live = if app.inert { tw().inert() } else { tw() };
    b.div(live.flex().flex_col().items_start().gap(2.0).p(2.0), |b| {
        b.widget(
            "name",
            tw().w_px(220.0),
            TextEdit::singleline(&app.one_line)
                .hint("Name")
                .named("Name"),
            |app, ui, field| {
                let response = field.show(ui, &mut app.one_line);
                app.used.insert("name", response.rect.size());
            },
        );
        b.widget(
            "notes",
            tw().w_px(220.0),
            TextEdit::multiline(&app.notes, 2)
                .max_rows(4)
                .hint("Notes")
                .named("Notes"),
            |app, ui, field| {
                let response = field.show(ui, &mut app.notes);
                app.used.insert("notes", response.rect.size());
            },
        );
    });
}

fn harness_of(view: support::View<App>, app: App) -> Harness<'static, Rig<App>> {
    support::harness(vec2(400.0, 500.0), base_text(), app, view)
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = harness_of(controls, App::default());
    harness.run_steps(3);
    harness
}

fn type_text(harness: &mut Harness<'_, Rig<App>>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run_steps(2);
}

fn focus(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    click_node(harness, path);
    harness.run_steps(2);
}

#[test]
fn measured_size_equals_the_size_used() {
    let harness = harness();
    let ctx = harness.ctx.clone();
    let text = base_text();
    let app = &harness.state().app;

    for (name, measured) in [
        (
            "name",
            TextEdit::singleline("")
                .hint("Name")
                .size(&ctx, &text, Some(Width::new(220.0))),
        ),
        (
            "notes",
            TextEdit::multiline("", 2).max_rows(4).hint("Notes").size(
                &ctx,
                &text,
                Some(Width::new(220.0)),
            ),
        ),
    ] {
        let path = format!("/{name:?}");
        let rect = rect_of(&harness, &path);
        assert_eq!(rect.size(), measured, "{name}: the leaf is as measured");
        // The draw phase snaps the content rect to whole pixels.
        let used = app.used[name];
        assert!(
            (used - measured).abs().max_elem() <= 0.5,
            "{name}: used {used:?}, measured {measured:?}"
        );
    }
}

#[test]
fn text_fields_edit_and_show_their_hint() {
    let mut harness = harness();
    focus(&mut harness, "/\"name\"");
    type_text(&mut harness, "Ada");
    assert_eq!(harness.state().app.one_line, "Ada");
}

#[test]
fn an_inert_subtree_disables_its_text_fields() {
    let mut harness = harness();
    focus(&mut harness, "/\"name\"");
    type_text(&mut harness, "x");
    assert_eq!(
        harness.state().app.one_line,
        "x",
        "precondition: editable while live"
    );

    harness.state_mut().app.inert = true;
    harness.run_steps(3);
    let before = harness.state().app.one_line.clone();

    focus(&mut harness, "/\"name\"");
    type_text(&mut harness, "yz");
    assert_eq!(harness.state().app.one_line, before, "no edit");
}

/// ```html
/// <div class="flex flex-col gap-1 p-2">
///   <input class="w-[200px]" *> <!-- one per row, keyed by id -->
/// </div>
/// ```
fn rows(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().gap(1.0).p(2.0), |b| {
        for (key, text) in &app.rows {
            let key = *key;
            b.widget(
                ("row", key),
                tw().w_px(200.0),
                TextEdit::singleline(text).named(format!("Row {key}")),
                move |app, ui, field| {
                    let Some((_, text)) = app.rows.iter_mut().find(|(k, _)| *k == key) else {
                        return;
                    };
                    let response = field.show(ui, text);
                    app.field_ids.insert(key, response.id);
                },
            );
        }
    });
}

#[test]
fn text_field_focus_and_cursor_survive_a_reorder() {
    let app = App {
        rows: vec![
            (1, "alpha".into()),
            (2, "beta gamma".into()),
            (3, "delta".into()),
        ],
        ..App::default()
    };
    let mut harness = harness_of(rows, app);
    harness.run_steps(3);

    focus(&mut harness, "/(\"row\", 2)");
    harness.key_press(Key::Home);
    harness.run_steps(2);
    for _ in 0..4 {
        harness.key_press(Key::ArrowRight);
        harness.run_steps(1);
    }
    let id = harness.state().app.field_ids[&2];
    let cursor = |harness: &Harness<'_, Rig<App>>| {
        egui::TextEdit::load_state(&harness.ctx, id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| usize::from(range.primary.index))
    };
    assert_eq!(cursor(&harness), Some(4), "precondition: the cursor moved");
    assert_eq!(harness.ctx.memory(|m| m.focused()), Some(id));
    let before = rect_of(&harness, "/(\"row\", 2)");

    harness.state_mut().app.rows.rotate_left(1);
    harness.run_steps(3);
    let after = rect_of(&harness, "/(\"row\", 2)");
    assert_eq!(
        after.min.x, before.min.x,
        "precondition: only the vertical position changed"
    );
    assert_ne!(after.min.y, before.min.y, "precondition: the row moved");
    assert_eq!(
        harness.state().app.field_ids[&2],
        id,
        "same id after the move"
    );
    assert_eq!(
        harness.ctx.memory(|m| m.focused()),
        Some(id),
        "still focused"
    );
    assert_eq!(cursor(&harness), Some(4), "cursor kept");

    type_text(&mut harness, "!");
    assert_eq!(harness.state().app.rows[0].1, "beta! gamma");
}

#[test]
fn a_multi_line_field_grows_with_its_content_up_to_its_cap() {
    let mut harness = harness();
    let height = |harness: &Harness<'_, Rig<App>>| rect_of(harness, "/\"notes\"").height();
    let resting = height(&harness);

    let mut heights = Vec::new();
    for lines in 1..=8 {
        harness.state_mut().app.notes = vec!["line"; lines].join("\n");
        harness.run_steps(3);
        heights.push(height(&harness));
    }
    assert_eq!(
        heights[0], resting,
        "up to the two-row minimum nothing grows"
    );
    assert!(heights[2] > heights[1], "a third row grows it: {heights:?}");
    assert!(
        heights[3] > heights[2],
        "a fourth row grows it: {heights:?}"
    );
    let cap = TextEdit::multiline("", 4)
        .size(&harness.ctx, &base_text(), Some(Width::new(220.0)))
        .y;
    assert!(
        heights[3] <= cap,
        "four rows of text fit the cap: {heights:?}"
    );
    assert!(
        heights[4..].iter().all(|&h| h == cap),
        "the cap is the height of a four-row field, {cap}: {heights:?}"
    );

    focus(&mut harness, "/\"notes\"");
    type_text(&mut harness, "tail");
    assert!(
        harness.state().app.notes.contains("tail"),
        "a capped field still edits: {:?}",
        harness.state().app.notes
    );
    assert_eq!(height(&harness), cap);
}

#[test]
fn typing_enter_in_a_multi_line_field_grows_it() {
    let mut harness = harness();
    focus(&mut harness, "/\"notes\"");
    let start = rect_of(&harness, "/\"notes\"").height();
    for _ in 0..3 {
        harness.key_press(Key::Enter);
        harness.run_steps(2);
    }
    assert!(
        rect_of(&harness, "/\"notes\"").height() > start,
        "three newlines make a fourth row"
    );
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
enum Mode {
    #[default]
    Day,
    Week,
    Month,
}

#[derive(Default)]
struct Picker {
    mode: Mode,
    inert: bool,
}

const ON: Color32 = Color32::from_rgb(20, 100, 220);
const OFF: Color32 = Color32::from_rgb(50, 50, 50);

/// ```html
/// <div class="flex gap-px">
///   <span class="px-3 py-1 bg-[#1464dc]">Day</span>   <!-- the pressed cell -->
///   <span class="px-3 py-1 bg-[#323232]">Week</span>
///   <span class="px-3 py-1 bg-[#323232]">Month</span>
/// </div>
/// ```
fn picker(b: &mut Builder<'_, '_, Picker>, app: &Picker) {
    let container = if app.inert { tw().inert() } else { tw() };
    Segmented::new(
        app.mode,
        &[
            (Mode::Day, "Day"),
            (Mode::Week, "Week"),
            (Mode::Month, "Month"),
        ],
    )
    .show_with(
        b,
        "mode",
        container.flex().gap(1.0).p(2.0),
        |cell| match cell {
            Cell::Pressed => tw().px(3.0).py(1.0).bg(ON),
            Cell::Released => tw().px(3.0).py(1.0).bg(OFF),
        },
        |app, _, mode| app.mode = mode,
    );
}

fn picker_harness() -> Harness<'static, Rig<Picker>> {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), Picker::default(), picker);
    harness.run_steps(3);
    harness
}

fn cell_path(mode: Mode) -> String {
    format!("/\"mode\"/{mode:?}")
}

/// The modes whose cells are painted pressed.
fn pressed(harness: &Harness<'_, Rig<Picker>>) -> Vec<Mode> {
    [Mode::Day, Mode::Week, Mode::Month]
        .into_iter()
        .filter(|&mode| support::fill_of(harness, &cell_path(mode)) == Some(ON))
        .collect()
}

fn choose(harness: &mut Harness<'_, Rig<Picker>>, mode: Mode) {
    click_node(harness, &cell_path(mode));
    harness.run_steps(2);
}

#[test]
fn a_segmented_control_has_exactly_one_pressed_cell_and_clicks_change_it() {
    let mut harness = picker_harness();
    let rects: Vec<_> = [Mode::Day, Mode::Week, Mode::Month]
        .map(|mode| rect_of(&harness, &cell_path(mode)))
        .into();
    assert!(
        rects.windows(2).all(|pair| pair[0].max.x < pair[1].min.x),
        "precondition: three separate cells in a row: {rects:?}"
    );
    assert_eq!(pressed(&harness), [Mode::Day]);

    choose(&mut harness, Mode::Month);
    assert_eq!(harness.state().app.mode, Mode::Month, "through the lens");
    assert_eq!(pressed(&harness), [Mode::Month]);

    click_node(&mut harness, &cell_path(Mode::Month));
    assert_eq!(
        pressed(&harness),
        [Mode::Month],
        "clicking the chosen cell keeps it"
    );

    choose(&mut harness, Mode::Week);
    assert_eq!(pressed(&harness), [Mode::Week]);
}

#[test]
fn the_keyboard_chooses_a_cell() {
    let mut harness = picker_harness();
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(
        pressed(&harness),
        [Mode::Week],
        "Tab visits cells in order, Enter chooses"
    );
}

#[test]
fn an_inert_segmented_control_does_not_change() {
    let mut harness = picker_harness();
    harness.state_mut().app.inert = true;
    harness.run_steps(3);
    choose(&mut harness, Mode::Month);
    assert_eq!(harness.state().app.mode, Mode::Day);
    assert_eq!(pressed(&harness), [Mode::Day]);
}

#[derive(Default)]
struct Centred {
    height: f32,
    text: String,
    multi: bool,
    bare: bool,
}

fn centred(b: &mut Builder<'_, '_, Centred>, app: &Centred) {
    let field = if app.multi {
        TextEdit::multiline(&app.text, 1)
    } else {
        TextEdit::singleline(&app.text)
    };
    let field = if app.bare { field.bare() } else { field };
    b.widget(
        "field",
        tw().w_px(200.0).h_px(app.height),
        field.named("Field"),
        |app, ui, field| {
            field.show(ui, &mut app.text);
        },
    );
}

/// The vertical extent of the text the field painted.
fn painted_text(harness: &Harness<'_, Rig<Centred>>) -> (f32, f32) {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "Hxg" => Some((
                text.pos.y + text.galley.rect.min.y,
                text.pos.y + text.galley.rect.max.y,
            )),
            _ => None,
        })
        .expect("the field painted its text")
}

#[test]
fn a_single_line_field_centres_its_line_in_a_taller_leaf() {
    for ppp in [1.0, 1.7, 2.0] {
        for size in [12.0, 16.0, 24.0] {
            for height in [60.0, 80.0, 110.0] {
                for bare in [false, true] {
                    let app = Centred {
                        height,
                        text: "Hxg".to_owned(),
                        bare,
                        ..Centred::default()
                    };
                    let mut base = base_text();
                    base.spec.size = size;
                    base.spec.line_height = size * 1.25;
                    let mut harness = support::harness_scaled(
                        vec2(300.0, 200.0),
                        ppp,
                        base,
                        app,
                        centred,
                        |_, _| {},
                    );
                    harness.run_steps(3);
                    let leaf = rect_of(&harness, "/\"field\"");
                    let (top, bottom) = painted_text(&harness);
                    assert!(
                        leaf.height() > bottom - top + 12.0,
                        "precondition: the leaf is taller than the line ({leaf:?}, {top}..{bottom})"
                    );
                    let centre = (top + bottom) / 2.0;
                    assert!(
                        (centre - leaf.center().y).abs() <= 1.0,
                        "ppp {ppp}, size {size}, height {height}, bare {bare}: \
                         text centre {centre}, leaf centre {}",
                        leaf.center().y
                    );
                }
            }
        }
    }
}

#[test]
fn a_multi_line_field_keeps_its_text_at_the_top() {
    for ppp in [1.0, 2.0] {
        let app = Centred {
            height: 90.0,
            text: "Hxg".to_owned(),
            multi: true,
            ..Centred::default()
        };
        let mut harness = support::harness_scaled(
            vec2(300.0, 200.0),
            ppp,
            base_text(),
            app,
            centred,
            |_, _| {},
        );
        harness.run_steps(3);
        let leaf = rect_of(&harness, "/\"field\"");
        let (top, _) = painted_text(&harness);
        assert!(leaf.height() >= 90.0, "precondition: a tall leaf");
        assert!(
            top - leaf.top() <= 6.0,
            "ppp {ppp}: text starts at the top, {top} in {leaf:?}"
        );
    }
}

#[test]
fn named_primitives_are_found_by_their_accessible_name() {
    let harness = harness();
    for name in ["Name", "Notes"] {
        let _ = harness.get_by_label(name);
    }
}
