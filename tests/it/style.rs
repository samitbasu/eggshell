//! TODO(prose): the shadcn preset: what a layout of it gives egui, the text edit under it, and
//! the library's primitives drawn with its proportions.

use eggshell_ui::theme::{BaseColor, Palette, Shadcn, ThemeColor, Tokens};
use eggshell_ui::widget::{Stock, TextEdit};
use eggshell_ui::{Builder, LayoutState, TextSpec, TextStyle, Weight, Width, tw};
use egui::epaint::Shape;
use egui::{Color32, Context, FontFamily, Rect, Theme, vec2};

use crate::support;

use eggshell_ui::testing::{Driver, Target};

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

fn context(theme: Theme) -> Context {
    let ctx = Context::default();
    ctx.set_theme(theme);
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

/// A layout given `preset`'s style, built once on `ctx`: what it gives egui.
fn given(ctx: &Context, preset: &Shadcn) -> LayoutState {
    let half = std::sync::Arc::new(preset.style());
    let mut layout = LayoutState::new(&eggshell_ui::SharedStyle::new(
        eggshell_ui::contract::Themed::both(half),
    ));
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let _ = layout.build(ui.ctx(), |_: &mut Builder<'_, '_, ()>| {});
    })
    .drop_without_applying_deltas();
    layout
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

#[test]
fn applying_the_light_preset_maps_tokens_onto_egui() {
    let ctx = context(Theme::Light);
    given(&ctx, &Shadcn::light());
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
    assert_eq!(v.handle_shape, egui::style::HandleShape::Circle);
    assert!(v.slider_trailing_fill);

    assert_eq!(s.interact_size.y, 36.0);
    assert_eq!(s.button_padding, vec2(16.0, 8.0));
    assert_eq!(s.icon_width, 16.0, "checkbox: size-4");
    assert_eq!(
        style.text_styles[&egui::TextStyle::Body].size,
        14.0,
        "text-sm"
    );
}

#[test]
fn applying_the_dark_preset_maps_the_dark_tokens() {
    let ctx = context(Theme::Dark);
    given(&ctx, &Shadcn::dark());
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
    preset.tokens.radius = eggshell_ui::contract::Radius::new(6.0);
    preset.tokens.control_units = 10.0;
    let layout = given(&ctx, &preset);
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
        layout.shell(&ctx).style().tokens.primary,
        rgb(0x12, 0x34, 0x56)
    );
}

#[test]
fn the_preset_is_picked_by_theme() {
    assert_eq!(Shadcn::for_theme(Theme::Light), Shadcn::light());
    assert_eq!(Shadcn::for_theme(Theme::Dark), Shadcn::dark());
    assert_ne!(Tokens::light(), Tokens::dark());
}

#[test]
fn inputs_take_the_input_proportions() {
    for theme in [Theme::Light, Theme::Dark] {
        let ctx = context(theme);
        let preset = Shadcn::for_theme(theme);
        given(&ctx, &preset);
        let text = base_text();
        let field = TextEdit::singleline("").size(
            &ctx,
            &text,
            &std::sync::Arc::new(preset.style()),
            Some(Width::new(200.0)),
        );
        assert_eq!(field, vec2(200.0, 36.0), "{theme:?}");
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
}

fn harness_of(view: support::View<App>) -> Driver<App> {
    let mut d = support::prepared_driver(
        vec2(400.0, 300.0),
        base_text(),
        App::default(),
        view,
        prepare_light,
    )
    .build();
    d.frames(3);
    d
}

fn switches(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0).p(2.0), |b| {
        for (key, track) in [
            ("default", None),
            ("sized", Some(vec2(48.0, 28.0))),
            ("wide", Some(vec2(64.0, 64.0 * 18.4 / 32.0))),
            ("tall", Some(vec2(36.0 * 32.0 / 18.4, 36.0))),
        ] {
            let switch = b.switch_unlabelled(key, app.on).named(key);
            let switch = match track {
                Some(track) => switch.track(track),
                None => switch,
            };
            switch.on_change(|app: &mut App, value| app.on = value);
        }
    });
}

fn thumb_in(d: &Driver<App>, rect: Rect) -> egui::epaint::CircleShape {
    d.harness()
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
    let mut d = harness_of(switches);
    let tokens = Tokens::light();
    let default = d.rect(Target::suffix("/\"default\"/Track"));
    assert_eq!(default.width(), 32.0, "w-8");
    assert!((default.height() - 18.4).abs() < 0.01, "h-[1.15rem]");

    let cases = [
        ("default", vec2(32.0, 18.4)),
        ("sized", vec2(48.0, 28.0)),
        ("wide", vec2(64.0, 64.0 * 18.4 / 32.0)),
        ("tall", vec2(36.0 * 32.0 / 18.4, 36.0)),
    ];
    for (key, track) in cases {
        let rect = d.rect(Target::suffix(&format!("/{key:?}/Track")));
        assert!(
            (rect.size() - track).abs().max_elem() < 0.01,
            "{key}: leaf {rect:?} for track {track:?}"
        );
    }

    for on in [false, true] {
        d.app_mut().on = on;
        d.frames(8);
        for (key, track) in cases {
            let rect = d.rect(Target::suffix(&format!("/{key:?}/Track")));
            let thumb = thumb_in(&d, rect);
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
    let small = thumb_in(&d, default).radius;
    let big = thumb_in(&d, d.rect(Target::suffix("/\"sized\"/Track"))).radius;
    assert!(
        big > small,
        "precondition: the larger track has a larger thumb"
    );
}

fn segments(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().p(2.0), |b| {
        b.segmented("units", app.unit, UNITS)
            .on_change(|app, unit| app.unit = unit);
    });
}

fn raised_cells(d: &Driver<App>) -> Vec<Rect> {
    let tokens = Tokens::light();
    let raised = tokens.shadow_sm.color;
    let window = d.ctx().content_rect();
    let shapes: Vec<_> = d
        .harness()
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if window.contains_rect(rect.rect) => Some(rect.clone()),
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
fn the_shadcn_segmented_style_raises_exactly_the_selected_cell() {
    let mut d = harness_of(segments);
    let track = d.rect(Target::suffix("/\"units\""));
    assert_eq!(track.height(), 36.0, "h-9");
    let metric = d.rect(Target::suffix("/\"units\"/0"));
    assert_eq!(
        metric.top() - track.top(),
        3.5,
        "p-[3px], and the trigger's h-[calc(100%-1px)] centred"
    );
    assert_eq!(metric.height(), 29.0, "h-[calc(100%-1px)]");
    assert_eq!(
        track.right() - d.rect(Target::suffix("/\"units\"/2")).right(),
        3.0
    );

    let cells = raised_cells(&d);
    assert_eq!(cells.len(), 1, "one raised cell");
    assert!(
        cells[0].center().distance(metric.center()) < 1.0,
        "and it is the pressed one: {:?} vs {metric:?}",
        cells[0]
    );

    d.click(Target::suffix("/\"units\"/1"));
    d.frames(3);
    assert_eq!(d.app().unit, Unit::Imperial, "precondition");
    let cells = raised_cells(&d);
    let imperial = d.rect(Target::suffix("/\"units\"/1"));
    assert_eq!(cells.len(), 1, "still one");
    assert!(cells[0].center().distance(imperial.center()) < 1.0);
}

#[test]
fn a_text_field_under_the_preset_is_h9_with_px_2_5() {
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
    let d = harness_of(field);
    let rect = d.rect(Target::suffix("/\"field\""));
    assert_eq!(rect.size(), vec2(200.0, 36.0));
    let hint = d
        .harness()
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "Name" => Some(text.pos),
            _ => None,
        })
        .expect("the hint is painted");
    assert_eq!(hint.x - rect.left(), 10.0, "px-2.5 (style-vega.css:678)");
}

fn palette(base: BaseColor, theme_color: ThemeColor) -> Palette {
    Palette::new().base(base).theme(theme_color)
}

#[test]
fn an_accent_retints_primary_and_the_charts_and_nothing_else() {
    for theme in [Theme::Light, Theme::Dark] {
        for base in BaseColor::ALL {
            let plain = Shadcn::for_theme(theme).with_palette(palette(base, ThemeColor::Base));
            for accent in ThemeColor::ALL
                .into_iter()
                .filter(|a| *a != ThemeColor::Base)
            {
                let tinted = Shadcn::for_theme(theme).with_palette(palette(base, accent));
                let (a, b) = (&plain.tokens, &tinted.tokens);
                assert_ne!(a.primary, b.primary, "{base:?} {accent:?} {theme:?}");
                assert_ne!(a.primary_foreground, b.primary_foreground);
                assert_ne!(a.chart_1, b.chart_1, "the charts follow the theme colour");
                assert_eq!(a.ring, b.ring, "the ring is the base colour's");
                assert_eq!(b.switch_track_on, b.primary, "the switch follows");
                let same = Tokens {
                    primary: a.primary,
                    primary_foreground: a.primary_foreground,
                    switch_track_on: a.switch_track_on,
                    switch_thumb_on: a.switch_thumb_on,
                    chart_1: a.chart_1,
                    chart_2: a.chart_2,
                    chart_3: a.chart_3,
                    chart_4: a.chart_4,
                    chart_5: a.chart_5,
                    ..b.clone()
                };
                assert_eq!(&same, a, "{base:?} {accent:?} {theme:?}: only those change");
            }
        }
    }
    let blue = |theme| {
        Shadcn::for_theme(theme)
            .with_palette(Palette::new().theme(ThemeColor::Blue))
            .tokens
    };
    assert_eq!(blue(Theme::Light).primary, rgb(0x14, 0x47, 0xe6));
    assert_eq!(blue(Theme::Dark).primary, rgb(0x2b, 0x7f, 0xff));
    assert_eq!(blue(Theme::Dark).primary_foreground, rgb(0, 0, 0));
}

#[test]
fn the_default_palette_is_the_plain_preset() {
    for theme in [Theme::Light, Theme::Dark] {
        let plain = Shadcn::for_theme(theme);
        assert_eq!(plain.clone().with_palette(Palette::new()), plain);
        assert_eq!(Palette::new(), Palette::default());
    }
}

fn luminance(c: egui::Color32) -> f32 {
    let linear = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

fn contrast(a: egui::Color32, b: egui::Color32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn a_dark_accent_fills_with_its_bright_shade_and_keeps_its_label_readable() {
    for base in BaseColor::ALL {
        let plain = Shadcn::dark()
            .with_palette(palette(base, ThemeColor::Base))
            .tokens;
        for accent in ThemeColor::ALL
            .into_iter()
            .filter(|a| *a != ThemeColor::Base)
        {
            let t = Shadcn::dark().with_palette(palette(base, accent)).tokens;
            assert!(
                contrast(t.primary, t.primary_foreground) >= 4.5,
                "{base:?} {accent:?}: {}",
                contrast(t.primary, t.primary_foreground)
            );
            assert!(
                contrast(t.primary, plain.background) >= 3.0,
                "{base:?} {accent:?} stands out on the page"
            );
        }
    }
    let neutral = Shadcn::dark().tokens;
    assert_eq!(
        neutral.primary,
        rgb(0xe5, 0xe5, 0xe5),
        "neutral dark is shadcn's"
    );
}

/// The link's contrast is checked in the shadcn crate's `contrast` test.
#[test]
fn hyperlinks_are_primary() {
    for theme in [Theme::Light, Theme::Dark] {
        for base in BaseColor::ALL {
            for accent in ThemeColor::ALL {
                let ctx = context(theme);
                let preset = Shadcn::for_theme(theme).with_palette(palette(base, accent));
                given(&ctx, &preset);
                assert_eq!(
                    ctx.style_of(theme).visuals.hyperlink_color,
                    preset.tokens.primary,
                    "{base:?} {accent:?} {theme:?}"
                );
            }
        }
    }
}

#[test]
fn an_accent_reaches_egui_when_applied() {
    let ctx = context(Theme::Light);
    let layout = given(
        &ctx,
        &Shadcn::light().with_palette(Palette::new().theme(ThemeColor::Violet)),
    );
    assert_eq!(
        ctx.style_of(Theme::Light).visuals.hyperlink_color,
        rgb(0x70, 0x08, 0xe7)
    );
    assert_eq!(
        layout.shell(&ctx).style().tokens.primary,
        rgb(0x70, 0x08, 0xe7)
    );
}

#[test]
fn semantic_colours_are_defined_in_both_themes() {
    let light = Tokens::light();
    let dark = Tokens::dark();
    assert_eq!(light.success, rgb(0x00, 0x82, 0x36), "green-700");
    assert_eq!(light.warning, rgb(0xbb, 0x4d, 0x00), "amber-700");
    assert_eq!(
        light.warning_foreground,
        rgb(0xfa, 0xfa, 0xfa),
        "neutral-50"
    );
    assert_eq!(light.info, rgb(0x00, 0x69, 0xa8), "sky-700");
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
    b.div(tw().w_px(160.0).p(2.0), |b| {
        b.segmented(
            "wrapped",
            app.unit,
            [
                (Unit::Metric, "Metric"),
                (Unit::Imperial, "Imperial"),
                (Unit::Nautical, "Nautical"),
            ],
        )
        .wrapping()
        .on_change(|app, unit| app.unit = unit);
    });
}

#[test]
fn segmented_cells_fit_their_track_and_can_wrap() {
    let d = harness_of(segments);
    let track = d.rect(Target::suffix("/\"units\""));
    let cells = [0, 1, 2].map(|c| d.rect(Target::suffix(&format!("/\"units\"/{c}"))));
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

    let d = harness_of(many_segments);
    let track = d.rect(Target::suffix("/\"wrapped\""));
    let cells = [0, 1, 2].map(|c| d.rect(Target::suffix(&format!("/\"wrapped\"/{c}"))));
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

#[test]
fn shadcn_segmented_cells_all_carry_a_one_point_border_and_only_its_colour_varies() {
    use eggshell_ui::contract::Size;
    use eggshell_ui::owned::{
        Enablement,
        segmented::{self, Cell},
        tabs,
    };
    for theme in [Theme::Light, Theme::Dark] {
        let style = Shadcn::for_theme(theme).style();
        let tokens = Tokens::for_theme(theme);
        let trigger = |cell| {
            crate::support::part_at(
                &style,
                tabs::Part::Trigger,
                &tabs::Part::trigger_keys(
                    Size::DEFAULT,
                    segmented::Rows::One,
                    cell,
                    Enablement::Enabled,
                ),
            )
        };
        let border = |cell| {
            let classes = trigger(cell);
            let width = classes
                .style_at(style.tokens.spacing)
                .border
                .top
                .into_raw()
                .value();
            let color = classes.paint_classes().border_colors.top;
            (width, color.unwrap_or(Color32::TRANSPARENT))
        };
        let selected = match theme {
            Theme::Dark => tokens.input,
            Theme::Light => Color32::TRANSPARENT,
        };
        assert_eq!(
            border(Cell::Released),
            (1.0, Color32::TRANSPARENT),
            "border-transparent, {theme:?}"
        );
        assert_eq!(border(Cell::Pressed), (1.0, selected), "{theme:?}");
    }
}

/// A section set after the preset changes what the widget reads of it and
/// keeps the preset's look: its rules are the widgets' defaults, under the
/// preset's.
#[test]
fn a_section_set_after_the_preset_keeps_the_presets_look() {
    use eggshell_ui::owned::tooltip;
    let mut style = Shadcn::for_theme(Theme::Light).style();
    let before = style
        .sheet
        .resolve(tooltip::Part::Content, &[], &style.tokens)
        .style()
        .clone();
    let mut tooltip = style.tooltip.clone();
    tooltip.delay = core::time::Duration::from_millis(123);
    style.tooltip = tooltip;
    assert_eq!(style.tooltip.delay, core::time::Duration::from_millis(123));
    let after = style
        .sheet
        .resolve(tooltip::Part::Content, &[], &style.tokens)
        .style()
        .clone();
    assert_eq!(after, before, "the preset's look");
    assert_eq!(
        style.sheet.layer(),
        eggshell_ui::sheet::Layer::Base,
        "the layer is kept"
    );
}
