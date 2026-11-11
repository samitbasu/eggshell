//! Scroll containers and the keyboard, through the harness: the keys that
//! scroll, which container they scroll, Tab into content that is scrolled
//! out of view, and the accessibility tree of that content.

use eggshell_ui::scroll::{self, Axis, Focusable, ScrollChain};
use eggshell_ui::testing::{Timeline, record};
use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::accesskit::Role;
use egui::epaint::{RectShape, Shape};
use egui::{Color32, FontFamily, Id, Key, Modifiers, Rect, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, rect_of};

const ROW: f32 = 30.0;
const VIEWPORT: f32 = 100.0;
const LINE: f32 = 18.0;
const SIZE: Vec2 = vec2(500.0, 400.0);
const LIST: &str = "/\"list\"";

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: LINE,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Content {
    /// Rows that paint and sense nothing.
    Painted,
    /// A button per row.
    Buttons,
    /// A button per row, in a container that is always a tab stop and is
    /// announced as a log.
    Transcript,
    /// A text field, then painted rows.
    Field,
    /// Two containers side by side, of painted rows.
    Siblings,
    /// A container with painted rows around another one.
    Nested,
}

struct App {
    content: Content,
    rows: u32,
    axis: Axis,
    text: String,
    drawn: Vec<u32>,
    pressed: Vec<u32>,
    focusable: Focusable,
    border: Option<Color32>,
    /// The container sets a `focus:` look of its own.
    explicit_focus: bool,
}

impl App {
    fn new(content: Content) -> Self {
        Self {
            content,
            rows: 20,
            axis: Axis::Y,
            text: String::new(),
            drawn: Vec::new(),
            pressed: Vec::new(),
            focusable: Focusable::Auto,
            border: None,
            explicit_focus: false,
        }
    }
}

fn strip(axis: Axis) -> eggshell_ui::Tw {
    match axis {
        Axis::X => tw().flex(),
        _ => tw().flex().flex_col(),
    }
}

fn painted_rows(s: &mut Builder<'_, '_, App>, rows: u32, axis: Axis) {
    for i in 0..rows {
        let size = match axis {
            Axis::X => tw().w_px(80.0).h_full().shrink_0(),
            _ => tw().h_px(ROW).w_full().shrink_0(),
        };
        s.div_id(i, size, |_| {})
            .draw(move |app: &mut App, _| app.drawn.push(i));
    }
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    fn list<'a, 'f>(
        b: &'a mut Builder<'_, 'f, App>,
        key: &'static str,
        width: f32,
        app: &App,
    ) -> ScrollChain<'a, 'f, App> {
        let mut classes = tw().w_px(width).h_px(VIEWPORT);
        if let Some(color) = app.border {
            classes = classes.border(1.0).border_color(color);
        }
        if app.explicit_focus {
            classes = classes.focus(|p| p.bg(Color32::LIGHT_RED));
        }
        b.scroll(key, classes)
            .axis(app.axis)
            .focusable(app.focusable)
    }
    let axis = app.axis;
    b.div(tw().flex(), |b| match app.content {
        Content::Painted => {
            list(b, "list", 200.0, app).children(|s| {
                s.div(strip(axis), |s| painted_rows(s, app.rows, axis));
            });
        }
        Content::Buttons | Content::Transcript => {
            let chain = list(b, "list", 200.0, app);
            let chain = if app.content == Content::Transcript {
                chain
                    .focusable(Focusable::Always)
                    .role(Role::Log)
                    .named("Conversation")
            } else {
                chain
            };
            chain.children(|s| {
                s.div(strip(axis), |s| {
                    for i in 0..app.rows {
                        s.button(i, format!("Row {i}"))
                            .on_click(move |app: &mut App| app.pressed.push(i));
                    }
                });
            });
        }
        Content::Field => {
            list(b, "list", 200.0, app).children(|s| {
                s.div(strip(axis), |s| {
                    s.text_field("field", app.text.clone())
                        .named("Message")
                        .on_change(|app: &mut App, text| app.text = text);
                    painted_rows(s, app.rows, axis);
                });
            });
        }
        Content::Siblings => {
            for key in ["a", "b"] {
                list(b, key, 200.0, app).children(|s| {
                    s.div(strip(axis), |s| painted_rows(s, app.rows, axis));
                });
            }
        }
        Content::Nested => {
            list(b, "outer", 220.0, app).children(|s| {
                s.div(strip(axis), |s| {
                    s.scroll("inner", tw().w_px(160.0).h_px(60.0))
                        .axis(axis)
                        .children(|s| {
                            s.div(strip(axis), |s| painted_rows(s, 10, axis));
                        });
                    s.div_id("tail", tw().h_px(400.0), |_| {});
                });
            });
        }
    });
}

type H = Harness<'static, Rig<App>>;

fn harness_with(app: App) -> H {
    let mut h = support::harness(SIZE, base(), app, view);
    h.run_steps(3);
    h
}

fn harness(content: Content) -> H {
    harness_with(App::new(content))
}

fn id(h: &H, path: &str) -> Id {
    support::id_of(h, path)
}

fn viewport(h: &H, path: &str) -> scroll::Viewport {
    scroll::viewport(&h.ctx, id(h, path)).expect("laid out")
}

fn offset(h: &H, path: &str) -> Vec2 {
    viewport(h, path).offset()
}

fn press(h: &mut H, key: Key) {
    h.key_press(key);
    h.run_steps(3);
}

fn press_with(h: &mut H, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
}

fn tab_to(h: &mut H, path: &str) {
    for _ in 0..40 {
        if support::focused_path(h).as_deref() == Some(path) {
            return;
        }
        press(h, Key::Tab);
    }
    panic!("Tab never reached {path}");
}

/// A page of the container at `path`: its viewport less one line.
fn page(h: &H, path: &str) -> f32 {
    viewport(h, path).rect.height() - LINE
}

fn row_path(i: u32) -> String {
    format!("{LIST}/{i}")
}

#[test]
fn each_key_scrolls_the_focused_container_by_its_amount() {
    let mut h = harness(Content::Painted);
    tab_to(&mut h, LIST);
    let page = page(&h, LIST);
    let max = viewport(&h, LIST).content.height() - viewport(&h, LIST).rect.height();
    assert!(
        max > 3.0 * page,
        "precondition: it scrolls a long way: {max}"
    );
    let steps: [(&str, Key, Modifiers, f32); 9] = [
        ("ArrowDown", Key::ArrowDown, Modifiers::NONE, LINE),
        ("ArrowUp", Key::ArrowUp, Modifiers::NONE, 0.0),
        ("PageDown", Key::PageDown, Modifiers::NONE, page),
        ("PageDown again", Key::PageDown, Modifiers::NONE, 2.0 * page),
        ("PageUp", Key::PageUp, Modifiers::NONE, page),
        ("Space", Key::Space, Modifiers::NONE, 2.0 * page),
        ("Shift+Space", Key::Space, Modifiers::SHIFT, page),
        ("End", Key::End, Modifiers::NONE, max),
        ("Home", Key::Home, Modifiers::NONE, 0.0),
    ];
    for (name, key, modifiers, expected) in steps {
        press_with(&mut h, modifiers, key);
        assert_eq!(offset(&h, LIST), vec2(0.0, expected), "{name}");
    }
}

#[test]
fn the_arrows_scroll_a_horizontal_container_sideways_only() {
    let mut h = harness_with(App {
        axis: Axis::X,
        ..App::new(Content::Painted)
    });
    tab_to(&mut h, LIST);
    press(&mut h, Key::ArrowRight);
    assert_eq!(offset(&h, LIST), vec2(LINE, 0.0), "right");
    press(&mut h, Key::ArrowDown);
    assert_eq!(offset(&h, LIST), vec2(LINE, 0.0), "down does not move it");
    press(&mut h, Key::ArrowLeft);
    assert_eq!(offset(&h, LIST), Vec2::ZERO, "left");
    press(&mut h, Key::End);
    let max = viewport(&h, LIST).content.width() - viewport(&h, LIST).rect.width();
    assert_eq!(offset(&h, LIST), vec2(max, 0.0), "End");
}

#[test]
fn a_text_field_in_a_container_keeps_its_keys() {
    let mut h = harness(Content::Field);
    press(&mut h, Key::Tab);
    assert!(
        h.ctx.text_edit_focused(),
        "precondition: the field is focused"
    );
    assert!(
        viewport(&h, LIST).content.height() > 2.0 * VIEWPORT,
        "precondition: the container scrolls"
    );
    h.event(egui::Event::Text("hello".to_owned()));
    h.run_steps(3);
    for key in [
        Key::ArrowDown,
        Key::ArrowLeft,
        Key::Home,
        Key::End,
        Key::PageDown,
        Key::Space,
    ] {
        press(&mut h, key);
        assert_eq!(offset(&h, LIST), Vec2::ZERO, "{key:?} stays with the field");
    }
    assert!(h.ctx.text_edit_focused(), "and the field kept the focus");
    assert!(
        h.state().app.text.starts_with("hello"),
        "{:?}",
        h.state().app.text
    );
}

#[test]
fn the_focused_container_scrolls_not_the_one_under_the_pointer() {
    let mut h = harness(Content::Siblings);
    tab_to(&mut h, "/\"a\"");
    let over_b = viewport(&h, "/\"b\"").rect.center();
    h.hover_at(over_b);
    h.run_steps(2);
    press(&mut h, Key::PageDown);
    let page = page(&h, "/\"a\"");
    assert_eq!(offset(&h, "/\"a\""), vec2(0.0, page), "the focused one");
    assert_eq!(offset(&h, "/\"b\""), Vec2::ZERO, "not the hovered one");
}

#[test]
fn a_container_last_pressed_in_takes_the_keys_when_nothing_is_focused() {
    let mut h = harness(Content::Siblings);
    let in_b = viewport(&h, "/\"b\"").rect.center();
    let away = viewport(&h, "/\"a\"").rect.center();
    support::press_and_release(&mut h, in_b);
    h.hover_at(away);
    h.run_steps(2);
    press(&mut h, Key::ArrowDown);
    assert_eq!(offset(&h, "/\"b\""), vec2(0.0, LINE), "the one pressed in");
    assert_eq!(offset(&h, "/\"a\""), Vec2::ZERO);
}

#[test]
fn the_innermost_container_holding_the_focus_scrolls_and_the_keys_do_not_chain() {
    let mut h = harness(Content::Nested);
    tab_to(&mut h, "/\"outer\"/\"inner\"");
    let inner = "/\"outer\"/\"inner\"";
    let max = viewport(&h, inner).content.height() - viewport(&h, inner).rect.height();
    assert!(max > 0.0, "precondition: the inner container scrolls");
    press(&mut h, Key::ArrowDown);
    assert_eq!(offset(&h, inner), vec2(0.0, LINE), "the inner one scrolls");
    assert_eq!(
        offset(&h, "/\"outer\""),
        Vec2::ZERO,
        "the outer one does not"
    );
    press(&mut h, Key::End);
    let end = offset(&h, inner).y;
    // egui's own clamp of a nested container's offset ends a few points short
    // of the content's: `scroll_by` does the same (see the report on #91).
    assert!(end > max - 6.0, "precondition: it went to the end: {end}");
    for key in [Key::ArrowDown, Key::PageDown, Key::End, Key::Space] {
        press(&mut h, key);
        assert!(
            offset(&h, inner).y <= max,
            "{key:?} at the end stays within the content"
        );
        assert_eq!(
            offset(&h, "/\"outer\""),
            Vec2::ZERO,
            "{key:?} at the end does not chain"
        );
    }
    press(&mut h, Key::Home);
    press(&mut h, Key::ArrowUp);
    assert!(
        offset(&h, inner).y < 2.0,
        "back at the start: {:?}",
        offset(&h, inner)
    );
    assert_eq!(offset(&h, "/\"outer\""), Vec2::ZERO, "nor at the start");
}

fn focus_rings(h: &H) -> Vec<Rect> {
    let ring = eggshell_ui::contract::Style::of(&h.ctx).focus.color;
    support::shapes(h)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(RectShape { rect, stroke, .. }) if stroke.color == ring => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn a_container_without_focusable_content_is_a_tab_stop_with_a_focus_ring() {
    let mut h = harness(Content::Siblings);
    assert!(focus_rings(&h).is_empty(), "precondition: no ring yet");
    press(&mut h, Key::Tab);
    assert_eq!(support::focused_path(&h).as_deref(), Some("/\"a\""));
    let rings = focus_rings(&h);
    assert!(
        rings
            .iter()
            .any(|ring| ring.contains_rect(rect_of(&h, "/\"a\""))),
        "a ring around the container: {rings:?}"
    );
    press(&mut h, Key::Tab);
    assert_eq!(
        support::focused_path(&h).as_deref(),
        Some("/\"b\""),
        "and Tab goes on to the next one"
    );
}

#[test]
fn a_container_with_focusable_content_is_not_a_tab_stop() {
    for content in [Content::Buttons, Content::Field] {
        let mut h = harness(content);
        let mut visited = Vec::new();
        for _ in 0..12 {
            press(&mut h, Key::Tab);
            visited.push(support::focused_path(&h));
        }
        assert!(
            visited.iter().flatten().count() > 3,
            "precondition: Tab visits the content: {visited:?}"
        );
        assert!(
            !visited.contains(&Some(LIST.to_owned())),
            "the container is no stop: {visited:?}"
        );
    }
}

#[test]
fn tab_from_the_last_visible_control_reaches_a_culled_one_and_scrolls_it_into_view() {
    let mut h = harness(Content::Buttons);
    let view = viewport(&h, LIST).rect;
    let last_visible = (0..20)
        .rfind(|&i| rect_of(&h, &row_path(i)).intersects(view))
        .expect("some row is visible");
    let next = last_visible + 1;
    assert!(
        rect_of(&h, &row_path(next)).min.y >= view.max.y,
        "precondition: row {next} is below the viewport"
    );
    assert!(h.state().app.drawn.is_empty());
    tab_to(&mut h, &row_path(last_visible));
    press(&mut h, Key::Tab);
    assert_eq!(
        support::focused_path(&h),
        Some(row_path(next)),
        "Tab reached the culled row"
    );
    let view = viewport(&h, LIST);
    assert!(view.offset().y > 0.0, "the list scrolled");
    assert!(
        view.rect.contains_rect(rect_of(&h, &row_path(next))),
        "and shows row {next}: {:?} in {:?}",
        rect_of(&h, &row_path(next)),
        view.rect
    );
}

#[test]
fn tab_walks_a_long_list_row_by_row_and_back() {
    let mut h = harness(Content::Buttons);
    for i in 0..12 {
        tab_to(&mut h, &row_path(i));
        let view = viewport(&h, LIST).rect;
        assert!(
            view.contains_rect(rect_of(&h, &row_path(i))),
            "row {i} is shown when it has the focus"
        );
    }
    for i in (4..11).rev() {
        press_with(&mut h, Modifiers::SHIFT, Key::Tab);
        assert_eq!(support::focused_path(&h), Some(row_path(i)), "back to {i}");
    }
}

#[test]
fn the_accessibility_tree_has_a_scrolled_out_button_with_its_role_and_name() {
    let h = harness(Content::Buttons);
    let view = viewport(&h, LIST).rect;
    assert!(
        !rect_of(&h, &row_path(15)).intersects(view),
        "precondition: row 15 is scrolled out"
    );
    let button = h.get_by_role_and_label(Role::Button, "Row 15");
    assert!(
        !button.rect().intersects(view) || button.rect().height() > 0.0,
        "it is at its real place"
    );
}

#[test]
fn the_draw_closures_of_culled_nodes_that_are_not_focusable_do_not_run() {
    let mut h = harness(Content::Painted);
    h.state_mut().app.drawn.clear();
    h.step();
    let mut drawn = h.state().app.drawn.clone();
    drawn.sort_unstable();
    assert_eq!(
        drawn,
        vec![0, 1, 2, 3],
        "only the rows that reach the viewport"
    );
}

#[test]
fn registering_a_culled_button_paints_nothing_of_it() {
    let h = harness(Content::Buttons);
    let hidden = rect_of(&h, &row_path(15));
    assert!(
        !hidden.intersects(viewport(&h, LIST).rect),
        "precondition: row 15 is scrolled out"
    );
    let painted_there = support::shapes(&h).into_iter().any(|shape| match shape {
        Shape::Rect(rect) => {
            rect.rect.intersects(hidden)
                && (rect.fill != Color32::TRANSPARENT || !rect.stroke.is_empty())
        }
        Shape::Text(text) => text.visual_bounding_rect().intersects(hidden),
        _ => false,
    });
    assert!(!painted_there, "nothing is painted where row 15 is");
}

#[test]
fn keyboard_scrolling_is_continuous() {
    let mut h = harness(Content::Painted);
    tab_to(&mut h, LIST);
    record(&h.ctx);
    h.run_steps(2);
    for key in [
        Key::ArrowDown,
        Key::PageDown,
        Key::PageDown,
        Key::ArrowUp,
        Key::End,
        Key::PageUp,
        Key::Home,
    ] {
        press(&mut h, key);
    }
    assert_eq!(
        offset(&h, LIST),
        Vec2::ZERO,
        "precondition: it went there and back"
    );
    Timeline::recorded(&h.ctx).assert_continuous();
}

#[test]
fn a_key_that_a_focused_button_uses_is_not_taken_for_scrolling() {
    let mut h = harness(Content::Buttons);
    tab_to(&mut h, &row_path(0));
    press(&mut h, Key::Space);
    assert_eq!(h.state().app.pressed, vec![0], "Space presses the button");
    assert_eq!(offset(&h, LIST), Vec2::ZERO, "and does not scroll");
}

#[test]
fn an_always_focusable_container_is_a_stop_before_its_controls_and_tab_enters_it() {
    let mut h = harness(Content::Transcript);
    press(&mut h, Key::Tab);
    assert_eq!(
        support::focused_path(&h).as_deref(),
        Some(LIST),
        "the container comes first"
    );
    press(&mut h, Key::Tab);
    assert_eq!(
        support::focused_path(&h),
        Some(row_path(0)),
        "and Tab moves into its first control"
    );
}

#[test]
fn a_focused_always_container_scrolls_with_the_keys_and_a_focused_child_has_them_first() {
    let mut h = harness(Content::Transcript);
    tab_to(&mut h, LIST);
    press(&mut h, Key::PageDown);
    assert_eq!(offset(&h, LIST), vec2(0.0, self::page(&h, LIST)));
    press(&mut h, Key::Home);
    assert_eq!(offset(&h, LIST), Vec2::ZERO);
    press(&mut h, Key::Tab);
    assert_eq!(support::focused_path(&h), Some(row_path(0)), "precondition");
    press(&mut h, Key::Space);
    assert_eq!(h.state().app.pressed, vec![0], "the button takes Space");
    assert_eq!(offset(&h, LIST), Vec2::ZERO);
}

#[test]
fn the_accessibility_tree_has_the_containers_role_and_name() {
    let h = harness(Content::Transcript);
    let log = h.get_by_role_and_label(Role::Log, "Conversation");
    assert!(log.rect().height() > 0.0);
}

#[test]
fn auto_is_no_stop_for_a_container_of_controls_and_no_role_unless_asked() {
    let mut h = harness(Content::Buttons);
    for _ in 0..3 {
        press(&mut h, Key::Tab);
        assert_ne!(support::focused_path(&h).as_deref(), Some(LIST));
    }
    assert!(h.query_by_role(Role::Log).is_none());
}

#[test]
fn a_focused_buttons_ring_is_still_outside() {
    let mut h = harness(Content::Buttons);
    press(&mut h, Key::Tab);
    let row = rect_of(&h, &row_path(0));
    let ring = support::ring_at(&h, row).expect("a ring outside the button");
    assert_eq!(ring.stroke_kind, egui::StrokeKind::Outside);
}

#[test]
fn a_click_in_a_container_shows_no_ring_only_the_keyboard_does() {
    let mut h = harness(Content::Siblings);
    let at = viewport(&h, "/\"a\"").rect.center();
    support::press_and_release(&mut h, at);
    h.run_steps(3);
    assert_eq!(support::focused_path(&h), None, "a click does not focus it");
    assert!(focus_rings(&h).is_empty(), "and shows no ring");
}

const FRAME: &str = "/\"list\"";

fn framed(focusable: Focusable) -> H {
    harness_with(App {
        focusable,
        ..App::new(Content::Buttons)
    })
}

fn focus_color(h: &H) -> Color32 {
    eggshell_ui::contract::Style::of(&h.ctx).focus.color
}

fn frame_width(h: &H) -> f32 {
    eggshell_ui::contract::Style::of(&h.ctx).focus.width
}

#[test]
fn a_focusable_containers_content_and_gutter_sit_inside_a_frame_that_does_not_shift_on_focus() {
    let plain = framed(Focusable::Auto);
    let mut framed = framed(Focusable::Always);
    let width = frame_width(&framed);
    let (plain_view, framed_view) = (viewport(&plain, FRAME), viewport(&framed, FRAME));
    assert_eq!(
        rect_of(&plain, FRAME),
        rect_of(&framed, FRAME),
        "precondition: the same border box"
    );
    assert_eq!(
        framed_view.rect,
        plain_view.rect.shrink(width),
        "the viewport (and so the gutter beside it) is inset by the frame"
    );
    assert_eq!(
        framed_view.rect.width() + 10.0 + 2.0 * width,
        plain_view.rect.width() + 10.0,
        "the gutter is inside the frame"
    );
    let at_rest = framed.state().nodes.clone();
    press(&mut framed, Key::Tab);
    assert_eq!(
        support::focused_path(&framed).as_deref(),
        Some(FRAME),
        "precondition: the container is focused"
    );
    assert_eq!(
        framed.state().nodes,
        at_rest,
        "no layout rect moves on focus"
    );
    assert_eq!(viewport(&framed, FRAME), framed_view);
}

fn frame_shapes(h: &H, color: Color32) -> Vec<RectShape> {
    support::shapes(h)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect)
                if rect.stroke.color == color
                    && rect.stroke.width > 0.0
                    && support::same(rect.rect, rect_of(h, FRAME)) =>
            {
                Some(rect.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_frame_takes_the_ring_colour_on_focus_and_is_clear_at_rest() {
    let mut h = framed(Focusable::Always);
    let ring = focus_color(&h);
    assert!(frame_shapes(&h, ring).is_empty(), "transparent at rest");
    press(&mut h, Key::Tab);
    h.run_steps(20);
    let frames = frame_shapes(&h, ring);
    assert_eq!(frames.len(), 1, "precondition: the frame is painted");
    assert_eq!(frames[0].stroke_kind, egui::StrokeKind::Inside);
    assert_eq!(frames[0].stroke.width, frame_width(&h));
    assert_eq!(frames[0].rect, rect_of(&h, FRAME));
    press(&mut h, Key::Tab);
    h.run_steps(20);
    assert_eq!(support::focused_path(&h), Some(row_path(0)), "precondition");
    assert!(frame_shapes(&h, ring).is_empty(), "and clear again");
}

#[test]
fn the_frame_at_rest_is_the_containers_own_border_colour_and_its_wider_width_wins() {
    let own = Color32::from_rgb(200, 30, 30);
    let mut h = harness_with(App {
        focusable: Focusable::Always,
        border: Some(own),
        ..App::new(Content::Buttons)
    });
    let rest = frame_shapes(&h, own);
    assert_eq!(rest.len(), 1, "precondition: the border is painted");
    assert_eq!(rest[0].stroke.width, frame_width(&h).max(1.0));
    press(&mut h, Key::Tab);
    h.run_steps(20);
    let ring = focus_color(&h);
    assert_eq!(frame_shapes(&h, ring).len(), 1, "the ring colour on focus");
}

#[test]
fn nothing_the_focus_look_paints_reaches_the_viewport_or_the_gutter() {
    let mut h = framed(Focusable::Always);
    press(&mut h, Key::Tab);
    h.run_steps(20);
    let frames = frame_shapes(&h, focus_color(&h));
    assert_eq!(frames.len(), 1, "precondition: the frame is painted");
    let frame = &frames[0];
    let inside = frame.rect.shrink(frame.stroke.width);
    let view = viewport(&h, FRAME).rect;
    assert!(
        inside.contains_rect(Rect::from_min_max(view.min, view.max + vec2(10.0, 0.0))),
        "the viewport and its gutter {view:?} are inside {inside:?}"
    );
    assert_eq!(
        support::shapes(&h)
            .into_iter()
            .filter(|shape| matches!(shape, Shape::Rect(r) if r.stroke.color == focus_color(&h)))
            .count(),
        1,
        "and the frame is all the focus look paints"
    );
}

#[test]
fn a_container_with_a_focus_look_of_its_own_gets_no_frame() {
    let plain = harness_with(App {
        explicit_focus: true,
        focusable: Focusable::Always,
        ..App::new(Content::Buttons)
    });
    let reference = harness(Content::Buttons);
    assert_eq!(
        viewport(&plain, FRAME).rect,
        viewport(&reference, FRAME).rect,
        "nothing is reserved"
    );
}

#[test]
fn focusing_a_container_in_and_out_is_continuous() {
    let mut h = framed(Focusable::Always);
    record(&h.ctx);
    h.run_steps(2);
    press(&mut h, Key::Tab);
    h.run_steps(10);
    press(&mut h, Key::Tab);
    h.run_steps(10);
    Timeline::recorded(&h.ctx).assert_continuous();
}

#[test]
fn whether_an_auto_container_is_a_stop_is_the_same_every_frame() {
    for content in [Content::Painted, Content::Buttons, Content::Field] {
        let mut h = harness(content);
        let first = viewport(&h, FRAME).rect;
        for _ in 0..5 {
            h.run_steps(2);
            assert_eq!(viewport(&h, FRAME).rect, first, "the frame never flickers");
        }
    }
}
