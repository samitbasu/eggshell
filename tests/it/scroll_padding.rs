//! `Tw::scroll_p*`, Tailwind's scroll padding, through build, layout and the
//! keys: scrolling into view keeps the padding from the edges, and the wheel
//! is unaffected.

use eggshell_ui::vlist::ListNav;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::Role;
use egui::{Color32, Event, FontFamily, Key, Modifiers, MouseWheelUnit, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, rect_of};

const ROW: f32 = 30.0;
const HEIGHT: f32 = 100.0;
const ROWS: u32 = 12;

struct App {
    padding: f32,
    picked: Option<u32>,
}

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.text_field("query", "")
        .named("Query")
        .on_change(|_: &mut App, _| {});
    let field = b.id_of("query");
    let rows: Vec<u32> = (0..ROWS).collect();
    b.virtual_list(
        "list",
        tw().w_px(200.0).h_px(HEIGHT).scroll_py(app.padding),
        rows,
    )
    .nav(ListNav::Listbox)
    .keys_from(field)
    .named("Rows")
    .row_name(|row| row.to_string())
    .fixed_px(ROW)
    .on_activate(|app: &mut App, row| app.picked = Some(row))
    .items(|s, row| {
        s.div_id(row, tw().w_full().h_px(ROW), |s| {
            s.text(tw(), row.to_string())
        });
    });
}

type H = Harness<'static, Rig<App>>;

fn harness(padding: f32) -> H {
    let mut harness = support::harness(
        vec2(300.0, 300.0),
        base(),
        App {
            padding,
            picked: None,
        },
        view,
    );
    harness.run_steps(4);
    harness
}

fn list(h: &H) -> egui::Rect {
    rect_of(h, "/\"list\"")
}

fn row(h: &H, row: u32) -> egui::Rect {
    h.get_by_role_and_label(Role::ListBoxOption, &row.to_string())
        .rect()
}

fn focus_field(h: &mut H) {
    let at = rect_of(h, "/\"query\"").center();
    h.hover_at(at);
    h.step();
    support::press_and_release(h, at);
    h.run_steps(4);
}

fn press(h: &mut H, key: Key) {
    h.key_press(key);
    h.run_steps(6);
}

#[test]
fn arrowing_down_keeps_the_padding_below_the_highlighted_row() {
    let mut h = harness(1.0);
    focus_field(&mut h);
    for _ in 0..5 {
        press(&mut h, Key::ArrowDown);
    }
    let (list, shown) = (list(&h), row(&h, 4));
    assert!(
        shown.max.y <= list.max.y - 4.0 + 0.5,
        "scrolled to show row 4, 4 pt clear of the bottom: {shown:?} in {list:?}"
    );
    assert!(
        (shown.max.y - (list.max.y - 4.0)).abs() < 1.0 || shown.min.y >= list.min.y,
        "it moved no more than it had to"
    );
}

/// How far below the list's top edge the highlighted row's top is after
/// each ArrowUp from the last row to the first.
fn gaps_above_while_arrowing_up(padding: f32) -> Vec<f32> {
    let mut h = harness(padding);
    focus_field(&mut h);
    for _ in 0..ROWS {
        press(&mut h, Key::ArrowDown);
    }
    (0..ROWS - 1)
        .rev()
        .map(|highlighted| {
            press(&mut h, Key::ArrowUp);
            row(&h, highlighted).min.y - list(&h).min.y
        })
        .collect()
}

#[test]
fn arrowing_up_keeps_the_padding_below_the_top_edge() {
    let gaps = gaps_above_while_arrowing_up(1.0);
    let (first, scrolled) = gaps.split_last().expect("a gap for each row");
    assert!(
        scrolled.iter().all(|gap| *gap >= 4.0 - 0.5),
        "no highlighted row comes nearer the top than 4 pt: {gaps:?}"
    );
    assert_eq!(*first, 0.0, "but the first row is where the content starts");
    assert!(
        gaps.iter().any(|gap| (gap - 4.0).abs() < 1.0),
        "a row scrolled to from below sits exactly 4 pt under the top: {gaps:?}"
    );
}

#[test]
fn without_padding_a_row_scrolled_to_sits_flush_with_the_top_edge() {
    let gaps = gaps_above_while_arrowing_up(0.0);
    assert!(
        gaps.iter().any(|gap| gap.abs() < 1.0),
        "flush with the top: {gaps:?}"
    );
}

#[test]
fn the_wheel_ignores_the_padding() {
    let mut h = harness(1.0);
    let at = list(&h).center();
    h.hover_at(at);
    h.step();
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -ROW),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    h.run_steps(12);
    let id = support::id_of(&h, "/\"list\"");
    let by = support::shell(&h)
        .viewport(id)
        .map_or(f32::NAN, |viewport| viewport.offset().y);
    assert!(
        (by - ROW).abs() < 1.0,
        "scrolled by exactly the wheel's delta: {by}"
    );
}
