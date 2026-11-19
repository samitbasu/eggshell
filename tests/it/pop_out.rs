//! Pop-out windows through the real path (#103): a keyed subtree inline, and
//! in a window of its own. kittest has one viewport, so its windows are
//! embedded `egui::Window`s; `native` drives the native path through an
//! immediate-viewport renderer that runs each window's pass as eframe does.

use eggshell_ui::overlay::{Anchor, Placement};
use eggshell_ui::window::{MinSize, Popped, Window, WindowFrame};
use eggshell_ui::{Builder, Chord, Command, Commands, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, Modifiers, Pos2, Vec2, vec2};
use egui_kittest::kittest::Queryable as _;

use eggshell_ui::testing::{Driver, Target};

use crate::support;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/pop_out.rs"]
mod example;

const SIZE: Vec2 = vec2(600.0, 400.0);
const PANE: &str = "/\"pane\"";
const PLACEHOLDER: &str = "/Placeholder(\"pane\")";
const LIST: &str = "/\"pane\"/\"list\"";
const ACT: &str = "/\"pane\"/\"act\"";
const OPENS_AT: WindowFrame = WindowFrame {
    position: Pos2::new(200.0, 80.0),
    size: Vec2::new(220.0, 160.0),
};

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

struct App {
    popped: Popped,
    frame: Option<WindowFrame>,
    min_size: MinSize,
    closes: u32,
    frames: Vec<WindowFrame>,
    acts: u32,
    helps: u32,
    menu: bool,
    items: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            popped: Popped::In,
            frame: Some(OPENS_AT),
            min_size: MinSize::Points(vec2(80.0, 60.0)),
            closes: 0,
            frames: Vec::new(),
            acts: 0,
            helps: 0,
            menu: false,
            items: 0,
        }
    }
}

/// ```html
/// <div class="flex gap-2 p-2">
///   <div class="w-[100px] h-[100px]">left</div>
///   <div id="pane" class="flex flex-col w-[200px] h-[150px]">  <!-- or a window -->
///     <button id="act" class="w-[150px] h-[20px] shrink-0">Act</button>
///     <div id="list" class="overflow-y-auto h-[100px] shrink-0"> 20 rows of 30px </div>
///   </div>
/// </div>
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    let help = Commands::new().add(
        "help",
        Command::new("Help", |app: &mut App| app.helps += 1)
            .shortcut(const { Chord::new(Modifiers::NONE, Key::H).expect("not reserved") }),
    );
    b.commands(&help, app);
    b.div(tw().flex().gap(2.0).p(2.0), |b| {
        b.div(tw().w_px(100.0).h_px(100.0), |_| {});
        b.pop_out("pane")
            .classes(tw().flex().flex_col().w_px(200.0).h_px(150.0))
            .popped(app.popped)
            .window(Window::new("Pane").frame(app.frame).min_size(app.min_size))
            .on_close(|app: &mut App| {
                app.closes += 1;
                app.popped = Popped::In;
            })
            .on_frame(|app: &mut App, frame| app.frames.push(frame))
            .placeholder(|s| s.text(tw(), "Shown in its own window"))
            .children(|s| {
                s.button("act", "Act")
                    .classes(tw().w_px(150.0).h_px(20.0).shrink_0())
                    .on_click(|app: &mut App| app.acts += 1);
                if app.menu {
                    let act = s.id_of("act");
                    s.overlay("menu")
                        .anchor(Anchor::Node(act))
                        .placement(Placement::below())
                        .children(|s| {
                            s.button("item", "Item")
                                .on_click(|app: &mut App| app.items += 1);
                        });
                }
                s.scroll("list", tw().h_px(100.0).w_full().shrink_0())
                    .children(|s| {
                        for i in 0..20u32 {
                            s.div_id(i, tw().h_px(30.0).w_full(), |_| {});
                        }
                    });
            });
    });
}

type H = Driver<App>;

fn harness(app: App) -> H {
    let mut h = support::driver(SIZE, base(), app, view).build();
    support::set_companion(&h, support::Companion::Alone);
    h.frames(4);
    h
}

fn has(h: &H, path: &str) -> bool {
    support::nodes(h)
        .iter()
        .any(|(candidate, _)| candidate == path)
}

fn offset(h: &H) -> Vec2 {
    h.shell()
        .viewport(support::id_of(h, LIST))
        .expect("the list has a viewport")
        .offset()
}

#[test]
fn inline_the_subtree_is_a_container_at_its_place() {
    let h = harness(App::default());
    assert!(h.ctx().embed_viewports(), "precondition: kittest embeds");
    assert_eq!(h.rect(Target::suffix(PANE)).size(), vec2(200.0, 150.0));
    assert!(!has(&h, PLACEHOLDER));
}

#[test]
fn popped_out_the_subtree_fills_a_window_at_the_frame_and_the_placeholder_holds_its_place() {
    let mut h = harness(App::default());
    h.app_mut().popped = Popped::Out;
    h.frames(4);
    assert_eq!(h.rect(Target::suffix(PANE)).size(), OPENS_AT.size);
    assert!(has(&h, PLACEHOLDER), "{:?}", support::nodes(&h));
    assert!(has(&h, LIST), "the subtree is laid out in the window");
}

#[test]
fn a_scroll_offset_survives_popping_out_and_back_in() {
    let mut h = harness(App::default());
    let list = support::id_of(&h, LIST);
    h.shell().scroll_by(list, vec2(0.0, 90.0));
    h.frames(4);
    let before = offset(&h);
    assert!(before.y > 0.0, "precondition: the list scrolled");
    h.app_mut().popped = Popped::Out;
    h.frames(4);
    assert_eq!(offset(&h), before, "in the window");
    h.app_mut().popped = Popped::In;
    h.frames(4);
    assert_eq!(offset(&h), before, "back inline");
}

#[test]
fn the_close_button_runs_on_close_once_and_the_window_goes() {
    let mut h = harness(App::default());
    h.app_mut().popped = Popped::Out;
    h.frames(4);
    h.harness().get_by_label("Close window").click();
    h.frames(4);
    assert_eq!(h.app().closes, 1);
    assert_eq!(h.app().popped, Popped::In);
    assert_eq!(
        h.rect(Target::suffix(PANE)).size(),
        vec2(200.0, 150.0),
        "back inline"
    );
    assert!(h.harness().query_by_label("Close window").is_none());
}

#[test]
fn on_frame_runs_when_the_window_moves_and_not_while_it_stays() {
    let mut h = harness(App::default());
    h.app_mut().popped = Popped::Out;
    h.frames(6);
    h.app_mut().frames.clear();
    h.frames(6);
    assert!(h.app().frames.is_empty(), "{:?}", h.app().frames);
    let window = h.harness().get_by_label("Pane").rect();
    let content = h.rect(Target::suffix(PANE));
    assert!(
        content.min.y - window.min.y > 16.0,
        "precondition: a title bar above the content"
    );
    let title = window.min + vec2(window.width() / 2.0, 8.0);
    h.drag(title, title + vec2(40.0, 30.0));
    h.frames(4);
    let frames = &h.app().frames;
    let last = frames.last().expect("the move was reported");
    assert!(last.position.x > OPENS_AT.position.x + 30.0, "{frames:?}");
    let moved = frames.len();
    h.frames(6);
    assert_eq!(h.app().frames.len(), moved, "no report while it stays");
}

#[test]
fn a_window_drawn_where_it_was_not_laid_out_is_laid_out_there_one_frame_later() {
    let mut h = harness(App::default());
    h.app_mut().popped = Popped::Out;
    h.frames(1);
    let laid = h.rect(Target::suffix(PANE));
    h.frames(1);
    let next = h.rect(Target::suffix(PANE));
    h.frames(4);
    let settled = h.rect(Target::suffix(PANE));
    assert_ne!(
        laid, settled,
        "precondition: the first frame was laid out off"
    );
    assert_eq!(next, settled, "settled one frame later");
}

#[test]
fn min_size_content_is_the_subtrees_min_content_size() {
    let mut h = harness(App {
        min_size: MinSize::Content,
        frame: Some(WindowFrame {
            position: OPENS_AT.position,
            size: vec2(10.0, 10.0),
        }),
        ..App::default()
    });
    h.app_mut().popped = Popped::Out;
    h.frames(4);
    let pane = h.rect(Target::suffix(PANE));
    assert_eq!(
        pane.size(),
        vec2(150.0, 120.0),
        "the header's width and the header and list's height"
    );
}

const INLINE: Vec2 = vec2(200.0, 150.0);

#[test]
fn without_a_frame_the_window_opens_at_the_subtrees_inline_size() {
    let mut h = harness(App {
        frame: None,
        ..App::default()
    });
    assert_eq!(
        h.rect(Target::suffix(PANE)).size(),
        INLINE,
        "precondition: inline"
    );
    h.app_mut().popped = Popped::Out;
    h.frames(4);
    assert_eq!(h.rect(Target::suffix(PANE)).size(), INLINE);
}

#[test]
fn a_given_frame_wins_over_the_inline_size() {
    let mut h = harness(App::default());
    assert_ne!(OPENS_AT.size, INLINE, "precondition: they differ");
    h.app_mut().popped = Popped::Out;
    h.frames(4);
    assert_eq!(h.rect(Target::suffix(PANE)).size(), OPENS_AT.size);
}

#[test]
fn popped_out_from_the_start_the_window_opens_at_the_max_content_size() {
    let h = harness(App {
        popped: Popped::Out,
        frame: None,
        ..App::default()
    });
    assert_eq!(
        h.rect(Target::suffix(PANE)).size(),
        vec2(150.0, 120.0),
        "the header's width and the header and list's height"
    );
}

/// The native path: kittest's context with `embed_viewports` off and an
/// immediate-viewport renderer that runs each window's pass as eframe does,
/// with the window's size, position, close request and events scripted.
mod native {
    use std::cell::RefCell;

    use egui::{
        Event, PointerButton, RawInput, Rect, ViewportBuilder, ViewportCommand, ViewportEvent,
        ViewportId, ViewportInfo, pos2,
    };

    use super::*;

    /// What the window is like, and what it was told.
    struct Child {
        inner: Vec2,
        position: Pos2,
        close: bool,
        events: Vec<Event>,
        shown: Vec<ViewportId>,
        builders: Vec<ViewportBuilder>,
        commands: Vec<ViewportCommand>,
        /// The widget with focus in the window at the end of its pass.
        focused: Option<egui::Id>,
    }

    /// Room for the title bar the native window has around its content.
    const TITLE: f32 = 30.0;

    thread_local! {
        static CHILD: RefCell<Child> = const { RefCell::new(Child {
            inner: OPENS_AT.size,
            position: OPENS_AT.position,
            close: false,
            events: Vec::new(),
            shown: Vec::new(),
            builders: Vec::new(),
            commands: Vec::new(),
            focused: None,
        }) };
    }

    fn install() {
        egui::Context::set_immediate_viewport_renderer(|ctx, viewport| {
            let id = viewport.ids.this;
            let input = CHILD.with_borrow_mut(|child| {
                child.shown.push(id);
                child.builders.push(viewport.builder.clone());
                let outer = Rect::from_min_size(child.position, child.inner + vec2(0.0, TITLE));
                let info = ViewportInfo {
                    inner_rect: Some(Rect::from_min_size(
                        child.position + vec2(0.0, TITLE),
                        child.inner,
                    )),
                    outer_rect: Some(outer),
                    events: if std::mem::take(&mut child.close) {
                        vec![ViewportEvent::Close]
                    } else {
                        Vec::new()
                    },
                    ..ViewportInfo::default()
                };
                RawInput {
                    viewport_id: id,
                    viewports: std::iter::once((id, info)).collect(),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, child.inner)),
                    events: std::mem::take(&mut child.events),
                    ..RawInput::default()
                }
            });
            let mut show = viewport.viewport_ui_cb;
            let mut output = ctx.run_ui(input, |ui| {
                show(ui);
                let focused = ui.ctx().memory(|memory| memory.focused());
                CHILD.with_borrow_mut(|child| child.focused = focused);
            });
            // Nothing paints the window, so nothing takes its textures.
            output.textures_delta.clear();
            CHILD.with_borrow_mut(|child| {
                if let Some(out) = output.viewport_output.get(&id) {
                    child.commands.extend(out.commands.iter().cloned());
                }
            });
        });
    }

    fn harness_native() -> H {
        install();
        let mut h = super::harness(App::default());
        h.ctx().set_embed_viewports(false);
        h.app_mut().popped = Popped::Out;
        h.frames(4);
        h
    }

    fn shown_this_step(h: &mut H) -> Vec<ViewportId> {
        CHILD.with_borrow_mut(|child| child.shown.clear());
        h.frames(1);
        CHILD.with_borrow(|child| child.shown.clone())
    }

    fn pane_viewport(h: &H) -> ViewportId {
        ViewportId(support::id_of(h, PANE))
    }

    #[test]
    fn the_subtree_is_laid_out_in_the_window_at_its_size_in_its_coordinates() {
        let mut h = harness_native();
        assert!(!h.ctx().embed_viewports(), "precondition: native windows");
        assert_eq!(shown_this_step(&mut h), vec![pane_viewport(&h)]);
        assert_eq!(
            h.rect(Target::suffix(PANE)),
            Rect::from_min_size(Pos2::ZERO, OPENS_AT.size)
        );
        assert!(has(&h, PLACEHOLDER));
    }

    fn repaint_asked(h: &H) -> bool {
        h.harness().output().viewport_output[&ViewportId::ROOT].repaint_delay
            == std::time::Duration::ZERO
    }

    #[test]
    fn a_resize_is_laid_out_one_frame_later_and_settles() {
        let mut h = harness_native();
        h.frames(4);
        assert!(!repaint_asked(&h), "precondition: idle");
        CHILD.with_borrow_mut(|child| child.inner = vec2(300.0, 220.0));
        h.frames(1);
        assert_eq!(
            h.rect(Target::suffix(PANE)).size(),
            OPENS_AT.size,
            "laid out at the last size"
        );
        assert!(repaint_asked(&h), "the mismatch asks for the next frame");
        h.frames(1);
        assert_eq!(
            h.rect(Target::suffix(PANE)).size(),
            vec2(300.0, 220.0),
            "one frame later"
        );
        let frames = h.app().frames.len();
        h.frames(4);
        assert_eq!(h.rect(Target::suffix(PANE)).size(), vec2(300.0, 220.0));
        assert_eq!(
            h.app().frames.len(),
            frames,
            "settled: nothing more to report"
        );
    }

    #[test]
    fn the_window_is_shown_with_the_builder_it_opened_with() {
        let mut h = harness_native();
        CHILD.with_borrow_mut(|child| {
            child.inner = vec2(300.0, 220.0);
            child.position = pos2(260.0, 120.0);
            child.commands.clear();
        });
        h.frames(4);
        let builders = CHILD.with_borrow(|child| child.builders.clone());
        assert!(builders.len() > 4, "precondition: several frames shown");
        assert!(
            builders.iter().all(|builder| *builder == builders[0]),
            "the builder never changes, so egui sends no command: {builders:?}"
        );
        let opened = &builders[0];
        assert_eq!(opened.inner_size, Some(OPENS_AT.size));
        assert_eq!(opened.position, Some(OPENS_AT.position));
    }

    #[test]
    fn without_a_frame_the_window_opens_at_the_subtrees_inline_size() {
        install();
        let mut h = super::harness(App {
            frame: None,
            ..App::default()
        });
        h.ctx().set_embed_viewports(false);
        h.frames(2);
        assert_eq!(
            h.rect(Target::suffix(PANE)).size(),
            INLINE,
            "precondition: inline"
        );
        CHILD.with_borrow_mut(|child| child.inner = INLINE);
        h.app_mut().popped = Popped::Out;
        h.frames(4);
        let opened = CHILD.with_borrow(|child| child.builders.first().cloned());
        assert_eq!(opened.expect("shown").inner_size, Some(INLINE));
        assert_eq!(
            h.rect(Target::suffix(PANE)),
            Rect::from_min_size(Pos2::ZERO, INLINE)
        );
    }

    #[test]
    fn a_move_and_a_resize_are_reported_once_each() {
        let mut h = harness_native();
        h.app_mut().frames.clear();
        h.frames(4);
        assert!(h.app().frames.is_empty(), "nothing while it stays");
        CHILD.with_borrow_mut(|child| child.position = pos2(260.0, 120.0));
        h.frames(4);
        assert_eq!(
            h.app().frames,
            vec![WindowFrame {
                position: pos2(260.0, 120.0),
                size: OPENS_AT.size,
            }]
        );
        CHILD.with_borrow_mut(|child| child.inner = vec2(300.0, 220.0));
        h.frames(4);
        assert_eq!(h.app().frames.len(), 2, "{:?}", h.app().frames);
        assert_eq!(h.app().frames[1].size, vec2(300.0, 220.0));
    }

    #[test]
    fn a_close_request_runs_on_close_once_and_the_window_is_gone_the_next_frame() {
        let mut h = harness_native();
        CHILD.with_borrow_mut(|child| child.close = true);
        h.frames(1);
        assert_eq!(h.app().closes, 1);
        assert!(
            shown_this_step(&mut h).is_empty(),
            "not shown once popped in"
        );
        h.frames(4);
        assert_eq!(h.app().closes, 1);
        assert_eq!(
            h.rect(Target::suffix(PANE)).size(),
            vec2(200.0, 150.0),
            "back inline"
        );
    }

    #[test]
    fn a_click_in_the_window_reaches_its_subtree() {
        let mut h = harness_native();
        let act = h.rect(Target::suffix(ACT)).center();
        for pressed in [true, false] {
            CHILD.with_borrow_mut(|child| {
                child.events.push(Event::PointerMoved(act));
                child.events.push(Event::PointerButton {
                    pos: act,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            });
            h.frames(1);
        }
        h.frames(2);
        assert_eq!(h.app().acts, 1);
    }

    fn press_in_window(h: &mut H, key: egui::Key) {
        CHILD.with_borrow_mut(|child| {
            for pressed in [true, false] {
                child.events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
        });
        h.frames(3);
    }

    #[test]
    fn focus_follows_the_subtree_into_its_window() {
        install();
        let mut h = super::harness(App::default());
        h.ctx().set_embed_viewports(false);
        let act = support::id_of(&h, ACT);
        for _ in 0..4 {
            if h.ctx().memory(|memory| memory.focused()) == Some(act) {
                break;
            }
            h.press(egui::Key::Tab).frames(1);
        }
        assert_eq!(
            h.ctx().memory(|memory| memory.focused()),
            Some(act),
            "precondition: the button has focus inline"
        );
        h.app_mut().popped = Popped::Out;
        h.frames(3);
        assert_eq!(CHILD.with_borrow(|child| child.focused), Some(act));
    }

    #[test]
    fn focus_follows_the_subtree_back_inline() {
        let mut h = harness_native();
        let act = support::id_of(&h, ACT);
        press_in_window(&mut h, egui::Key::Tab);
        assert_eq!(
            CHILD.with_borrow(|child| child.focused),
            Some(act),
            "precondition: the button has focus in the window"
        );
        assert_ne!(h.ctx().memory(|memory| memory.focused()), Some(act));
        h.app_mut().popped = Popped::In;
        h.frames(3);
        assert_eq!(h.ctx().memory(|memory| memory.focused()), Some(act));
    }

    #[test]
    fn tab_in_the_window_stays_in_the_window() {
        let mut h = harness_native();
        let stops = [support::id_of(&h, ACT), support::id_of(&h, LIST)];
        let mut reached = Vec::new();
        for _ in 0..6 {
            press_in_window(&mut h, egui::Key::Tab);
            let focused = CHILD.with_borrow(|child| child.focused);
            assert!(
                focused.is_some_and(|focused| stops.contains(&focused)),
                "Tab left the window's subtree: {focused:?}"
            );
            reached.extend(focused);
            assert_eq!(
                h.ctx().memory(|memory| memory.focused()),
                None,
                "the page is untouched"
            );
        }
        assert!(
            stops.iter().all(|stop| reached.contains(stop)),
            "precondition: Tab went round the window's stops"
        );
    }

    #[test]
    fn a_shortcut_of_the_page_fires_from_the_window() {
        let mut h = harness_native();
        press_in_window(&mut h, Key::H);
        assert_eq!(h.app().helps, 1);
    }

    #[test]
    fn an_overlay_declared_in_the_window_opens_in_the_window() {
        let mut h = harness_native();
        h.app_mut().menu = true;
        h.frames(4);
        let item = h.rect(Target::suffix("/\"pane\"/\"menu\"/\"item\""));
        let window = Rect::from_min_size(Pos2::ZERO, OPENS_AT.size);
        assert!(
            window.contains_rect(item),
            "placed in the window's own coordinates: {item:?}"
        );
        for pressed in [true, false] {
            CHILD.with_borrow_mut(|child| {
                child.events.push(Event::PointerMoved(item.center()));
                child.events.push(Event::PointerButton {
                    pos: item.center(),
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            });
            h.frames(1);
        }
        h.frames(2);
        assert_eq!(h.app().items, 1);
    }
}

/// The `pop_out` example through the real path: a card's button pops it out
/// into a window and back in.
mod example_cards {
    use super::*;

    use super::example;

    #[test]
    fn a_cards_button_pops_it_out_and_back_in() {
        let tokens = eggshell_ui_shadcn::Tokens::for_theme(egui::Theme::Light);
        let mut h = support::driver(
            vec2(800.0, 700.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            example::State::default(),
            example::layout,
        )
        .build();
        support::set_companion(&h, support::Companion::Alone);
        h.frames(4);
        h.harness()
            .get_all_by_label("Pop out")
            .next()
            .expect("a card")
            .click();
        h.frames(4);
        assert_eq!(h.app().popped[0], Popped::Out);
        assert!(
            h.harness()
                .query_by_label("Revenue is in its own window")
                .is_some()
        );
        h.harness().get_by_label("Pop in").click();
        h.frames(4);
        assert_eq!(h.app().popped[0], Popped::In);
        assert!(
            h.harness()
                .query_by_label("Revenue is in its own window")
                .is_none()
        );
    }

    #[test]
    fn a_card_with_a_scroll_body_opens_at_its_inline_size() {
        const CARD: &str = "/Grid/Card(0)";
        let tokens = eggshell_ui_shadcn::Tokens::for_theme(egui::Theme::Light);
        let mut h = support::driver(
            vec2(800.0, 700.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            example::State::default(),
            example::layout,
        )
        .build();
        support::set_companion(&h, support::Companion::Alone);
        h.frames(4);
        let inline = h.rect(Target::suffix(CARD)).size();
        assert_eq!(inline.y, 220.0, "precondition: the card's inline height");
        h.app_mut().popped[0] = Popped::Out;
        h.frames(4);
        assert!(
            h.harness()
                .query_by_label("Revenue is in its own window")
                .is_some()
        );
        assert_eq!(h.rect(Target::suffix(CARD)).size(), inline);
    }
}
