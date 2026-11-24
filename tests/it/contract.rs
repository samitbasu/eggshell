//! `eggshell_ui::Style`: the neutral default, installing and caching, and open
//! keys that fall back to the default entry and are reported.

use std::sync::Arc;

use eggshell_ui::contract::{Keyed, Size, Variant};
use eggshell_ui::testing::{Headless, assert_no_reports, assert_reported};
use eggshell_ui::{Builder, LayoutState, Problem, Style, Weight};
use eggshell_ui_shadcn::Shadcn;
use egui::{Color32, Context, Theme, vec2};

#[test]
fn unknown_keys_fall_back_to_the_default_entry() {
    let keyed = Keyed::new(Variant("plain"), 1).with(Variant("loud"), 2);
    assert_eq!(*keyed.get(Variant("loud")), 2);
    assert_eq!(*keyed.get(Variant("plain")), 1);
    assert_eq!(keyed.default_key(), Variant("plain"));
    assert_eq!(
        keyed.keys().collect::<Vec<_>>(),
        [Variant("plain"), Variant("loud")]
    );

    assert_eq!(*keyed.get(Variant("missing")), 1);
}

#[test]
fn a_widget_drawn_at_a_size_the_style_lacks_is_reported_and_uses_the_default() {
    let mut headless = Headless::new(vec2(200.0, 100.0));
    let placed_at = |headless: &mut Headless, size: Size| {
        headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
            b.checkbox("check", "Agree", false)
                .size(size)
                .on_change(|_, _| {});
        })
    };
    let default = placed_at(&mut headless, Size::DEFAULT);
    assert_no_reports(headless.ctx());
    let missing = placed_at(&mut headless, Size("huge"));
    let report = assert_reported(headless.ctx(), Problem::MissingStyleEntry);
    assert!(
        report.message.contains("Size(\"huge\")"),
        "{}",
        report.message
    );
    assert!(
        default.nodes().count() > 1,
        "precondition: the checkbox has nodes"
    );
    assert!(
        missing.diff(&default).is_empty(),
        "laid out with the default entry"
    );
}

#[test]
fn a_replaced_default_entry_is_what_unknown_keys_get() {
    let mut keyed = Keyed::new(Size("md"), 1);
    keyed.insert(Size("md"), 5);
    assert_eq!(*keyed.get(Size("md")), 5);
    assert!(keyed.get_mut(Size("sm")).is_none());
}

#[test]
fn a_layout_given_no_style_draws_with_the_default_in_egui_s_fonts_and_keeps_it() {
    let ctx = Context::default();
    let shell = LayoutState::new().shell(&ctx);
    let style = shell.style();
    let default = Style::default();
    let button = |style: &Style| {
        *crate::support::button::classes(style, Variant::DEFAULT, Size::DEFAULT).paint_classes()
    };
    assert_eq!(button(&style), button(&default));
    assert_eq!(style.input, default.input);
    assert_eq!(style.text, default.text);
    assert_eq!(
        style.faces,
        eggshell_ui::Faces::default(),
        "no face, so egui's fonts draw it"
    );
    assert_eq!(
        style.toggle.text.as_ref().map(|text| text.weight),
        Some(Weight::Normal)
    );
    assert!(Arc::ptr_eq(&style, &shell.style()), "no rebuild per call");
}

fn disabled_colours(style: &Style) -> Vec<(String, eggshell_ui::contract::Colors)> {
    let mut all = vec![("label".to_string(), style.label.disabled)];
    let mut look = |name: &str, paint: &eggshell_ui::contract::Paint| {
        all.push((name.to_string(), paint.states.disabled));
    };
    for key in style.toggle.off.keys() {
        look(&format!("toggle off {key:?}"), style.toggle.off.get(key));
    }
    for key in style.toggle.on.keys() {
        look(&format!("toggle on {key:?}"), style.toggle.on.get(key));
    }
    look("slider thumb", &style.slider.thumb.paint);
    look("stepper group", &style.stepper.group.paint);
    look("stepper button", &style.stepper.button.paint);
    let item = crate::support::part_at(
        style,
        eggshell_ui::owned::MenuPart::Item,
        &[
            eggshell_ui::sheet::Key::Variant(Variant::DEFAULT),
            eggshell_ui::sheet::Key::Aria(eggshell_ui::Aria::Disabled),
        ],
    );
    // shadcn fades a disabled item with `opacity-50` and gives it no colours.
    let text = crate::support::ink(style, &item);
    if text != Color32::PLACEHOLDER {
        all.push((
            "menu item".to_string(),
            eggshell_ui::contract::Colors {
                fill: item
                    .paint_classes()
                    .background
                    .unwrap_or(Color32::TRANSPARENT),
                border: Color32::TRANSPARENT,
                text,
            },
        ));
    }
    all.push(("slider track".to_string(), style.slider.track.disabled));
    all.push(("slider range".to_string(), style.slider.range.disabled));
    all
}

#[test]
fn disabled_colours_are_opaque_or_transparent_never_partly_faded() {
    let mut styles = Vec::new();
    for theme in [Theme::Light, Theme::Dark] {
        styles.push((format!("{theme:?}"), Shadcn::for_theme(theme).style()));
    }
    for (which, style) in styles {
        let all = disabled_colours(&style);
        assert!(
            all.len() >= 8,
            "precondition: the walk reaches every look (a button's is the sheet's)"
        );
        for (name, colors) in all {
            for (part, color) in [
                ("fill", colors.fill),
                ("border", colors.border),
                ("text", colors.text),
            ] {
                assert!(
                    color.a() == 255 || color.a() == 0,
                    "{which} {name} disabled {part} is {color:?}"
                );
            }
        }
    }
}

#[test]
fn a_disabled_colour_is_half_way_to_the_surface_it_sits_on() {
    use eggshell_ui::Opacity;
    use eggshell_ui::contract::Colors;
    let surface = Color32::from_rgb(200, 100, 0);
    let rest = Colors {
        fill: Color32::from_rgb(0, 0, 0),
        border: Color32::TRANSPARENT,
        text: Color32::from_rgb(200, 100, 0),
    };
    let faded = rest.faded_over(Opacity::new(0.5), surface);
    assert_eq!(faded.border, Color32::TRANSPARENT);
    assert_eq!(faded.text, surface);
    let fill = faded.fill;
    assert_eq!(fill.a(), 255);
    assert!((i32::from(fill.r()) - 100).abs() <= 8, "{fill:?}");
}

/// The input section and the field's rule map onto the node's egui style: a
/// deliberately unusual one, every value distinct, so one that is not mapped
/// shows.
#[test]
fn the_input_section_is_mapped_onto_the_nodes_egui_style() {
    let mut style = Style::default();
    let c = |n: u8| Color32::from_rgb(n, n, n);
    let mut input = style.input.clone();
    input.sizes = eggshell_ui::contract::Keyed::new(
        eggshell_ui::contract::Size::DEFAULT,
        eggshell_ui::contract::InputMetrics {
            text: None,
            icon_size: 15.0,
            icon_gap: 7.0,
        },
    );
    input.hint = Some(c(99));
    input.icon = c(55);
    style.input = input;
    style
        .sheet
        .as_app()
        .rule(eggshell_ui::owned::InputPart::Root, move |t, _| {
            t.bg(c(10))
                .border(3.0)
                .border_color(c(20))
                .text(c(30))
                .rounded_px(7.0)
                .h_px(41.0)
                .px_px(9.0)
                .py(1.25)
                .hover(|p| p.border_color(c(21)))
                .focus(|p| p.border_color(c(23)))
        });
    style.selection.fill = c(77);
    style.selection.text = Some(c(88));

    let mut egui_style = egui::Style::default();
    style.map_input(&mut egui_style, eggshell_ui::contract::Size::DEFAULT);
    let v = &egui_style.visuals;
    assert_eq!(v.text_edit_bg_color, Some(c(10)));
    assert_eq!(v.widgets.inactive.bg_stroke, egui::Stroke::new(3.0, c(20)));
    assert_eq!(v.widgets.hovered.bg_stroke, egui::Stroke::new(3.0, c(21)));
    assert_eq!(v.widgets.inactive.fg_stroke.color, c(30));
    assert_eq!(
        v.widgets.inactive.corner_radius,
        egui::CornerRadius::same(7)
    );
    assert_eq!(v.widgets.hovered.expansion, 0.0);
    assert_eq!(v.weak_text_color, Some(c(99)));
    assert_eq!(v.selection.bg_fill, c(77));
    assert_eq!(
        v.selection.stroke,
        egui::Stroke::new(3.0, c(23)),
        "the focus border is the focused one, whatever the selected text's colour"
    );
    assert_eq!(
        egui_style.spacing.button_padding,
        vec2(9.0, 5.0),
        "`py-1.25` at 4 points a unit"
    );
    assert_eq!(egui_style.spacing.interact_size.y, 41.0);
}

#[test]
fn the_input_frame_is_drawn_from_the_sheets_rule() {
    use eggshell_ui::owned::InputPart;
    let mut style = Style::default();
    style.sheet.as_app().rule(InputPart::Root, |t, _| {
        t.bg(Color32::from_rgb(1, 2, 3))
            .border_color(Color32::from_rgb(4, 5, 6))
    });
    let mut egui_style = egui::Style::default();
    style.map_input(&mut egui_style, eggshell_ui::contract::Size::DEFAULT);
    assert_eq!(
        egui_style.visuals.text_edit_bg_color,
        Some(Color32::from_rgb(1, 2, 3))
    );
    assert_eq!(
        egui_style.visuals.widgets.inactive.bg_stroke.color,
        Color32::from_rgb(4, 5, 6)
    );
}
