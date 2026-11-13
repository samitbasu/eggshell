//! Who gets a press when boxes overlap: every painted box is a hit-test
//! target, and a press it wins goes to the nearest eggshell ancestor with a
//! handler; egui-native content gets presses only where no box covers it.

use eggshell_ui::prim::text_field::Field;
use eggshell_ui::scroll;
use eggshell_ui::{Builder, NodeUi, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Sense, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, hover_over, press_and_release, rect_of};

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
        },
        Color32::WHITE,
    )
}

#[derive(Default)]
struct App {
    clicks: u32,
    plot_clicks: u32,
    pass_through: bool,
    canvas_clicked: u32,
    row_clicks: Vec<u32>,
    text: String,
    sees_payload: bool,
    ended: u32,
}

type H = Harness<'static, Rig<App>>;

fn harness(app: App, view: support::View<App>) -> H {
    let mut h = support::harness(vec2(400.0, 400.0), base(), app, view);
    h.run_steps(3);
    h
}

/// ```html
/// <button class="w-[120px] h-[60px] bg-gray p-2" onclick="...">
///   <div id="badge" class="w-[30px] h-[20px] bg-red"></div>
/// </button>
/// ```
fn badge(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div_id(
        "button",
        tw().w_px(120.0)
            .h_px(60.0)
            .p(8.0)
            .bg(Color32::GRAY)
            .sense_click(),
        |b| {
            b.div_id("badge", tw().w_px(30.0).h_px(20.0).bg(Color32::RED), |_| {});
        },
    )
    .on_click(|app: &mut App| app.clicks += 1);
}

#[test]
fn a_press_on_a_painted_badge_clicks_the_button() {
    let mut h = harness(App::default(), badge);
    let at = rect_of(&h, "/\"button\"/\"badge\"").center();
    hover_over(&mut h, at);
    press_and_release(&mut h, at);
    assert_eq!(h.state().app.clicks, 1);
}

/// ```html
/// <div class="flex flex-col">
///   <div id="plot" class="w-[100px] h-[100px]"><!-- egui widget, senses clicks --></div>
///   <div id="label" class="absolute w-[50px] h-[20px] bg-black [pointer-events-none]">x</div>
/// </div>
/// ```
fn label_over_plot(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col(), |b| {
        b.custom("plot", tw().w_px(100.0).h_px(100.0))
            .ui(|app: &mut App, ui| {
                let response =
                    ui.interact(ui.max_rect(), ui.scope_id().with("plot"), Sense::click());
                if response.clicked() {
                    app.plot_clicks += 1;
                }
            });
        let mut label = tw().absolute().w_px(50.0).h_px(20.0).bg(Color32::BLACK);
        if app.pass_through {
            label = label.pointer_events_none();
        }
        b.div_id("label", label, |_| {});
    });
}

fn press_label(pass_through: bool) -> u32 {
    let mut h = harness(
        App {
            pass_through,
            ..App::default()
        },
        label_over_plot,
    );
    let label = rect_of(&h, "/\"label\"");
    let plot = rect_of(&h, "/\"plot\"");
    assert!(
        plot.contains_rect(label),
        "precondition: the label is over the plot"
    );
    let at = label.center();
    hover_over(&mut h, at);
    press_and_release(&mut h, at);
    h.state().app.plot_clicks
}

#[test]
fn a_painted_label_over_a_plot_occludes_it() {
    assert_eq!(press_label(false), 0);
}

#[test]
fn a_label_that_lets_the_pointer_through_leaves_the_plot_its_click() {
    assert_eq!(press_label(true), 1);
}

/// ```html
/// <div id="canvas" class="w-[200px] h-[100px] bg-gray flex gap-2" onclick="...">
///   <span id="a" class="w-[60px] h-[40px] bg-red">a</span>
/// </div>
/// ```
fn canvas(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div_id(
        "canvas",
        tw().w_px(200.0)
            .h_px(100.0)
            .bg(Color32::GRAY)
            .flex()
            .gap(8.0)
            .sense_click(),
        |b| {
            b.span("a", tw().w_px(60.0).h_px(40.0).bg(Color32::RED), "a");
        },
    )
    .draw_widgets(|app: &mut App, ui| {
        if ui.node_response().clicked() {
            app.canvas_clicked += 1;
        }
    });
}

#[test]
fn a_press_on_a_child_shows_in_the_canvas_node_response() {
    let mut h = harness(App::default(), canvas);
    let at = rect_of(&h, "/\"canvas\"/\"a\"").center();
    hover_over(&mut h, at);
    press_and_release(&mut h, at);
    assert_eq!(h.state().app.canvas_clicked, 1);
}

/// ```html
/// <div id="list" class="w-[200px] h-[100px] overflow-y-auto">
///   <div id="0..9" class="h-[30px] bg-… cursor-pointer"></div>
/// </div>
/// ```
fn rows(b: &mut Builder<'_, '_, App>, app: &App) {
    let drags = app.pass_through;
    b.on_drag_end::<u32>(|app: &mut App, _, _| app.ended += 1);
    b.scroll("list", tw().w_px(200.0).h_px(100.0))
        .children(|s| {
            s.div(tw().flex().flex_col(), |s| {
                for i in 0..10u32 {
                    let mut row = tw()
                        .h_px(30.0)
                        .w_full()
                        .bg(Color32::from_gray(40 + 10 * i as u8));
                    if !drags {
                        row = row.sense_click();
                    }
                    let node = s
                        .div_id(i, row, |_| {})
                        .on_click(move |app: &mut App| app.row_clicks.push(i));
                    if drags {
                        node.drag_source(i);
                    }
                }
            });
        });
}

/// egui's scroll areas drag to scroll only once a touch has been seen.
fn touch_screen(h: &mut H) {
    h.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Cancel,
        pos: egui::Pos2::ZERO,
        force: None,
    });
    h.step();
}

fn scrolled(h: &H) -> f32 {
    scroll::viewport(&h.ctx, support::id_of(h, "/\"list\""))
        .expect("laid out")
        .offset()
        .y
}

#[test]
fn a_click_on_a_row_in_a_scroll_area_clicks_it() {
    let mut h = harness(App::default(), rows);
    let at = rect_of(&h, "/\"list\"/1").center();
    hover_over(&mut h, at);
    press_and_release(&mut h, at);
    assert_eq!(h.state().app.row_clicks, [1]);
}

#[test]
fn a_drag_on_a_clickable_row_scrolls_and_does_not_click() {
    let mut h = harness(App::default(), rows);
    touch_screen(&mut h);
    let from = rect_of(&h, "/\"list\"/1").center();
    support::drag(&mut h, from, from + vec2(0.0, -40.0));
    assert!(scrolled(&h) > 10.0, "scrolled {}", scrolled(&h));
    assert!(h.state().app.row_clicks.is_empty());
}

#[test]
fn a_drag_on_a_dnd_source_in_a_scroll_area_is_the_sources() {
    let mut h = harness(
        App {
            pass_through: true,
            ..App::default()
        },
        rows,
    );
    touch_screen(&mut h);
    let from = rect_of(&h, "/\"list\"/1").center();
    support::drag(&mut h, from, from + vec2(0.0, -40.0));
    assert_eq!(scrolled(&h), 0.0, "the area did not scroll");
    assert_eq!(h.state().app.ended, 1, "the drag was the DnD's");
}

/// ```html
/// <div id="card" class="w-[200px] p-2 bg-gray" onclick="..."><input id="name"></div>
/// ```
fn card(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(
        "card",
        tw().w_px(200.0).p(8.0).bg(Color32::DARK_GRAY).sense_click(),
        |b| {
            b.text_field_with("name", Field::singleline(app.text.as_str()).named("Name"))
                .on_change(|app: &mut App, value| app.text = value);
        },
    )
    .on_click(|app: &mut App| app.clicks += 1);
}

#[test]
fn clicking_a_field_in_a_card_focuses_it_and_does_not_click_the_card() {
    let mut h = harness(App::default(), card);
    let at = rect_of(&h, "/\"card\"/\"name\"").center();
    hover_over(&mut h, at);
    press_and_release(&mut h, at);
    h.run_steps(2);
    assert_eq!(h.state().app.clicks, 0);
    assert_eq!(
        support::focused_node(&h).as_deref(),
        Some("/\"card\"/\"name\"")
    );
}

/// ```html
/// <div class="flex gap-4">
///   <div id="source" class="w-[40px] h-[40px] bg-red" draggable></div>
///   <div id="target" class="w-[100px] h-[100px]"><!-- egui widget reads the drag --></div>
/// </div>
/// ```
fn onto_custom(b: &mut Builder<'_, '_, App>, _: &App) {
    b.on_drag_end::<u32>(|app: &mut App, _, _| app.ended += 1);
    b.div(tw().flex().gap(16.0), |b| {
        b.div_id(
            "source",
            tw().w_px(40.0).h_px(40.0).bg(Color32::RED),
            |_| {},
        )
        .drag_source(1u32);
        b.custom("target", tw().w_px(100.0).h_px(100.0))
            .ui(|app: &mut App, ui| {
                let over = ui.node_response().contains_pointer();
                app.sees_payload |= over && egui::DragAndDrop::has_any_payload(ui.ctx());
            });
    });
}

#[test]
fn a_custom_node_sees_the_payload_of_an_eggshell_drag_over_it() {
    let mut h = harness(App::default(), onto_custom);
    let from = rect_of(&h, "/\"source\"").center();
    let to = rect_of(&h, "/\"target\"").center();
    hover_over(&mut h, from);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(6.0, 0.0));
    h.step();
    h.hover_at(from + vec2(12.0, 0.0));
    h.run_steps(2);
    h.hover_at(to);
    h.run_steps(3);
    assert!(
        eggshell_ui::drag_pointer(&h.ctx).is_some(),
        "precondition: the source is lifted"
    );
    assert!(h.state().app.sees_payload);
}

/// ```html
/// <div id="parent" class="w-[100px] p-4 bg-gray hover:bg-red">
///   <div id="child" class="w-[40px] h-[40px] bg-blue"></div>
/// </div>
/// ```
fn hover_parent(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div_id(
        "parent",
        tw().w_px(100.0)
            .p(16.0)
            .bg(Color32::GRAY)
            .hover(|p| p.bg(Color32::RED)),
        |b| {
            b.div_id(
                "child",
                tw().w_px(40.0).h_px(40.0).bg(Color32::BLUE),
                |_| {},
            );
        },
    );
}

#[test]
fn hovering_a_child_box_shows_the_parents_hover_variant() {
    let mut h = harness(App::default(), hover_parent);
    let at = rect_of(&h, "/\"parent\"/\"child\"").center();
    hover_over(&mut h, at);
    assert_eq!(support::fill_of(&h, "/\"parent\""), Some(Color32::RED));
}

/// ```html
/// <div class="flex flex-col">
///   <div id="box" class="w-[40px] h-[40px] [bg-gray]"></div>
///   <button id="tab" class="w-[40px] h-[20px]"></button>
/// </div>
/// ```
fn maybe_painted(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col(), |b| {
        let mut face = tw().w_px(40.0).h_px(40.0);
        if app.pass_through {
            face = face.bg(Color32::GRAY);
        }
        b.div_id("box", face, |_| {});
        b.button("tab", "Tab").on_click(|_: &mut App| {});
    });
}

fn widget_count(h: &H) -> usize {
    h.ctx
        .viewport(|v| v.this_pass.widgets.layers().map(|(_, w)| w.len()).sum())
}

fn accessible_nodes(h: &H) -> usize {
    use egui_kittest::kittest::NodeT as _;
    h.root().children_recursive().count()
}

#[test]
fn a_box_is_a_widget_but_neither_an_accessibility_node_nor_a_tab_stop() {
    let plain = harness(App::default(), maybe_painted);
    let painted = harness(
        App {
            pass_through: true,
            ..App::default()
        },
        maybe_painted,
    );
    assert_eq!(widget_count(&painted), widget_count(&plain) + 1);
    assert_eq!(accessible_nodes(&painted), accessible_nodes(&plain));
}

/// ```html
/// <div id="list" class="w-[200px] h-[100px] overflow-y-auto">
///   <div class="relative flex flex-col">
///     <div id="under" class="w-[100px] h-[100px]" draggable></div>
///     <div id="panel" class="absolute w-[60px] h-[60px] bg-gray"></div>
///     <div class="h-[300px]"></div>
///   </div>
/// </div>
/// ```
fn panel_in_scroll(b: &mut Builder<'_, '_, App>, app: &App) {
    let under = if app.pass_through {
        tw().w_px(100.0).h_px(100.0)
    } else {
        tw().w_px(100.0).h_px(100.0).sense_drag()
    };
    b.scroll("list", tw().w_px(200.0).h_px(100.0))
        .children(|s| {
            s.div(tw().flex().flex_col(), |s| {
                s.div_id("under", under, |_| {})
                    .on_response(|app: &mut App, response| app.sees_payload |= response.dragged());
                s.div_id(
                    "panel",
                    tw().absolute().w_px(60.0).h_px(60.0).bg(Color32::GRAY),
                    |_| {},
                );
                s.div(tw().h_px(300.0), |_| {});
            });
        });
}

#[test]
fn with_a_mouse_a_painted_panel_in_a_scroll_area_blocks_a_sibling_drag() {
    let mut h = harness(App::default(), panel_in_scroll);
    let panel = rect_of(&h, "/\"list\"/\"panel\"");
    let under = rect_of(&h, "/\"list\"/\"under\"");
    assert!(
        under.contains_rect(panel),
        "precondition: the panel covers it"
    );
    support::drag(&mut h, panel.center(), panel.center() + vec2(0.0, 20.0));
    assert!(!h.state().app.sees_payload);
}

#[test]
fn with_a_touch_screen_a_drag_on_painted_content_scrolls_the_area() {
    let mut h = harness(
        App {
            pass_through: true,
            ..App::default()
        },
        panel_in_scroll,
    );
    touch_screen(&mut h);
    let panel = rect_of(&h, "/\"list\"/\"panel\"");
    support::drag(&mut h, panel.center(), panel.center() + vec2(0.0, -40.0));
    assert!(scrolled(&h) > 10.0, "scrolled {}", scrolled(&h));
}

/// ```html
/// <button id="other" class="w-[60px] h-[20px]"></button>
/// <div id="card" class="w-[100px] p-4 bg-gray" onclick="..."> <!-- focusable -->
///   <div id="badge" class="w-[40px] h-[40px] bg-red"></div>
/// </div>
/// ```
fn focusable_card(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().gap(8.0), |b| {
        b.button("other", "Other").on_click(|_: &mut App| {});
        b.div_id(
            "card",
            tw().w_px(100.0)
                .p(16.0)
                .bg(Color32::DARK_GRAY)
                .sense_click(),
            |b| {
                b.div_id("badge", tw().w_px(40.0).h_px(40.0).bg(Color32::RED), |_| {});
            },
        )
        .on_click(|app: &mut App| app.clicks += 1);
    });
}

fn focus(h: &mut H, path: &str) {
    let id = support::id_of(h, path);
    h.ctx.memory_mut(|memory| memory.request_focus(id));
    h.run_steps(2);
    assert_eq!(
        support::focused_path(h).as_deref(),
        Some(path),
        "precondition"
    );
}

#[test]
fn a_press_on_a_painted_child_keeps_the_focus_of_its_focused_card() {
    let mut h = harness(App::default(), focusable_card);
    focus(&mut h, "/\"card\"");
    let at = rect_of(&h, "/\"card\"/\"badge\"").center();
    hover_over(&mut h, at);
    press_and_release(&mut h, at);
    h.run_steps(2);
    assert_eq!(h.state().app.clicks, 1);
    assert_eq!(support::focused_path(&h).as_deref(), Some("/\"card\""));
}

#[test]
fn a_press_on_a_painted_child_leaves_the_focus_elsewhere_as_a_press_on_the_card_does() {
    let press_at = |badge: bool| {
        let mut h = harness(App::default(), focusable_card);
        focus(&mut h, "/\"other\"");
        let card = rect_of(&h, "/\"card\"");
        let at = if badge {
            rect_of(&h, "/\"card\"/\"badge\"").center()
        } else {
            card.left_top() + vec2(4.0, 4.0)
        };
        hover_over(&mut h, at);
        press_and_release(&mut h, at);
        h.run_steps(2);
        support::focused_path(&h)
    };
    assert_eq!(press_at(true), press_at(false));
}
