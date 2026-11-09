//! The kaolin README layouts, shared by the gallery example, the layout tests
//! and the snapshot tests.
//!
//! kaolin builds static layouts with no interaction, so every node is
//! anonymous and the layouts are generic over the application type. Sizes are
//! kaolin's pixels, written with `_px` (egui points).
#![allow(
    dead_code,
    reason = "each crate that includes this file uses a subset of it"
)]

use bullseye::{Builder, TextSpec, Tw, tw};
use bullseye_shadcn::{Shadcn, Tokens};
use egui::{Color32, FontFamily, ImageSource};

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// The base text role.
pub const BODY: TextSpec = TextSpec {
    size: 16.0,
    line_height: 20.0,
    family: FontFamily::Proportional,
};

/// The greys of the README's layouts, as the theme's surfaces. The colours
/// that are the subject of a demo (the blues, the dark grey footer, the black
/// embedded stage) are not here: they are the same in both themes.
struct Greys {
    /// The stage a layout sits on: kaolin's white, with black text.
    stage: Color32,
    ink: Color32,
    /// A container on the stage: kaolin's light grey.
    panel: Color32,
    /// A panel beside a panel, and an edge: kaolin's grey.
    deep: Color32,
    /// A card on a panel: kaolin's white.
    card: Color32,
}

impl Greys {
    fn of(tokens: &Tokens) -> Self {
        Self {
            stage: tokens.background,
            ink: tokens.foreground,
            panel: tokens.muted,
            deep: tokens.ring,
            card: tokens.card,
        }
    }
}

fn greys<A>(b: &B<'_, '_, A>) -> Greys {
    Greys::of(&Shadcn::of(b.ctx()).tokens)
}

/// A proportional text role of `size` points.
pub fn sized(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family: FontFamily::Proportional,
    }
}

/// A monospace text role of `size` points, standing in for kaolin's font 1.
pub fn mono(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family: FontFamily::Monospace,
    }
}

/// ```html
/// <div class="flex size-full items-center justify-center bg-background">
///   <span class="text-[48px]/[60px] text-foreground">Hello, World!</span>
/// </div>
/// ```
pub fn hello_world<A>(b: &mut B<'_, '_, A>) {
    let greys = greys(b);
    b.div(
        tw().flex()
            .size_full()
            .items_center()
            .justify_center()
            .bg(greys.stage),
        |b| {
            b.text(tw().role(sized(48.0)).text(greys.ink), "Hello, World!");
        },
    );
}

/// Container classes shared by the direction layouts; the caller adds the size.
///
/// ```html
/// <div class="flex flex-row items-center justify-center gap-2.5"> ... </div>
/// ```
pub fn direction_and_alignment() -> Tw {
    tw().flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(2.5)
}

/// ```html
/// <div class="w-[40px] h-[20px] bg-[#00008b]"></div>
/// <!-- x3, widths 40/50/60px -->
/// ```
pub fn three_boxes<A>(b: &mut B<'_, '_, A>) {
    for width in [40.0, 50.0, 60.0] {
        b.div(tw().w_px(width).h_px(20.0).bg(Color32::DARK_BLUE), |_| {});
    }
}

/// The four layout directions, one container each.
///
/// ```html
/// <div class="flex flex-col gap-2 p-2">
///   <div class="flex flex-row items-center justify-center gap-2.5 w-[300px] h-[100px] bg-muted">
///     <div class="w-[40px] h-[20px] bg-[#00008b]"></div>
///     <div class="w-[50px] h-[20px] bg-[#00008b]"></div>
///     <div class="w-[60px] h-[20px] bg-[#00008b]"></div>
///   </div>
///   <!-- the same container with flex-row-reverse, flex-col and flex-col-reverse in place of flex-row -->
/// </div>
/// ```
pub fn directions<A>(b: &mut B<'_, '_, A>) {
    let panel = greys(b).panel;
    let container = || direction_and_alignment().w_px(300.0).h_px(100.0).bg(panel);
    b.div(tw().flex().flex_col().gap(2.0).p(2.0), |b| {
        for direction in [
            container(),
            container().flex_row_reverse(),
            container().flex_col(),
            container().flex_col_reverse(),
        ] {
            b.div(direction, |b| three_boxes(b));
        }
    });
}

/// The eight sizing modes, as the classes on the child:
///
/// ```html
/// <div class="w-[200px] h-[100px]"></div>
/// <div class="size-[150px]"></div>
/// <div></div>
/// <div class="max-w-[320px]"></div>
/// <div class="min-w-[120px] max-w-[320px]"></div>
/// <div class="flex-1"></div>
/// <div class="grow-[2]"></div>
/// <div class="flex-1 min-w-[120px] max-w-[480px]"></div>
/// ```
pub fn sizing_modes() -> [Tw; 8] {
    [
        tw().w_px(200.0).h_px(100.0),
        tw().size_px(150.0),
        tw(),
        tw().max_w_px(320.0),
        tw().min_w_px(120.0).max_w_px(320.0),
        tw().flex_1(),
        tw().grow_by(2.0),
        tw().flex_1().min_w_px(120.0).max_w_px(480.0),
    ]
}

/// The text each sizing mode is shown with, if any.
pub const SIZING_CONTENT: [&str; 8] = ["", "", "Fit me", LONG, "Hi", "", "", ""];

/// A text that wraps in a 320 point box.
pub const LONG: &str = "This is a long text that will automatically wrap to multiple lines of text";

/// One sizing mode in a 600 point row that does not stretch its child.
///
/// ```html
/// <div class="flex w-[600px] items-start bg-muted">
///   <div class="min-h-[16px] bg-[#00008b] text-white"><!-- one of the sizing modes --></div>
/// </div>
/// ```
pub fn sized_child<A>(b: &mut B<'_, '_, A>, style: Tw, content: &'static str) {
    let panel = greys(b).panel;
    b.div(tw().flex().w_px(600.0).items_start().bg(panel), |b| {
        b.div(
            style
                .min_h_px(16.0)
                .bg(Color32::DARK_BLUE)
                .text(Color32::WHITE),
            |b| {
                if !content.is_empty() {
                    b.text(tw(), content)
                }
            },
        )
    });
}

/// ```html
/// <div class="flex flex-col gap-1 p-2">
///   <div class="flex w-[600px] items-start bg-muted">
///     <div class="w-[200px] h-[100px] min-h-[16px] bg-[#00008b] text-white"></div>
///   </div>
///   <!-- seven more rows, one per sizing mode -->
/// </div>
/// ```
pub fn sizing<A>(b: &mut B<'_, '_, A>) {
    b.div(tw().flex().flex_col().gap(1.0).p(2.0), |b| {
        for (mode, content) in sizing_modes().into_iter().zip(SIZING_CONTENT) {
            sized_child(b, mode, content);
        }
    });
}

/// ```html
/// <div class="bg-[#0000ff] rounded-lg p-4 border-2 border-black">
///   <span class="font-mono text-[24px]/[30px] text-white">Styled text</span>
/// </div>
/// ```
pub fn styling<A>(b: &mut B<'_, '_, A>) {
    b.div(
        tw().bg(Color32::BLUE)
            .rounded_lg()
            .p(4.0)
            .border(2.0)
            .border_color(Color32::BLACK),
        |b| {
            b.text(tw().role(mono(24.0)).text(Color32::WHITE), "Styled text");
        },
    );
}

/// ```html
/// <div class="flex flex-col size-full">
///   <div class="w-full h-[60px] bg-[#00008b] text-white">Header</div>
///   <div class="w-full flex-1 bg-muted">Main Content</div>
///   <div class="w-full h-[40px] bg-[#606060] text-white">Footer</div>
/// </div>
/// ```
pub fn responsive<A>(b: &mut B<'_, '_, A>) {
    let panel = greys(b).panel;
    b.div(tw().flex().flex_col().size_full(), |b| {
        b.div(
            tw().w_full()
                .h_px(60.0)
                .bg(Color32::DARK_BLUE)
                .text(Color32::WHITE),
            |b| b.text(tw(), "Header"),
        );
        b.div(tw().w_full().flex_1().bg(panel), |b| {
            b.text(tw(), "Main Content")
        });
        b.div(
            tw().w_full()
                .h_px(40.0)
                .bg(Color32::DARK_GRAY)
                .text(Color32::WHITE),
            |b| b.text(tw(), "Footer"),
        );
    });
}

/// ```html
/// <div class="flex flex-row gap-5 size-full">
///   <div class="w-[200px] flex-none bg-ring">Sidebar</div>
///   <div class="flex-1 bg-background">Main Content</div>
/// </div>
/// ```
pub fn multi_column<A>(b: &mut B<'_, '_, A>) {
    let greys = greys(b);
    b.div(tw().flex().flex_row().gap(5.0).size_full(), |b| {
        b.div(tw().w_px(200.0).flex_none().bg(greys.deep), |b| {
            b.text(tw(), "Sidebar")
        });
        b.div(tw().flex_1().bg(greys.stage), |b| {
            b.text(tw(), "Main Content")
        });
    });
}

/// The text of [`text_wrapping`].
pub const WRAPPING: &str = "This is a long text that will automatically wrap to multiple lines";

/// ```html
/// <div class="w-[200px]">This is a long text that will automatically wrap to multiple lines</div>
/// ```
pub fn text_wrapping<A>(b: &mut B<'_, '_, A>) {
    b.div(tw().w_px(200.0), |b| b.text(tw(), WRAPPING));
}

/// ```html
/// <div class="flex flex-row size-full">
///   <div class="grow-[1] bg-muted"></div>
///   <div class="grow-[3] bg-ring"></div>
/// </div>
/// ```
pub fn proportional_growth<A>(b: &mut B<'_, '_, A>) {
    let greys = greys(b);
    b.div(tw().flex().flex_row().size_full(), |b| {
        b.div(tw().grow_by(1.0).bg(greys.panel), |_| {});
        b.div(tw().grow_by(3.0).bg(greys.deep), |_| {});
    });
}

/// ```html
/// <div class="p-2 rounded-md bg-card border border-ring">label</div>
/// ```
pub fn card<A>(b: &mut B<'_, '_, A>, label: &str) {
    let greys = greys(b);
    b.div(
        tw().p(2.0)
            .rounded_md()
            .bg(greys.card)
            .border(1.0)
            .border_color(greys.deep),
        |b| b.text(tw(), label),
    );
}

/// ```html
/// <div class="flex flex-col gap-2 p-4">
///   <div class="p-2 bg-muted">Content 1</div>
///   <div class="p-2 bg-muted">
///     <div class="p-2 rounded-md bg-card border border-ring">Nested content</div>
///   </div>
/// </div>
/// ```
pub fn nested<A>(b: &mut B<'_, '_, A>) {
    let panel = greys(b).panel;
    b.div(tw().flex().flex_col().gap(2.0).p(4.0), |b| {
        b.div(tw().p(2.0).bg(panel), |b| b.text(tw(), "Content 1"));
        b.div(tw().p(2.0).bg(panel), |b| {
            card(b, "Nested content");
        });
    });
}

/// ```html
/// <div class="flex flex-col size-full items-center justify-center bg-black text-white">
///   <div class="flex flex-row gap-1">
///     <span class="font-mono text-[10px]/[12.5px]">Hello, World!</span>
///     <img class="size-[16px]" src="...">
///   </div>
///   Kaolin Renderer
/// </div>
/// ```
pub fn embedded_layout<A>(b: &mut B<'_, '_, A>, logo: ImageSource<'static>) {
    b.div(
        tw().flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .bg(Color32::BLACK)
            .text(Color32::WHITE),
        |b| {
            b.div(tw().flex().flex_row().gap(1.0), |b| {
                b.text(tw().role(mono(10.0)), "Hello, World!");
                b.image(tw().size_px(16.0), logo);
            });
            b.text(tw(), "Kaolin Renderer");
        },
    );
}

/// A 16 point bullseye.
pub fn logo_image() -> egui::ColorImage {
    const SIZE: usize = 16;
    let centre = (SIZE as f32 - 1.0) / 2.0;
    let pixels = (0..SIZE * SIZE)
        .map(|index| {
            let (x, y) = (
                (index % SIZE) as f32 - centre,
                (index / SIZE) as f32 - centre,
            );
            match (x.hypot(y) / 2.0) as usize {
                0 | 2 | 4 => Color32::RED,
                1 | 3 => Color32::WHITE,
                _ => Color32::TRANSPARENT,
            }
        })
        .collect();
    egui::ColorImage::new([SIZE, SIZE], pixels)
}
