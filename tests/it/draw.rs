//! The draw phase through the public API, asserted on the shapes egui is
//! handed and on what the draw closures observe, so no GPU is needed.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bullseye::{Icon, LayoutState, NodeUi, Opacity, TextSpec, TextStyle, Tracking, tw};
use egui::epaint::{ClippedShape, Shape};
use egui::{Color32, FontFamily, FontId, Id, Pos2, RawInput, Rect, pos2, vec2};
use egui_kittest::Harness;

pub(super) const SCREEN: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(800.0, 600.0),
};

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

/// Builds `build` over the whole screen, draws it with `app`, and returns the shapes.
pub(super) fn shapes<A>(
    app: &mut A,
    pixels_per_point: f32,
    build: impl for<'f> FnOnce(&mut bullseye::Builder<'_, 'f, A>),
) -> Vec<ClippedShape> {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(pixels_per_point);
    let input = RawInput {
        screen_rect: Some(SCREEN),
        ..RawInput::default()
    };
    ctx.run_ui(input.clone(), |_| {})
        .drop_without_applying_deltas();
    let mut build = Some(build);
    let mut output = ctx.run_ui(input, |ui| {
        let build = build.take().expect("one pass");
        LayoutState::new()
            .with_base_text(base())
            .build(ui.ctx(), build)
            .layout(SCREEN)
            .draw(&mut *app, ui);
    });
    output.textures_delta.clear();
    output.shapes
}

fn rects(shapes: &[ClippedShape]) -> Vec<(&egui::epaint::RectShape, Rect)> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) => Some((rect, clipped.clip_rect)),
            _ => None,
        })
        .collect()
}

fn texts(shapes: &[ClippedShape]) -> Vec<(&egui::epaint::TextShape, Rect)> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) => Some((text, clipped.clip_rect)),
            _ => None,
        })
        .collect()
}

fn filled(shapes: &[ClippedShape], color: Color32) -> Vec<(Rect, Rect)> {
    rects(shapes)
        .into_iter()
        .filter(|(shape, _)| shape.fill == color)
        .map(|(shape, clip)| (shape.rect, clip))
        .collect()
}

// ------------------------------------------------------------------ box paint

/// ```html
/// <div class="w-[100px] h-[50px] bg-[#ff0000] rounded-lg border-2 border-[#0000ff]"></div>
/// ```
#[test]
fn a_uniform_border_and_background_are_one_rounded_rect() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().w_px(100.0)
                .h_px(50.0)
                .bg(Color32::RED)
                .rounded_lg()
                .border(2.0)
                .border_color(Color32::BLUE),
            |_| {},
        );
    });
    let rects = rects(&shapes);
    assert_eq!(rects.len(), 1);
    let (shape, _) = rects[0];
    assert_eq!(
        shape.rect,
        Rect::from_min_size(Pos2::ZERO, vec2(100.0, 50.0))
    );
    assert_eq!(shape.fill, Color32::RED);
    assert_eq!(shape.stroke.width, 2.0);
    assert_eq!(shape.stroke.color, Color32::BLUE);
    assert_eq!(shape.corner_radius, egui::CornerRadius::same(8));
}

/// ```html
/// <div class="ml-[0.3px] w-[100px] h-[50px] rounded-lg border border-[#0000ff]"></div>
/// ```
#[test]
fn a_rounded_border_stays_one_stroke_where_snapping_makes_its_sides_unequal() {
    const SCALE: f32 = 1.5;
    let pixels = |value: f32| (value * SCALE).round();
    let (left, right) = (0.3_f32, 100.3_f32);
    assert_ne!(
        pixels(left + 1.0) - pixels(left),
        pixels(right) - pixels(right - 1.0),
        "precondition: the snapped left and right borders differ"
    );

    let shapes = shapes(&mut (), SCALE, |b| {
        b.div(
            tw().ml(0.3)
                .w_px(100.0)
                .h_px(50.0)
                .rounded_lg()
                .border(1.0)
                .border_color(Color32::BLUE),
            |_| {},
        );
    });
    let rects = rects(&shapes);
    assert_eq!(rects.len(), 1, "one shape, no side strips");
    let stroke = rects[0].0.stroke;
    assert_eq!(stroke.color, Color32::BLUE);
    assert_eq!(rects[0].0.corner_radius, egui::CornerRadius::same(8));
    assert!(stroke.width > 0.0);
    assert!(
        (stroke.width * SCALE).fract().abs() < 1e-4,
        "{} is off the pixel grid",
        stroke.width
    );
}

/// ```html
/// <div class="w-[100px] h-[50px] border-t-4 border-t-[#0000ff] border-b-2 border-b-[#00ff00]"></div>
/// ```
#[test]
fn borders_on_single_sides_are_strips_inside_the_border_box() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().w_px(100.0)
                .h_px(50.0)
                .border_t(4.0)
                .border_t_color(Color32::BLUE)
                .border_b(2.0)
                .border_b_color(Color32::GREEN),
            |_| {},
        );
    });
    let top = filled(&shapes, Color32::BLUE);
    let bottom = filled(&shapes, Color32::GREEN);
    assert_eq!(top.len() + bottom.len(), 2);
    assert_eq!(
        top[0].0,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 4.0))
    );
    assert_eq!(
        bottom[0].0,
        Rect::from_min_max(pos2(0.0, 48.0), pos2(100.0, 50.0))
    );
}

/// ```html
/// <div class="w-[100px] h-[50px] ring-2 ring-[#00ff00] shadow"></div>
/// ```
#[test]
fn ring_is_outside_the_box_and_shadow_is_under_it() {
    let shadow = egui::Shadow {
        offset: [0, 4],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().w_px(100.0)
                .h_px(50.0)
                .bg(Color32::RED)
                .ring(2.0, Color32::GREEN)
                .shadow(shadow),
            |_| {},
        );
    });
    let rects = rects(&shapes);
    assert_eq!(rects.len(), 3, "shadow, background, ring: {rects:?}");
    assert_eq!(rects[0].0.fill, shadow.color);
    assert!(rects[0].0.blur_width > 0.0);
    assert_eq!(rects[1].0.fill, Color32::RED);
    assert_eq!(rects[2].0.stroke.color, Color32::GREEN);
    assert_eq!(rects[2].0.stroke_kind, egui::StrokeKind::Outside);
}

/// ```html
/// <div class="w-[100px] h-[50px] shadow-[0_4px_8px_#00000050]"></div>
/// ```
#[test]
fn a_shadow_never_shows_inside_a_box_that_does_not_cover_it() {
    let shadow = egui::Shadow {
        offset: [0, 4],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    for background in [None, Some(Color32::from_white_alpha(100))] {
        let shapes = shapes(&mut (), 1.0, |b| {
            let look = tw().w_px(100.0).h_px(50.0).shadow(shadow);
            b.div(background.map_or(look.clone(), |c| look.bg(c)), |_| {});
        });
        let border = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 50.0));
        let painted: Vec<_> = rects(&shapes)
            .into_iter()
            .filter(|(rect, _)| rect.fill == shadow.color && rect.blur_width > 0.0)
            .collect();
        assert!(!painted.is_empty(), "precondition: the shadow is painted");
        for (rect, clip) in &painted {
            assert!(
                rect.rect.intersects(border),
                "precondition: it is cast around the box: {rect:?}"
            );
            assert!(
                !clip.intersects(border.shrink(0.5)),
                "{background:?}: shadow clip {clip:?} reaches into {border:?}"
            );
        }
        let beside = |x: f32, y: f32| painted.iter().any(|(_, clip)| clip.contains(pos2(x, y)));
        assert!(beside(50.0, 52.0), "it still shows below");
        assert!(beside(102.0, 25.0), "and to the right");
        assert!(!beside(50.0, 25.0), "but not in the middle");
    }
}

/// ```html
/// <div class="flex w-[300px] h-[40px] divide-x-2 divide-[#808080]">
///   <div class="w-[100px]"></div> <div class="w-[100px]"></div> <div class="w-[100px]"></div>
/// </div>
/// ```
#[test]
fn dividers_are_drawn_between_adjacent_children_only() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().flex()
                .w_px(300.0)
                .h_px(40.0)
                .divide_x(2.0, Color32::GRAY),
            |b| {
                for _ in 0..3 {
                    b.div(tw().w_px(100.0), |_| {});
                }
            },
        );
    });
    let lines = filled(&shapes, Color32::GRAY);
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0].0,
        Rect::from_min_max(pos2(99.0, 0.0), pos2(101.0, 40.0))
    );
    assert_eq!(
        lines[1].0,
        Rect::from_min_max(pos2(199.0, 0.0), pos2(201.0, 40.0))
    );
}

/// ```html
/// <div class="flex flex-col w-[100px] divide-y-1 divide-[#808080]">
///   <div class="h-[20px]"></div> <div class="h-[20px]"></div>
/// </div>
/// ```
#[test]
fn divide_y_separates_a_column() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().flex()
                .flex_col()
                .w_px(100.0)
                .divide_y(1.0, Color32::GRAY),
            |b| {
                b.div(tw().h_px(20.0), |_| {});
                b.div(tw().h_px(20.0), |_| {});
            },
        );
    });
    let lines = filled(&shapes, Color32::GRAY);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0].0,
        Rect::from_min_max(pos2(0.0, 20.0), pos2(100.0, 21.0)),
        "the line is a whole pixel wide and starts on the seam"
    );
}

// ------------------------------------------------------------------ opacity and inertness

/// ```html
/// <div class="opacity-50">
///   <div class="opacity-50 w-[10px] h-[10px] bg-[#ff0000]"></div>
/// </div>
/// ```
#[test]
fn opacity_multiplies_down_the_subtree() {
    let seen = Rc::new(Cell::new(f32::NAN));
    let probe = Rc::clone(&seen);
    let shapes = shapes(&mut (), 1.0, move |b| {
        b.div(tw().opacity(Opacity::new(0.5)), |b| {
            b.div_id(
                "child",
                tw().opacity(Opacity::new(0.5))
                    .w_px(10.0)
                    .h_px(10.0)
                    .bg(Color32::RED),
                |_| {},
            )
            .draw(move |_, ui| probe.set(ui.opacity()));
        });
    });
    let quarter = Color32::RED.gamma_multiply(0.25);
    assert_ne!(quarter, Color32::RED, "precondition: opacity is visible");
    assert_eq!(filled(&shapes, quarter).len(), 1);
    assert_eq!(seen.get(), 0.25);
}

/// ```html
/// <div class="pointer-events-none"> <div id="inner"> ... </div> </div>
/// <div id="outside"></div>
/// ```
#[test]
fn inert_disables_the_whole_subtree_and_only_it() {
    let enabled = Rc::new(RefCell::new(Vec::new()));
    let record = |name: &'static str| {
        let enabled = Rc::clone(&enabled);
        move |_: &mut (), ui: &mut egui::Ui| enabled.borrow_mut().push((name, ui.is_enabled()))
    };
    let (inner, deep, outside) = (record("inner"), record("deep"), record("outside"));
    shapes(&mut (), 1.0, move |b| {
        b.div(tw().inert(), |b| {
            b.div_id("inner", tw(), |b| {
                b.div(tw(), |b| {
                    b.div_id("deep", tw(), |_| {}).draw(deep);
                });
            })
            .draw(inner);
        });
        b.div_id("outside", tw(), |_| {}).draw(outside);
    });
    assert_eq!(
        *enabled.borrow(),
        [("inner", false), ("deep", false), ("outside", true)]
    );
}

// ------------------------------------------------------------------ order and clipping

/// ```html
/// <div class="relative size-full">
///   <div class="absolute inset-0 z-10 bg-[#ff0000]"></div>
///   <div class="absolute inset-0 z-0 bg-[#0000ff]"></div>
/// </div>
/// ```
#[test]
fn higher_z_paints_later_regardless_of_tree_order() {
    let stacked = |first: i16, second: i16| {
        shapes(&mut (), 1.0, move |b| {
            b.div(tw().relative().size_full(), |b| {
                b.div(tw().absolute().inset(0.0).z(first).bg(Color32::RED), |_| {});
                b.div(
                    tw().absolute().inset(0.0).z(second).bg(Color32::BLUE),
                    |_| {},
                );
            });
        })
    };
    let order = |shapes: &[ClippedShape]| {
        rects(shapes)
            .iter()
            .map(|(shape, _)| shape.fill)
            .collect::<Vec<_>>()
    };

    let red_on_top = stacked(10, 0);
    let (red, blue) = (
        filled(&red_on_top, Color32::RED)[0].0,
        filled(&red_on_top, Color32::BLUE)[0].0,
    );
    assert!(red.intersects(blue), "precondition: the siblings overlap");
    assert_eq!(order(&red_on_top), [Color32::BLUE, Color32::RED]);
    assert_eq!(order(&stacked(0, 10)), [Color32::RED, Color32::BLUE]);
    assert_eq!(
        order(&stacked(0, 0)),
        [Color32::RED, Color32::BLUE],
        "equal z keeps tree order"
    );
}

/// ```html
/// <div class="w-[100px] h-[50px] overflow-hidden bg-[#00ff00]">
///   <div class="w-[200px] h-[200px] bg-[#ff0000]"></div>
/// </div>
/// ```
#[test]
fn overflow_hidden_clips_the_subtree_but_not_the_node_itself() {
    let probe = Rc::new(Cell::new(Rect::NOTHING));
    let seen = Rc::clone(&probe);
    let shapes = shapes(&mut (), 1.0, move |b| {
        b.div(
            tw().w_px(100.0)
                .h_px(50.0)
                .overflow_hidden()
                .bg(Color32::GREEN),
            |b| {
                b.div_id(
                    "child",
                    tw().w_px(200.0).h_px(200.0).bg(Color32::RED),
                    |_| {},
                )
                .draw(move |_, ui| seen.set(ui.clip_rect()));
            },
        );
    });
    let parent = Rect::from_min_size(Pos2::ZERO, vec2(100.0, 50.0));
    let (child, child_clip) = filled(&shapes, Color32::RED)[0];
    assert!(
        child.max.x > parent.max.x && child.max.y > parent.max.y,
        "precondition: the child overflows its parent"
    );
    assert_eq!(child_clip, parent);
    assert_eq!(probe.get(), parent, "the draw closure's Ui is clipped too");
    let (own, own_clip) = filled(&shapes, Color32::GREEN)[0];
    assert_eq!(own, parent);
    assert!(
        own_clip.contains_rect(SCREEN),
        "a node's own box is not clipped by itself"
    );
}

/// ```html
/// <div class="w-[100px] h-[50px] overflow-x-hidden">
///   <div class="w-[200px] h-[200px] bg-[#ff0000]"></div>
/// </div>
/// ```
#[test]
fn overflow_clips_each_axis_separately() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(tw().w_px(100.0).h_px(50.0).overflow_x_hidden(), |b| {
            b.div(tw().w_px(200.0).h_px(200.0).bg(Color32::RED), |_| {});
        });
    });
    let (child, clip) = filled(&shapes, Color32::RED)[0];
    assert!(
        child.height() > 50.0,
        "precondition: it overflows vertically"
    );
    assert_eq!((clip.min.x, clip.max.x), (0.0, 100.0));
    assert!(clip.height() > 200.0, "no clipping vertically: {clip:?}");
}

/// ```html
/// <div class="w-[100px] h-[50px] p-2 border-4 overflow-hidden">
///   <div class="w-[200px] h-[200px] bg-[#ff0000]"></div>
/// </div>
/// ```
#[test]
fn clipping_is_to_the_padding_box() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().w_px(100.0)
                .h_px(50.0)
                .p(2.0)
                .border(4.0)
                .overflow_hidden(),
            |b| {
                b.div(tw().w_px(200.0).h_px(200.0).bg(Color32::RED), |_| {});
            },
        );
    });
    let clip = filled(&shapes, Color32::RED)[0].1;
    assert_eq!(clip, Rect::from_min_max(pos2(4.0, 4.0), pos2(96.0, 46.0)));
}

// ------------------------------------------------------------------ pixels

/// ```html
/// <div class="ml-[10.3px] mt-[5.1px] w-[20.4px] h-[10.2px] bg-[#ff0000]"></div>
/// ```
#[test]
fn rects_snap_to_physical_pixels() {
    const SCALE: f32 = 1.5;
    let exact = |value: f32| value * SCALE;
    let on_grid = |value: f32| (value * SCALE).fract().abs() < 1e-4;

    let shapes = shapes(&mut (), SCALE, |b| {
        b.div(
            tw().ml(2.575)
                .mt(1.275)
                .w_px(20.4)
                .h_px(10.2)
                .bg(Color32::RED),
            |_| {},
        );
    });
    let rect = filled(&shapes, Color32::RED)[0].0;
    for value in [10.3, 5.1, 10.3 + 20.4, 5.1 + 10.2] {
        assert!(
            exact(value).fract() > 1e-3,
            "precondition: {value} is off the pixel grid"
        );
    }
    for value in [rect.min.x, rect.min.y, rect.max.x, rect.max.y] {
        assert!(on_grid(value), "{value} is off the pixel grid");
    }
    assert_eq!(rect.min, pos2(10.0, 16.0 / 3.0));
}

// ------------------------------------------------------------------ primitives

fn text_of(shapes: &[ClippedShape], text: &str) -> (egui::epaint::TextShape, Rect) {
    let (shape, clip) = texts(shapes)
        .into_iter()
        .find(|(shape, _)| shape.galley.text() == text)
        .unwrap_or_else(|| panic!("{text:?} was painted"));
    (shape.clone(), clip)
}

/// ```html
/// <div class="w-[400px] text-[#ff0000] italic underline uppercase tracking-[1px] text-center">
///   hello
/// </div>
/// ```
#[test]
fn text_is_painted_in_the_resolved_style() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(
            tw().w_px(400.0)
                .text(Color32::RED)
                .italic()
                .underline()
                .uppercase()
                .tracking(Tracking::points(1.0))
                .text_center(),
            |b| b.text(tw(), "hello"),
        );
    });
    let (shape, _) = text_of(&shapes, "HELLO");
    assert_eq!(shape.fallback_color, Color32::RED);
    let format = &shape.galley.job.sections[0].format;
    assert!(format.italics);
    assert!(!format.underline.is_empty());
    assert_eq!(format.extra_letter_spacing, 1.0);
    assert_eq!(format.font_id, base().spec.font_id());
    assert_eq!(shape.pos.x, 200.0, "centred in the 400 point box");
    assert_eq!(shape.pos.y, 0.0);
}

/// ```html
/// <div class="w-[80px] truncate">A sentence far too long for its box</div>
/// ```
#[test]
fn truncated_text_ends_in_an_ellipsis_within_its_box() {
    let long = "A sentence far too long for its box";
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(tw().w_px(80.0).truncate(), |b| b.text(tw(), long));
    });
    let (shape, clip) = texts(&shapes)
        .into_iter()
        .next()
        .map(|(shape, clip)| (shape.clone(), clip))
        .expect("text was painted");
    assert!(shape.galley.elided);
    let last_glyph = shape
        .galley
        .rows
        .last()
        .and_then(|row| row.row.glyphs.last());
    assert_eq!(last_glyph.map(|glyph| glyph.chr), Some('…'));
    assert!(shape.galley.size().x <= 80.0);
    assert_eq!(clip.width(), 80.0, "truncate also clips");
}

/// ```html
/// <div class="w-[60px]">alpha beta gamma delta</div>
/// ```
#[test]
fn normal_text_wraps_in_its_box() {
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div(tw().w_px(60.0), |b| b.text(tw(), "alpha beta gamma delta"));
    });
    let (shape, _) = text_of(&shapes, "alpha beta gamma delta");
    assert!(shape.galley.rows.len() > 1);
    assert!(shape.galley.size().x <= 60.0);
}

/// The word must not break where pixel snapping narrows its box.
///
/// ```html
/// <div class="flex">
///   <div class="ml-[Npx]">Blog</div>
/// </div>
/// ```
#[test]
fn a_word_stays_on_one_row_where_snapping_narrows_its_box() {
    const SCALE: f32 = 3.0;
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(SCALE);
    ctx.run_ui(RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    let galley = |width: f32| {
        let mut job = egui::text::LayoutJob::single_section(
            "Blog".to_owned(),
            egui::TextFormat::simple(base().spec.font_id(), Color32::BLACK),
        );
        job.wrap.max_width = width;
        ctx.fonts_mut(|fonts| fonts.layout_job(job))
    };
    let width = galley(f32::INFINITY).size().x;
    let snap = |value: f32| (value * SCALE).round() / SCALE;
    let offset = (0..12)
        .map(|step| 10.0 + step as f32 / 12.0)
        .find(|&offset| galley(snap(offset + width) - snap(offset)).rows.len() > 1)
        .expect("precondition: at some offset the snapped box is too narrow for the word");

    let shapes = shapes(&mut (), SCALE, |b| {
        b.div(tw().flex(), |b| {
            b.div(tw().ml(offset / 4.0), |b| b.text(tw(), "Blog"));
        });
    });
    let (shape, _) = text_of(&shapes, "Blog");
    assert_eq!(shape.galley.rows.len(), 1, "broke at x = {offset}");
}

/// ```html
/// <svg class="size-[16px] text-[#ff0000]"> ... </svg>
/// ```
#[test]
fn icons_are_tinted_with_the_text_colour() {
    let ctx = egui::Context::default();
    let texture = ctx.load_texture(
        "icon",
        egui::ColorImage::filled([4, 4], Color32::WHITE),
        egui::TextureOptions::NEAREST,
    );
    let icon = Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
        texture.id(),
        vec2(4.0, 4.0),
    )))
    .flipped(bullseye::Flip::Horizontal);
    let shapes = shapes(&mut (), 1.0, move |b| {
        b.div(tw().text(Color32::RED), |b| {
            b.glyph(tw().size_px(16.0), icon);
        });
    });
    let mesh = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Mesh(mesh) => Some(mesh),
            _ => None,
        })
        .expect("the icon was painted as a textured mesh");
    assert!(mesh.vertices.iter().all(|v| v.color == Color32::RED));
    assert_eq!(mesh.calc_bounds().size(), vec2(16.0, 16.0));
    let uv_x = |at: Pos2| {
        mesh.vertices
            .iter()
            .find(|v| v.pos == at)
            .expect("a corner")
            .uv
            .x
    };
    assert_eq!(
        (uv_x(pos2(0.0, 0.0)), uv_x(pos2(16.0, 0.0))),
        (1.0, 0.0),
        "a horizontal flip mirrors the uv"
    );
}

/// ```html
/// <div class="w-[200px] h-[100px] p-2" style="..."></div>
/// ```
#[test]
fn paint_closures_receive_the_content_box() {
    let seen = Rc::new(Cell::new(Rect::NOTHING));
    let probe = Rc::clone(&seen);
    shapes(&mut (), 1.0, move |b| {
        b.div(tw().w_px(200.0).h_px(100.0).p(2.0), |b| {
            b.paint(tw().w_full().h_full(), move |_, rect, _| probe.set(rect));
        });
    });
    assert_eq!(
        seen.get(),
        Rect::from_min_max(pos2(8.0, 8.0), pos2(192.0, 92.0))
    );
}

// ------------------------------------------------------------------ draw closures

#[derive(Default)]
struct Counter {
    visits: Vec<&'static str>,
}

/// ```html
/// <div id="outer"> <div id="inner"></div> </div> <div id="sibling"></div>
/// ```
#[test]
fn draw_closures_mutate_the_application_in_pre_order() {
    let mut app = Counter::default();
    let record =
        |name: &'static str| move |app: &mut Counter, _: &mut egui::Ui| app.visits.push(name);
    let (outer, inner, sibling) = (record("outer"), record("inner"), record("sibling"));
    shapes(&mut app, 1.0, move |b| {
        b.div_id("outer", tw(), |b| {
            b.div_id("inner", tw().h_px(10.0), |_| {}).draw(inner);
        })
        .draw(outer);
        b.span("sibling", tw(), "after").draw(sibling);
        b.div_id("skipped", tw().hidden(), |_| {});
    });
    assert_eq!(app.visits, ["outer", "inner", "sibling"]);
}

/// ```html
/// <div id="styled" class="font-mono text-[31px] text-[#ff0000]"> <label>styled</label> </div>
/// ```
#[test]
fn the_nodes_text_style_reaches_egui_widgets_in_its_ui() {
    let family = FontFamily::Monospace;
    let shapes = shapes(&mut (), 1.0, |b| {
        b.div_id(
            "styled",
            tw().role(TextSpec {
                size: 31.0,
                line_height: 40.0,
                family: family.clone(),
            })
            .text(Color32::RED),
            |_| {},
        )
        .draw(|_, ui| {
            ui.label("styled");
        });
    });
    let (label, _) = text_of(&shapes, "styled");
    assert_eq!(label.fallback_color, Color32::RED);
    assert_eq!(
        label.galley.job.sections[0].format.font_id,
        FontId::new(31.0, FontFamily::Monospace)
    );
}

// ------------------------------------------------------------------ identity

/// Where a node is drawn from: the egui parent or layer its `Ui` descends from.
#[derive(Clone, Copy, Debug)]
enum Host {
    Root,
    PushedId,
    SaltedScope,
    Layer,
}

/// The node's id as built, and the node and scope ids its `Ui` reports when drawn from `host`.
fn node_ui_ids(host: Host) -> (Id, Vec<(Id, Id)>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let expected = Rc::new(Cell::new(Id::NULL));
    let mut layout = LayoutState::new().with_base_text(base());
    let mut draw_into = |ui: &mut egui::Ui| {
        let probe = Rc::clone(&seen);
        layout
            .build(ui.ctx(), |b| {
                expected.set(b.id_of("node"));
                b.div_id("node", tw().w_px(10.0).h_px(10.0), |_| {})
                    .draw(move |_: &mut (), ui| {
                        probe.borrow_mut().push((ui.node_id(), ui.scope_id()))
                    });
            })
            .layout(SCREEN)
            .draw(&mut (), ui);
    };
    let mut harness = Harness::new_ui(|ui| {
        seen.borrow_mut().clear();
        match host {
            Host::Root => draw_into(ui),
            Host::PushedId => {
                ui.push_id("another parent", |ui| draw_into(ui));
            }
            Host::SaltedScope => {
                ui.scope_builder(egui::UiBuilder::new().id_salt("deeper"), |ui| draw_into(ui));
            }
            Host::Layer => {
                egui::Area::new(Id::unique("a layer")).show(ui.ctx(), |ui| draw_into(ui));
            }
        }
    });
    harness.run();
    drop(harness);
    (expected.get(), seen.take())
}

/// ```html
/// <div id="node"></div>
/// ```
#[test]
fn a_node_ui_knows_the_nodes_id_and_has_a_stable_scope_under_any_parent_or_layer() {
    let mut scopes = Vec::new();
    for host in [Host::Root, Host::PushedId, Host::SaltedScope, Host::Layer] {
        let (id, seen) = node_ui_ids(host);
        assert_ne!(id, Id::NULL, "precondition: the node has an id");
        let [(node, scope)] = seen[..] else {
            panic!("{host:?}: one draw, saw {seen:?}")
        };
        assert_eq!(node, id, "{host:?}");
        assert_ne!(scope, id, "{host:?}: the node's own id is its widget's");
        scopes.push(scope);
    }
    assert!(
        scopes.windows(2).all(|pair| pair[0] == pair[1]),
        "{scopes:?}"
    );
}

// ------------------------------------------------------------------ egui backdrop

/// The `bg_color` each draw closure's `Ui` reports, in a central panel filled
/// with `PANEL`.
fn observed_backdrops(
    build: impl for<'f> FnOnce(&mut bullseye::Builder<'_, 'f, Vec<(&'static str, Color32)>>),
) -> Vec<(&'static str, Color32)> {
    const PANEL: Color32 = Color32::from_rgb(10, 20, 30);
    let ctx = egui::Context::default();
    let input = RawInput {
        screen_rect: Some(SCREEN),
        ..RawInput::default()
    };
    let mut seen = Vec::new();
    let mut build = Some(build);
    ctx.run_ui(input.clone(), |_| {})
        .drop_without_applying_deltas();
    let mut output = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(PANEL))
            .show(ui, |ui| {
                let build = build.take().expect("one pass");
                LayoutState::new()
                    .with_base_text(base())
                    .build(ui.ctx(), build)
                    .layout(SCREEN)
                    .draw(&mut seen, ui);
            });
    });
    output.textures_delta.clear();
    seen
}

fn probe(
    name: &'static str,
) -> impl FnOnce(&mut Vec<(&'static str, Color32)>, &mut egui::Ui) + 'static {
    move |seen, ui| seen.push((name, ui.stack().bg_color()))
}

/// ```html
/// <div class="bg-[#c80000]" id="plain">
///   <div class="bg-[#0000ff80]" id="nested"></div>
/// </div>
/// <div id="sibling"></div>
/// ```
#[test]
fn egui_sees_the_backgrounds_bullseye_painted_as_the_stack_backdrop() {
    let panel = Color32::from_rgb(10, 20, 30);
    let under = Color32::from_rgb(200, 0, 0);
    let over = Color32::from_rgba_unmultiplied(0, 0, 255, 128);
    let seen = observed_backdrops(|b| {
        b.div_id("bare", tw(), |_| {}).draw(probe("bare"));
        b.div_id("plain", tw().bg(under), |b| {
            b.div_id("nested", tw().bg(over), |_| {})
                .draw(probe("nested"));
        })
        .draw(probe("plain"));
        b.div_id("sibling", tw(), |_| {}).draw(probe("sibling"));
    });
    assert_ne!(under.blend(over), over, "precondition: B is translucent");
    assert_eq!(
        seen,
        [
            ("bare", panel),
            ("plain", under),
            ("nested", under.blend(over)),
            ("sibling", panel),
        ]
    );
}
