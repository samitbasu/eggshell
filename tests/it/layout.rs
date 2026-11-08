//! Layout tests through the public API: build, lay out, assert rects.
//!
//! The kaolin README examples are the acceptance layouts. Each asserts its
//! preconditions (the text is wide enough to wrap, the siblings differ) so it
//! cannot silently stop testing anything.

use std::sync::OnceLock;

use bullseye::{
    Builder, FontStyle, Icon, LayoutState, NodeKind, Placed, TextDecoration, TextSpec, TextStyle,
    Track, none, tw,
};

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/custom_background.rs"]
mod custom_background;
#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flex.rs"]
mod flex;
#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flex_grid.rs"]
mod flex_grid;
#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flex_wrap.rs"]
mod flex_wrap;
#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flexbox_guide.rs"]
mod flexbox_guide;
#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/grow.rs"]
mod grow;
#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/holy_grail.rs"]
mod holy_grail;
#[path = "../../examples/kaolin/layouts.rs"]
mod layouts;
use egui::epaint::text::LayoutJob;
use egui::{Color32, Context, Id, Pos2, Rect, pos2, vec2};
use layouts::{BODY, LONG, WRAPPING, mono, sized};

const AREA: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(800.0, 600.0),
};

fn base() -> TextStyle {
    TextStyle::new(BODY, Color32::BLACK)
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| {
        let ctx = Context::default();
        ctx.run_ui(Default::default(), |_| {})
            .drop_without_applying_deltas();
        ctx
    })
}

fn fixed_state() -> LayoutState {
    LayoutState::new().with_base_text(base())
}

fn place(f: impl for<'f> FnOnce(&mut Builder<'_, 'f, ()>)) -> Placed<()> {
    place_in(AREA, f)
}

fn place_in(area: Rect, f: impl for<'f> FnOnce(&mut Builder<'_, 'f, ()>)) -> Placed<()> {
    place_app(area, f)
}

fn place_app<A>(area: Rect, f: impl for<'f> FnOnce(&mut Builder<'_, 'f, A>)) -> Placed<A> {
    fixed_state().build(ctx(), f).layout(area)
}

/// The unwrapped width of `text`, from egui directly.
fn width_of(spec: &TextSpec, text: &str) -> f32 {
    ctx().fonts_mut(|fonts| {
        fonts
            .layout_job(LayoutJob::simple(
                text.to_owned(),
                spec.font_id(),
                Color32::WHITE,
                f32::INFINITY,
            ))
            .size()
            .x
    })
}

fn rects<A>(placed: &Placed<A>, kind: NodeKind) -> Vec<Rect> {
    placed
        .nodes()
        .filter(|node| node.kind() == kind)
        .map(|node| node.rect())
        .collect()
}

#[track_caller]
fn assert_rect(actual: Rect, [x0, y0, x1, y1]: [f32; 4]) {
    let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(
        close(actual.min.x, x0)
            && close(actual.min.y, y0)
            && close(actual.max.x, x1)
            && close(actual.max.y, y1),
        "expected [{x0}, {y0} .. {x1}, {y1}], got {actual:?}"
    );
}

#[track_caller]
fn assert_centre(actual: Rect, expected: Pos2) {
    let centre = actual.center();
    assert!(
        (centre - expected).length() < 1e-3,
        "expected centre {expected:?}, got {centre:?} of {actual:?}"
    );
}

// ---------------------------------------------------------------- M0 exit

/// ```html
/// <div class="flex size-full items-center justify-center">Hello</div>
/// ```
#[test]
fn hello_is_centred_in_a_flex_container() {
    let placed = place(|b| {
        b.div(
            tw().flex().size_full().items_center().justify_center(),
            |b| {
                b.text(tw(), "Hello");
            },
        );
    });
    let text = rects(&placed, NodeKind::Text)[0];
    let width = width_of(&BODY, "Hello");
    assert!(width > 0.0 && width < AREA.width(), "precondition");
    assert_eq!(text.size(), vec2(width, 20.0));
    assert_centre(text, AREA.center());
}

// ---------------------------------------------------------------- kaolin 1

/// See [`layouts::hello_world`].
#[test]
fn kaolin_hello_world() {
    let placed = place(layouts::hello_world);
    let div = rects(&placed, NodeKind::Div)[0];
    assert_rect(div, [0.0, 0.0, 800.0, 600.0]);
    let text = rects(&placed, NodeKind::Text)[0];
    let width = width_of(&sized(48.0), "Hello, World!");
    assert!(width < 800.0, "precondition: the text fits");
    assert_eq!(text.size(), vec2(width, 60.0));
    assert_centre(text, AREA.center());
    let div_node = placed.nodes().find(|n| n.kind() == NodeKind::Div).unwrap();
    assert_eq!(
        div_node.paint().background,
        Some(bullseye::style::Shadcn::of(ctx()).tokens.background)
    );
}

// ---------------------------------------------------------------- kaolin 2

/// ```html
/// <div class="flex flex-row items-center justify-center gap-2.5 w-[300px] h-[100px]">
///   <div class="w-[40px] h-[20px]"></div>
///   <div class="w-[50px] h-[20px]"></div>
///   <div class="w-[60px] h-[20px]"></div>
/// </div>
/// <!-- variants of the container: flex-row-reverse, flex-col, flex-col-reverse (in place of flex-row) -->
/// ```
#[test]
fn kaolin_direction_and_alignment() {
    let container = || layouts::direction_and_alignment().w_px(300.0).h_px(100.0);
    let rects_of = |tw: bullseye::Tw| {
        let placed = place(|b| b.div(tw, layouts::three_boxes));
        rects(&placed, NodeKind::Div)[1..].to_vec()
    };

    assert_eq!(
        layouts::direction_and_alignment().style().gap.width,
        taffy::LengthPercentage::length(10.0)
    );

    // LeftToRight: 150 of content plus two 10 point gaps, centred in 300 x 100.
    let row = rects_of(container());
    assert_rect(row[0], [65.0, 40.0, 105.0, 60.0]);
    assert_rect(row[1], [115.0, 40.0, 165.0, 60.0]);
    assert_rect(row[2], [175.0, 40.0, 235.0, 60.0]);

    // RightToLeft
    let reversed = rects_of(container().flex_row_reverse());
    assert_rect(reversed[0], [195.0, 40.0, 235.0, 60.0]);
    assert_rect(reversed[2], [65.0, 40.0, 125.0, 60.0]);

    // TopToBottom: 60 of content plus two gaps.
    let column = rects_of(container().flex_col());
    assert_rect(column[0], [130.0, 10.0, 170.0, 30.0]);
    assert_rect(column[1], [125.0, 40.0, 175.0, 60.0]);
    assert_rect(column[2], [120.0, 70.0, 180.0, 90.0]);

    // BottomToTop
    let flipped = rects_of(container().flex_col_reverse());
    assert_rect(flipped[0], [130.0, 70.0, 170.0, 90.0]);
    assert_rect(flipped[2], [120.0, 10.0, 180.0, 30.0]);
}

// ---------------------------------------------------------------- kaolin 3

/// The first child of a 600 x 300 row that does not stretch its children.
/// ```html
/// <div class="flex w-[600px] h-[300px] items-start">
///   <div class="..."><!-- one of the sizing modes, with the content text if any --></div>
/// </div>
/// ```
fn sized_child(style: bullseye::Tw, content: &'static str) -> Rect {
    let placed = place(|b| {
        b.div(tw().flex().w_px(600.0).h_px(300.0).items_start(), |b| {
            b.div(style, |b| {
                if !content.is_empty() {
                    b.text(tw(), content)
                }
            })
        })
    });
    rects(&placed, NodeKind::Div)[1]
}

/// ```html
/// <div class="w-[200px] h-[100px]"></div>
/// <div class="size-[150px]"></div>
/// ```
#[test]
fn kaolin_sizing_fixed() {
    let [fixed, square, ..] = layouts::sizing_modes();
    assert_eq!(sized_child(fixed, "").size(), vec2(200.0, 100.0));
    assert_eq!(sized_child(square, "").size(), vec2(150.0, 150.0));
}

/// ```html
/// <div>Fit me</div>
/// ```
#[test]
fn kaolin_sizing_fit() {
    let [_, _, fit, ..] = layouts::sizing_modes();
    let width = width_of(&BODY, "Fit me");
    assert_eq!(sized_child(fit, "Fit me").size(), vec2(width, 20.0));
}

/// ```html
/// <div class="max-w-[320px]">This is a long text that will automatically wrap to multiple lines of text</div>
/// ```
#[test]
fn kaolin_sizing_fit_with_max_width() {
    let [_, _, _, max, ..] = layouts::sizing_modes();
    assert!(
        width_of(&BODY, LONG) > 320.0,
        "precondition: the text overflows"
    );
    let rect = sized_child(max, LONG);
    assert!(rect.width() <= 320.0);
    assert!(rect.height() > 20.0, "wrapped");
    assert_eq!(rect.height() % 20.0, 0.0);
}

/// ```html
/// <div class="min-w-[120px] max-w-[320px]">Hi</div>
/// ```
#[test]
fn kaolin_sizing_fit_with_min_and_max_width() {
    let [_, _, _, _, bounded, ..] = layouts::sizing_modes();
    assert!(
        width_of(&BODY, "Hi") < 120.0,
        "precondition: the text is narrower than the minimum"
    );
    assert_eq!(sized_child(bounded, "Hi").width(), 120.0);
}

/// ```html
/// <div class="flex w-[600px] h-[300px] items-start">
///   <div class="flex-1"></div>
/// </div>
/// ```
#[test]
fn kaolin_sizing_grow() {
    let [.., grow, _, _] = layouts::sizing_modes();
    assert_eq!(sized_child(grow, "").width(), 600.0);
}

/// ```html
/// <div class="flex w-[600px] h-[300px]">
///   <div class="grow-[2]"></div>
///   <div class="grow"></div>
/// </div>
/// ```
#[test]
fn kaolin_sizing_proportional_grow_factor() {
    let [.., _, double, _] = layouts::sizing_modes();
    let placed = place(|b| {
        b.div(tw().flex().w_px(600.0).h_px(300.0), |b| {
            b.div(double, |_| {});
            b.div(tw().grow(), |_| {});
        })
    });
    let divs = rects(&placed, NodeKind::Div);
    assert_eq!((divs[1].width(), divs[2].width()), (400.0, 200.0));
}

/// ```html
/// <div class="flex w-[600px] h-[300px] items-start">
///   <div class="flex-1 min-w-[120px] max-w-[480px]"></div>
/// </div>
/// ```
#[test]
fn kaolin_sizing_grow_with_bounds() {
    let [.., bounded] = layouts::sizing_modes();
    assert_eq!(sized_child(bounded, "").width(), 480.0);
}

// ---------------------------------------------------------------- kaolin 4

/// See [`layouts::styling`].
#[test]
fn kaolin_styling() {
    let placed = place(layouts::styling);
    let width = width_of(&mono(24.0), "Styled text");
    assert!(width + 36.0 < 800.0, "precondition: the text fits");
    let styled = rects(&placed, NodeKind::Div)[0];
    assert_rect(styled, [0.0, 0.0, 800.0, 66.0]);
    let text = rects(&placed, NodeKind::Text)[0];
    assert_rect(text, [18.0, 18.0, 782.0, 48.0]);
    assert!(text.width() > width, "a block child fills the content box");

    let node = placed.nodes().find(|n| n.kind() == NodeKind::Div).unwrap();
    assert_eq!(node.paint().background, Some(Color32::BLUE));
    assert_eq!(node.paint().radius, egui::CornerRadius::same(8));
}

// ---------------------------------------------------------------- kaolin 5

/// See [`layouts::responsive`].
#[test]
fn kaolin_responsive() {
    let placed = place(layouts::responsive);
    let divs = rects(&placed, NodeKind::Div);
    assert_rect(divs[1], [0.0, 0.0, 800.0, 60.0]);
    assert_rect(divs[2], [0.0, 60.0, 800.0, 560.0]);
    assert_rect(divs[3], [0.0, 560.0, 800.0, 600.0]);

    let texts: Vec<_> = placed
        .nodes()
        .filter(|n| n.kind() == NodeKind::Text)
        .collect();
    assert_eq!(texts[0].text().color, Color32::WHITE);
    assert_eq!(
        texts[1].text().color,
        Color32::BLACK,
        "main content does not inherit white"
    );
    assert_eq!(texts[2].text().color, Color32::WHITE);
}

/// ```html
/// <div class="flex flex-col size-full">
///   <div class="w-full h-[60px]"></div>
///   <div class="w-full flex-1"></div>
/// </div>
/// ```
#[test]
fn kaolin_responsive_follows_the_area() {
    let placed = place_in(
        Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 300.0)),
        |b| {
            b.div(tw().flex().flex_col().size_full(), |b| {
                b.div(tw().w_full().h_px(60.0), |_| {});
                b.div(tw().w_full().flex_1(), |_| {});
            });
        },
    );
    let divs = rects(&placed, NodeKind::Div);
    assert_rect(divs[1], [10.0, 20.0, 410.0, 80.0]);
    assert_rect(divs[2], [10.0, 80.0, 410.0, 320.0]);
}

// ---------------------------------------------------------------- kaolin 6

/// See [`layouts::multi_column`].
#[test]
fn kaolin_multi_column() {
    let placed = place(layouts::multi_column);
    let divs = rects(&placed, NodeKind::Div);
    assert_rect(divs[1], [0.0, 0.0, 200.0, 600.0]);
    assert_rect(divs[2], [220.0, 0.0, 800.0, 600.0]);
}

// ---------------------------------------------------------------- kaolin 7

/// See [`layouts::text_wrapping`].
#[test]
fn kaolin_text_wrapping() {
    assert!(
        width_of(&BODY, WRAPPING) > 200.0,
        "precondition: the text does not fit one line"
    );
    let placed = place(layouts::text_wrapping);
    let container = rects(&placed, NodeKind::Div)[0];
    let wrapped = rects(&placed, NodeKind::Text)[0];
    assert_eq!(container.width(), 200.0);
    assert!(wrapped.width() <= 200.0);
    assert!(wrapped.height() >= 40.0, "at least two lines");
    assert_eq!(wrapped.height() % 20.0, 0.0, "whole lines");
    assert_eq!(
        container.height(),
        wrapped.height(),
        "the container fits its text"
    );
}

// ---------------------------------------------------------------- kaolin 8

/// See [`layouts::proportional_growth`].
#[test]
fn kaolin_proportional_growth() {
    let placed = place(layouts::proportional_growth);
    let divs = rects(&placed, NodeKind::Div);
    assert_rect(divs[1], [0.0, 0.0, 200.0, 600.0]);
    assert_rect(divs[2], [200.0, 0.0, 800.0, 600.0]);
}

// ---------------------------------------------------------------- kaolin 9

/// See [`layouts::nested`].
#[test]
fn kaolin_nested() {
    let placed = place(layouts::nested);
    let divs = rects(&placed, NodeKind::Div);
    assert_eq!(divs.len(), 4, "outer, two inner, one card");
    assert_rect(divs[0], [0.0, 0.0, 800.0, 16.0 + 36.0 + 8.0 + 54.0 + 16.0]);
    assert_rect(divs[1], [16.0, 16.0, 784.0, 52.0]);
    assert_rect(divs[2], [16.0, 60.0, 784.0, 114.0]);
    assert_rect(divs[3], [24.0, 68.0, 776.0, 106.0]);
    let texts = rects(&placed, NodeKind::Text);
    assert_eq!(texts[0].min, pos2(24.0, 24.0));
    assert_eq!(texts[1].min, pos2(24.0 + 1.0 + 8.0, 68.0 + 1.0 + 8.0));
}

// ---------------------------------------------------------------- kaolin 10

/// See [`layouts::embedded_layout`].
#[test]
fn kaolin_embedded_layout() {
    let logo = egui::ImageSource::Uri("bytes://logo".into());
    let placed = place(move |b| layouts::embedded_layout(b, logo));
    let hello = width_of(&mono(10.0), "Hello, World!");
    let kaolin = width_of(&BODY, "Kaolin Renderer");
    let row_width = hello + 4.0 + 16.0;
    assert!(
        row_width != kaolin,
        "precondition: the two lines differ in width"
    );

    let divs = rects(&placed, NodeKind::Div);
    let row = divs[1];
    let top = (600.0 - 36.0) / 2.0;
    assert_rect(
        row,
        [
            (800.0 - row_width) / 2.0,
            top,
            (800.0 + row_width) / 2.0,
            top + 16.0,
        ],
    );
    let image = rects(&placed, NodeKind::Image)[0];
    assert_rect(image, [row.max.x - 16.0, top, row.max.x, top + 16.0]);
    let texts = rects(&placed, NodeKind::Text);
    assert_eq!(texts[0].min.x, row.min.x);
    assert_rect(
        texts[1],
        [
            (800.0 - kaolin) / 2.0,
            top + 16.0,
            (800.0 + kaolin) / 2.0,
            top + 36.0,
        ],
    );
}

// ---------------------------------------------------------------- identity

#[test]
fn identified_nodes_are_found_by_id_and_path() {
    let mut ids = Vec::new();
    let placed = place(|b| {
        b.scope("props", |b| {
            ids.push(b.id_of("title"));
            b.span("title", tw(), "Title", none);
            b.scope(("block", 2u32), |b| {
                ids.push(b.id_of("name"));
                b.span("name", tw(), "Name", none);
            });
        });
        b.text(tw(), "anonymous");
    });
    let title = placed.node(ids[0]).expect("title is placed");
    assert_eq!(title.path(), "/\"props\"/\"title\"");
    assert_eq!(title.kind(), NodeKind::Span);
    assert!(title.has_draw());
    assert_eq!(placed.rect(ids[0]), Some(title.rect()));
    assert_eq!(
        placed.node(ids[1]).unwrap().path(),
        "/\"props\"/(\"block\", 2)/\"name\""
    );
    assert_eq!(placed.rect(Id::NULL), None);
}

#[test]
fn dump_lists_every_node_including_anonymous_ones() {
    let placed = place(|b| {
        b.div(tw().flex().w_px(100.0).h_px(50.0), |b| {
            b.scope("s", |b| {
                b.span("t", tw(), "x", none);
                b.rule(tw().w_px(1.0));
            });
        });
    });
    let kinds: Vec<_> = placed.nodes().map(|n| n.kind()).collect();
    assert_eq!(
        kinds,
        [
            NodeKind::Root,
            NodeKind::Div,
            NodeKind::Span,
            NodeKind::Rule
        ]
    );
    assert_eq!(placed.len(), 4);
    assert!(!placed.is_empty());
    let dump = placed.dump();
    assert_eq!(dump.lines().count(), 4);
    assert!(dump.contains("/<div>"), "{dump}");
    assert!(dump.contains("/\"s\"/\"t\" Span"), "{dump}");
    assert!(dump.contains("/\"s\"/<rule> Rule"), "{dump}");
    assert!(
        dump.lines().nth(2).unwrap().starts_with("    "),
        "indented by depth: {dump}"
    );
}

#[test]
fn div_id_scopes_its_children_under_its_key() {
    let mut ids = Vec::new();
    let placed = place(|b| {
        for row in [1u32, 2] {
            b.div_id(("row", row), tw(), none, |b| {
                ids.push(b.id_of("label"));
                b.span("label", tw(), "x", none);
            });
        }
    });
    assert_ne!(ids[0], ids[1], "the same child key under different rows");
    for id in ids {
        assert!(placed.rect(id).is_some());
    }
    assert!(
        placed
            .nodes()
            .any(|n| n.path() == "/(\"row\", 2)/\"label\"")
    );
}

#[test]
fn enum_keys_of_different_types_in_one_scope_do_not_collide() {
    #[derive(Hash, Debug)]
    enum Appearance {
        Light,
    }
    #[derive(Hash, Debug)]
    enum Lettering {
        Basic,
    }
    let mut ids = Vec::new();
    let placed = place(|b| {
        ids.push(b.id_of(Appearance::Light));
        ids.push(b.id_of(Lettering::Basic));
        b.span(Appearance::Light, tw(), "a", none);
        b.span(Lettering::Basic, tw(), "b", none);
    });
    assert_ne!(ids[0], ids[1]);
    assert!(placed.rect(ids[0]).is_some() && placed.rect(ids[1]).is_some());
}

#[test]
fn integers_and_newtypes_in_one_scope_do_not_collide() {
    #[derive(Hash, Debug)]
    struct BlockKey(u32);
    let placed = place(|b| {
        b.span(1u32, tw(), "a", none);
        b.span(BlockKey(1), tw(), "b", none);
    });
    assert_eq!(placed.nodes().filter(|n| n.id().is_some()).count(), 2);
}

#[test]
fn the_same_key_may_recur_in_different_scopes() {
    let placed = place(|b| {
        b.scope("a", |b| b.span("title", tw(), "x", none));
        b.scope("b", |b| b.span("title", tw(), "x", none));
    });
    assert_eq!(placed.nodes().filter(|n| n.id().is_some()).count(), 2);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "duplicate key path /\"outer\"/\"dup\"")]
fn a_duplicate_path_panics_with_the_readable_path() {
    place(|b| {
        b.scope("outer", |b| {
            b.span("dup", tw(), "x", none);
            b.leaf("dup", tw(), bullseye::fill, none);
        });
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "duplicate key path /(\"row\", 1)")]
fn duplicate_identified_containers_panic() {
    place(|b| {
        b.div_id(("row", 1), tw(), none, |_| {});
        b.div_id(("row", 1), tw(), none, |_| {});
    });
}

#[test]
fn a_new_build_forgets_the_previous_frames_paths() {
    let mut state = fixed_state();
    for _ in 0..2 {
        state
            .build::<()>(ctx(), |b| b.span("same", tw(), "x", none))
            .layout(AREA);
    }
}

// ---------------------------------------------------------------- inheritance

fn text_styles(placed: &Placed<()>) -> Vec<TextStyle> {
    placed
        .nodes()
        .filter(|n| n.kind() == NodeKind::Text)
        .map(|n| n.text().clone())
        .collect()
}

/// ```html
/// outside
/// <div class="text-[#ff0000] italic text-[30px]/[37.5px]">
///   inherits all
///   <div class="text-[#00ff00]">
///     green, still italic and large
///     <span class="not-italic text-[10px]/[12.5px]">overrides</span>
///   </div>
/// </div>
/// after
/// ```
#[test]
fn text_classes_inherit_down_and_override_field_by_field() {
    let placed = place(|b| {
        b.text(tw(), "outside");
        b.div(tw().text(Color32::RED).italic().role(sized(30.0)), |b| {
            b.text(tw(), "inherits all");
            b.div(tw().text(Color32::GREEN), |b| {
                b.text(tw(), "green, still italic and large");
                b.text(tw().not_italic().role(sized(10.0)), "overrides");
            });
        });
        b.text(tw(), "after");
    });
    let styles = text_styles(&placed);
    let big = sized(30.0);

    assert_eq!(styles[0], base());
    assert_eq!(styles[4], base(), "the siblings after are unaffected");

    assert_eq!(styles[1].color, Color32::RED);
    assert_eq!(styles[1].font_style, FontStyle::Italic);
    assert_eq!(styles[1].spec, big);

    assert_eq!(styles[2].color, Color32::GREEN);
    assert_eq!(styles[2].font_style, FontStyle::Italic);
    assert_eq!(styles[2].spec, big);

    assert_eq!(
        styles[3].color,
        Color32::GREEN,
        "the colour is still inherited"
    );
    assert_eq!(styles[3].font_style, FontStyle::Normal);
    assert_eq!(styles[3].spec, sized(10.0));
}

/// ```html
/// <div class="text-[#ff0000]">
///   <div data-key="d" class="underline">deep</div>
/// </div>
/// <!-- the scope "s" has no markup: it only namespaces keys -->
/// ```
#[test]
fn inheritance_crosses_scopes_and_identified_containers() {
    let placed = place(|b| {
        b.div(tw().text(Color32::RED), |b| {
            b.scope("s", |b| {
                b.div_id("d", tw().underline(), none, |b| b.text(tw(), "deep"));
            })
        });
    });
    let style = &text_styles(&placed)[0];
    assert_eq!(style.color, Color32::RED);
    assert_eq!(style.decoration, TextDecoration::Underline);
}

/// ```html
/// <span data-key="s" class="text-[#0000ff] uppercase">x</span>
/// <img class="text-[#ff0000]" src="bytes://i"> <!-- the glyph: an icon coloured by text-* -->
/// ```
#[test]
fn the_nodes_own_classes_apply_to_its_own_content() {
    let placed = place(|b| {
        b.span("s", tw().text(Color32::BLUE).uppercase(), "x", none);
        b.glyph(
            tw().text(Color32::RED),
            Icon::new(egui::ImageSource::Uri("bytes://i".into())),
        );
    });
    let nodes: Vec<_> = placed.nodes().collect();
    assert_eq!(nodes[1].text().color, Color32::BLUE);
    assert_eq!(nodes[1].text().case, bullseye::TextCase::Upper);
    assert_eq!(nodes[2].text().color, Color32::RED);
    assert_eq!(nodes[2].text().case, bullseye::TextCase::Normal);
}

/// ```html
/// one line
/// <span class="text-[16px]/[40px]">one line</span>
/// ```
#[test]
fn line_height_comes_from_the_role() {
    let placed = place(|b| {
        b.text(tw(), "one line");
        b.text(
            tw().role(TextSpec {
                line_height: 40.0,
                ..BODY
            }),
            "one line",
        );
    });
    let heights: Vec<_> = rects(&placed, NodeKind::Text)
        .iter()
        .map(|r| r.height())
        .collect();
    assert_eq!(heights, [20.0, 40.0]);
}

// ---------------------------------------------------------------- measures

/// ```html
/// <div class="flex items-start">
///   wide wide
///   <span class="uppercase">wide wide</span>
/// </div>
/// ```
#[test]
fn case_changes_the_measured_width() {
    let placed = place(|b| {
        b.div(tw().flex().items_start(), |b| {
            b.text(tw(), "wide wide");
            b.text(tw().uppercase(), "wide wide");
        })
    });
    let texts = rects(&placed, NodeKind::Text);
    assert_ne!(texts[0].width(), texts[1].width());
    assert_eq!(texts[1].width(), width_of(&BODY, "WIDE WIDE"));
}

/// ```html
/// <div class="w-[100px]"><span class="whitespace-nowrap">several words that would wrap</span></div>
/// <div class="w-[100px]">several words that would wrap</div>
/// ```
#[test]
fn nowrap_text_keeps_one_line_in_a_narrow_box() {
    let text = "several words that would wrap";
    assert!(width_of(&BODY, text) > 100.0, "precondition");
    let placed = place(|b| {
        b.div(tw().w_px(100.0), |b| b.text(tw().whitespace_nowrap(), text));
        b.div(tw().w_px(100.0), |b| b.text(tw(), text));
    });
    let texts = rects(&placed, NodeKind::Text);
    assert_eq!(texts[0].height(), 20.0);
    assert!(texts[1].height() > 20.0);
}

/// ```html
/// <div class="flex items-start">
///   <div data-key="note"><!-- sized by the application's measure closure --></div>
///   <div data-key="fixed"><!-- 30 x 12, from the measure closure --></div>
///   <div data-key="sized by classes" class="w-[7px] h-[9px]"></div>
/// </div>
/// ```
#[test]
fn application_leaves_are_measured_with_borrowed_data() {
    let note = String::from("borrowed from the application");
    let mut state = fixed_state();
    let placed = state
        .build::<()>(ctx(), |b| {
            b.div(tw().flex().items_start(), |b| {
                b.leaf("note", tw(), |m| m.text_size(&note), none);
                b.leaf(
                    "fixed",
                    tw(),
                    |_| taffy::Size {
                        width: 30.0,
                        height: 12.0,
                    },
                    none,
                );
                b.leaf(
                    "sized by classes",
                    tw().w_px(7.0).h_px(9.0),
                    bullseye::fill,
                    none,
                );
            })
        })
        .layout(AREA);
    let leaves = rects(&placed, NodeKind::Leaf);
    assert_eq!(leaves[0].size(), vec2(width_of(&BODY, &note), 20.0));
    assert_eq!(leaves[1].size(), vec2(30.0, 12.0));
    assert_eq!(leaves[2].size(), vec2(7.0, 9.0));
}

/// ```html
/// <div class="flex items-start">
///   <img class="size-[1em]" src="bytes://icon">
///   <img class="size-[1em] text-[30px]/[37.5px]" src="bytes://icon">
///   <img class="size-[11px]" src="bytes://icon">
///   <img data-key="i" class="size-[1em]" src="bytes://icon">
/// </div>
/// <!-- bullseye glyphs and icons default to one em; HTML images have no such default, hence size-[1em] -->
/// ```
#[test]
fn glyphs_are_one_em_square_and_classes_override() {
    let src = || egui::ImageSource::Uri("bytes://icon".into());
    let placed = place(|b| {
        b.div(tw().flex().items_start(), |b| {
            b.glyph(tw(), Icon::new(src()));
            b.glyph(tw().role(sized(30.0)), Icon::new(src()));
            b.glyph(tw().size_px(11.0), Icon::new(src()));
            b.icon("i", tw(), Icon::new(src()), none);
        })
    });
    let sizes: Vec<_> = rects(&placed, NodeKind::Glyph)
        .iter()
        .chain(&rects(&placed, NodeKind::Icon))
        .map(|r| r.size())
        .collect();
    assert_eq!(
        sizes,
        [
            vec2(16.0, 16.0),
            vec2(30.0, 30.0),
            vec2(11.0, 11.0),
            vec2(16.0, 16.0)
        ]
    );
}

/// ```html
/// <div class="flex flex-col w-[100px]">
///   <div class="h-[1px] w-full bg-[#a0a0a0]"></div>
/// </div>
/// ```
#[test]
fn rules_are_boxes_sized_by_classes() {
    let placed = place(|b| {
        b.div(tw().flex().flex_col().w_px(100.0), |b| {
            b.rule(tw().h_px(1.0).w_full().bg(Color32::GRAY));
        })
    });
    assert_eq!(rects(&placed, NodeKind::Rule)[0].size(), vec2(100.0, 1.0));
}

/// ```html
/// <canvas class="size-[20px]"></canvas>
/// ```
#[test]
fn paint_nodes_are_laid_out_by_their_classes() {
    let placed = place(|b| b.paint(tw().size_px(20.0), |_, _| {}));
    assert_eq!(rects(&placed, NodeKind::Paint)[0].size(), vec2(20.0, 20.0));
}

/// ```html
/// <div class="flex w-[100px]">
///   <div class="flex-1"></div>
///   <div class="flex-1"></div>
///   <div class="flex-1"></div>
/// </div>
/// <!-- no pixel snapping: browsers snap layout to device pixels, bullseye does not -->
/// ```
#[test]
fn rounding_is_disabled() {
    let placed = place(|b| {
        b.div(tw().flex().w_px(100.0), |b| {
            for _ in 0..3 {
                b.div(tw().flex_1(), |_| {});
            }
        })
    });
    let widths: Vec<_> = rects(&placed, NodeKind::Div)[1..]
        .iter()
        .map(|r| r.width())
        .collect();
    assert!((widths[0] - 100.0 / 3.0).abs() < 1e-3, "{widths:?}");
}

/// ```html
/// <div class="flex flex-col w-[500px]">
///   <div>stretched</div>
///   <div class="w-fit">fitted</div>
/// </div>
/// ```
#[test]
fn w_fit_does_not_stretch_in_a_column() {
    let placed = place(|b| {
        b.div(tw().flex().flex_col().w_px(500.0), |b| {
            b.div(tw(), |b| b.text(tw(), "stretched"));
            b.div(tw().w_fit(), |b| b.text(tw(), "fitted"));
        })
    });
    let divs = rects(&placed, NodeKind::Div);
    assert_eq!(divs[1].width(), 500.0);
    assert_eq!(divs[2].width(), width_of(&BODY, "fitted"));
}

// ---------------------------------------------------------------- grid

/// ```html
/// <div class="grid w-[600px] grid-cols-[48px_minmax(0,1fr)_68px_73.6px_48px_20px]">
///   <div class="h-[10px]"></div>
///   <!-- x6 -->
/// </div>
/// ```
#[test]
fn grid_with_an_arbitrary_track_list() {
    let tracks = [
        Track::px(48.0),
        Track::minmax(Track::px(0.0), Track::fr(1.0)),
        Track::px(68.0),
        Track::px(73.6),
        Track::px(48.0),
        Track::px(20.0),
    ];
    let placed = place(|b| {
        b.div(tw().grid().w_px(600.0).grid_cols_tracks(&tracks), |b| {
            for _ in 0..6 {
                b.rule(tw().h_px(10.0));
            }
        })
    });
    let cells = rects(&placed, NodeKind::Rule);
    let flexible = 600.0 - (48.0 + 68.0 + 73.6 + 48.0 + 20.0);
    assert!(flexible > 0.0, "precondition: the flexible track has room");
    let mut x = 0.0;
    for (cell, width) in cells.iter().zip([48.0, flexible, 68.0, 73.6, 48.0, 20.0]) {
        assert_rect(*cell, [x, 0.0, x + width, 10.0]);
        x += width;
    }
}

/// ```html
/// <div class="grid w-[300px] grid-cols-3 gap-0">
///   <div class="h-[10px] col-span-2"></div>
///   <div class="h-[10px]"></div>
///   <div class="h-[10px] col-span-full"></div>
/// </div>
/// ```
#[test]
fn grid_columns_share_space_evenly_and_spans_cross_them() {
    let placed = place(|b| {
        b.div(tw().grid().w_px(300.0).grid_cols(3).gap(0.0), |b| {
            b.rule(tw().h_px(10.0).col_span(2));
            b.rule(tw().h_px(10.0));
            b.rule(tw().h_px(10.0).col_span_full());
        })
    });
    let cells = rects(&placed, NodeKind::Rule);
    assert_rect(cells[0], [0.0, 0.0, 200.0, 10.0]);
    assert_rect(cells[1], [200.0, 0.0, 300.0, 10.0]);
    assert_rect(cells[2], [0.0, 10.0, 300.0, 20.0]);
}

// ---------------------------------------------------------------- positioning

/// ```html
/// <div class="relative w-[200px] h-[100px]">
///   <div class="absolute inset-0"></div>
///   <div class="absolute top-[5px] right-[7px] size-[10px]"></div>
/// </div>
/// ```
#[test]
fn absolute_children_are_positioned_in_a_relative_parent() {
    let placed = place(|b| {
        b.div(tw().relative().w_px(200.0).h_px(100.0), |b| {
            b.div(tw().absolute().inset(0.0), |_| {});
            b.div(
                tw().absolute().top_px(5.0).right_px(7.0).size_px(10.0),
                |_| {},
            );
        })
    });
    let divs = rects(&placed, NodeKind::Div);
    assert_rect(divs[1], [0.0, 0.0, 200.0, 100.0]);
    assert_rect(divs[2], [183.0, 5.0, 193.0, 15.0]);
}

/// ```html
/// <div class="z-30"></div>
/// <div class="pointer-events-none"></div> <!-- nearest to bullseye's inert(): no event handling, no equivalent for layout -->
/// ```
#[test]
fn z_is_recorded_without_affecting_layout() {
    let placed = place(|b| {
        b.div(tw().z(30), |_| {});
        b.div(tw().inert(), |_| {});
    });
    let nodes: Vec<_> = placed.nodes().collect();
    assert_eq!(nodes[1].z(), 30);
    assert!(nodes[2].is_inert() && !nodes[1].is_inert());
}

// ------------------------------------------------------------ base text style

fn root_text(state: &mut LayoutState, ctx: &Context) -> TextStyle {
    let placed = state.build::<()>(ctx, |_| {}).layout(AREA);
    let root = placed
        .nodes()
        .find(|n| n.kind() == NodeKind::Root)
        .expect("every frame has a root");
    root.text().clone()
}

fn fresh_ctx() -> Context {
    let ctx = Context::default();
    ctx.run_ui(Default::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

#[test]
fn the_default_root_text_style_is_eguis() {
    let ctx = fresh_ctx();
    let expected = TextStyle::from_egui(&ctx);
    assert_ne!(
        expected,
        base(),
        "the default must differ from the override"
    );
    assert_eq!(root_text(&mut LayoutState::new(), &ctx), expected);
}

#[test]
fn the_override_replaces_the_default() {
    let ctx = fresh_ctx();
    let mut state = LayoutState::new().with_base_text(base());
    assert_ne!(TextStyle::from_egui(&ctx), base());
    assert_eq!(root_text(&mut state, &ctx), base());
}

#[test]
fn the_default_follows_the_theme() {
    let ctx = fresh_ctx();
    let mut state = LayoutState::new();
    ctx.set_visuals(egui::Visuals::dark());
    let dark = root_text(&mut state, &ctx).color;
    ctx.set_visuals(egui::Visuals::light());
    let light = root_text(&mut state, &ctx).color;
    assert_ne!(dark, light);
}

fn guide(title: &str, choice: usize) -> Placed<flexbox_guide::State> {
    let state = flexbox_guide::State::showing(title, choice);
    place_app(AREA, |b| flexbox_guide::guide(b, &state))
}

fn divs<A>(placed: &Placed<A>) -> Vec<Rect> {
    rects(placed, NodeKind::Div)
}

/// Tolerance for sums of fractional lengths.
#[track_caller]
fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-2,
        "expected {expected}, got {actual}"
    );
}

/// The padding and border of the guide's nav bars: 8 and 1 points.
const BAR_INSET: f32 = 9.0;

#[test]
fn guide_auto_margins_centre_the_child_on_both_axes() {
    let divs = divs(&guide("Perfect centering", 0));
    let (bar, child) = (divs[1], divs[2]);
    assert_eq!(bar.height(), 300.0);
    assert!(child.width() < bar.width() / 2.0, "room to centre in");
    assert_centre(child, bar.center());
}

#[test]
fn guide_six_items_distribute_evenly_and_wrap_with_the_window() {
    let state = flexbox_guide::State::showing("Six items, evenly distributed", 0);
    let at_width = |width: f32| {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 600.0));
        let divs = divs(&place_app(area, |b| flexbox_guide::guide(b, &state)));
        (divs[1], divs[2..8].to_vec())
    };

    let (bar, items) = at_width(800.0);
    assert!(items.iter().all(|item| item.min.y == items[0].min.y));
    let between = items[1].min.x - items[0].max.x;
    let edge = items[0].min.x - (bar.min.x + BAR_INSET);
    assert!(edge > 1.0, "free space to distribute");
    assert_close(between - 8.0, 2.0 * edge);

    let (bar, items) = at_width(400.0);
    assert!(items.iter().any(|item| item.min.y > items[0].min.y));
    assert!(items.iter().all(|item| bar.contains_rect(*item)));
}

#[test]
fn guide_space_evenly_leaves_equal_space_at_the_ends_and_between() {
    let divs = divs(&guide("justify-content", 5));
    let (bar, items) = (divs[1], &divs[2..7]);
    let between = items[1].min.x - items[0].max.x;
    assert!(between > 9.0, "free space to distribute");
    assert_close(items[0].min.x - (bar.min.x + BAR_INSET), between - 8.0);
    assert_close(bar.max.x - BAR_INSET - items[4].max.x, between - 8.0);
}

#[test]
fn guide_align_content_stretch_grows_the_lines_to_fill_the_bar() {
    let packed = divs(&guide("align-content", 0));
    let stretched = divs(&guide("align-content", 6));
    let (bar, items) = (stretched[1], &stretched[2..7]);
    assert!(items[4].min.y > items[0].max.y, "two lines");
    assert!(items[0].height() > packed[2].height() + 20.0);
    assert_close(items[0].min.y, bar.min.y + BAR_INSET);
    assert_close(items[4].max.y, bar.max.y - BAR_INSET);
    assert!(packed[6].max.y < packed[1].max.y - BAR_INSET - 20.0);
}

#[test]
fn guide_shrink_zero_keeps_its_width_while_the_others_shrink() {
    let all = divs(&guide("flex-shrink", 0));
    let first = divs(&guide("flex-shrink", 1));
    assert!(all[2..7].iter().all(|link| link.width() < 100.0));
    assert_eq!(first[2].width(), 100.0);
    assert!(first[3..7].iter().all(|link| link.width() < 100.0));
}

#[test]
fn guide_baseline_aligns_text_of_different_sizes() {
    let tops = |choice| {
        let divs = divs(&guide("align-items", choice));
        divs[2..7].iter().map(|link| link.min.y).collect::<Vec<_>>()
    };
    let start = tops(1);
    assert!(start.iter().all(|top| *top == start[0]));
    let baseline = tops(4);
    assert!(baseline.iter().any(|top| *top != baseline[0]));
    let texts = rects(&guide("align-items", 4), NodeKind::Text);
    let baselines: Vec<f32> = texts.iter().skip(1).map(|text| text.min.y).collect();
    assert_eq!(baselines.len(), 5);
    assert!(baselines.iter().any(|y| *y != baselines[0]));
}

#[test]
fn guide_align_self_moves_only_the_third_link() {
    let divs = divs(&guide("align-self", 2));
    let (bar, links) = (divs[1], &divs[2..7]);
    assert_close(links[2].max.y, bar.max.y - BAR_INSET);
    for index in [0, 1, 3, 4] {
        assert_close(links[index].center().y, bar.center().y);
    }
}

#[test]
fn guide_order_is_the_build_order() {
    let widths = |choice| {
        let divs = divs(&guide("order", choice));
        divs[2..7]
            .iter()
            .map(|link| link.width())
            .collect::<Vec<_>>()
    };
    let source = widths(0);
    assert!(source.iter().any(|width| *width != source[0]));
    let reordered = widths(1);
    for (position, index) in [2, 0, 3, 4, 1].into_iter().enumerate() {
        assert_close(reordered[position], source[index]);
    }
}

#[test]
fn every_guide_demo_builds_for_every_value() {
    let demos = flexbox_guide::State::default().demos;
    for (demo, entry) in demos.iter().enumerate() {
        for choice in 0..entry.choices.len().max(1) {
            let state = flexbox_guide::State {
                demo,
                choice,
                ..flexbox_guide::State::default()
            };
            let placed = place_app(AREA, |b| flexbox_guide::guide(b, &state));
            assert!(placed.len() > 3, "{} builds its demo", entry.title);
        }
    }
}

#[test]
fn flex_stretch_with_grow_fills_the_row_and_without_it_packs_at_the_start() {
    let divs = divs(&place(|b| flex::layout(b, &())));
    let row = |index: usize| {
        let first = 1 + index * 7;
        (divs[first + 2], divs[first + 3..first + 7].to_vec())
    };
    let (packed, items) = row(2);
    assert_close(items[0].min.x, packed.min.x + BAR_INSET);
    assert!(items[3].max.x < packed.max.x - 100.0);

    let (grown, items) = row(3);
    assert_close(items[0].min.x, grown.min.x + BAR_INSET);
    assert_close(items[3].max.x, grown.max.x - BAR_INSET);
}

#[test]
fn flex_wrap_wraps_and_grows_each_line_to_the_edge() {
    let words = |wrapping| {
        let state = flex_wrap::State {
            wrapping,
            ..flex_wrap::State::default()
        };
        divs(&place(|b| flex_wrap::demo(b, &state)))[3..21].to_vec()
    };
    let nowrap = words(flex_wrap::Wrapping::NoWrap);
    assert!(nowrap.iter().all(|word| word.min.y == nowrap[0].min.y));
    let wrapped = words(flex_wrap::Wrapping::Wrap);
    let last_of_first_line = wrapped
        .iter()
        .rposition(|word| word.min.y == wrapped[0].min.y)
        .expect("the first word is on the first line");
    assert!(last_of_first_line < wrapped.len() - 1, "more than one line");
    let first_edge = wrapped[last_of_first_line].max.x;
    assert_close(wrapped[wrapped.len() - 1].max.x, first_edge);
}

#[test]
fn flex_grid_spans_and_aligns_within_cells() {
    let divs = divs(&place(|b| flex_grid::demo(b, &flex_grid::State::default())));
    let (grid, span_cols, span_rows) = (divs[0], divs[1], divs[2]);
    assert_close(span_rows.height(), grid.height() - 2.0 * 8.0);
    assert_eq!(span_cols.min.y, span_rows.min.y);
    assert!(span_cols.max.x < span_rows.min.x);

    let (header_start, header_stretch) = (divs[4], divs[7]);
    let (row_header_stretch, start_start, stretch_stretch) = (divs[23], divs[9], divs[27]);
    assert!(start_start.width() < header_start.width());
    assert!(start_start.height() < row_header_stretch.height());
    assert_close(stretch_stretch.width(), header_stretch.width());
    assert_close(stretch_stretch.height(), row_header_stretch.height());
}

#[test]
fn grow_shares_the_free_space_by_factor() {
    let divs = divs(&place(|b| grow::demo(b, &grow::State::default())));
    let heights: Vec<f32> = divs[1..5].iter().map(|item| item.height()).collect();
    let unit = heights[1] - heights[0];
    assert!(unit > 1.0, "free space to share");
    assert_close(heights[2] - heights[0], 2.0 * unit);
    assert_close(heights[3] - heights[0], 3.0 * unit);

    let nested: Vec<f32> = divs[6..10].iter().map(|item| item.height()).collect();
    let unit = nested[1] - nested[0];
    assert!(unit > 1.0, "free space to share");
    assert_close(nested[3] - nested[0], 3.0 * unit);
}

#[test]
fn holy_grail_areas_fill_the_grid_around_the_content() {
    let divs = divs(&place(|b| holy_grail::layout(b, &())));
    assert_eq!(AREA.size(), vec2(800.0, 600.0));
    assert_rect(divs[1], [0.0, 0.0, 800.0, 100.0]);
    assert_rect(divs[2], [0.0, 100.0, 100.0, 500.0]);
    assert_rect(divs[3], [100.0, 100.0, 700.0, 500.0]);
    assert_rect(divs[4], [700.0, 100.0, 800.0, 500.0]);
    assert_rect(divs[5], [0.0, 500.0, 800.0, 600.0]);
}

#[test]
fn custom_background_boxes_wrap_into_rows_of_the_slider_size() {
    let boxes = |size| {
        let state = custom_background::State {
            size,
            ..Default::default()
        };
        let placed = fixed_state()
            .build(ctx(), |b| custom_background::demo(b, &state))
            .layout(AREA);
        divs(&placed)[2..8].to_vec()
    };
    let big = boxes(200.0);
    assert!(big.iter().all(|rect| rect.size() == vec2(200.0, 200.0)));
    assert!(big[2].min.y == big[0].min.y && big[3].min.y > big[2].min.y);
    let small = boxes(100.0);
    assert!(small.iter().all(|rect| rect.min.y == small[0].min.y));
}

// ------------------------------------------------------------ leaf padding

/// ```html
/// <div class="flex">
///   <span id="padded" class="px-2 py-1 border-2">Hello</span>
/// </div>
/// ```
#[test]
fn a_leaf_is_as_large_as_its_content_plus_padding_and_border() {
    let placed = place(|b| {
        b.div(tw().flex(), |b| {
            b.span("padded", tw().px(2.0).py(1.0).border(2.0), "Hello", none);
        });
    });
    let width = width_of(&BODY, "Hello");
    assert!(width > 0.0, "precondition: the text has a width");
    let [padded] = rects(&placed, NodeKind::Span)[..] else {
        panic!("one span")
    };
    assert_eq!(
        padded.size(),
        vec2(width + 2.0 * (8.0 + 2.0), 20.0 + 2.0 * (4.0 + 2.0))
    );
}

/// ```html
/// <div class="flex items-baseline">
///   <span class="py-3">Hello</span><span>Hello</span>
/// </div>
/// ```
#[test]
fn baseline_alignment_counts_a_leafs_padding() {
    let placed = place(|b| {
        b.div(tw().flex().items_baseline(), |b| {
            b.span("padded", tw().py(3.0), "Hello", none);
            b.span("plain", tw(), "Hello", none);
        });
    });
    let [padded, plain] = rects(&placed, NodeKind::Span)[..] else {
        panic!("two spans")
    };
    assert!(
        padded.height() > plain.height(),
        "precondition: padding differs"
    );
    assert_eq!(
        padded.min.y + 12.0,
        plain.min.y,
        "the text lines up: {padded:?} {plain:?}"
    );
}

/// A text leaf stretched across a column wraps at the width inside its
/// padding, and is as tall as the lines it wraps to.
///
/// ```html
/// <div class="flex flex-col w-[…]">
///   <span class="px-2">Six items, evenly distributed</span>
/// </div>
/// ```
#[test]
fn a_stretched_padded_text_leaf_is_as_tall_as_its_wrapped_lines() {
    let text = "Six items, evenly distributed";
    let natural = width_of(&BODY, text);
    let padding = 8.0;
    let column = natural + padding;
    assert!(
        column - 2.0 * padding < natural && column > natural * 0.6,
        "precondition: the text fits the column but not inside the padding"
    );
    let placed = place(|b| {
        b.div(tw().flex().flex_col().w_px(column), |b| {
            b.span("padded", tw().px(2.0), text, none);
        });
    });
    let [padded] = rects(&placed, NodeKind::Span)[..] else {
        panic!("one span")
    };
    assert_eq!(padded.width(), column);
    assert_eq!(padded.height(), 40.0, "two lines of 20: {padded:?}");
}
