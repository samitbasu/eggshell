//! Why a node has the value it has (#443): the class that won each property,
//! and, for a pinned node, each class it beat.

use eggshell_ui::sheet::{Layer, Part, When};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Debug, Match, Property, Source, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, vec2};

use crate::support;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Chip;

impl Part for Chip {
    const SOURCE: &'static str = "badge.tsx";

    fn slot(self) -> &'static str {
        "chip"
    }
}

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(
        |style| {
            style.sheet.as_preset().rule(Chip, When::always(), |t| {
                t.w_px(100.0).h_px(30.0).hover(|p| p.bg(Color32::RED))
            });
            style
                .sheet
                .as_app()
                .rule(Chip, When::always(), |t| t.w_px(80.0));
        },
        |b| {
            b.div_id(
                "container",
                tw().descendants(Match::slot("chip"), |t| t.w_px(60.0)),
                |b| {
                    let classes = b.part(Chip, &[]).merge(tw().w_px(50.0));
                    b.div_id("chip", classes, |_| {});
                },
            );
        },
    );
}

fn driver() -> Driver<()> {
    let mut d = support::driver(vec2(300.0, 200.0), base_text(), (), view).build();
    d.frames(4);
    d
}

#[test]
fn a_node_keeps_the_class_that_won_each_property() {
    let mut d = driver();
    let width = d
        .why(Target::suffix("\"chip\""), Property::width)
        .expect("a class set it");
    assert_eq!(width.winner.origin.layer, Layer::CallSite);
    let Source::Rust(at) = width.winner.origin.source else {
        panic!("a Rust call");
    };
    assert!(at.file().ends_with("why.rs"), "{at}");
    let height = d
        .why(Target::suffix("\"chip\""), Property::height)
        .expect("the preset's");
    assert_eq!(height.winner.origin.layer, Layer::Preset);
    assert_eq!(
        d.why(Target::suffix("\"chip\""), Property::padding_left),
        None
    );
}

#[test]
fn a_pinned_node_keeps_each_class_it_beat_from_the_next_frame() {
    let mut d = driver();
    let path = "/\"container\"/\"chip\"";
    d.frames(1);
    assert!(
        d.frame()
            .why(path, Property::width)
            .expect("set")
            .beaten
            .is_empty(),
        "not traced"
    );
    Debug::pin(d.ctx(), Some(path.to_owned()));
    assert!(
        d.frame()
            .why(path, Property::width)
            .expect("set")
            .beaten
            .is_empty(),
        "the frame already built is as it was"
    );
    d.frames(1);
    let why = d.frame().why(path, Property::width).expect("set");
    let layers: Vec<Layer> = why.beaten.iter().map(|step| step.origin.layer).collect();
    assert_eq!(layers, [Layer::Preset, Layer::App, Layer::Ancestor]);
    assert_eq!(why.winner.origin.layer, Layer::CallSite);
    for step in why.beaten.iter().chain([&why.winner]) {
        assert!(!step.value.is_empty(), "{step:?}");
    }
}

#[test]
fn the_computed_style_lists_the_winner_and_what_it_beat() {
    let mut d = driver();
    let path = "/\"container\"/\"chip\"";
    Debug::pin(d.ctx(), Some(path.to_owned()));
    d.frames(2);
    let table = d.frame().computed_style(path).expect("the node");
    assert!(table.starts_with(path), "{table}");
    let width: Vec<&str> = table
        .lines()
        .filter(|line| line.contains("Preset"))
        .collect();
    assert!(
        width
            .iter()
            .any(|line| line.starts_with("width") || line.contains("(overridden)")),
        "{table}"
    );
    assert!(
        table.contains("CallSite") && table.contains("why.rs"),
        "{table}"
    );
    assert!(table.contains("(overridden)"), "{table}");
    assert!(
        table.contains("VariantHover"),
        "a hover: variant too: {table}"
    );
}

#[test]
fn a_click_with_the_overlay_on_pins_the_node_and_escape_unpins_it() {
    let mut d = driver();
    Debug::set(d.ctx(), Debug::Outlines);
    d.frames(1);
    assert_eq!(Debug::pinned(d.ctx()), None);
    d.click(Target::suffix("\"chip\""));
    assert_eq!(
        Debug::pinned(d.ctx()).as_deref(),
        Some("/\"container\"/\"chip\"")
    );
    d.press(Key::Escape);
    assert_eq!(Debug::pinned(d.ctx()), None);
}
