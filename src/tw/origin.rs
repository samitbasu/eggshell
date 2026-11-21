//! Which class set each property of a class list, and in which layer it was
//! written. Debug builds only: a release `Tw` carries none of it.

use std::panic::Location;

#[cfg(debug_assertions)]
use super::Property;
use super::Tw;
use crate::sheet::Layer;

/// TODO(prose): who wrote a class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// TODO(prose): the Rust call that wrote it.
    Rust(&'static Location<'static>),
    /// TODO(prose): a class of a shadcn style sheet.
    Shadcn {
        /// TODO(prose): the file of the pinned shadcn checkout.
        file: &'static str,
        /// TODO(prose): the line in `file`.
        line: u32,
        /// TODO(prose): the class as written there.
        class: &'static str,
    },
}

/// TODO(prose): where the value a property has came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin {
    /// TODO(prose): the layer whose rule or call set it.
    pub layer: Layer,
    /// TODO(prose): the class that set it.
    pub source: Source,
}

/// The winner of each property a class list sets.
#[cfg(debug_assertions)]
#[derive(Clone, Debug, Default)]
pub(crate) struct Origins(Vec<(Property, Origin)>);

#[cfg(debug_assertions)]
impl Origins {
    fn set(&mut self, property: Property, origin: Origin) {
        match self.0.iter_mut().find(|(known, _)| *known == property) {
            Some(known) => known.1 = origin,
            None => self.0.push((property, origin)),
        }
    }

    fn get(&self, property: Property) -> Option<Origin> {
        self.0
            .iter()
            .find_map(|(known, origin)| (*known == property).then_some(*origin))
    }

    /// These origins with `later`'s over them, property by property.
    pub(crate) fn over(mut self, later: Self) -> Self {
        for (property, origin) in later.0 {
            self.set(property, origin);
        }
        self
    }
}

#[cfg(debug_assertions)]
impl Tw {
    /// TODO(prose): the origin of `property`'s value, if a class set one. Debug builds only.
    pub fn origin_of(&self, property: Property) -> Option<Origin> {
        self.origins.as_ref()?.get(property)
    }

    /// Notes that the caller of the class method being run set `property`.
    #[track_caller]
    pub(super) fn note(&mut self, property: Property) {
        let origin = Origin {
            layer: Layer::CallSite,
            source: Source::Rust(Location::caller()),
        };
        self.origins
            .get_or_insert_with(Box::default)
            .set(property, origin);
    }

    /// Notes each of `properties` the caller set.
    #[track_caller]
    pub(super) fn note_all(&mut self, properties: impl IntoIterator<Item = Property>) {
        for property in properties {
            self.note(property);
        }
    }

    /// Gives the properties set so far by plain calls the layer `layer`.
    pub(crate) fn staged(mut self, layer: Layer) -> Self {
        for (_, origin) in self
            .origins
            .iter_mut()
            .flat_map(|origins| origins.0.iter_mut())
        {
            if origin.layer == Layer::CallSite {
                origin.layer = layer;
            }
        }
        self
    }

    /// TODO(prose): gives the properties set so far by plain Rust calls `source`: the class of a shadcn sheet they were written for. Nothing in release builds.
    #[must_use]
    pub fn sourced(mut self, source: Source) -> Self {
        for (_, origin) in self
            .origins
            .iter_mut()
            .flat_map(|origins| origins.0.iter_mut())
        {
            if matches!(origin.source, Source::Rust(_)) {
                origin.source = source;
            }
        }
        self
    }
}

#[cfg(not(debug_assertions))]
impl Tw {
    #[inline(always)]
    pub(crate) fn staged(self, _: Layer) -> Self {
        self
    }

    /// TODO(prose): gives the properties set so far by plain Rust calls `source`: the class of a shadcn sheet they were written for. Nothing in release builds.
    #[must_use]
    #[inline(always)]
    pub fn sourced(self, _: Source) -> Self {
        self
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use egui::Color32;

    use super::*;
    use crate::tw::tw;

    fn rust(origin: Option<Origin>) -> (Layer, u32) {
        let Some(Origin {
            layer,
            source: Source::Rust(at),
        }) = origin
        else {
            return (Layer::Defaults, 0);
        };
        (layer, at.line())
    }

    #[test]
    fn a_class_notes_the_line_that_called_it() {
        let (classes, line) = (tw().px(2.0), line!());
        assert_eq!(
            rust(classes.origin_of(Property::padding_left)),
            (Layer::CallSite, line)
        );
        assert_eq!(classes.origin_of(Property::padding_top), None, "only px");
        assert_eq!(classes.origin_of(Property::width), None);
    }

    #[test]
    fn every_kind_of_class_notes_its_property() {
        let line = line!() + 2;
        let classes = tw()
            .rounded_md()
            .bg(Color32::RED)
            .border_t_color(Color32::BLUE)
            .text(Color32::GREEN);
        for (property, extra) in [
            (Property::RadiusNw, 0),
            (Property::Background, 1),
            (Property::BorderColorTop, 2),
            (Property::TextColor, 3),
        ] {
            assert_eq!(
                rust(classes.origin_of(property)),
                (Layer::CallSite, line + extra),
                "{property:?}"
            );
        }
        assert_eq!(classes.origin_of(Property::BorderColorLeft), None);
    }

    #[test]
    fn a_merge_keeps_the_origin_of_the_class_that_won_each_property() {
        let (first, first_line) = (tw().px(2.0).gap(1.0), line!());
        let (second, second_line) = (tw().px(4.0), line!());
        let merged = first.merge(second);
        assert_eq!(
            rust(merged.origin_of(Property::padding_left)),
            (Layer::CallSite, second_line)
        );
        assert_eq!(
            rust(merged.origin_of(Property::gap_width)),
            (Layer::CallSite, first_line)
        );
    }

    #[test]
    fn a_class_given_a_non_finite_length_notes_nothing() {
        let rejected = tw().w_px(f32::NAN);
        assert_eq!(rejected.origin_of(Property::width), None);
    }

    #[test]
    fn staging_gives_plain_calls_a_layer_and_leaves_staged_ones() {
        let staged = tw().px(2.0).staged(Layer::Preset).merge(tw().py(1.0));
        let staged = staged.staged(Layer::App);
        assert_eq!(
            staged.origin_of(Property::padding_left).map(|o| o.layer),
            Some(Layer::Preset)
        );
        assert_eq!(
            staged.origin_of(Property::padding_top).map(|o| o.layer),
            Some(Layer::App)
        );
    }

    #[test]
    fn sourcing_names_the_class_a_rust_call_stands_for() {
        let class = Source::Shadcn {
            file: "styles/style-vega.css",
            line: 182,
            class: "px-2.5",
        };
        let sourced = tw().px(2.5).sourced(class);
        let again = sourced.clone().merge(tw().py(1.0)).sourced(Source::Shadcn {
            file: "styles/style-vega.css",
            line: 183,
            class: "py-1",
        });
        assert_eq!(
            sourced.origin_of(Property::padding_left).map(|o| o.source),
            Some(class)
        );
        assert_eq!(
            again.origin_of(Property::padding_left).map(|o| o.source),
            Some(class),
            "a class already sourced keeps its source"
        );
        assert!(matches!(
            again.origin_of(Property::padding_top).map(|o| o.source),
            Some(Source::Shadcn { line: 183, .. })
        ));
    }
}
