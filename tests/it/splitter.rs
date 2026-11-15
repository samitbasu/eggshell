//! Splitters through kittest's real pointer and key events: the drag, the
//! keys, the reset, the accessibility tree and how layout squeezes a pane.

use std::time::Duration;

use eggshell_ui::owned::PaneSize;
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::epaint::Shape;
use egui::{Color32, CursorIcon, Key, Modifiers, Pos2, Role, Theme, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, drag, focused_path, hover_over, rect_of};

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

fn harness() -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        WINDOW,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(4);
    harness
}

const LEFT: &str = "/\"frame\"/\"left\"";
const RIGHT: &str = "/\"frame\"/\"right\"";
const BOTTOM: &str = "/\"frame\"/\"editor\"/\"bottom\"";
const SIDEBAR: &str = "/\"frame\"/\"sidebar\"";
const INSPECTOR: &str = "/\"frame\"/\"inspector\"";
const TERMINAL: &str = "/\"frame\"/\"editor\"/\"terminal\"";
const EDITOR_PATH: &str = "/\"frame\"/\"editor\"";

fn sidebar(harness: &Harness<'_, Rig<App>>) -> f32 {
    harness.state().app.sidebar.into()
}

fn inspector(harness: &Harness<'_, Rig<App>>) -> f32 {
    harness.state().app.inspector.into()
}

fn terminal(harness: &Harness<'_, Rig<App>>) -> f32 {
    harness.state().app.terminal.into()
}

fn centre(harness: &Harness<'_, Rig<App>>, path: &str) -> Pos2 {
    rect_of(harness, path).center()
}

/// Presses at `from` and moves the pointer through `to`, one frame a point,
/// without releasing.
fn press_and_move(harness: &mut Harness<'_, Rig<App>>, from: Pos2, to: &[Pos2]) {
    hover_over(harness, from);
    harness.drag_at(from);
    harness.step();
    for &at in to {
        harness.hover_at(at);
        harness.step();
    }
}

fn release(harness: &mut Harness<'_, Rig<App>>, at: Pos2) {
    harness.drop_at(at);
    harness.run_steps(3);
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..12 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(harness));
}

fn press(harness: &mut Harness<'_, Rig<App>>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run_steps(3);
}

mod pointer {
    use super::*;

    #[test]
    fn dragging_a_splitter_sizes_the_pane_before_it() {
        let mut harness = harness();
        assert_eq!(rect_of(&harness, SIDEBAR).width(), 240.0, "precondition");
        let from = centre(&harness, LEFT);
        drag(&mut harness, from, from + vec2(40.0, 0.0));
        assert_eq!(sidebar(&harness), 280.0);
        assert_eq!(rect_of(&harness, SIDEBAR).width(), 280.0);
        assert_eq!(
            rect_of(&harness, LEFT).center().x,
            from.x + 40.0,
            "the handle follows"
        );
    }

    #[test]
    fn a_splitter_after_its_pane_grows_it_towards_the_start() {
        let mut harness = harness();
        let from = centre(&harness, RIGHT);
        drag(&mut harness, from, from - vec2(50.0, 0.0));
        assert_eq!(inspector(&harness), 250.0);
        assert_eq!(rect_of(&harness, INSPECTOR).width(), 250.0);
    }

    #[test]
    fn a_splitter_in_a_column_moves_vertically() {
        let mut harness = harness();
        let from = centre(&harness, BOTTOM);
        drag(&mut harness, from, from - vec2(0.0, 30.0));
        assert_eq!(terminal(&harness), 180.0);
        assert_eq!(rect_of(&harness, TERMINAL).height(), 180.0);
    }

    #[test]
    fn a_drag_past_the_range_clamps_and_coming_back_does_not_drift() {
        let mut harness = harness();
        let from = centre(&harness, LEFT);
        let far = from + vec2(600.0, 200.0);
        press_and_move(&mut harness, from, &[far]);
        assert!(
            !rect_of(&harness, LEFT).contains(far),
            "precondition: the pointer left the handle"
        );
        assert_eq!(sidebar(&harness), 480.0, "held at the max");
        let back = from + vec2(-500.0, 0.0);
        harness.hover_at(back);
        harness.step();
        assert_eq!(sidebar(&harness), 160.0, "held at the min");
        let near = from + vec2(20.0, 0.0);
        harness.hover_at(near);
        harness.step();
        release(&mut harness, near);
        assert_eq!(sidebar(&harness), 260.0, "measured from the press");
    }

    #[test]
    fn a_double_click_resets_the_pane() {
        let mut harness = harness();
        let from = centre(&harness, LEFT);
        drag(&mut harness, from, from + vec2(100.0, 0.0));
        assert_eq!(sidebar(&harness), 340.0, "precondition");
        let at = centre(&harness, LEFT);
        hover_over(&mut harness, at);
        for _ in 0..2 {
            support::press_and_release(&mut harness, at);
        }
        harness.run_steps(2);
        assert_eq!(sidebar(&harness), 240.0);
    }

    #[test]
    fn a_click_without_a_move_reports_nothing() {
        let mut harness = harness();
        let at = centre(&harness, LEFT);
        hover_over(&mut harness, at);
        support::press_and_release(&mut harness, at);
        harness.run_steps(2);
        assert_eq!(harness.state().app.resizes, 0);
    }

    #[test]
    fn the_pointer_shows_a_resize_cursor_over_the_handle() {
        let mut harness = harness();
        let left = centre(&harness, LEFT);
        hover_over(&mut harness, left);
        assert_eq!(
            harness.output().platform_output.cursor_icon,
            CursorIcon::ResizeColumn
        );
        let bottom = centre(&harness, BOTTOM);
        hover_over(&mut harness, bottom);
        assert_eq!(
            harness.output().platform_output.cursor_icon,
            CursorIcon::ResizeRow
        );
    }

    /// The editor slides when its place changes, but a drag moves it smoothly
    /// already, so it follows the handle frame by frame.
    #[test]
    fn a_drag_starts_no_slide_on_a_neighbour_that_animates_its_layout() {
        let mut harness = harness();
        let from = centre(&harness, LEFT);
        let steps: Vec<Pos2> = (1..=4).map(|i| from + vec2(20.0 * i as f32, 0.0)).collect();
        press_and_move(&mut harness, from, &steps);
        harness.step();
        let laid = rect_of(&harness, EDITOR_PATH);
        assert_ne!(laid.min.x, 241.0 + 6.0, "precondition: the editor moved");
        assert_eq!(painted_left(&harness, EDITOR), laid.min.x);
        release(&mut harness, *steps.last().expect("steps"));
    }

    fn painted_left(harness: &Harness<'_, Rig<App>>, fill: Color32) -> f32 {
        support::shapes(harness)
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
        let mut harness = harness();
        for path in [LEFT, BOTTOM, RIGHT] {
            tab_to(&mut harness, path);
        }
    }

    #[test]
    fn arrows_step_ten_points_and_shift_fifty() {
        let mut harness = harness();
        tab_to(&mut harness, LEFT);
        press(&mut harness, Modifiers::NONE, Key::ArrowRight);
        assert_eq!(sidebar(&harness), 250.0);
        press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
        assert_eq!(sidebar(&harness), 300.0);
        press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
        assert_eq!(sidebar(&harness), 290.0);
        press(&mut harness, Modifiers::NONE, Key::ArrowDown);
        assert_eq!(
            sidebar(&harness),
            290.0,
            "the other axis's arrows do nothing"
        );
        assert_eq!(
            focused_path(&harness).as_deref(),
            Some(LEFT),
            "and do not move focus"
        );
    }

    #[test]
    fn an_arrow_towards_a_pane_after_the_splitter_shrinks_it() {
        let mut harness = harness();
        tab_to(&mut harness, RIGHT);
        press(&mut harness, Modifiers::NONE, Key::ArrowRight);
        assert_eq!(inspector(&harness), 190.0);
        tab_to(&mut harness, BOTTOM);
        press(&mut harness, Modifiers::NONE, Key::ArrowUp);
        assert_eq!(terminal(&harness), 160.0);
    }

    #[test]
    fn home_and_end_go_to_the_ends_of_the_range() {
        let mut harness = harness();
        tab_to(&mut harness, LEFT);
        press(&mut harness, Modifiers::NONE, Key::End);
        assert_eq!(sidebar(&harness), 480.0);
        press(&mut harness, Modifiers::NONE, Key::Home);
        assert_eq!(sidebar(&harness), 160.0);
    }

    #[test]
    fn end_does_nothing_without_a_max_and_home_goes_to_zero_without_a_min() {
        let mut harness = harness();
        tab_to(&mut harness, RIGHT);
        press(&mut harness, Modifiers::NONE, Key::End);
        assert_eq!(inspector(&harness), 200.0);
        tab_to(&mut harness, BOTTOM);
        press(&mut harness, Modifiers::NONE, Key::Home);
        assert_eq!(terminal(&harness), 0.0);
    }
}

mod squeeze {
    use super::*;

    #[test]
    fn a_narrow_window_squeezes_the_pane_and_keeps_its_size() {
        let mut harness = harness();
        harness.set_size(vec2(400.0, 600.0));
        harness.run_steps(3);
        let squeezed = rect_of(&harness, SIDEBAR).width();
        assert!(squeezed < 240.0, "precondition: squeezed to {squeezed}");
        assert_eq!(sidebar(&harness), 240.0, "the stored size stays");
        harness.set_size(WINDOW);
        harness.run_steps(3);
        assert_eq!(rect_of(&harness, SIDEBAR).width(), 240.0, "and comes back");
    }

    #[test]
    fn a_drag_on_a_squeezed_pane_starts_from_its_laid_out_size() {
        let mut harness = harness();
        harness.set_size(vec2(340.0, 600.0));
        harness.run_steps(3);
        let squeezed = rect_of(&harness, SIDEBAR).width();
        assert!(
            (170.0..200.0).contains(&squeezed),
            "precondition: squeezed to {squeezed}, clear of the min"
        );
        let from = centre(&harness, LEFT);
        drag(&mut harness, from, from - vec2(10.0, 0.0));
        assert_eq!(sidebar(&harness), squeezed - 10.0);
    }
}

mod access {
    use super::*;

    #[test]
    fn each_splitter_is_a_named_splitter_with_its_size_and_range() {
        let harness = harness();
        let node = harness.get_by_label("Sidebar");
        let node = node.accesskit_node();
        assert_eq!(node.role(), Role::Splitter);
        assert_eq!(node.numeric_value(), Some(240.0));
        assert_eq!(node.min_numeric_value(), Some(160.0));
        assert_eq!(node.max_numeric_value(), Some(480.0));
        assert_eq!(harness.query_all_by_role(Role::Splitter).count(), 3);
    }
}

mod misuse {
    use super::*;

    fn outside_flex(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().size_full(), |b| {
            b.splitter("loose")
                .before(app.sidebar)
                .on_resize(|app: &mut App, size| app.sidebar = size);
        });
    }

    #[test]
    #[should_panic(expected = "\"loose\": a splitter must be built directly in a flex container")]
    fn a_splitter_outside_a_flex_container_panics() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            outside_flex,
        );
        harness.run_steps(1);
    }

    fn backwards(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().flex().size_full(), |b| {
            b.splitter("backwards")
                .before(app.sidebar)
                .range(300.0..=100.0)
                .on_resize(|app: &mut App, size| app.sidebar = size);
        });
    }

    #[test]
    #[should_panic(expected = "a splitter's range must not end before it starts")]
    fn a_range_that_ends_before_it_starts_panics() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::default(),
            backwards,
        );
        harness.run_steps(1);
    }
}

mod example {
    use super::*;

    #[test]
    fn the_example_resizes_all_three_panes_and_says_their_sizes() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            WINDOW,
            TextStyle::new(tokens.text_role(), tokens.foreground),
            split_panes::Panes::default(),
            split_panes::layout,
        );
        harness.run_steps(4);
        let moves = [
            ("/Frame/Left", vec2(30.0, 0.0)),
            ("/Frame/Editor/Bottom", vec2(0.0, -40.0)),
            ("/Frame/Right", vec2(-20.0, 0.0)),
        ];
        for (path, by) in moves {
            let from = rect_of(&harness, path).center();
            drag(&mut harness, from, from + by);
        }
        let panes = &harness.state().app;
        let sizes = [panes.sidebar, panes.terminal, panes.inspector].map(f32::from);
        assert_eq!(sizes, [250.0, 200.0, 260.0]);
        for said in ["250 pt", "200 pt", "260 pt"] {
            harness.get_by_label(said);
        }
    }
}
