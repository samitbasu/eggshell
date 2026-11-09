//! The contract gaps of issues #42 and #43: text placed in taller boxes, icon
//! cells and buttons, caller classes merged over the styled ones, text roles,
//! toggle variants, node-opacity disabling and the cheap install. Pointer and
//! key input goes through kittest's real events.

use bullseye::contract::{Disabled, Paint, Size, Variant};
use bullseye::owned::{
    Button, Checkbox, Enablement, Mark, RadioGroup, Segmented, Select, Slider, Stepper, Switch,
};
use bullseye::{Builder, Icon, Opacity, Style, TextSpec, TextStyle, none, tw};
use bullseye_shadcn::{Shadcn, ShadcnStyle, Tokens, size};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Rect, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, box_at, click_node, hover_node, rect_of, shapes};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Mode {
    #[default]
    Light,
    Dark,
    System,
}

const MODES: [(Mode, &str); 3] = [
    (Mode::Light, "Light"),
    (Mode::Dark, "Dark"),
    (Mode::System, "System"),
];

const ARMED: Variant = Variant("armed");

struct App {
    style: Style,
    mode: Mode,
    height: f32,
    font: f32,
    on: bool,
    clicks: u32,
    texture: Option<egui::TextureHandle>,
}

fn spec(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: (size * 1.25).round(),
        family: FontFamily::Proportional,
    }
}

impl App {
    fn icon(&self) -> Icon {
        let id = self
            .texture
            .as_ref()
            .map_or(egui::TextureId::default(), |texture| texture.id());
        Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
            id,
            vec2(4.0, 4.0),
        )))
    }
}

fn prepare(ctx: &egui::Context, app: &App) {
    app.style.clone().install(ctx);
}

type Harn = Harness<'static, Rig<App>>;

fn harness_with(
    theme: Theme,
    shadcn: ShadcnStyle,
    edit: impl FnOnce(&mut Style),
    view: support::View<App>,
) -> Harn {
    let mut style = Shadcn::for_theme(theme).with_style(shadcn).style();
    edit(&mut style);
    let tokens = Tokens::for_theme(theme).in_style(shadcn);
    let app = App {
        style,
        mode: Mode::default(),
        height: 40.0,
        font: 14.0,
        on: false,
        clicks: 0,
        texture: None,
    };
    let mut harness = support::harness_prepared(
        vec2(520.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
        prepare,
    );
    harness.ctx.set_theme(theme);
    let texture = harness.ctx.load_texture(
        "icon",
        egui::ColorImage::filled([4, 4], Color32::WHITE),
        egui::TextureOptions::NEAREST,
    );
    harness.state_mut().app.texture = Some(texture);
    harness.run_steps(14);
    harness
}

fn harness(theme: Theme, view: support::View<App>) -> Harn {
    harness_with(theme, ShadcnStyle::Vega, |_| {}, view)
}

const COMBINATIONS: [(Theme, ShadcnStyle); 4] = [
    (Theme::Light, ShadcnStyle::Vega),
    (Theme::Dark, ShadcnStyle::Vega),
    (Theme::Light, ShadcnStyle::Lyra),
    (Theme::Dark, ShadcnStyle::Lyra),
];

fn cell_path(mode: Mode) -> String {
    format!("/\"seg\"/{mode:?}")
}

/// The box a text was painted in, if one was painted inside `inside`.
fn text_box(harness: &Harn, inside: Rect, text: &str) -> Rect {
    shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == text && inside.expand(1.0).contains(t.pos) => {
                Some(Rect::from_min_size(
                    t.pos + t.galley.rect.min.to_vec2(),
                    t.galley.rect.size(),
                ))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text:?} in {inside:?}"))
}

/// The tint and box of the image painted inside `inside`.
fn icon_in(harness: &Harn, inside: Rect) -> (Color32, Rect) {
    shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Mesh(mesh) if inside.expand(0.5).contains_rect(mesh.calc_bounds()) => {
                Some((mesh.vertices[0].color, mesh.calc_bounds()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no image in {inside:?}"))
}

// ----- #42: text in a box taller than it -----

/// ```html
/// <div class="flex items-center w-fit p-[3px] rounded-lg bg-muted" style="height: <h>">
///   <span class="grow flex items-center justify-center text-center h-full">Light</span> …
/// </div>
/// ```
fn centred_cells(b: &mut Builder<'_, '_, App>, app: &App) {
    Segmented::new(app.mode, &MODES)
        .track(tw().h_px(app.height))
        .cells(tw().role(spec(app.font)))
        .show(b, "seg", |app: &mut App, _, mode| app.mode = mode);
}

#[test]
fn a_segmented_label_is_centred_in_a_cell_taller_than_it_at_any_height_and_size() {
    for (theme, shadcn) in COMBINATIONS {
        let mut harness = harness_with(theme, shadcn, |_| {}, centred_cells);
        for height in [64.0, 77.0, 120.0] {
            for font in [11.0, 14.0, 20.0, 28.0] {
                harness.state_mut().app.height = height;
                harness.state_mut().app.font = font;
                harness.run_steps(4);
                for (mode, label) in MODES {
                    let cell = rect_of(&harness, &cell_path(mode));
                    let text = text_box(&harness, cell, label);
                    assert!(
                        cell.height() > text.height() + 8.0,
                        "precondition: the cell is taller than its text ({cell:?}, {text:?})"
                    );
                    assert!(
                        (text.center().y - cell.center().y).abs() <= 0.75,
                        "{theme:?} {shadcn:?} height {height} font {font} {label}: \
                         text centre {}, cell centre {}",
                        text.center().y,
                        cell.center().y,
                    );
                }
            }
        }
    }
}

/// ```html
/// <div class="flex flex-col gap-2">
///   <span class="h-24">top</span>
///   <span class="flex items-center h-24">middle</span>
///   <span class="flex flex-col justify-center h-24">column</span>
///   <span class="flex items-end h-24">bottom</span>
///   <span class="flex flex-col-reverse justify-end h-24">reversed</span>
/// </div>
/// ```
fn alignments(b: &mut Builder<'_, '_, App>, app: &App) {
    let h = app.height;
    b.div(tw().flex().flex_col().gap(2.0).role(spec(app.font)), |b| {
        b.span("top", tw().h_px(h), "top", none);
        b.span("middle", tw().flex().items_center().h_px(h), "middle", none);
        b.span(
            "column",
            tw().flex().flex_col().justify_center().h_px(h),
            "column",
            none,
        );
        b.span("bottom", tw().flex().items_end().h_px(h), "bottom", none);
        b.span(
            "reversed",
            tw().flex().flex_col_reverse().justify_end().h_px(h),
            "reversed",
            none,
        );
        b.span("grid", tw().grid().items_center().h_px(h), "grid", none);
        b.span(
            "baseline",
            tw().flex().items_baseline().h_px(h),
            "baseline",
            none,
        );
    });
}

#[test]
fn a_text_leaf_places_its_line_by_its_own_vertical_alignment() {
    let mut harness = harness(Theme::Light, alignments);
    for height in [30.0, 64.0, 100.0] {
        for font in [12.0, 16.0, 24.0] {
            harness.state_mut().app.height = height;
            harness.state_mut().app.font = font;
            harness.run_steps(4);
            for (name, expected) in [
                ("top", 0.0),
                ("middle", 0.5),
                ("column", 0.5),
                ("bottom", 1.0),
                ("reversed", 0.0),
                ("grid", 0.5),
                ("baseline", 0.0),
            ] {
                let leaf = rect_of(&harness, &format!("/\"{name}\""));
                let text = text_box(&harness, leaf, name);
                assert!(
                    leaf.height() >= text.height(),
                    "precondition: {name} fits its text at height {height}, font {font}"
                );
                let spare = leaf.height() - text.height();
                let at = text.min.y - leaf.min.y;
                assert!(
                    (at - spare * expected).abs() <= 0.75,
                    "{name} at height {height}, font {font}: text {at} down of {spare} spare"
                );
            }
        }
    }
}

// ----- #43 item 1 and 4: icon cells and the gap between cells -----

/// ```html
/// <div class="flex items-center w-fit h-9 p-[3px] gap-[2px] rounded-lg bg-muted">
///   <span class="grow flex items-center justify-center gap-[6px] px-2 py-1 rounded-md"><svg class="size-4"/>Light</span> …
/// </div>
/// ```
fn icon_cells(b: &mut Builder<'_, '_, App>, app: &App) {
    Segmented::new(app.mode, &MODES)
        .icon(Mode::Light, app.icon())
        .icon(Mode::Dark, app.icon())
        .show(b, "seg", |app: &mut App, _, mode| app.mode = mode);
}

#[test]
fn an_icon_cell_has_the_styled_icon_gap_label_and_the_cells_text_colour() {
    for (theme, shadcn) in COMBINATIONS {
        let harness = harness_with(
            theme,
            shadcn,
            |style| {
                style.segmented.icon_size = 18.0;
                style.segmented.icon_gap = 7.0;
                style.segmented.column_gap = 3.0;
            },
            icon_cells,
        );
        let seg = harness.state().app.style.segmented.clone();
        let cells = MODES.map(|(mode, _)| rect_of(&harness, &cell_path(mode)));
        for pair in cells.windows(2) {
            assert!(
                (pair[1].min.x - pair[0].max.x - 3.0).abs() < 0.01,
                "{theme:?} {shadcn:?}: the column gap separates the cells ({pair:?})"
            );
        }
        for (index, (mode, label)) in MODES.into_iter().enumerate() {
            let cell = cells[index];
            let text = text_box(&harness, cell, label);
            let selected = mode == Mode::Light;
            let colors = if selected {
                seg.selected.states.rest
            } else {
                seg.unselected.states.rest
            };
            if mode == Mode::System {
                assert!(
                    shapes(&harness).into_iter().all(|shape| !matches!(
                        shape,
                        Shape::Mesh(mesh) if cell.contains_rect(mesh.calc_bounds())
                    )),
                    "a cell without an icon draws none"
                );
                continue;
            }
            let (tint, icon) = icon_in(&harness, cell);
            assert_eq!(icon.size(), vec2(18.0, 18.0), "icon_size");
            assert!(
                (text.min.x - icon.max.x - 7.0).abs() < 0.75,
                "{theme:?} {shadcn:?} {label}: icon_gap ({icon:?} then {text:?})"
            );
            assert_eq!(tint, colors.text, "{theme:?} {shadcn:?} {label}: tint");
            assert!(
                (icon.center().y - cell.center().y).abs() <= 0.75,
                "the icon is centred in the cell"
            );
        }
        let (_, unselected_before) = icon_in(&harness, cells[1]);
        assert!(unselected_before.width() > 0.0);
    }
}

#[test]
fn an_icon_follows_the_hover_colour_and_clicking_the_cell_chooses_it() {
    let mut harness = harness_with(
        Theme::Light,
        ShadcnStyle::Vega,
        |style| {
            let hover = style.segmented.unselected.states.hover;
            style.segmented.unselected.states.hover.text = Color32::from_rgb(200, 30, 30);
            assert_ne!(
                hover.text,
                Color32::from_rgb(200, 30, 30),
                "precondition: the hover colour differs"
            );
        },
        icon_cells,
    );
    let dark = cell_path(Mode::Dark);
    hover_node(&mut harness, &dark);
    harness.run_steps(12);
    let (tint, _) = icon_in(&harness, rect_of(&harness, &dark));
    assert_eq!(tint, Color32::from_rgb(200, 30, 30), "tinted by hover text");
    click_node(&mut harness, &dark);
    harness.run_steps(2);
    assert_eq!(harness.state().app.mode, Mode::Dark);
    let label = harness
        .get_by_role_and_label(Role::RadioButton, "Dark")
        .accesskit_node()
        .toggled();
    assert_eq!(
        label,
        Some(egui::accesskit::Toggled::True),
        "the icon cell is a chosen radio button named by its label"
    );
}

// ----- #43 item 2: caller classes over the styled ones -----

fn merged_segmented(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().w_px(400.0), |b| {
        Segmented::new(app.mode, &MODES)
            .track(tw().w_full())
            .cells(tw().flex_1())
            .show(b, "seg", |app: &mut App, _, mode| app.mode = mode);
    });
}

#[test]
fn track_and_cell_classes_merge_over_the_styled_ones_and_keep_the_style() {
    for (theme, shadcn) in COMBINATIONS {
        let mut harness = harness_with(theme, shadcn, |_| {}, merged_segmented);
        let seg = harness.state().app.style.segmented.clone();
        let track = rect_of(&harness, "/\"seg\"");
        assert_eq!(track.width(), 400.0, "w_full replaced w_fit");
        assert_eq!(track.height(), seg.height, "the styled height stays");
        let cells = MODES.map(|(mode, _)| rect_of(&harness, &cell_path(mode)));
        let widths: Vec<_> = cells.iter().map(|cell| cell.width()).collect();
        assert!(
            widths.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01),
            "flex-1 shares the track equally: {widths:?}"
        );
        assert!(widths[0] > 100.0, "precondition: the cells are wide");
        let selected = box_at(&harness, cells[0]);
        assert_eq!(selected.fill, seg.selected.states.rest.fill, "styled paint");
        for (mode, label) in MODES {
            let cell = rect_of(&harness, &cell_path(mode));
            let text = text_box(&harness, cell, label);
            assert!(
                (text.center().x - cell.center().x).abs() <= 0.75,
                "{theme:?} {shadcn:?} {label}: centred across a wide cell"
            );
        }
        click_node(&mut harness, &cell_path(Mode::System));
        harness.run_steps(2);
        assert_eq!(harness.state().app.mode, Mode::System, "still interactive");
    }
}

/// One of every owned widget, each with a caller width.
fn classed(b: &mut Builder<'_, '_, App>, _: &App) {
    let wide = || tw().w_px(321.0);
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        Button::new("Save")
            .classes(wide())
            .show(b, "button", |_: &mut App| {});
        Checkbox::new("Agree", Mark::Off)
            .classes(wide())
            .show(b, "checkbox", |_: &mut App| {});
        RadioGroup::new(Mode::Light, &MODES)
            .classes(wide())
            .show(b, "radio", |_: &mut App, _| {});
        Switch::new("Alerts", Mark::Off)
            .classes(wide())
            .show(b, "switch", |_: &mut App| {});
        Slider::new(0.5, 0.0..=1.0)
            .named("Volume")
            .classes(wide())
            .show(b, "slider", |_: &mut App, _| {});
        Select::new(Mode::Light, &MODES)
            .named("Mode")
            .classes(wide())
            .show(b, "select", |_: &mut App, _, _| {});
        Stepper::new(1.0, 0.0..=9.0)
            .named("Count")
            .classes(wide())
            .show(b, "stepper", |_: &mut App, _| {});
        Segmented::new(Mode::Light, &MODES).classes(wide()).show(
            b,
            "segmented",
            |_: &mut App, _, _| {},
        );
    });
}

#[test]
fn every_owned_widget_takes_caller_classes_over_its_own() {
    for (theme, shadcn) in COMBINATIONS {
        let harness = harness_with(theme, shadcn, |_| {}, classed);
        let style = &harness.state().app.style;
        for path in [
            "/\"button\"",
            "/\"checkbox\"/<div>",
            "/\"radio\"/<div>",
            "/\"switch\"/<div>",
            "/\"slider\"/Track",
            "/\"select\"",
            "/\"stepper\"/Box",
            "/\"segmented\"",
        ] {
            assert_eq!(rect_of(&harness, path).width(), 321.0, "{path}");
        }
        let height = |path| rect_of(&harness, path).height();
        assert_eq!(
            height("/\"button\""),
            style.button.sizes.get(Size::DEFAULT).height
        );
        assert_eq!(height("/\"select\""), style.select.metrics.height);
        assert_eq!(height("/\"stepper\"/Box"), style.stepper.height);
        assert_eq!(height("/\"segmented\""), style.segmented.height);
    }
}

// ----- #43 item 5: a text role per widget section -----

fn roles(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        Button::new("Save").show(b, "button", |_: &mut App| {});
        Segmented::new(app.mode, &MODES).show(b, "seg", |_: &mut App, _, _| {});
        Select::new(Mode::Light, &MODES)
            .named("Mode")
            .show(b, "select", |_: &mut App, _, _| {});
        Checkbox::new("Agree", Mark::Off).show(b, "check", |_: &mut App| {});
    });
}

fn size_of_text(harness: &Harn, inside: &str, text: &str) -> f32 {
    shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t)
                if t.galley.text() == text
                    && rect_of(harness, inside).expand(1.0).contains(t.pos) =>
            {
                Some(t.galley.job.sections[0].format.font_id.size)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text:?} in {inside}"))
}

#[test]
fn a_widget_section_has_its_own_text_role_that_defaults_to_the_styles() {
    for (theme, shadcn) in COMBINATIONS {
        let plain = harness_with(theme, shadcn, |_| {}, roles);
        let base = plain.state().app.style.text.size;
        for (path, text) in [
            ("/\"button\"", "Save"),
            ("/\"seg\"/Light", "Light"),
            ("/\"select\"", "Light"),
            ("/\"check\"/<div>", "Agree"),
        ] {
            assert_eq!(size_of_text(&plain, path, text), base, "{path}: default");
        }
        let harness = harness_with(
            theme,
            shadcn,
            |style| {
                style.button.text = Some(spec(21.0));
                style.segmented.text = Some(spec(17.0));
                style.select.text = Some(spec(19.0));
                style.checkbox.text = Some(spec(23.0));
            },
            roles,
        );
        for (path, text, size) in [
            ("/\"button\"", "Save", 21.0),
            ("/\"seg\"/Light", "Light", 17.0),
            ("/\"select\"", "Light", 19.0),
            ("/\"check\"/<div>", "Agree", 23.0),
        ] {
            assert_eq!(size_of_text(&harness, path, text), size, "{path}");
        }
        assert_ne!(
            base, 21.0,
            "precondition: the roles differ from the default"
        );
    }
}

// ----- #43 item 7: install only when changed -----

#[test]
fn installing_an_unchanged_style_keeps_the_installed_one() {
    let ctx = egui::Context::default();
    let style = Shadcn::light().style();
    style.clone().install(&ctx);
    let first = Style::of(&ctx);
    style.clone().install(&ctx);
    assert!(
        std::sync::Arc::ptr_eq(&first, &Style::of(&ctx)),
        "same style"
    );
    let mut changed = style.clone();
    changed.segmented.icon_gap += 1.0;
    changed.clone().install(&ctx);
    let second = Style::of(&ctx);
    assert!(!std::sync::Arc::ptr_eq(&first, &second));
    assert_eq!(*second, changed);
    changed.clone().install_for(&ctx, Theme::Light);
    assert!(
        std::sync::Arc::ptr_eq(&second, &Style::of(&ctx)),
        "per theme too"
    );
    style.clone().install_for(&ctx, Theme::Dark);
    ctx.set_theme(Theme::Light);
    assert_eq!(*Style::of(&ctx), changed);
    ctx.set_theme(Theme::Dark);
    assert_eq!(*Style::of(&ctx), style, "the other theme took its own");
}

// ----- #43 item 8: toggle looks keyed by variant -----

fn armed_paint() -> Paint {
    let colors = |fill| bullseye::contract::Colors {
        fill,
        border: Color32::TRANSPARENT,
        text: Color32::WHITE,
    };
    Paint::new(bullseye::contract::States::still(
        colors(Color32::from_rgb(10, 120, 60)),
        Color32::BLACK,
    ))
}

fn toggles(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        Button::toggle("Plain", Mark::from(app.on)).show(b, "plain", |app: &mut App| {
            app.on = !app.on;
        });
        Button::toggle("Tool", Mark::from(app.on))
            .variant(ARMED)
            .show(b, "tool", |app: &mut App| app.on = !app.on);
    });
}

#[test]
fn a_toggle_takes_its_off_and_on_looks_from_its_variant_and_keeps_toggling() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = harness_with(
            theme,
            ShadcnStyle::Vega,
            |style| {
                let off = Paint::new(bullseye::contract::States::still(
                    bullseye::contract::Colors {
                        fill: Color32::from_rgb(40, 40, 40),
                        border: Color32::TRANSPARENT,
                        text: Color32::GRAY,
                    },
                    Color32::BLACK,
                ));
                style.button.toggle_off.insert(ARMED, off);
                style.button.toggle_on.insert(ARMED, armed_paint());
            },
            toggles,
        );
        let style = harness.state().app.style.clone();
        let plain = rect_of(&harness, "/\"plain\"");
        let tool = rect_of(&harness, "/\"tool\"");
        assert_eq!(
            box_at(&harness, plain).fill,
            style
                .button
                .toggle_off
                .get(Variant::DEFAULT)
                .states
                .rest
                .fill,
            "the default variant is the old toggle"
        );
        assert_eq!(box_at(&harness, tool).fill, Color32::from_rgb(40, 40, 40));
        click_node(&mut harness, "/\"tool\"");
        harness.run_steps(14);
        assert!(harness.state().app.on, "a toggle variant still toggles");
        assert_eq!(box_at(&harness, tool).fill, Color32::from_rgb(10, 120, 60));
        assert_eq!(
            box_at(&harness, plain).fill,
            style
                .button
                .toggle_on
                .get(Variant::DEFAULT)
                .states
                .rest
                .fill
        );
        assert_eq!(
            support::text_color(&harness, tool, "Tool"),
            Color32::WHITE,
            "the variant's text colour"
        );
        for name in ["Plain", "Tool"] {
            assert_eq!(
                harness
                    .get_by_role_and_label(Role::Button, name)
                    .accesskit_node()
                    .toggled(),
                Some(egui::accesskit::Toggled::True),
                "{name} reports its pressed state"
            );
        }
    }
}

// ----- #43 item 9: disabled as node opacity -----

fn disabled_row(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        for enablement in [Enablement::Enabled, Enablement::Disabled] {
            Button::new("Save")
                .variant(bullseye_shadcn::variant::PRIMARY)
                .enablement(enablement)
                .show(b, ("button", enablement), |app: &mut App| app.clicks += 1);
            Checkbox::new("Agree", Mark::On)
                .enablement(enablement)
                .show(b, ("check", enablement), |app: &mut App| app.clicks += 1);
        }
    });
}

#[test]
fn a_faded_disabled_control_is_node_opacity_over_its_rest_colours() {
    for (theme, shadcn) in COMBINATIONS {
        let opacity = 0.4;
        let mut harness = harness_with(
            theme,
            shadcn,
            |style| style.disabled = Disabled::Faded(Opacity::new(opacity)),
            disabled_row,
        );
        let style = harness.state().app.style.clone();
        let rest = style
            .button
            .variants
            .get(bullseye_shadcn::variant::PRIMARY)
            .paint
            .states
            .rest;
        let enabled = rect_of(&harness, "/(\"button\", Enabled)");
        let disabled = rect_of(&harness, "/(\"button\", Disabled)");
        assert_eq!(box_at(&harness, enabled).fill, rest.fill, "precondition");
        let fill = box_at(&harness, disabled).fill;
        assert_eq!(
            fill,
            rest.fill.gamma_multiply(opacity),
            "{theme:?} {shadcn:?}: the rest fill at node opacity, translucent not opaque"
        );
        assert!(fill.a() < 255 && fill.a() > 0);
        assert_eq!(
            support::text_color(&harness, disabled, "Save"),
            rest.text.gamma_multiply(opacity),
            "the label fades with its node"
        );
        let label = rect_of(&harness, "/(\"check\", Disabled)/<div>");
        let enabled_label = style.label.rest.text;
        assert_eq!(
            support::text_color(&harness, label, "Agree"),
            enabled_label.gamma_multiply(opacity),
            "a choice's label fades with its row"
        );
        click_node(&mut harness, "/(\"button\", Disabled)");
        click_node(&mut harness, "/(\"check\", Disabled)/<div>");
        assert_eq!(harness.state().app.clicks, 0, "still inert");
        click_node(&mut harness, "/(\"button\", Enabled)");
        assert_eq!(harness.state().app.clicks, 1);
    }
}

#[test]
fn a_faded_control_on_a_translucent_surface_lets_the_surface_through() {
    let translucent = Color32::from_rgba_unmultiplied(20, 20, 220, 128);
    let mut harness = harness_with(
        Theme::Light,
        ShadcnStyle::Vega,
        |style| {
            style.disabled = Disabled::Faded(Opacity::new(0.3));
            let look = style
                .button
                .variants
                .get_mut(bullseye_shadcn::variant::PRIMARY)
                .expect("primary");
            look.paint.states.rest.fill = translucent;
        },
        disabled_row,
    );
    let disabled = rect_of(&harness, "/(\"button\", Disabled)");
    let fill = box_at(&harness, disabled).fill;
    assert_eq!(fill, translucent.gamma_multiply(0.3));
    assert!(
        fill.a() < translucent.a(),
        "fainter than the enabled fill, so the surface shows through"
    );
    harness.run_steps(1);
}

#[test]
fn the_colors_form_still_draws_the_styles_disabled_colours() {
    let harness = harness(Theme::Light, disabled_row);
    let style = &harness.state().app.style;
    assert_eq!(style.disabled, Disabled::Colors);
    let primary = style.button.variants.get(bullseye_shadcn::variant::PRIMARY);
    let disabled = rect_of(&harness, "/(\"button\", Disabled)");
    assert_eq!(
        box_at(&harness, disabled).fill,
        primary.paint.states.disabled.fill
    );
}

// ----- #43 item 10: icon buttons -----

fn icon_buttons(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().gap(2.0).items_center(), |b| {
        Button::icon_only(app.icon(), "Search")
            .size(size::ICON)
            .variant(bullseye_shadcn::variant::GHOST)
            .show(b, "icon", |app: &mut App| app.clicks += 1);
        Button::new("Add")
            .icon(app.icon())
            .show(b, "labelled", |app: &mut App| app.clicks += 10);
        Button::toggle("", Mark::from(app.on))
            .icon(app.icon())
            .named("Pin")
            .size(size::ICON)
            .show(b, "toggle", |app: &mut App| app.on = !app.on);
    });
}

#[test]
fn an_icon_button_is_a_glyph_in_the_size_and_named_for_assistive_technology() {
    for (theme, shadcn) in COMBINATIONS {
        let mut harness = harness_with(theme, shadcn, |_| {}, icon_buttons);
        let style = harness.state().app.style.clone();
        let metrics = *style.button.sizes.get(size::ICON);
        let button = rect_of(&harness, "/\"icon\"");
        assert_eq!(
            button.size(),
            vec2(style.button.sizes.get(size::ICON).min_width, metrics.height),
            "{theme:?} {shadcn:?}: a square control"
        );
        let (tint, glyph) = icon_in(&harness, button);
        assert_eq!(glyph.size(), vec2(metrics.glyph, metrics.glyph));
        assert!(glyph.center().distance(button.center()) <= 0.75, "centred");
        let ghost = style.button.variants.get(bullseye_shadcn::variant::GHOST);
        assert_eq!(tint, ghost.paint.states.rest.text);

        hover_node(&mut harness, "/\"icon\"");
        harness.run_steps(14);
        let (hovered, _) = icon_in(&harness, button);
        assert_eq!(hovered, ghost.paint.states.hover.text, "tinted per state");

        let labelled = rect_of(&harness, "/\"labelled\"");
        let default = style.button.sizes.get(Size::DEFAULT);
        let (_, glyph) = icon_in(&harness, labelled);
        assert_eq!(glyph.size(), vec2(default.glyph, default.glyph));
        let text = text_box(&harness, labelled, "Add");
        assert!(
            (text.min.x - glyph.max.x - default.gap).abs() < 0.75,
            "icon, then the control gap, then the label"
        );

        click_node(&mut harness, "/\"icon\"");
        click_node(&mut harness, "/\"toggle\"");
        harness.run_steps(2);
        assert_eq!(harness.state().app.clicks, 1);
        assert!(harness.state().app.on);
        let _ = harness.get_by_role_and_label(Role::Button, "Search");
        assert_eq!(
            harness
                .get_by_role_and_label(Role::Button, "Pin")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
    }
}

fn unnamed(b: &mut Builder<'_, '_, App>, _: &App) {
    Button::new("").show(b, "nameless", |_: &mut App| {});
}

#[test]
#[should_panic(expected = "no label and no name")]
fn a_button_with_neither_label_nor_name_is_a_bug_in_a_debug_build() {
    let _ = harness(Theme::Light, unnamed);
}

// ----- #44: the stepper's number text fits its box -----

fn stepper(b: &mut Builder<'_, '_, App>, _: &App) {
    Stepper::new(3.0, 0.0..=9.0)
        .named("Count")
        .show(b, "stepper", |_: &mut App, _| {});
}

#[test]
fn the_steppers_number_field_fits_inside_the_group_border() {
    for (theme, shadcn) in COMBINATIONS {
        let harness = harness_with(theme, shadcn, |_| {}, stepper);
        let style = &harness.state().app.style;
        let group = rect_of(&harness, "/\"stepper\"/Box");
        let inside = group.shrink(style.stepper.group.border_width);
        let field_box = rect_of(&harness, "/\"stepper\"/Box/<div>");
        let field = rect_of(&harness, "/\"stepper\"/Box/Field");
        assert_eq!(group.height(), style.stepper.height, "precondition");
        assert!(
            field_box.height() <= inside.height() + 0.01,
            "{theme:?} {shadcn:?}: the box is inside the border"
        );
        assert!(
            field.top() >= field_box.top() - 0.01 && field.bottom() <= field_box.bottom() + 0.01,
            "{theme:?} {shadcn:?}: the field {field:?} is inside its box {field_box:?}"
        );
    }
}
