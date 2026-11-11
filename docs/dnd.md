# Drag and drop: design and spike findings

**Status:** in-app drag and drop is implemented (`src/dnd.rs`; rustdoc is the API reference). The sections below are the spike's findings (#71), which the implementation follows. **As built** records where production differs from the sketch. Outside file drops with a live position wait on winit 0.31 (#78).

## As built: where production differs from the spike's sketch

- **No live mutation.** The app's data changes only on drop. During a drag the view builds its preview order from `Dragging::index_hint` / `index_hint_in(target)`, so Esc and release-outside need no undo. The hint reaches the build one frame after the pointer moves. It's a fixed point of its own preview: it can settle on either side depending on the direction of approach, but it doesn't oscillate.
- **`on_drag_end` is on the `Builder`**, not the source node, because a vanished source (`Cause::SourceGone`) has nowhere else to deliver. It delivers `(payload, DragEnd)` once.
- **`drop_target_if(accept, handler)`** filters by payload value. **`drop_fallback()`** marks the target that takes a drop arriving with no position (winit 0.30 outside drops); there, `position` and `index_hint` are `None`.
- **`index_hint` items** are the target's direct children that are drag sources or contain one, minus the dragged one's subtree, so a slot wrapper can hold the source. `insertion_index` splits a full-width item (a list or a single column) at its vertical middle.
- **The source's look while lifted** is the `drag_lifted(|p| ..)` variant. It applies to the live image, or to the node in its slot when the image is a preview or `None`.
- **`DragImage` is `{ Live, None }`.** `NodeHandle::drag_preview(|b| ..)` makes any subtree the image and wins over `Live`. `build` runs only while this source is the one lifted (so the preview first shows the frame after the lift). The library puts the subtree under an absolute (out-of-flow) node that is a sibling of the source, with an id derived from the source's, so neither the siblings' layout nor the target's `index_hint` items change. Its shapes go to the drag layer, unclipped, with its top-left `drag_grab(ctx)` behind the pointer: the live image's anchor, resolved against this frame's layout. A preview that should sit at the pointer offsets its content by `drag_grab`.
- **The chip lives in the shadcn crate**, not the core: `drag_chip(b, text)` builds shadcn's tooltip look (`primary` background, `primary-foreground` text, `rounded-md`, `text-xs`, `px-3 py-1.5`, from the tokens of the current theme), 12 points down and right of the pointer; the `DragChip` trait gives `with_drag_chip(text)` on a `NodeHandle`, one line over `drag_preview`. The core knows nothing about chips.
- **Outside drags:** the pointer position counts as known only after `PointerMoved` since the files appeared, or in the drop pass. A position from before the drag entered is never used, and `drag_over` stays off without one. A refused file drop has no callback; the examples route refusals with a page-wide `Files` target (the innermost accepting target wins).
- **A gesture belongs to one `Placed`.** Targets laid out by another `Placed` on the same context don't see it (cross-`Placed` drops are tracked separately). A gesture whose scope is no longer drawn is cleared.
- **Auto-scroll** is done by the scroll containers themselves (M5): while a drag is live and the pointer is near the edge of a `b.scroll(..)` container, it scrolls by `EdgeScroll`'s velocity (the default band and speed unless the chain sets `edge_scroll(..)` or `no_edge_scroll()`) and requests frames. The innermost container that can move in that direction takes the drag. `EdgeScroll` and `drag_pointer` remain for scroll areas that are not eggshell's.
- **The scroll container is M5's** (see `eggshell_ui::scroll`): the content is a separate root laid out in the same frame at the offset egui holds, so rects are screen rects and `animate_layout` sees only moves within a parent. Nodes outside the viewport are not drawn, but a lifted node, the drag's image and the node with the keyboard focus always are.
- **Library fixes the drag tests forced:** the continuity checker judges a node by its own paint offset, not its ancestors' slides; and `animate_layout` treats only a **size** change of the layout area as a resize, so a scrolled area (whose origin moves) no longer snaps slides.

---

# Spike findings (#71)

Status: spike, 2026-11-10. Spike code is not for merge as-is. Everything below was run against egui/eframe/kittest 0.37.0 (winit 0.30.13) unless marked **open** (not run) or **read** (from source only).

## What was built

| Scenario | Example | Tests (`tests/it/`) |
|---|---|---|
| 1 Reorder list, live, FLIP, Esc, release outside, Alt+arrows | `examples/dnd_reorder.rs` | `dnd_reorder.rs` (7) |
| 2 List to zone, compatible/incompatible payload, drop position | `examples/dnd_zones.rs` | `dnd_zones.rs` (3 of 5) |
| 3 OS files on zones, hint, accept/refuse by extension | `examples/dnd_zones.rs` | `dnd_zones.rs` (2 of 5) |
| 4 Gallery: 1/2/3 columns, 2D reorder, scroll, live drag image, outside drop | `examples/dnd_gallery.rs` | `dnd_gallery.rs` (13) |

Shared: `examples/common/dnd.rs` (`insertion_index`, `paint_chip`, and the superseded `capture` / `paint_ghost`, still used by `dnd_reorder`) and `FrameRig` / `frame_harness` in `tests/it/support/mod.rs` (a harness whose frame body is the example's own `frame`, needed because the stock `Rig` draws `Placed` itself and cannot run work after the draw or inside a scroll area).

No file under `src/` changed. Every experiment ran on today's public API.

## Headline

1. **egui's DnD payload API carries all of it**, with one sharp edge: `Response::contains_pointer` / `dnd_hover_payload` / `dnd_release_payload` answer from the *previous* frame's widget rects. With live reordering that thrashes (see Hit-testing). The fix needs no egui change: one resolver, run after `draw`, over this frame's final layout rects.
2. **No core change is required to run the four scenarios.** Core changes are needed to make them *good* and cheap: see "Core changes".
3. The drag image is **live**: the dragged node itself is painted at the pointer every frame, on a top layer, so its content keeps animating. The core needed two small paint properties, `paint_layer(PaintLayer::Drag)` and `paint_centered_at(Pos2)` (an anchor resolved against this frame's rect at draw time). An earlier snapshot "ghost" (copied shapes) froze its content and is superseded; see *The drag image*.
4. Insertion index is **one function** (`insertion_index`) shared by in-app reorder and outside file drops.

## Questions in #71

### Does egui's payload API carry the scenarios?

Yes. `dnd_set_drag_payload(key)` on `drag_started()` in `on_response`; `DragAndDrop::payload::<T>(ctx)` anywhere (build, draw, after draw) for "what is dragged"; egui clears it on Esc at the start of the pass and on release at the end of the pass (so it is still readable on the release frame, inside `draw`). Typed payloads give the "incompatible shows nothing" rule for free (`dnd_zones::an_incompatible_payload_shows_no_feedback_and_is_not_delivered`).

What is missing, none of it in egui's job description: a *resolver* (which target wins), the drag image, repaint after draw-time state changes (egui does not repaint after a handler mutates the app; every handler here calls `request_repaint`), cancel on focus loss, and OS-drag pointer position (below).

### Build-time state and lag

- "What is being dragged" is readable **at build with no lag and no app state**: `DragAndDrop::payload::<ItemKey>(b.ctx())` (`dnd_reorder::row` dims the lifted row from it).
- "What it is over" is computed after draw from the final rects, so a highlight built from it is **one frame behind** (`dnd_zones`: `State::hover` set in `update`, read at the next build). Reorder is also one frame behind: the move happens in frame N's post-draw, the new order is built in N+1 and FLIP slides it.
- One frame of lag is acceptable for highlights, gaps and placeholders (tests run at 50 ms per step and the gap, hint and slide all arrive within one step). It would be visible only as a flash on the very first frame of a drag. A zero-lag highlight is possible in a draw closure (`ui.node_response().contains_pointer()` paints directly), at the cost of bypassing the style system: that is the case for a `drag_over` paint variant (below), which is resolved at draw time next to `hover`.
- State that must survive the gesture (original order for Esc, the grab offset) lives in the app, created in the `drag_started` handler.

### Identity across rebuilds

egui keeps the drag as long as the dragged widget id is re-registered every frame. Eggshell ids are key paths, so a row moved to another index keeps its id and its drag: `dnd_reorder::dragging_a_row_reorders_live_and_keeps_the_drag_under_a_new_position` (the row moves 0 -> 2 -> 3 in one drag; payload and `dragged_id` persist). Guarantees eggshell must keep: (1) the source is `div_id`/`span` with a data key, never position; (2) it declares `sense_drag` on itself (a child cannot start the drag for its parent; a press on the row's label starts the row's drag because text inside a sensing node is not selectable: `a_press_on_the_label_starts_the_row_drag`); (3) it is built every frame of the drag. **Open:** making the source `inert` mid-drag, and removing the source from the tree mid-drag (the payload would outlive its source until release; the app must treat a missing key as a cancel).

Note: a drag-only node did not take focus on press in the harness; `Tab` did. Keyboard reorder is therefore reached by Tab, not by click.

### Hit-testing targets

- **Do not use per-target `contains_pointer` for live reorder.** Evidence: with a pointer parked over row 2, order alternated `[1,2,0,3]` / `[1,0,2,3]` every frame. After the move, row 2 sits at index 1 but egui still holds its previous-frame rect (under the pointer), so it claims the pointer and moves the dragged row back. This is a lag, not a bug in layout: layout rects are final at once, egui's hit set is one frame old.
- **Layout vs painted rect (FLIP).** Targets are the **layout (final) rects**. Painted rects lag during the slide, and a target that followed the painted rect would move under a stationary pointer and flicker. `dnd_gallery::layout_rects_are_final_while_painting_slides` shows rects final in the first frame after a move while the painted box is still near the old position, and settles within the animation.
- **One resolver, encoded once.** `common::dnd::insertion_index(rects, pos)`: the index of the first item that starts below the pointer, or is on the pointer's row (vertical span, or the gap above it) with its centre to the right of the pointer; else the end. Rects are the items' final rects with the dragged item excluded, in order. It is the vertical-list rule when there is one column, and the wrap rule otherwise. Used by `dnd_reorder::resolve`, `dnd_gallery::State::index_at` (in-app reorder *and* outside file drops, same gap), tested in `dnd_gallery::drop_position_gives_the_insertion_index` (left/right of centre, end of row, gap between rows, below all). The rule is stable under reorder because the dragged item is excluded: the result is the same before and after the move.
- Equal-sized items only so far. Rows of differing heights need a hysteresis rule (otherwise a tall item swaps back and forth around its own centre). **Open.**
- **Zones** use `State::zone_at(pos)`: the last zone in paint order whose final rect holds the pointer, used for in-app payloads and OS files alike (`dnd_zones`).
- **Nested targets: open.** egui's `contains_pointer` is true for every widget under the pointer, so nested targets all claim it; the production resolver must pick the innermost (last registered, since registration is pre-order) among targets that accept the payload.

### The drag image

- **Live (the chosen design).** While lifted, the dragged node is painted **at the pointer, every frame**: `paint_layer(PaintLayer::Drag)` moves its subtree's shapes to a top `Tooltip`-order layer, unclipped by the scroll area and above the toolbar, and `paint_centered_at(pointer - grab)` places it. Because the node is rebuilt every frame, the image *is* the live widget, so its telemetry trace keeps moving (`the_live_image_keeps_moving_under_the_pointer`). Layout is untouched, so the slot keeps its place and shows an outline (`the_slot_shows_an_outline_while_the_panel_is_lifted`).
- **Anchor in screen space, resolved at draw time.** The first version painted the lifted node at an offset from its slot computed at build time from *last* frame's slot. On the frame a live reorder moved the slot, the image jumped by the slot's move (seen by the user as a tile drawn over column 1). `paint_centered_at` converts the anchor into an offset from **this** frame's rect when the node is drawn, so it cannot lag (`the_lifted_panel_never_lags_its_slot_when_the_order_changes`, which walks the pointer one frame at a time across the reorder). The same fix removed a one-frame trail under auto-scroll.
- **The lifted node does not `animate_layout`**, so it never slides behind the image, and FLIP measures layout positions, so the pointer offset never feeds a slide. On release the panel is opaque in its slot on the release frame (`on_release_the_panel_is_opaque_in_its_slot_with_no_flash`).
- **Superseded: the snapshot ghost.** The first spike copied the source's painted shapes once at drag start (`capture` / `paint_ghost`, by rect containment) and repainted them at the pointer. It froze animated content, lost shadows outside the rect, and baked in the source's opacity. `dnd_reorder` still uses it; production won't. A per-node painted shape range is now needed only by exit transitions (#35).
- **Chip:** the spike painted a hard-coded label (`paint_chip`). Production has no chip in the core: see **As built** (`drag_preview`, and the chip in the shadcn crate).
- The image is painted on `LayerId::new(Order::Tooltip, ..)` with a screen-wide clip, so it is above the scroll clip and above every ordinary layer. Above overlays: see M6.

### Scroll (M5)

Built the minimum: egui's `ScrollArea::vertical()` hosting the eggshell build/layout/draw (`dnd_gallery::frame`). It works, with two costs: the layout root area's height is last frame's content height (a one-frame lag on the first frame and after a column change), and eggshell has no idea about the viewport except `ui.clip_rect()`.

- **Auto-scroll near the edges** (`edge_speed`: linear in the 48 px band, 600 pt/s at the edge; `ui.scroll_with_delta` each frame while dragging; `dragging_near_the_bottom_edge_scrolls...`). **Whose job:** the drag's policy (speed, band, which payloads), but the container must make it *possible*: expose the viewport rect and a `scroll_by(delta)` / `scroll_to_rect`, and request repaint while scrolling. Both reorder and file hover use the same function.
- **Wheel during a drag** scrolls with no work (origin 8 -> -142 in `the_wheel_scrolls_during_a_drag...`, payload kept). The container must not swallow or ignore wheel while a drag is live.
- **Insertion index in content coordinates.** `State::index_at` subtracts the content origin (the grid node's rect min) from both rects and pointer before applying the rule. Since the rects are this frame's, the answer is equal in screen space; content coordinates are what a *retained* answer (a drop recorded for later, a persisted order) needs. Scrolling mid-drag re-runs the resolver every frame, so the index follows the content (`outside_drop_on_a_scrolled_away_part_via_auto_scroll`: a file hovering at the bottom edge ends with a preview index >= 9, and the drop lands there).
- **FLIP vs scroll.** Scrolling alone does not animate: `scrolling_alone_does_not_slide_panels` checks every checked panel is painted at its layout rect after a wheel scroll. Layout animation measures against the root rect (content origin), which moves with the scroll. M5 must keep that property: the scroll container must translate the root, not re-layout with a different origin per child (#55's note).
- **Drag image above the clip:** yes, the lifted node paints on the drag layer, unclipped by the scroll area (see *The drag image*).
- **Outside drop onto a scrolled-away part:** works through auto-scroll for a file hover provided the pointer position is known during the OS drag, which is not guaranteed (below).

**M5 provides** (built; see `eggshell_ui::scroll`): (1) `scroll::viewport`, (2) `scroll_by` / `scroll_to_rect` / `scroll_into_view`, edge auto-scroll with repaint, (3) the wheel is egui's and ignores drags, (4) the content is solved in the frame it is built, (5) the viewport clips hit testing, (6) the content root moves, nothing is re-laid out. The requirements it was built to:

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
- Simulated in the harness through `Harness::input_mut().hovered_files` (persists across steps until cleared) and `.dropped_files` (consumed by the next step): `dnd_zones::files_hint_...`, `dropped_files_are_accepted_or_refused...`, `dnd_gallery::outside_files_show_a_gap_and_insert_at_the_pointer`, `outside_files_of_the_wrong_type...`. These prove eggshell's routing given a pointer position; they do not prove the OS supplies one.
- **Verified by hand (2026-11-10), winit 0.30:**
  - **macOS:** the drag cursor shows, there's **no position during the hover** (no live gap), and **the drop lands where released**.
  - **Linux/Wayland (Fedora):** nothing is delivered; winit 0.30 implements file DnD only in its X11 backend (`platform_impl/linux/x11/event_processor.rs`).
  - winit **0.31** replaces these with `DragEntered`/`DragMoved`/`DragDropped`/`DragLeft`, carrying the pointer position, on Windows, Wayland and macOS (winit #4571). Tracked as #78; adopt when eframe moves.
- Routing: eggshell needs nothing in the core beyond the resolver; the app reads `ctx.input(|i| i.raw.hovered_files)` at build (no lag; the files do not depend on layout) and resolves the zone after draw.

## Target feedback: two levels

Whether a target can tell *the cursor is over me* depends on the drag's source and the platform. *A compatible drag is in progress* is always known. So target feedback has two levels, styled once by the app; each level lights up wherever the platform supplies it:

| Level | Meaning | In-app drag | Outside drag, winit 0.30 | Outside drag, winit 0.31 |
|---|---|---|---|---|
| **accepts** | a drag this target would take is in progress anywhere | yes | yes (files are hovering; types known except on the web, where only MIME types are) | yes |
| **targeted** | that drag is over this target, so a release here drops here | yes | **no** (no position during the hover) | yes (`DragMoved`) |

- Paint variants: `drag_accepts(|p| ..)` and `drag_over(|p| ..)`. `drag_over` applies on top of `drag_accepts` (e.g. a dashed outline on every accepting zone, and a filled ring on the targeted one).
- **No guessed target:** when the position is unknown, nothing is shown as targeted. A guess, such as the last pointer position before the drag entered, is worse than none.
- **The drop still lands correctly** where the platform gives a position at release (macOS). Where it gives none, the drop goes to a declared fallback (e.g. append to the list) and the new item is scrolled into view, so a drop is never silent.
- Apps don't change when eframe moves to winit 0.31: `drag_over` simply starts lighting up for outside drags.

## Core changes

None were required to run the experiments. For production:

| Change | Reason |
|---|---|
| `NodeHandle::on_drag_source(payload)` / `.on_drop::<T>(accept, handler)` sugar over `on_response` | The handler body above is the same in every use: set payload on `drag_started`, request repaint, route cancel. |
| An active-gesture model: `Dragging { payload, source, started }` owned by `LayoutState`/ctx, with Esc, focus loss, release-outside, vanished-source all folded into one `Ended { Dropped, Cancelled }` | Cancel was spread across three signals (Esc via `drag_stopped` with no payload, release position, focus loss which egui does not signal). |
| One target resolver in the library: final rects of targets in `Placed`, the innermost target accepting the payload type wins, layer-aware | Per-widget `contains_pointer` is a frame stale and not exclusive. CLAUDE.md: encode priority once. |
| `drag_accepts` and `drag_over` paint variants (draw-time, with `transition`; see *Target feedback*), plus `hover` variants keep working during a foreign drag | egui's `hovered()` is false for every other widget while one drags, so `hover(..)` styles go dead during a drag (read from egui source); variants resolved from the resolver avoid the one-frame build-time lag, and the two levels degrade cleanly where the platform gives no position. |
| `paint_layer(PaintLayer::Drag)` plus `paint_centered_at(Pos2)` (built in this spike) | The live drag image is the real node painted at the pointer, anchored in screen space at draw time, so it can't lag a moving slot or freeze. The per-node painted shape range is now needed only by #35. |
| Repaint while auto-scrolling or with the pointer held still mid-drag | egui already repaints after every input frame (#63), so handlers need nothing; only motion without input (auto-scroll) must request frames. |
| Test support: `FrameRig`/`frame_harness` (added here) so tests can run post-draw work and scroll areas | The stock `Rig` cannot. |
| M5: viewport/content-origin/scroll-by/hit-clip (above); M6: overlay layers as targets (above) | Needed by auto-scroll and layered targets. |

## Proposed production API sketch

```rust
// Source: any identified node that senses drag.
b.div_id(key, tw().sense_drag().drag_over(|p| p.ring()), |b| ..)
    .drag_source(Item(key))                       // payload: Any + Send + Sync
    .drag_image(DragImage::Live)                  // default: the node itself, painted at the pointer
                                                  // or ::None (no image; the slot's look carries it)
    .drag_preview(|b| ..)                         // any subtree as the image; wins over Live. The shadcn
                                                  // crate's `with_drag_chip(text)` is one (see As built)

// Target: accepts by payload type, optionally by predicate.
b.div_id("canvas", tw().sense_hover()
        .drag_accepts(|p| p.border_dashed(ring))   // a compatible drag is in progress
        .drag_over(|p| p.bg(accent)),              // ...and it is over this target
    |b| ..)
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
2. `drag_accepts` / `drag_over` paint variants (two-level target feedback) and hover styles during a foreign drag.
3. Library target resolver (innermost, layer-aware, uses final rects; insertion index helper) with hysteresis for unequal item sizes.
4. Per-node painted shape range from `draw` (#35 only; the drag image is the live node).
5. Cancel on window focus loss and release outside the window.
6. Frames during auto-scroll and held-still drags (handlers need nothing; #63).
7. M5 requirements: viewport/content origin, `scroll_by`/`scroll_to_rect`, wheel during drag, layout told viewport size, hit-clip, stable origin for `animate_layout`, auto-scroll helper.
8. M6 requirements: overlays as drop targets in their own layers, below the drag image, and what a drag does to light-dismiss.
9. Keyboard and accessibility: Space-to-lift mode, accesskit custom actions for move.
10. OS drag pointer position: verify per platform; web has no position and mime-only hover. Possibly upstream (egui / winit).
11. Nested targets and unequal-height reorder, **open** questions from this spike.

## Not verified here

- Real-window OS file drag on any platform (needs a person with a file manager): run `cargo run --example dnd_gallery` and `--example dnd_zones`, drag a `.png` from the file manager over the window. Check that the placeholder gap follows the cursor in the gallery and that zones hint per zone. If nothing follows the cursor, the pointer position is not updated during OS drags (see above).
- The web build in a browser (`cargo xtask web dnd_zones`); only the wasm32 compile check ran.
- Overlays, nested targets, unequal item heights, inert-source and removed-source mid-drag.
