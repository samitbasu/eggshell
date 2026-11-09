//! Switching a widget between its two states moves nothing: every node of the
//! frame, labels included, keeps its rectangle. Driven by real clicks, in both
//! themes.

use bullseye::owned::{Button, Checkbox, RadioGroup, Segmented, Switch};
use bullseye::style::{Mark, Tokens};
use bullseye::{Builder, TextStyle, tw};
use egui::{Rect, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Unit {
    #[default]
    Metric,
    Imperial,
}

const UNITS: [(Unit, &str); 2] = [(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")];

#[derive(Default)]
struct App {
    toggle: bool,
    check: bool,
    radio: Unit,
    switch: bool,
    segment: Unit,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        Button::toggle("Bold", Mark::from(app.toggle)).show(b, "toggle", |app: &mut App| {
            app.toggle = !app.toggle;
        });
        Checkbox::new("Accept", Mark::from(app.check)).show(b, "check", |app: &mut App| {
            app.check = !app.check;
        });
        RadioGroup::new(app.radio, &UNITS).show(b, "radio", |app: &mut App, unit| {
            app.radio = unit;
        });
        Switch::new("Airplane mode", Mark::from(app.switch)).show(b, "switch", |app: &mut App| {
            app.switch = !app.switch;
        });
        Segmented::new(app.segment, &UNITS).show(b, "seg", |app: &mut App, _, unit| {
            app.segment = unit;
        });
    });
}

fn harness(theme: Theme) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(theme);
    let mut harness = support::harness(
        vec2(500.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    );
    harness.ctx.set_theme(theme);
    harness.run_steps(12);
    harness
}

/// The boxes and labels of the frame; the glyphs a state adds or removes come
/// and go and are not part of the layout.
fn nodes(harness: &Harness<'_, Rig<App>>) -> Vec<(String, Rect)> {
    harness
        .state()
        .nodes
        .iter()
        .filter(|(path, _)| !path.ends_with("/<paint>"))
        .cloned()
        .collect()
}

struct Flip {
    widget: &'static str,
    click: &'static str,
    changed: fn(&App) -> bool,
}

const FLIPS: [Flip; 6] = [
    Flip {
        widget: "toggle button",
        click: "/\"toggle\"",
        changed: |app| app.toggle,
    },
    Flip {
        widget: "checkbox",
        click: "/\"check\"/Box",
        changed: |app| app.check,
    },
    Flip {
        widget: "radio",
        click: "/\"radio\"/Circle(Imperial)",
        changed: |app| app.radio == Unit::Imperial,
    },
    Flip {
        widget: "switch",
        click: "/\"switch\"/Track",
        changed: |app| app.switch,
    },
    Flip {
        widget: "segmented",
        click: "/\"seg\"/Imperial",
        changed: |app| app.segment == Unit::Imperial,
    },
    Flip {
        widget: "segmented (back to the first cell)",
        click: "/\"seg\"/Metric",
        changed: |app| app.segment == Unit::Metric,
    },
];

#[test]
fn changing_a_widgets_state_moves_no_node() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness(theme);
        assert!(!nodes(&harness).is_empty(), "precondition: nodes exist");
        for flip in &FLIPS {
            let before = nodes(&harness);
            click_node(&mut harness, flip.click);
            harness.run_steps(30);
            assert!(
                (flip.changed)(&harness.state().app),
                "precondition: clicking the {} changes it ({theme:?})",
                flip.widget
            );
            assert_eq!(
                nodes(&harness),
                before,
                "{} moved something in {theme:?}",
                flip.widget
            );
        }
    }
}
