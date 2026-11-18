//! Relations between accessibility nodes: which node controls which, and
//! which item a listbox's arrows reached.
//!
//! The platform adapters expect every node a relation names to be in the
//! frame's tree, so relations are gathered during the pass and announced
//! once it has drawn, only between nodes that are there.

use egui::{Context, Id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Relation {
    /// `node` controls `target`: the `aria-controls` of the web.
    Controls { node: Id, target: Id },
    /// `target` is the item of `node`'s listbox the keys reached: the
    /// `aria-activedescendant` of the web.
    ActiveDescendant { node: Id, target: Id },
}

impl Relation {
    fn ends(self) -> (Id, Id) {
        match self {
            Self::Controls { node, target } | Self::ActiveDescendant { node, target } => {
                (node, target)
            }
        }
    }
}

/// Announces each of `relations` whose two nodes are in this pass's
/// accessibility tree, and drops the others.
pub(crate) fn announce(ctx: &Context, relations: &[Relation]) {
    for &relation in relations {
        let (node, target) = relation.ends();
        if !(ctx.has_accesskit_node(node) && ctx.has_accesskit_node(target)) {
            continue;
        }
        ctx.accesskit_node_builder(node, |built| match relation {
            Relation::Controls { .. } => built.push_controlled(target.accesskit_id()),
            Relation::ActiveDescendant { .. } => {
                built.set_active_descendant(target.accesskit_id());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::TreeUpdate;
    use egui::{Rect, Sense, pos2, vec2};

    fn id(name: &str) -> Id {
        Id::new(name)
    }

    /// The tree of a pass that builds a widget for each of `built` and then
    /// announces `relations`.
    fn tree(built: &[&str], relations: &[Relation]) -> TreeUpdate {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let output = ctx.run_ui(Default::default(), |ui| {
            for (at, name) in built.iter().enumerate() {
                let rect = Rect::from_min_size(pos2(0.0, at as f32 * 20.0), vec2(20.0, 20.0));
                ui.interact(rect, id(name), Sense::click());
            }
            announce(ui.ctx(), relations);
        });
        let mut output = output;
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .expect("accesskit is on")
    }

    fn node(tree: &TreeUpdate, name: &str) -> Option<egui::accesskit::Node> {
        let wanted = id(name).accesskit_id();
        tree.nodes
            .iter()
            .find(|(at, _)| *at == wanted)
            .map(|(_, node)| node.clone())
    }

    #[test]
    fn a_relation_between_built_nodes_is_announced() {
        let relations = [
            Relation::Controls {
                node: id("field"),
                target: id("list"),
            },
            Relation::ActiveDescendant {
                node: id("field"),
                target: id("row"),
            },
        ];
        let tree = tree(&["field", "list", "row"], &relations);
        let field = node(&tree, "field").expect("precondition: the field is in the tree");
        assert_eq!(field.controls(), [id("list").accesskit_id()]);
        assert_eq!(field.active_descendant(), Some(id("row").accesskit_id()));
    }

    #[test]
    fn a_relation_to_a_node_not_built_is_dropped() {
        let relations = [
            Relation::Controls {
                node: id("field"),
                target: id("list"),
            },
            Relation::ActiveDescendant {
                node: id("field"),
                target: id("row"),
            },
        ];
        let tree = tree(&["field"], &relations);
        let field = node(&tree, "field").expect("precondition: the field is in the tree");
        assert!(node(&tree, "row").is_none() && node(&tree, "list").is_none());
        assert!(field.controls().is_empty());
        assert_eq!(field.active_descendant(), None);
    }

    #[test]
    fn a_relation_from_a_node_not_built_adds_no_node() {
        let relations = [Relation::ActiveDescendant {
            node: id("field"),
            target: id("row"),
        }];
        let tree = tree(&["row"], &relations);
        assert!(node(&tree, "field").is_none());
    }
}
