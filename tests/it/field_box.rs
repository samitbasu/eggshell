//! A field box, and a text field with a leading or trailing icon (#245).

use eggshell_ui::contract::Size;
use eggshell_ui::{Builder, Icon, TextSpec, TextStyle, Weight, icons, tw};
use eggshell_ui_shadcn::{Shadcn, size};
use egui::{Color32, FontFamily, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, box_at, click_node, rect_of, ring_at};

struct App {
    text: String,
    size: Size,
    disabled: bool,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let enablement = if app.disabled {
        eggshell_ui::owned::Enablement::Disabled
    } else {
        eggshell_ui::owned::Enablement::Enabled
    };
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(4.0).p(4.0).w_px(300.0),
        |s| {
            s.div_id("sized", tw().control_size(app.size), |s| {
                s.text_field("search", app.text.clone())
                    .named("Search")
                    .hint("Search")
                    .leading(Icon::new(icons::SEARCH))
                    .on_change(|app: &mut App, text| app.text = text);
                s.text_field("clear", "x")
                    .named("Clear")
                    .trailing(Icon::new(icons::CLOSE))
                    .on_change(|_: &mut App, _| {});
                s.text_field("both", "")
                    .named("Both")
                    .leading(Icon::new(icons::SEARCH))
                    .trailing(Icon::new(icons::HELP))
                    .on_change(|_: &mut App, _| {});
                s.text_field("plain", "plain")
                    .named("Plain")
                    .on_change(|_: &mut App, _| {});
            });
            s.field_box("custom")
                .size(size::SM)
                .enablement(enablement)
                .classes(tw().w_px(200.0))
                .children(|s| {
                    s.text_field("inner", "inside")
                        .named("Inner")
                        .bare()
                        .on_change(|_: &mut App, _| {});
                });
        },
    );
}

fn harness(size: Size) -> Harness<'static, Rig<App>> {
    let tokens = Shadcn::for_theme(Theme::Light).tokens;
    let mut harness = support::harness_prepared(
        vec2(340.0, 360.0),
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            tokens.foreground,
        ),
        App {
            text: String::new(),
            size,
            disabled: false,
        },
        view,
        |ctx, _| {
            Shadcn::apply_both(ctx);
        },
    );
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(6);
    harness
}

const SEARCH: &str = r#"/"Page"/"sized"/"search""#;
const CLEAR: &str = r#"/"Page"/"sized"/"clear""#;
const BOTH: &str = r#"/"Page"/"sized"/"both""#;
const CUSTOM: &str = r#"/"Page"/"custom""#;

fn input() -> eggshell_ui::contract::InputStyle {
    Shadcn::for_theme(Theme::Light).style().input
}

#[test]
fn a_field_box_is_the_inputs_box_at_its_size() {
    let harness = harness(Size::DEFAULT);
    let input = input();
    let metrics = *input.sizes.get(size::SM);
    let rect = rect_of(&harness, CUSTOM);
    assert_eq!(rect.height(), metrics.height);
    assert_eq!(rect.width(), 200.0);
    let shape = box_at(&harness, rect);
    assert_eq!(shape.fill, input.look.paint.states.rest.fill);
    assert_eq!(shape.stroke.color, input.look.paint.states.rest.border);
    assert_eq!(
        shape.corner_radius,
        egui::CornerRadius::same(metrics.radius)
    );
}

#[test]
fn a_field_box_rings_while_a_field_inside_has_focus() {
    let mut harness = harness(Size::DEFAULT);
    let rect = rect_of(&harness, CUSTOM);
    assert!(
        ring_at(&harness, rect).is_none(),
        "precondition: no ring at rest"
    );
    click_node(&mut harness, r#"/"Page"/"custom"/"inner""#);
    harness.run_steps(6);
    assert!(ring_at(&harness, rect).is_some());
}

#[test]
fn a_disabled_field_box_is_inert() {
    let mut harness = harness(Size::DEFAULT);
    harness.state_mut().app.disabled = true;
    harness.run_steps(4);
    let rect = rect_of(&harness, CUSTOM);
    click_node(&mut harness, r#"/"Page"/"custom"/"inner""#);
    harness.run_steps(6);
    assert!(ring_at(&harness, rect).is_none());
}

#[test]
fn a_leading_icon_sits_inside_the_box_before_the_field() {
    for (size, label) in [
        (Size::DEFAULT, "default"),
        (size::SM, "sm"),
        (size::XS, "xs"),
    ] {
        let harness = harness(size);
        let metrics = *input().sizes.get(size);
        let border = input().look.border_width;
        let (boxed, icon, field) = (
            rect_of(&harness, SEARCH),
            rect_of(&harness, &format!("{SEARCH}/Leading")),
            rect_of(&harness, &format!("{SEARCH}/Field")),
        );
        assert_eq!(boxed.height(), metrics.height, "{label}");
        assert_eq!(
            icon.size(),
            vec2(metrics.icon_size, metrics.icon_size),
            "{label}"
        );
        assert!(
            (icon.left() - boxed.left() - border - metrics.padding.x).abs() < 0.6,
            "{label}: {icon:?} in {boxed:?}"
        );
        assert!((icon.center().y - boxed.center().y).abs() < 0.6, "{label}");
        assert!(
            (field.left() - icon.right() - metrics.icon_gap).abs() < 0.6,
            "{label}: {field:?} after {icon:?}"
        );
        assert!(field.right() <= boxed.right());
    }
}

#[test]
fn a_trailing_icon_sits_inside_the_box_after_the_field() {
    let harness = harness(Size::DEFAULT);
    let metrics = *input().sizes.get(Size::DEFAULT);
    let border = input().look.border_width;
    let (boxed, icon, field) = (
        rect_of(&harness, CLEAR),
        rect_of(&harness, &format!("{CLEAR}/Trailing")),
        rect_of(&harness, &format!("{CLEAR}/Field")),
    );
    assert!((boxed.right() - icon.right() - border - metrics.padding.x).abs() < 0.6);
    assert!((icon.left() - field.right() - metrics.icon_gap).abs() < 0.6);
}

#[test]
fn a_field_may_have_both_icons() {
    let harness = harness(Size::DEFAULT);
    let (boxed, lead, trail) = (
        rect_of(&harness, BOTH),
        rect_of(&harness, &format!("{BOTH}/Leading")),
        rect_of(&harness, &format!("{BOTH}/Trailing")),
    );
    assert!(lead.right() < trail.left());
    assert!(boxed.contains_rect(lead) && boxed.contains_rect(trail));
}

#[test]
fn the_icons_are_drawn_in_the_styles_icon_colour() {
    let harness = harness(Size::DEFAULT);
    let boxed = rect_of(&harness, SEARCH);
    let icon = input().icon;
    let tinted = support::textured(&harness).into_iter().any(|mesh| {
        boxed.contains_rect(mesh.calc_bounds())
            && mesh.vertices.iter().all(|vertex| vertex.color == icon)
    });
    assert!(tinted, "an icon in {icon:?} inside {boxed:?}");
    assert_ne!(icon, Color32::TRANSPARENT);
}

#[test]
fn typing_in_a_field_with_an_icon_changes_the_text() {
    let mut harness = harness(Size::DEFAULT);
    click_node(&mut harness, &format!("{SEARCH}/Field"));
    harness.run_steps(4);
    harness.event(egui::Event::Text("abc".into()));
    harness.run_steps(4);
    assert_eq!(harness.state().app.text, "abc");
}

#[test]
fn a_focused_field_with_an_icon_rings_its_box() {
    let mut harness = harness(Size::DEFAULT);
    let boxed = rect_of(&harness, SEARCH);
    assert!(ring_at(&harness, boxed).is_none(), "precondition");
    click_node(&mut harness, &format!("{SEARCH}/Field"));
    harness.run_steps(6);
    assert!(ring_at(&harness, boxed).is_some());
}

#[test]
fn a_field_without_icons_keeps_its_leaf() {
    let harness = harness(Size::DEFAULT);
    rect_of(&harness, r#"/"Page"/"sized"/"plain""#);
}

#[test]
fn a_multiline_field_shows_no_icon_and_does_not_panic() {
    let tokens = Shadcn::for_theme(Theme::Light).tokens;
    let mut harness = support::harness(
        vec2(300.0, 200.0),
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            tokens.foreground,
        ),
        (),
        |b: &mut Builder<'_, '_, ()>, _| {
            b.text_field("notes", "")
                .named("Notes")
                .multiline(3)
                .leading(Icon::new(icons::SEARCH))
                .on_change(|_, _| {});
        },
    );
    harness.run_steps(2);
    rect_of(&harness, r#"/"notes""#);
    assert!(support::textured(&harness).is_empty(), "no icon is drawn");
}

#[test]
fn icons_on_a_multi_line_field_are_reported() {
    use eggshell_ui::Problem;
    use eggshell_ui::testing::{Headless, assert_no_reports, assert_reported};

    let build = |lines: Option<usize>| {
        let mut headless = Headless::new(vec2(300.0, 200.0));
        headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
            let field = b.text_field("notes", "").leading(Icon::new(icons::SEARCH));
            let field = match lines {
                Some(rows) => field.multiline(rows),
                None => field,
            };
            field.on_change(|_, _| {});
        });
        headless
    };
    assert_no_reports(build(None).ctx());
    let multi = build(Some(3));
    let report = assert_reported(multi.ctx(), Problem::IgnoredIcons);
    assert!(report.message.contains("\"notes\""), "{}", report.message);
}
