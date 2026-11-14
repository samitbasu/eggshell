//! The frame-continuity checker against small views that reproduce the four
//! classes of one-frame glitch from #76, each beside a fixed twin.

use core::time::Duration;

use eggshell_ui::testing::{Timeline, ViolationKind, record};
use eggshell_ui::{Builder, LayoutState, Opacity, Placed, tw};
use egui::{Pos2, Rect, Vec2, vec2};
use egui_kittest::Harness;

/// A view, the area it is laid out in, and what it reads back from each
/// laid-out frame to use in the next one: the last-frame state the bugs are
/// made of.
struct Fixture<A> {
    app: A,
    view: fn(&mut Builder<'_, '_, A>, &A),
    area: fn(&A) -> Vec2,
    observe: fn(&mut A, &Placed<A>),
    layout: LayoutState,
}

fn rig<A>(
    app: A,
    view: fn(&mut Builder<'_, '_, A>, &A),
    area: fn(&A) -> Vec2,
    observe: fn(&mut A, &Placed<A>),
) -> Harness<'static, Fixture<A>> {
    let fixture = Fixture {
        app,
        view,
        area,
        observe,
        layout: LayoutState::new(),
    };
    let harness = Harness::builder()
        .with_size(vec2(4000.0, 400.0))
        .with_step_dt(0.05)
        .build_ui_state(
            |ui, fixture: &mut Fixture<A>| {
                crate::support::full_motion(ui.ctx());
                let area = Rect::from_min_size(Pos2::ZERO, (fixture.area)(&fixture.app));
                let placed = fixture
                    .layout
                    .build(ui.ctx(), |b| (fixture.view)(b, &fixture.app))
                    .layout(area);
                (fixture.observe)(&mut fixture.app, &placed);
                placed.draw(&mut fixture.app, ui);
            },
            fixture,
        );
    record(&harness.ctx);
    harness
}

fn timeline<A>(harness: &Harness<'_, Fixture<A>>) -> Timeline {
    Timeline::recorded(&harness.ctx)
}

fn kinds_at(timeline: &Timeline, path: &str) -> Vec<ViolationKind> {
    timeline
        .violations()
        .into_iter()
        .filter(|violation| violation.path == path)
        .map(|violation| violation.kind)
        .collect()
}

fn full_area<A>(_: &A) -> Vec2 {
    vec2(400.0, 400.0)
}

#[derive(Clone, Copy, PartialEq)]
enum Flaw {
    Buggy,
    Fixed,
}

mod stale_offset {
    use super::*;

    const TARGET: f32 = 20.0;
    const CHASER: &str = r#"/"row"/"chaser""#;

    struct Chase {
        flaw: Flaw,
        lead: f32,
        previous_slot: f32,
    }

    fn view(b: &mut Builder<'_, '_, Chase>, app: &Chase) {
        let slot = match app.flaw {
            Flaw::Buggy => app.previous_slot,
            Flaw::Fixed => app.lead,
        };
        b.div_id("row", tw().flex().w_px(400.0).h_px(40.0), |b| {
            b.div(tw().w_px(app.lead).h_px(40.0), |_| {});
            b.div_id(
                "chaser",
                tw().w_px(300.0)
                    .h_px(40.0)
                    .sense_click()
                    .hover(|p| p.translate(TARGET - slot, 0.0)),
                |_| {},
            );
        });
    }

    fn observe(app: &mut Chase, placed: &Placed<Chase>) {
        app.previous_slot = placed.at("row").at("chaser").rect().min.x;
    }

    fn run(flaw: Flaw) -> (Timeline, Vec<f32>) {
        let mut harness = rig(
            Chase {
                flaw,
                lead: 0.0,
                previous_slot: 0.0,
            },
            view,
            full_area,
            observe,
        );
        harness.hover_at(Pos2::new(150.0, 20.0));
        harness.run_steps(4);
        harness.state_mut().app.lead = 100.0;
        harness.run_steps(6);
        let timeline = timeline(&harness);
        let painted = (0..timeline.len())
            .filter_map(|k| {
                let frame = timeline.frame(k);
                Some(frame.iter().find(|n| n.path == CHASER)?.painted_rect.min.x)
            })
            .collect();
        (timeline, painted)
    }

    #[test]
    fn an_offset_from_last_frames_slot_teleports_when_the_slot_moves() {
        let (timeline, painted) = run(Flaw::Buggy);
        assert!(
            painted.iter().any(|&x| x > TARGET + 50.0),
            "precondition: the buggy chaser really is painted away from its target: {painted:?}"
        );
        let kinds = kinds_at(&timeline, CHASER);
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, ViolationKind::Teleport { .. })),
            "{kinds:?}"
        );
    }

    #[test]
    #[should_panic(
        expected = r#"teleport: /"row"/"chaser" was painted at [[120.0 0.0] - [420.0 40.0]] in frame 4 and at [[20.0 0.0] - [320.0 40.0]] in frame 5"#
    )]
    fn the_report_names_the_node_the_frame_and_both_rects() {
        run(Flaw::Buggy).0.assert_no_teleports();
    }

    #[test]
    fn an_offset_from_this_frames_slot_stays_put() {
        let (timeline, painted) = run(Flaw::Fixed);
        assert!(
            painted.iter().all(|&x| (x - TARGET).abs() < 0.01) && painted.len() > 8,
            "precondition: the chaser is held at its target by a translate: {painted:?}"
        );
        timeline.assert_continuous();
    }
}

mod area_feedback {
    use super::*;

    const SLIDE: Duration = Duration::from_millis(1000);
    const ROW: f32 = 80.0;

    struct List {
        flaw: Flaw,
        order: Vec<u32>,
        last_height: f32,
    }

    fn view(b: &mut Builder<'_, '_, List>, app: &List) {
        for &key in &app.order {
            b.div_id(
                key,
                tw().w_px(100.0)
                    .h_px(ROW)
                    .animate_layout(SLIDE)
                    .ease_linear(),
                |_| {},
            );
        }
    }

    fn area(app: &List) -> Vec2 {
        match app.flaw {
            Flaw::Buggy => vec2(400.0, app.last_height),
            Flaw::Fixed => vec2(400.0, 400.0),
        }
    }

    fn observe(app: &mut List, _: &Placed<List>) {
        app.last_height = app.order.len() as f32 * ROW;
    }

    fn run(flaw: Flaw) -> Timeline {
        let mut harness = rig(
            List {
                flaw,
                order: vec![0, 1, 2],
                last_height: 3.0 * ROW,
            },
            view,
            area,
            observe,
        );
        harness.run_steps(4);
        let order = &mut harness.state_mut().app.order;
        order.reverse();
        order.push(3);
        harness.run_steps(25);
        timeline(&harness)
    }

    /// Before #80 this was a bug the checker caught: the area following the
    /// content is a resize frame, which cut the slide short.
    #[test]
    fn an_area_that_follows_last_frames_content_no_longer_abandons_the_slide() {
        let timeline = run(Flaw::Buggy);
        let sliding = (0..timeline.len()).any(|k| {
            timeline
                .frame(k)
                .iter()
                .any(|n| n.path == "/2" && n.painted_rect != n.layout_rect)
        });
        assert!(sliding, "precondition: the rows slide");
        timeline.assert_continuous();
    }

    #[test]
    fn a_stable_area_lets_the_slide_finish() {
        let timeline = run(Flaw::Fixed);
        let sliding = (0..timeline.len()).any(|k| {
            timeline
                .frame(k)
                .iter()
                .any(|n| n.path == "/2" && n.painted_rect != n.layout_rect)
        });
        assert!(sliding, "precondition: the rows really do slide");
        timeline.assert_continuous();
    }
}

mod blip {
    use super::*;

    const TILE: &str = r#"/"tile""#;

    #[derive(Clone, Copy)]
    enum Stale {
        Fades,
        Absent,
    }

    struct Tile {
        stale: Stale,
        flaw: Flaw,
        memo: f32,
    }

    fn view(b: &mut Builder<'_, '_, Tile>, app: &Tile) {
        let memo = match app.flaw {
            Flaw::Buggy => app.memo,
            Flaw::Fixed => 1.0,
        };
        b.div(tw().flex(), |b| match app.stale {
            Stale::Absent if memo == 0.0 => {}
            _ => {
                b.div_id(
                    "tile",
                    tw().w_px(100.0).h_px(40.0).opacity(Opacity::new(memo)),
                    |_| {},
                );
            }
        });
    }

    fn observe(app: &mut Tile, _: &Placed<Tile>) {
        app.memo = 1.0;
    }

    fn run(stale: Stale, flaw: Flaw) -> Timeline {
        let mut harness = rig(
            Tile {
                stale,
                flaw,
                memo: 1.0,
            },
            view,
            full_area,
            observe,
        );
        harness.run_steps(3);
        harness.state_mut().app.memo = 0.0;
        harness.run_steps(5);
        timeline(&harness)
    }

    #[test]
    fn a_node_faded_for_one_frame_by_stale_state_blips() {
        let timeline = run(Stale::Fades, Flaw::Buggy);
        let blips: Vec<_> = timeline
            .gaps()
            .into_iter()
            .filter(
                |v| matches!(v.kind, ViolationKind::OpacityBlip { during, .. } if during == 0.0),
            )
            .collect();
        assert_eq!(blips.len(), 1, "{blips:?}");
        assert_eq!(blips[0].path, TILE);
    }

    #[test]
    fn a_node_absent_for_one_frame_by_stale_state_leaves_a_gap() {
        let timeline = run(Stale::Absent, Flaw::Buggy);
        let gaps = timeline.gaps();
        assert!(
            matches!(&gaps[..], [v] if v.kind == ViolationKind::Gap && v.path == TILE),
            "{gaps:?}"
        );
    }

    #[test]
    fn without_the_stale_state_there_is_no_blip() {
        for stale in [Stale::Fades, Stale::Absent] {
            let timeline = run(stale, Flaw::Fixed);
            assert!(timeline.len() >= 8, "precondition: frames were recorded");
            timeline.assert_continuous();
        }
    }
}

mod oscillation {
    use super::*;

    const BOX: &str = r#"/"box""#;

    struct Panel {
        flaw: Flaw,
        last_width: f32,
    }

    fn view(b: &mut Builder<'_, '_, Panel>, app: &Panel) {
        let width = match app.flaw {
            Flaw::Buggy if app.last_width > 100.0 => 80.0,
            _ => 120.0,
        };
        b.div_id("box", tw().w_px(width).h_px(40.0), |_| {});
    }

    fn observe(app: &mut Panel, placed: &Placed<Panel>) {
        app.last_width = placed.at("box").rect().width();
    }

    fn run(flaw: Flaw) -> Timeline {
        let mut harness = rig(
            Panel {
                flaw,
                last_width: 0.0,
            },
            view,
            full_area,
            observe,
        );
        harness.run_steps(12);
        timeline(&harness)
    }

    #[test]
    fn a_width_chosen_from_last_frames_width_never_settles() {
        let timeline = run(Flaw::Buggy);
        let widths: Vec<f32> = (0..timeline.len())
            .map(|k| {
                let frame = timeline.frame(k);
                frame
                    .iter()
                    .find(|n| n.path == BOX)
                    .unwrap()
                    .layout_rect
                    .width()
            })
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] != pair[1]),
            "precondition: the width flips every frame: {widths:?}"
        );
        let unsettled = timeline.settles();
        assert!(
            unsettled
                .iter()
                .all(|v| v.path == BOX && matches!(v.kind, ViolationKind::Unsettled { .. }))
                && !unsettled.is_empty(),
            "{unsettled:?}"
        );
    }

    #[test]
    #[should_panic(expected = r#"unsettled: layout of /"box" changed from"#)]
    fn the_report_names_the_node_and_both_rects() {
        run(Flaw::Buggy).assert_continuous();
    }

    #[test]
    fn a_stable_width_settles() {
        let timeline = run(Flaw::Fixed);
        assert!(timeline.len() >= 12, "precondition: frames were recorded");
        timeline.assert_continuous();
    }
}

mod slide_path {
    use super::*;

    const SLIDE: Duration = Duration::from_millis(450);
    const DISTANCE: f32 = 3000.0;
    const MOVER: &str = r#"/"row"/"mover""#;
    /// Frames after the move at which the buggy views misbehave: halfway.
    const HALFWAY: u32 = 5;
    const FRAME_SECONDS: f32 = 0.05;

    #[derive(Clone, Copy, PartialEq)]
    enum Fault {
        None,
        SnapsToTheEnd,
        JumpsBack,
    }

    struct Slider {
        fault: Fault,
        lead: f32,
        frames: u32,
        moved_at: Option<u32>,
    }

    impl Slider {
        fn since_move(&self) -> Option<u32> {
            self.moved_at.map(|at| self.frames - at)
        }

        fn remaining(&self, since: u32) -> f32 {
            let elapsed = since as f32 * FRAME_SECONDS / SLIDE.as_secs_f32();
            DISTANCE * (1.0 - elapsed.min(1.0))
        }

        /// How far the buggy views move the painting, by hover translate.
        fn push(&self) -> f32 {
            match (self.fault, self.since_move()) {
                (Fault::SnapsToTheEnd, Some(since)) if since >= HALFWAY => self.remaining(since),
                (Fault::JumpsBack, Some(since)) if since >= HALFWAY => -0.3 * DISTANCE,
                _ => 0.0,
            }
        }
    }

    fn view(b: &mut Builder<'_, '_, Slider>, app: &Slider) {
        let push = app.push();
        b.div_id("row", tw().flex().w_px(DISTANCE + 1000.0).h_px(40.0), |b| {
            b.div(tw().w_px(app.lead).h_px(40.0), |_| {});
            b.div_id(
                "mover",
                tw().w_px(300.0)
                    .h_px(40.0)
                    .hover(|p| p.translate(push, 0.0))
                    .animate_layout(SLIDE)
                    .ease_linear(),
                |_| {},
            );
        });
    }

    fn observe(app: &mut Slider, _: &Placed<Slider>) {
        app.frames += 1;
    }

    fn run(fault: Fault) -> Timeline {
        let mut harness = rig(
            Slider {
                fault,
                lead: 0.0,
                frames: 0,
                moved_at: None,
            },
            view,
            |_| vec2(4000.0, 400.0),
            observe,
        );
        harness.hover_at(Pos2::new(DISTANCE + 150.0, 20.0));
        harness.run_steps(3);
        let app = &mut harness.state_mut().app;
        app.lead = DISTANCE;
        app.moved_at = Some(app.frames);
        harness.run_steps(14);
        timeline(&harness)
    }

    fn painted_x(timeline: &Timeline) -> Vec<f32> {
        (0..timeline.len())
            .filter_map(|k| {
                let frame = timeline.frame(k);
                Some(frame.iter().find(|n| n.path == MOVER)?.painted_rect.min.x)
            })
            .collect()
    }

    fn kinds(timeline: &Timeline) -> Vec<ViolationKind> {
        kinds_at(timeline, MOVER)
    }

    #[test]
    fn a_slide_that_snaps_to_the_end_halfway_is_off_its_path() {
        let timeline = run(Fault::SnapsToTheEnd);
        let painted = painted_x(&timeline);
        let steps: Vec<f32> = painted.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert!(
            steps.iter().filter(|step| step.abs() > 200.0).count() >= 2,
            "precondition: a long, fast slide with a snap in it: {steps:?}"
        );
        assert!(
            kinds(&timeline)
                .iter()
                .any(|kind| matches!(kind, ViolationKind::OffPath { .. })),
            "{:?}",
            kinds(&timeline)
        );
    }

    #[test]
    #[should_panic(expected = r#"off path: /"row"/"mover" slid from frame"#)]
    fn the_report_names_the_node_and_both_moves() {
        run(Fault::SnapsToTheEnd).assert_continuous();
    }

    #[test]
    fn a_slide_that_jumps_backwards_is_off_its_path() {
        let timeline = run(Fault::JumpsBack);
        assert!(
            kinds(&timeline)
                .iter()
                .any(|kind| matches!(kind, ViolationKind::OffPath { .. })),
            "{:?}",
            kinds(&timeline)
        );
    }

    #[test]
    fn the_fixed_twin_slides_the_whole_way() {
        let timeline = run(Fault::None);
        let painted = painted_x(&timeline);
        let biggest = painted
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0, f32::max);
        assert!(
            biggest > 200.0 && painted.first() == Some(&0.0),
            "precondition: a long, fast slide: {painted:?}"
        );
        assert_eq!(painted.last(), Some(&DISTANCE));
        timeline.assert_continuous();
    }
}

mod long_grid_slide {
    use super::*;

    const SLIDE: Duration = Duration::from_millis(450);
    const PANEL: f32 = 1000.0;

    struct Grid {
        columns: u32,
    }

    fn view(b: &mut Builder<'_, '_, Grid>, app: &Grid) {
        b.div_id(
            "grid",
            tw().flex()
                .flex_wrap()
                .w_px(PANEL * app.columns as f32)
                .h_px(400.0),
            |b| {
                for panel in 0..3u32 {
                    b.div_id(
                        panel,
                        tw().w_px(PANEL).h_px(100.0).animate_layout(SLIDE),
                        |_| {},
                    );
                }
            },
        );
    }

    #[test]
    fn three_columns_to_one_slides_thousands_of_points_without_an_allowance() {
        let mut harness = rig(
            Grid { columns: 3 },
            view,
            |_| vec2(4000.0, 400.0),
            |_, _| {},
        );
        harness.run_steps(3);
        harness.state_mut().app.columns = 1;
        harness.run_steps(14);
        let timeline = timeline(&harness);
        let travelled: f32 = (0..timeline.len())
            .filter_map(|k| {
                timeline
                    .frame(k)
                    .iter()
                    .find(|n| n.path == r#"/"grid"/2"#)
                    .map(|n| n.painted_rect.min.x)
            })
            .collect::<Vec<_>>()
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0, f32::max);
        assert!(
            travelled > 100.0,
            "precondition: panels move far in one frame: {travelled}"
        );
        timeline.assert_continuous();
    }
}

/// What an ancestor's slide moves a node by is the ancestor's to keep
/// continuous, so a node that does not slide itself is judged by its own paint
/// offset.
mod inside_a_slide {
    use super::*;

    const MOVER: &str = r#"/"row"/"mover""#;
    const INSIDE: &str = r#"/"row"/"mover"/"inside""#;

    fn view(b: &mut Builder<'_, '_, f32>, lead: &f32) {
        b.div_id("row", tw().flex().w_px(4000.0).h_px(40.0), |b| {
            b.div(tw().w_px(*lead).h_px(40.0), |_| {});
            b.div_id(
                "mover",
                tw().w_px(100.0)
                    .h_px(40.0)
                    .animate_layout(Duration::from_millis(200)),
                |b| {
                    b.div_id(
                        "inside",
                        tw().w_px(50.0)
                            .h_px(40.0)
                            .hover(|p| p.translate(200.0, 0.0)),
                        |_| {},
                    );
                },
            );
        });
    }

    fn run(hover_it_at_the_end: bool) -> Timeline {
        let mut harness = rig(0.0, view, |_| vec2(4000.0, 400.0), |_, _| {});
        harness.run_steps(3);
        harness.state_mut().app = 3000.0;
        harness.run_steps(16);
        if hover_it_at_the_end {
            harness.hover_at(Pos2::new(3010.0, 20.0));
            harness.run_steps(3);
        }
        timeline(&harness)
    }

    #[test]
    fn a_child_that_follows_a_fast_slide_is_continuous() {
        let timeline = run(false);
        let at = |k: usize| {
            timeline
                .frame(k)
                .iter()
                .find(|n| n.path == INSIDE)
                .map(|n| n.painted_rect.min.x)
        };
        let fastest = (1..timeline.len())
            .filter_map(|k| Some((at(k)? - at(k - 1)?).abs()))
            .fold(0.0, f32::max);
        assert!(
            fastest > 40.0,
            "precondition: the slide moves the child by more than a step: {fastest}"
        );
        assert!(
            kinds_at(&timeline, INSIDE).is_empty(),
            "{:?}",
            timeline.violations()
        );
        assert!(kinds_at(&timeline, MOVER).is_empty());
    }

    #[test]
    fn a_child_that_jumps_by_itself_inside_a_slide_is_still_found() {
        let timeline = run(true);
        assert!(
            kinds_at(&timeline, INSIDE)
                .iter()
                .any(|kind| matches!(kind, ViolationKind::Teleport { .. })),
            "{:?}",
            timeline.violations()
        );
    }
}
