//! `bullseye::testing`: layout queries without a window, a GPU or a draw.

use bullseye::testing::Headless;
use bullseye::{Builder, Placed, TextSpec, TextStyle, tw};
use bullseye_shadcn::Shadcn;
use egui::{Color32, FontFamily, TextStyle as EguiTextStyle, Theme, vec2};

use crate::support::{self, rect_of};

#[derive(Hash, Debug, Clone, Copy)]
enum Region {
    Viewport,
}

#[derive(Hash, Debug, Clone, Copy)]
enum Part {
    Panel,
    Card,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    Mono,
    Serif,
}

const SIZE: egui::Vec2 = vec2(600.0, 400.0);

fn body() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

fn headless() -> Headless {
    Headless::new(SIZE).base_text(body())
}

/// ```html
/// <div class="flex flex-col gap-2 p-4">          <!-- Viewport / Panel -->
///   <div class="w-[200px] p-2">Mono</div>        <!-- Mono -->
///   <div class="w-[200px] p-2">Serif</div>       <!-- Serif -->
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.scope(Region::Viewport, |b| {
        b.div_id(Part::Panel, tw().flex().flex_col().gap(8.0).p(16.0), |b| {
            for pick in [Pick::Mono, Pick::Serif] {
                b.div_id(pick, tw().w_px(200.0).p(8.0), |b| {
                    b.span("name", tw(), format!("{pick:?}"));
                    b.text(tw(), "tail");
                });
            }
        });
    });
}

fn placed() -> Placed<()> {
    headless().layout(&(), view)
}

#[test]
fn headless_rects_match_the_support_harness() {
    let inset = vec2(16.0, 16.0);
    let placed = Headless::new(SIZE - inset)
        .base_text(body())
        .layout(&(), view);
    let mut harness = support::harness(SIZE, body(), (), view);
    harness.run_steps(2);
    let panel = placed.at(Region::Viewport).at(Part::Panel);
    let mono = panel.at(Pick::Mono);
    assert_eq!(
        mono.rect().width(),
        200.0,
        "precondition: the card is fixed width"
    );
    let shift = rect_of(&harness, panel.path()).min - panel.rect().min;
    assert!(
        shift.length() > 0.0,
        "precondition: the harness insets its page"
    );
    for query in [
        panel.clone(),
        mono.clone(),
        mono.at("name"),
        panel.at(Pick::Serif),
    ] {
        assert_eq!(
            query.rect().translate(shift),
            rect_of(&harness, query.path()),
            "headless and harness disagree on {}",
            query.path()
        );
    }
}

#[test]
fn at_hashes_keys_as_the_builder_does() {
    let ids = std::cell::RefCell::new(Vec::new());
    let placed = headless().layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
        b.scope(Region::Viewport, |b| {
            b.div_id(Part::Card, tw().w_px(50.0), |b| {
                ids.borrow_mut().push(b.id_of("sample"));
                b.span("sample", tw(), "Aa");
            });
            ids.borrow_mut().push(b.id_of(Part::Card));
        });
    });
    let ids = ids.into_inner();
    let sample = placed.at(Region::Viewport).at(Part::Card).at("sample");
    assert_eq!(sample.id(), Some(ids[0]));
    assert_eq!(placed.rect(ids[0]), Some(sample.rect()));
    assert_eq!(
        placed.at(Region::Viewport).at(Part::Card).rect().width(),
        50.0
    );
}

#[test]
#[should_panic(
    expected = r#"no node /Panel/Card; below /Panel there are: /Panel/Mono, /Panel/Serif"#
)]
fn at_on_a_missing_node_names_the_path_and_the_siblings() {
    let placed = headless().layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
        b.div_id(Part::Panel, tw(), |b| {
            b.div_id(Pick::Mono, tw(), |_| {});
            b.div_id(Pick::Serif, tw(), |_| {});
        });
    });
    placed.at(Part::Panel).at(Part::Card);
}

#[test]
fn try_at_is_none_for_a_missing_node() {
    let placed = placed();
    let panel = placed.at(Region::Viewport).at(Part::Panel);
    assert!(panel.try_at(Part::Card).is_none());
    assert!(panel.at(Pick::Mono).try_at("name").is_some());
    assert!(placed.try_at(Part::Card).is_none());
}

#[test]
fn text_finds_the_unique_node_with_that_text() {
    let placed = placed();
    let mono = placed.at(Region::Viewport).at(Part::Panel).at(Pick::Mono);
    assert_eq!(mono.text("Mono").rect(), mono.at("name").rect());
    let tail = mono.text("tail");
    assert!(tail.id().is_none(), "precondition: the tail is unkeyed");
    assert!(mono.rect().contains_rect(tail.rect()));
}

#[test]
#[should_panic(
    expected = r#"no text "Absent" below /Viewport/Panel/Mono; text below it: "Mono" at /Viewport/Panel/Mono/"name", "tail" at"#
)]
fn text_panics_when_nothing_matches() {
    let placed = headless().layout(&(), view);
    placed
        .at(Region::Viewport)
        .at(Part::Panel)
        .at(Pick::Mono)
        .text("Absent");
}

#[test]
#[should_panic(expected = r#"2 nodes show "tail" below /Viewport/Panel"#)]
fn text_panics_when_two_nodes_match() {
    let placed = placed();
    placed.at(Region::Viewport).at(Part::Panel).text("tail");
}

#[test]
fn identical_layouts_have_no_changes() {
    assert!(placed().diff(&placed()).is_empty());
}

fn card_of(width: f32) -> Placed<()> {
    headless().layout(&(), move |b: &mut Builder<'_, '_, ()>, _: &()| {
        b.div(tw().flex().flex_col(), |b| {
            b.div_id("a", tw().w_px(100.0).h_px(10.0), |_| {});
            b.div_id("b", tw().w_px(width).h_px(10.0), |_| {});
            b.div_id("c", tw().w_px(100.0).h_px(10.0), |_| {});
        });
    })
}

#[test]
fn diff_reports_the_changed_node_and_nothing_else_unrelated() {
    let (before, after) = (card_of(100.0), card_of(150.0));
    let changes = before.diff(&after);
    let paths: Vec<&str> = changes.iter().map(|change| change.path()).collect();
    assert_eq!(paths, ["/\"b\""], "{changes:#?}");
    assert_eq!(changes[0].before().map(|r| r.width()), Some(100.0));
    assert_eq!(changes[0].after().map(|r| r.width()), Some(150.0));
}

#[test]
fn diff_reports_the_siblings_a_change_pushes() {
    let make = |height: f32| {
        headless().layout(&(), move |b: &mut Builder<'_, '_, ()>, _: &()| {
            b.div_id("a", tw().h_px(height).w_px(10.0), |_| {});
            b.div_id("b", tw().h_px(10.0).w_px(10.0), |_| {});
        })
    };
    let changes = make(10.0).diff(&make(30.0));
    let paths: Vec<&str> = changes.iter().map(|change| change.path()).collect();
    assert_eq!(paths, ["/\"a\"", "/\"b\""], "{changes:#?}");
}

#[test]
fn diff_reports_a_node_present_in_one_layout() {
    let with = |extra: bool| {
        headless().layout(&(), move |b: &mut Builder<'_, '_, ()>, _: &()| {
            b.div_id("a", tw().w_px(10.0).h_px(10.0), |_| {});
            if extra {
                b.div_id("extra", tw().w_px(10.0).h_px(10.0), |_| {});
            }
        })
    };
    let (without, with) = (with(false), with(true));
    let added = without.diff(&with);
    assert_eq!(added.len(), 1, "{added:#?}");
    assert_eq!(added[0].path(), "/\"extra\"");
    assert!(added[0].before().is_none() && added[0].after().is_some());
    let removed = with.diff(&without);
    assert!(removed[0].before().is_some() && removed[0].after().is_none());
}

fn text_width(mut headless: Headless) -> f32 {
    headless
        .layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
            b.div(tw().flex(), |b| {
                b.span("t", tw(), "Some words to measure");
            });
        })
        .at("t")
        .rect()
        .width()
}

#[test]
fn prepare_changes_the_fonts_layout_measures_with() {
    let plain = text_width(Headless::new(SIZE));
    let larger = text_width(Headless::new(SIZE).prepare(|ctx| {
        ctx.all_styles_mut(|style| {
            style
                .text_styles
                .get_mut(&EguiTextStyle::Body)
                .expect("body style")
                .size = 40.0;
        });
    }));
    assert!(larger > plain * 1.5, "{larger} vs {plain}");
}

fn themed_width(theme: Theme) -> f32 {
    let mut headless = Headless::new(SIZE)
        .base_text(body())
        .prepare(Shadcn::apply_both)
        .theme(theme);
    headless
        .layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
            let width = match b.ctx().theme() {
                Theme::Light => 30.0,
                Theme::Dark => 60.0,
            };
            assert!(
                bullseye_shadcn::tokens(b.ctx()).is_some(),
                "prepare ran before the build"
            );
            b.div_id("t", tw().w_px(width), |_| {});
        })
        .at("t")
        .rect()
        .width()
}

#[test]
fn theme_is_visible_to_the_build() {
    assert_eq!(themed_width(Theme::Light), 30.0);
    assert_eq!(themed_width(Theme::Dark), 60.0);
}

#[derive(Hash, Debug, Clone, Copy)]
enum Item {
    Lettering(Pick),
}

/// A card of a toggle group: a fixed-size sample, a name, and a check only
/// when chosen, in a card of fixed width.
///
/// ```html
/// <div class="flex items-center gap-2 p-2 gap-2 w-[140px]">
///   <span class="w-[34px] flex-none">Aa</span> <span>Monospace</span>
///   <span class="w-4">ok</span>   <!-- chosen only -->
/// </div>
/// ```
fn toggle_card(chosen: bool) -> Placed<()> {
    headless().layout(&(), move |b: &mut Builder<'_, '_, ()>, _: &()| {
        b.div_id(
            Item::Lettering(Pick::Mono),
            tw().flex().items_center().gap(2.0).p(2.0).w_px(140.0),
            |b| {
                b.span("sample", tw().w_px(34.0).flex_none(), "Aa");
                b.span("name", tw(), "Monospace");
                if chosen {
                    b.span("check", tw().w_px(16.0), "ok");
                }
            },
        );
    })
}

/// Guards the rule that an item's layout must not change with its selection:
/// choosing a card adds a check that overflows it, and the sample (a fixed
/// `w_px` is still shrinkable by default) must keep its width instead of
/// shrinking and moving the name. Without `flex_none` on the sample this
/// reports the sample and the name as changed.
#[test]
fn choosing_a_card_changes_only_its_check() {
    let (off, on) = (toggle_card(false), toggle_card(true));
    let card = off.at(Item::Lettering(Pick::Mono));
    let (sample, name) = (card.at("sample").rect(), card.at("name").rect());
    let (gap, inner) = (8.0, card.rect().width() - 16.0);
    assert!(
        sample.width() + gap + name.width() <= inner,
        "precondition: the unchosen card fits"
    );
    assert!(
        sample.width() + gap + name.width() + gap + 16.0 > inner,
        "precondition: the chosen card overflows"
    );
    for (before, after) in [(&off, &on), (&on, &off)] {
        let changes = before.diff(after);
        assert!(
            changes.len() == 1 && changes[0].path().ends_with("\"check\""),
            "{changes:#?}"
        );
    }
}
