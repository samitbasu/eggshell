//! Key paths: the identity of every node.
//!
//! A path is a sequence of keys, each any `Hash + Debug + 'static` value. The
//! egui [`Id`] of a node is the running hash of its path, and the readable
//! form (each key's `Debug` text) is kept for diagnostics.
//!
//! Each segment hashes the key together with its type name, so keys need only
//! be unique among siblings of the same type: variants of different enums, or
//! `1u32` and a `BlockKey(1)` newtype, never collide. The type name (unlike
//! `TypeId`) is stable across runs, so state egui persists keeps its identity.

use std::fmt::{Debug, Write as _};
use std::hash::Hash;

use egui::Id;

/// The root of every key path; a salt keeps bullseye ids apart from other
/// users of [`Id::NULL`].
pub(crate) fn root_id() -> Id {
    Id::NULL.with("bullseye")
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
}

/// The id a key has under `parent`.
pub(crate) fn child_id<K: Hash + Debug + 'static>(parent: Id, key: &K) -> Id {
    parent.with((std::any::type_name::<K>(), key))
}

/// Debug-build registry that no identified path occurs twice in a frame.
#[derive(Default)]
pub(crate) struct Seen {
    #[cfg(debug_assertions)]
    paths: std::collections::HashMap<Id, String>,
}

impl Seen {
    pub(crate) fn clear(&mut self) {
        #[cfg(debug_assertions)]
        self.paths.clear();
    }

    /// Registers an identified path.
    ///
    /// # Panics
    ///
    /// In debug builds, if `id` was registered earlier in the frame. The
    /// message names both readable paths, which differ when two distinct keys
    /// hash alike (for example variants of different enums).
    #[cfg_attr(not(debug_assertions), expect(unused_variables))]
    pub(crate) fn register(&mut self, id: Id, readable: &str) {
        #[cfg(debug_assertions)]
        if let Some(first) = self.paths.insert(id, readable.to_owned()) {
            if first == readable {
                panic!("duplicate key path {readable}");
            }
            panic!("key paths {first} and {readable} have the same id");
        }
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
    #[should_panic(expected = "duplicate key path /\"a\"")]
    fn registering_a_path_twice_panics_with_the_readable_path() {
        let mut seen = Seen::default();
        let id = child_id(root_id(), &"a");
        seen.register(id, "/\"a\"");
        seen.register(id, "/\"a\"");
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "/Light and /Basic have the same id")]
    fn colliding_distinct_paths_name_both() {
        let mut seen = Seen::default();
        let id = child_id(root_id(), &Appearance::Light);
        seen.register(id, "/Light");
        seen.register(id, "/Basic");
    }
}
