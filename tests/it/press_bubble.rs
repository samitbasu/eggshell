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
}

const LEAVES: [Leaf; 10] = [
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
];

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
