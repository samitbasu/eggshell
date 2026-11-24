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

/// TODO(prose): one class that set a property: its origin, and the value it gave while the node was traced.
#[cfg(debug_assertions)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// TODO(prose): who set it.
    pub origin: Origin,
    /// TODO(prose): the value it gave, written out; empty unless the node was traced.
    pub value: String,
}

#[cfg(debug_assertions)]
impl Step {
    /// The layer and source of this step, as the Computed-style pane writes them.
    pub fn describe(&self) -> String {
        let at = match self.origin.source {
            Source::Rust(at) => format!("{}:{}", at.file(), at.line()),
            Source::Shadcn { file, line, class } => format!("{file}:{line}  {class}"),
        };
        format!("{:?}  {at}", self.origin.layer)
    }
}

/// TODO(prose): why a node has the value it has for a property: the class that won it and, for a traced node, each it beat.
#[cfg(debug_assertions)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Why {
    /// TODO(prose): the class whose value the node has.
    pub winner: Step,
    /// TODO(prose): the classes it beat, in the order they were set. Empty unless the node was traced.
    pub beaten: Vec<Step>,
}

#[cfg(debug_assertions)]
thread_local! {
    static TRACING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the classes built now keep every class that set a property, with
/// its value, and not only the winner.
#[cfg(debug_assertions)]
pub(crate) fn tracing() -> bool {
    TRACING.with(std::cell::Cell::get)
}

/// Sets whether [`tracing`] holds, for the builds that follow.
#[cfg(debug_assertions)]
pub(crate) fn trace(on: bool) {
    TRACING.with(|tracing| tracing.set(on));
}

/// The winner of each property a class list sets.
#[cfg(debug_assertions)]
#[derive(Clone, Debug, Default)]
pub(crate) struct Origins(Vec<(Property, Why)>, Option<Source>);

#[cfg(debug_assertions)]
impl Origins {
    fn set(&mut self, property: Property, step: Step) {
        let traced = tracing();
        match self.0.iter_mut().find(|(known, _)| *known == property) {
            Some((_, known)) => {
                let beaten = std::mem::replace(&mut known.winner, step);
                if traced {
                    known.beaten.push(beaten);
                }
            }
            None => self.0.push((
                property,
                Why {
                    winner: step,
                    beaten: Vec::new(),
                },
            )),
        }
    }

    pub(crate) fn get(&self, property: Property) -> Option<&Why> {
        self.0
            .iter()
            .find_map(|(known, why)| (*known == property).then_some(why))
    }

    /// The properties with a winner, in their own order.
    pub(crate) fn properties(&self) -> Vec<(Property, &Why)> {
        let mut all: Vec<(Property, &Why)> = self.0.iter().map(|(p, why)| (*p, why)).collect();
        all.sort_by_key(|(property, _)| *property);
        all
    }

    /// These origins with `later`'s over them, property by property.
    pub(crate) fn over(mut self, later: Self) -> Self {
        let traced = tracing();
        for (property, why) in later.0 {
            match self.0.iter_mut().find(|(known, _)| *known == property) {
                Some((_, known)) => {
                    let beaten = std::mem::replace(&mut known.winner, why.winner);
                    if traced {
                        known.beaten.push(beaten);
                        known.beaten.extend(why.beaten);
                    }
                }
                None => self.0.push((property, why)),
            }
        }
        self
    }

    fn steps_mut(&mut self) -> impl Iterator<Item = &mut Step> {
        self.0
            .iter_mut()
            .flat_map(|(_, why)| std::iter::once(&mut why.winner).chain(&mut why.beaten))
    }
}

#[cfg(debug_assertions)]
impl Tw {
    /// TODO(prose): the origin of `property`'s value, if a class set one. Debug builds only.
    pub fn origin_of(&self, property: Property) -> Option<Origin> {
        Some(self.origins.as_ref()?.get(property)?.winner.origin)
    }

    /// TODO(prose): the winner of `property` and, if the classes were built while traced, what it beat. Debug builds only.
    pub fn why_of(&self, property: Property) -> Option<Why> {
        self.origins.as_ref()?.get(property).cloned()
    }

    /// Notes that the caller of the class method being run set `property`,
    /// or the class a [`from`](Self::from) before it named.
    #[track_caller]
    pub(super) fn note(&mut self, property: Property) {
        let value = if tracing() {
            self.value_of(property)
        } else {
            String::new()
        };
        let origins = self.origins.get_or_insert_with(Box::default);
        let step = Step {
            origin: Origin {
                layer: Layer::CallSite,
                source: origins.1.unwrap_or(Source::Rust(Location::caller())),
            },
            value,
        };
        origins.set(property, step);
    }

    /// TODO(prose): the classes called after this, up to the next `from`, were written for `class` at `file:line` of a shadcn sheet. Nothing in release builds.
    #[must_use]
    pub fn from(mut self, file: &'static str, line: u32, class: &'static str) -> Self {
        self.origins.get_or_insert_with(Box::default).1 =
            Some(Source::Shadcn { file, line, class });
        self
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
        for step in self
            .origins
            .iter_mut()
            .flat_map(|origins| origins.steps_mut())
        {
            if step.origin.layer == Layer::CallSite {
                step.origin.layer = layer;
            }
        }
        self
    }

    /// TODO(prose): gives the properties set so far by plain Rust calls `source`: the class of a shadcn sheet they were written for. Nothing in release builds.
    #[must_use]
    pub fn sourced(mut self, source: Source) -> Self {
        for step in self
            .origins
            .iter_mut()
            .flat_map(|origins| origins.steps_mut())
        {
            if matches!(step.origin.source, Source::Rust(_)) {
                step.origin.source = source;
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

    /// TODO(prose): the classes called after this, up to the next `from`, were written for `class` at `file:line` of a shadcn sheet. Nothing in release builds.
    #[must_use]
    #[inline(always)]
    pub fn from(self, _: &'static str, _: u32, _: &'static str) -> Self {
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

    fn shadcn(origin: Option<Origin>) -> Option<(u32, &'static str)> {
        match origin?.source {
            Source::Shadcn { line, class, .. } => Some((line, class)),
            Source::Rust(_) => None,
        }
    }

    #[test]
    fn from_gives_the_classes_after_it_their_source_until_the_next() {
        let classes = tw()
            .gap(1.0)
            .from("style.css", 3, "rounded-md")
            .rounded_px(8.0)
            .from("style.css", 4, "border px-2")
            .border(1.0)
            .px(2.0);
        assert_eq!(
            shadcn(classes.origin_of(Property::gap_width)),
            None,
            "before any"
        );
        assert_eq!(
            shadcn(classes.origin_of(Property::RadiusNw)),
            Some((3, "rounded-md"))
        );
        for property in [Property::border_left, Property::padding_left] {
            assert_eq!(
                shadcn(classes.origin_of(property)),
                Some((4, "border px-2")),
                "{property:?}"
            );
        }
        assert_eq!(
            classes.origin_of(Property::RadiusNw).map(|o| o.layer),
            Some(Layer::CallSite),
            "the layer is still the writer's"
        );
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
        let staged = tw().px(2.0).staged(Layer::Base).merge(tw().py(1.0));
        let staged = staged.staged(Layer::App);
        assert_eq!(
            staged.origin_of(Property::padding_left).map(|o| o.layer),
            Some(Layer::Base)
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

    #[test]
    fn tracing_keeps_each_class_a_winner_beat_with_its_value() {
        trace(true);
        let classes = tw()
            .px(2.0)
            .staged(Layer::Base)
            .merge(tw().px(4.0).staged(Layer::App))
            .merge(tw().px(6.0));
        trace(false);
        let why = classes.why_of(Property::padding_left).expect("set");
        assert_eq!(why.winner.origin.layer, Layer::CallSite);
        let beaten: Vec<Layer> = why.beaten.iter().map(|s| s.origin.layer).collect();
        assert_eq!(beaten, [Layer::Base, Layer::App]);
        assert_eq!(why.winner.value, "6 units");
        assert_eq!(why.beaten[0].value, "2 units");
    }

    #[test]
    fn without_tracing_only_the_winner_is_kept() {
        let classes = tw().px(2.0).merge(tw().px(4.0));
        let why = classes.why_of(Property::padding_left).expect("set");
        assert!(why.beaten.is_empty());
        assert_eq!(why.winner.value, "");
    }

    #[test]
    fn a_variant_notes_the_line_that_declared_it() {
        let (classes, line) = (tw().hover(|p| p.bg(Color32::RED)), line!());
        assert_eq!(
            rust(classes.origin_of(Property::VariantHover)),
            (Layer::CallSite, line)
        );
        assert_eq!(classes.origin_of(Property::VariantActive), None);
    }
}
