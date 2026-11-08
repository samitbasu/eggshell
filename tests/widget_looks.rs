//! How the stock widgets look under the shadcn preset, through the real draw
//! path: the shapes egui emits for a slider and a selectable button, in each
//! theme, with the pointer where the test puts it.

use bullseye::style::{Mark, Shadcn, Tokens};
use bullseye::widget::{Button, Slider};
use bullseye::{Builder, LayoutState, tw};
use egui::epaint::{CircleShape, RectShape, Shape};
use egui::{Color32, Pos2, Rect, Theme, vec2};
use egui_kittest::Harness;

const THEMES: [Theme; 2] = [Theme::Light, Theme::Dark];

#[derive(Default)]
struct App {
    value: f64,
}

struct Rig {
    app: App,
    layout: LayoutState,
    nodes: Vec<(String, Rect)>,
}

type View = for<'b, 'f> fn(&mut Builder<'b, 'f, App>);

fn harness(theme: Theme, view: View) -> Harness<'static, Rig> {
    let mut rig = Rig {
        app: App::default(),
        layout: LayoutState::new().with_base_text(common_text(theme)),
        nodes: Vec::new(),
    };
    rig.app.value = 0.5;
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, 200.0))
        .with_theme(theme)
        .build_ui_state(
            move |ui, rig: &mut Rig| {
                Shadcn::apply_both(ui.ctx());
                let area = ui.max_rect();
                let Rig { app, layout, nodes } = rig;
                let placed = layout.build(ui.ctx(), |b| view(b)).layout(area);
                *nodes = placed
                    .nodes()
                    .map(|node| (node.path().to_owned(), node.rect()))
                    .collect();
                placed.draw(app, ui);
            },
            rig,
        );
    harness.run_steps(3);
    harness
}

fn common_text(theme: Theme) -> bullseye::TextStyle {
    let tokens = Tokens::for_theme(theme);
    bullseye::TextStyle::new(tokens.text_role(), tokens.foreground)
}

fn rect_of(harness: &Harness<'_, Rig>, path: &str) -> Rect {
    harness
        .state()
        .nodes
        .iter()
        .find(|(candidate, _)| candidate == path)
        .unwrap_or_else(|| panic!("no node {path}"))
        .1
}

fn hover(harness: &mut Harness<'_, Rig>, at: Pos2) {
    harness.hover_at(at);
    harness.run_steps(3);
}

fn flatten<'a>(shape: &'a Shape, into: &mut Vec<&'a Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| flatten(shape, into)),
        shape => into.push(shape),
    }
}

/// Every shape of the last frame.
fn shapes<'h>(harness: &'h Harness<'_, Rig>) -> Vec<&'h Shape> {
    let mut all = Vec::new();
    for clipped in &harness.output().shapes {
        flatten(&clipped.shape, &mut all);
    }
    all
}

fn rects(harness: &Harness<'_, Rig>) -> Vec<RectShape> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) => Some(rect.clone()),
            _ => None,
        })
        .collect()
}

fn circles(harness: &Harness<'_, Rig>) -> Vec<CircleShape> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Circle(circle) => Some(*circle),
            _ => None,
        })
        .collect()
}

/// The colour each text shape starting inside `area` is drawn in.
fn text_colours(harness: &Harness<'_, Rig>, area: Rect) -> Vec<Color32> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Text(text) if area.contains(text.pos) => {
                Some(text.override_text_color.unwrap_or(text.fallback_color))
            }
            _ => None,
        })
        .collect()
}

fn slider_view(b: &mut Builder<'_, '_, App>) {
    b.div(tw().flex().flex_col().gap(8.0).p(8.0), |b| {
        b.widget(
            "slider",
            tw().w_px(200.0),
            Slider::new().named("Level"),
            |app: &mut App, ui, slider| {
                slider.show(ui, &mut app.value, 0.0..=1.0);
            },
        );
        b.widget(
            "button",
            tw().self_start(),
            Button::new("Save"),
            |_, ui, b| {
                b.show(ui);
            },
        );
    });
}

fn button_only_view(b: &mut Builder<'_, '_, App>) {
    b.div(tw().flex().flex_col().gap(8.0).p(8.0), |b| {
        b.widget(
            "button",
            tw().self_start(),
            Button::new("Save"),
            |_, ui, b| {
                b.show(ui);
            },
        );
    });
}

fn button_look(harness: &Harness<'_, Rig>) -> (Vec<Color32>, Vec<Color32>) {
    let area = rect_of(harness, "/\"button\"");
    let fills = rects(harness)
        .into_iter()
        .filter(|rect| area.expand(0.5).contains_rect(rect.rect))
        .map(|rect| rect.fill)
        .collect();
    (fills, text_colours(harness, area))
}

#[test]
fn the_thumb_is_a_circle_filled_with_background_and_outlined_in_primary() {
    for theme in THEMES {
        let t = Tokens::for_theme(theme);
        let mut harness = harness(theme, slider_view);
        let rail = rect_of(&harness, "/\"slider\"");
        for pointer in [None, Some(rail.center()), Some(rail.right_center())] {
            if let Some(at) = pointer {
                hover(&mut harness, at);
            }
            let thumbs = circles(&harness);
            assert_eq!(thumbs.len(), 1, "{theme:?} {pointer:?}: one thumb");
            let thumb = thumbs[0];
            assert_eq!(thumb.fill, t.background, "{theme:?} {pointer:?}");
            assert_eq!(thumb.fill.a(), 255, "{theme:?}: opaque");
            assert_eq!(thumb.stroke.color, t.primary, "{theme:?} {pointer:?}");
            assert_eq!(thumb.stroke.width, 1.0, "{theme:?}");
            assert_eq!(thumb.radius * 2.0, t.slider_thumb, "{theme:?}: size-4");

            let fills: Vec<Color32> = rects(&harness)
                .into_iter()
                .filter(|rect| rail.expand(0.5).contains_rect(rect.rect))
                .map(|rect| rect.fill)
                .collect();
            assert!(fills.contains(&t.muted), "{theme:?}: the track is muted");
            assert!(fills.contains(&t.primary), "{theme:?}: trailing fill");
        }
    }
}

#[test]
fn the_slider_is_as_tall_as_its_thumb_asks() {
    for theme in THEMES {
        let t = Tokens::for_theme(theme);
        let harness = harness(theme, slider_view);
        assert_eq!(rect_of(&harness, "/\"slider\"").height(), t.slider_height());
    }
}

#[test]
fn a_slider_leaves_every_other_widgets_look_alone() {
    for theme in THEMES {
        let alone = harness(theme, button_only_view);
        let beside = harness(theme, slider_view);
        let (fills, text) = button_look(&alone);
        assert!(
            !text.is_empty(),
            "{theme:?}: precondition: the label is drawn"
        );
        assert_eq!(button_look(&beside), (fills, text), "{theme:?}");

        let t = Tokens::for_theme(theme);
        let visuals = &beside.ctx.global_style().visuals;
        assert_eq!(visuals.selection.bg_fill, t.primary.gamma_multiply(0.3));
        assert_eq!(visuals.widgets.inactive.bg_fill, t.input);
        assert_eq!(
            visuals.widgets.hovered.fg_stroke.color,
            t.secondary_foreground
        );
    }
}

fn toggle_view(mark: Mark, b: &mut Builder<'_, '_, App>) {
    b.div(tw().flex().flex_col().gap(8.0).p(8.0), |b| {
        b.widget(
            "toggle",
            tw().self_start(),
            Button::selectable("Bold", mark),
            |_, ui, button| {
                button.show(ui);
            },
        );
    });
}

fn toggle_off(b: &mut Builder<'_, '_, App>) {
    toggle_view(Mark::Off, b);
}

fn toggle_on(b: &mut Builder<'_, '_, App>) {
    toggle_view(Mark::On, b);
}

/// The fill painted under the toggle's label, if any, and the label's colour.
fn toggle_look(harness: &Harness<'_, Rig>) -> (Option<Color32>, Color32) {
    let area = rect_of(harness, "/\"toggle\"");
    let fill = rects(harness)
        .into_iter()
        .filter(|rect| rect.fill != Color32::TRANSPARENT)
        .find(|rect| area.expand(0.5).contains_rect(rect.rect))
        .map(|rect| rect.fill);
    let text = text_colours(harness, area);
    assert_eq!(text.len(), 1, "precondition: one label is drawn");
    (fill, text[0])
}

#[test]
fn a_selectable_button_is_a_shadcn_toggle() {
    for theme in THEMES {
        let t = Tokens::for_theme(theme);

        let mut off = harness(theme, toggle_off);
        assert_eq!(toggle_look(&off), (None, t.foreground), "{theme:?}: rest");
        let centre = rect_of(&off, "/\"toggle\"").center();
        hover(&mut off, centre);
        assert_eq!(
            toggle_look(&off),
            (Some(t.muted), t.muted_foreground),
            "{theme:?}: hover"
        );

        let mut on = harness(theme, toggle_on);
        assert_eq!(
            toggle_look(&on),
            (Some(t.accent), t.accent_foreground),
            "{theme:?}: selected"
        );
        let centre = rect_of(&on, "/\"toggle\"").center();
        hover(&mut on, centre);
        assert_eq!(
            toggle_look(&on),
            (Some(t.accent), t.accent_foreground),
            "{theme:?}: selected keeps its fill under the pointer"
        );

        let selection = &on.ctx.global_style().visuals.selection;
        assert_eq!(
            selection.bg_fill,
            t.primary.gamma_multiply(0.3),
            "{theme:?}"
        );
        assert_eq!(selection.stroke.color, t.foreground, "{theme:?}");
    }
}
