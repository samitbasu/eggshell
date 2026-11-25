//! The Markdown widget's test page: a fixed sample with every construct the
//! widget shows, and the ones it shows as their source.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use eggshell_ui::owned::markdown::{Directive, Directives, MarkdownExt, Typeset, heading_id};
use eggshell_ui::{Builder, Scope, Shell, Span, tw};
use egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint};
use egui::{Align, Color32, ColorImage, Context, Id, Rect};

pub const SAMPLE: &str = r#"# Markdown in eggshell

A paragraph with **strong**, *emphasis*, ***both***, `inline code` and links: [internal](#directives), [relative](guide.md) and [external](https://github.com/samitbasu/eggshell).
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

###### Tables

| Left | Centred | Right |
|:-----|:-------:|------:|
| *emphasis* and `code` | [a relative link](table.md) | 1 |
| a longer cell, whose text wraps when the table is wider than the page, as a paragraph's does | two | 22 |
| a ragged row |

###### What degrades

<div>raw HTML</div>
"#;

/// A sibling page, shown in place of the sample when a link names it.
pub const GUIDE: &str = r#"# A guide

A sibling page, shown by the demo when a relative link names it. From here:

- [the table page](table.md)
- [its alignments](table.md#alignments)
- [the sample's tables](index.md#tables)
- [a page that is not there](missing.md)

## Routing

The widget passes a relative link to `on_link`; the demo looks the page up, shows it from its top, and keeps the trail that Back retraces.
"#;

/// A sibling page with a heading far enough down to land on.
pub const TABLE_PAGE: &str = r#"# Table page

A table's columns take their alignment from the delimiter row.

## Columns

Each column is as wide as its widest cell, and a cell's text wraps when the table would be wider than the page.

## Rows

A row with fewer cells than the header is padded with empty ones; a row with more is cut.

## Alignments

| Left | Centred | Right |
|:-----|:-------:|------:|
| `:---` | `:---:` | `---:` |

A colon on the left of the delimiter aligns its column left, on the right aligns it right, and on both centres it; without one, the column takes the default, left.

## Where next

Back on [the guide](guide.md), or on [the sample's tables](index.md#tables).

The demo keeps no history beyond its trail: a reload starts on the sample again.
"#;

/// The pages the demo can show, named as relative links name them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Page {
    #[default]
    Sample,
    Guide,
    Table,
}

impl Page {
    fn named(name: &str) -> Option<Self> {
        match name {
            "index.md" => Some(Self::Sample),
            "guide.md" => Some(Self::Guide),
            "table.md" => Some(Self::Table),
            _ => None,
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::Sample => SAMPLE,
            Self::Guide => GUIDE,
            Self::Table => TABLE_PAGE,
        }
    }
}

#[derive(Default)]
pub struct State {
    pub page: Page,
    /// The pages Back returns to, the latest last.
    pub trail: Vec<Page>,
    pub followed: Vec<String>,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Page,
    Back,
    Doc,
    Followed,
}

/// Where a link to another page lands: on the heading its anchor names, or
/// at the page's top.
#[derive(Clone)]
struct Landing {
    shell: Shell,
    page: Id,
    doc: Id,
}

impl Landing {
    fn on(&self, anchor: Option<&str>) {
        match anchor {
            Some(anchor) => self
                .shell
                .scroll_into_view(heading_id(self.doc, anchor), Some(Align::Min)),
            None => self
                .shell
                .scroll_to_rect(self.page, Rect::ZERO, Some(Align::Min)),
        }
    }

    /// Shows the page `href` names, if the demo has it.
    fn follow(&self, state: &mut State, href: &str) {
        state.followed.push(href.to_owned());
        let (name, anchor) = match href.split_once('#') {
            Some((name, anchor)) => (name, Some(anchor)),
            None => (href, None),
        };
        if let Some(page) = Page::named(name) {
            state.trail.push(std::mem::replace(&mut state.page, page));
            self.on(anchor);
        }
    }

    fn back(&self, state: &mut State) {
        if let Some(page) = state.trail.pop() {
            state.page = page;
            self.on(None);
        }
    }
}

/// The directives of the page: `eggshell-example counter` mounts a counter.
pub fn directives() -> Directives<State> {
    Directives::new().with("eggshell-example", example)
}

fn example(s: &mut Scope<'_, '_, State>, _: &State, directive: &Directive<'_>) {
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
/// <main class="overflow-y-auto size-full p-6"><article class="max-w-[37em]">
///   <a href="…">Back</a> …
/// </article></main>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    install_swatches(b.ctx());
    let directives = directives();
    let muted = common::tokens(b.ctx()).muted_foreground;
    let page = b.id_of(Part::Page);
    b.scroll(Part::Page, tw().size_full().p(6.0)).children(|s| {
        let landing = Landing {
            shell: s.shell(),
            page,
            doc: s.id_of(Part::Doc),
        };
        s.div(
            tw().flex()
                .flex_col()
                .gap(6.0)
                .max_w_px(Typeset::default().measure_width()),
            |b| {
                if !state.trail.is_empty() {
                    let back = landing.clone();
                    b.spans(Part::Back, tw(), Span::new("Back").link("back"))
                        .on_link(move |state: &mut State, _| back.back(state));
                }
                b.markdown(Part::Doc, state.page.source())
                    .on_link(move |state: &mut State, href| landing.follow(state, href))
                    .directives(&directives, state)
                    .add();
                let followed = state.followed.last().map_or("nothing yet", String::as_str);
                b.span(
                    Part::Followed,
                    tw().text(muted),
                    format!("Followed: {followed}; counted {}", state.count),
                );
            },
        );
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
