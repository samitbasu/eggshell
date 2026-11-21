//! The contract gaps of issues #42 and #43: text placed in taller boxes, icon
//! cells and buttons, caller classes merged over the styled ones, text roles,
//! toggle variants, node-opacity disabling and the cheap install. Pointer and
//! key input goes through kittest's real events.

use std::sync::Arc;

use eggshell_ui::contract::{Disabled, Paint, Size, Themed, Variant};
use eggshell_ui::owned::{Choice, Enablement, Face};
use eggshell_ui::{Builder, Icon, Opacity, Style, TextSpec, TextStyle, Weight, tw};
use eggshell_ui_shadcn::{Shadcn, ShadcnStyle, Tokens, size};
use egui::TextureId;
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Rect, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use eggshell_ui::testing::{Driver, Target};

use crate::support::{self, box_at, rect_of, shapes, text_color};

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
        weight: Weight::Normal,
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

type Harn = Driver<App>;

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
        style: style.clone(),
        mode: Mode::default(),
        height: 40.0,
        font: 14.0,
        on: false,
        clicks: 0,
        texture: None,
    };
    let mut d = support::driver(
        vec2(520.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .style(Themed::both(Arc::new(style)))
    .build();
    d.ctx().set_theme(theme);
    let texture = d.ctx().load_texture(
        "icon",
        egui::ColorImage::filled([4, 4], Color32::WHITE),
        egui::TextureOptions::NEAREST,
    );
    d.app_mut().texture = Some(texture);
    d.frames(14);
    d
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
    format!("/\"seg\"/{}", mode as usize)
}

/// The box a text was painted in, if one was painted inside `inside`.
fn text_box(d: &Harn, inside: Rect, text: &str) -> Rect {
    shapes(d)
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
fn icon_in(d: &Harn, inside: Rect) -> (Color32, Rect) {
    shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Mesh(mesh)
                if mesh.texture_id != TextureId::default()
                    && inside.expand(0.5).contains_rect(mesh.calc_bounds()) =>
            {
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
    b.segmented("seg", app.mode, MODES)
        .track(tw().h_px(app.height))
        .cells(tw().role(spec(app.font)))
        .on_change(|app: &mut App, mode| app.mode = mode);
}

#[test]
fn a_segmented_label_is_centred_in_a_cell_taller_than_it_at_any_height_and_size() {
    for (theme, shadcn) in COMBINATIONS {
        let mut d = harness_with(theme, shadcn, |_| {}, centred_cells);
        for height in [64.0, 77.0, 120.0] {
            for font in [11.0, 14.0, 20.0, 28.0] {
                d.app_mut().height = height;
                d.app_mut().font = font;
                d.frames(4);
                for (mode, label) in MODES {
                    let cell = d.rect(Target::suffix(&cell_path(mode)));
                    let text = text_box(&d, cell, label);
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
        b.span("top", tw().h_px(h), "top");
        b.span("middle", tw().flex().items_center().h_px(h), "middle");
        b.span(
            "column",
            tw().flex().flex_col().justify_center().h_px(h),
            "column",
        );
        b.span("bottom", tw().flex().items_end().h_px(h), "bottom");
        b.span(
            "reversed",
            tw().flex().flex_col_reverse().justify_end().h_px(h),
            "reversed",
        );
        b.span("grid", tw().grid().items_center().h_px(h), "grid");
        b.span("baseline", tw().flex().items_baseline().h_px(h), "baseline");
    });
}

#[test]
fn a_text_leaf_places_its_line_by_its_own_vertical_alignment() {
    let mut d = harness(Theme::Light, alignments);
    for height in [30.0, 64.0, 100.0] {
        for font in [12.0, 16.0, 24.0] {
            d.app_mut().height = height;
            d.app_mut().font = font;
            d.frames(4);
            for (name, expected) in [
                ("top", 0.0),
                ("middle", 0.5),
                ("column", 0.5),
                ("bottom", 1.0),
                ("reversed", 0.0),
                ("grid", 0.5),
                ("baseline", 0.0),
            ] {
                let leaf = d.rect(Target::suffix(&format!("/\"{name}\"")));
                let text = text_box(&d, leaf, name);
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
    let options = [
        Choice::new(Mode::Light, "Light").icon(app.icon()),
        Choice::new(Mode::Dark, "Dark").icon(app.icon()),
        Choice::new(Mode::System, "System"),
    ];
    b.segmented("seg", app.mode, options)
        .on_change(|app: &mut App, mode| app.mode = mode);
}

#[test]
fn an_icon_cell_has_the_styled_icon_gap_label_and_the_cells_text_colour() {
    for (theme, shadcn) in COMBINATIONS {
        let d = harness_with(
            theme,
            shadcn,
            |style| {
                style.segmented.sizes.default_entry_mut().icon_size = 18.0;
                style.segmented.sizes.default_entry_mut().icon_gap = 7.0;
                style.segmented.sizes.default_entry_mut().column_gap = 3.0;
            },
            icon_cells,
        );
        let seg = d.app().style.segmented.clone();
        let cells = MODES.map(|(mode, _)| d.rect(Target::suffix(&cell_path(mode))));
        for pair in cells.windows(2) {
            assert!(
                (pair[1].min.x - pair[0].max.x - 3.0).abs() < 0.01,
                "{theme:?} {shadcn:?}: the column gap separates the cells ({pair:?})"
            );
        }
        for (index, (mode, label)) in MODES.into_iter().enumerate() {
            let cell = cells[index];
            let text = text_box(&d, cell, label);
            let selected = mode == Mode::Light;
            let colors = if selected {
                seg.selected.states.rest
            } else {
                seg.unselected.states.rest
            };
            if mode == Mode::System {
                assert!(
                    shapes(&d).into_iter().all(|shape| !matches!(
                        shape,
                        Shape::Mesh(mesh) if cell.contains_rect(mesh.calc_bounds())
                    )),
                    "a cell without an icon draws none"
                );
                continue;
            }
            let (tint, icon) = icon_in(&d, cell);
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
        let (_, unselected_before) = icon_in(&d, cells[1]);
        assert!(unselected_before.width() > 0.0);
    }
}

#[test]
fn an_icon_follows_the_hover_colour_and_clicking_the_cell_chooses_it() {
    let mut d = harness_with(
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
    d.hover(Target::suffix(&dark));
    d.frames(12);
    let (tint, _) = icon_in(&d, d.rect(Target::suffix(&dark)));
    assert_eq!(tint, Color32::from_rgb(200, 30, 30), "tinted by hover text");
    d.click(Target::suffix(&dark));
    d.frames(2);
    assert_eq!(d.app().mode, Mode::Dark);
    let label = d
        .harness()
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
        b.segmented("seg", app.mode, MODES)
            .track(tw().w_full())
            .cells(tw().flex_1())
            .on_change(|app: &mut App, mode| app.mode = mode);
    });
}

#[test]
fn track_and_cell_classes_merge_over_the_styled_ones_and_keep_the_style() {
    for (theme, shadcn) in COMBINATIONS {
        let mut d = harness_with(theme, shadcn, |_| {}, merged_segmented);
        let seg = d.app().style.segmented.clone();
        let track = d.rect(Target::suffix("/\"seg\""));
        assert_eq!(track.width(), 400.0, "w_full replaced w_fit");
        assert_eq!(
            track.height(),
            seg.sizes.get(Size::DEFAULT).height,
            "the styled height stays"
        );
        let cells = MODES.map(|(mode, _)| d.rect(Target::suffix(&cell_path(mode))));
        let widths: Vec<_> = cells.iter().map(|cell| cell.width()).collect();
        assert!(
            widths.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01),
            "flex-1 shares the track equally: {widths:?}"
        );
        assert!(widths[0] > 100.0, "precondition: the cells are wide");
        let selected = box_at(&d, cells[0]);
        assert_eq!(selected.fill, seg.selected.states.rest.fill, "styled paint");
        for (mode, label) in MODES {
            let cell = d.rect(Target::suffix(&cell_path(mode)));
            let text = text_box(&d, cell, label);
            assert!(
                (text.center().x - cell.center().x).abs() <= 0.75,
                "{theme:?} {shadcn:?} {label}: centred across a wide cell"
            );
        }
        d.click(Target::suffix(&cell_path(Mode::System)));
        d.frames(2);
        assert_eq!(d.app().mode, Mode::System, "still interactive");
    }
}

/// One of every owned widget, each with a caller width.
fn classed(b: &mut Builder<'_, '_, App>, _: &App) {
    let wide = || tw().w_px(321.0);
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        b.button("button", "Save").classes(wide()).add();
        b.checkbox("checkbox", "Agree", false)
            .classes(wide())
            .on_change(|_: &mut App, _| {});
        b.radio_group("radio", Mode::Light, MODES)
            .classes(wide())
            .on_change(|_: &mut App, _| {});
        b.switch("switch", "Alerts", false)
            .classes(wide())
            .on_change(|_: &mut App, _| {});
        b.slider("slider", 0.5, 0.0..=1.0)
            .named("Volume")
            .classes(wide())
            .on_change(|_: &mut App, _| {});
        b.select("select", Mode::Light, MODES)
            .named("Mode")
            .classes(wide())
            .on_change(|_: &mut App, _| {});
        b.stepper("stepper", 1.0, 0.0..=9.0)
            .named("Count")
            .classes(wide())
            .on_change(|_: &mut App, _| {});
        b.segmented("segmented", Mode::Light, MODES)
            .classes(wide())
            .on_change(|_: &mut App, _| {});
    });
}

#[test]
fn every_owned_widget_takes_caller_classes_over_its_own() {
    for (theme, shadcn) in COMBINATIONS {
        let d = harness_with(theme, shadcn, |_| {}, classed);
        let style = &d.app().style;
        for path in [
            "/\"button\"",
            "/\"checkbox\"/<div>",
            "/\"radio\"",
            "/\"switch\"/<div>",
            "/\"slider\"/Track",
            "/\"select\"",
            "/\"stepper\"/Box",
            "/\"segmented\"",
        ] {
            assert_eq!(rect_of(&d, path).width(), 321.0, "{path}");
        }
        let height = |path| d.rect(Target::suffix(path)).height();
        assert_eq!(
            height("/\"button\""),
            style.button.sizes.get(Size::DEFAULT).height
        );
        assert_eq!(
            height("/\"select\""),
            style.select.sizes.get(Size::DEFAULT).height
        );
        assert_eq!(
            height("/\"stepper\"/Box"),
            style.stepper.sizes.get(Size::DEFAULT).height
        );
        assert_eq!(
            height("/\"segmented\""),
            style.segmented.sizes.get(Size::DEFAULT).height
        );
    }
}

// ----- #43 item 5: a text role per widget section -----

fn roles(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        b.button("button", "Save").add();
        b.segmented("seg", app.mode, MODES)
            .on_change(|_: &mut App, _| {});
        b.select("select", Mode::Light, MODES)
            .named("Mode")
            .on_change(|_: &mut App, _| {});
        b.checkbox("check", "Agree", false)
            .on_change(|_: &mut App, _| {});
    });
}

fn size_of_text(d: &Harn, inside: &str, text: &str) -> f32 {
    shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(t)
                if t.galley.text() == text
                    && d.rect(Target::suffix(inside)).expand(1.0).contains(t.pos) =>
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
        let base = plain.app().style.text.size;
        for (path, text) in [
            ("/\"button\"", "Save"),
            ("/\"seg\"/0", "Light"),
            ("/\"select\"", "Light"),
            ("/\"check\"/<div>", "Agree"),
        ] {
            assert_eq!(size_of_text(&plain, path, text), base, "{path}: default");
        }
        let d = harness_with(
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
            ("/\"seg\"/0", "Light", 17.0),
            ("/\"select\"", "Light", 19.0),
            ("/\"check\"/<div>", "Agree", 23.0),
        ] {
            assert_eq!(size_of_text(&d, path, text), size, "{path}");
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
    changed.segmented.sizes.default_entry_mut().icon_gap += 1.0;
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
    let colors = |fill| eggshell_ui::contract::Colors {
        fill,
        border: Color32::TRANSPARENT,
        text: Color32::WHITE,
    };
    Paint::new(eggshell_ui::contract::States::still(
        colors(Color32::from_rgb(10, 120, 60)),
        Color32::BLACK,
    ))
}

fn toggles(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(2.0), |b| {
        b.toggle("plain", "Plain", app.on)
            .on_change(|app: &mut App, on| app.on = on);
        b.toggle("tool", "Tool", app.on)
            .variant(ARMED)
            .on_change(|app: &mut App, on| app.on = on);
    });
}

#[test]
fn a_toggle_takes_its_off_and_on_looks_from_its_variant_and_keeps_toggling() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut d = harness_with(
            theme,
            ShadcnStyle::Vega,
            |style| {
                let off = Paint::new(eggshell_ui::contract::States::still(
                    eggshell_ui::contract::Colors {
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
        let style = d.app().style.clone();
        let plain = d.rect(Target::suffix("/\"plain\""));
        let tool = d.rect(Target::suffix("/\"tool\""));
        assert_eq!(
            box_at(&d, plain).fill,
            style
                .button
                .toggle_off
                .get(Variant::DEFAULT)
                .states
                .rest
                .fill,
            "the default variant is the old toggle"
        );
        assert_eq!(box_at(&d, tool).fill, Color32::from_rgb(40, 40, 40));
        d.click(Target::suffix("/\"tool\""));
        d.frames(14);
        assert!(d.app().on, "a toggle variant still toggles");
        assert_eq!(box_at(&d, tool).fill, Color32::from_rgb(10, 120, 60));
        assert_eq!(
            box_at(&d, plain).fill,
            style
                .button
                .toggle_on
                .get(Variant::DEFAULT)
                .states
                .rest
                .fill
        );
        assert_eq!(
            text_color(&d, tool, "Tool"),
            Color32::WHITE,
            "the variant's text colour"
        );
        for name in ["Plain", "Tool"] {
            assert_eq!(
                d.harness()
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
            b.button(("button", enablement), "Save")
                .variant(eggshell_ui_shadcn::variant::PRIMARY)
                .enablement(enablement)
                .on_click(|app: &mut App| app.clicks += 1);
            b.checkbox(("check", enablement), "Agree", true)
                .enablement(enablement)
                .on_change(|app: &mut App, _| app.clicks += 1);
        }
    });
}

#[test]
fn a_faded_disabled_control_is_node_opacity_over_its_rest_colours() {
    for (theme, shadcn) in COMBINATIONS {
        let opacity = 0.4;
        let mut d = harness_with(
            theme,
            shadcn,
            |style| style.set_disabled(Disabled::Faded(Opacity::new(opacity))),
            disabled_row,
        );
        let style = d.app().style.clone();
        let rest = style
            .button
            .variants
            .get(eggshell_ui_shadcn::variant::PRIMARY)
            .paint
            .states
            .rest;
        let enabled = d.rect(Target::suffix("/(\"button\", Enabled)"));
        let disabled = d.rect(Target::suffix("/(\"button\", Disabled)"));
        assert_eq!(box_at(&d, enabled).fill, rest.fill, "precondition");
        let fill = box_at(&d, disabled).fill;
        assert_eq!(
            fill,
            rest.fill.gamma_multiply(opacity),
            "{theme:?} {shadcn:?}: the rest fill at node opacity, translucent not opaque"
        );
        assert!(fill.a() < 255 && fill.a() > 0);
        assert_eq!(
            text_color(&d, disabled, "Save"),
            rest.text.gamma_multiply(opacity),
            "the label fades with its node"
        );
        let label = d.rect(Target::suffix("/(\"check\", Disabled)/<div>"));
        let enabled_label = style.label.rest.text;
        assert_eq!(
            text_color(&d, label, "Agree"),
            enabled_label.gamma_multiply(opacity),
            "a choice's label fades with its row"
        );
        d.click(Target::suffix("/(\"button\", Disabled)"));
        d.click(Target::suffix("/(\"check\", Disabled)/<div>"));
        assert_eq!(d.app().clicks, 0, "still inert");
        d.click(Target::suffix("/(\"button\", Enabled)"));
        assert_eq!(d.app().clicks, 1);
    }
}

#[test]
fn a_faded_control_on_a_translucent_surface_lets_the_surface_through() {
    let translucent = Color32::from_rgba_unmultiplied(20, 20, 220, 128);
    let mut d = harness_with(
        Theme::Light,
        ShadcnStyle::Vega,
        |style| {
            style.set_disabled(Disabled::Faded(Opacity::new(0.3)));
            let look = style
                .button
                .variants
                .get_mut(eggshell_ui_shadcn::variant::PRIMARY)
                .expect("primary");
            look.paint.states.rest.fill = translucent;
        },
        disabled_row,
    );
    let disabled = d.rect(Target::suffix("/(\"button\", Disabled)"));
    let fill = box_at(&d, disabled).fill;
    assert_eq!(fill, translucent.gamma_multiply(0.3));
    assert!(
        fill.a() < translucent.a(),
        "fainter than the enabled fill, so the surface shows through"
    );
    d.frames(1);
}

#[test]
fn the_colors_form_still_draws_the_styles_disabled_colours() {
    let d = harness(Theme::Light, disabled_row);
    let style = &d.app().style;
    assert_eq!(style.disabled(), Disabled::Colors);
    let primary = style
        .button
        .variants
        .get(eggshell_ui_shadcn::variant::PRIMARY);
    let disabled = d.rect(Target::suffix("/(\"button\", Disabled)"));
    assert_eq!(
        box_at(&d, disabled).fill,
        primary.paint.states.disabled.fill
    );
}

// ----- #43 item 10: icon buttons -----

fn icon_buttons(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().gap(2.0).items_center(), |b| {
        b.button("icon", (app.icon(), "Search"))
            .size(size::ICON)
            .variant(eggshell_ui_shadcn::variant::GHOST)
            .on_click(|app: &mut App| app.clicks += 1);
        b.button("labelled", "Add")
            .icon(app.icon())
            .on_click(|app: &mut App| app.clicks += 10);
        b.toggle("toggle", (app.icon(), "Pin"), app.on)
            .size(size::ICON)
            .on_change(|app: &mut App, on| app.on = on);
    });
}

#[test]
fn an_icon_button_is_a_glyph_in_the_size_and_named_for_assistive_technology() {
    for (theme, shadcn) in COMBINATIONS {
        let mut d = harness_with(theme, shadcn, |_| {}, icon_buttons);
        let style = d.app().style.clone();
        let metrics = *style.button.sizes.get(size::ICON);
        let button = d.rect(Target::suffix("/\"icon\""));
        assert_eq!(
            button.size(),
            vec2(style.button.sizes.get(size::ICON).min_width, metrics.height),
            "{theme:?} {shadcn:?}: a square control"
        );
        let (tint, glyph) = icon_in(&d, button);
        assert_eq!(glyph.size(), vec2(metrics.glyph, metrics.glyph));
        assert!(glyph.center().distance(button.center()) <= 0.75, "centred");
        let ghost = style
            .button
            .variants
            .get(eggshell_ui_shadcn::variant::GHOST);
        assert_eq!(tint, ghost.paint.states.rest.text);

        d.hover(Target::suffix("/\"icon\""));
        d.frames(14);
        let (hovered, _) = icon_in(&d, button);
        assert_eq!(hovered, ghost.paint.states.hover.text, "tinted per state");

        let labelled = d.rect(Target::suffix("/\"labelled\""));
        let default = style.button.sizes.get(Size::DEFAULT);
        let (_, glyph) = icon_in(&d, labelled);
        assert_eq!(glyph.size(), vec2(default.glyph, default.glyph));
        let text = text_box(&d, labelled, "Add");
        assert!(
            (text.min.x - glyph.max.x - default.gap).abs() < 0.75,
            "icon, then the control gap, then the label"
        );

        d.click(Target::suffix("/\"icon\""));
        d.click(Target::suffix("/\"toggle\""));
        d.frames(2);
        assert_eq!(d.app().clicks, 1);
        assert!(d.app().on);
        let _ = d.harness().get_by_role_and_label(Role::Button, "Search");
        assert_eq!(
            d.harness()
                .get_by_role_and_label(Role::Button, "Pin")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
    }
}

/// The name assistive technology reads for the button drawn at `path`.
fn read_name(d: &Harn, path: &str) -> String {
    let centre = d.rect(Target::suffix(path)).center();
    let mut found = d
        .harness()
        .query_all_by_role(Role::Button)
        .filter(|node| node.rect().contains(centre));
    let node = found.next();
    assert!(node.is_some(), "an accessible button at {path}");
    assert!(found.next().is_none(), "one accessible button at {path}");
    node.and_then(|node| node.accesskit_node().label())
        .unwrap_or_default()
}

#[test]
fn an_icon_faces_name_is_what_assistive_technology_reads() {
    let d = harness(Theme::Light, icon_buttons);
    assert_eq!(read_name(&d, "/\"icon\""), "Search", "an icon button");
    assert_eq!(read_name(&d, "/\"toggle\""), "Pin", "an icon toggle");
    assert_eq!(read_name(&d, "/\"labelled\""), "Add", "a text face");
}

fn icon_dropdown(b: &mut Builder<'_, '_, App>, app: &App) {
    b.dropdown_menu("more", (app.icon(), "More")).items(|m| {
        m.item("copy", "Copy")
            .on_click(|app: &mut App| app.clicks += 1)
    });
}

#[test]
fn an_icon_dropdowns_name_names_its_trigger_and_its_menu() {
    let mut d = harness(Theme::Light, icon_dropdown);
    assert_eq!(read_name(&d, "/\"more\""), "More", "the trigger");
    d.click(Target::suffix("/\"more\""));
    d.frames(4);
    let menu = d.harness().get_by_role(Role::Menu);
    assert_eq!(
        menu.accesskit_node().label().as_deref(),
        Some("More"),
        "the menu"
    );
}

fn renamed_faces(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().gap(2.0).items_center(), |b| {
        b.button("role", "Viewer").named("Role").add();
        b.toggle("bold", "B", app.on)
            .named("Bold")
            .on_change(|app: &mut App, on| app.on = on);
        b.dropdown_menu("levels", "···")
            .named("The levels between")
            .items(|m| {
                m.item("docs", "Docs")
                    .on_click(|app: &mut App| app.clicks += 1)
            });
    });
}

#[test]
fn a_text_face_with_a_name_is_read_by_the_name_and_shows_the_text() {
    let d = harness(Theme::Light, renamed_faces);
    for (path, text, name) in [
        ("/\"role\"", "Viewer", "Role"),
        ("/\"bold\"", "B", "Bold"),
        ("/\"levels\"", "···", "The levels between"),
    ] {
        let rect = d.rect(Target::suffix(path));
        let _ = text_box(&d, rect, text);
        assert_eq!(read_name(&d, path), name, "{path}");
    }
}

#[test]
fn a_renamed_dropdown_names_its_trigger_and_its_menu() {
    let mut d = harness(Theme::Light, renamed_faces);
    d.click(Target::suffix("/\"levels\""));
    d.frames(4);
    let menu = d.harness().get_by_role(Role::Menu);
    assert_eq!(
        menu.accesskit_node().label().as_deref(),
        Some("The levels between"),
        "the menu"
    );
    assert_eq!(
        read_name(&d, "/\"levels\""),
        "The levels between",
        "the trigger"
    );
}

fn overridden_names(b: &mut Builder<'_, '_, App>, app: &App) {
    let named = || eggshell_ui::owned::ButtonOptions::new().named("From options");
    b.div(tw().flex().gap(2.0).items_center(), |b| {
        b.button("icon", (app.icon(), "Search")).named("Find").add();
        b.button("empty", Face::Empty("Card".into()))
            .named("Open the card")
            .add();
        b.button("options", "Text").with_options(named()).add();
        b.button("after", "Text")
            .with_options(named())
            .named("From the chain")
            .add();
        b.button("before", "Text")
            .named("From the chain")
            .with_options(named())
            .add();
        b.toggle_item(
            &eggshell_ui::owned::ToggleGroup::new("group", 0u8),
            1,
            "One",
        )
        .named("The first")
        .add();
    });
}

#[test]
fn named_overrides_whatever_name_the_face_gives() {
    let d = harness(Theme::Light, overridden_names);
    assert_eq!(read_name(&d, "/\"icon\""), "Find", "an icon face's tuple");
    assert_eq!(
        read_name(&d, "/\"empty\""),
        "Open the card",
        "an empty face"
    );
    assert_eq!(read_name(&d, "/\"options\""), "From options", "options");
    assert_eq!(
        read_name(&d, "/\"after\""),
        "From the chain",
        "set after options"
    );
    assert_eq!(
        read_name(&d, "/\"before\""),
        "From options",
        "options set after"
    );
    let member = d.harness().get_by_label("The first");
    assert_eq!(
        member.accesskit_node().role(),
        Role::RadioButton,
        "a group member"
    );
}

fn texture_icon() -> Icon {
    Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
        egui::TextureId::default(),
        vec2(4.0, 4.0),
    )))
}

fn named_icons(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().gap(2.0), |b| {
        b.button("tuple", (app.icon(), "Search")).add();
        b.button("named", app.icon()).named("Search").add();
        b.toggle("toggle", app.icon(), app.on)
            .named("Pin")
            .on_change(|app: &mut App, on| app.on = on);
    });
}

#[cfg(debug_assertions)]
#[test]
fn a_bare_icon_face_is_reported_unnamed() {
    let mut headless = eggshell_ui::testing::Headless::new(vec2(200.0, 100.0));
    headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _| {
        b.button("bare", texture_icon()).add();
    });
    let report =
        eggshell_ui::testing::assert_reported(headless.ctx(), eggshell_ui::Problem::Unnamed);
    assert!(report.message.contains("\"bare\""), "{}", report.message);
}

#[test]
fn an_icon_named_by_its_tuple_or_by_named_is_not_reported() {
    let d = harness(Theme::Light, named_icons);
    assert_eq!(read_name(&d, "/\"named\""), "Search");
}

#[cfg(debug_assertions)]
#[test]
fn a_button_with_neither_label_nor_name_is_reported() {
    let mut headless = eggshell_ui::testing::Headless::new(vec2(200.0, 100.0));
    headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _| {
        b.button("nameless", "").add();
    });
    let report =
        eggshell_ui::testing::assert_reported(headless.ctx(), eggshell_ui::Problem::Unnamed);
    assert!(
        report.message.contains("no label and no name"),
        "{}",
        report.message
    );
}

#[test]
fn a_button_with_an_empty_label_and_a_name_is_not_reported() {
    let _d = harness(Theme::Light, |b, _| {
        b.button("named", Face::Empty("Named".into())).add()
    });
}

/// ```html
/// <button aria-pressed="false"><img> Pin</button>
/// ```
fn icon_toggle(b: &mut Builder<'_, '_, App>, app: &App) {
    b.toggle("toggle", "Pin", app.on)
        .icon(app.icon())
        .on_change(|app: &mut App, on| app.on = on);
}

#[test]
fn a_toggle_paints_its_icon_before_its_label_in_its_on_and_off_looks() {
    for (theme, shadcn) in COMBINATIONS {
        let mut d = harness_with(theme, shadcn, |_| {}, icon_toggle);
        let style = d.app().style.clone();
        let toggle = d.rect(Target::suffix("/\"toggle\""));
        let default = style.button.sizes.get(Size::DEFAULT);
        let (tint, glyph) = icon_in(&d, toggle);
        assert_eq!(glyph.size(), vec2(default.glyph, default.glyph));
        let text = text_box(&d, toggle, "Pin");
        assert!(
            (text.min.x - glyph.max.x - default.gap).abs() < 0.75,
            "{theme:?} {shadcn:?}: icon, then the control gap, then the label"
        );
        let off = style.button.toggle_off.get(Variant::DEFAULT).states.rest;
        assert_eq!(tint, off.text, "{theme:?} {shadcn:?}: tinted off");

        d.click(Target::suffix("/\"toggle\""));
        d.hover(egui::pos2(510.0, 390.0));
        d.frames(14);
        assert!(d.app().on, "the click toggled it");
        let on = style.button.toggle_on.get(Variant::DEFAULT).states.rest;
        assert_eq!(box_at(&d, toggle).fill, on.fill, "drawn in the on look");
        let (tint, _) = icon_in(&d, toggle);
        assert_eq!(tint, on.text, "{theme:?} {shadcn:?}: tinted on");
        assert_eq!(
            d.harness()
                .get_by_role_and_label(Role::Button, "Pin")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
    }
}
// ----- #44: the stepper's number text fits its box -----

fn stepper(b: &mut Builder<'_, '_, App>, _: &App) {
    b.stepper("stepper", 3.0, 0.0..=9.0)
        .named("Count")
        .on_change(|_: &mut App, _| {});
}

#[test]
fn the_steppers_number_field_fits_inside_the_group_border() {
    for (theme, shadcn) in COMBINATIONS {
        let d = harness_with(theme, shadcn, |_| {}, stepper);
        let style = &d.app().style;
        let group = d.rect(Target::suffix("/\"stepper\"/Box"));
        let inside = group.shrink(style.stepper.group.border_width);
        let field_box = d.rect(Target::suffix("/\"stepper\"/Box/<div>"));
        let field = d.rect(Target::suffix("/\"stepper\"/Box/Field"));
        assert_eq!(
            group.height(),
            style.stepper.sizes.get(Size::DEFAULT).height,
            "precondition"
        );
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

fn family_of(d: &Harn, text: &str) -> egui::FontFamily {
    shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.galley.job.sections[0].format.font_id.family.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text:?} was painted"))
}

fn geist(weight: Weight) -> FontFamily {
    FontFamily::Name(format!("Geist/{}", weight.number()).into())
}

fn plain_segmented(b: &mut Builder<'_, '_, App>, app: &App) {
    b.segmented("seg", app.mode, MODES)
        .on_change(|app: &mut App, mode| app.mode = mode);
}

#[test]
fn the_selected_cell_draws_its_own_weight_and_the_others_keep_the_base() {
    let family = |selected_weight| {
        let d = harness_with(
            Theme::Light,
            ShadcnStyle::default(),
            |style| {
                let text = style.role(&style.segmented.text).clone();
                style.segmented.text = Some(TextSpec {
                    weight: Weight::Medium,
                    ..text
                });
                style.segmented.selected_weight = selected_weight;
            },
            plain_segmented,
        );
        (
            family_of(&d, "Light"),
            family_of(&d, "Dark"),
            family_of(&d, "System"),
        )
    };
    let base = family(None);
    assert_eq!(
        base,
        (
            geist(Weight::Medium),
            geist(Weight::Medium),
            geist(Weight::Medium)
        ),
        "precondition: with no selected weight every cell draws the base"
    );
    assert_eq!(
        family(Some(Weight::SemiBold)),
        (
            geist(Weight::SemiBold),
            geist(Weight::Medium),
            geist(Weight::Medium)
        ),
        "Light is the selected cell"
    );
}
