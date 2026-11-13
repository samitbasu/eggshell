//! A node that senses nothing and has no handler is transparent to presses:
//! the nearest sensing ancestor gets them, whatever the leaf paints.

use eggshell_ui::{Builder, Icon, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Pos2, vec2};

use crate::support::{self, hover_over, press_and_release, rect_of};

#[derive(Clone, Copy, Debug)]
enum Leaf {
    Text,
    Span,
    Glyph,
    Icon,
    Image,
    Rule,
    RuleWithBackground,
    Paint,
    PaintedDiv,
    BorderedDiv,
    RingedRule,
    RingedRoundedRule,
    RingedImage,
}

const LEAVES: [Leaf; 13] = [
    Leaf::Text,
    Leaf::Span,
    Leaf::Glyph,
    Leaf::Icon,
    Leaf::Image,
    Leaf::Rule,
    Leaf::RuleWithBackground,
    Leaf::Paint,
    Leaf::PaintedDiv,
    Leaf::BorderedDiv,
    Leaf::RingedRule,
    Leaf::RingedRoundedRule,
    Leaf::RingedImage,
];

const TILE: &str = "/\"tile\"";
const PARENT: &str = "/\"parent\"";

struct App {
    leaf: Leaf,
    clicks: u32,
}

fn source() -> egui::ImageSource<'static> {
    egui::ImageSource::Texture(egui::load::SizedTexture::new(
        egui::TextureId::default(),
        vec2(4.0, 4.0),
    ))
}

/// ```html
/// <div class="group flex flex-col p-1 w-[200px] h-[100px] bg-gray-700 border" onclick="...">
///   <!-- leaf, w-full h-[28px] -->
///   <span>name</span>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let fill = tw().w_full().h_px(28.0);
    b.div_id(
        "parent",
        tw().sense_click()
            .group()
            .hover(|p| p.bg(Color32::RED))
            .flex()
            .flex_col()
            .p(4.0)
            .bg(Color32::DARK_GRAY)
            .border(1.0)
            .border_color(Color32::GRAY)
            .w_px(200.0)
            .h_px(100.0),
        |b| {
            match app.leaf {
                Leaf::Text => b.text(fill, "pressed"),
                Leaf::Span => {
                    b.span("leaf", fill, "pressed");
                }
                Leaf::Glyph => b.glyph(fill, Icon::new(source())),
                Leaf::Icon => {
                    b.icon("leaf", fill, Icon::new(source()));
                }
                Leaf::Image => b.image(fill, source()),
                Leaf::Rule => b.rule(fill),
                Leaf::RuleWithBackground => b.rule(fill.bg(Color32::GRAY)),
                Leaf::RingedRule => b.rule(fill.ring(1.0, Color32::GRAY)),
                Leaf::RingedRoundedRule => b.rule(
                    fill.rounded_lg()
                        .ring(1.0, Color32::GRAY)
                        .bg(Color32::BLACK),
                ),
                Leaf::RingedImage => b.image(fill.rounded_lg().ring(1.0, Color32::GRAY), source()),
                Leaf::Paint => b.paint(fill, |_, _, _| {}),
                Leaf::PaintedDiv => b.div(fill.bg(Color32::GRAY), |_| {}),
                Leaf::BorderedDiv => b.div(fill.border(1.0).border_color(Color32::GRAY), |_| {}),
            }
            b.text(tw(), "name");
        },
    )
    .on_click(|app: &mut App| app.clicks += 1);
}

#[test]
fn a_press_on_any_non_sensing_leaf_reaches_the_sensing_parent() {
    let mut failed = Vec::new();
    for leaf in LEAVES {
        let mut harness = support::harness(
            vec2(400.0, 300.0),
            TextStyle::new(
                TextSpec {
                    size: 16.0,
                    line_height: 20.0,
                    family: FontFamily::Proportional,
                },
                Color32::WHITE,
            ),
            App { leaf, clicks: 0 },
            view,
        );
        harness.run();
        let parent = rect_of(&harness, PARENT);
        let first = harness
            .state()
            .nodes
            .iter()
            .skip_while(|(path, _)| path != PARENT)
            .nth(1)
            .expect("a leaf")
            .1;
        let at: Pos2 = first.center();
        let covering: Vec<_> = harness
            .state()
            .nodes
            .iter()
            .filter(|(path, rect)| {
                path != PARENT && rect.contains(at) && parent.contains_rect(*rect)
            })
            .collect();
        assert!(
            !covering.is_empty(),
            "{leaf:?}: a leaf inside the parent holds the press point"
        );
        hover_over(&mut harness, at);
        press_and_release(&mut harness, at);
        if harness.state().app.clicks != 1 {
            failed.push(leaf);
        }
    }
    assert!(
        failed.is_empty(),
        "the parent lost the press under {failed:?}"
    );
}

#[derive(Default)]
struct Sheet {
    clicks: u32,
}

/// ```html
/// <div class="absolute inset-0" onclick="..."></div> <!-- the canvas beneath -->
/// <div class="absolute inset-0 flex">
///   <div class="bg ring shadow rounded-xl p-2">
///     <div class="flex">
///       <button class="flex flex-col gap-1.5 w-46 p-1.5 rounded-xl hover:bg" title="Open">
///         <div class="w-full h-28 rounded-lg ring-1 bg"></div>
///         <span>name</span><span>kind</span>
///       </button>
///     </div>
///   </div>
/// </div>
/// ```
fn sheet(b: &mut Builder<'_, '_, Sheet>, _: &Sheet) {
    let shadow = egui::Shadow {
        offset: [0, 4],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    b.div_id("canvas", tw().absolute().inset(0.0).sense_click(), |_| {});
    b.div(tw().absolute().inset(0.0).flex(), |b| {
        b.div(
            tw().bg(Color32::DARK_GRAY)
                .ring(1.0, Color32::GRAY)
                .shadow(shadow)
                .rounded_xl()
                .p(2.0),
            |b| {
                b.div(tw().flex(), |b| {
                    b.div_id(
                        "tile",
                        tw().flex()
                            .flex_col()
                            .gap(1.5)
                            .w(46.0)
                            .p(1.5)
                            .rounded_xl()
                            .sense_click()
                            .hover(|p| p.bg(Color32::RED)),
                        |b| {
                            b.rule(
                                tw().w_full()
                                    .h(28.0)
                                    .rounded_lg()
                                    .ring(1.0, Color32::GRAY)
                                    .bg(Color32::BLACK),
                            );
                            b.text(tw(), "name");
                            b.text(tw(), "kind");
                        },
                    )
                    .role(egui::Role::Button)
                    .named("Open")
                    .tooltip("Open it")
                    .on_click(|app: &mut Sheet| app.clicks += 1);
                });
            },
        );
    });
}

#[test]
fn a_ringed_leaf_in_a_ringed_sheet_leaves_the_tile_its_press() {
    let mut harness = support::harness(
        vec2(500.0, 400.0),
        TextStyle::new(
            TextSpec {
                size: 16.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
            },
            Color32::WHITE,
        ),
        Sheet::default(),
        sheet,
    );
    harness.run();
    let tile = rect_of(&harness, TILE);
    let leaf = harness
        .state()
        .nodes
        .iter()
        .skip_while(|(path, _)| path != TILE)
        .nth(1)
        .expect("the leaf")
        .1;
    assert!(tile.contains_rect(leaf));
    hover_over(&mut harness, leaf.center());
    harness.run_steps(40);
    press_and_release(&mut harness, leaf.center());
    assert_eq!(harness.state().app.clicks, 1);
}
