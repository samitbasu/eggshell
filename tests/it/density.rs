//! A theme's metric tokens resize the generated looks and the call site's
//! own classes, with no application rule (#487).

use eggshell_ui::prelude::*;
use std::sync::Arc;

use eggshell_ui::contract::{Radius, Spacing, TextSize, Themed};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, ShadcnStyle, Tokens, size};
use eggshell_ui::{Builder, Icon, Match, TextStyle, icons, tw};
use egui::epaint::Shape;
use egui::{Rect, vec2};

use crate::support;

const DENSE_TEXT: TextSize = TextSize {
    size: 13.0,
    line_height: 18.0,
};

fn dense(tokens: &mut Tokens) {
    tokens.spacing = Spacing::new(3.5);
    tokens.radius = Radius::new(8.0);
    tokens.text.xs = DENSE_TEXT;
    tokens.text.sm = DENSE_TEXT;
}

fn styled(style: ShadcnStyle, edit: fn(&mut Tokens)) -> Themed<Arc<eggshell_ui::Style>> {
    let half = |mut preset: Shadcn| {
        preset = preset.with_style(style);
        edit(&mut preset.tokens);
        Arc::new(preset.style())
    };
    Themed {
        light: half(Shadcn::light()),
        dark: half(Shadcn::dark()),
    }
}

fn controls(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().items_start().gap(2.0), |s| {
        s.div_id("Padded", tw().p(4.0), |_| {});
        s.div_id("Pointed", tw().p_px(4.0), |_| {});
        s.div_id("Parent", tw().children(Match::any(), |t| t.p(4.0)), |s| {
            s.div_id("Child", tw(), |_| {});
        });
        s.div_id("Holder", tw().has(Match::any(), |b| b.p(4.0)), |s| {
            s.div_id("Held", tw(), |_| {});
        });
        s.scroll("Scroller", tw().h_px(40.0).p(4.0)).children(|s| {
            s.div_id("Inside", tw().h_px(1.0), |_| {});
        });
        s.button("save", "Save").on_click(|_| {});
        s.switch("air", "Airplane mode", false).on_change(|_, _| {});
        s.text_field("field", "Typed")
            .named("Field")
            .on_change(|_, _| {});
        s.text_field("small", "Small")
            .named("Small")
            .size(size::SM)
            .on_change(|_, _| {});
        s.text_field("found", "Found")
            .named("Found")
            .size(size::SM)
            .leading(Icon::new(icons::SEARCH))
            .on_change(|_, _| {});
    });
}

fn driver(style: ShadcnStyle, edit: fn(&mut Tokens)) -> Driver<()> {
    let tokens = Tokens::light().in_style(style);
    let mut d = support::driver(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        controls,
    )
    .style(&eggshell_ui::SharedStyle::new(styled(style, edit)))
    .build();
    d.frames(4);
    d
}

fn rect(d: &Driver<()>, path: &str) -> Rect {
    d.rect(Target::suffix(path))
}

/// The font size of the text `text` is drawn in inside `inside`.
fn font_size(d: &Driver<()>, inside: Rect, text: &str) -> f32 {
    support::shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == text && inside.contains(t.pos) => {
                Some(t.galley.job.sections[0].format.font_id.size)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {text:?} drawn in {inside:?}"))
}

#[test]
fn a_dense_lyra_button_is_28_points_tall_in_13_point_text() {
    let d = driver(ShadcnStyle::Lyra, dense);
    let button = rect(&d, "/\"save\"");
    assert_eq!(button.height(), 28.0, "h-8 at 3.5 points a unit");
    assert_eq!(font_size(&d, button, "Save"), 13.0, "text-xs");
}

#[test]
fn a_dense_vega_input_has_6_point_corners_and_draws_13_point_text() {
    let d = driver(ShadcnStyle::Vega, dense);
    let field = rect(&d, "/\"field\"");
    let frame = support::box_at(&d, field);
    assert_eq!(frame.corner_radius.nw, 6, "rounded-md: the radius less 2");
    assert_eq!(
        font_size(&d, field, "Typed"),
        13.0,
        "the TextEdit's own glyphs"
    );
    assert_eq!(field.height(), 31.5, "h-9 at 3.5 points a unit");
}

#[test]
fn a_dense_small_field_is_a_unit_shorter_and_padded_in_units() {
    let d = driver(ShadcnStyle::Vega, dense);
    assert_eq!(rect(&d, "/\"small\"").height(), 28.0, "h-8 at 3.5 a unit");
    let (boxed, icon) = (
        rect(&d, "/\"found\""),
        rect(&d, "/\"found\"/LeadingAddon/Leading"),
    );
    assert_eq!(boxed.height(), 28.0, "the box is the field's height");
    let border = 1.0;
    assert_eq!(
        icon.left() - boxed.left(),
        border + 3.0 * 3.5,
        "own_input_padding_x, 3 units at 3.5"
    );
}

#[test]
fn a_step_left_at_its_default_keeps_its_size() {
    let d = driver(ShadcnStyle::Lyra, |tokens| {
        dense(tokens);
        tokens.text.xs = Tokens::light().text.xs;
    });
    let button = rect(&d, "/\"save\"");
    assert_eq!(font_size(&d, button, "Save"), 12.0, "text-xs, untouched");
}

#[test]
fn a_call_sites_spacing_units_follow_the_theme_and_its_points_do_not() {
    let d = driver(ShadcnStyle::Vega, dense);
    assert_eq!(rect(&d, "/\"Padded\"").height(), 28.0, "p-4: 14 a side");
    assert_eq!(rect(&d, "/\"Pointed\"").height(), 8.0, "4 points a side");
}

#[test]
fn the_presets_unit_tokens_scale_the_metrics_they_feed() {
    let d = driver(ShadcnStyle::Vega, dense);
    let style = d.shell().style();
    let u = |units: f32| units * 3.5;
    assert_eq!(style.menu.padding, u(1.0), "p-1");
    assert_eq!(style.menu.item_padding, vec2(u(2.0), u(1.5)), "px-2 py-1.5");
    let slider = style.slider.sizes.get(eggshell_ui::contract::Size::DEFAULT);
    assert_eq!(
        (slider.track_height, slider.thumb_size),
        (u(1.5), u(4.0)),
        "h-1.5, size-4"
    );
    assert_eq!(
        style.tokens.points_vec(style.tokens.button_padding),
        vec2(u(4.0), u(2.0)),
        "px-4 py-2, as egui's button padding"
    );
}

#[test]
fn the_default_tokens_lay_out_as_before() {
    let d = driver(ShadcnStyle::Lyra, |_| {});
    assert_eq!(rect(&d, "/\"save\"").height(), 32.0);
    assert_eq!(rect(&d, "/\"Padded\"").height(), 32.0);
}

#[test]
fn rules_a_node_gets_from_its_parent_or_its_children_scale_too() {
    let d = driver(ShadcnStyle::Vega, dense);
    assert_eq!(
        rect(&d, "/\"Child\"").height(),
        28.0,
        "a children rule's p-4"
    );
    assert_eq!(rect(&d, "/\"Holder\"").height(), 28.0, "a has rule's p-4");
}

#[test]
fn a_scroll_containers_padding_scales() {
    let d = driver(ShadcnStyle::Vega, dense);
    let inset = rect(&d, "/\"Inside\"").min.y - rect(&d, "/\"Scroller\"").min.y;
    assert_eq!(inset, 14.0, "p-4 on its content");
}

#[test]
fn new_tokens_on_the_same_sheet_lay_out_again() {
    let mut d = driver(ShadcnStyle::Vega, |_| {});
    let field = rect(&d, "/\"field\"");
    assert_eq!(support::box_at(&d, field).corner_radius.nw, 8);
    assert_eq!(rect(&d, "/\"Padded\"").height(), 32.0);
    let mut edited = (*d.shell().style()).clone();
    let shared = edited.sheet.clone();
    edited.tokens.spacing = Spacing::new(3.5);
    edited.tokens.radius = Radius::new(8.0);
    let edited = Arc::new(edited);
    d.set_style(Themed {
        light: Arc::clone(&edited),
        dark: edited,
    });
    d.frames(4);
    assert!(
        d.shell().style().sheet == shared,
        "precondition: the sheet is the one resolved before"
    );
    assert_eq!(rect(&d, "/\"Padded\"").height(), 28.0);
    let field = rect(&d, "/\"field\"");
    assert_eq!(support::box_at(&d, field).corner_radius.nw, 6);
}

#[test]
fn a_dense_switch_thumb_is_its_size_4_at_the_theme_spacing() {
    let d = driver(ShadcnStyle::Vega, dense);
    let track = rect(&d, "/\"air\"/Track");
    let thumb = support::shapes(&d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Circle(circle) if track.expand(0.5).contains(circle.center) => Some(*circle),
            _ => None,
        })
        .next_back()
        .unwrap_or_else(|| panic!("no thumb in {track:?}"));
    assert_eq!(thumb.radius * 2.0, 14.0, "size-4 at 3.5 points a unit");
}
