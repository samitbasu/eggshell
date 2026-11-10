# Drag and drop: spike findings (#71)

Status: spike, 2026-11-10. Spike code is not for merge as-is. Everything below was run against egui/eframe/kittest 0.37.0 (winit 0.30.13) unless marked **open** (not run) or **read** (from source only).

## What was built

| Scenario | Example | Tests (`tests/it/`) |
|---|---|---|
| 1 Reorder list, live, FLIP, Esc, release outside, Alt+arrows | `examples/dnd_reorder.rs` | `dnd_reorder.rs` (7) |
| 2 List to zone, compatible/incompatible payload, drop position | `examples/dnd_zones.rs` | `dnd_zones.rs` (3 of 5) |
| 3 OS files on zones, hint, accept/refuse by extension | `examples/dnd_zones.rs` | `dnd_zones.rs` (2 of 5) |
| 4 Gallery: 1/2/3 columns, 2D reorder, scroll, ghost, outside drop | `examples/dnd_gallery.rs` | `dnd_gallery.rs` (13) |

Shared: `examples/common/dnd.rs` (`insertion_index`, `capture`, `paint_ghost`, `paint_chip`) and `FrameRig` / `frame_harness` in `tests/it/support/mod.rs` (a harness whose frame body is the example's own `frame`, needed because the stock `Rig` draws `Placed` itself and cannot run work after the draw or inside a scroll area).

No file under `src/` changed. Every experiment ran on today's public API.

## Headline

1. **egui's DnD payload API carries all of it**, with one sharp edge: `Response::contains_pointer` / `dnd_hover_payload` / `dnd_release_payload` answer from the *previous* frame's widget rects. With live reordering that thrashes (see Hit-testing). The fix needs no egui change: one resolver, run after `draw`, over this frame's final layout rects.
2. **No core change is required to run the four scenarios.** Core changes are needed to make them *good* and cheap: see "Core changes".
3. A **ghost** was reached, from public API only, by copying the shapes the source painted (heuristic by rect). The exact version needs a per-node shape range from the draw walk (#35).
4. Insertion index is **one function** (`insertion_index`) shared by in-app reorder and outside file drops.

## Questions in #71

### Does egui's payload API carry the scenarios?

Yes. `dnd_set_drag_payload(key)` on `drag_started()` in `on_response`; `DragAndDrop::payload::<T>(ctx)` anywhere (build, draw, after draw) for "what is dragged"; egui clears it on Esc at the start of the pass and on release at the end of the pass (so it is still readable on the release frame, inside `draw`). Typed payloads give the "incompatible shows nothing" rule for free (`dnd_zones::an_incompatible_payload_shows_no_feedback_and_is_not_delivered`).

What is missing, none of it in egui's job description: a *resolver* (which target wins), the drag image, repaint after draw-time state changes (egui does not repaint after a handler mutates the app; every handler here calls `request_repaint`), cancel on focus loss, and OS-drag pointer position (below).

### Build-time state and lag

- "What is being dragged" is readable **at build with no lag and no app state**: `DragAndDrop::payload::<ItemKey>(b.ctx())` (`dnd_reorder::row` dims the lifted row from it).
- "What it is over" is computed after draw from the final rects, so a highlight built from it is **one frame behind** (`dnd_zones`: `State::hover` set in `update`, read at the next build). Reorder is also one frame behind: the move happens in frame N's post-draw, the new order is built in N+1 and FLIP slides it.
- One frame of lag is acceptable for highlights, gaps and placeholders (tests run at 50 ms per step and the gap, hint and slide all arrive within one step). It would be visible only as a flash on the very first frame of a drag. A zero-lag highlight is possible in a draw closure (`ui.node_response().contains_pointer()` paints directly), at the cost of bypassing the style system: that is the case for a `drag_over` paint variant (below), which is resolved at draw time next to `hover`.
- State that must survive the gesture (original order for Esc, captured ghost) lives in the app, created in the `drag_started` handler.

### Identity across rebuilds

egui keeps the drag as long as the dragged widget id is re-registered every frame. Bullseye ids are key paths, so a row moved to another index keeps its id and its drag: `dnd_reorder::dragging_a_row_reorders_live_and_keeps_the_drag_under_a_new_position` (the row moves 0 -> 2 -> 3 in one drag; payload and `dragged_id` persist). Guarantees bullseye must keep: (1) the source is `div_id`/`span` with a data key, never position; (2) it declares `sense_drag` on itself (a child cannot start the drag for its parent; a press on the row's label starts the row's drag because text inside a sensing node is not selectable: `a_press_on_the_label_starts_the_row_drag`); (3) it is built every frame of the drag. **Open:** making the source `inert` mid-drag, and removing the source from the tree mid-drag (the payload would outlive its source until release; the app must treat a missing key as a cancel).

Note: a drag-only node did not take focus on press in the harness; `Tab` did. Keyboard reorder is therefore reached by Tab, not by click.

### Hit-testing targets

- **Do not use per-target `contains_pointer` for live reorder.** Evidence: with a pointer parked over row 2, order alternated `[1,2,0,3]` / `[1,0,2,3]` every frame. After the move, row 2 sits at index 1 but egui still holds its previous-frame rect (under the pointer), so it claims the pointer and moves the dragged row back. This is a lag, not a bug in layout: layout rects are final at once, egui's hit set is one frame old.
- **Layout vs painted rect (FLIP).** Targets are the **layout (final) rects**. Painted rects lag during the slide, and a target that followed the painted rect would move under a stationary pointer and flicker. `dnd_gallery::layout_rects_are_final_while_painting_slides` shows rects final in the first frame after a move while the painted box is still near the old position, and settles within the animation.
- **One resolver, encoded once.** `common::dnd::insertion_index(rects, pos)`: the index of the first item that starts below the pointer, or is on the pointer's row (vertical span, or the gap above it) with its centre to the right of the pointer; else the end. Rects are the items' final rects with the dragged item excluded, in order. It is the vertical-list rule when there is one column, and the wrap rule otherwise. Used by `dnd_reorder::resolve`, `dnd_gallery::State::index_at` (in-app reorder *and* outside file drops, same gap), tested in `dnd_gallery::drop_position_gives_the_insertion_index` (left/right of centre, end of row, gap between rows, below all). The rule is stable under reorder because the dragged item is excluded: the result is the same before and after the move.
- Equal-sized items only so far. Rows of differing heights need a hysteresis rule (otherwise a tall item swaps back and forth around its own centre). **Open.**
- **Zones** use `State::zone_at(pos)`: the last zone in paint order whose final rect holds the pointer, used for in-app payloads and OS files alike (`dnd_zones`).
- **Nested targets: open.** egui's `contains_pointer` is true for every widget under the pointer, so nested targets all claim it; the production resolver must pick the innermost (last registered, since registration is pre-order) among targets that accept the payload.

### The drag image

- **Ghost reached.** `common::dnd::capture` copies, on the first frame of the drag, every shape on the layer whose bounds lie inside the source's rect; `paint_ghost` re-adds them translated to the pointer on a `Tooltip`-order layer. `dnd_gallery::the_ghost_is_the_panels_own_shapes_following_the_pointer` finds a panel-sized box centred on the pointer, none in chip mode.
- Capturing on the first frame matters: the next build dims the lifted source, and captured shapes have the opacity baked in. A real per-node record (#35) should store shapes with the node's opacity factored out.
- Limits of the heuristic: it takes shapes by rect containment, so a shadow or ring outside the rect is lost, and a sibling whose bounds fall inside the source's rect is captured with it. **What the exact ghost needs from the core:** `draw` knows, per node, the start and end `ShapeIdx` of everything it painted (own shapes plus children, clipped). Exposing that per identified node (for example `Placed::draw` returning, or a draw-time callback receiving, `HashMap<Id, Range<ShapeIdx>>`, or a post-order `NodeHandle::on_painted`) makes capture exact and is the same data #35 needs for exit transitions. Until then the rect heuristic is good enough to see.
- The chip (`paint_chip`) is ten lines and needs nothing; it is the fallback and is what the reorder example uses by default (G toggles).
- The image is painted on `LayerId::new(Order::Tooltip, ..)` with a screen-wide clip, so it is above the scroll clip and above every ordinary layer. Above overlays: see M6.

### Scroll (M5)

Built the minimum: egui's `ScrollArea::vertical()` hosting the bullseye build/layout/draw (`dnd_gallery::frame`). It works, with two costs: the layout root area's height is last frame's content height (a one-frame lag on the first frame and after a column change), and bullseye has no idea about the viewport except `ui.clip_rect()`.

- **Auto-scroll near the edges** (`edge_speed`: linear in the 48 px band, 600 pt/s at the edge; `ui.scroll_with_delta` each frame while dragging; `dragging_near_the_bottom_edge_scrolls...`). **Whose job:** the drag's policy (speed, band, which payloads), but the container must make it *possible*: expose the viewport rect and a `scroll_by(delta)` / `scroll_to_rect`, and request repaint while scrolling. Both reorder and file hover use the same function.
- **Wheel during a drag** scrolls with no work (origin 8 -> -142 in `the_wheel_scrolls_during_a_drag...`, payload kept). The container must not swallow or ignore wheel while a drag is live.
- **Insertion index in content coordinates.** `State::index_at` subtracts the content origin (the grid node's rect min) from both rects and pointer before applying the rule. Since the rects are this frame's, the answer is equal in screen space; content coordinates are what a *retained* answer (a drop recorded for later, a persisted order) needs. Scrolling mid-drag re-runs the resolver every frame, so the index follows the content (`outside_drop_on_a_scrolled_away_part_via_auto_scroll`: a file hovering at the bottom edge ends with a preview index >= 9, and the drop lands there).
- **FLIP vs scroll.** Scrolling alone does not animate: `scrolling_alone_does_not_slide_panels` checks every checked panel is painted at its layout rect after a wheel scroll. Layout animation measures against the root rect (content origin), which moves with the scroll. M5 must keep that property: the scroll container must translate the root, not re-layout with a different origin per child (#55's note).
- **Ghost above the clip:** yes, it is on its own layer (see above).
- **Outside drop onto a scrolled-away part:** works through auto-scroll for a file hover provided the pointer position is known during the OS drag, which is not guaranteed (below).

**M5 must provide:** (1) viewport rect and content origin readable by the app and by the draw closures; (2) programmatic scroll (`scroll_by`, `scroll_to_rect`, and a way to request repaint); (3) wheel handled while another widget drags; (4) layout told the viewport size so a node can be `h_full` and content height is right on the first frame (here: one frame late); (5) the clip applied to hit-testing as well as painting; (6) the content origin stable for `animate_layout`.

### Overlays (M6)

**Open, not built** (no overlay exists). Read from egui: layers are ordered Background < Middle < Foreground < Tooltip < Debug; the drag image on Tooltip is above any popover or sheet. A target under a popover must not claim the pointer: the resolver has to consult egui's layer ordering (`ctx.layer_id_at(pos)`), not just rects. M6 must: (1) let an overlay be a drop target with its own layer so the resolver can compare layers; (2) draw overlays below Tooltip; (3) say whether a dragging payload dismisses a popover on outside click (egui treats the held pointer as a click-away on release, which would close a popover at drop).

### Keyboard and accesskit

- Alt+Up/Alt+Down moves the focused row (`alt_arrows_move_the_focused_row_without_a_pointer`, focus follows the key). It needs focus on the row (Tab) and `consume_key` so the arrows do not also move focus. A modifier-free alternative (Space to lift, arrows, Space to drop) fits the same state machine and is **open**.
- accesskit (0.25): no drag-and-drop role or action; only `CustomAction { id, description }`. Expose "Move up" / "Move down" (and "Move to ...") as custom actions on the source and run the same move as the keys. Whether egui 0.37 routes custom actions to the app is **open**.

### Cancel

| Case | Result |
|---|---|
| Esc | egui clears the payload at pass start and consumes the key; the widget's drag ends the same frame (`drag_stopped` with no payload). The app restores the original order (`escape_restores_the_original_order`, `escape_cancels_a_2d_reorder`). Moving further with the button still held does not reorder again. A `drag_stopped` with no payload is the cancel signal; a release with a payload is a drop. |
| Release outside the list/viewport | App restores (`releasing_outside_the_list_cancels`). egui clears the payload by itself. |
| Window loses focus (`Event::WindowFocused(false)`) | **egui does not cancel**: payload and gesture both stay (`window_focus_loss_does_not_end_the_drag_by_itself` prints `payload=true gesture=true`). A release outside the window may never arrive. Production: clear the payload and restore on focus loss. Real-window behaviour of the pointer-up outside the window is **open**. |

### OS file drops

- Native, what is read of egui-winit 0.37 / winit 0.30: `HoveredFile(path)` events fill `raw.hovered_files` (path only), `DroppedFile` fills `raw.dropped_files`; winit 0.30 has **no position events during an OS drag**, so the pointer position egui holds is whatever it was before the drag entered (on some platforms it updates, on others not). **Open: verify by hand** on X11, Wayland, macOS, Windows that `ctx.pointer_latest_pos()` follows the cursor during an OS drag. If it does not, per-zone hints and drop positions are impossible natively until winit's drag-position events arrive (a newer winit than egui pins), and the fallback is a single whole-window hint.
- Web (read of eframe 0.37 `web/events.rs`): `dragover` fills `hovered_files` with **mime only** (the names are empty in `dragover`, so accept-by-extension is impossible while hovering; mime filtering is the only pre-drop test); `drop` fills `dropped_files` (name, bytes via `bytes_async`). **Neither handler updates the pointer position**, so on the web the zone under the drag is unknown. Not run in a browser; `cargo check --target wasm32-unknown-unknown --examples` passes in CI for all four examples.
- Simulated in the harness through `Harness::input_mut().hovered_files` (persists across steps until cleared) and `.dropped_files` (consumed by the next step): `dnd_zones::files_hint_...`, `dropped_files_are_accepted_or_refused...`, `dnd_gallery::outside_files_show_a_gap_and_insert_at_the_pointer`, `outside_files_of_the_wrong_type...`. These prove bullseye's routing given a pointer position; they do not prove the OS supplies one.
- Routing: bullseye needs nothing in the core beyond the resolver; the app reads `ctx.input(|i| i.raw.hovered_files)` at build (no lag; the files do not depend on layout) and resolves the zone after draw.

## Core changes

None were required to run the experiments. For production:

| Change | Reason |
|---|---|
| `NodeHandle::on_drag_source(payload)` / `.on_drop::<T>(accept, handler)` sugar over `on_response` | The handler body above is the same in every use: set payload on `drag_started`, request repaint, route cancel. |
| An active-gesture model: `Dragging { payload, source, started }` owned by `LayoutState`/ctx, with Esc, focus loss, release-outside, vanished-source all folded into one `Ended { Dropped, Cancelled }` | Cancel was spread across three signals (Esc via `drag_stopped` with no payload, release position, focus loss which egui does not signal). |
| One target resolver in the library: final rects of targets in `Placed`, the innermost target accepting the payload type wins, layer-aware | Per-widget `contains_pointer` is a frame stale and not exclusive. CLAUDE.md: encode priority once. |
| `drag_over` paint variant (draw-time, with `transition`), plus `hover` variants keep working during a foreign drag | egui's `hovered()` is false for every other widget while one drags, so `hover(..)` styles go dead during a drag (read from egui source); a `drag_over` resolved from the resolver avoids the one-frame build-time lag. |
| Per-node painted shape range exposed after `draw` (shared with #35) | Exact ghost; replaces rect-containment capture. |
| Repaint requested when a draw-time handler changes app state (or a documented `ctx.request_repaint()`) | Every handler had to remember it. |
| Test support: `FrameRig`/`frame_harness` (added here) so tests can run post-draw work and scroll areas | The stock `Rig` cannot. |
| M5: viewport/content-origin/scroll-by/hit-clip (above); M6: overlay layers as targets (above) | Needed by auto-scroll and layered targets. |

## Proposed production API sketch

```rust
// Source: any identified node that senses drag.
b.div_id(key, tw().sense_drag().drag_over(|p| p.ring()), |b| ..)
    .drag_source(Item(key))                       // payload: Any + Send + Sync
    .drag_image(DragImage::Ghost)                 // or ::Chip(text) or ::None

// Target: accepts by payload type, optionally by predicate.
b.div_id("canvas", tw().sense_hover().drag_over(|p| p.bg(accent)), |b| ..)
    .drop_target::<Item>(|app, drop: Drop<Item>| {
        app.place(drop.payload, drop.position)    // position: content coords, node-local
    });

// Files: same node, a separate payload type.
    .drop_target::<Files>(|app, drop| ..)         // Files { paths, accepts: fn(&Path) -> bool }

// Build-time state, no lag:
let gesture: Option<&Dragging> = b.dragging();    // payload, source id, pointer
// 2D reorder: one resolver, shared by in-app drags and OS files.
fn insertion_index(rects: &[Rect], pos: Pos2) -> usize;

// Outcome of a gesture, delivered once:
enum DragEnd { Dropped, Cancelled(Cause) }        // Cause: Esc | Outside | FocusLost | SourceGone
```

`Drop<T>` carries `payload`, `position` (node-local), `index_hint: Option<usize>` for lists via the shared resolver.

## Issues to file

1. Production DnD: `drag_source`, `drop_target`, active-gesture model, `DragEnd` causes (covers #33, #23).
2. `drag_over` paint variant and hover styles during a foreign drag.
3. Library target resolver (innermost, layer-aware, uses final rects; insertion index helper) with hysteresis for unequal item sizes.
4. Per-node painted shape range from `draw` (#35 and the exact drag ghost).
5. Cancel on window focus loss and release outside the window.
6. Repaint after draw-time state change.
7. M5 requirements: viewport/content origin, `scroll_by`/`scroll_to_rect`, wheel during drag, layout told viewport size, hit-clip, stable origin for `animate_layout`, auto-scroll helper.
8. M6 requirements: overlays as drop targets in their own layers, below the drag image, and what a drag does to light-dismiss.
9. Keyboard and accessibility: Space-to-lift mode, accesskit custom actions for move.
10. OS drag pointer position: verify per platform; web has no position and mime-only hover. Possibly upstream (egui / winit).
11. Nested targets and unequal-height reorder, **open** questions from this spike.

## Not verified here

- Real-window OS file drag on any platform (needs a person with a file manager): run `cargo run --example dnd_gallery` and `--example dnd_zones`, drag a `.png` from the file manager over the window. Check that the placeholder gap follows the cursor in the gallery and that zones hint per zone. If nothing follows the cursor, the pointer position is not updated during OS drags (see above).
- The web build in a browser (`cargo xtask web dnd_zones`); only the wasm32 compile check ran.
- Overlays, nested targets, unequal item heights, inert-source and removed-source mid-drag.
