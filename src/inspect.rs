//! Why a node has the values it has (debug builds): the node pinned with the
//! debug overlay, its Computed-style table, and the console dump of it.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

use egui::{Color32, Context, FontId, Id, Painter, Rect, Shape, vec2};

use crate::nodes::Nodes;
use crate::pipeline::PlacedNode;

fn pin_id() -> Id {
    Id::NULL.with("eggshell-inspect-pin")
}

fn table_id() -> Id {
    Id::NULL.with("eggshell-inspect-table")
}

fn dumped_id() -> Id {
    Id::NULL.with("eggshell-inspect-dumped")
}

/// The readable path of the pinned node, if one is pinned.
pub(crate) fn pinned(ctx: &Context) -> Option<String> {
    ctx.data(|data| data.get_temp::<Option<String>>(pin_id()))
        .flatten()
}

pub(crate) fn pin(ctx: &Context, path: Option<String>) {
    ctx.data_mut(|data| data.insert_temp(pin_id(), path));
}

/// The path suffix `EGGSHELL_WHY` names, if it is set.
fn watched() -> Option<&'static str> {
    static WATCHED: OnceLock<Option<String>> = OnceLock::new();
    WATCHED
        .get_or_init(|| std::env::var("EGGSHELL_WHY").ok().filter(|v| !v.is_empty()))
        .as_deref()
}

/// Whether the classes built now keep every class that set a property.
pub(crate) fn wants_trace(ctx: &Context) -> bool {
    pinned(ctx).is_some() || watched().is_some()
}

/// The Computed-style table of `node`: each property a class set, its
/// winner, and each class it beat.
pub(crate) fn table(node: &PlacedNode) -> String {
    let mut out = format!("{}  ({:?})\n", node.path(), node.kind());
    let why = node.why_all();
    if why.is_empty() {
        out.push_str("no class set a property");
    }
    for (property, why) in why {
        let value = |step: &crate::Step| {
            if step.value.is_empty() {
                "-".to_owned()
            } else {
                step.value.clone()
            }
        };
        let name = format!("{property:?}");
        let _ = writeln!(
            out,
            "{name:<22} {}  {}",
            value(&why.winner),
            why.winner.describe()
        );
        for beaten in why.beaten.iter().rev() {
            let _ = writeln!(
                out,
                "{:<22} {}  {}  (overridden)",
                "",
                value(beaten),
                beaten.describe()
            );
        }
    }
    out.truncate(out.trim_end().len());
    out
}

/// The pane that shows `table`, in the corner of `screen`.
pub(crate) fn pane(painter: &Painter, screen: Rect, table: String) -> Shape {
    let galley = painter.layout_no_wrap(table, FontId::monospace(11.0), Color32::WHITE);
    let margin = vec2(4.0, 2.0);
    let size = galley.size() + margin * 2.0;
    let origin = screen.min + vec2(8.0, 8.0);
    Shape::Vec(vec![
        Shape::rect_filled(
            Rect::from_min_size(origin, size),
            2.0,
            Color32::from_black_alpha(220),
        ),
        Shape::galley(origin + margin, galley, Color32::WHITE),
    ])
}

/// Keeps the pinned node's table for [`print_pinned`].
pub(crate) fn remember(ctx: &Context, table: String) {
    ctx.data_mut(|data| data.insert_temp(table_id(), table));
}

/// Prints the table of the pinned node to stderr.
pub(crate) fn print_pinned(ctx: &Context) {
    if let Some(table) = ctx.data(|data| data.get_temp::<String>(table_id())) {
        eprintln!("{table}");
    }
}

/// Prints the table of each node whose path ends with `EGGSHELL_WHY`
/// whenever it changes.
pub(crate) fn dump_why(ctx: &Context, nodes: &Nodes) {
    let Some(suffix) = watched() else {
        return;
    };
    let mut seen: HashMap<String, String> = ctx
        .data(|data| data.get_temp(dumped_id()))
        .unwrap_or_default();
    let mut changed = false;
    for node in nodes.iter().filter(|node| node.path().ends_with(suffix)) {
        let table = table(node);
        if seen.get(node.path()) != Some(&table) {
            eprintln!("{table}");
            seen.insert(node.path().to_owned(), table);
            changed = true;
        }
    }
    if changed {
        ctx.data_mut(|data| data.insert_temp(dumped_id(), seen));
    }
}
