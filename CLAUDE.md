# Engineering practices

These practices are project-agnostic. They travel with me between repositories;
project-specific notes live in the final section, which you can delete when
copying this file elsewhere.

## Design philosophy

1. **Prefer the simplest design that exploits the problem's invariants.**
   Before designing for the general case, identify the invariant that makes the
   common case cheap (e.g. "the routing graph is fixed while a gesture is in
   progress — cache the router per gesture" beats an incremental graph
   migration). When a plan is getting large, stop and check whether a simpler
   observation collapses it.
2. **"Simple" means no over-engineering — it does not mean avoiding
   complexity or abstraction.** Sharp tools (higher-ranked trait bounds,
   advanced generics) are permitted, but they are sharp: each use needs a
   justification. Speculative machinery for problems we don't have yet is
   over-engineering; a well-chosen abstraction for a problem we do have is not.
3. **Fix the API, don't work around it.** If a new feature is awkward because
   of the current API's shape, spike and rework the API first. Work-arounds are
   fragile, accumulate as rot, and the rework is an opportunity to reconsider
   the structure. (A per-feature fallback that patched around a missing input
   model lived exactly one day before the model grew the missing concept and
   the fallback was deleted — build the concept first.)
4. **Tech debt is a real concern.** A forked or parallel implementation of
   anything *will* diverge from its sibling and generate issues downstream.
   Treat duplication and drift as costs on par with bugs.

## DRY and shared functionality

- **Copy-paste is to be avoided wherever possible.** Parallel implementations
  that must be updated in lockstep are silent-drift hazards.
- The preference ladder for sharing functionality:
  **pure functions → helper functions → methods on structs/enums → macros
  (last resort).** All of these beat pasting the logic again.
- Policy that several call sites must agree on (a hit-test priority order, a
  z-order, a dispatch sequence) should be **encoded once** — as data or a
  single resolver — and consumed everywhere, so the sites cannot disagree.
- **External crates are preferred to reproducing their functionality in-app.**
  If a library we already depend on computes the thing (e.g. the UI toolkit's
  text-layout engine), consume its output rather than approximating it —
  even at modest integration cost — because the in-house approximation will
  drift.

## The type system encodes invariants

- **Illegal states should be unrepresentable.** Mutually exclusive application
  modes are one enum, not N `Option`/`bool` fields. A press-with-origin is
  `Option<Press { origin }>`, never `pointer_down: bool` + `origin:
  Option<_>` which can disagree.
- **No naked `bool` arguments.** Use a two-variant enum or a typed wrapper
  that names the meaning at the call site.
- **No naked `f32`/`f64` arguments.** A bare float in a signature carries no
  meaning. Wrap it in a newtype naming the semantic (`Zoom`, `Progress`,
  `WorldDistance`): constructor to wrap, `impl From<Newtype> for f32` so callees
  extract via `Into`. If the value is *constrained* (non-negative, unit
  interval), it stays wrapped everywhere and the newtype enforces the
  constraint — clamping constructors, saturating ops implemented once on
  `std::ops` impls (never re-clamped at call sites), NaN handled so ordering
  is total.
- **Durations are `core::time::Duration`. Always.** Convert to float seconds
  only at foreign API boundaries. This makes negative durations and unit
  confusion unrepresentable, and deletes their guard code.
- Prefer typed enums with `Into` impls over threading primitives through
  layers.

## Comments

- Follow the clean-code principle: **comment only surprising or unexpected
  behavior.** Never narrate what the code does, where a change came from, or
  why a change is correct — that is reviewer-talk, noise once merged.
- If a function needs a comment to be understood, that is refactoring
  pressure, not documentation debt. **Heavily commented code is a warning to
  the reader that it requires extra thought** — restructure until the comment
  is unnecessary.
- Prefer names and types that make intent clear; comments drift, names and
  types do not.
- Data structures are commented only when part of a public API or when their
  intent is non-obvious.

## Dependencies

- Before building a feature on a deprecated or outdated API, **migrate the
  dependency to its latest version first**, then build on the current API.
  Building on the old surface doubles the eventual migration.

## Testing and verification

- A test must **prove** what it claims: if it depends on a geometric or
  structural precondition (two rects overlap, a target exists), assert the
  precondition inside the test so it can't silently stop testing anything.
- Verify UI behavior **through the full real path** (dispatch, tool switching,
  event flow), not only through isolated component tests — isolated tests can
  pass while the integrated path misbehaves.
- Refactors declared behavior-preserving must not change any test outcome.
  Behavior *changes* are enumerated explicitly (what, where, why) at review
  time, never discovered later.
- Golden/regression tests guard end-to-end behavior; regenerating a golden is
  an acceptance step whose diff gets reviewed, not a chore to silence.
- Zero clippy warnings; new code lands with its tests.

## Developer-facing tooling

- For author/dev tooling, prefer the console and an edit-and-re-run loop over
  in-app UI (transport controls, guards, on-screen error panes). Print errors
  where the developer already is; keep flags few and composable.
- Errors that point at a file should carry spans and render with the source
  attached (e.g. miette-style reports), not stringly byte offsets.

## Workflow conventions

- Commit messages explain the *why* and the shape of the change; intentional
  behavior changes are listed in the message.
- Breaking changes are acceptable while a project is undeployed, in the name
  of simplification — note them clearly.
- **A worktree is cut from `origin/main`, fetched first** (`git fetch origin
  && git worktree add <dir> -b <branch> origin/main`), never from the local
  `main` or another branch: fixes land in quick succession,
  and a branch cut from a stale base rebuilds what is already merged and
  conflicts on the way in.
- **A rebase or merge that touches what the user tried needs a manual
  verification again.** A UI change the user has approved is approved as it
  looked then; once a rebase has brought in other changes to the same
  region, put the rebased build up for them to try again before the PR is
  called ready — green CI and a green walk do not stand in for it.
- **Stop only the processes you started, by PID or task handle — never
  `pkill`, `killall` or `pkill -f` by name.** Agents share the machine, and a
  pattern matches every agent's browsers, servers and walks. Background
  commands are stopped through their task; a process you did not start is
  left alone, and the user is told of it. `cargo xtask` puts the children it
  spawns (a walk's server, chromedriver, scripts) in process groups it ends
  itself, on exit and on an interrupt or termination signal, so a leftover is
  a bug to report, not something to sweep.
- **Aesthetic choices are the user's.** Icons, colours, layout, spacing,
  typography, wording on screen — anything about how the app looks or
  reads — are run past the user before they land, never decided on their
  behalf. Where there are candidates, show all of them (rendered, at the
  size they will be used, beside what surrounds them), give a
  recommendation, and wait for the pick.
- **A change that touches the UI or UX is tried by the user before its PR
  is approved.** Put the build up where they can reach it and say so on
  the PR; passing tests and screenshots do not
  stand in for their own hands on it.

---

## Project-specific notes

- Repetitive work and scripts go through `cargo xtask`, never shell scripts,
  Makefiles or justfiles. CI calls the same commands a developer runs.
- All integration tests are one binary, `tests/it/main.rs`, with one module
  per former test file (`cargo test --test it layout::`). Each test crate
  statically links egui and friends, and a binary per file exhausted the CI
  runner's disk. Add new test files as modules there, not as `tests/*.rs`.
- `cargo xtask ci` is the single definition of "CI passes". Also available:
  `fmt [--check]`, `test [-- ARGS]`, `coverage [--lcov]`,
  `example [NAME] [--release]`. See `cargo xtask --help`.
- Golden images (`tests/snapshots/`, rendered by `tests/it/snapshots.rs` behind the
  `snapshots` cargo feature) are checked with `cargo xtask snapshots`, which
  forces Mesa lavapipe so every machine renders the same pixels. It is not in
  `cargo xtask ci` yet. After a change that can move pixels, run it; on a
  mismatch open the `.diff.png` files it prints. If the change is intended,
  `cargo xtask snapshots --update` regenerates the goldens and lists the
  added, modified and removed ones: look at each image (the PNG diff is the
  review, as in a code diff) before committing, and say in the commit message
  what changed and why. Never update goldens to silence a failure.
  `--filter NAME` limits the run to goldens whose test name starts with `NAME`; `--clean` deletes stray `.new.png` and
  `.diff.png` files.
