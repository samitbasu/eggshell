//! The frame-continuity checker against small views that reproduce the four
//! classes of one-frame glitch from #76, each beside a fixed twin.

use core::time::Duration;

use bullseye::testing::{Timeline, ViolationKind, record};
use bullseye::{Builder, LayoutState, Opacity, Placed, tw};
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

fn rig<A: 'static>(
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
        .with_size(vec2(400.0, 400.0))
        .with_step_dt(0.05)
        .build_ui_state(
            |ui, fixture: &mut Fixture<A>| {
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

    #[test]
    fn a_slide_is_abandoned_when_the_area_follows_last_frames_content() {
        let timeline = run(Flaw::Buggy);
        let kinds = kinds_at(&timeline, "/2");
        assert!(
            kinds
                .iter()
                .any(|kind| matches!(kind, ViolationKind::Teleport { .. })),
            "{kinds:?}"
        );
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
