//! A field box, and a text field with a leading or trailing icon (#245).

use eggshell_ui::contract::Size;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Icon, TextSpec, TextStyle, Weight, icons, tw};
use eggshell_ui_shadcn::{Shadcn, size};
use egui::{Color32, FontFamily, Theme, vec2};

use crate::support::{self, box_at, ring_at};

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

fn driver(size: Size) -> Driver<App> {
    let tokens = Shadcn::for_theme(Theme::Light).tokens;
    let mut d = support::prepared_driver(
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
    )
    .build();
    support::set_page(&d, support::Page::Themed);
    support::set_companion(&d, support::Companion::Alone);
    d.ctx().set_theme(Theme::Light);
    d.frames(6);
    d
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
    let d = driver(Size::DEFAULT);
    let input = input();
    let metrics = *input.sizes.get(size::SM);
    let rect = d.rect(Target::suffix(CUSTOM));
    assert_eq!(rect.height(), metrics.height);
    assert_eq!(rect.width(), 200.0);
    let shape = box_at(&d, rect);
    assert_eq!(shape.fill, input.look.paint.states.rest.fill);
    assert_eq!(shape.stroke.color, input.look.paint.states.rest.border);
    assert_eq!(
        shape.corner_radius,
        egui::CornerRadius::same(metrics.radius)
    );
}

#[test]
fn a_field_box_rings_while_a_field_inside_has_focus() {
    let mut d = driver(Size::DEFAULT);
    let rect = d.rect(Target::suffix(CUSTOM));
    assert!(ring_at(&d, rect).is_none(), "precondition: no ring at rest");
    d.click(Target::suffix(r#"/"Page"/"custom"/"inner""#));
    d.frames(6);
    assert!(ring_at(&d, rect).is_some());
}

#[test]
fn a_disabled_field_box_is_inert() {
    let mut d = driver(Size::DEFAULT);
    d.app_mut().disabled = true;
    d.frames(4);
    let rect = d.rect(Target::suffix(CUSTOM));
    d.click(Target::suffix(r#"/"Page"/"custom"/"inner""#));
    d.frames(6);
    assert!(ring_at(&d, rect).is_none());
}

#[test]
fn a_leading_icon_sits_inside_the_box_before_the_field() {
    for (size, label) in [
        (Size::DEFAULT, "default"),
        (size::SM, "sm"),
        (size::XS, "xs"),
    ] {
        let d = driver(size);
        let metrics = *input().sizes.get(size);
        let border = input().look.border_width;
        let (boxed, icon, field) = (
            d.rect(Target::suffix(SEARCH)),
            d.rect(Target::suffix(&format!("{SEARCH}/Leading"))),
            d.rect(Target::suffix(&format!("{SEARCH}/Field"))),
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
    let d = driver(Size::DEFAULT);
    let metrics = *input().sizes.get(Size::DEFAULT);
    let border = input().look.border_width;
    let (boxed, icon, field) = (
        d.rect(Target::suffix(CLEAR)),
        d.rect(Target::suffix(&format!("{CLEAR}/Trailing"))),
        d.rect(Target::suffix(&format!("{CLEAR}/Field"))),
    );
    assert!((boxed.right() - icon.right() - border - metrics.padding.x).abs() < 0.6);
    assert!((icon.left() - field.right() - metrics.icon_gap).abs() < 0.6);
}

#[test]
fn a_field_may_have_both_icons() {
    let d = driver(Size::DEFAULT);
    let (boxed, lead, trail) = (
        d.rect(Target::suffix(BOTH)),
        d.rect(Target::suffix(&format!("{BOTH}/Leading"))),
        d.rect(Target::suffix(&format!("{BOTH}/Trailing"))),
    );
    assert!(lead.right() < trail.left());
    assert!(boxed.contains_rect(lead) && boxed.contains_rect(trail));
}

#[test]
fn the_icons_are_drawn_in_the_styles_icon_colour() {
    let d = driver(Size::DEFAULT);
    let boxed = d.rect(Target::suffix(SEARCH));
    let icon = input().icon;
    let tinted = support::textured(&d).into_iter().any(|mesh| {
        boxed.contains_rect(mesh.calc_bounds())
            && mesh.vertices.iter().all(|vertex| vertex.color == icon)
    });
    assert!(tinted, "an icon in {icon:?} inside {boxed:?}");
    assert_ne!(icon, Color32::TRANSPARENT);
}

#[test]
fn typing_in_a_field_with_an_icon_changes_the_text() {
    let mut d = driver(Size::DEFAULT);
    d.click(Target::suffix(&format!("{SEARCH}/Field")));
    d.frames(4);
    d.type_text("abc").frames(3);
    assert_eq!(d.app().text, "abc");
}

#[test]
fn a_focused_field_with_an_icon_rings_its_box() {
    let mut d = driver(Size::DEFAULT);
    let boxed = d.rect(Target::suffix(SEARCH));
    assert!(ring_at(&d, boxed).is_none(), "precondition");
    d.click(Target::suffix(&format!("{SEARCH}/Field")));
    d.frames(6);
    assert!(ring_at(&d, boxed).is_some());
}

#[test]
fn a_field_without_icons_keeps_its_leaf() {
    let d = driver(Size::DEFAULT);
    d.rect(Target::suffix(r#"/"Page"/"sized"/"plain""#));
}

#[test]
fn a_multiline_field_shows_no_icon_and_does_not_panic() {
    let tokens = Shadcn::for_theme(Theme::Light).tokens;
    let mut d = support::driver(
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
    )
    .build();
    d.frames(2);
    d.rect(Target::suffix(r#"/"notes""#));
    assert!(support::textured(&d).is_empty(), "no icon is drawn");
    d.expect_reported(eggshell_ui::Problem::IgnoredIcons);
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
