//! Parsed blocks into eggshell nodes.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use crate::owned::table::{Cells, Column};
use crate::{Builder, Key, Scope, Shell, Spans, Track, Tw, child_id, tw};
use egui::load::{SizeHint, TexturePoll};
use egui::{Align, Id, ImageSource, TextureOptions};

use super::parse::{Block, Construct, Item, List, Table, Task};
use super::{Directive, Directives, OnLink, Style};

pub(super) struct Context<'s, 'f, A> {
    pub style: Arc<Style>,
    pub links: Rc<Links<A>>,
    pub directives: Option<(&'s Directives<A>, &'f A)>,
    pub ctx: egui::Context,
}

/// Where a document's links go: an anchor naming one of its headings
/// scrolls to that heading, as in a browser, and any other link goes to
/// `on_link`.
pub(super) struct Links<A> {
    on_link: Option<OnLink<A>>,
    shell: Shell,
    doc: Id,
    /// Filled as the document is built; read when a link is followed, which
    /// is after, so a link may name a heading below it.
    anchors: RefCell<HashSet<String>>,
}

impl<A> Links<A> {
    pub fn new(on_link: Option<OnLink<A>>, shell: Shell, doc: Id) -> Self {
        Self {
            on_link,
            shell,
            doc,
            anchors: RefCell::default(),
        }
    }

    fn heading(&self, anchor: &str) {
        self.anchors.borrow_mut().insert(anchor.to_owned());
    }

    fn follow(&self, app: &mut A, href: &str) {
        let heading = href
            .strip_prefix('#')
            .filter(|anchor| self.anchors.borrow().contains(*anchor));
        match (heading, &self.on_link) {
            (Some(anchor), _) => self
                .shell
                .scroll_into_view(heading_id(self.doc, anchor), Some(Align::Min)),
            (None, Some(on_link)) => on_link(app, href),
            (None, None) => {}
        }
    }
}

/// The id of the heading with `anchor` in the document `doc`: headings are
/// keyed under the document, however deep they sit, so that this needs no
/// layout.
pub(super) fn heading_id(doc: Id, anchor: &str) -> Id {
    child_id(doc, &Part::Heading(anchor.to_owned()))
}

/// The keys of a document's nodes. A heading is keyed by its anchor, so that
/// its id survives edits above it.
#[derive(Clone, Debug, Hash)]
enum Part {
    Block(usize),
    Heading(String),
    Item(usize),
    Marker,
    Content,
    Source,
    Table,
    Cell,
}

#[track_caller]
pub(super) fn document<'f, A: 'static, K: Key>(
    b: &mut Builder<'_, 'f, A>,
    key: K,
    blocks: &[Block],
    context: &Context<'_, 'f, A>,
) {
    let body = |b: &mut Builder<'_, 'f, A>| {
        b.div_id(key, tw().flex().flex_col(), |s| {
            emit(s, blocks, context, Depth::default());
        });
    };
    let style = &context.style;
    if style.restyles() {
        b.styled(|page| style.restyle(page), body);
    } else {
        body(b);
    }
}

/// How many lists the blocks are inside.
#[derive(Clone, Copy, Default)]
struct Depth(usize);

impl Depth {
    /// The space between blocks: a list item's blocks sit as close as its
    /// items do.
    fn gap(self, style: &Style) -> f32 {
        match self.0 {
            0 => style.block_gap,
            _ => style.list.item_gap,
        }
    }
}

fn emit<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    blocks: &[Block],
    context: &Context<'_, 'f, A>,
    depth: Depth,
) {
    let style = &context.style;
    let mut previous = None;
    for (index, block) in blocks.iter().enumerate() {
        let top = match previous {
            None => 0.0,
            Some(previous) => space_above(style, depth, previous, block),
        };
        previous = Some(block);
        let at = tw().mt(top);
        let key = Part::Block(index);
        match block {
            Block::Paragraph(spans) => paragraph(s, key, prose(at, style), spans, context),
            Block::Heading(heading) => {
                let look = style.heading(heading.level);
                let at = at.role(look.text.clone()).tracking(look.tracking);
                let at = match look.color.or(style.color) {
                    Some(color) => at.text(color),
                    None => at,
                };
                let at = match look.case {
                    super::Case::AsWritten => at,
                    super::Case::Upper => at.uppercase(),
                };
                let at = match look.rule {
                    super::HeadingRule::None => at,
                    super::HeadingRule::Below { padding } => {
                        at.pb(padding).border_b(1.0).border_color(style.rule)
                    }
                };
                context.links.heading(&heading.anchor);
                s.keyed_under(context.links.doc, |s| {
                    let key = Part::Heading(heading.anchor.clone());
                    paragraph(s, key, at, &heading.spans, context);
                });
            }
            Block::List(list_) => list(s, key, at, list_, context, depth),
            Block::Quote(inner) => {
                let quote = &style.quote;
                let at = at
                    .flex()
                    .flex_col()
                    .border_l(quote.border_width)
                    .border_color(quote.border)
                    .pl(quote.padding)
                    .text(quote.text);
                let at = match quote.font_style {
                    crate::FontStyle::Italic => at.italic(),
                    crate::FontStyle::Normal => at,
                };
                s.div_id(key, at, |s| emit(s, inner, context, depth));
            }
            Block::Code { info, text } => code(s, key, at, info, text, context),
            Block::Image { uri, alt } => image(s, key, at, uri, alt, context),
            Block::Rule => s.div(at.h_px(1.0).bg(style.rule), |_| {}),
            Block::Table(table_) => table(s, key, at, table_, context),
            Block::Unsupported { construct, source } => {
                let name = match construct {
                    Construct::Html => "HTML",
                };
                unsupported(s, key, at, &format!("Not shown: {name}"), source, context);
            }
        }
    }
}

/// The space above `block`, which follows `previous`: typeset.css's
/// cascade, in which a heading owns the space below it and a list item's
/// blocks sit as close as its items.
fn space_above(style: &Style, depth: Depth, previous: &Block, block: &Block) -> f32 {
    match (previous, block) {
        (Block::Heading(_), Block::Heading(heading)) => style.heading(heading.level).after_heading,
        (Block::Rule, Block::Heading(heading)) => style.heading(heading.level).after_rule,
        (_, Block::Heading(heading)) => style.heading(heading.level).margin_top,
        (_, Block::Rule) => style.rule_gap,
        _ if depth.0 > 0 => depth.gap(style),
        (Block::Heading(_), Block::Paragraph(_)) => style.after_heading,
        _ => style.block_gap,
    }
}

/// A table cell's text: the document's tables', else its body text.
fn cell_text(style: &Style) -> Tw {
    let at = prose(tw().whitespace_normal(), style);
    match &style.table {
        Some(table) => at.role(table.text.clone()).tabular_nums(),
        None => at,
    }
}

/// `at` in the document's body text, which only prose takes: a directive's
/// output keeps the page's.
fn prose(at: Tw, style: &Style) -> Tw {
    let at = match &style.body {
        Some(body) => at.role(body.clone()),
        None => at,
    };
    match style.color {
        Some(color) => at.text(color),
        None => at,
    }
}

fn paragraph<'f, A: 'static>(
    s: &mut Builder<'_, 'f, A>,
    key: impl Key,
    at: Tw,
    spans: &Spans,
    context: &Context<'_, 'f, A>,
) {
    let node = s.spans(key, at, spans.clone());
    if spans.iter().any(|span| span.href().is_some()) {
        let links = context.links.clone();
        node.on_link(move |app, href| links.follow(app, href));
    }
}

/// A GFM table through `b.table`: a content-sized column per alignment, and
/// every row padded or cut to that many cells, whose text wraps.
fn table<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    key: Part,
    at: Tw,
    table: &Table,
    context: &Context<'_, 'f, A>,
) {
    let width = table.align.len();
    let columns = table
        .align
        .iter()
        .map(|align| Column::new(Track::auto()).align(*align));
    let row = |cells: &mut Cells<'_, 'f, A>, row: &[Spans]| {
        for column in 0..width {
            let spans = row.get(column);
            cells.cell(|b| {
                if let Some(spans) = spans {
                    paragraph(b, Part::Cell, cell_text(&context.style), spans, context);
                }
            });
        }
    };
    let rows: Vec<(usize, &Vec<Spans>)> = table.rows.iter().enumerate().collect();
    s.div_id(key, at, |s| {
        s.table(Part::Table)
            .columns(columns)
            .header(|cells| row(cells, &table.header))
            .rows(
                &rows,
                |(index, _)| *index,
                |cells, (_, cells_of)| row(cells, cells_of),
            )
            .add();
    });
}

fn list<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    key: Part,
    at: Tw,
    list: &List,
    context: &Context<'_, 'f, A>,
    depth: Depth,
) {
    let look = &context.style.list;
    s.div_id(key, at.flex().flex_col(), |s| {
        for (index, item) in list.items.iter().enumerate() {
            let top = if index == 0 { 0.0 } else { look.item_gap };
            let row = tw()
                .flex()
                .flex_row()
                .items_start()
                .gap(look.marker_gap)
                .mt(top);
            s.div_id(Part::Item(index), row, |s| {
                let column = prose(tw(), &context.style)
                    .w(look.indent)
                    .shrink_0()
                    .text_right()
                    .text(look.marker);
                match marker(list, index, item, depth) {
                    Marker::Number(number) => {
                        s.span(Part::Marker, column, format!("{number}."));
                    }
                    Marker::Shape(shape) => {
                        s.span(Part::Marker, column, " ")
                            .draw(move |_, ui| shape.paint(ui));
                    }
                }
                let content = tw().flex().flex_col().grow().min_w_0();
                s.div_id(Part::Content, content, |s| {
                    emit(s, &item.blocks, context, Depth(depth.0 + 1));
                });
            });
        }
    });
}

/// A browser's markers: numbers for an ordered list, a box for a task, and
/// a disc, circle or square by how deep the list is.
fn marker(list: &List, index: usize, item: &Item, depth: Depth) -> Marker {
    match (item.task, list.start) {
        (Some(task), _) => Marker::Shape(Shape::Task(task)),
        (None, Some(start)) => Marker::Number(start.saturating_add(index as u64)),
        (None, None) => Marker::Shape(match depth.0 % 3 {
            0 => Shape::Disc,
            1 => Shape::Circle,
            _ => Shape::Square,
        }),
    }
}

enum Marker {
    Number(u64),
    Shape(Shape),
}

/// A marker painted, not written, as fonts seldom have every one.
#[derive(Clone, Copy)]
enum Shape {
    Disc,
    Circle,
    Square,
    Task(Task),
}

impl Shape {
    /// Paints the marker at the right of the line `ui`'s rect is.
    fn paint(self, ui: &egui::Ui) {
        let line = ui.max_rect();
        let color = ui.visuals().text_color();
        let side = match self {
            Self::Task(_) => line.height() * 0.6,
            Self::Disc | Self::Circle | Self::Square => line.height() * 0.3,
        };
        let at = egui::Rect::from_center_size(
            egui::pos2(line.right() - side / 2.0, line.center().y),
            egui::vec2(side, side),
        );
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.0, color);
        match self {
            Self::Disc => {
                painter.circle_filled(at.center(), side / 2.0, color);
            }
            Self::Circle => {
                painter.circle_stroke(at.center(), side / 2.0 - 0.5, stroke);
            }
            Self::Square => {
                painter.rect_filled(at, 0.0, color);
            }
            Self::Task(Task::Open) => {
                painter.rect_stroke(at, 2.0, stroke, egui::StrokeKind::Inside);
            }
            Self::Task(Task::Done) => {
                painter.rect_filled(at, 2.0, color);
                let check = [(0.22, 0.52), (0.42, 0.72), (0.78, 0.3)]
                    .map(|(x, y)| at.lerp_inside(egui::vec2(x, y)));
                let back = ui.visuals().panel_fill;
                painter.line(check.to_vec(), egui::Stroke::new(side * 0.14, back));
            }
        }
    }
}

fn code<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    key: Part,
    at: Tw,
    info: &str,
    text: &str,
    context: &Context<'_, 'f, A>,
) {
    let (name, arg) = info.split_once(' ').unwrap_or((info, ""));
    if let Some((mount, app)) = context
        .directives
        .and_then(|(all, app)| all.mount(name).map(|mount| (mount, app)))
    {
        let directive = Directive {
            name,
            arg: arg.trim(),
            body: text,
        };
        s.div_id(key, at, |s| mount(s, app, &directive));
        return;
    }
    let look = &context.style.code;
    let block = at
        .bg(look.fill)
        .border(look.border_width)
        .border_color(look.border)
        .rounded_px(look.radius)
        .px(look.padding_x)
        .py(look.padding_y);
    s.div_id(key, block, |s| {
        s.span(
            Part::Source,
            tw().role(look.text.clone()),
            text.trim_end_matches('\n'),
        );
    });
}

fn image<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    key: Part,
    at: Tw,
    uri: &str,
    alt: &str,
    context: &Context<'_, 'f, A>,
) {
    match context
        .ctx
        .try_load_texture(uri, TextureOptions::default(), SizeHint::default())
    {
        Ok(TexturePoll::Ready { texture }) => {
            let aspect = texture.size.x / texture.size.y.max(1.0);
            s.div_id(key, at, |s| {
                s.image(
                    tw().w_px(texture.size.x).max_w_full().aspect(aspect),
                    ImageSource::Texture(texture),
                );
            });
        }
        Ok(TexturePoll::Pending { .. }) => {
            let pending = tw().text(context.style.unsupported.label);
            s.div_id(key, at, |s| {
                s.span(Part::Source, pending, alt.to_owned());
            });
        }
        Err(error) => {
            let label = format!("Image not shown: {alt}");
            unsupported(s, key, at, &label, &format!("{uri}: {error}"), context);
        }
    }
}

fn unsupported<'f, A: 'static>(
    s: &mut Scope<'_, 'f, A>,
    key: Part,
    at: Tw,
    label: &str,
    source: &str,
    context: &Context<'_, 'f, A>,
) {
    let look = &context.style.unsupported;
    let block = at
        .flex()
        .flex_col()
        .gap(1.0)
        .p(3.0)
        .bg(look.fill)
        .border(1.0)
        .border_color(look.border)
        .rounded_px(context.style.code.radius);
    s.div_id(key, block, |s| {
        s.text(tw().text(look.label), label);
        s.span(Part::Source, tw().role(look.text.clone()), source);
    });
}
