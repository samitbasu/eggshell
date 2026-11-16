//! Markdown source into a tree of blocks whose text is eggshell spans.

use std::collections::HashMap;
use std::ops::Range;

use eggshell_ui::owned::Align;
use eggshell_ui::{FontStyle, Span, Spans, Weight};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// A block of a Markdown document.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Block {
    Paragraph(Spans),
    Heading(Heading),
    List(List),
    Quote(Vec<Block>),
    Code {
        info: String,
        text: String,
    },
    /// Inline in Markdown; a block here, splitting its paragraph.
    Image {
        uri: String,
        alt: String,
    },
    Rule,
    Table(Table),
    Unsupported {
        construct: Construct,
        source: String,
    },
}

/// A GFM table: a column per alignment, and rows of cells as written, which
/// may be shorter or longer than the header.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Table {
    pub align: Vec<Align>,
    pub header: Vec<Spans>,
    pub rows: Vec<Vec<Spans>>,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    /// TODO(prose)
    pub level: Level,
    /// TODO(prose)
    pub anchor: String,
    pub(crate) spans: Spans,
}

impl Heading {
    /// TODO(prose)
    pub fn text(&self) -> String {
        self.spans.text()
    }
}

/// TODO(prose)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// `#`
    H1,
    /// `##`
    H2,
    /// `###`
    H3,
    /// `####`
    H4,
    /// `#####`
    H5,
    /// `######`
    H6,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct List {
    /// The first number of an ordered list.
    pub start: Option<u64>,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Item {
    pub task: Option<Task>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Task {
    Open,
    Done,
}

/// A construct the widget shows as its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Construct {
    Html,
}

/// The blocks of `source`. HTML is recognised so that it can be shown as
/// its source; other extensions are off, so their syntax reads
/// as the text it is.
pub(crate) fn parse(source: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_GFM;
    let mut walk = Walk::default();
    let mut events = Parser::new_ext(source, options).into_offset_iter();
    while let Some((event, range)) = events.next() {
        match event {
            Event::Start(Tag::HtmlBlock) => {
                skip_to(&mut events, TagEnd::HtmlBlock);
                walk.unsupported(Construct::Html, source.get(range).unwrap_or_default());
            }
            event => walk.event(event),
        }
    }
    walk.finish()
}

fn skip_to<'a>(events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>, end: TagEnd) {
    for (event, _) in events.by_ref() {
        if event == Event::End(end) {
            return;
        }
    }
}

/// A block that holds blocks, open while its events arrive.
enum Container {
    Quote(Vec<Block>),
    List(List),
    Item(Item),
    /// A table, and the cells of the row being read.
    Table(Table, Vec<Spans>),
}

/// The text-bearing block open while its inline events arrive.
enum Leaf {
    Paragraph,
    Heading { level: Level, id: Option<String> },
    Code { info: String, text: String },
    Image { uri: String, alt: String },
    Cell,
}

/// The inline styles in force: how deep in each, and the links entered.
#[derive(Clone, Default, PartialEq)]
struct Marks {
    strong: u32,
    emphasis: u32,
    links: Vec<String>,
    font: Font,
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Font {
    #[default]
    Text,
    Code,
}

impl Marks {
    fn span(&self, text: String) -> Span {
        let mut span = Span::new(text);
        if self.strong > 0 {
            span = span.weight(Weight::Bold);
        }
        if self.emphasis > 0 {
            span = span.font_style(FontStyle::Italic);
        }
        if self.font == Font::Code {
            span = span.code();
        }
        match self.links.last() {
            Some(href) => span.link(href.clone()),
            None => span,
        }
    }
}

struct Walk {
    blocks: Vec<Block>,
    /// The containers open around the next block, innermost last.
    containers: Vec<Container>,
    leaf: Option<Leaf>,
    marks: Marks,
    /// Text of one style, not yet a span.
    run: (Marks, String),
    spans: Spans,
    anchors: HashMap<String, usize>,
}

impl Default for Walk {
    fn default() -> Self {
        Self {
            blocks: Vec::new(),
            containers: Vec::new(),
            leaf: None,
            marks: Marks::default(),
            run: (Marks::default(), String::new()),
            spans: Spans::new(),
            anchors: HashMap::new(),
        }
    }
}

impl Walk {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.styled(Font::Code, &code),
            Event::InlineHtml(html) | Event::Html(html) => self.styled(Font::Code, &html),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.text("\n"),
            Event::Rule => self.block(Block::Rule),
            Event::TaskListMarker(done) => {
                if let Some(Container::Item(item)) = self.containers.last_mut() {
                    item.task = Some(if done { Task::Done } else { Task::Open });
                }
            }
            Event::InlineMath(text) | Event::DisplayMath(text) | Event::FootnoteReference(text) => {
                self.text(&text)
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.open(Leaf::Paragraph),
            Tag::Heading { level, id, .. } => self.open(Leaf::Heading {
                level: level.into(),
                id: id.map(|id| id.to_string()),
            }),
            Tag::CodeBlock(kind) => self.open(Leaf::Code {
                info: match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                },
                text: String::new(),
            }),
            Tag::Table(align) => self.contain(Container::Table(
                Table {
                    align: align.into_iter().map(column_align).collect(),
                    ..Table::default()
                },
                Vec::new(),
            )),
            Tag::TableCell => self.open(Leaf::Cell),
            Tag::Image { .. } if matches!(self.leaf, Some(Leaf::Cell)) => {}
            Tag::Image { dest_url, .. } => {
                self.close_leaf();
                self.leaf = Some(Leaf::Image {
                    uri: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            Tag::BlockQuote(_) => self.contain(Container::Quote(Vec::new())),
            Tag::List(start) => self.contain(Container::List(List {
                start,
                items: Vec::new(),
            })),
            Tag::Item => self.contain(Container::Item(Item::default())),
            Tag::Emphasis => self.marks.emphasis = self.marks.emphasis.saturating_add(1),
            Tag::Strong => self.marks.strong = self.marks.strong.saturating_add(1),
            Tag::Link { dest_url, .. } => self.marks.links.push(dest_url.to_string()),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock => self.close_leaf(),
            TagEnd::Image if matches!(self.leaf, Some(Leaf::Cell)) => {}
            TagEnd::Image => self.open(Leaf::Paragraph),
            TagEnd::TableCell => self.close_leaf(),
            TagEnd::TableHead | TagEnd::TableRow => self.end_row(tag),
            TagEnd::Table => self.uncontain(),
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item => self.uncontain(),
            TagEnd::Emphasis => self.marks.emphasis = self.marks.emphasis.saturating_sub(1),
            TagEnd::Strong => self.marks.strong = self.marks.strong.saturating_sub(1),
            TagEnd::Link => {
                self.marks.links.pop();
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        match &mut self.leaf {
            Some(Leaf::Code { text: code, .. }) => code.push_str(text),
            Some(Leaf::Image { alt, .. }) => alt.push_str(text),
            Some(Leaf::Paragraph | Leaf::Heading { .. } | Leaf::Cell) => self.push_run(text),
            None => {
                self.leaf = Some(Leaf::Paragraph);
                self.push_run(text);
            }
        }
    }

    fn styled(&mut self, font: Font, text: &str) {
        let marks = std::mem::take(&mut self.marks);
        self.marks = Marks {
            font,
            ..marks.clone()
        };
        self.text(text);
        self.marks = marks;
    }

    fn push_run(&mut self, text: &str) {
        if self.run.0 != self.marks {
            self.flush_run();
            self.run.0 = self.marks.clone();
        }
        self.run.1.push_str(text);
    }

    fn flush_run(&mut self) {
        let (marks, text) = &mut self.run;
        if !text.is_empty() {
            self.spans.push(marks.span(std::mem::take(text)));
        }
    }

    fn open(&mut self, leaf: Leaf) {
        self.close_leaf();
        self.leaf = Some(leaf);
    }

    fn close_leaf(&mut self) {
        self.flush_run();
        let spans = std::mem::take(&mut self.spans);
        let block = match self.leaf.take() {
            None => return,
            Some(Leaf::Paragraph) if spans.is_empty() => return,
            Some(Leaf::Paragraph) => Block::Paragraph(spans),
            Some(Leaf::Heading { level, id }) => {
                let anchor = self.anchor(id.unwrap_or_else(|| slug(&spans.text())));
                Block::Heading(Heading {
                    level,
                    anchor,
                    spans,
                })
            }
            Some(Leaf::Code { info, text }) => Block::Code { info, text },
            Some(Leaf::Image { uri, alt }) => Block::Image { uri, alt },
            Some(Leaf::Cell) => match self.containers.last_mut() {
                Some(Container::Table(_, row)) => return row.push(spans),
                // TODO(#279): report a cell outside a table; its text is a paragraph.
                _ if spans.is_empty() => return,
                _ => Block::Paragraph(spans),
            },
        };
        self.block(block);
    }

    /// Ends the header or a body row of the open table, if one is open.
    fn end_row(&mut self, end: TagEnd) {
        self.close_leaf();
        // TODO(#279): report a row ended outside a table; it is ignored.
        if let Some(Container::Table(table, row)) = self.containers.last_mut() {
            let row = std::mem::take(row);
            match end {
                TagEnd::TableHead => table.header = row,
                _ => table.rows.push(row),
            }
        }
    }

    /// `wanted`, or `wanted-1`, `wanted-2`… when an earlier heading has it.
    fn anchor(&mut self, wanted: String) -> String {
        let seen = self.anchors.entry(wanted.clone()).or_insert(0);
        let anchor = match *seen {
            0 => wanted,
            n => format!("{wanted}-{n}"),
        };
        *seen += 1;
        anchor
    }

    fn block(&mut self, block: Block) {
        self.close_leaf();
        match self.containers.last_mut() {
            None => self.blocks.push(block),
            Some(Container::Quote(blocks)) => blocks.push(block),
            Some(Container::Item(item)) => item.blocks.push(block),
            // TODO(#279): report a block inside a table, which holds only cells.
            Some(Container::Table(..)) => self.blocks.push(block),
            Some(Container::List(list)) => list.items.push(Item {
                task: None,
                blocks: vec![block],
            }),
        }
    }

    fn unsupported(&mut self, construct: Construct, source: &str) {
        self.block(Block::Unsupported {
            construct,
            source: source.trim_end().to_owned(),
        });
    }

    fn contain(&mut self, container: Container) {
        self.close_leaf();
        self.containers.push(container);
    }

    /// Closes the innermost container. pulldown-cmark balances what it
    /// opens; should it not, an item outside a list keeps its blocks and a
    /// close with nothing open is ignored.
    fn uncontain(&mut self) {
        self.close_leaf();
        // TODO(#279): report a close with nothing open, or an item outside a list.
        match self.containers.pop() {
            None => {}
            Some(Container::Item(item)) => match self.containers.last_mut() {
                Some(Container::List(list)) => list.items.push(item),
                _ => item.blocks.into_iter().for_each(|block| self.block(block)),
            },
            Some(Container::Quote(blocks)) => self.block(Block::Quote(blocks)),
            Some(Container::List(list)) => self.block(Block::List(list)),
            Some(Container::Table(table, _)) => self.block(Block::Table(table)),
        }
    }

    fn finish(mut self) -> Vec<Block> {
        while !self.containers.is_empty() {
            self.uncontain();
        }
        self.close_leaf();
        self.blocks
    }
}

/// Every heading of `blocks`, in document order.
pub(crate) fn headings(blocks: &[Block]) -> Vec<Heading> {
    let mut found = Vec::new();
    collect_headings(blocks, &mut found);
    found
}

fn collect_headings(blocks: &[Block], found: &mut Vec<Heading>) {
    for block in blocks {
        match block {
            Block::Heading(heading) => found.push(heading.clone()),
            Block::Quote(inner) => collect_headings(inner, found),
            Block::List(list) => list
                .items
                .iter()
                .for_each(|item| collect_headings(&item.blocks, found)),
            Block::Paragraph(_)
            | Block::Code { .. }
            | Block::Image { .. }
            | Block::Rule
            | Block::Table(_)
            | Block::Unsupported { .. } => {}
        }
    }
}

fn column_align(align: Alignment) -> Align {
    match align {
        Alignment::None | Alignment::Left => Align::Start,
        Alignment::Center => Align::Center,
        Alignment::Right => Align::End,
    }
}

/// A GitHub-style anchor: lower case, spaces as hyphens, and only letters,
/// digits, hyphens and underscores kept.
pub(crate) fn slug(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

impl From<HeadingLevel> for Level {
    fn from(level: HeadingLevel) -> Self {
        match level {
            HeadingLevel::H1 => Self::H1,
            HeadingLevel::H2 => Self::H2,
            HeadingLevel::H3 => Self::H3,
            HeadingLevel::H4 => Self::H4,
            HeadingLevel::H5 => Self::H5,
            HeadingLevel::H6 => Self::H6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(spans: Vec<Span>) -> Block {
        Block::Paragraph(Spans::from(spans))
    }

    fn text(text: &str) -> Block {
        paragraph(vec![Span::new(text)])
    }

    fn heading(level: Level, anchor: &str, text: &str) -> Block {
        Block::Heading(Heading {
            level,
            anchor: anchor.to_owned(),
            spans: Spans::from(vec![Span::new(text)]),
        })
    }

    fn item(blocks: Vec<Block>) -> Item {
        Item { task: None, blocks }
    }

    #[test]
    fn inline_styles_become_spans() {
        assert_eq!(
            parse("Plain *em* **strong** ***both*** `code` [link](#to) [`code link`](https://x.y)"),
            vec![paragraph(vec![
                Span::new("Plain "),
                Span::new("em").font_style(FontStyle::Italic),
                Span::new(" "),
                Span::new("strong").weight(Weight::Bold),
                Span::new(" "),
                Span::new("both")
                    .weight(Weight::Bold)
                    .font_style(FontStyle::Italic),
                Span::new(" "),
                Span::new("code").code(),
                Span::new(" "),
                Span::new("link").link("#to"),
                Span::new(" "),
                Span::new("code link").code().link("https://x.y"),
            ])]
        );
    }

    #[test]
    fn breaks_are_a_space_and_a_new_line() {
        assert_eq!(
            parse("soft\nbreak and hard  \nbreak"),
            vec![text("soft break and hard\nbreak")],
            "text of one style is one span"
        );
    }

    #[test]
    fn headings_take_a_level_and_a_unique_anchor() {
        assert_eq!(
            parse(
                "# Getting Started!\n\n## Getting started\n\n### Set up `cargo` {#setup}\n\n###### Deep"
            ),
            vec![
                heading(Level::H1, "getting-started", "Getting Started!"),
                heading(Level::H2, "getting-started-1", "Getting started"),
                Block::Heading(Heading {
                    level: Level::H3,
                    anchor: "setup".to_owned(),
                    spans: Spans::from(vec![Span::new("Set up "), Span::new("cargo").code()]),
                }),
                heading(Level::H6, "deep", "Deep"),
            ]
        );
    }

    #[test]
    fn lists_nest_and_keep_their_numbering_and_tasks() {
        let source = "3. three\n4. four\n   - inner\n   - [x] done\n   - [ ] open\n\n- loose\n\n  second paragraph\n";
        assert_eq!(
            parse(source),
            vec![
                Block::List(List {
                    start: Some(3),
                    items: vec![
                        item(vec![text("three")]),
                        item(vec![
                            text("four"),
                            Block::List(List {
                                start: None,
                                items: vec![
                                    item(vec![text("inner")]),
                                    Item {
                                        task: Some(Task::Done),
                                        blocks: vec![text("done")],
                                    },
                                    Item {
                                        task: Some(Task::Open),
                                        blocks: vec![text("open")],
                                    },
                                ],
                            }),
                        ]),
                    ],
                }),
                Block::List(List {
                    start: None,
                    items: vec![item(vec![text("loose"), text("second paragraph")])],
                }),
            ]
        );
    }

    #[test]
    fn a_quote_holds_blocks() {
        assert_eq!(
            parse("> quoted\n>\n> - listed\n"),
            vec![Block::Quote(vec![
                text("quoted"),
                Block::List(List {
                    start: None,
                    items: vec![item(vec![text("listed")])],
                }),
            ])]
        );
    }

    #[test]
    fn code_blocks_keep_their_info_and_text() {
        assert_eq!(
            parse("```rust ignore\nfn main() {}\n```\n\n    indented\n"),
            vec![
                Block::Code {
                    info: "rust ignore".to_owned(),
                    text: "fn main() {}\n".to_owned(),
                },
                Block::Code {
                    info: String::new(),
                    text: "indented\n".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn an_image_is_a_block_of_its_own() {
        assert_eq!(
            parse("![a *cat*](cat.png)\n\nbefore ![dog](dog.png) after"),
            vec![
                Block::Image {
                    uri: "cat.png".to_owned(),
                    alt: "a cat".to_owned(),
                },
                text("before "),
                Block::Image {
                    uri: "dog.png".to_owned(),
                    alt: "dog".to_owned(),
                },
                text(" after"),
            ]
        );
    }

    #[test]
    fn headings_are_found_inside_quotes_and_lists_in_order() {
        let found = headings(&parse("# One\n\n> ## Two\n\n- ### Three\n"));
        let summary: Vec<(Level, String, String)> = found
            .iter()
            .map(|heading| (heading.level, heading.anchor.clone(), heading.text()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (Level::H1, "one".to_owned(), "One".to_owned()),
                (Level::H2, "two".to_owned(), "Two".to_owned()),
                (Level::H3, "three".to_owned(), "Three".to_owned()),
            ]
        );
    }

    #[test]
    fn a_rule_is_a_block() {
        assert_eq!(
            parse("a\n\n---\n\nb"),
            vec![text("a"), Block::Rule, text("b")]
        );
    }

    #[test]
    fn html_keeps_its_source() {
        let html = "<div>\nraw\n</div>";
        assert_eq!(
            parse(&format!("{html}\n\ninline <b>html</b>")),
            vec![
                Block::Unsupported {
                    construct: Construct::Html,
                    source: html.to_owned(),
                },
                paragraph(vec![
                    Span::new("inline "),
                    Span::new("<b>").code(),
                    Span::new("html"),
                    Span::new("</b>").code(),
                ]),
            ]
        );
    }

    fn cells(texts: &[&str]) -> Vec<Spans> {
        texts.iter().map(|text| Spans::from(*text)).collect()
    }

    #[test]
    fn a_table_keeps_its_alignments_header_and_rows() {
        let source = "| a | b | c | d |\n|---|:--|:-:|--:|\n| 1 | 2 | 3 | 4 |\n| 5 | 6 | 7 | 8 |";
        assert_eq!(
            parse(source),
            vec![Block::Table(Table {
                align: vec![Align::Start, Align::Start, Align::Center, Align::End],
                header: cells(&["a", "b", "c", "d"]),
                rows: vec![cells(&["1", "2", "3", "4"]), cells(&["5", "6", "7", "8"])],
            })]
        );
    }

    #[test]
    fn a_cell_keeps_its_inline_styles_and_links() {
        let source = "| head |\n|---|\n| *em* `code` [to](#a) ![alt](x.png) |";
        let blocks = parse(source);
        let [Block::Table(table)] = blocks.as_slice() else {
            panic!("one table");
        };
        assert_eq!(
            table.rows,
            vec![vec![Spans::from(vec![
                Span::new("em").font_style(FontStyle::Italic),
                Span::new(" "),
                Span::new("code").code(),
                Span::new(" "),
                Span::new("to").link("#a"),
                Span::new(" alt"),
            ])]]
        );
    }

    #[test]
    fn a_ragged_row_takes_the_headers_width() {
        let source = "| a | b | c |\n|---|---|---|\n| 1 |\n| 1 | 2 | 3 | 4 |";
        let blocks = parse(source);
        let [Block::Table(table)] = blocks.as_slice() else {
            panic!("one table");
        };
        assert_eq!(table.header.len(), 3);
        assert_eq!(table.rows.iter().map(Vec::len).collect::<Vec<_>>(), [3, 3]);
    }

    #[test]
    fn cell_events_outside_a_table_do_not_panic() {
        let mut walk = Walk::default();
        for event in [
            Event::Start(Tag::TableCell),
            Event::Text("loose".into()),
            Event::End(TagEnd::TableCell),
            Event::End(TagEnd::TableRow),
            Event::End(TagEnd::TableHead),
            Event::End(TagEnd::Table),
        ] {
            walk.event(event);
        }
        assert_eq!(walk.finish(), [text("loose")]);
    }

    #[test]
    fn markdown_that_is_not_enabled_reads_as_its_source() {
        assert_eq!(
            parse("~~struck~~ and $x$"),
            vec![text("~~struck~~ and $x$")]
        );
    }

    #[test]
    fn unbalanced_events_keep_their_text_and_do_not_panic() {
        let mut walk = Walk::default();
        for event in [
            Event::End(TagEnd::Item),
            Event::End(TagEnd::Emphasis),
            Event::Start(Tag::Item),
            Event::Text("kept".into()),
            Event::End(TagEnd::Item),
            Event::Start(Tag::BlockQuote(None)),
            Event::Text("open".into()),
        ] {
            walk.event(event);
        }
        assert_eq!(
            walk.finish(),
            [
                Block::Paragraph(Span::new("kept").into()),
                Block::Quote(vec![Block::Paragraph(Span::new("open").into())]),
            ]
        );
    }
}
