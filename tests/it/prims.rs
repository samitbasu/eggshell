//! The interactive primitives through the full path: measured size against
//! the size egui uses, ids that follow keys through
//! reordering, inert subtrees, and a multi-line field that grows to a cap.

use std::collections::BTreeMap;

use eggshell_ui::owned::Cell;
use eggshell_ui::prim::text_field::Field;
use eggshell_ui::testing::Headless;
use eggshell_ui::testing::{Driver, PixelsPerPoint, Target};
use eggshell_ui::widget::{Stock, TextEdit};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, Width, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Id, Key, Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
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

fn harness_of(view: support::View<App>, app: App) -> Driver<App> {
    support::driver(vec2(400.0, 500.0), base_text(), app, view).build()
}

fn harness() -> Driver<App> {
    let mut d = harness_of(controls, App::default());
    d.frames(3);
    d
}

#[test]
fn measured_size_equals_the_size_used() {
    let d = harness();
    let ctx = d.ctx().clone();
    let text = base_text();
    let style = d.shell().style();
    let app = &d.app();

    for (name, measured) in [
        (
            "name",
            TextEdit::singleline("").hint("Name").size(
                &ctx,
                &text,
                &style,
                Some(Width::new(220.0)),
            ),
        ),
        (
            "notes",
            TextEdit::multiline("", 2).max_rows(4).hint("Notes").size(
                &ctx,
                &text,
                &style,
                Some(Width::new(220.0)),
            ),
        ),
    ] {
        let path = format!("/{name:?}");
        let rect = d.rect(Target::suffix(&path));
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
    let mut d = harness();
    d.click(Target::suffix("/\"name\"")).frames(2);
    d.type_text("Ada").frames(1);
    assert_eq!(d.app().one_line, "Ada");
}

#[test]
fn an_inert_subtree_disables_its_text_fields() {
    let mut d = harness();
    d.click(Target::suffix("/\"name\"")).frames(2);
    d.type_text("x").frames(1);
    assert_eq!(d.app().one_line, "x", "precondition: editable while live");

    d.app_mut().inert = true;
    d.frames(3);
    let before = d.app().one_line.clone();

    d.click(Target::suffix("/\"name\"")).frames(2);
    d.type_text("yz").frames(1);
    assert_eq!(d.app().one_line, before, "no edit");
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
    let mut d = harness_of(rows, app);
    d.frames(3);

    d.click(Target::suffix("/(\"row\", 2)")).frames(2);
    d.press(Key::Home).frames(1);
    for _ in 0..4 {
        d.press(Key::ArrowRight);
    }
    let id = d.app().field_ids[&2];
    let cursor = |d: &Driver<App>| {
        egui::TextEdit::load_state(d.ctx(), id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| usize::from(range.primary.index))
    };
    assert_eq!(cursor(&d), Some(4), "precondition: the cursor moved");
    assert_eq!(d.ctx().memory(|m| m.focused()), Some(id));
    let before = d.rect(Target::suffix("/(\"row\", 2)"));

    d.app_mut().rows.rotate_left(1);
    d.frames(3);
    let after = d.rect(Target::suffix("/(\"row\", 2)"));
    assert_eq!(
        after.min.x, before.min.x,
        "precondition: only the vertical position changed"
    );
    assert_ne!(after.min.y, before.min.y, "precondition: the row moved");
    assert_eq!(d.app().field_ids[&2], id, "same id after the move");
    assert_eq!(d.ctx().memory(|m| m.focused()), Some(id), "still focused");
    assert_eq!(cursor(&d), Some(4), "cursor kept");

    d.type_text("!").frames(1);
    assert_eq!(d.app().rows[0].1, "beta! gamma");
}

#[test]
fn a_multi_line_field_grows_with_its_content_up_to_its_cap() {
    let mut d = harness();
    let height = |d: &Driver<App>| d.rect(Target::suffix("/\"notes\"")).height();
    let resting = height(&d);

    let mut heights = Vec::new();
    for lines in 1..=8 {
        d.app_mut().notes = vec!["line"; lines].join("\n");
        d.frames(3);
        heights.push(height(&d));
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
        .size(
            d.ctx(),
            &base_text(),
            &d.shell().style(),
            Some(Width::new(220.0)),
        )
        .y;
    assert!(
        heights[3] <= cap,
        "four rows of text fit the cap: {heights:?}"
    );
    assert!(
        heights[4..].iter().all(|&h| h == cap),
        "the cap is the height of a four-row field, {cap}: {heights:?}"
    );

    d.click(Target::suffix("/\"notes\"")).frames(2);
    d.type_text("tail").frames(1);
    assert!(
        d.app().notes.contains("tail"),
        "a capped field still edits: {:?}",
        d.app().notes
    );
    assert_eq!(height(&d), cap);
}

#[test]
fn typing_enter_in_a_multi_line_field_grows_it() {
    let mut d = harness();
    d.click(Target::suffix("/\"notes\"")).frames(2);
    let start = d.rect(Target::suffix("/\"notes\"")).height();
    for _ in 0..3 {
        d.press(Key::Enter).frames(1);
    }
    assert!(
        d.rect(Target::suffix("/\"notes\"")).height() > start,
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
    b.segmented(
        "mode",
        app.mode,
        [
            (Mode::Day, "Day"),
            (Mode::Week, "Week"),
            (Mode::Month, "Month"),
        ],
    )
    .custom_classes(container.flex().gap(1.0).p(2.0), |cell| match cell {
        Cell::Pressed => tw().px(3.0).py(1.0).bg(ON),
        Cell::Released => tw().px(3.0).py(1.0).bg(OFF),
    })
    .on_change(|app, mode| app.mode = mode);
}

fn picker_harness() -> Driver<Picker> {
    let mut d = support::driver(vec2(300.0, 100.0), base_text(), Picker::default(), picker).build();
    d.frames(3);
    d
}

fn cell_path(mode: Mode) -> String {
    format!("/\"mode\"/{}", mode as usize)
}

/// The modes whose cells are painted pressed.
fn pressed(d: &Driver<Picker>) -> Vec<Mode> {
    [Mode::Day, Mode::Week, Mode::Month]
        .into_iter()
        .filter(|&mode| support::fill_of(d, &cell_path(mode)) == Some(ON))
        .collect()
}

fn choose(d: &mut Driver<Picker>, mode: Mode) {
    d.click(Target::suffix(&cell_path(mode))).frames(2);
}

#[test]
fn a_segmented_control_has_exactly_one_pressed_cell_and_clicks_change_it() {
    let mut d = picker_harness();
    let rects: Vec<_> = [Mode::Day, Mode::Week, Mode::Month]
        .map(|mode| d.rect(Target::suffix(&cell_path(mode))))
        .into();
    assert!(
        rects.windows(2).all(|pair| pair[0].max.x < pair[1].min.x),
        "precondition: three separate cells in a row: {rects:?}"
    );
    assert_eq!(pressed(&d), [Mode::Day]);

    choose(&mut d, Mode::Month);
    assert_eq!(d.app().mode, Mode::Month, "through the lens");
    assert_eq!(pressed(&d), [Mode::Month]);

    d.click(Target::suffix(&cell_path(Mode::Month)));
    assert_eq!(
        pressed(&d),
        [Mode::Month],
        "clicking the chosen cell keeps it"
    );

    choose(&mut d, Mode::Week);
    assert_eq!(pressed(&d), [Mode::Week]);
}

#[test]
fn the_keyboard_chooses_a_cell() {
    let mut d = picker_harness();
    support::drive_past_bar(&mut d);
    d.harness().key_press(Key::Tab);
    d.press(Key::Tab).frames(1);
    d.press(Key::Enter).frames(2);
    assert_eq!(
        pressed(&d),
        [Mode::Week],
        "Tab visits cells in order, Enter chooses"
    );
}

#[test]
fn an_inert_segmented_control_does_not_change() {
    let mut d = picker_harness();
    d.app_mut().inert = true;
    d.frames(3);
    choose(&mut d, Mode::Month);
    assert_eq!(d.app().mode, Mode::Day);
    assert_eq!(pressed(&d), [Mode::Day]);
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
fn painted_text(d: &Driver<Centred>) -> (f32, f32) {
    d.harness()
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
                    let mut d =
                        support::prepared_driver(vec2(300.0, 200.0), base, app, centred, |_, _| {})
                            .pixels_per_point(PixelsPerPoint::new(ppp))
                            .build();
                    d.frames(3);
                    let leaf = d.rect(Target::suffix("/\"field\""));
                    let (top, bottom) = painted_text(&d);
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
        let mut d =
            support::prepared_driver(vec2(300.0, 200.0), base_text(), app, centred, |_, _| {})
                .pixels_per_point(PixelsPerPoint::new(ppp))
                .build();
        d.frames(3);
        let leaf = d.rect(Target::suffix("/\"field\""));
        let (top, _) = painted_text(&d);
        assert!(leaf.height() >= 90.0, "precondition: a tall leaf");
        assert!(
            top - leaf.top() <= 6.0,
            "ppp {ppp}: text starts at the top, {top} in {leaf:?}"
        );
    }
}

#[test]
fn named_primitives_are_found_by_their_accessible_name() {
    let d = harness();
    for name in ["Name", "Notes"] {
        let _ = d.harness().get_by_label(name);
    }
}

#[test]
fn a_bare_field_is_as_tall_as_the_text_it_replaces() {
    let tokens = Tokens::for_theme(Theme::Light);
    let placed = Headless::new(vec2(400.0, 300.0))
        .theme(Theme::Light)
        .prepare(Shadcn::apply_both)
        .base_text(TextStyle::new(tokens.text_role(), tokens.foreground))
        .layout(&(), |b, _| {
            b.div(tw().flex().flex_col().w_px(120.0), |b| {
                b.span("readout", tw(), "1.234");
                b.text_field("framed", "1.234").on_change(|_, _| {});
                b.text_field_with("bare", Field::singleline("1.234").bare())
                    .on_change(|_, _| {});
            });
        });
    let height = |key| placed.at(key).rect().height();
    let (readout, framed, bare) = (height("readout"), height("framed"), height("bare"));
    assert!(
        framed > readout,
        "precondition: the input.height floor is taller than a line ({framed} vs {readout})"
    );
    assert_eq!(bare, readout);
}

fn bare_field_in_a_box(b: &mut Builder<'_, '_, String>, text: &String) {
    b.div_id(
        "box",
        tw().flex().flex_col().w_px(160.0).px(1.5).py(0.5),
        |b| {
            b.text_field_with("bare", Field::singleline(text).bare().named("Bare"))
                .on_change(|app: &mut String, value| *app = value);
        },
    );
}

#[test]
fn a_bare_field_draws_its_widget_in_its_leaf_not_at_the_input_height() {
    let tokens = Tokens::for_theme(Theme::Light);
    let d = support::prepared_driver(
        vec2(400.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        "x".to_owned(),
        bare_field_in_a_box,
        |ctx, _| Shadcn::apply_both(ctx),
    )
    .build();
    let leaf = d.rect(Target::suffix("/\"box\"/\"bare\""));
    let input_height = d
        .shell()
        .style()
        .input
        .sizes
        .get(eggshell_ui::contract::Size::DEFAULT)
        .height;
    assert!(
        leaf.height() < input_height,
        "precondition: the leaf ({}) is shorter than input.height ({input_height})",
        leaf.height()
    );
    let drawn = d.harness().get_by_label("Bare").rect();
    assert_eq!(drawn, leaf, "the field's widget rect is its leaf");
}
