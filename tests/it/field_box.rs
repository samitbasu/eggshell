//! A field box, and a text field with a leading or trailing icon (#245).

use eggshell_ui::contract::Size;
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, size};
use eggshell_ui::{Builder, Icon, TextSpec, TextStyle, Weight, icons, tw};
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
                    s.button("addon", "Go")
                        .variant(eggshell_ui::theme::variant::GHOST)
                        .size(size::SM)
                        .on_click(|_: &mut App| {});
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
        |_, _| {},
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
    Shadcn::for_theme(Theme::Light).style().input.clone()
}

/// The space between the border and an icon, and between the icon and the
/// text: shadcn's at the default size (`pl-2` on the addon, `pl-1.5` on the
/// control, style-vega.css:1406 and 1398), the size's metrics at the others
/// (a recorded exception, #446).
fn spacing(size: Size) -> (f32, f32) {
    if size == Size::DEFAULT {
        (8.0, 6.0)
    } else {
        let style = Shadcn::for_theme(Theme::Light).style();
        (
            support::field_box(&style, size).padding.x,
            style.tokens.points(1.5),
        )
    }
}

#[test]
fn a_field_box_is_the_inputs_box_at_its_size() {
    let d = driver(Size::DEFAULT);
    let field = support::field_box(&Shadcn::for_theme(Theme::Light).style(), size::SM);
    let rect = d.rect(Target::suffix(CUSTOM));
    assert_eq!(rect.height(), field.height);
    assert_eq!(rect.width(), 200.0);
    let shape = box_at(&d, rect);
    assert_eq!(
        shape.fill,
        Color32::TRANSPARENT,
        "cn-input-group: no fill in light"
    );
    assert_eq!(
        shape.stroke.color,
        support::input_border(&Shadcn::for_theme(Theme::Light).style()).1
    );
    assert_eq!(shape.corner_radius, egui::CornerRadius::same(field.radius));
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
fn a_focused_addon_button_rings_itself_not_its_group() {
    let mut d = driver(Size::DEFAULT);
    let group = d.rect(Target::suffix(CUSTOM));
    d.click(Target::suffix(r#"/"Page"/"custom"/"inner""#));
    d.frames(6);
    assert!(
        ring_at(&d, group).is_some(),
        "precondition: the field rings it"
    );
    d.press(egui::Key::Tab).frames(6);
    let addon = r#"/"Page"/"custom"/"addon""#;
    assert_eq!(support::focused_path(&d).as_deref(), Some(addon));
    let button = d.rect(Target::suffix(addon));
    assert!(ring_at(&d, button).is_some(), "the button rings");
    assert!(ring_at(&d, group).is_none(), "the group does not");
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
        let tokens = Shadcn::for_theme(Theme::Light).tokens;
        let icon_side = tokens.points(match size {
            s if s == size::SM => 3.5,
            s if s == size::XS => 3.0,
            _ => 4.0,
        });
        let border = support::input_border(&Shadcn::for_theme(Theme::Light).style()).0;
        let (boxed, icon, field) = (
            d.rect(Target::suffix(SEARCH)),
            d.rect(Target::suffix(&format!("{SEARCH}/LeadingAddon/Leading"))),
            d.rect(Target::suffix(&format!("{SEARCH}/Field"))),
        );
        let (edge, gap) = spacing(size);
        assert_eq!(
            boxed.height(),
            support::field_box(&Shadcn::for_theme(Theme::Light).style(), size).height,
            "{label}"
        );
        assert_eq!(icon.size(), vec2(icon_side, icon_side), "{label}");
        assert!(
            (icon.left() - boxed.left() - border - edge).abs() < 0.6,
            "{label}: {icon:?} in {boxed:?}"
        );
        assert!((icon.center().y - boxed.center().y).abs() < 0.6, "{label}");
        let text = support::text_left(&d, field, "Search");
        assert!(
            (text - icon.right() - gap).abs() < 0.6,
            "{label}: the text at {text} after {icon:?}"
        );
        assert!(field.right() <= boxed.right());
    }
}

#[test]
fn a_trailing_icon_sits_inside_the_box_after_the_field() {
    let d = driver(Size::DEFAULT);
    let border = support::input_border(&Shadcn::for_theme(Theme::Light).style()).0;
    let (boxed, icon, field) = (
        d.rect(Target::suffix(CLEAR)),
        d.rect(Target::suffix(&format!("{CLEAR}/TrailingAddon/Trailing"))),
        d.rect(Target::suffix(&format!("{CLEAR}/Field"))),
    );
    let (edge, gap) = spacing(Size::DEFAULT);
    assert!((boxed.right() - icon.right() - border - edge).abs() < 0.6);
    assert!(
        (icon.left() - field.right()).abs() < 0.6,
        "the addon follows the field: {field:?} before {icon:?}"
    );
    let style = Shadcn::for_theme(Theme::Light).style();
    let beside = style.sheet.resolve(
        eggshell_ui::owned::input::GroupPart::Input,
        &[
            eggshell_ui::sheet::Key::Size(Size::DEFAULT),
            eggshell_ui::sheet::Key::Attr("addon", "inline-end"),
        ],
        &style.tokens,
    );
    assert_eq!(
        beside
            .style_at(style.tokens.spacing)
            .padding
            .right
            .into_raw()
            .value(),
        gap,
        "the field's own padding beside it, pr-1.5"
    );
}

#[test]
fn a_field_may_have_both_icons() {
    let d = driver(Size::DEFAULT);
    let (boxed, lead, trail) = (
        d.rect(Target::suffix(BOTH)),
        d.rect(Target::suffix(&format!("{BOTH}/LeadingAddon/Leading"))),
        d.rect(Target::suffix(&format!("{BOTH}/TrailingAddon/Trailing"))),
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

/// The left and right padding the style gives a default-size field with an
/// icon on the `addon` side.
fn control_padding(addon: &'static str) -> (f32, f32) {
    let style = Shadcn::for_theme(Theme::Light).style();
    let padding = style
        .sheet
        .resolve(
            eggshell_ui::owned::input::GroupPart::Input,
            &[
                eggshell_ui::sheet::Key::Size(Size::DEFAULT),
                eggshell_ui::sheet::Key::Attr("addon", addon),
            ],
            &style.tokens,
        )
        .style_at(style.tokens.spacing)
        .padding;
    (
        padding.left.into_raw().value(),
        padding.right.into_raw().value(),
    )
}

/// Which side of a field a click lands on.
#[derive(Clone, Copy, Debug)]
enum Side {
    Left,
    Right,
}

/// Clicks in the padding on `side` of the field of `path`, whose icon is on
/// the `addon` side: the click focuses the field.
fn a_click_in_the_padding_focuses(path: &str, addon: &'static str, side: Side) {
    let mut d = driver(Size::DEFAULT);
    let (left, right) = control_padding(addon);
    let (boxed, field) = (
        d.rect(Target::suffix(path)),
        d.rect(Target::suffix(&format!("{path}/Field"))),
    );
    let y = field.center().y;
    let at = match side {
        Side::Left => egui::pos2(field.left() + left / 2.0, y),
        Side::Right => egui::pos2(field.right() - right / 2.0, y),
    };
    let text =
        egui::Rect::from_x_y_ranges(field.left() + left..=field.right() - right, field.y_range());
    assert!(
        left > 0.0 && right > 0.0,
        "precondition: the input pads both sides: {left}, {right}"
    );
    assert!(
        boxed.contains(at) && field.contains(at) && !text.contains(at),
        "precondition: {at:?} is in the box {boxed:?} and the field {field:?}, in its padding outside {text:?}"
    );
    assert_eq!(
        support::focused_node(&d),
        None,
        "precondition: nothing has focus"
    );
    d.click(at).frames(4);
    assert_eq!(
        support::focused_node(&d),
        Some(format!("{path}/Field")),
        "{side:?} padding of {path}"
    );
}

#[test]
fn a_click_in_the_padding_between_a_leading_icon_and_the_text_focuses_the_field() {
    a_click_in_the_padding_focuses(SEARCH, "inline-start", Side::Left);
}

#[test]
fn a_click_in_the_padding_after_the_text_of_a_field_with_a_leading_icon_focuses_it() {
    a_click_in_the_padding_focuses(SEARCH, "inline-start", Side::Right);
}

#[test]
fn a_click_in_the_padding_before_the_text_of_a_field_with_a_trailing_icon_focuses_it() {
    a_click_in_the_padding_focuses(CLEAR, "inline-end", Side::Left);
}

#[test]
fn a_click_in_the_padding_between_the_text_and_a_trailing_icon_focuses_the_field() {
    a_click_in_the_padding_focuses(CLEAR, "inline-end", Side::Right);
}

/// Where `at` lies in the field of `path` with its icon on `side`: inside
/// the addon, and outside the field's leaf.
fn in_the_addon(d: &Driver<App>, path: &str, side: &str, at: egui::Pos2) {
    let (addon, field) = (
        d.rect(Target::suffix(&format!("{path}/{side}Addon"))),
        d.rect(Target::suffix(&format!("{path}/Field"))),
    );
    assert!(
        addon.contains(at) && !field.contains(at),
        "precondition: {at:?} is in the addon {addon:?}, not the field {field:?}"
    );
}

/// Clicks `at`, in the addon on `side` of the field of `path`: the click
/// focuses the field.
fn a_click_in_the_addon_focuses(mut d: Driver<App>, path: &str, side: &str, at: egui::Pos2) {
    in_the_addon(&d, path, side, at);
    assert_eq!(
        support::focused_node(&d),
        None,
        "precondition: nothing has focus"
    );
    d.click(at).frames(4);
    assert_eq!(
        support::focused_node(&d),
        Some(format!("{path}/Field")),
        "a click at {at:?} in the {side} addon of {path}"
    );
}

#[test]
fn a_click_on_a_leading_icon_focuses_the_field() {
    let d = driver(Size::DEFAULT);
    let icon = d.rect(Target::suffix(&format!("{SEARCH}/LeadingAddon/Leading")));
    a_click_in_the_addon_focuses(d, SEARCH, "Leading", icon.center());
}

#[test]
fn a_click_on_a_trailing_icon_focuses_the_field() {
    let d = driver(Size::DEFAULT);
    let icon = d.rect(Target::suffix(&format!("{CLEAR}/TrailingAddon/Trailing")));
    a_click_in_the_addon_focuses(d, CLEAR, "Trailing", icon.center());
}

/// A point in the leading addon's padding of [`SEARCH`], between the box's
/// border and the icon.
fn in_the_leading_padding(d: &Driver<App>) -> egui::Pos2 {
    let border = support::input_border(&Shadcn::for_theme(Theme::Light).style()).0;
    let (boxed, icon) = (
        d.rect(Target::suffix(SEARCH)),
        d.rect(Target::suffix(&format!("{SEARCH}/LeadingAddon/Leading"))),
    );
    let at = egui::pos2((boxed.left() + border + icon.left()) / 2.0, icon.center().y);
    assert!(
        !icon.contains(at) && at.x > boxed.left() + border,
        "precondition: {at:?} is between the border of {boxed:?} and the icon {icon:?}"
    );
    at
}

#[test]
fn a_click_in_an_addons_padding_focuses_the_field() {
    let d = driver(Size::DEFAULT);
    let at = in_the_leading_padding(&d);
    a_click_in_the_addon_focuses(d, SEARCH, "Leading", at);
}

#[test]
fn a_click_on_an_icon_keeps_the_fields_caret() {
    let mut d = driver(Size::DEFAULT);
    d.click(Target::suffix(&format!("{SEARCH}/Field")))
        .frames(4);
    d.type_text("abc").frames(3);
    d.press(egui::Key::Home).frames(3);
    let page = d.rect(Target::suffix(r#"/"Page""#));
    d.click(egui::pos2(page.center().x, page.bottom() - 2.0))
        .frames(4);
    assert_eq!(
        support::focused_node(&d),
        None,
        "precondition: the field gave up focus"
    );
    let icon = d.rect(Target::suffix(&format!("{SEARCH}/LeadingAddon/Leading")));
    d.click(icon.center()).frames(4);
    d.type_text("Z").frames(3);
    assert_eq!(d.app().text, "Zabc", "typed at the caret Home left");
}

#[test]
fn the_pointer_is_a_text_cursor_over_an_addon() {
    let mut d = driver(Size::DEFAULT);
    let at = in_the_leading_padding(&d);
    in_the_addon(&d, SEARCH, "Leading", at);
    let page = d.rect(Target::suffix(r#"/"Page""#));
    d.hover(egui::pos2(page.center().x, page.bottom() - 2.0))
        .frames(2);
    assert_ne!(
        d.harness().output().platform_output.cursor_icon,
        egui::CursorIcon::Text,
        "precondition: not a text cursor over the page"
    );
    d.hover(at).frames(2);
    assert_eq!(
        d.harness().output().platform_output.cursor_icon,
        egui::CursorIcon::Text
    );
}

#[test]
fn a_field_with_an_icon_keeps_its_text_inside_its_padding() {
    let d = driver(Size::DEFAULT);
    let field = d.rect(Target::suffix(&format!("{SEARCH}/Field")));
    let (left, _) = control_padding("inline-start");
    let text = support::text_left(&d, field, "Search");
    assert!(
        (text - field.left() - left).abs() < 0.6,
        "the hint sits the field's padding in: {text} in {field:?}"
    );
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

fn restyled(b: &mut Builder<'_, '_, App>, _: &App) {
    b.styled(
        |style| {
            style
                .sheet
                .as_app()
                .rule(eggshell_ui::owned::input::GroupPart::Root, |t, _| {
                    t.bg(Color32::from_rgb(200, 10, 10))
                });
        },
        |s| {
            s.field_box("custom")
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

#[test]
fn a_rule_on_the_input_group_part_restyles_the_box() {
    let mut d = support::prepared_driver(
        vec2(340.0, 120.0),
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            Color32::BLACK,
        ),
        App {
            text: String::new(),
            size: Size::DEFAULT,
            disabled: false,
        },
        restyled,
        |_, _| {},
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(6);
    let shape = box_at(&d, d.rect(Target::suffix(r#"/"custom""#)));
    assert_eq!(shape.fill, Color32::from_rgb(200, 10, 10));
    assert_eq!(
        shape.stroke.color,
        support::input_border(&Shadcn::for_theme(Theme::Light).style()).1,
        "the preset's other properties stand"
    );
}

#[test]
fn a_bare_field_is_the_groups_control_only_inside_an_input_group() {
    let d = driver(Size::DEFAULT);
    let inside = d.rect(Target::suffix(r#"/"Page"/"custom"/"inner""#));
    let custom = d.rect(Target::suffix(CUSTOM));
    assert!(
        inside.width() < custom.width(),
        "precondition: the control sits in the group beside its addon"
    );
    let placed = eggshell_ui::testing::Headless::new(vec2(400.0, 300.0))
        .theme(Theme::Light)
        .style(&eggshell_ui::SharedStyle::new(support::shadcn()))
        .base_text(TextStyle::new(
            Shadcn::for_theme(Theme::Light).tokens.text_role(),
            Color32::BLACK,
        ))
        .layout(&(), |b, _| {
            b.div(tw().flex().flex_col().w_px(120.0), |b| {
                b.span("line", tw(), "1.234");
                b.text_field_with(
                    "bare",
                    eggshell_ui::owned::text_field::Field::singleline("1.234").bare(),
                )
                .on_change(|_, _| {});
            });
        });
    assert_eq!(
        placed.at("bare").rect().height(),
        placed.at("line").rect().height(),
        "outside a group, a bare field is its text: no control padding or height"
    );
}

#[test]
fn a_disabled_group_fades_as_a_whole_and_its_addon_with_it() {
    use eggshell_ui::owned::input;
    use eggshell_ui::sheet::Key;
    let style = Shadcn::for_theme(Theme::Light).style();
    let addon = |disabled: bool| {
        let mut keys = vec![Key::Size(Size::DEFAULT), Key::Attr("align", "inline-start")];
        if disabled {
            keys.push(Key::Aria(eggshell_ui::Aria::Disabled));
        }
        style
            .sheet
            .resolve(input::GroupPart::Addon, &keys, &style.tokens)
            .paint_classes()
            .opacity
    };
    let root = |keys: &[Key]| {
        style
            .sheet
            .resolve(input::GroupPart::Root, keys, &style.tokens)
            .paint_classes()
            .opacity
    };
    let disabled = [
        Key::Size(Size::DEFAULT),
        Key::Aria(eggshell_ui::Aria::Disabled),
    ];
    assert_eq!(
        root(&disabled),
        eggshell_ui::Opacity::new(0.5),
        "the whole group at half: the disabled look (#385)"
    );
    assert_eq!(root(&disabled[..1]), eggshell_ui::Opacity::new(1.0));
    assert_eq!(
        addon(true),
        addon(false),
        "the addon fades with its group, not again on its own"
    );
}

/// A command list, whose search box is an input group with the search icon
/// in its addon.
fn command_list(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().p(4.0).w_px(300.0), |s| {
        s.command_list("commands").named("Commands").items(|items| {
            items.item("alpha", "Alpha").on_pick(|_| {});
        });
    });
}

#[test]
fn a_click_on_the_command_search_icon_focuses_the_query_field() {
    let mut d = support::driver(
        vec2(340.0, 300.0),
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            },
            Color32::BLACK,
        ),
        (),
        command_list,
    )
    .build();
    d.frames(6);
    let query = "/Wrapper/Search/Query";
    let (icon, field) = (
        d.rect(Target::suffix(r#"/Search/SearchIcon/"icon""#)),
        d.rect(Target::suffix(query)),
    );
    let at = icon.center();
    assert!(
        icon.contains(at) && !field.contains(at),
        "precondition: {at:?} is on the icon {icon:?}, not the field {field:?}"
    );
    let on_query =
        |d: &Driver<()>| support::focused_node(d).is_some_and(|path| path.ends_with(query));
    assert!(
        !on_query(&d),
        "precondition: the query field does not have focus"
    );
    d.click(at).frames(4);
    assert!(on_query(&d), "focused: {:?}", support::focused_node(&d));
}
