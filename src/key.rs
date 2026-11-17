//! Key paths: the identity of every node.
//!
//! A path is a sequence of [`Key`]s. The egui [`Id`] of a node is the running hash of its path, and the readable
//! form (each key's `Debug` text) is kept for diagnostics.
//!
//! Each segment hashes the key together with its type name, so keys need only
//! be unique among siblings of the same type: variants of different enums, or
//! `1u32` and a `BlockKey(1)` newtype, never collide. The type name (unlike
//! `TypeId`) is stable across runs, so state egui persists keeps its identity.

use std::fmt::{Debug, Write as _};
use std::hash::Hash;

use egui::Id;

/// What a node is named by: any `Hash + Debug + 'static` value. It is
/// implemented for every such type, so there is nothing to implement.
///
/// - **`Hash`** gives the node's egui [`Id`]. The key's type name is mixed in,
///   so keys need only differ from siblings *of the same type*: a bare enum
///   variant, `1u32` and a `BlockKey(1)` newtype never collide.
/// - **`Debug`** gives the readable path used in diagnostics, such as the
///   duplicate-key report and the accessibility tree.
/// - **`'static`** makes keys owned values, so a key outlives the call that
///   made it.
///
/// Neither `Eq` nor `Clone` is needed.
///
/// # Conventions
///
/// - Give each component **its own key enum**. Its parts then cannot collide
///   with a sibling's, whatever the variants are called.
/// - Name a component's instances with a key at the call site
///   ([`Builder::scope`](crate::Builder::scope)); two uses of one component in
///   a parent differ by that key.
/// - Rows of a dynamic list use **data keys** (a record id, never an index),
///   so a row keeps its state through a reorder.
///
/// ```
/// # use eggshell_ui::Key;
/// #[derive(Hash, Debug)]
/// enum Toolbar {
///     Save,
///     Row(u32),
/// }
/// fn needs_key(_: impl Key) {}
/// needs_key(Toolbar::Row(3));
/// needs_key(("pin", 4u64));
/// ```
pub trait Key: Hash + Debug + 'static {}

impl<T: Hash + Debug + 'static> Key for T {}

/// The root of every key path; a salt keeps eggshell ids apart from other
/// users of [`Id::NULL`].
pub(crate) fn root_id() -> Id {
    Id::NULL.with("eggshell")
}

/// The id of the state `name` of the viewport `ctx` is running a pass of:
/// each viewport has its own input.
pub(crate) fn viewport_id(ctx: &egui::Context, name: impl egui::AsIdSalt) -> Id {
    root_id().with((name, ctx.viewport_id()))
}

/// The root of a layout's key paths: [`root_id`], salted by the layout's name
/// if it has one. The salt is a hash of the name, so ids (and the state egui
/// keeps under them) are stable across runs.
pub(crate) fn layout_root(name: Option<&'static str>) -> Id {
    name.map_or_else(root_id, |name| root_id().with(("named-layout", name)))
}

/// What one `LayoutState` claimed in a pass: the debug-build cross-layout check.
///
/// This lives in the egui context, not in a `LayoutState`, because it exists
/// to see across the layouts of one context; like the other diagnostics it is
/// context-wide by nature.
#[cfg(debug_assertions)]
pub(crate) mod claims {
    use std::collections::HashMap;
    use std::panic::Location;
    use std::sync::{Arc, Weak};

    use egui::Id;

    /// A claim outlives its layout in the context's data; `alive` tells a
    /// layout that is gone (a throwaway built once and dropped) from one
    /// still in use, which alone can collide.
    #[derive(Clone)]
    struct Claim {
        scope: Id,
        alive: Weak<()>,
        at: &'static Location<'static>,
    }

    impl Claim {
        fn new(layout: &Layout, at: &'static Location<'static>) -> Self {
            Self {
                scope: layout.scope,
                alive: Arc::downgrade(&layout.alive),
                at,
            }
        }

        fn clashes_with(&self, layout: &Layout) -> bool {
            self.scope != layout.scope && self.alive.strong_count() > 0
        }
    }

    /// Who is claiming.
    pub(crate) struct Layout {
        pub(crate) scope: Id,
        pub(crate) alive: Arc<()>,
    }

    #[derive(Default, Clone)]
    struct Claims {
        pass: u64,
        names: HashMap<Id, Claim>,
        tops: HashMap<Id, Claim>,
    }

    fn with<R>(ctx: &egui::Context, f: impl FnOnce(&mut Claims) -> R) -> R {
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|data| {
            let claims = data.get_temp_mut_or_default::<Claims>(Id::NULL.with("eggshell-claims"));
            if claims.pass != pass {
                *claims = Claims {
                    pass,
                    ..Claims::default()
                };
            }
            f(claims)
        })
    }

    /// Claims the name of a named layout for one pass, reporting a
    /// [`LayoutNameClash`](crate::Problem::LayoutNameClash) if another
    /// layout of the context claimed it this pass.
    pub(crate) fn name(
        ctx: &egui::Context,
        root: Id,
        layout: &Layout,
        at: &'static Location<'static>,
    ) {
        let first = with(ctx, |claims| {
            let first = claims
                .names
                .entry(root)
                .or_insert_with(|| Claim::new(layout, at));
            if first.clashes_with(layout) {
                Some(first.clone())
            } else {
                *first = Claim::new(layout, at);
                None
            }
        });
        if let Some(first) = first {
            crate::report::report_at(ctx, crate::Problem::LayoutNameClash, Some(root), at, || {
                format!(
                    "two layouts in one egui context are named the same (built at {} and \
                         {at}); a name salts one layout's ids, so it must be unique",
                    first.at
                )
            });
        }
    }

    /// Claims a top-level key (`id` is under `root`) for one pass, reporting
    /// a [`TopLevelKeyClash`](crate::Problem::TopLevelKeyClash) if another
    /// layout with the same root claimed it this pass.
    pub(crate) fn top_level(
        ctx: &egui::Context,
        id: Id,
        readable: &str,
        layout: &Layout,
        at: &'static Location<'static>,
    ) {
        let first = with(ctx, |claims| {
            let first = claims
                .tops
                .entry(id)
                .or_insert_with(|| Claim::new(layout, at));
            if first.clashes_with(layout) {
                Some(first.clone())
            } else {
                *first = Claim::new(layout, at);
                None
            }
        });
        if let Some(first) = first {
            crate::report::report_at(ctx, crate::Problem::TopLevelKeyClash, Some(id), at, || {
                format!(
                    "top-level key {readable} is used by two layouts in one egui context \
                         (built at {} and {at}); their ids and egui's state under them collide. \
                         Give one `LayoutState::named(..)`",
                    first.at
                )
            });
        }
    }
}

/// The readable form of the path being built, as a stack of segments.
#[derive(Default)]
pub(crate) struct ReadablePath(String);

impl ReadablePath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Pushes a key, returning the mark to [`Self::pop_to`] afterwards.
    pub(crate) fn push(&mut self, key: &dyn Debug) -> usize {
        let mark = self.0.len();
        write!(self.0, "/{key:?}").expect("writing to a String cannot fail");
        mark
    }

    /// Pushes a segment that is not a key, such as the kind of an anonymous node.
    pub(crate) fn push_label(&mut self, label: &str) -> usize {
        let mark = self.0.len();
        write!(self.0, "/<{label}>").expect("writing to a String cannot fail");
        mark
    }

    pub(crate) fn pop_to(&mut self, mark: usize) {
        self.0.truncate(mark);
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    /// Makes `path`, a readable path taken earlier, the one being built.
    pub(crate) fn reset_to(&mut self, path: &str) {
        self.0.clear();
        self.0.push_str(path);
    }
}

/// TODO(prose)
///
/// ```
/// # use eggshell_ui::{Builder, child_id, tw};
/// fn list(b: &mut Builder<'_, '_, ()>) {
///     let list = b.id_of("list");
///     b.div_id("list", tw(), |s| {
///         assert_eq!(s.id_of("row"), child_id(list, &"row"));
///     });
/// }
/// ```
pub fn child_id<K: Key>(parent: Id, key: &K) -> Id {
    parent.with((std::any::type_name::<K>(), key))
}

/// Debug-build registry of the identified paths of a frame, which gives a
/// repeated key an id of its own so egui's state under the two cannot mix.
#[derive(Default)]
pub(crate) struct Seen {
    #[cfg(debug_assertions)]
    paths: std::collections::HashMap<Id, String>,
}

/// How a registered path repeats an earlier one.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(not(debug_assertions), expect(dead_code))]
pub(crate) enum Repeat {
    /// The same readable path.
    Duplicate,
    /// A different readable path, `first`, with the same id.
    Collision { first: String },
}

impl Repeat {
    pub(crate) fn problem(&self) -> crate::Problem {
        match self {
            Self::Duplicate => crate::Problem::DuplicateKey,
            Self::Collision { .. } => crate::Problem::KeyCollision,
        }
    }
}

impl Seen {
    pub(crate) fn clear(&mut self) {
        #[cfg(debug_assertions)]
        self.paths.clear();
    }

    /// The id a node keyed to `declared` gets: `declared` itself, unless a
    /// path registered this frame has it, then the first of its occurrence
    /// salts that none has.
    pub(crate) fn vacant(&self, declared: Id) -> Id {
        #[cfg(debug_assertions)]
        if self.paths.contains_key(&declared) {
            return (2..=u32::MAX)
                .map(|occurrence| declared.with(("eggshell-occurrence", occurrence)))
                .find(|id| !self.paths.contains_key(id))
                .unwrap_or(declared);
        }
        declared
    }

    /// Registers `readable` under `id`, which [`Self::vacant`] gave for
    /// `declared`, telling whether it repeats a path registered earlier in
    /// the frame.
    #[cfg(debug_assertions)]
    pub(crate) fn register(&mut self, declared: Id, id: Id, readable: &str) -> Option<Repeat> {
        let first = self.paths.get(&declared).cloned();
        self.paths.insert(id, readable.to_owned());
        first.map(|first| {
            if first == readable {
                Repeat::Duplicate
            } else {
                Repeat::Collision { first }
            }
        })
    }

    /// Release builds register nothing.
    #[cfg(not(debug_assertions))]
    pub(crate) fn register(&mut self, _: Id, _: Id, _: &str) -> Option<Repeat> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Hash, Debug, Clone, Copy)]
    enum Appearance {
        Light,
        #[expect(dead_code)]
        Dark,
    }

    #[derive(Hash, Debug, Clone, Copy)]
    enum Lettering {
        Basic,
        #[expect(dead_code)]
        Fancy,
    }

    #[test]
    fn child_ids_depend_on_the_whole_path() {
        let a = child_id(child_id(root_id(), &"x"), &"y");
        let b = child_id(child_id(root_id(), &"y"), &"x");
        assert_ne!(a, b);
        assert_eq!(a, child_id(child_id(root_id(), &"x"), &"y"));
    }

    #[test]
    fn bare_enum_keys_of_different_types_do_not_collide() {
        let hash = |k: &dyn Fn(&mut std::hash::DefaultHasher)| {
            let mut h = std::hash::DefaultHasher::new();
            k(&mut h);
            std::hash::Hasher::finish(&h)
        };
        assert_eq!(
            hash(&|h| Appearance::Light.hash(h)),
            hash(&|h| Lettering::Basic.hash(h)),
            "precondition: derive(Hash) alone does not hash the enum type"
        );
        assert_ne!(
            child_id(root_id(), &Appearance::Light),
            child_id(root_id(), &Lettering::Basic)
        );
    }

    #[test]
    fn integers_and_newtypes_of_them_do_not_collide() {
        #[derive(Hash, Debug)]
        struct BlockKey(u32);
        assert_ne!(
            child_id(root_id(), &1u32),
            child_id(root_id(), &BlockKey(1))
        );
        assert_ne!(child_id(root_id(), &1u32), child_id(root_id(), &1u64));
    }

    #[test]
    fn same_type_and_value_gives_the_same_id() {
        assert_eq!(
            child_id(root_id(), &Appearance::Light),
            child_id(root_id(), &Appearance::Light)
        );
    }

    #[test]
    fn readable_path_follows_pushes_and_pops() {
        let mut path = ReadablePath::default();
        let outer = path.push(&"props");
        let inner = path.push(&("pin", 3));
        assert_eq!(path.as_str(), "/\"props\"/(\"pin\", 3)");
        path.pop_to(inner);
        assert_eq!(path.as_str(), "/\"props\"");
        path.pop_to(outer);
        assert_eq!(path.as_str(), "");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_repeated_path_is_a_duplicate_with_an_id_of_its_own() {
        let mut seen = Seen::default();
        let declared = child_id(root_id(), &"a");
        assert_eq!(seen.vacant(declared), declared);
        assert_eq!(seen.register(declared, declared, "/\"a\""), None);
        let second = seen.vacant(declared);
        assert_ne!(second, declared);
        assert_eq!(
            seen.register(declared, second, "/\"a\""),
            Some(Repeat::Duplicate)
        );
        let third = seen.vacant(declared);
        assert!(third != declared && third != second);
        assert_eq!(
            seen.register(declared, third, "/\"a\""),
            Some(Repeat::Duplicate)
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn colliding_distinct_paths_name_the_first() {
        let mut seen = Seen::default();
        let id = child_id(root_id(), &Appearance::Light);
        seen.register(id, id, "/Light");
        assert_eq!(
            seen.register(id, seen.vacant(id), "/Basic"),
            Some(Repeat::Collision {
                first: "/Light".to_owned()
            })
        );
    }

    #[test]
    fn a_new_frame_forgets_the_paths() {
        let mut seen = Seen::default();
        let id = child_id(root_id(), &"a");
        seen.register(id, id, "/\"a\"");
        seen.clear();
        assert_eq!(seen.vacant(id), id);
        assert_eq!(seen.register(id, id, "/\"a\""), None);
    }
}
