//! The Markdown widget's test page: a fixed sample with every construct the
//! widget shows, and the ones it shows as their source.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use eggshell_ui::{Builder, Scope, tw};
use eggshell_ui_extras::markdown::{Directive, Directives, MarkdownExt};
use egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint};
use egui::{Color32, ColorImage, Context};

pub const SAMPLE: &str = r#"# Markdown in eggshell

A paragraph with **strong**, *emphasis*, ***both***, `inline code`, an [internal link](#lists) and an [external link](https://github.com/samitbasu/eggshell).
A soft break joins lines; a hard break\
starts a new one.

## Lists

- A bullet
- Another, with a nested list:
  - nested once
    - nested twice
- [x] a done task
- [ ] an open task

3. Ordered lists keep their start
4. and count on

### Quotes and code

> A block quote holds blocks.
>
> - even lists

```rust
fn main() {
    println!("plain for now");
}
```

#### Images

![A gradient swatch](swatch://gradient)

![A missing swatch](swatch://missing)

---

##### Directives

```eggshell-example counter
```

```eggshell-example nonexistent
```

###### What degrades

| tables | wait for #20 |
|--------|--------------|
| a      | b            |

<div>raw HTML</div>
"#;

#[derive(Default)]
pub struct State {
    pub followed: Vec<String>,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Page,
    Doc,
    Followed,
}

/// The directives of the page: `eggshell-example counter` mounts a counter.
pub fn directives() -> Directives<State> {
    Directives::new().with("eggshell-example", example)
}

fn example(s: &mut Scope<'_, '_, State>, directive: &Directive<'_>) {
    match directive.arg {
        "counter" => {
            s.div(tw().flex().flex_row().items_center().gap(3.0), |b| {
                b.button("add", "Add one")
                    .on_click(|state: &mut State| state.count += 1);
                b.span("count", tw(), "Counted");
            });
        }
        name => {
            let muted = common::tokens(s.ctx()).muted_foreground;
            s.span(
                "missing",
                tw().text(muted),
                format!("No example named {name:?}"),
            );
        }
    }
}

/// ```html
/// <main class="overflow-y-auto size-full p-6"><article class="max-w-[640px]">…</article></main>
/// ```
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    install_swatches(b.ctx());
    let directives = directives();
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.scroll(Part::Page, tw().size_full().p(6.0)).children(|s| {
        s.div(tw().flex().flex_col().gap(6.0).max_w_px(640.0), |b| {
            b.markdown(Part::Doc, SAMPLE)
                .on_link(|state: &mut State, href| state.followed.push(href.to_owned()))
                .directives(&directives)
                .add();
            let followed = state.followed.last().map_or("nothing yet", String::as_str);
            b.span(
                Part::Followed,
                tw().text(muted),
                format!("Followed: {followed}; counted {}", state.count),
            );
        });
    });
}

/// Images for the page, drawn in code: `swatch://gradient` is a gradient,
/// and any other swatch fails to load.
pub fn install_swatches(ctx: &Context) {
    if !ctx.is_loader_installed(Swatches::ID) {
        ctx.add_image_loader(Arc::new(Swatches));
    }
}

struct Swatches;

impl Swatches {
    const ID: &str = "eggshell-markdown-example-swatches";
}

impl ImageLoader for Swatches {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, _: &Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        match uri.strip_prefix("swatch://") {
            Some("gradient") => {
                let (width, height) = (240, 64);
                let pixels = (0..width * height)
                    .map(|at| {
                        let t = (at % width) as f32 / (width - 1) as f32;
                        Color32::from_rgb((40.0 + 200.0 * t) as u8, 90, (230.0 - 180.0 * t) as u8)
                    })
                    .collect();
                Ok(ImagePoll::Ready {
                    image: Arc::new(ColorImage::new([width, height], pixels)),
                })
            }
            Some(other) => Err(LoadError::Loading(format!("no swatch named {other:?}"))),
            None => Err(LoadError::NotSupported),
        }
    }

    fn forget(&self, _: &str) {}

    fn forget_all(&self) {}

    fn byte_size(&self) -> usize {
        0
    }
}

#[allow(dead_code, reason = "the tests include this file for `layout`")]
fn main() -> eframe::Result {
    common::run_app("markdown", State::default(), layout)
}
