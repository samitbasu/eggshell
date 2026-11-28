//! Relations between accessibility nodes: which node controls which, which
//! item a listbox's arrows reached, which label names a control.
//!
//! The platform adapters expect every node a relation names to be in the
//! frame's tree, so relations are gathered during the pass and announced
//! once it has drawn, only between nodes that are there. This module is the
//! one place that writes a relation to a node.

use egui::{Context, Id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Relation {
    /// `node` controls `target`: the `aria-controls` of the web.
    Controls { node: Id, target: Id },
    /// `target` is the item of `node`'s listbox the keys reached: the
    /// `aria-activedescendant` of the web.
    ActiveDescendant { node: Id, target: Id },
    /// `target` is the visible label that names `node`, in place of the
    /// name `node` gave itself: the `aria-labelledby` of the web.
    LabelledBy { node: Id, target: Id },
}

impl Relation {
    fn ends(self) -> (Id, Id) {
        match self {
            Self::Controls { node, target }
            | Self::ActiveDescendant { node, target }
            | Self::LabelledBy { node, target } => (node, target),
        }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "the one writer of relations, behind the check that both ends are in the tree"
    )]
    fn write(self, built: &mut egui::accesskit::Node) {
        let target = self.ends().1.accesskit_id();
        match self {
            Self::Controls { .. } => built.push_controlled(target),
            Self::ActiveDescendant { .. } => built.set_active_descendant(target),
            Self::LabelledBy { .. } => {
                built.clear_label();
                built.push_labelled_by(target);
            }
        }
    }
}

/// Runs `f` on the relations noted in the viewport's collection: `None`
/// when no collection is open.
fn with_noted<R>(ctx: &Context, f: impl FnOnce(&mut Option<Vec<Relation>>) -> R) -> R {
    let key = crate::key::viewport_id(ctx, "relations");
    ctx.data_mut(|data| f(data.get_temp_mut_or_default(key)))
}

/// The relations noted while it lives, for [`Collecting::announce`].
/// Collections nest: a layout drawn from a draw closure of another collects
/// its own, and the outer one's resume when it ends.
pub(crate) struct Collecting {
    ctx: Context,
    outer: Option<Vec<Relation>>,
}

impl Collecting {
    pub(crate) fn enter(ctx: &Context) -> Self {
        let outer = with_noted(ctx, |noted| noted.replace(Vec::new()));
        Self {
            ctx: ctx.clone(),
            outer,
        }
    }

    /// Announces the relations noted since the collection began, and `more`,
    /// each whose two nodes are in this pass's accessibility tree, and drops
    /// the others.
    pub(crate) fn announce(self, more: impl IntoIterator<Item = Relation>) {
        let ctx = &self.ctx;
        let noted = with_noted(ctx, |noted| noted.take().unwrap_or_default());
        for relation in noted.into_iter().chain(more) {
            let (node, target) = relation.ends();
            if ctx.has_accesskit_node(node) && ctx.has_accesskit_node(target) {
                ctx.accesskit_node_builder(node, |built| relation.write(built));
            }
        }
    }
}

impl Drop for Collecting {
    fn drop(&mut self) {
        let outer = self.outer.take();
        with_noted(&self.ctx, |noted| *noted = outer);
    }
}

/// Notes `relation` for the collection the pass is drawing in, to announce
/// once the pass has drawn. Outside of one it is dropped.
pub(crate) fn note(ctx: &Context, relation: Relation) {
    with_noted(ctx, |noted| {
        if let Some(noted) = noted {
            noted.push(relation);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{NodeId, TreeUpdate};
    use egui::{Rect, Sense, pos2, vec2};

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    /// The tree of a pass that notes `relations` and then builds a widget,
    /// named after it, for each of `built`.
    fn tree(built: &[&str], relations: &[Relation]) -> TreeUpdate {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let collecting = Collecting::enter(ui.ctx());
            for &relation in relations {
                note(ui.ctx(), relation);
            }
            for (at, name) in built.iter().enumerate() {
                let rect = Rect::from_min_size(pos2(0.0, at as f32 * 20.0), vec2(20.0, 20.0));
                ui.interact(rect, id(name), Sense::click())
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, *name));
            }
            collecting.announce([]);
        });
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

    /// Each kind of relation, from "field" to "target".
    fn every_kind() -> [Relation; 3] {
        let (node, target) = (id("field"), id("target"));
        [
            Relation::Controls { node, target },
            Relation::ActiveDescendant { node, target },
            Relation::LabelledBy { node, target },
        ]
    }

    /// The ids `relation`'s kind names on `field`.
    fn named(field: &egui::accesskit::Node, relation: Relation) -> Vec<NodeId> {
        match relation {
            Relation::Controls { .. } => field.controls().to_vec(),
            Relation::ActiveDescendant { .. } => field.active_descendant().into_iter().collect(),
            Relation::LabelledBy { .. } => field.labelled_by().to_vec(),
        }
    }

    /// Every id any node of `tree` names in a relation.
    fn every_named(tree: &TreeUpdate) -> Vec<NodeId> {
        tree.nodes
            .iter()
            .flat_map(|(_, node)| {
                every_kind()
                    .into_iter()
                    .flat_map(move |relation| named(node, relation))
            })
            .collect()
    }

    #[test]
    fn each_relation_between_built_nodes_is_announced() {
        for relation in every_kind() {
            let tree = tree(&["field", "target"], &[relation]);
            let field = node(&tree, "field").expect("precondition: the field is in the tree");
            assert!(
                node(&tree, "target").is_some(),
                "precondition: {relation:?}"
            );
            assert_eq!(named(&field, relation), [id("target").accesskit_id()]);
        }
    }

    #[test]
    fn each_relation_to_a_node_not_built_is_dropped() {
        for relation in every_kind() {
            let tree = tree(&["field"], &[relation]);
            let field = node(&tree, "field").expect("precondition: the field is in the tree");
            assert!(
                node(&tree, "target").is_none(),
                "precondition: {relation:?}"
            );
            assert!(named(&field, relation).is_empty(), "{relation:?}");
            assert!(every_named(&tree).is_empty(), "{relation:?}");
        }
    }

    #[test]
    fn each_relation_from_a_node_not_built_adds_no_node() {
        for relation in every_kind() {
            let tree = tree(&["target"], &[relation]);
            assert!(
                node(&tree, "target").is_some(),
                "precondition: {relation:?}"
            );
            assert!(node(&tree, "field").is_none(), "{relation:?}");
            assert!(every_named(&tree).is_empty(), "{relation:?}");
        }
    }

    #[test]
    fn a_label_replaces_the_name_the_control_gave_itself() {
        let unlabelled = tree(&["field", "target"], &[]);
        let field = node(&unlabelled, "field").expect("precondition: the field is in the tree");
        assert_eq!(
            field.label(),
            Some("field"),
            "precondition: the field names itself"
        );
        let tree = tree(
            &["field", "target"],
            &[Relation::LabelledBy {
                node: id("field"),
                target: id("target"),
            }],
        );
        let field = node(&tree, "field").expect("precondition: the field is in the tree");
        assert_eq!(field.label(), None);
    }

    #[test]
    fn a_nested_collection_keeps_the_outer_ones_relations() {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let outer = Collecting::enter(ui.ctx());
            note(
                ui.ctx(),
                Relation::Controls {
                    node: id("field"),
                    target: id("target"),
                },
            );
            Collecting::enter(ui.ctx()).announce([]);
            for (at, name) in ["field", "target"].into_iter().enumerate() {
                let rect = Rect::from_min_size(pos2(0.0, at as f32 * 20.0), vec2(20.0, 20.0));
                ui.interact(rect, id(name), Sense::click());
            }
            outer.announce([]);
        });
        output.textures_delta.clear();
        let tree = output
            .platform_output
            .accesskit_update
            .expect("accesskit is on");
        let field = node(&tree, "field").expect("precondition: the field is in the tree");
        assert_eq!(field.controls(), [id("target").accesskit_id()]);
    }

    #[test]
    fn a_relation_noted_outside_a_collection_is_dropped() {
        let ctx = Context::default();
        let relation = Relation::Controls {
            node: id("field"),
            target: id("target"),
        };
        note(&ctx, relation);
        assert_eq!(with_noted(&ctx, |noted| noted.clone()), None);
    }
}
