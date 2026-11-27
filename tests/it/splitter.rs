//! Splitters through kittest's real pointer and key events: the drag, the
//! keys, the reset, the accessibility tree and how layout squeezes a pane.

use eggshell_ui::prelude::*;
use std::time::Duration;

use eggshell_ui::owned::splitter::PaneSize;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, Tokens};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::epaint::Shape;
use egui::{Color32, CursorIcon, Key, Modifiers, Pos2, Role, Theme, Vec2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path, press_and_release};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/split_panes.rs"]
mod split_panes;

const WINDOW: Vec2 = vec2(900.0, 600.0);
const EDITOR: Color32 = Color32::from_rgb(10, 20, 30);

struct App {
    sidebar: PaneSize,
    inspector: PaneSize,
    terminal: PaneSize,
    resizes: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            sidebar: PaneSize::new(240.0),
            inspector: PaneSize::new(200.0),
            terminal: PaneSize::new(150.0),
            resizes: 0,
        }
    }
}

/// ```html
/// <div class="flex size-full">
///   <div style="width: 240px"/> <splitter/>
///   <div class="flex flex-col flex-1"> <div class="flex-1"/> <splitter/> <div style="height: 150px"/> </div>
///   <splitter/> <div style="width: 200px"/>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("frame", tw().flex().size_full(), |b| {
        b.div_id("sidebar", tw().w_px(app.sidebar.into()), |_| {});
        b.splitter("left")
            .before(app.sidebar)
            .range(160.0..=480.0)
            .reset_to(240.0)
            .named("Sidebar")
            .on_resize(|app: &mut App, size| {
                app.sidebar = size;
                app.resizes += 1;
            });
        b.div_id(
            "editor",
            tw().flex()
                .flex_col()
                .flex_1()
                .bg(EDITOR)
                .animate_layout(Duration::from_millis(200)),
            |b| {
                b.div_id("code", tw().flex_1(), |_| {});
                b.splitter("bottom")
                    .after(app.terminal)
                    .range(..=400.0)
                    .named("Terminal")
                    .on_resize(|app: &mut App, size| app.terminal = size);
                b.div_id("terminal", tw().h_px(app.terminal.into()), |_| {});
            },
        );
        b.splitter("right")
            .after(app.inspector)
            .range(120.0..)
            .named("Inspector")
            .on_resize(|app: &mut App, size| app.inspector = size);
        b.div_id("inspector", tw().w_px(app.inspector.into()), |_| {});
    });
}

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(4);
    d
}

const LEFT: &str = "/\"frame\"/\"left\"";
const RIGHT: &str = "/\"frame\"/\"right\"";
const BOTTOM: &str = "/\"frame\"/\"editor\"/\"bottom\"";
const SIDEBAR: &str = "/\"frame\"/\"sidebar\"";
const INSPECTOR: &str = "/\"frame\"/\"inspector\"";
const TERMINAL: &str = "/\"frame\"/\"editor\"/\"terminal\"";
const EDITOR_PATH: &str = "/\"frame\"/\"editor\"";

fn sidebar(d: &Driver<App>) -> f32 {
    d.app().sidebar.into()
}

fn inspector(d: &Driver<App>) -> f32 {
    d.app().inspector.into()
}

fn terminal(d: &Driver<App>) -> f32 {
    d.app().terminal.into()
}

fn centre(d: &Driver<App>, path: &str) -> Pos2 {
    d.rect(Target::suffix(path)).center()
}

/// Presses at `from` and moves the pointer through `to`, one frame a point,
/// without releasing.
fn press_and_move(d: &mut Driver<App>, from: Pos2, to: &[Pos2]) {
    d.hover(from);
    d.harness().drag_at(from);
    d.frames(1);
    for &at in to {
        d.harness().hover_at(at);
        d.frames(1);
    }
}

fn release(d: &mut Driver<App>, at: Pos2) {
    d.harness().drop_at(at);
    d.frames(3);
}

fn tab_to(d: &mut Driver<App>, path: &str) {
    for _ in 0..12 {
        if focused_path(d).as_deref() == Some(path) {
            return;
        }
        d.press(Key::Tab).frames(1);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(d));
}

fn press(d: &mut Driver<App>, modifiers: Modifiers, key: Key) {
    d.harness().key_press_modifiers(modifiers, key);
    d.frames(3);
}

mod pointer {
    use super::*;

    #[test]
    fn dragging_a_splitter_sizes_the_pane_before_it() {
        let mut d = harness();
        assert_eq!(
            d.rect(Target::suffix(SIDEBAR)).width(),
            240.0,
            "precondition"
        );
        let from = centre(&d, LEFT);
        d.drag(from, from + vec2(40.0, 0.0));
        assert_eq!(sidebar(&d), 280.0);
        assert_eq!(d.rect(Target::suffix(SIDEBAR)).width(), 280.0);
        assert_eq!(
            d.rect(Target::suffix(LEFT)).center().x,
            from.x + 40.0,
            "the handle follows"
        );
    }

    #[test]
    fn a_splitter_after_its_pane_grows_it_towards_the_start() {
        let mut d = harness();
        let from = centre(&d, RIGHT);
        d.drag(from, from - vec2(50.0, 0.0));
        assert_eq!(inspector(&d), 250.0);
        assert_eq!(d.rect(Target::suffix(INSPECTOR)).width(), 250.0);
    }

    #[test]
    fn a_splitter_in_a_column_moves_vertically() {
        let mut d = harness();
        let from = centre(&d, BOTTOM);
        d.drag(from, from - vec2(0.0, 30.0));
        assert_eq!(terminal(&d), 180.0);
        assert_eq!(d.rect(Target::suffix(TERMINAL)).height(), 180.0);
    }

    #[test]
    fn a_drag_past_the_range_clamps_and_coming_back_does_not_drift() {
        let mut d = harness();
        let from = centre(&d, LEFT);
        let far = from + vec2(600.0, 200.0);
        press_and_move(&mut d, from, &[far]);
        assert!(
            !d.rect(Target::suffix(LEFT)).contains(far),
            "precondition: the pointer left the handle"
        );
        assert_eq!(sidebar(&d), 480.0, "held at the max");
        let back = from + vec2(-500.0, 0.0);
        d.harness().hover_at(back);
        d.frames(1);
        assert_eq!(sidebar(&d), 160.0, "held at the min");
        let near = from + vec2(20.0, 0.0);
        d.harness().hover_at(near);
        d.frames(1);
        release(&mut d, near);
        assert_eq!(sidebar(&d), 260.0, "measured from the press");
    }

    #[test]
    fn a_double_click_resets_the_pane() {
        let mut d = harness();
        let from = centre(&d, LEFT);
        d.drag(from, from + vec2(100.0, 0.0));
        assert_eq!(sidebar(&d), 340.0, "precondition");
        let at = centre(&d, LEFT);
        d.hover(at);
        for _ in 0..2 {
            press_and_release(&mut d, at);
        }
        d.frames(2);
        assert_eq!(sidebar(&d), 240.0);
    }

    #[test]
    fn a_click_without_a_move_reports_nothing() {
        let mut d = harness();
        let at = centre(&d, LEFT);
        d.hover(at);
        press_and_release(&mut d, at);
        d.frames(2);
        assert_eq!(d.app().resizes, 0);
    }

    #[test]
    fn the_pointer_shows_a_resize_cursor_over_the_handle() {
        let mut d = harness();
        let left = centre(&d, LEFT);
        d.hover(left);
        assert_eq!(
            d.harness().output().platform_output.cursor_icon,
            CursorIcon::ResizeColumn
        );
        let bottom = centre(&d, BOTTOM);
        d.hover(bottom);
        assert_eq!(
            d.harness().output().platform_output.cursor_icon,
            CursorIcon::ResizeRow
        );
    }

    /// The editor slides when its place changes, but a drag moves it smoothly
    /// already, so it follows the handle frame by frame.
    #[test]
    fn a_drag_starts_no_slide_on_a_neighbour_that_animates_its_layout() {
        let mut d = harness();
        let from = centre(&d, LEFT);
        let steps: Vec<Pos2> = (1..=4).map(|i| from + vec2(20.0 * i as f32, 0.0)).collect();
        press_and_move(&mut d, from, &steps);
        d.frames(1);
        let laid = d.rect(Target::suffix(EDITOR_PATH));
        assert_ne!(laid.min.x, 241.0 + 6.0, "precondition: the editor moved");
        assert_eq!(painted_left(&d, EDITOR), laid.min.x);
        release(&mut d, *steps.last().expect("steps"));
    }

    fn painted_left(d: &Driver<App>, fill: Color32) -> f32 {
        support::shapes(d)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Rect(rect) if rect.fill == fill => Some(rect.rect.min.x),
                _ => None,
            })
            .unwrap_or_else(|| panic!("nothing is painted in {fill:?}"))
    }
}

mod keys {
    use super::*;

    #[test]
    fn tab_reaches_each_splitter() {
        let mut d = harness();
        for path in [LEFT, BOTTOM, RIGHT] {
            tab_to(&mut d, path);
        }
    }

    /// The focus ring a splitter paints around `rect`, outside it.
    fn ring_around(d: &Driver<App>, rect: egui::Rect) -> Option<(usize, egui::epaint::RectShape)> {
        d.harness()
            .output()
            .shapes
            .iter()
            .enumerate()
            .find_map(|(at, clipped)| match &clipped.shape {
                Shape::Rect(ring)
                    if support::same(ring.rect, rect)
                        && ring.stroke.width > 0.0
                        && ring.stroke_kind == egui::StrokeKind::Outside =>
                {
                    Some((at, ring.clone()))
                }
                _ => None,
            })
    }

    #[test]
    fn a_focused_splitter_shows_the_focus_ring_and_lights_its_band() {
        let mut d = harness();
        let rect = d.rect(Target::suffix(LEFT));
        assert!(
            ring_around(&d, rect).is_none(),
            "precondition: no ring at rest"
        );
        tab_to(&mut d, LEFT);
        d.frames(12);
        let (_, ring) = ring_around(&d, rect).expect("a ring outside the hit area");
        let focus = Shadcn::for_theme(Theme::Light).style().tokens.focus();
        assert_eq!(
            (ring.stroke.width, ring.stroke.color),
            (focus.width, focus.color)
        );
        let hot = Shadcn::for_theme(Theme::Light).style().splitter.hot.band;
        assert_ne!(hot, Color32::TRANSPARENT, "precondition: a band when hot");
        assert!(
            support::shapes(&d).into_iter().any(|shape| matches!(
                shape,
                Shape::Rect(band) if band.rect == rect && band.fill == hot
            )),
            "the band fills the hit area"
        );
    }

    #[test]
    fn the_next_panes_fill_does_not_cover_the_ring() {
        let mut d = harness();
        let rect = d.rect(Target::suffix(LEFT));
        let editor = d.rect(Target::suffix(EDITOR_PATH));
        tab_to(&mut d, LEFT);
        d.frames(12);
        let (ring_at, ring) = ring_around(&d, rect).expect("a ring outside the hit area");
        let fill_at = d
            .harness().output()
            .shapes
            .iter()
            .position(|clipped| {
                matches!(&clipped.shape, Shape::Rect(pane) if pane.fill == EDITOR && support::same(pane.rect, editor))
            })
            .expect("precondition: the editor pane is filled");
        assert!(
            ring.rect.expand(ring.stroke.width).intersects(editor),
            "precondition: the ring reaches over the editor pane: {:?} and {editor:?}",
            ring.rect
        );
        assert!(fill_at < ring_at, "the ring is painted over the next pane");
    }

    #[test]
    fn a_callers_focus_ring_replaces_the_splitters() {
        const OWN: Color32 = Color32::from_rgb(220, 30, 60);
        fn view(b: &mut Builder<'_, '_, App>, app: &App) {
            b.div_id("frame", tw().flex().size_full(), |b| {
                b.div_id("sidebar", tw().w_px(app.sidebar.into()), |_| {});
                b.splitter("left")
                    .before(app.sidebar)
                    .named("Sidebar")
                    .classes(tw().focus(|p| p.ring(2.0, OWN)))
                    .on_resize(|app: &mut App, size| app.sidebar = size);
                b.div_id("editor", tw().flex_1().bg(EDITOR), |_| {});
            });
        }
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            view,
        )
        .build();
        d.ctx().set_theme(Theme::Light);
        d.frames(4);
        let rect = d.rect(Target::suffix(LEFT));
        tab_to(&mut d, LEFT);
        d.frames(12);
        let (_, ring) = ring_around(&d, rect).expect("a ring outside the hit area");
        assert_eq!((ring.stroke.width, ring.stroke.color), (2.0, OWN));
        let focus = Shadcn::for_theme(Theme::Light).style().tokens.focus();
        assert!(
            !support::shapes(&d).into_iter().any(|shape| matches!(
                shape,
                Shape::Rect(other) if other.stroke.color == focus.color && other.stroke.width > 0.0
            )),
            "and no other ring"
        );
    }

    #[test]
    fn arrows_step_ten_points_and_shift_fifty() {
        let mut d = harness();
        tab_to(&mut d, LEFT);
        press(&mut d, Modifiers::NONE, Key::ArrowRight);
        assert_eq!(sidebar(&d), 250.0);
        press(&mut d, Modifiers::SHIFT, Key::ArrowRight);
        assert_eq!(sidebar(&d), 300.0);
        press(&mut d, Modifiers::NONE, Key::ArrowLeft);
        assert_eq!(sidebar(&d), 290.0);
        press(&mut d, Modifiers::NONE, Key::ArrowDown);
        assert_eq!(sidebar(&d), 290.0, "the other axis's arrows do nothing");
        assert_eq!(
            focused_path(&d).as_deref(),
            Some(LEFT),
            "and do not move focus"
        );
    }

    #[test]
    fn an_arrow_towards_a_pane_after_the_splitter_shrinks_it() {
        let mut d = harness();
        tab_to(&mut d, RIGHT);
        press(&mut d, Modifiers::NONE, Key::ArrowRight);
        assert_eq!(inspector(&d), 190.0);
        tab_to(&mut d, BOTTOM);
        press(&mut d, Modifiers::NONE, Key::ArrowUp);
        assert_eq!(terminal(&d), 160.0);
    }

    #[test]
    fn home_and_end_go_to_the_ends_of_the_range() {
        let mut d = harness();
        tab_to(&mut d, LEFT);
        press(&mut d, Modifiers::NONE, Key::End);
        assert_eq!(sidebar(&d), 480.0);
        press(&mut d, Modifiers::NONE, Key::Home);
        assert_eq!(sidebar(&d), 160.0);
    }

    #[test]
    fn end_does_nothing_without_a_max_and_home_goes_to_zero_without_a_min() {
        let mut d = harness();
        tab_to(&mut d, RIGHT);
        press(&mut d, Modifiers::NONE, Key::End);
        assert_eq!(inspector(&d), 200.0);
        tab_to(&mut d, BOTTOM);
        press(&mut d, Modifiers::NONE, Key::Home);
        assert_eq!(terminal(&d), 0.0);
    }
}

mod squeeze {
    use super::*;

    #[test]
    fn a_narrow_window_squeezes_the_pane_and_keeps_its_size() {
        let mut d = harness();
        d.harness_mut().set_size(vec2(400.0, 600.0));
        d.frames(3);
        let squeezed = d.rect(Target::suffix(SIDEBAR)).width();
        assert!(squeezed < 240.0, "precondition: squeezed to {squeezed}");
        assert_eq!(sidebar(&d), 240.0, "the stored size stays");
        d.harness_mut().set_size(WINDOW);
        d.frames(3);
        assert_eq!(
            d.rect(Target::suffix(SIDEBAR)).width(),
            240.0,
            "and comes back"
        );
    }

    #[test]
    fn a_drag_on_a_squeezed_pane_starts_from_its_laid_out_size() {
        let mut d = harness();
        d.harness_mut().set_size(vec2(340.0, 600.0));
        d.frames(3);
        let squeezed = d.rect(Target::suffix(SIDEBAR)).width();
        assert!(
            (170.0..200.0).contains(&squeezed),
            "precondition: squeezed to {squeezed}, clear of the min"
        );
        let from = centre(&d, LEFT);
        d.drag(from, from - vec2(10.0, 0.0));
        assert_eq!(sidebar(&d), squeezed - 10.0);
    }
}

mod access {
    use super::*;

    #[test]
    fn each_splitter_is_a_named_splitter_with_its_size_and_range() {
        let d = harness();
        let node = d.harness().get_by_label("Sidebar");
        let node = node.accesskit_node();
        assert_eq!(node.role(), Role::Splitter);
        assert_eq!(node.numeric_value(), Some(240.0));
        assert_eq!(node.min_numeric_value(), Some(160.0));
        assert_eq!(node.max_numeric_value(), Some(480.0));
        assert_eq!(d.harness().query_all_by_role(Role::Splitter).count(), 3);
    }
}

mod misuse {
    use eggshell_ui::Problem;

    use super::*;

    fn outside_flex(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().size_full(), |b| {
            b.splitter("loose")
                .before(app.sidebar)
                .named("Sidebar")
                .on_resize(|app: &mut App, size| app.sidebar = size);
        });
    }

    #[test]
    fn a_splitter_outside_a_flex_container_is_still_drawn() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            outside_flex,
        )
        .build();
        d.frames(2);
        assert_eq!(d.harness().query_all_by_role(Role::Splitter).count(), 1);
        let report = d.expect_reported(Problem::MisplacedSplitter);
        assert!(report.message.contains("\"loose\""), "{}", report.message);
    }

    fn backwards(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().flex().size_full(), |b| {
            b.splitter("backwards")
                .before(app.sidebar)
                .named("Sidebar")
                .range(300.0..=100.0)
                .on_resize(|app: &mut App, size| app.sidebar = size);
        });
    }

    #[test]
    fn a_range_that_ends_before_it_starts_ends_at_its_start() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            backwards,
        )
        .build();
        d.frames(2);
        let node = d.harness().get_by_role(Role::Splitter);
        let node = node.accesskit_node();
        assert_eq!(
            (node.min_numeric_value(), node.max_numeric_value()),
            (Some(300.0), Some(300.0))
        );
        let report = d.expect_reported(Problem::BadRange);
        assert!(
            report.message.contains("ends before it starts"),
            "{}",
            report.message
        );
    }

    #[test]
    fn splitters_in_flex_containers_with_ranges_report_nothing() {
        let mut d = harness();
        d.frames(2);
    }
}

mod example {
    use super::*;

    #[test]
    fn the_example_resizes_all_three_panes_and_says_their_sizes() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            split_panes::Panes::default(),
            split_panes::layout,
        )
        .build();
        d.frames(4);
        let moves = [
            ("/Frame/Left", vec2(30.0, 0.0)),
            ("/Frame/Editor/Bottom", vec2(0.0, -40.0)),
            ("/Frame/Right", vec2(-20.0, 0.0)),
        ];
        for (path, by) in moves {
            let from = d.rect(Target::suffix(path)).center();
            d.drag(from, from + by);
        }
        let panes = &d.app();
        let sizes = [panes.sidebar, panes.terminal, panes.inspector].map(f32::from);
        assert_eq!(sizes, [250.0, 200.0, 260.0]);
        for said in ["250 pt", "200 pt", "260 pt"] {
            d.harness().get_by_label(said);
        }
    }
}
