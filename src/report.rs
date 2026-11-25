//! The library's one channel for misuse and recovered invariants: each is
//! logged once, counted after that, listed by the debug overlay and queryable
//! by tests.

use std::panic::Location;

use egui::{Context, Id};

/// TODO(prose): what went wrong, by kind.
///
/// ```
/// use eggshell_ui::Problem;
/// assert_ne!(Problem::DuplicateKey, Problem::KeyCollision);
/// assert_eq!(Problem::Internal("tree"), Problem::Internal("tree"));
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Problem {
    /// TODO(prose): a key path declared twice in a frame.
    DuplicateKey,
    /// TODO(prose): two different key paths with the same id.
    KeyCollision,
    /// TODO(prose): text wider than the widest template its node reserves
    /// room for ([`Tw::min_w_text`](crate::Tw::min_w_text)): the node grew,
    /// so its neighbours moved. Reported in debug builds.
    OutgrewReserve,
    /// TODO(prose): two live layouts of a context with the same name.
    LayoutNameClash,
    /// TODO(prose): two live layouts of a context with the same top-level key.
    TopLevelKeyClash,
    /// TODO(prose): a command table with a key twice.
    DuplicateCommand,
    /// TODO(prose): two commands of a table on one chord.
    DuplicateChord,
    /// TODO(prose): a command bound to a chord the browser keeps, such as
    /// `Cmd+W`. Debug builds on the web only.
    BrowserChord,
    /// TODO(prose): commands declared twice in one root.
    CommandsTwice,
    /// TODO(prose): a command id no live table declares.
    UnknownCommand,
    /// TODO(prose): Markdown whose structure the parser had to recover from.
    MalformedMarkdown,
    /// TODO(prose): a key the style has no entry for.
    MissingStyleEntry,
    /// TODO(prose): two nodes of an overlay marked to take focus on open.
    FocusOnOpenTwice,
    /// TODO(prose): a call that needs an identified node, on an anonymous one.
    NeedsIdentity,
    /// TODO(prose): a keyboard pattern only an overlay has, on a group on the page.
    NavNeedsOverlay,
    /// TODO(prose): a group variant with no group around it.
    GroupHoverWithoutGroup,
    /// TODO(prose): a group that senses nothing, so is never hovered.
    GroupSensesNothing,
    /// TODO(prose): a control with no label and no name. Debug builds only.
    Unnamed,
    /// TODO(prose): a drag handle where there is no drag source.
    DragHandle,
    /// TODO(prose): an overlay whose anchor is not in the frame.
    MissingAnchor,
    /// TODO(prose): a table row of more or fewer cells than columns, or a
    /// table of more rows than it can number.
    TableShape,
    /// TODO(prose): a splitter outside a flex container.
    MisplacedSplitter,
    /// TODO(prose): icons given to a text field that shows none.
    IgnoredIcons,
    /// TODO(prose): a value range that is reversed or not a number.
    BadRange,
    /// TODO(prose): a length that is infinite or not a number.
    NonFiniteLength,
    /// TODO(prose): a value that is not a number.
    NanValue,
    /// TODO(prose): a weight the style declares no face for.
    MissingFace,
    /// TODO(prose): the shell asked for outside a draw.
    ShellOutsideDraw,
    /// TODO(prose): a node accessor of [`NodeUi`](crate::NodeUi) on a `Ui` no draw closure was given.
    NodeUiOutsideDraw,
    /// TODO(prose): a text colour on a surface under WCAG 2's AA contrast.
    LowContrast,
    /// TODO(prose): a hover card declared inside another hover card's
    /// content. It never opens.
    NestedHoverCard,
    /// TODO(prose): two layouts on one context built in the same pass with styles that give egui different looks, so egui's own widgets draw in whichever gave it last.
    EguiStyleClash,
    /// TODO(prose): an invariant of the library that did not hold, by name.
    Internal(&'static str),
}

impl Problem {
    fn target(self) -> Target {
        match self {
            Self::Internal(_) => Target::Internal,
            _ => Target::Misuse,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Misuse,
    Internal,
}

/// TODO(prose): one problem at one place, with how often it happened.
///
/// ```
/// use eggshell_ui::testing::Headless;
/// use eggshell_ui::{Builder, tw};
/// use egui::vec2;
///
/// let mut headless = Headless::new(vec2(100.0, 100.0));
/// headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
///     b.div_id("a", tw(), |_| {});
///     b.div_id("a", tw(), |_| {});
/// });
/// for report in headless.reports() {
///     println!("{}x {:?} at {}: {}", report.count, report.problem, report.at, report.message);
/// }
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// TODO(prose): the kind of problem.
    pub problem: Problem,
    /// TODO(prose): the call that ran into it.
    pub at: &'static Location<'static>,
    /// TODO(prose): what the first occurrence said.
    pub message: String,
    /// TODO(prose): how many times it happened.
    pub count: u32,
}

/// The most distinct reports a context keeps; later ones are only counted.
const CAPACITY: usize = 256;

#[derive(Clone, Default)]
struct Store {
    entries: Vec<(Option<Id>, Report)>,
    dropped: u32,
}

enum Seen {
    Again,
    New,
    Dropped { first: bool },
}

impl Store {
    fn bump(
        &mut self,
        problem: Problem,
        at: &'static Location<'static>,
        subject: Option<Id>,
    ) -> Seen {
        if let Some((_, report)) = self
            .entries
            .iter_mut()
            .find(|(s, report)| *s == subject && report.problem == problem && report.at == at)
        {
            report.count = report.count.saturating_add(1);
            return Seen::Again;
        }
        if self.entries.len() < CAPACITY {
            return Seen::New;
        }
        self.dropped = self.dropped.saturating_add(1);
        Seen::Dropped {
            first: self.dropped == 1,
        }
    }
}

fn store_id() -> Id {
    Id::NULL.with("eggshell-reports")
}

fn with_store<R>(ctx: &Context, f: impl FnOnce(&mut Store) -> R) -> R {
    ctx.data_mut(|data| f(data.get_temp_mut_or_default::<Store>(store_id())))
}

/// Reports `problem` at the caller, about `subject` if it concerns one
/// identified thing. Repeats of a (problem, caller, subject) are counted, and
/// only the first runs `message` and is logged.
#[track_caller]
pub(crate) fn report(
    ctx: &Context,
    problem: Problem,
    subject: Option<Id>,
    message: impl FnOnce() -> String,
) {
    report_at(ctx, problem, subject, Location::caller(), message);
}

/// [`report`] with the place given rather than taken from the caller.
pub(crate) fn report_at(
    ctx: &Context,
    problem: Problem,
    subject: Option<Id>,
    at: &'static Location<'static>,
    message: impl FnOnce() -> String,
) {
    match with_store(ctx, |store| store.bump(problem, at, subject)) {
        Seen::Again | Seen::Dropped { first: false } => {}
        Seen::Dropped { first: true } => log::warn!(
            target: "eggshell::misuse",
            "more than {CAPACITY} distinct problems were reported; later ones are only counted"
        ),
        Seen::New => {
            // `message` runs outside the context lock: it may read the context.
            let message = message();
            match problem.target() {
                Target::Misuse => log::warn!(target: "eggshell::misuse", "{message} (at {at})"),
                Target::Internal => {
                    log::error!(target: "eggshell::internal", "{message} (at {at})");
                }
            }
            with_store(ctx, |store| {
                store.entries.push((
                    subject,
                    Report {
                        problem,
                        at,
                        message,
                        count: 1,
                    },
                ));
            });
        }
    }
}

/// The reports kept in `ctx`, oldest first.
pub(crate) fn reports(ctx: &Context) -> Vec<Report> {
    with_store(ctx, |store| {
        store
            .entries
            .iter()
            .map(|(_, report)| report.clone())
            .collect()
    })
}

/// The reports kept in `ctx`, which keeps none after.
pub(crate) fn take(ctx: &Context) -> Vec<Report> {
    with_store(ctx, std::mem::take)
        .entries
        .into_iter()
        .map(|(_, report)| report)
        .collect()
}

/// How many distinct problems were not kept because the store was full.
pub(crate) fn dropped(ctx: &Context) -> u32 {
    with_store(ctx, |store| store.dropped)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock, PoisonError};

    use super::*;

    struct Recorder(Mutex<Vec<(log::Level, String, String)>>);

    impl log::Log for Recorder {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push((
                record.level(),
                record.target().to_owned(),
                record.args().to_string(),
            ));
        }

        fn flush(&self) {}
    }

    /// The records logged so far whose message contains `needle`.
    fn logged(needle: &str) -> Vec<(log::Level, String, String)> {
        static RECORDER: OnceLock<&'static Recorder> = OnceLock::new();
        let recorder = RECORDER.get_or_init(|| {
            let recorder: &'static Recorder = Box::leak(Box::new(Recorder(Mutex::new(Vec::new()))));
            log::set_logger(recorder).expect("no other logger in the unit tests");
            log::set_max_level(log::LevelFilter::Trace);
            recorder
        });
        recorder
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|(_, _, message)| message.contains(needle))
            .cloned()
            .collect()
    }

    #[track_caller]
    fn here(ctx: &Context, problem: Problem, subject: Option<Id>, message: &str) {
        report(ctx, problem, subject, || message.to_owned());
    }

    #[test]
    fn repeats_are_counted_and_only_the_first_builds_its_message() {
        let ctx = Context::default();
        let mut built = 0;
        for _ in 0..3 {
            report(&ctx, Problem::DuplicateKey, None, || {
                built += 1;
                "dup".to_owned()
            });
        }
        assert_eq!(built, 1);
        let reports = reports(&ctx);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].count, 3);
        assert_eq!(reports[0].message, "dup");
        assert_eq!(reports[0].at.file(), file!());
    }

    #[test]
    fn problem_place_and_subject_each_tell_reports_apart() {
        let ctx = Context::default();
        let a = Some(Id::NULL.with("a"));
        for _ in 0..2 {
            here(&ctx, Problem::DuplicateKey, a, "one");
            here(&ctx, Problem::DuplicateKey, None, "two");
            here(&ctx, Problem::KeyCollision, a, "three");
        }
        here(&ctx, Problem::DuplicateKey, a, "four");
        let counts: Vec<_> = reports(&ctx)
            .into_iter()
            .map(|report| (report.message, report.count))
            .collect();
        assert_eq!(
            counts,
            [
                ("one".to_owned(), 2),
                ("two".to_owned(), 2),
                ("three".to_owned(), 2),
                ("four".to_owned(), 1),
            ]
        );
    }

    #[test]
    fn past_the_cap_new_problems_are_only_counted() {
        let ctx = Context::default();
        for n in (0..CAPACITY + 3).chain([0]) {
            here(&ctx, Problem::DuplicateKey, Some(Id::NULL.with(n)), "full");
        }
        let reports = reports(&ctx);
        assert_eq!(reports.len(), CAPACITY);
        assert_eq!(reports[0].count, 2, "a kept report still counts");
        assert_eq!(dropped(&ctx), 3);
    }

    #[test]
    fn taking_empties_the_store() {
        let ctx = Context::default();
        here(&ctx, Problem::MissingFace, None, "taken");
        assert_eq!(take(&ctx).len(), 1);
        assert!(reports(&ctx).is_empty());
        here(&ctx, Problem::MissingFace, None, "taken");
        assert_eq!(reports(&ctx)[0].count, 1, "a taken report starts over");
    }

    #[test]
    fn misuse_is_warned_and_internal_errors_are_errors_once_each() {
        logged("");
        let ctx = Context::default();
        for _ in 0..2 {
            here(&ctx, Problem::DuplicateKey, None, "logged misuse 7c1f");
            here(
                &ctx,
                Problem::Internal("tree"),
                None,
                "logged invariant 7c1f",
            );
        }
        let misuse = logged("logged misuse 7c1f");
        assert_eq!(misuse.len(), 1, "{misuse:?}");
        assert_eq!(misuse[0].0, log::Level::Warn);
        assert_eq!(misuse[0].1, "eggshell::misuse");
        assert!(misuse[0].2.contains(file!()), "{misuse:?}");
        let internal = logged("logged invariant 7c1f");
        assert_eq!(internal.len(), 1, "{internal:?}");
        assert_eq!(internal[0].0, log::Level::Error);
        assert_eq!(internal[0].1, "eggshell::internal");
    }
}
