//! The interactive primitives through the full path: measured size against
//! the size egui or the primitive uses, ids that follow keys through
//! reordering, inert subtrees, and a multi-line field that grows to a cap.

use std::collections::BTreeMap;

use bullseye::prim::segmented::{Cell, segmented};
use bullseye::prim::select::{self, Options};
use bullseye::prim::switch::Switch;
use bullseye::widget::{Stock, TextEdit};
use bullseye::{Builder, TextSpec, TextStyle, Width, tw};
use egui::epaint::Shape;
use egui::{Color32, Event, FontFamily, Id, Key, Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

mod support;

use support::{Accessibility, Rig, click_node, rect_of};

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
    on: bool,
    pick: usize,
    inert: bool,
    /// Keyed rows, in display order.
    rows: Vec<(u32, String)>,
    /// The egui id of each row's field, as `show` reported it last frame.
    field_ids: BTreeMap<u32, Id>,
    /// The size egui reported for each leaf's widget.
    used: BTreeMap<&'static str, egui::Vec2>,
}

const CHOICES: [&str; 3] = ["North", "South-west", "Up"];

fn choices() -> Options {
    Options::new(CHOICES)
}

/// ```html
/// <div class="flex flex-col items-start gap-2 p-2">
///   <input class="w-[220px]" placeholder="Name">
///   <textarea class="w-[220px]" placeholder="Notes"></textarea>
///   <label class="switch">Wi-Fi</label>
///   <select>…</select>
/// </div>
/// ```
fn controls(b: &mut Builder<'_, '_, App>, app: &App) {
    let live = if app.inert { tw().inert() } else { tw() };
    b.div(live.flex().flex_col().items_start().gap(2.0).p(2.0), |b| {
        b.widget(
            "name",
            tw().w_px(220.0),
            TextEdit::singleline(&app.one_line).hint("Name"),
            |app, ui, field| {
                let response = field.show(ui, &mut app.one_line);
                app.used.insert("name", response.rect.size());
            },
        );
        b.widget(
            "notes",
            tw().w_px(220.0),
            TextEdit::multiline(&app.notes, 2).max_rows(4).hint("Notes"),
            |app, ui, field| {
                let response = field.show(ui, &mut app.notes);
                app.used.insert("notes", response.rect.size());
            },
        );
        b.widget("wifi", tw(), Switch::new("Wi-Fi"), |app, ui, switch| {
            let response = switch.show(ui, &mut app.on);
            app.used.insert("wifi", response.rect.size());
        });
        b.widget("bare", tw(), Switch::unlabelled(), |app, ui, switch| {
            switch.show(ui, &mut app.on);
        });
        b.widget("pick", tw(), choices(), |app, ui, options| {
            let response = select::show(ui, &mut app.pick, &options);
            app.used.insert("pick", response.rect.size());
        });
    });
}

fn harness_of(view: support::View<App>, app: App) -> Harness<'static, Rig<App>> {
    support::harness_with(
        vec2(400.0, 500.0),
        base_text(),
        app,
        view,
        Accessibility::Unchecked,
    )
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
        ("wifi", Switch::new("Wi-Fi").size(&ctx, &text, None)),
        ("pick", choices().size(&ctx, &text, None)),
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
fn a_switch_paints_exactly_its_measured_row() {
    let harness = harness();
    let rect = rect_of(&harness, "/\"wifi\"");
    let painted = harness
        .output()
        .shapes
        .iter()
        .filter(|clipped| rect.contains_rect(clipped.clip_rect.intersect(rect)))
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(_) | Shape::Rect(_) | Shape::Circle(_) => {
                Some(clipped.shape.visual_bounding_rect())
            }
            _ => None,
        })
        .filter(|bounds| rect.expand(0.5).contains_rect(*bounds))
        .fold(Rect::NOTHING, Rect::union);
    assert!(painted.is_positive(), "precondition: the switch painted");
    assert_eq!(painted.min.x, rect.min.x);
    assert!(
        painted.max.x <= rect.max.x + 0.01 && rect.max.x - painted.max.x < 2.0,
        "the label's ink ends where the measure says: {painted:?} in {rect:?}"
    );
}

#[test]
fn a_switch_toggles_and_the_unlabelled_one_is_just_the_track() {
    let mut harness = harness();
    assert!(!harness.state().app.on, "precondition");
    click_node(&mut harness, "/\"wifi\"");
    assert!(harness.state().app.on);
    click_node(&mut harness, "/\"bare\"");
    assert!(!harness.state().app.on);
    assert!(
        rect_of(&harness, "/\"bare\"").width() < rect_of(&harness, "/\"wifi\"").width(),
        "the label takes room"
    );
}

#[test]
fn a_select_opens_chooses_and_keeps_its_size() {
    let mut harness = harness();
    let before = rect_of(&harness, "/\"pick\"");
    click_node(&mut harness, "/\"pick\"");
    harness.run_steps(3);
    harness.get_by_label("South-west").click();
    harness.run_steps(3);
    assert_eq!(harness.state().app.pick, 1);
    assert_eq!(rect_of(&harness, "/\"pick\""), before);

    let text = base_text();
    let widest = choices().size(&harness.ctx, &text, None);
    let each = CHOICES.map(|label| Options::new([label]).size(&harness.ctx, &text, None));
    assert!(
        each.iter().any(|size| size.x < widest.x - 0.5),
        "precondition: the options differ in width"
    );
    assert_eq!(
        widest.x,
        each.iter().fold(0.0, |wide, size| size.x.max(wide))
    );
}

#[test]
fn text_fields_edit_and_show_their_hint() {
    let mut harness = harness();
    focus(&mut harness, "/\"name\"");
    type_text(&mut harness, "Ada");
    assert_eq!(harness.state().app.one_line, "Ada");
}

#[test]
fn an_inert_subtree_disables_its_widgets() {
    let mut harness = harness();
    focus(&mut harness, "/\"name\"");
    type_text(&mut harness, "x");
    assert_eq!(
        harness.state().app.one_line,
        "x",
        "precondition: editable while live"
    );
    click_node(&mut harness, "/\"wifi\"");
    assert!(harness.state().app.on, "precondition: toggles while live");
    click_node(&mut harness, "/\"wifi\"");

    harness.state_mut().app.inert = true;
    harness.run_steps(3);
    let app = |harness: &Harness<'_, Rig<App>>| {
        let app = &harness.state().app;
        (app.one_line.clone(), app.on, app.pick)
    };
    let before = app(&harness);

    focus(&mut harness, "/\"name\"");
    type_text(&mut harness, "yz");
    click_node(&mut harness, "/\"wifi\"");
    click_node(&mut harness, "/\"pick\"");
    harness.run_steps(3);
    assert!(
        harness.query_by_label("South-west").is_none(),
        "the select did not open"
    );
    assert_eq!(app(&harness), before, "no edit, no toggle, no choice");
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
                TextEdit::singleline(text),
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
    segmented(
        b,
        "mode",
        container.flex().gap(1.0).p(2.0),
        app.mode,
        &[
            (Mode::Day, "Day"),
            (Mode::Week, "Week"),
            (Mode::Month, "Month"),
        ],
        |cell| match cell {
            Cell::Pressed => tw().px(3.0).py(1.0).bg(ON),
            Cell::Released => tw().px(3.0).py(1.0).bg(OFF),
        },
        |app| &mut app.mode,
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
