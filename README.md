# eggshell

Taffy lays out, egui draws and interacts: the opposite of `egui_taffy`.

Each frame has three phases: build a taffy tree from application state, solve
layout, then draw with egui. The crate is under construction; see
[`docs/design.md`](docs/design.md) for the design brief and `CLAUDE.md` for the
engineering practices.

## Development

Automation lives in the `xtask` crate; run `cargo xtask --help` for the list.

- `cargo xtask ci`: everything CI runs (fmt check, clock check, clippy, tests, doctests, rustdoc, wasm build, golden images); every step runs, and the failed ones are listed at the end
- `cargo xtask fmt [--check]`: format the workspace
- `cargo xtask test [-- ARGS]`: all tests and doctests; `ARGS` go to `cargo test`
- `cargo xtask coverage [--lcov]`: coverage via `cargo-llvm-cov` (HTML report, or `lcov.info`)
- `cargo xtask fuzz [--cases N]`: the randomised continuity fuzzers with many cases (2000 by default, in release); not part of `ci`, whose default is 32 per test. A failing sequence is shrunk and saved under `tests/proptest-regressions/`, to be committed
- `cargo xtask example [NAME] [--release]`: run an example, or list them
- `cargo xtask web NAME [--release] [--port N]`: build an example for the browser (wasm32) and serve it locally until Ctrl+C (see below)
- `cargo xtask snapshots [--update] [--filter NAME] [--clean]`: golden-image tests (see below)

### Examples in the browser

Every example also runs on the web. `cargo xtask web hello` builds it for
`wasm32-unknown-unknown`, runs `wasm-bindgen` into `target/web/hello/`, and
serves it, printing the URL. It needs the wasm target
(`rustup target add wasm32-unknown-unknown`) and a `wasm-bindgen` CLI of exactly
the version `Cargo.lock` pins; if yours differs, the command prints the
`cargo install wasm-bindgen-cli --version X --locked` line to run. `cargo xtask ci`
checks that the library and all examples compile for wasm32, and that library
code does not use `std::time::Instant` or `SystemTime` (they panic in the
browser; use egui's clock). Try UI changes in the browser as well as natively.

### Golden images

Drawing is guarded by reference screenshots in `tests/snapshots/`, rendered
headlessly by `egui_kittest` with wgpu. They sit behind the `snapshots` cargo
feature, so plain `cargo test` needs no GPU. `cargo xtask snapshots` enables the
feature and points Vulkan at Mesa lavapipe (the software renderer, from the
`mesa-vulkan-drivers` package) so the pixels do not depend on your GPU; it fails
with a message if lavapipe is not installed. `cargo xtask ci` runs the check,
so lavapipe is needed there too.

- Check (the default) fails on any difference and prints the path of every
  `name.new.png` (what was drawn) and `name.diff.png` (where it differs) so you
  can open them.
- `--update` regenerates the goldens (it sets `UPDATE_SNAPSHOTS=1` for the test
  run only) and lists the added, modified and removed ones. Regenerating is a
  reviewed step, not a way to silence a failure: look at every changed image, and
  state in the commit message what changed and why.
- `--filter NAME` runs only the goldens whose test name starts with `NAME` (they live in the `snapshots` module of the `it` test binary, so `NAME` is matched after `snapshots::`).
- `--clean` deletes the stray `.new.png` and `.diff.png` files.

## License

MIT

The symbol fallback face (`src/fonts/DejaVuSans-symbols.ttf`) is a subset of DejaVu Sans, under the licence in `src/fonts/DejaVuSans-LICENSE.txt` (Bitstream Vera with public-domain changes). `src/fonts/README.md` says how it was made.
