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

## License

MIT
