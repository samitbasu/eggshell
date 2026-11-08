//! The shadcn preset: what `apply` sets in egui, stock widgets under it, and
//! the library's primitives drawn with its proportions.

use bullseye::prim::segmented::segmented;
use bullseye::prim::switch::Switch;
use bullseye::style::{Accent, ButtonVariant, Shadcn, Tokens, tokens};
use bullseye::widget::{Button, ComboBox, Stock, TextEdit};
use bullseye::{Builder, LayoutState, TextSpec, TextStyle, Width, tw};
use egui::epaint::Shape;
use egui::{Color32, Context, FontFamily, Rect, Theme, Vec2, vec2};
use egui_kittest::Harness;

mod support;

use support::{Rig, click_node, hover_over, rect_of};

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

fn context(theme: Theme) -> Context {
    let ctx = Context::default();
    ctx.set_theme(theme);
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

#[test]
fn applying_the_light_preset_maps_tokens_onto_egui() {
    let ctx = context(Theme::Light);
    let untouched = ctx.style_of(Theme::Dark);
    Shadcn::light().apply(&ctx);
    let style = ctx.style_of(Theme::Light);
    let v = &style.visuals;
    let s = &style.spacing;

    assert!(!v.dark_mode);
    assert_eq!(v.panel_fill, Color32::WHITE);
    assert_eq!(v.window_fill, Color32::WHITE);
    assert_eq!(v.faint_bg_color, rgb(0xf5, 0xf5, 0xf5));
    assert_eq!(v.error_fg_color, rgb(0xe7, 0x00, 0x0b));
    assert_eq!(
        v.widgets.noninteractive.fg_stroke.color,
        rgb(0x0a, 0x0a, 0x0a)
    );
    assert_eq!(
        v.widgets.noninteractive.bg_stroke.color,
        rgb(0xe5, 0xe5, 0xe5)
    );
    assert_eq!(v.widgets.inactive.weak_bg_fill, rgb(0xf5, 0xf5, 0xf5));
    assert_eq!(v.widgets.inactive.bg_fill, rgb(0xe5, 0xe5, 0xe5));
    assert_eq!(v.widgets.inactive.fg_stroke.color, rgb(0x17, 0x17, 0x17));
    assert_eq!(
        v.selection.bg_fill,
        rgb(0x17, 0x17, 0x17).gamma_multiply(0.3),
        "primary/30, under text drawn in foreground"
    );
    assert_eq!(
        v.selection.stroke.color,
        rgb(0x0a, 0x0a, 0x0a),
        "foreground: the focus border and selected text"
    );
    let row = s.interact_size.y + s.item_spacing.y;
    assert!(
        s.combo_height >= 9.0 * row - s.item_spacing.y,
        "nine select rows fit the popup"
    );
    for widget in [
        v.widgets.noninteractive,
        v.widgets.inactive,
        v.widgets.hovered,
        v.widgets.active,
        v.widgets.open,
    ] {
        assert_eq!(
            widget.corner_radius,
            egui::CornerRadius::same(8),
            "rounded-md"
        );
        assert_eq!(widget.expansion, 0.0);
    }
    assert_eq!(v.window_corner_radius, egui::CornerRadius::same(10));

    assert_eq!(s.interact_size.y, 36.0);
    assert_eq!(s.button_padding, vec2(16.0, 8.0));
    assert_eq!(s.icon_width, 16.0, "checkbox: size-4");
    assert_eq!(
        style.text_styles[&egui::TextStyle::Body].size,
        14.0,
        "text-sm"
    );

    assert_eq!(
        *ctx.style_of(Theme::Dark),
        *untouched,
        "applying one theme leaves the other alone"
    );
}

#[test]
fn applying_the_dark_preset_maps_the_dark_tokens() {
    let ctx = context(Theme::Dark);
    Shadcn::dark().apply(&ctx);
    let v = &ctx.style_of(Theme::Dark).visuals;
    assert!(v.dark_mode);
    assert_eq!(v.panel_fill, rgb(0x0a, 0x0a, 0x0a));
    assert_eq!(v.window_fill, rgb(0x17, 0x17, 0x17));
    assert_eq!(v.widgets.inactive.weak_bg_fill, rgb(0x26, 0x26, 0x26));
    assert_eq!(
        v.selection.bg_fill,
        rgb(0xe5, 0xe5, 0xe5).gamma_multiply(0.3)
    );
    assert_eq!(v.selection.stroke.color, rgb(0xfa, 0xfa, 0xfa));
    assert_eq!(v.error_fg_color, rgb(0xff, 0x64, 0x67));
    assert_eq!(
        v.widgets.noninteractive.bg_stroke.color,
        Color32::from_white_alpha(26),
        "border is white at 10%"
    );
    assert_eq!(v.widgets.inactive.bg_fill, Color32::from_white_alpha(38));
}

#[test]
fn every_token_can_be_overridden() {
    let ctx = context(Theme::Light);
    let mut preset = Shadcn::light();
    preset.tokens.primary = rgb(0x12, 0x34, 0x56);
    preset.tokens.radius = 6;
    preset.tokens.control_height = 40.0;
    preset.apply(&ctx);
    let style = ctx.style_of(Theme::Light);
    assert_eq!(
        style.visuals.selection.bg_fill,
        rgb(0x12, 0x34, 0x56).gamma_multiply(0.3)
    );
    assert_eq!(
        style.visuals.widgets.inactive.corner_radius,
        egui::CornerRadius::same(4)
    );
    assert_eq!(style.spacing.interact_size.y, 40.0);
    assert_eq!(
        tokens(&ctx).expect("remembered").primary,
        rgb(0x12, 0x34, 0x56)
    );
}

#[test]
fn the_preset_is_picked_by_theme_and_nothing_applies_unasked() {
    assert_eq!(Shadcn::for_theme(Theme::Light), Shadcn::light());
    assert_eq!(Shadcn::for_theme(Theme::Dark), Shadcn::dark());
    assert_ne!(Tokens::light(), Tokens::dark());

    let ctx = context(Theme::Light);
    let before = ctx.global_style();
    let mut layout = LayoutState::new();
    layout
        .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
            b.text(tw(), "x");
        })
        .layout(Rect::from_min_size(egui::Pos2::ZERO, vec2(100.0, 100.0)));
    assert_eq!(*ctx.global_style(), *before, "building changes no style");
    assert!(tokens(&ctx).is_none(), "and remembers no preset");

    Shadcn::apply_both(&ctx);
    assert_eq!(
        tokens(&ctx).expect("light").background,
        Color32::WHITE,
        "light theme"
    );
    ctx.set_theme(Theme::Dark);
    assert_eq!(
        tokens(&ctx).expect("dark").background,
        rgb(0x0a, 0x0a, 0x0a),
        "the tokens follow the context's theme"
    );
}

#[test]
fn a_stock_button_is_shadcns_h9_at_14pt() {
    for theme in [Theme::Light, Theme::Dark] {
        let ctx = context(theme);
        Shadcn::for_theme(theme).apply(&ctx);
        let mut size = Vec2::ZERO;
        let mut text_width = 0.0;
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let galley = egui::WidgetText::from("Save").into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Button,
                );
                text_width = galley.size().x;
                size = ui.add(egui::Button::new("Save")).rect.size();
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(size.y, 36.0, "{theme:?}: h-9");
        assert_eq!(size.x, text_width + 32.0, "{theme:?}: px-4");
        assert_eq!(
            Button::new("Save").size(&ctx, &base_text(), None),
            size,
            "{theme:?}: the estimate agrees"
        );
    }
}

#[test]
fn inputs_and_selects_take_the_input_proportions() {
    for theme in [Theme::Light, Theme::Dark] {
        let ctx = context(theme);
        Shadcn::for_theme(theme).apply(&ctx);
        let text = base_text();
        let field = TextEdit::singleline("").size(&ctx, &text, Some(Width::new(200.0)));
        assert_eq!(field, vec2(200.0, 36.0), "{theme:?}");
        let select = ComboBox::new("A").size(&ctx, &text, None);
        assert_eq!(select.y, 36.0, "{theme:?}");
    }
}

#[derive(Default)]
struct App {
    on: bool,
    unit: Unit,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
enum Unit {
    #[default]
    Metric,
    Imperial,
    Nautical,
}

const UNITS: [(Unit, &str); 3] = [
    (Unit::Metric, "Metric"),
    (Unit::Imperial, "Imperial"),
    (Unit::Nautical, "Nautical"),
];

fn prepare_light(ctx: &Context, _: &App) {
    ctx.set_theme(Theme::Light);
    Shadcn::light().apply(ctx);
}

fn harness_of(view: support::View<App>) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness_prepared(
        vec2(400.0, 300.0),
        base_text(),
        App::default(),
        view,
        prepare_light,
    );
    harness.run_steps(3);
    harness
}

fn switches(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0).p(2.0), |b| {
        for (key, classes) in [
            ("default", tw()),
            ("sized", tw().w_px(48.0).h_px(28.0)),
            ("wide", tw().w_px(64.0)),
            ("tall", tw().h_px(36.0)),
        ] {
            b.widget(
                key,
                classes,
                Switch::unlabelled().named(key),
                |app, ui, switch| {
                    switch.show(ui, &mut app.on);
                },
            );
        }
    });
}

fn thumb_in(harness: &Harness<'_, Rig<App>>, rect: Rect) -> egui::epaint::CircleShape {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Circle(circle) if rect.expand(1.0).contains(circle.center) => Some(*circle),
            _ => None,
        })
        .next_back()
        .unwrap_or_else(|| panic!("no thumb in {rect:?}"))
}

#[test]
fn a_switch_follows_its_classes_and_the_thumb_and_travel_scale() {
    let mut harness = harness_of(switches);
    let tokens = Tokens::light();
    let default = rect_of(&harness, "/\"default\"");
    assert_eq!(default.width(), 32.0, "w-8");
    assert!((default.height() - 18.4).abs() < 0.01, "h-[1.15rem]");

    let cases = [
        ("default", vec2(32.0, 18.4)),
        ("sized", vec2(48.0, 28.0)),
        ("wide", vec2(64.0, 64.0 * 18.4 / 32.0)),
        ("tall", vec2(36.0 * 32.0 / 18.4, 36.0)),
    ];
    for (key, track) in cases {
        let rect = rect_of(&harness, &format!("/{key:?}"));
        assert!(
            (rect.size() - track).abs().max_elem() < 0.01,
            "{key}: leaf {rect:?} for track {track:?}"
        );
    }

    for on in [false, true] {
        harness.state_mut().app.on = on;
        harness.run_steps(8);
        for (key, track) in cases {
            let rect = rect_of(&harness, &format!("/{key:?}"));
            let thumb = thumb_in(&harness, rect);
            let radius = tokens.switch_thumb_for(rect.size()) / 2.0;
            assert!(
                (thumb.radius - radius).abs() < 0.25,
                "{key}: thumb {} for {radius}",
                thumb.radius
            );
            let half_track = rect.height() / 2.0;
            let expected_x = if on {
                rect.right() - half_track
            } else {
                rect.left() + half_track
            };
            assert!(
                (thumb.center.x - expected_x).abs() < 0.6,
                "{key}, on {on}: thumb at {} for {expected_x} (track {track:?})",
                thumb.center.x
            );
            assert!((thumb.center.y - rect.center().y).abs() < 0.6);
        }
    }
    let small = thumb_in(&harness, default).radius;
    let big = thumb_in(&harness, rect_of(&harness, "/\"sized\"")).radius;
    assert!(
        big > small,
        "precondition: the larger track has a larger thumb"
    );
}

fn segments(b: &mut Builder<'_, '_, App>, app: &App) {
    let classes = Shadcn::light().segmented();
    b.div(tw().p(2.0), |b| {
        segmented(
            b,
            "units",
            classes.container(),
            app.unit,
            &UNITS,
            |cell| classes.cell(cell),
            |app| &mut app.unit,
        );
    });
}

fn raised_cells(harness: &Harness<'_, Rig<App>>) -> Vec<Rect> {
    let tokens = Tokens::light();
    let raised = tokens.shadow_sm.color;
    let shapes: Vec<_> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) => Some(rect.clone()),
            _ => None,
        })
        .collect();
    let shadows = shapes
        .iter()
        .filter(|rect| rect.fill == raised && rect.blur_width > 0.0)
        .count();
    let cells: Vec<Rect> = shapes
        .iter()
        .filter(|rect| rect.fill == tokens.background && rect.rect.is_positive())
        .map(|rect| rect.rect)
        .collect();
    assert_eq!(shadows, cells.len(), "each raised cell has its shadow");
    cells
}

#[test]
fn the_shadcn_segmented_classes_raise_exactly_the_pressed_cell() {
    let mut harness = harness_of(segments);
    let track = rect_of(&harness, "/\"units\"");
    assert_eq!(track.height(), 36.0, "h-9");
    let metric = rect_of(&harness, "/\"units\"/Metric");
    assert_eq!(metric.top() - track.top(), 3.0, "p-[3px]");
    assert_eq!(
        track.right() - rect_of(&harness, "/\"units\"/Nautical").right(),
        3.0
    );

    let cells = raised_cells(&harness);
    assert_eq!(cells.len(), 1, "one raised cell");
    assert!(
        cells[0].center().distance(metric.center()) < 1.0,
        "and it is the pressed one: {:?} vs {metric:?}",
        cells[0]
    );

    click_node(&mut harness, "/\"units\"/Imperial");
    harness.run_steps(3);
    assert_eq!(harness.state().app.unit, Unit::Imperial, "precondition");
    let cells = raised_cells(&harness);
    let imperial = rect_of(&harness, "/\"units\"/Imperial");
    assert_eq!(cells.len(), 1, "still one");
    assert!(cells[0].center().distance(imperial.center()) < 1.0);
}

#[test]
fn a_text_field_under_the_preset_is_h9_with_px3() {
    fn field(b: &mut Builder<'_, '_, App>, _: &App) {
        b.widget(
            "field",
            tw().w_px(200.0),
            TextEdit::singleline("").hint("Name").named("Name"),
            |_, ui, field| {
                field.show(ui, &mut String::new());
            },
        );
    }
    let harness = harness_of(field);
    let rect = rect_of(&harness, "/\"field\"");
    assert_eq!(rect.size(), vec2(200.0, 36.0));
    let hint = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "Name" => Some(text.pos),
            _ => None,
        })
        .expect("the hint is painted");
    assert_eq!(hint.x - rect.left(), 12.0, "px-3");
}

#[test]
fn an_accent_retints_primary_and_ring_and_nothing_else() {
    for theme in [Theme::Light, Theme::Dark] {
        let neutral = Shadcn::for_theme(theme);
        assert_eq!(neutral.clone().accent(Accent::Neutral), neutral);
        for accent in Accent::ALL.into_iter().filter(|a| *a != Accent::Neutral) {
            let tinted = neutral.clone().accent(accent);
            let (a, b) = (&neutral.tokens, &tinted.tokens);
            assert_ne!(a.primary, b.primary, "{accent:?} {theme:?}");
            assert_ne!(a.primary_foreground, b.primary_foreground);
            assert_ne!(a.ring, b.ring);
            assert_eq!(b.switch_track_on, b.primary, "the switch follows");
            let same = Tokens {
                primary: a.primary,
                primary_foreground: a.primary_foreground,
                ring: a.ring,
                switch_track_on: a.switch_track_on,
                switch_thumb_on: a.switch_thumb_on,
                ..b.clone()
            };
            assert_eq!(&same, a, "{accent:?} {theme:?}: only those change");
        }
    }
    let blue = |theme| Shadcn::for_theme(theme).accent(Accent::Blue).tokens;
    assert_eq!(blue(Theme::Light).primary, rgb(0x14, 0x47, 0xe6));
    assert_eq!(blue(Theme::Dark).primary, rgb(0x19, 0x3c, 0xb8));
    assert_eq!(blue(Theme::Dark).primary_foreground, rgb(0xef, 0xf6, 0xff));
}

#[test]
fn an_accent_reaches_egui_when_applied() {
    let ctx = context(Theme::Light);
    Shadcn::light().accent(Accent::Violet).apply(&ctx);
    assert_eq!(
        ctx.style_of(Theme::Light).visuals.hyperlink_color,
        rgb(0x70, 0x08, 0xe7)
    );
    assert_eq!(Shadcn::of(&ctx).tokens.primary, rgb(0x70, 0x08, 0xe7));
}

#[test]
fn of_is_the_applied_preset_for_the_contexts_theme() {
    let ctx = context(Theme::Light);
    assert_eq!(
        Shadcn::of(&ctx),
        Shadcn::light(),
        "none applied: the default"
    );
    let green = Shadcn::light().accent(Accent::Green);
    green.apply(&ctx);
    Shadcn::dark().apply(&ctx);
    assert_eq!(Shadcn::of(&ctx), green);
    ctx.set_theme(Theme::Dark);
    assert_eq!(Shadcn::of(&ctx), Shadcn::dark());
}

#[test]
fn semantic_colours_are_defined_in_both_themes() {
    let light = Tokens::light();
    let dark = Tokens::dark();
    assert_eq!(light.success, rgb(0x00, 0xa6, 0x3e), "green-600");
    assert_eq!(light.warning, rgb(0xfe, 0x9a, 0x00), "amber-500");
    assert_eq!(light.info, rgb(0x00, 0x84, 0xd1), "sky-600");
    assert_eq!(dark.success, rgb(0x05, 0xdf, 0x72), "green-400");
    assert_eq!(dark.warning, rgb(0xff, 0xb9, 0x00), "amber-400");
    assert_eq!(dark.info, rgb(0x00, 0xbc, 0xff), "sky-400");
    for t in [light, dark] {
        let all = [
            t.destructive,
            t.success,
            t.warning,
            t.info,
            t.primary,
            t.muted,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
        for (fill, text) in [
            (t.destructive, t.destructive_foreground),
            (t.success, t.success_foreground),
            (t.warning, t.warning_foreground),
            (t.info, t.info_foreground),
        ] {
            assert_ne!(fill, text, "text is told from its fill");
        }
    }
}

#[test]
fn hover_variants_are_translucent_steps_of_their_token() {
    let t = Tokens::light();
    assert_eq!(t.accent_50(), t.accent.gamma_multiply(0.5));
    assert_ne!(t.accent_50(), t.accent, "hover differs from selected");
    assert_ne!(t.muted_50(), t.muted);
    assert_eq!(t.secondary_80().a(), 204);
    assert_eq!(t.primary_90().a(), 230);
    let d = Tokens::dark();
    assert!(d.input_30().a() < d.input_50().a() && d.input_50().a() < d.input.a());
}

fn many_segments(b: &mut Builder<'_, '_, App>, app: &App) {
    let classes = Shadcn::light().segmented().wrapping();
    b.div(tw().w_px(160.0).p(2.0), |b| {
        segmented(
            b,
            "wrapped",
            classes.container(),
            app.unit,
            &[
                (Unit::Metric, "Metric"),
                (Unit::Imperial, "Imperial"),
                (Unit::Nautical, "Nautical"),
            ],
            |cell| classes.cell(cell),
            |app| &mut app.unit,
        );
    });
}

#[test]
fn segmented_classes_fit_their_cells_and_can_wrap() {
    let harness = harness_of(segments);
    let track = rect_of(&harness, "/\"units\"");
    let cells =
        ["Metric", "Imperial", "Nautical"].map(|c| rect_of(&harness, &format!("/\"units\"/{c}")));
    let widest_row = cells.iter().map(|c| c.width()).sum::<f32>() + 6.0;
    assert!(
        track.width() < 395.0,
        "precondition: the window is wider: {track:?}"
    );
    assert!(
        (track.width() - widest_row).abs() < 0.5,
        "w-fit: {} for cells {widest_row}",
        track.width()
    );

    let harness = harness_of(many_segments);
    let track = rect_of(&harness, "/\"wrapped\"");
    let cells =
        ["Metric", "Imperial", "Nautical"].map(|c| rect_of(&harness, &format!("/\"wrapped\"/{c}")));
    assert!(
        cells[2].top() > cells[0].top(),
        "precondition: the cells do not fit one row: {cells:?}"
    );
    assert!(
        track.height() > 36.0,
        "the track grows with its rows: {track:?}"
    );
    assert!(track.width() <= 160.0);
}

fn button_look(theme: Theme, variant: ButtonVariant) -> egui::style::Widgets {
    let ctx = context(theme);
    let preset = Shadcn::for_theme(theme);
    preset.apply(&ctx);
    let mut visuals = ctx.style_of(theme).visuals.clone();
    preset.style_button(variant, &mut visuals);
    visuals.widgets
}

#[test]
fn button_variants_come_from_the_tokens_and_keep_the_size_inputs() {
    for theme in [Theme::Light, Theme::Dark] {
        let t = Tokens::for_theme(theme);
        let base = button_look(theme, ButtonVariant::Secondary);
        assert_eq!(
            base.inactive.weak_bg_fill, t.secondary,
            "secondary is egui's own"
        );
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Destructive,
            ButtonVariant::Outline,
            ButtonVariant::Ghost,
        ] {
            let look = button_look(theme, variant);
            assert_ne!(
                look.inactive.weak_bg_fill, base.inactive.weak_bg_fill,
                "{variant:?} {theme:?} differs from secondary"
            );
            assert_ne!(look.hovered.weak_bg_fill, look.inactive.weak_bg_fill);
        }
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Destructive,
            ButtonVariant::Outline,
            ButtonVariant::Ghost,
        ] {
            let (new, old) = (button_look(theme, variant), &base);
            for (a, b) in [
                (new.noninteractive, old.noninteractive),
                (new.inactive, old.inactive),
                (new.hovered, old.hovered),
                (new.active, old.active),
                (new.open, old.open),
            ] {
                assert_eq!(a.bg_stroke.width, b.bg_stroke.width, "{variant:?}");
                assert_eq!(a.expansion, b.expansion);
                assert_eq!(a.corner_radius, b.corner_radius);
            }
        }
        let primary = button_look(theme, ButtonVariant::Primary);
        assert_eq!(primary.inactive.weak_bg_fill, t.primary);
        assert_eq!(primary.inactive.fg_stroke.color, t.primary_foreground);
        assert_eq!(primary.hovered.weak_bg_fill, t.primary_90());
        let destructive = button_look(theme, ButtonVariant::Destructive);
        assert_eq!(destructive.hovered.weak_bg_fill, t.destructive_90());
        assert_eq!(destructive.inactive.fg_stroke.color, Color32::WHITE);
        let outline = button_look(theme, ButtonVariant::Outline);
        assert_eq!(outline.inactive.bg_stroke.color, t.input);
        let ghost = button_look(theme, ButtonVariant::Ghost);
        assert_eq!(ghost.inactive.weak_bg_fill, Color32::TRANSPARENT);
        assert_ne!(ghost.hovered.weak_bg_fill, Color32::TRANSPARENT);
    }
}

fn variant_controls(b: &mut Builder<'_, '_, App>, app: &App) {
    use bullseye::widget::{Checkbox, RadioButton};
    b.div(tw().flex().flex_col().items_start().gap(2.0).p(2.0), |b| {
        b.widget(
            "primary",
            tw(),
            Button::new("Save").primary(),
            |_, ui, w| {
                w.show(ui);
            },
        );
        b.widget(
            "disabled",
            tw(),
            Button::new("Send").primary(),
            |_, ui, w| {
                ui.disable();
                w.show(ui);
            },
        );
        b.widget("ghost", tw(), Button::new("Cancel").ghost(), |_, ui, w| {
            w.show(ui);
        });
        let on = app.on;
        b.widget("box", tw(), Checkbox::new("Agree"), move |app, ui, w| {
            w.show(ui, &mut app.on);
        });
        b.widget("radio", tw(), RadioButton::new("Pick"), move |_, ui, w| {
            w.show(ui, on);
        });
    });
}

fn rects_filled(harness: &Harness<'_, Rig<App>>, fill: Color32) -> Vec<Rect> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.fill == fill && rect.rect.is_positive() => Some(rect.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn variants_boxes_and_dots_follow_the_tokens_through_the_real_path() {
    let t = Tokens::light();
    let mut harness = harness_of(variant_controls);

    let save = rect_of(&harness, "/\"primary\"");
    let fills = rects_filled(&harness, t.primary);
    assert!(
        fills.iter().any(|r| (r.min - save.min).length() < 1.0
            && (r.size() - save.size()).abs().max_elem() < 1.0),
        "the primary button is filled with primary: {fills:?} for {save:?}"
    );
    let text_color =
        |text: &str| {
            harness
                .output()
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    Shape::Text(shape) if shape.galley.text() == text => {
                        let drawn = shape.galley.job.sections[0].format.color;
                        Some(shape.override_text_color.unwrap_or(
                            if drawn == Color32::PLACEHOLDER {
                                shape.fallback_color
                            } else {
                                drawn
                            },
                        ))
                    }
                    _ => None,
                })
        };
    assert_eq!(
        text_color("Save"),
        Some(t.primary_foreground),
        "the label is drawn in the variant's text colour, not the node's"
    );
    assert_eq!(text_color("Cancel"), Some(t.foreground));
    let send = rect_of(&harness, "/\"disabled\"");
    let faded = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if (rect.rect.min - send.min).length() < 1.0 => Some(rect.fill),
            _ => None,
        })
        .next_back()
        .expect("the disabled button has a fill");
    assert!(
        (i32::from(faded.a()) - 128).abs() <= 2 && faded.r() <= t.primary.r() + 1,
        "a disabled button is its variant at half opacity: {faded:?}"
    );
    let cancel = rect_of(&harness, "/\"ghost\"");
    let ghost_fills = rects_filled(&harness, t.accent);
    assert!(
        ghost_fills
            .iter()
            .all(|r| (r.min - cancel.min).length() >= 1.0),
        "a resting ghost button has no fill"
    );
    hover_over(&mut harness, cancel.center());
    assert!(
        rects_filled(&harness, t.accent)
            .iter()
            .any(|r| (r.min - cancel.min).length() < 1.0),
        "and takes accent when hovered"
    );

    let boxes = |harness: &Harness<'_, Rig<App>>, fill| {
        rects_filled(harness, fill)
            .into_iter()
            .filter(|r| r.size() == vec2(16.0, 16.0))
            .count()
    };
    assert_eq!(
        boxes(&harness, t.background),
        1,
        "an unticked box is background"
    );
    assert_eq!(boxes(&harness, t.primary), 0);
    harness.state_mut().app.on = true;
    harness.run_steps(3);
    assert_eq!(boxes(&harness, t.primary), 1, "a ticked box is primary");
    let tick = |harness: &Harness<'_, Rig<App>>| {
        harness
            .output()
            .shapes
            .iter()
            .filter(|clipped| match &clipped.shape {
                Shape::Path(path) => {
                    path.stroke.color == egui::epaint::ColorMode::Solid(t.primary_foreground)
                }
                _ => false,
            })
            .count()
    };
    assert_eq!(tick(&harness), 1, "with a primary-foreground tick");
}
