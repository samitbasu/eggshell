# eggshell

Taffy lays out, egui draws and interacts: the opposite of `egui_taffy`.

The application owns its state, and every frame rebuilds the interface from
it. A nested builder walks the state and emits a taffy tree, with classes in
Tailwind's vocabulary; taffy solves the layout; egui draws it and handles
input. Each identified node has a key path, which is also its egui id, so
egui's state for a widget follows the data it shows.

```rust,no_run
use eggshell_ui::{Builder, LayoutState, tw};

struct App {
    count: u32,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().items_center().gap(2.0).p(4.0), |b| {
        b.button("add", "Add").on_click(|app: &mut App| app.count += 1);
        b.text(tw(), format!("{} clicks", app.count));
    });
}

fn main() -> eframe::Result {
    let mut layout = LayoutState::new();
    let mut app = App { count: 0 };
    eframe::run_ui_native("counter", Default::default(), move |ui, _| {
        egui::CentralPanel::default().show(ui, |ui| {
            let area = ui.max_rect();
            layout.build(ui.ctx(), |b| view(b, &app)).layout(area).draw(&mut app, ui);
        });
    })
}
```

## What's in v0.1

- Flexbox and grid layout, sizing, spacing and text, written as Tailwind
  classes (`tw().flex().gap(2.0).p(4.0)`).
- Paint: backgrounds, borders, radii, rings, shadows, opacity, state
  variants (`hover`, `active`, `focus`, `disabled`, `group-hover`), colour
  transitions and layout slides.
- Controls drawn by the library: button, toggle and toggle group,
  checkbox, switch, radio group, segmented control, slider, select,
  stepper, progress bar, text field, dropdown and context menus, tooltips.
- Scroll containers, virtual lists, regions and collapsibles.
- Overlays: popovers, menus, selects, tooltips, modal dialogs and sheets,
  with dismissal, focus handling and arrow-key navigation (menus, listboxes,
  grids).
- Drag and drop, commands with keyboard shortcuts, and per-layout state
  (`Shell`).
- A style contract (`eggshell_ui::Style`), filled by the
  `eggshell-ui-shadcn` crate with shadcn/ui's styles and theme colours,
  light and dark.
- Testing support: headless layout queries, and continuity checks over
  recorded frames (`eggshell_ui::testing`).
- The library and every example also build for the browser (wasm32).

The crate documentation (`cargo doc --open`) is the reference, with a
section per feature and a testing guide in the `testing` module.
`docs/design.md` is the design brief the library was built from;
`docs/overlays.md` and `docs/dnd.md` record the overlay and drag-and-drop
designs and what was built. `CLAUDE.md` holds the engineering practices.

## Development

Automation lives in the `xtask` crate; run `cargo xtask --help` for the list.

- `cargo xtask ci [--skip STEP]`: everything CI runs (fmt check, clock check, clippy, tests, doctests, rustdoc, wasm build, golden images, fuzz); every step runs, and the failed ones are listed at the end. `--skip` leaves a step out by name (repeatable); GitHub runs the `fuzz` step as a parallel job
- `cargo xtask fmt [--check]`: format the workspace
- `cargo xtask test [-- ARGS]`: all tests and doctests; `ARGS` go to `cargo test`
- `cargo xtask coverage [--lcov]`: coverage via `cargo-llvm-cov` (HTML report, or `lcov.info`)
- `cargo xtask fuzz [--cases N]`: the randomised continuity fuzzers with many cases (500 per test by default, in the lightly optimised `fuzz` profile), the `fuzz` step of `ci`; plain `cargo test` runs 32 per test. Pass a larger `--cases` for a deeper hunt. A failing sequence is shrunk and saved under `tests/proptest-regressions/`, to be committed
- `cargo xtask example [NAME] [--release]`: run an example, or list them
- `cargo xtask loc`: lines of code per source file, without comments, blank lines or `#[cfg(test)]` items
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
