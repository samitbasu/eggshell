# bullseye

Taffy lays out, egui draws and interacts: the opposite of `egui_taffy`.

Each frame has three phases: build a taffy tree from application state, solve
layout, then draw with egui. The crate is under construction; see
[`docs/design.md`](docs/design.md) for the design brief and `CLAUDE.md` for the
engineering practices.

## Development

Automation lives in the `xtask` crate; run `cargo xtask --help` for the list.

- `cargo xtask ci`: everything CI runs (fmt check, clippy, tests, doctests, rustdoc), stopping at the first failure
- `cargo xtask fmt [--check]`: format the workspace
- `cargo xtask test [-- ARGS]`: all tests and doctests; `ARGS` go to `cargo test`
- `cargo xtask coverage [--lcov]`: coverage via `cargo-llvm-cov` (HTML report, or `lcov.info`)
- `cargo xtask example [NAME] [--release]`: run an example, or list them
- `cargo xtask snapshots [--update] [--filter NAME] [--clean]`: golden-image tests (see below)

### Golden images

Drawing is guarded by reference screenshots in `tests/snapshots/`, rendered
headlessly by `egui_kittest` with wgpu. They sit behind the `snapshots` cargo
feature, so plain `cargo test` needs no GPU. `cargo xtask snapshots` enables the
feature and points Vulkan at Mesa lavapipe (the software renderer, from the
`mesa-vulkan-drivers` package) so the pixels do not depend on your GPU; it fails
with a message if lavapipe is not installed. It is not part of `cargo xtask ci`
yet.

- Check (the default) fails on any difference and prints the path of every
  `name.new.png` (what was drawn) and `name.diff.png` (where it differs) so you
  can open them.
- `--update` regenerates the goldens (it sets `UPDATE_SNAPSHOTS=1` for the test
  run only) and lists the added, modified and removed ones. Regenerating is a
  reviewed step, not a way to silence a failure: look at every changed image, and
  state in the commit message what changed and why.
- `--filter NAME` runs only the tests whose name contains `NAME`.
- `--clean` deletes the stray `.new.png` and `.diff.png` files.

## License

MIT
