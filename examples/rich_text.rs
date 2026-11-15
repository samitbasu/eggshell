//! Rich inline text: paragraphs of styled spans, each one wrapping node,
//! with links, weights, italics and inline code.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, FontStyle, Span, Weight, tw};

#[derive(Default)]
pub struct State {
    pub followed: Option<String>,
}

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Intro,
    Wrapping,
    Followed,
}

/// ```html
/// <article class="flex flex-col gap-4 p-6 w-[440px]">
///   <p>Spans of <b>bold</b>, <strong class="font-medium">medium</strong>, <i>italic</i>
///      and <code>inline_code()</code> text, with an <a href="#links">internal link</a>
///      and an <a href="https://github.com/samitbasu/eggshell">external one</a>.</p>
///   <p>A longer paragraph … <code>a_long_function_name(with, arguments)</code> …</p>
///   <p class="text-muted-foreground">Followed: …</p>
/// </article>
/// ```
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(4.0).p(6.0).w_px(440.0), |b| {
        b.spans(
            Part::Intro,
            tw(),
            vec![
                Span::new("Spans of "),
                Span::new("bold").weight(Weight::Bold),
                Span::new(", "),
                Span::new("medium").weight(Weight::Medium),
                Span::new(", "),
                Span::new("italic").font_style(FontStyle::Italic),
                Span::new(" and "),
                Span::new("inline_code()").code(),
                Span::new(" text, with an "),
                Span::new("internal link").link("#links"),
                Span::new(" and an "),
                Span::new("external one").link("https://github.com/samitbasu/eggshell"),
                Span::new("."),
            ],
        )
        .on_link(|state: &mut State, href| state.followed = Some(href.to_owned()));
        b.spans(
            Part::Wrapping,
            tw(),
            vec![
                Span::new("A longer paragraph wraps as one node: its spans flow across rows, and "),
                Span::new("a_long_function_name(with, arguments)").code(),
                Span::new(" keeps its tint on every row it reaches, while "),
                Span::new("a link that wraps").link("#wrapped"),
                Span::new(" stays one link."),
            ],
        )
        .on_link(|state: &mut State, href| state.followed = Some(href.to_owned()));
        let followed = state.followed.as_deref().unwrap_or("nothing yet");
        b.span(
            Part::Followed,
            tw().text(muted),
            format!("Followed: {followed}"),
        );
    });
}

#[allow(
    dead_code,
    reason = "the snapshot tests include this file for `layout`"
)]
fn main() -> eframe::Result {
    common::run_app("rich_text", State::default(), layout)
}
