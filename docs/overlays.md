# Overlays (M6): design note

**Status:** approved design for M6 (decisions recorded at the end). PR 1 (`place`), PR 2 (roots, floating overlays, stacking, the blocker; see "As built in PR 2" at the end) and PR 3 (dismissal, focus, `Nav`, `root_at` in the resolver; "As built in PR 3") and PR 4 (modals, default and cancel actions; "As built in PR 4") and PR 5 (the stock `Select`, and the focus rules of #150 items 3 and 4; "As built in PR 5") and PR 6b (the stock menus and `CustomChain::role`; "As built in PR 6b") are built; the rest is not. The egui findings F1 and F2 are kept as tests in `tests/it/overlay.rs`; the rest of the prototype evidence lives in the history of the design branch.

Inputs: plan issue #1 (Container API, M6), D18, D19, D22 and §6 and §11 of `design.md`, #21, #35, #85, #40, `dnd.md`, and the code as of origin/main 855eb20 (M5 scroll is built; the owned `Select` uses egui's popup; tooltips use egui's).

## 0. Summary

| # | Question | Design |
|---|---|---|
| 1 | Tree model | An overlay is a **separate root in the same tree, solved and drawn in the same frame**, like scroll content. One `Placed`. Drag and drop across overlays needs nothing; #85 stays open only for apps that use several `Placed`, and is not an M6 dependency. |
| 2 | Stacking | `Band` maps to `egui::Order`; **stacking inside a band is opening order**, with nested overlays always above their parent. **D18's "build order via `move_to_top` every frame" does not work** (finding F1) and is replaced by a **repair step**: each frame the library flags exactly the overlays whose place in egui's order is wrong, which converges. |
| 3 | Placement | One pure function, `place(anchor, size, &Placement, bounds) -> Placed`, flipping to the opposite side, shifting on the cross axis, reporting the room left (for scrolling a tall list). Prototyped with 12 tests. |
| 4 | Dismissal | Judged **at the press**, top-down through the stack, stops at the first overlay that contains the press (its descendants and its anchor count as inside). Esc goes to the topmost dismissible overlay. Nothing is judged while a drag is live. |
| 5 | Focus | Three modes, `Enter`, `Stay`, `None`. Esc blurs focus inside egui (F4), so the library restores focus to the trigger on close. |
| 6 | Modals | Eggshell's own scrim (a full-screen root that senses clicks), plus `Memory::set_modal_layer` for the Tab trap. **Not `egui::Modal`.** Scroll lock is free (F5). |
| 7 | Sheets, #35 | An edge-anchored overlay. Enter and exit motion is part of the overlay chain (`.shown`, `.transition`, `.entrance`, `.on_exited`); the library keeps the overlay present during its exit. **M6 does not need ghosts**; #35 stays independent. |
| 8 | Scroll | Anchored overlays follow their anchor with no lag (same frame), hide when the anchor is clipped away, and do not close on scroll. Point and pointer anchors stay in screen space. |
| 9 | Drag and drop | One resolver for "which root is under the pointer"; drop targets inside overlays work; the drag image stays on top. |
| 10 | Migrations | Move the owned `Select` onto overlays in M6. Tooltips move too, last, with the shadcn look. Stock widgets own their open state. |
| 11 | Look | New style-contract entries `popover`, `menu`, `dialog`, `sheet`, `tooltip`, `scrim`. The entries exist now; every aesthetic value is the user's to pick, from rendered candidates, in the PR that introduces it. |
| 12 | Testing | `Placed` exposes overlays; continuity gains two invariants; the fuzzer gets an `OverlayAction` session. |
| 13 | API | Section 13. |
| 14 | Build order | Section 14: 7 PRs. |

### Findings from the prototypes

| | Finding | Evidence |
|---|---|---|
| F1 | `ctx.move_to_top(layer)` only sets a flag. At the end of the frame egui does a **stable sort by `(order, flagged)`**. Flagging every overlay every frame lifts them above other layers of the same `Order`, but cannot reorder them among themselves: they keep the order they were first shown in. Swapping the declaration order does not restack. egui also never forgets a layer's rank (`Areas::end_pass` sorts and does not prune), so a reopened layer keeps its stale place. | `flagging_every_frame_in_a_new_declaration_order_does_not_restack`; egui's own comment on `wants_to_be_on_top` ("they will keep their relative order") |
| F2 | An `Area` raises itself on a press, which would put a parent above its open submenu unless the layers above it are flagged too. | `a_press_on_a_parent_raises_it_over_its_submenu_unless_all_are_flagged` |
| F3 | `Memory::set_modal_layer` traps **focus** (Tab no longer reaches layers below) but does **not** block the pointer. `egui::Modal` blocks the pointer with a full-screen click-and-drag widget in its layer; without it clicks pass through. The same applies to any overlay panel: a widget-less box lets the pointer through to the page (a painted box now registers a widget of its own, see "Hit testing: every box registers" below), but an overlay root also takes drags, so it keeps a blocker. | `a_modal_layer_blocks_layers_below_it_and_not_those_above` |
| F4 | A focused widget loses focus on Esc **inside egui, at the start of the pass**, before any library code runs. | `escape_blurs_the_focused_widget_before_any_overlay_code_runs` |
| F5 | A screen-sized layer with a blocker widget also takes the wheel from the page below. Scroll lock needs no code. (`rect_contains_pointer` requires `layer_id_at(pos) == layer`.) | `a_screen_sized_modal_layer_takes_the_wheel_from_the_page` |
| F6 | `any_pressed` is true on the press frame only, and `press_origin` stays at the press position through a drag that ends elsewhere. Dismissal judged at the press cannot fire on a release outside. | `a_press_keeps_its_origin_through_a_drag_that_ends_elsewhere` |
| F7 | Every `Order`, including `Tooltip`, allows interaction in egui 0.37 (the doc comment on `Order::Tooltip` is stale). A tooltip overlay can be hovered, so it must be non-sensing by design. | `Order::allow_interaction` |
| F8 | `layer_id_at` and the wheel use a layer's **bounding rect**. One shared layer per band holding scattered overlays would steal the wheel and hover from the page between them. One layer per overlay, with a tight rect, is required. | `Areas::layer_id_at`, `rect_contains_pointer` |
| F9 | kittest's accessibility check rejects a text input without a name; overlays with roles must be named (`named(..)`), and `get_by_role(Role::Dialog)` etc. become the test handles. | harness failure while writing the prototypes |

## 1. The tree model

**Question.** Separate root inside the same tree and frame, or a separate `Placed`?

**Analysis.** Scroll content (§11, `src/scroll.rs`) is already a root of the same `TaffyTree` that is solved right after the tree around it and collected into the same `nodes` vector, so one `Placed` owns it. An overlay has the same shape: its root is a taffy root whose size is MaxContent and whose position comes from an anchor in an already-solved root.

What the same-tree model gives:

- **Anchors are this frame's rects.** `Anchor::Node(id)` reads the final rect from the nodes collected so far. No frame lag, no flicker when the anchor moves (animated layout, scrolling), which is exactly the "overlay first-frame flicker" risk in #1 for the anchor-position half. (The size half is solved by solving the overlay before drawing.)
- **One `nodes` vector, one `by_id`, one `Walk`.** Hit-order, Tab order, text inheritance, the draw closures with `&mut App`, the debug overlay and the continuity recorder all see overlay nodes with no new code path.
- **Drag and drop across overlays just works** (section 9): sources and targets are nodes of one `Placed`, so the gesture, the resolver and `index_hint` are one thing. #85 is moot *for overlays*.
- **A single borrow of the app.** A separate `Placed` per overlay would need the host to call `draw` once per tree with the same `&mut A` and to order the calls itself.

What a separate `Placed` would cost: the host builds, lays out and draws each overlay (so dismissal order, stacking and anchor lookup across trees become the host's problem), and #85's context-wide resolver becomes a hard dependency of M6.

**Consequences** for each concern the brief names:

| Concern | In the same-tree model |
|---|---|
| **#85** | Not needed for overlays. It remains for apps that lay out *panels* as separate `Placed` (unchanged by M6). The dnd resolver is lifted from "walk the root" to "walk every root, in stack order, with blockers" (section 9); that is the same-`Placed` generalisation and does not need the context-wide design. #85 is re-titled "across separate `Placed`" and drops the M6 label. |
| **Hit-testing** | Registration order is egui's hit order inside a layer, and each overlay is its own layer (F8). Each overlay root registers a **blocker** widget first (F3), so the pointer cannot reach the page through a panel; inside a root, every painted box registers its own widget (below). The same "which root is under the pointer" function serves dismissal, drop targets and hover (section 4, section 9). |
| **Culling** | Overlays are not clipped by the base tree's `overflow`, nor culled by the scroll viewports the anchor sits in (that is the point of an overlay). The overlay's clip is the screen. An overlay's own scroll containers cull as today. |
| **Keyboard focus** | Tab order is egui's registration order: base, then overlays in stack order, so Tab walks past the base into the overlays. A modal narrows this with `set_modal_layer` (section 6). Nodes inside overlays get `UiBuilder::id(path)` as everywhere, so ids do not depend on the layer; `focus::focus_within` works for an overlay subtree unchanged. |
| **Continuity checker** | Overlay nodes are recorded like any identified node (path, painted rect, opacity). Opening is a first appearance, closing is an absence at the end, neither is a violation. Two new invariants (section 12). |

**Mechanics (for the build, not the API).**

- `Frame` gets `roots: Vec<RootDecl>`; each root records its kind (base, scroll content, overlay), its parent root, and the scroll leaves inside it. Today `scrolls` is one flat list solved after the base; it has to be partitioned per root so that a scroll container *inside* an overlay is solved after the overlay is placed.
- D19: the overlay's slot is reserved in `roots` **before** its body is built, so overlays declared inside the body (submenus) come later in the list and are solved, placed and stacked after it.
- Layout order: base, then its scroll contents (existing), then each overlay in slot order: solve at MaxContent, resolve the anchor from nodes collected so far, `place`, solve again with a clamped size if the natural size exceeds the room (section 3), solve its scroll contents, collect.
- Draw: the `Walk` is run once per root; each overlay inside its own `egui::Area` (id from the overlay's path, tight rect via `ui.allocate_rect`) so it is its own layer.

**Design.** Same tree, separate root, one `Placed`. #85 is re-scoped to separate `Placed` only.

## 2. Layers and stacking

**Band to `Order`.**

| `Band` | `egui::Order` | Used for |
|---|---|---|
| `Floating` | `Middle` | Non-modal panels that sit with the page: sheets, docked flyouts. |
| `Popup` (default) | `Foreground` | Popovers, menus, selects, dialogs and their scrims. |
| `Tooltip` | `Tooltip` | Tooltips; hover cards. Never sensing. |

The drag image stays on `Order::Tooltip` (`draw.rs`, `Id::NULL.with("eggshell-drag-layer")`). It is above every `Floating` and `Popup` overlay and shares the order with `Tooltip` overlays, so the library flags it last each frame (it is not an overlay and is outside the repair step). Hover-intent tooltips are not opened while a drag is live (the library decides; nothing relies on egui doing so), so the image never competes with one.

**Stacking inside a band (corrects D18).** D18 says "stacking = band, then build order; the driver calls `move_to_top` every frame". F1 shows the second half does not produce the first: egui sorts stably on `(order, flag)`, so the final order inside a band is the order in which the layers were *first shown*. egui never forgets a layer (`Areas::end_pass` is a stable `sort_by_key((order, wants_to_be_on_top))` with no pruning), so a *reopened* overlay keeps its stale rank.

What is guaranteed:

1. **Band, then opening order.** A newly opened overlay is above every overlay already open in its band, including one reopened after an earlier life.
2. **Nested overlays are always above their parent** (D19, "submenus above parents"). A nested overlay is declared inside its parent's body, so it exists only while the parent does and opens after (or in the same frame as) it.

Example of why egui's order alone is not enough: a Settings dialog is opened once and closed. Later a Folder picker modal opens, and from it Settings is reopened as a sibling declaration. egui still ranks Settings below the picker, so it would come back under the picker's scrim.

**The repair step.** The library keeps its own stack `D`: the open overlays in opening order (overlays opened in the same frame in declaration order; a nested overlay always after its parent). Each frame, after drawing:

1. Let `E` be egui's current order restricted to the band. Let `k` be the length of the longest prefix of `D` that appears in `E` in the same relative order.
2. On a press on the overlay at stack index `j` (presses are judged by the library, section 4), use `k = min(k, j)`. egui flags a pressed `Area` itself (F2); flagging everything above it too keeps their order.
3. Flag (`ctx.move_to_top`) exactly `D[k..]`.

egui's stable partition puts the unflagged `D[0..k]` (already correct) below the flagged `D[k..]` (in `E`'s relative order). `D[k]` ends above all of `D[0..k]`, so the correct prefix grows by at least one per frame: the repair never oscillates and converges in at most as many frames as there are misplaced layers. The realistic cases converge in one frame:

- a new overlay: no flags, egui appends it;
- one reopened overlay: flag just it;
- a press on a parent with an open submenu: flag the parent and the submenu;
- two stale overlays reopened in reverse order: flag just the upper.

Only three or more stale overlays reopened in the same frame in reverse order take two or more frames.

egui sorts the order in `Context::end_pass` (`memory.end_pass`) before painting (`graphics.drain(order)`), so the repair is visible in the frame it runs. Hit-testing in that frame used the previous order; that is harmless, since the user cannot click a layer they have not seen.

There is no `.raise()` in the API: restacking an already-open unrelated overlay is not a use case. The repair step is the mechanism, and overlays stay above any plain egui `Area` the host shows in the same order because they are flagged when misplaced and appended when new.

**Modals.** A modal is a `Popup`-band overlay whose root is the scrim. Opening order puts it above what is open. A popover opened from inside a dialog (a select in a form) is nested in the dialog's body, so it is above it and, being above the modal layer, interactive (verified in `a_modal_layer_blocks_layers_below_it_and_not_those_above`).

**Design.** `Band { Floating, Popup, Tooltip }`; D18 is amended to "band, then opening order; nested above parent; a repair step flags what is misplaced". The build PR carries the F1 test so a future egui that changes this is noticed, plus the F2, reopen-as-sibling, press-on-parent and convergence tests (section 14).

## 3. Placement

**Anchors.**

```rust
pub enum Anchor {
    /// An identified node's final rect this frame: `s.id_of("trigger")`.
    Node(egui::Id),
    /// A point in screen space (a context menu): a zero-size rect.
    Point(egui::Pos2),
    /// A rect in screen space (the canvas computes it during build).
    Rect(egui::Rect),
    /// The pointer position when the overlay opened; fixed afterwards.
    Pointer,
    /// The screen (or the boundary): sheets and other edge overlays.
    Screen,
}
```

`D18` listed `Free` and `Rect`; `Free` is `Point` with `Placement::at`, and is dropped. `Pointer` is resolved once, on the first frame the overlay is present (stored in egui temp data under the overlay id), so a tooltip or context menu does not chase the cursor (cursor-following is a different feature and is not v0.1).

**Placement.**

```rust
pub enum Side { Top, Bottom, Left, Right }
pub enum Align { Start, Center, End }
pub enum Flip { Never, ToOpposite }

pub struct Placement {
    side: Side, align: Align,
    gap: Distance,      // between anchor and overlay; negative overlaps (submenu)
    shift: Distance,    // along the anchor's edge (Radix alignOffset)
    flip: Flip,
    margin: Distance,   // kept clear of the boundary
}
// Presets: Placement::below() (select, popover), ::menu() (context menu: At a point),
// ::submenu() (Right, Start, gap -4, shift -4, flips Left), ::tooltip() (Top, Center, gap 4), ::edge(Side) (sheet)
```

`Distance` is a signed-length newtype (the project's rule against bare `f32` in signatures; `Tw` classes are exempt but `Placement` is not a `Tw`).

**The pure function** (prototype in `tests/it/overlay_placement_prototype.rs`; moves to `src/overlay.rs`):

```rust
fn place(anchor: Rect, size: Vec2, p: &Placement, bounds: Rect) -> Placed;

struct Placed {
    rect: Rect,
    side: Side,            // the side it ended on (a flip changes it)
    available: Vec2,       // the most the overlay may measure here: main axis = room on `side`, cross = bounds
    anchored: Anchored,    // Attached | Detached (the anchor lies wholly outside `bounds`)
}
```

Rules (each pinned by a test):

1. Put the overlay adjacent to the anchor on `side`, `gap` away, aligned along the anchor's edge by `align`, moved by `shift`.
2. **Flip** to the opposite side when the preferred does not fit and the opposite fits, *or* neither fits and the opposite has more room. `Flip::Never` stays.
3. **Shift (clamp)** into `bounds.shrink(margin)` on both axes: the cross axis is the usual shift; on the main axis it only matters when nothing fits (the overlay then covers the anchor rather than leaving the screen). An overlay larger than the bounds starts at the bounds' edge.
4. **`available`** is reported so a list taller than the room can scroll: if the natural size exceeds it, the layout solves the overlay a second time with the root's max size clamped to `available` (the same two-attempt pattern as `solve_content`). The flip decision always uses the **natural** size, so the clamp cannot feed back into the choice and flicker (test `the_choice_depends_on_the_natural_size_only...`).
5. **Detached**: the anchor is wholly outside the bounds (see section 8). The overlay is still given a legal rect; the caller hides it.
6. `Anchor::Point` is a zero-size rect. At the screen's corner a `Bottom/Start` point flips to `Top` and shifts left (test `a_point_anchor_is_a_context_menu...`).

**Matching width (selects).** `MatchWidth::Anchor` sets the overlay root's `min_width` to the anchor's width before the first solve (shadcn: `min-w-[var(--radix-select-trigger-width)]`). It is a layout input, not a placement rule, so `place` is unchanged. `max_w` stays a class on the content (`max-w-xs` on tooltips) and is what makes long text wrap, since the overlay is otherwise solved at MaxContent.

**Arrows.** shadcn's popover and menu have none; its tooltip does (a small rotated square). **Not in `place`.** `Placed::side` and the anchor's centre are enough to draw one as a child node later; the pure function would only need to return the arrow's cross offset. Defer until the user picks the tooltip look (section 11).

**Edge placement (sheets, `Anchor::Screen`).** `Placement::edge(side)` is the "inside" variant: the overlay is pinned inside the bounds against `side`, no flip, and fills the cross axis when its root is `h_full`/`w_full`. It is a second small branch of `place` (not prototyped; add to the test matrix as 4 sides x with/without safe margin).

**Test matrix** (the 12 prototype tests, plus the edge variant): every side x alignment with room; gap and shift incl. negative; flip only when needed; neither fits (roomier side, `available`); cross shift in both directions; margin counts against room; overlay larger than bounds; point anchor in the corner; detached anchor; mirror/transpose symmetry (reflection in y maps Bottom to Top, reflection in x swaps Start and End: one table covers what 12 hand cases would); a result is always inside the bounds that can hold it (a sweep of anchors x sides x aligns); flip stability under clamping.

**Design.** `Anchor`, `Placement` and `place` as above. Bounds are the screen by default; `.boundary(Rect)` is deferred until needed (a region-scoped popover can pass `Anchor::Rect` and a clamp rect later).

**As built (#172, `Placement::boundary`).**

- `Placement::boundary(impl Into<InsetBoundary>)`, with `Boundary::{Window (default), Node(Id), Rect(Rect)}` and `Boundary::inset(Sides<Distance>) -> InsetBoundary`; a bare `Boundary` converts with no inset. The inset is on `Boundary` (not on `Placement`) so that a region and its insets travel together: `.boundary(Boundary::Node(canvas).inset(Sides { top: d(8.0), left: d(8.0), right: d(8.0), bottom: d(48.0) }))`. `Sides<T>` is the existing `tw::Sides` (re-exported at the root) with a new `Sides::all`; the other constructors are struct update from `Sides::default()`.
- `place(anchor, size, placement, bounds)` keeps its signature: `bounds` stays the window. The boundary rect (inset, then intersected with the window, then `margin`) is the area the overlay slides into, flips by and measures `available` in. An empty intersection collapses to a point at the window-side corner rather than panicking. `Anchored` still compares the anchor with the window. `Boundary::Node` is resolved by the pipeline (`Placement::with_node_rect`) from this frame's rects, as an anchor is; `place` treats an unresolved node as the window. `pipeline::room_for` (the stock select's build-time scroll decision) resolves it the same way, from the last frame's response.
- **Missing boundary node** follows #139: debug builds panic on the frame the overlay opens, naming the overlay and its declaration (`bounded by a node that doesn't exist`); afterwards (and in release) the lookup is `Missing` and the overlay is dismissed. It is the same `AnchorLookup`, so one rule covers anchor and boundary.
- **Behaviour change under `Flip::Never`** (beside an anchor): `Spot::available` along the main axis is the whole area, not the room on the side, because a never-flipping overlay slides over its anchor and keeps its size. Before, content with no room on its side was clamped to zero height. Nothing in the crate used `Flip::Never` besides `Placement::edge` (unchanged: it already offered the whole area) and the internal refit (which ignores `available`).
- `Default` placements are unchanged: no golden or existing test moved.


## 4. Dismissal (D22, encoded once)

Open state lives on `App`; the library calls `on_dismiss` and the app closes. Everything below is **one function** run after the draw, over this frame's final rects and input; no site decides dismissal on its own.

```rust
fn dismissals(stack: &[OverlayState], input: &FrameInput) -> Vec<usize>   // overlays to dismiss, top-down
```

**Rules.**

1. **Nothing is judged while a drag gesture is live** (dnd's `Dragging`). A drag that starts inside an overlay and ends outside cannot dismiss it, and neither can a drag from the page onto it.
2. **Outside press.** On the frame a pointer button goes down (any button; F6 shows `any_pressed` is the press frame only), walk the stack **top-down**. An overlay *contains* the press if the press position is inside its root rect, or inside any overlay nested in it (declared in its body), or, if it passes presses through (see "What becomes of the press" below), inside its anchor node's rect (so the trigger's own click toggles instead of dismiss-then-reopen). For each overlay in turn: if it contains the press, **stop**; otherwise (and if its `DismissOn` includes `Outside`) dismiss it. A modal also stops the walk: the scrim is the dialog's "outside", so a press on it dismisses the dialog and nothing below. Judged at the press, not the release, as Radix does, so there is no click-versus-drag ambiguity and a release outside never dismisses (F6).
3. **Esc.** The topmost overlay with `Esc` in its `DismissOn`, or with a cancel button (see "Default and cancel actions" below), takes the key and consumes it: it is dismissed, or, if it has a cancel button, the button is pressed instead and `on_dismiss` is not called. Esc during a live drag is egui's (it consumes it at the start of the pass to cancel the drag, so the library never sees it): the drag cancel wins and the overlay stays.
4. **Focus leaving.** If focus was inside the overlay or its anchor last frame and is now on a widget outside both, dismiss (menus and popovers; not dialogs, which trap, nor tooltips). Focus becoming *none* is not "leaving": an Esc blur or a press on a non-focusable area is handled by rules 2 and 3.
5. **Window focus lost** (`WindowFocused(false)`): dismiss every non-modal overlay. This matches dnd's cancel on focus loss.
5a. **Anchor gone** (#139). An overlay anchored to a node (`Anchor::Node`) that is not in the frame is dismissed through `on_dismiss`, and takes no part in the other rules. If it is missing on the frame the overlay opens, it is a programming error (usually an `id_of` from the wrong scope): debug builds panic at the overlay's declaration, release builds dismiss it. `Anchored::Detached` means only clipped or scrolled out of view; M8's virtualised lists will need a carve-out for an anchor outside the realised range.
6. **Scroll of the anchor's container: no dismissal.** The overlay follows (section 8).
7. **Nested overlays.** An outside press on a parent's area closes only the child (the parent contains the press, so the walk stops there). A press on the page closes both, child first. Esc closes the child only.
8. **Activation** (choosing a menu item, a select option) is the *app's* handler setting its state, as in D22's example. The library does not guess that an item "closes the menu".

`DismissOn` is a closed set, not flags: `Default` (Outside, Esc and FocusOut), `EscOnly` (a confirm dialog that must be answered, or an autosuggest whose outside click is the field's own concern), `Never` (tooltips, which the library closes itself).

**What becomes of the press (decided 2026-11-11, #105).** The press that dismisses an overlay is **swallowed** for menus, context menus and `Select` (native and Radix `DropdownMenu`: while open, the overlay's blocker covers the whole screen and draws nothing) and **passes through** for popovers (Radix `Popover`). It is a closed enum on the overlay, `Outside { PassThrough, Swallow }`, with a per-overlay override (`.outside(..)`). A swallowing overlay's anchor is *not* inside it, since the blocker covers the trigger: a press on the trigger closes the menu instead of toggling it. A swallowing overlay ends the walk of rule 2 (the press went to it and no further) except for the overlays it is nested in, which the same press closes after it. An overlay nested in a swallowing one has no blocker of its own: its parent's covers the page, so a press on the parent's items still reaches them.

**Menus are exclusive (decided 2026-11-11, #105).** When any overlay opens (by input or programmatically) while a swallowing overlay is open and the new one is not nested in it, the library calls that menu's `on_dismiss`. Popovers may coexist; nested overlays (submenus, a `Select` in a dialog) are exempt. The library never mutates application state itself (D22 holds); it only reports. A menu item that opens a dialog therefore needs no explicit close. Judged in the same pass as the rules above, from the opening-order stack and `nested_in`. Tooltips do not count as opening, and only an overlay with `DismissOn::Default` is dismissed this way.

**Order of effects in a frame.** Draw runs first (so a click on an item inside runs its handler), dismissal runs second, from the same frame's input. If the same press activates an item and is judged "inside", nothing dismisses; if an item handler closed its own overlay, the dismissal list simply no longer contains it next frame.

**Default and cancel actions (#126, decided in PR 4).** `ButtonChain::default_action()` marks the button Return presses and `.cancel_action()` the one Esc presses. One rule each, encoded once:

- **Esc** is rule 3: it goes to the topmost overlay that takes it, which is one that dismisses on Esc (`Default`, `EscOnly`) or one that has a cancel button, whatever its `DismissOn`. If that overlay has a cancel button, the button's `on_click` runs (as a click, in the next frame) and `on_dismiss` does not; otherwise the overlay is dismissed. A press on a modal's scrim, or outside a popover, is still the dismissal, so `on_dismiss` is the one place that cleans up for every way of leaving. A cancel button that is disabled takes the key and does nothing: the overlay is held open. When no overlay takes Esc it is the window's cancel button's (the base root's), if there is one. Esc goes to the one overlay on top, so with a `Select` open in a dialog the first Esc closes the select and the second presses Cancel.
- **Return** goes to the default button of the nearest root: the overlay (or the window) that holds the focus, else the topmost open overlay, else the window. It is left alone when the focused widget takes Return itself: a multi-line text field (a newline), or a node that senses clicks (a button or any control Return activates; as on the web, and as Radix's `AlertDialog`, whose first focus is Cancel, does). A single-line text field does not (egui drops its focus on Return before the library looks), so Return in it presses the default. A disabled default button does nothing.
- If a root has several buttons of one kind, the first declared is the one.
- The page's default and cancel buttons are not reachable from inside an overlay (the nearest root is the overlay), nor under a modal (it traps the focus).

**Design.** Rules 1 to 8 as one function with a unit-test matrix (stack shapes x press positions x Esc x drag live). `on_dismiss` stays `FnOnce(&mut A)`, as in D22.

## 5. Focus and keyboard

**Focus modes.**

```rust
pub enum Focus {
    /// Move focus into the overlay on open: the first focusable, or `Focus::initial(key)`.
    /// Menus, dialogs, sheets, popovers with fields.
    Enter,
    /// Leave focus where it is: select listboxes and autosuggest, where the trigger or
    /// field keeps focus and the highlight is virtual (aria-activedescendant).
    Stay,
    /// Tooltips.
    None,
}
```

**On close.** The library stores the focused widget when an `Enter` overlay opens. On close, if focus is inside the overlay **or is none** (F4: Esc has already blurred it), it requests focus for the stored widget when that widget is still registered. For a `Stay` overlay opened from a text field (autosuggest) the same rule re-focuses the field after Esc, which egui would otherwise have blurred; this is the case the M6 autosuggest test needs.

**Tab.** Non-modal overlays do not trap Tab; Tab out of one is "focus leaving" and dismisses it. Modals trap (section 6).

**Arrow navigation (WAI-ARIA).** An overlay can declare `Nav`:

| `Nav` | Pattern | Keys |
|---|---|---|
| `Menu` | menu | Up/Down move focus between items (roving focus: items are real focus targets), Home/End, wrap, typeahead (first letter) later. Disabled items are skipped. |
| `Submenu` | menu opened from an item | `Menu` plus Left dismisses it and focuses the parent item. |
| `Listbox` | listbox | Same keys, but focus `Stay`s and the highlight is a virtual active item: the library keeps the active index in the overlay's temp data and applies the item's `highlighted` look. Enter/Space activate the active item. |
| `None` | | The content handles its own keys. |

Items are nodes with an item role (`MenuItem`, `MenuItemCheckbox`, `MenuItemRadio`, `ListBoxOption`); the library collects them from the overlay's subtree in paint order. The existing `Select` already implements the `Listbox` behaviour by hand (`List::show`, `hold_keys`, `step`, `edge`); that code moves into the library and is deleted from `select.rs`. Right (or Enter, or a click) on an item that owns a submenu is the **item's** concern (`s.menu_item(..).submenu(..)` in the stock widgets), because only it knows its submenu's open state on `App`.

**Accesskit.** Roles come from the overlay: `.role(Role::Dialog | Menu | ListBox | Tooltip)`, set with `named(..)` (F9). A modal root additionally sets accesskit's modal flag (`aria-modal`). The trigger gets `expanded` and `has_popup`, as `Select` already sets `expanded`. Item roles are as above. Tooltips are described to the trigger (`aria-describedby`) rather than focused. All of this is what egui already sends for widgets; there is no accessibility work beyond roles and flags.

**Design.** `Focus` and `Nav` as above. v0.1 submenus open on click, Enter and Right only. Typeahead, hover-open and the submenu "safe triangle" (keeping a hover-opened submenu open while the pointer crosses to it) are post-0.1.

## 6. Modals

**`egui::Modal` or our own backdrop?** Own.

What `egui::Modal` does (read from egui 0.37 `containers/modal.rs`, confirmed by the prototypes): an `Area` in `Order::Foreground` with a full-screen child `Ui` sensing click+drag (the pointer blocker), a translucent rect, a framed scope that also senses click+drag so the dialog's own clicks don't reach the backdrop, `Memory::set_modal_layer` (the focus trap), and `should_close()` (backdrop click, or Esc when topmost and no egui `Popup` is open).

Why not use it: the dialog's content is an eggshell subtree in the same `Placed` (section 1), so it cannot be shown through `Modal::show`'s closure without splitting the tree; its Esc rule is tied to egui's popup state, which eggshell overlays do not use; its backdrop colour and frame come from egui's style rather than the style contract; and `on_dismiss` is D22's, not `ModalResponse`'s.

What we keep from it, as the two things it actually does:

```rust
.modal(Modal::Center)            // or Modal::Edge(Side) for a sheet
```

- The overlay's root is a **screen-sized node** with the scrim fill from the style contract and `sense_click_drag` (the blocker, F3). The dialog box is its flex child: `Modal::Center` is `flex items-center justify-center` on that root, `Modal::Edge(side)` is `justify-end`/`items-end` style placement, and the app's classes size the box. There is no placement function for modals.
- The library calls `ctx.memory_mut(|m| m.set_modal_layer(layer))` each frame the modal is mounted (focus trap: Tab and Shift+Tab cycle inside the modal and layers above it; one-frame lag, as egui's, which is why the prototype probes after three steps).
- **Scroll lock** is automatic: the root is screen-sized and is its own layer, so `layer_id_at` returns it everywhere and the page's `ScrollArea` is never "hovered" (F5). Nothing to build and nothing to undo.
- A press on the scrim, or Esc, is dismissal (section 4), unless `DismissOn::EscOnly`/`Never`.
- `Focus::Enter` by default; initial focus can be a named node (`Focus::initial("cancel")`); focus returns to the opener on close.
- Stacked modals: opening order decides (section 2); `set_modal_layer` keeps the topmost, and the lower dialog is inert under the upper scrim.

**Design.** Own scrim, built from the two egui mechanisms above. `Modal` is a presentation of the overlay, not a second type.

## 7. Sheets, drawers, slide-in sidebars (#21) and #35

**API.**

- **Sheet (modal):** `s.overlay("settings").modal(Modal::Edge(Side::Right))`, the box sized by its classes (`w_px(380.0)`, `h_full()`).
- **Sheet (non-modal):** `s.overlay("panel").anchor(Anchor::Screen).placement(Placement::edge(Side::Right))`, in `Band::Floating`, no scrim, the page stays usable.
- **Drawer** (draggable bottom sheet, vaul): the same with `Side::Bottom` plus the app's drag handle (`sense_drag` and a paint-only translate that follows the pointer, #21 part 2). Library-side needs nothing beyond the chain's presence methods and the existing translate; a springing helper can come later. Defer to the #21 PR; it is not an M6 requirement.
- **Pushing sidebar:** a layout pattern, not an overlay. As #21 says: animate the width at build, clip, an example. No M6 machinery.

**Enter and exit motion, and #35.** Overlays are *real nodes of the app's tree* while they exist, so the library can keep one present during its exit. The presence mechanism lives on the overlay chain, and the overlay is always declared:

```rust
// App: settings: Shown   (pub enum Shown { Open, Closed })
s.overlay("settings")
    .modal(Modal::Edge(Side::Right))
    .shown(self.settings)                          // required
    .transition(Duration::from_millis(200))        // optional; absent = appears/vanishes at once
    .ease_out()                                    // optional; the Tailwind default curve otherwise (the existing Tw `Easing`)
    .entrance(Entrance::Slide(Side::Right))        // Fade | Zoom | Slide(Side) | SlideFade(Side); default from the style contract
    .role(Role::Dialog).named("Settings")
    .on_dismiss(|app| app.settings = Shown::Closed)
    .children(|s| sheet_box(s, |s| { /* w_px(380.0).h_full() */ }));
```

- The library owns the phase (`Closed | Entering | Open | Exiting`) under the overlay id. Progress is `src/transition.rs`'s retargetable eased `Progress` (`transition::animate`), so there is no second animation clock; it requests repaints only while moving and is idle otherwise. Reopening mid-exit reverses from the current progress: never both a ghost and a live node.
- `children` is not called while the overlay is fully `Closed`.
- **Exiting:** the overlay is inert (a closing dialog takes no input) but still blocks the page until it is gone. The scrim fades with progress. The box translates by `(1 - progress) x its own size` toward the edge, **resolved at draw time** from the node's final rect (the same trick as `paint_centered_at`, since the size is unknown at build). Entrance is paint-only: layout is final from the first frame.
- `.on_exited(|app| ..)` fires once when `Exiting` reaches `Closed`. It clears a payload the content needs during the exit. Example, a confirm dialog:

```rust
// App: confirm: Shown, confirm_target: Option<ItemId>
s.overlay("confirm")
    .modal(Modal::Center)
    .shown(self.confirm)
    .transition(Duration::from_millis(150))
    .role(Role::Dialog).named("Delete item")
    .on_dismiss(|app| app.confirm = Shown::Closed)
    .on_exited(|app| app.confirm_target = None)
    .children(|s| {
        // reads self.confirm_target: still set while the dialog fades out
    });
```

  The dialog's closing click sets `confirm` to `Closed` and leaves `confirm_target` alone, so the content still has its data for the exit. This boundary is why ghosts (#35) remain for data that is already gone; M6 does not depend on #35.
- Stock widgets carry this internally, with an entrance from the preset.

**Do we need ghosts (#35) for M6?** No. Ghosts exist for nodes that *vanish because their data did* (todo's Remove): there is no state left to keep them mounted from. An overlay's existence is an app flag, so the library can hold it. Ghosts are also the wrong tool for modals: a ghost takes no input, so the page would be clickable through a fading scrim, whereas an exiting overlay keeps blocking. If #35 lands later, nothing here changes; ghosts can additionally serve overlays that the app removes abruptly.

**Design.** Presence on the chain, built with the sheet PR (small, and used by both sheet and dialog). M6 is decoupled from #35; this is stated on #35. The "enter" side needs no machinery beyond the same phase. Motion choice, duration and easing are the user's (section 11).

## 8. Scroll interplay

**An overlay anchored to a node inside a scroll container.**

- **Follow, free.** The overlay is solved after the scroll content, with the content already at this frame's offset (§11: "laid out at the offset egui holds"), so the anchor rect is current. The overlay is placed against it in the same frame; there is no lag to hide and no reposition event to subscribe to. (Floating UI needs `autoUpdate` for this; the same-tree model does not.)
- **Hide when clipped.** If the anchor's *visible* part (its rect cut by its ancestors' clips and the scroll viewport; the same clip chain `dnd::targets` already computes) is empty, `place` reports `Detached` and the overlay is not drawn, takes no input and does not take part in dismissal. Its open state and its place in the stack are kept, so scrolling back shows it again. A partially visible anchor keeps the overlay (placed against its full rect).
- **No dismissal on scroll.** Menus and popovers stay open; the wheel over the *page* while a popover is open scrolls the page, and the popover follows. Native context menus that close on scroll are an app choice (the app can compare the viewport offset, or use `Anchor::Point`).
- **`Anchor::Point` and `Pointer` do not follow.** They are screen-space and fixed, as native context menus are. Content that wants a menu to stay on a content point stores content coordinates and converts each frame with `Viewport::to_screen` (it exists) into `Anchor::Rect`.

**Overlays containing scroll containers.** Supported by construction: the overlay's scroll leaves are solved after the overlay is placed (section 1). A tall list: pass 1 at MaxContent, `place` reports `available`, pass 2 clamps the overlay root's max height to it; the list's scroll container (`grow`, `min_h_0`) takes the clamped height and scrolls. The wheel goes to the overlay under the pointer (its layer rect is tight, F8) and, at the list's end, does *not* chain to the page (the page is another layer), matching native popups.

**Design.** Follow, hide when detached, never close on scroll.

## 9. Drag and drop interplay

- **One `Placed`, one gesture.** A source in an overlay and a target in the page (or the reverse, or both in overlays) are in the same `nodes`. `Dragging`, `index_hint` and `DragEnd` need no change.
- **Resolver.** `dnd::targets` walks from node 0. It becomes a walk over every root in stack order, and a target is *reachable* at pointer `p` only if the topmost root whose **blocker** contains `p` is the target's own root (floating overlay: its root rect; modal: the whole screen; page: everything else). That function, `Placed::root_at(p)`, is the same one dismissal uses (section 4) and the one hover uses, so the three cannot disagree (CLAUDE.md: encode priority once). It replaces `dnd.md`'s suggestion of consulting `ctx.layer_id_at`: the library owns the overlay rects and does not need to ask egui, which also works in the headless harness.
- **Drag image.** On `Order::Tooltip`, above `Floating`/`Popup` overlays, flagged last (section 2). Unclipped.
- **Dragging over an open popover.** The popover is not dismissed (dismissal is press-based, and not judged during a drag). It blocks the targets under it. If it declares a target of its own it is a normal target.
- **Light dismiss and a drag.** Answers `dnd.md` M6 requirement (3): a payload's drop **does not** dismiss. egui's click-away fires on the release; ours fires on the press and is suppressed while a drag is live, so a drop onto a popover or a release outside it is inert. A drag whose source lives in an overlay that is dismissed mid-drag (by a window blur) ends as `SourceGone`; Esc cancels the drag first.
- **Auto-scroll** in an overlay's scroll container works as in the page (`EdgeScroll` is per scroll node).
- **Spring-loaded opening** (hover a trigger during a drag to open it) is post-0.1.

**#85.** Not needed. Keep it open for separate `Placed`, and note that the "which root is under the pointer" function is the part that would be lifted to the context if #85 is built.

`root_at` asks `PlacedNode::blocks_pointer` of the roots; inside a tree, painted boxes take the pointer by registering widgets (see "Hit testing: every box registers").

**Design.** One resolver, `root_at`, shared by dnd, dismissal and hover.

## 10. Migrations

**`Select` (owned): migrate in M6.** The brief in `select.rs` says "until eggshell has overlays of its own". It uses `egui::Popup` with its own open memory, close behaviour, Esc, click-away, width and gap, and hand-painted rows. Moving it gives:

- one dismissal path (an Esc in a dialog closes the select first, then the dialog: correct nesting, which egui's `Popup` and eggshell's dismissal could not agree on);
- the list is eggshell nodes: identified, keyed, inspectable in `Headless` by path, covered by the continuity checker and the debug overlay, themed by the style contract and tested with `get_by_role(ListBoxOption)`;
- flip, clamp and a scrollable list for long option sets (`place`'s `available`); egui's popup has neither the clamp-to-room nor our scroll;
- it works inside a modal and inside a scroll container (anchor follows);
- it deletes `List::show`, `hold_keys`-for-popup and `Popup` plumbing (about 150 lines), replaced by `Focus::Stay` + `Nav::Listbox` shared with autosuggest and the combobox.

**Stock widgets own their open state.** Select's open state stays in egui temp data keyed by the widget id (as it is now, `highlight_id`), and the overlay's `on_dismiss` clears it. The same holds for tooltips and the stock menus (dropdown menu, context menu). D22 is about application-owned overlays; stock widgets own their own. A stock menu closes itself on item activation. There is no controlled mode (`open`/`on_open_change`) in v0.1; it can be added later without breaking anything. State this in the build PR.

**Tooltips: migrate, last, with the shadcn look.** `tipped` uses `Response::on_hover_text`. egui's tooltip is a plain popup frame, so the shadcn tooltip (inverted colours, small radius, arrow, short delay) cannot be had from it. A migrated tooltip: hover-intent state in egui temp data keyed by the trigger id (delay from `Style::interaction.tooltip_delay` and grace from `tooltip_grace_time`, so timing is unchanged), a `Band::Tooltip` overlay, `Focus::None`, never sensing, `DismissOn::Never` (closed by the library when the pointer leaves, on press or Esc, or when a drag starts), described to the trigger. Buys: the style-contract look and arrow, rich content (a label with a `kbd` chip), the same placement and flip, visibility in `Headless`, no stray egui layer. Costs: re-doing the hover-intent timing and the disabled-hover case (`on_disabled_hover_text`), both small. The tooltip is the least coupled migration and goes last.

**Stock menus.** The stock menu widgets, with submenus, are in v0.1 (section 5 for the submenu gesture).

**Design.** `Select` migrates in the same milestone (PR 5). Tooltips migrate in PR 7 with the shadcn look. Stock menu widgets with submenus land in PR 6.

## 11. Look

Look decisions are the user's. The entries below exist now; their values are chosen by the user from rendered candidates, with shadcn's values recommended, in the PRs that introduce them. The note fixes **which style-contract entries exist**, not their values; shadcn's component sources are the reference, as in the `shadcn defaults` rule.

New entries in `Style` (each a `Look` with a rest state, border, radius, shadow, padding, as `SelectStyle::popup` is):

| Entry | Covers | Open choices |
|---|---|---|
| `popover` | popover box | background, border, radius, shadow, padding, default width |
| `menu` | menu box and `item`, `item_destructive`, `separator`, `label`, `shortcut`, check/radio indicators, submenu chevron | item height and padding, highlight colour (focus vs hover), shortcut text style |
| `dialog` | box, header/footer, title and description text, close button | width, radius, padding, footer layout, close button present or not |
| `sheet` | box per side, border side, header/footer | default width, border, shadow |
| `tooltip` | bubble, arrow | **inverted vs popover look**, radius, padding, arrow on or off, delay |
| `scrim` | modal backdrop | colour/alpha (shadcn `bg-black/50`), blur is out of scope (#40, known difference). Built in PR 4 as `Style::scrim`, a colour: shadcn black at 204/255 (80%, darker than shadcn's `bg-black/50` because there is no blur; user's pick from rendered candidates, 2026-11-12), neutral `from_black_alpha(100)` (egui's own modal backdrop). |
| `motion` | open/close duration, easing, entrance per kind | the numbers and curves |

The select's list is drawn from `Style::menu` (PR 6b): `SelectStyle` lost its `popup*` and `item*` fields to `MenuStyle`, so the list and the menus cannot drift.

The shadcn preset crate fills these from the shadcn tokens, as it does for the controls. The **neutral default** (no preset) gets plain values from the egui style (no change of default look without a decision).

**Needs the user's choice, with candidates rendered at the size they will be used beside what surrounds them:** the tooltip look (inverted or popover; arrow or none), menu item highlight (hover vs focus ring), dialog and sheet geometry, scrim alpha, all durations and easings, close-button presence. Each is offered with shadcn's values as the recommended candidate, as rendered images, in the build PRs that introduce them.

**Known CSS differences that touch overlays** (#40): no rounded clipping, so a scrolling list inside a rounded popover needs its own inner radius; no group opacity, so fading a whole overlay fades each shape separately (the entrance fade is a scrim fade plus per-shape alpha; overlapping translucent shapes inside a dialog could show through during the fade). Pre-blend opaque surfaces as the controls already do.

## 12. Testing

**Layers of test.**

- **Pure:** `place` (section 3) and `dismissals` (section 4) are unit-tested with no egui.
- **Headless (`eggshell_ui::testing`)**: layout only, no draw. `Placed` gains `overlays()` (stack order), `overlay(key) -> OverlayRef` with `rect()`, `side()` (to assert flips), `anchored()` and `stack_index()`. Placement flips, `MatchWidth`, the clamp second pass, detached anchors, and the D19 slot order are all asserted here.
- **kittest through `frame_harness`** (the real path: build, layout, draw, dismissal, focus): outside press, Esc, press inside, trigger-press toggles, nested dismissal, drag-from-inside-release-outside, modal blocks the page and traps Tab, focus returns to the trigger after Esc, the autosuggest keeps the field focused while open and a suggestion click applies it, menu arrow navigation, accesskit roles (`get_by_role(Dialog)`), stacking (`layer_id_at` at the overlap, the F1 and F2 tests promoted).
- **Continuity** (`Timeline`): overlay nodes are already in the record. Add two invariants: `OffScreen` (an overlay's painted rect leaves the bounds that can hold it) and `Reflip` (an overlay's side changes between two frames in which neither its anchor rect nor its natural size changed). Existing ones apply: no teleport when the anchor does not move, no double paint (one overlay drawn twice), `Settles` (an open overlay's layout stops changing).
- **Golden/snapshots:** one per overlay kind in the `menu` example, light and dark, behind the `snapshots` feature as the others are; regenerated goldens get reviewed, per CLAUDE.md.

**Fuzz actions** (a new `overlay_session` in `tests/it/fuzz.rs`, same `proptest` shape as `drag_action`):

`OpenMenu(i)`, `OpenSubmenu(item)`, `HoverItem(i)`, `ClickItem(i)`, `PressOutside(pos)`, `ReleaseOutside`, `Esc`, `Tab`, `ShiftTab`, `ArrowDown`, `ArrowUp`, `Home`, `End`, `ScrollAnchorContainer(delta)`, `ResizeWindow(w, h)`, `OpenModal`, `DragFromOverlay(to)`, `Idle(n)`.

Invariants checked after every action: every overlay rect is inside the bounds that can hold it; the stack has no child above a missing parent and no child below its parent; no press reaches the page when a modal is open; after Esc the number of open overlays dropped by exactly one (or a drag was cancelled); focus is inside the topmost `Focus::Enter` overlay or is none or has returned to its trigger; `Timeline::assert_continuous` over the session.

**Test plan from #1** is kept: placement (pure), submenu above parent, Esc and outside click dismiss and inside does not, modal blocks the base, autosuggest keeps the field's focus and applies a click.

## 13. API sketch

The agreed shape: `s.overlay(key).anchor(..).placement(..).on_dismiss(..).children(|s| ..)`. A modal replaces `anchor`/`placement` (a typestate on the chain, so an overlay is never both modal and anchored: the illegal state is unrepresentable rather than a runtime check).

```rust
s.overlay(key)                          // OverlayChain<Floating>
    .anchor(Anchor)                     // required for Floating
    .placement(Placement)               // default Placement::below()
    .band(Band)                         // default Popup
    .match_width(MatchWidth::Anchor)    // default MatchWidth::Content
    .focus(Focus)                       // default Enter
    .nav(Nav)                           // default None
    .dismiss_on(DismissOn)              // default Default
    .role(Role).named("..")             // accesskit
    .shown(Shown)                       // open/closed phase owned by the library (section 7)
    .transition(Duration)               // optional; absent = no motion
    .ease_out()                         // optional easing
    .entrance(Entrance)                 // Fade | Zoom | Slide(Side) | SlideFade(Side)
    .on_exited(|app| ..)                // FnOnce(&mut A), once when Exiting reaches Closed
    .on_dismiss(|app| ..)               // FnOnce(&mut A), D22
    .children(|s| ..);

s.overlay(key).modal(Modal::Center)     // OverlayChain<Modal>: no anchor or placement
    .focus(..).dismiss_on(..).role(Role::Dialog).named("..")
    .shown(..).transition(..).entrance(..).on_dismiss(..).on_exited(..).children(|s| ..);
```

Overlay content is ordinary builder code; it takes the text style in effect at the declaration point, like scroll content. The overlay's own box comes from the content's classes (or, for stock kinds, from the style contract).

### Popover

```rust
// App: filters_open: bool
s.button("filters", "Filters").on_click(|app| app.filters_open = !app.filters_open);
if self.filters_open {
    s.overlay("filters-pop")
        .anchor(Anchor::Node(s.id_of("filters")))
        .placement(Placement::below().align(Align::Start).gap(Distance(4.0)))
        .focus(Focus::Enter).role(Role::Dialog).named("Filters")
        .on_dismiss(|app| app.filters_open = false)
        .children(|s| popover_box(s, |s| { /* fields */ }));
}
```

### Context menu with a submenu

```rust
// App: menu: Option<Menu { at: Pos2, sub: bool }>
// in the canvas node's response handler:  if r.secondary_clicked() { app.menu = r.interact_pointer_pos().map(|at| Menu { at, sub: false }) }
if let Some(menu) = &self.menu {
    s.overlay("canvas-menu")
        .anchor(Anchor::Point(menu.at)).placement(Placement::menu())
        .nav(Nav::Menu).role(Role::Menu).named("Canvas")
        .on_dismiss(|app| app.menu = None)
        .children(|s| {
            menu_box(s, |s| {
                s.menu_item("copy", "Copy").shortcut("Ctrl+C").on_click(|app| { app.copy(); app.menu = None });
                s.menu_item("arrange", "Arrange")
                    .submenu_open(menu.sub)                       // chevron, aria-haspopup, expanded
                    .on_open_submenu(|app| app.menu.as_mut().unwrap().sub = true);
                s.menu_separator();
                s.menu_item("delete", "Delete").destructive().on_click(|app| { app.delete(); app.menu = None });
                if menu.sub {                                      // declared in the parent's body: D19
                    s.overlay("arrange-sub")
                        .anchor(Anchor::Node(s.id_of("arrange"))).placement(Placement::submenu())
                        .nav(Nav::Submenu).role(Role::Menu).named("Arrange")
                        .on_dismiss(|app| app.menu.as_mut().unwrap().sub = false)   // Left or Esc: parent stays
                        .children(|s| menu_box(s, |s| { /* items */ }));
                }
            })
        });
}
```

`menu_item`, `menu_separator` and `menu_box` are stock widgets in the owned set, styled by `Style::menu`; `submenu_open` and `on_open_submenu` are the item's part of the submenu protocol (section 5). The overlay itself only needs `nav`, `role` and the anchor.

### Select / listbox (stock widget, what `Select` becomes)

```rust
s.overlay((slot_key, "list"))
    .anchor(Anchor::Node(trigger_id))
    .placement(Placement::below().gap(Distance(4.0)))
    .match_width(MatchWidth::Anchor)
    .focus(Focus::Stay).nav(Nav::Listbox)       // the trigger keeps focus; the highlight is virtual
    .role(Role::ListBox).named(&name)
    .on_dismiss(close)                           // clears the widget's own open state
    .children(|s| list_box(s, |s| { for option in options { s.list_option(option) } }));
```

### Autosuggest under a focused text field

```rust
if self.suggest_open {
    s.overlay("suggest")
        .anchor(Anchor::Node(s.id_of("city")))
        .placement(Placement::below())
        .match_width(MatchWidth::Anchor)
        .focus(Focus::Stay).nav(Nav::Listbox)
        .dismiss_on(DismissOn::Default)         // press on the field is "inside" (the anchor)
        .role(Role::ListBox).named("Suggestions")
        .on_dismiss(|app| app.suggest_open = false)
        .children(|s| for hit in &self.hits { s.list_option(hit.key, &hit.label).on_click(pick(hit.key)) });
}
```

### Tooltip (declarative; the library owns hover-intent state, so there is no `App` flag)

```rust
s.button("save", "Save").tooltip("Saves the file");                         // as today: text
s.button("save", "Save").tooltip_with(|s| { s.text(tw(), "Save"); s.kbd("Ctrl+S"); });   // rich
// Internally an overlay in Band::Tooltip, Placement::tooltip(), Focus::None, DismissOn::Never, not sensing.
```

### Modal dialog

```rust
if self.confirm.is_some() {
    s.overlay("confirm")
        .modal(Modal::Center)
        .role(Role::Dialog).named("Delete project").focus(Focus::initial("cancel"))
        .dismiss_on(DismissOn::EscOnly)                                    // scrim click does nothing
        .on_dismiss(|app| app.confirm = None)
        .children(|s| dialog_box(s, |s| {
            s.dialog_title("Delete project?");
            s.dialog_description("This cannot be undone.");
            s.dialog_footer(|s| {
                s.button("cancel", "Cancel").on_click(|app| app.confirm = None);
                s.button("ok", "Delete").destructive().on_click(|app| { app.delete(); app.confirm = None });
            });
        }));
}
```

### Sheet (modal, animated)

```rust
// App: sheet: Shown
s.overlay("settings-sheet")
    .modal(Modal::Edge(Side::Right))
    .shown(self.sheet)
    .transition(Duration::from_millis(200)).ease_out()
    .entrance(Entrance::Slide(Side::Right))
    .role(Role::Dialog).named("Settings")
    .on_dismiss(|app| app.sheet = Shown::Closed)
    .children(|s| sheet_box(s, |s| { /* w_px(380.0).h_full() */ }));
// non-modal: .anchor(Anchor::Screen).placement(Placement::edge(Side::Right)).band(Band::Floating)
```

## 14. Issues to file and build order

**Issues to file** (all linked from #1):

1. M6a: overlay core: roots, `place`, `Anchor`/`Placement`, band layers, blocker, stacking (this note's sections 1 to 3).
2. M6b: dismissal, focus modes, `Nav` (sections 4 and 5), `root_at`.
3. M6c: modal presentation, scrim, focus trap (section 6).
4. M6d: presence on the overlay chain (`.shown`, `.transition`, `.entrance`, `.on_exited`) and sheets (section 7; replaces the M6 part of #21; also records "#35 not required").
5. M6e: owned `Select` onto overlays (section 10).
6. M6f: stock menu widgets (`menu_item`, separators, submenu protocol), `menu` example, autosuggest test.
7. M6g: tooltips onto overlays, shadcn look.
8. dnd: `root_at` in the resolver (part of M6b; a dnd test that a target under a popover is unreachable and one inside is).
9. Amend D18 (stacking; D18 amended to opening order with the repair step) and D22 (stock widgets own their state) in `design.md`; re-title #85.
10. Continuity: `OffScreen`, `Reflip`. Fuzz: `overlay_session`.
11. Post-0.1, label it: typeahead, submenu hover-open, submenu safe triangle, spring-loaded opening, arrows, `.boundary(Rect)`, drawer drag-to-dismiss.

**Build order** (each PR small, tested, with the user's hands on any UI change before approval, per CLAUDE.md):

| PR | Content | Why this position |
|---|---|---|
| 1 | `src/overlay.rs`: `place` + tests (promote the prototype); `Anchor`, `Placement`, `Band` types; amend D18 | Pure, reviewable alone, nothing integrated |
| 2 | Roots in the pipeline: `roots` partition, slot reservation (D19), solve/place/collect, `Placed::overlays`, `s.overlay(..).children(..)` for floating overlays, per-overlay `Area` with blocker, stacking repair step + tests (F1, F2, reopen, convergence), `Headless` accessors, a minimal `menu` example | The tree model; everything else builds on it |
| 3 | Dismissal (`dismissals`, `root_at`, `on_dismiss`), `Focus` modes and restore, `Nav::Menu`/`Listbox`/`Submenu`, accesskit roles; dnd resolver uses `root_at` | Behaviour on a working core |
| 4 | Modal: scrim, `set_modal_layer`, focus trap, scroll-lock test | Needs 3 |
| 5 | `Select` migrated; autosuggest test | First consumer; proves `Stay`/`Listbox` |
| 6 | Chain presence API (`.shown`/`.transition`/`.entrance`/`.on_exited`), sheets (modal and non-modal), `menu`/`sheets` examples; stock menu widgets with submenus, `chat_shell` menus | Look decisions land here with rendered candidates |
| 7 | Tooltips migrated (shadcn look); continuity invariants; `overlay_session` fuzz | Last: least coupled |

PRs 1 and 3's pure parts can proceed while the look entries (6) wait for the user. The user merges each.

## Decisions (2026-11-11)

1. **Stacking:** band, then opening order; nested overlays above their parent; a repair step flags what egui misplaces (section 2).
2. **Tree model:** same tree, one `Placed`; #85 re-scoped to separate `Placed` (section 1).
3. **Modals:** own scrim plus `set_modal_layer`, not `egui::Modal` (section 6).
4. **Presence:** on the overlay chain (`.shown`, `.transition`, `.entrance`, `.on_exited`); M6 does not use ghosts and #35 stays independent (section 7).
5. **Dismissal:** as section 4.
6. **Scroll:** as section 8; anchored overlays follow, hide when clipped, never close on scroll.
7. **Tooltips:** migrate to overlays with the shadcn look (section 10).
8. **Stock widgets** (Select, tooltips, stock menus) own their open state in egui temp data; no controlled mode in v0.1 (section 10).
9. **Looks:** the style-contract entries exist now; values are shown rendered and chosen in the build PRs that introduce them (section 11).
10. **Stock menus with submenus** are in v0.1.
11. **Submenus** open on click, Enter and Right only; hover-open is post-0.1 (section 5).

## As built in PR 2

Where the build differs from, or settles, what the sections above leave open.

- **Roots.** `Frame` holds a `RootDecl` per root (`RootKind::Base` or `RootKind::Overlay`; a pop-out window is a third kind, #103). The slot is pushed before the body is built (D19), `Frame::current` names the root being built, and each root keeps its own scroll leaves. A scroll container's content is *not* a `RootDecl`: it hangs off its leaf and is solved with the leaf's root, as before. `Placed` keeps a `PlacedRoot` per root, with the overlay's side, `Anchored`, anchor rect and place in the stack.
- **Layout.** An overlay root is a flex column, solved at `MaxContent`, with `min_width` set to the anchor's width for `MatchWidth::Anchor`. The clamp pass gives the root a definite size on the axes that exceed `available`, so `h_full` and shrinking children resolve, and re-places with `Spot::refit` (same side, no flip). An anchor that is not in the frame (a typo, or declared in a later overlay) is `Detached`; so is an overlay declared in a detached overlay. The clip chain `dnd::targets` walks is `PlacedNode::clip_for_children`, used by both, and `reachable` applies it to one node.
- **Stack.** `Stack` lives in egui temp data, is updated at layout (so `Headless` sees it) and holds the pointer position a `Anchor::Pointer` overlay was opened at. `OverlayRef::stack_index` is the place in it. One stack per egui context: apps that lay out several `Placed` in one context share it (#85).
- **Draw.** One `egui::Area` per attached overlay, id `overlay_id.with("overlay")`, tight rect, the area's state seeded with the laid-out rect so there is no sizing pass, `fade_in(false)`, not interactable in `Band::Tooltip`. The repair reads the order from `Memory::layer_ids` (`Areas::order` is not public). A pointer press is judged with `root_at`, so a press on a parent flags its submenus too. The drag image is not flagged: it is painted on a layer that is in no area order, and egui draws those after the ordered layers of its `Order`, so it is above `Band::Tooltip` overlays already.
- **Blocker.** `PlacedNode::blocks_pointer` is the one definition: a node blocks if it paints a background or a border (a colour and a width) and did not say `pointer_events_none()`, or is the root of an overlay that is not in `Band::Tooltip`. `Placed::root_at` asks it of the roots. The draw registers a blocker (a non-focusable click-and-drag widget over the border box, before the root's descendants) for the root of an overlay, since what lies beneath it is another layer; every other painted box is an ordinary box (next item). `pointer_events_none` applies to the node and its descendants, and only stops boxes taking the pointer: a node inside that senses still does. An inert subtree takes no input, so its boxes take none. The library's drag preview wrapper is `pointer_events_none`.
- **Hit testing: every box registers (#167, replaces #102's overlap rules).** egui stays the only hit tester, so what is painted and what is hit agree, and its press latch gives pointer capture. A node that paints a background or border and does not sense registers a non-focusable click widget over its border box, in paint order (no accessibility node, no Tab stop). It senses drag too unless an eggshell ancestor senses drag, or it is inside a scroll container whose `ScrollArea` drags its content to scroll (`scroll::source()`, egui's `ScrollSource`, which on a touch screen drags); there the drag belongs to the ancestor, and egui reports click and drag hits separately. A node that senses hover only takes clicks like any box. A press a box wins goes along eggshell ancestry only, to the nearest ancestor that senses clicks, and into that node's `node_response()`; hover reaches every sensing ancestor (CSS `:hover`). That is nearest-handler-only bubbling: CSS bubbles to every ancestor until a handler stops it, which would merge the press into each sensing ancestor and add a stop flag to the response. egui-native content (`custom` and `widget` leaves, widgets added in draw closures) is never a routing target: it gets presses through egui's hit test where no box covers it, a box over it occludes it as in CSS, and `pointer_events_none` lets the press through. If egui took the focus from the ancestor because the press landed on a box in its subtree, the draw gives it back. Unidentified boxes get ids from their key path and ordinal, so a press held across a rebuild keeps its widget.
- **Not yet.** Drop targets inside overlays and `root_at` in the dnd resolver (PR 3): `dnd::targets` still walks the page only. Dismissal, focus, `Nav`, modals, presence.

## As built in PR 3

Where the build differs from, or settles, what sections 4, 5 and 9 leave open.

- **Dismissal.** `dismissals(layers, input)` in `src/dismiss.rs` is the one function (pure, unit-tested over stack shapes, press positions, Esc, a live drag, focus, window blur and opening). `src/overlay_input.rs` reads the frame into its input at the end of `Placed::draw`, runs the `on_dismiss` of what it names, topmost first, and consumes Esc if Esc was the cause. `on_dismiss` is `FnOnce(&mut A) + 'static`, kept by overlay id on the frame. Within one frame the draw runs first, so a click on an item runs its handler before a dismissal caused by the same press.
- **Drag.** "Nothing is judged while a drag is live" covers the press, Esc and focus leaving. The window losing focus and the exclusivity of menus are not judged from a pointer and are still applied.
- **Window blur** dismisses the overlays whose `DismissOn` is `Default`, as focus leaving does, not every non-modal overlay: an `EscOnly` overlay is one the app wants answered.
- **Swallowing.** The default comes from `Nav`: `Menu` and `Submenu` swallow, `Listbox` and `None` pass through, since an autosuggest's anchor is the field a press may be on. The stock `Select` (#108) says `.outside(Outside::Swallow)` itself. The blocker is the overlay's own layer made screen-sized (one extra `ui.interact` over the screen before the root), so it needs no extra layer in the stacking repair, and `root_at` returns the swallowing overlay for every point. A tooltip never swallows.
- **Focus.** `Focus::initial(key)` is not built: the overlay's first focusable is where focus goes, and an item that wants it first is declared first. Entering is done in the draw that first draws the overlay, from the focusables its subtree registered, so an overlay detached at its first frame enters when it is first drawn. The widget focused at the layout in which the overlay opens is kept in the stack and given back at the layout in which it is gone, if focus is in a closed overlay or nowhere and the widget was on screen. A pointer button down, or released this frame, delays it (egui drops the focus of a click that lands elsewhere, which would undo it); after a click on a place that cannot take focus the library therefore gives it back one frame later. Mouse clicks do not focus the owned buttons, so an overlay opened by the pointer has nothing to give back until the trigger had focus from the keyboard.
- **Focus leaving** compares the last frame's `Inside` with this frame's `Elsewhere`. `Inside` includes the overlay's anchor and the overlays nested in it. Focus on a widget that was not drawn this pass (one of an overlay that just closed) is `Nowhere`.
- **Nav.** An item is a node that senses and has an item role, given with `NodeHandle::role(Role::MenuItem | MenuItemCheckBox | MenuItemRadio | ListBoxOption)` and `.named(..)`; the role is also what assistive technology gets, so the library does not read it back from accesskit (which is only built while a screen reader is on). Items that are disabled (their egui widget is) are skipped. The keys go to the topmost overlay with a `Nav` that holds focus in its own widgets, then to the topmost whose anchor holds it, and with nothing focused to the topmost. Home and End are left to a focused text field, and so is Space for a listbox. `Menu` holds the arrow keys on the focused item so that egui does not also move focus.
- **Listbox.** The highlighted item per listbox is kept in egui temp data; the draw gives it its `hover` look (there is no separate `highlighted` look yet: #108 decides it with the stock look). Enter, and Space unless a text field is being typed in, are taken before the frame's widgets, so that a trigger with focus does not take them, and activate the item as a click: the item's `on_click` runs in the same frame. The highlight does not scroll its item into view; roving focus (`Menu`) does, through the scroll container's focus following.
- **Submenu.** Left dismisses the submenu and focuses its anchor. Opening is the item's concern (click, Enter, Right). The hover rule of #109 (hovering a sibling that owns a submenu switches to it, hovering a plain item closes the open submenu) needs the stock menu item and is left to #109; the library side is only the two overlays' exclusivity and dismissal above.
- **Accesskit.** While an overlay anchored to a node is open, that node gets `expanded` and, for a menu, listbox or dialog role, `has_popup`. It cannot say `expanded = false` when nothing is open; the widget that owns a trigger does (#109, #108).
- **DnD.** `targets` walks the page and then every overlay root, bottom of the stack first; a target is reachable at the pointer only if `root_at(p)` is its own root. A tooltip is never reachable, a detached overlay's targets neither, and a swallowing overlay hides the page's targets everywhere.
- **egui facts kept as tests** (`tests/it/overlay_input.rs`, `egui_facts`): F4 (Esc blurs before library code, and the key is still readable after the draw) and F6 (a press keeps its origin through a drag; `any_pressed` is the press frame only).
- **Not yet.** Modals and the focus trap (PR 4), `Select` (PR 5), presence and stock menus (PR 6), tooltips (PR 7).

## As built in PR 4

- **Typestate.** `Builder::overlay(key)` is `OverlayChain<Unplaced>`; `.anchor(..)` makes it `Floating` (which now owns `.placement(..)` and `.match_width(..)`, so a modal cannot call them) and `.modal(Modal::Center | Modal::Edge(Side))` makes it `Modally`. `OverlayRef::side()` is `Option<Side>` (a modal has none) and `OverlayRef::modal()` reports the presentation: a breaking change to `side()`.
- **Layout.** A modal's root is solved at `Context::content_rect()` (the whole screen, not the layout `area`, which a central panel narrows): a flex row, `Center` is `items-center justify-center`, `Edge(Left|Right)` is `justify-start|end` with the box stretched, `Edge(Top|Bottom)` the same in a column. The root node paints `Style::scrim` as its background and is the blocker (it was already one: the root of an overlay always blocks), so the page behind takes no pointer input and no wheel (F5; the scroll-lock test is `the_wheel_scrolls_the_page_until_a_modal_covers_it`). The box is the root's child: the union of its children's rects is what a press must be inside to not be on the scrim.
- **Focus trap.** Each frame, `Walk::overlays` calls `Memory::set_modal_layer` with the layer of the topmost attached modal in the library's own stack order (egui keeps the topmost of the layers it is told, by its own order, which can lag the repair step). A nested popover is above the trap. egui learns of a closed modal a frame late and drops focus given to a widget below it, so `focus::restore` waits until the opener is above the trap again.
- **Accesskit.** The blocker of a modal carries the overlay's role and name and `modal`.
- **Dismissal.** `dismiss::Layer` gained `boundary` (`Open`, or `Scrim` for a modal), `rect` is the box for a modal, and `cancel`. The walk of rule 2 stops at a modal whatever it is nested in (a press on a nested confirmation's scrim is not a press on the dialog below it), and a modal is not dismissed by focus leaving or the window losing focus. A modal is not a swallowing overlay, so it is not dismissed by another overlay opening.
- **Actions.** `Cause::Cancel` is Esc answered by a cancel button. The button is pressed by `nav::request_activation`, the listbox's mechanism, so the activation is a click the button's `on_click` receives in the next frame (Esc is judged after the draw). Return is decided in the same place. A multi-line `TextEdit` notes its focus for Return.
- **Look.** The dialog and the sheet have no contract entry yet: the example builds the boxes with classes copying shadcn's `DialogContent` and `SheetContent` (`bg-background rounded-lg border p-6 shadow-lg gap-4 max-w-lg w-full`; `shadow-lg` without its negative spread, which `Shadow` cannot express). Their values, and the entrance motion, are PR 6's with rendered candidates.
- **One stack per `LayoutState`.** The stack of open overlays is keyed by the `LayoutState` that built the frame, no longer one per context. The examples' shell lays out a theme bar and then the page in one context; the bar declares no overlay, so a shared stack closed the page's every frame (focus entered the dialog again each frame, so Tab in a text field did nothing on the web and natively). Stacking across several `LayoutState`s is still #85.
- **Keys are taken before the draw (#146).** `claim_keys` runs after layout and before any draw closure: it decides where Esc goes (`escape_goes`, one function shared with the judgement: the topmost overlay that takes it, else the window's cancel button) and which default button Return goes to, and consumes those keys, so a draw closure (a canvas under a menu) never sees an Escape or Return that belongs to the library. The decisions are applied by `judge` after the draw as before. Return is decided from the focus at the start of the frame: a single-line field no longer sees the Return that presses a default button.
- **Menus open as in Radix.** A `Nav::Menu` or `Nav::Submenu` overlay opened with the pointer takes no focus (the library remembers whether the last input was a pointer press or a key; an overlay the application opens with no input counts as the keyboard): nothing is highlighted, the first Down goes to the first item and the first Up to the last. Opened from the keyboard (Enter, Space, Down on the trigger), focus enters the first item as before. Other overlays (dialogs, popovers with fields) still enter on a pointer open. egui reads the arrow as a move of focus in the pass it is pressed, so with nothing focused the navigation gives the item its focus at the start of the next frame.
- **Focused item look.** An item (a node with an item role) that has keyboard focus is drawn with its `hover` look, as a Listbox's virtual highlight is, so shadcn's `focus:bg-accent` comes with `hover:` and needs no `focus:` class. A hovered item and a focused one can both show it until hover moves focus (Radix does that; not built).
- **Not yet.** `Focus::initial(key)`, `Shown` and transitions (PR 6: the sheet appears at once), the stock `dialog_*` widgets.

## As built in PR 5

- **Select.** The list is an overlay anchored to the trigger: `Placement::below()` with a 4 pt gap, `MatchWidth::Anchor`, `Focus::Stay`, `Nav::Listbox`, `Outside::Swallow`, role `ListBox` named like the select. Whether it is open is the select's own state in egui temp data under the trigger's id (`Listing`, no `App` field); the overlay's `on_dismiss` clears it, so do choosing, Esc and a second click on the trigger. A select that becomes disabled closes. The overlay key is `(key, List)`, so the rows are at `<select>'s parent>/(key, List)/Box/Option(i)`, or `.../Box/Rows/Option(i)` when the list scrolls. `egui::Popup`, `List::show` and the hand-painted rows are gone; `hold_keys` lost its `escape` flag (Esc is the library's, and focus comes back through `focus::restore`) and is now `hold_arrows`.
- **Look.** Unchanged, and pinned: the box is a node drawn from `SelectStyle::popup` (fill, border, radius, padding, shadow), each row from `item` (rest, `hover` for the highlight, `disabled`) with the check mark where the chosen option is. A row has no `active` colour and no transition, as before. The open list in the controls gallery is a new golden pair; every existing golden is unchanged. The rendered list is pixel-identical to the `egui::Popup` one; only the trigger differs, which is no longer drawn hovered while its list is open (the swallowing blocker covers it, as Radix's `pointer-events: none` on the page does).
- **Scrolling.** A scroll container always reserves its gutter, so the rows are put in one only when the list does not fit. The select asks `place` itself (`pipeline::room_for`, with the layout area and the trigger's rect of the last layout) whether the rows' natural height gets its room under the same `Placement`, so the build-time decision agrees with the layout's clamp and the side chosen. The answer depends on the trigger and the screen, never on its own result, so it cannot oscillate. The box has `min_h_0` so that it shrinks to the clamped root and the rows scroll inside it.
- **Highlight.** The initial highlight is the chosen option (else the first enabled one): `nav::highlight_initially`, from the rows' build, centred in the container. Arrow moves scroll the new item into view with the least movement (`scroll_into_view`); a pointer move over an item moves the highlight to it, and an item of a `Nav::Listbox` is hovered only by its highlight (`Walk::hovering`), so the pointer and the keys never light two rows. The highlight still uses the item's `hover` look, as PR 3 left it; there is no separate `highlighted` look.
- **`Wrap`.** `OverlayChain::wrap(Wrap::Around | Wrap::Stop)`, `Around` by default (menus, and the PR 3 listbox tests); the select says `Stop`, as Radix's does, which its existing tests pin.
- **Keys.** Enter, Space, Down and Up on the focused closed trigger open it (the trigger's draw: Enter and Space are egui's click, the arrows are held by `hold_arrows`). Enter and Space activate the highlight when nothing has focus, so a list opened with the pointer (which does not focus the trigger) is chosen with the keyboard, as `keyed` already sent it the arrows.
- **Focus stays (#150 item 3).** While the pointer is on the box of a `Focus::Stay` overlay, `Placed::draw` holds `InputOptions::surrender_focus_on` at `Never` for the pass (`focus::AnchorFocusHeld`, an RAII guard; `overlay_input::pointer_on` says where the pointer is). egui surrenders the focus of a widget when a click lands elsewhere, as the widget registers, which blurred the field (running its blur handler) before the pick applied. This is `preventDefault` on `mousedown`. A press anywhere else still moves focus.
- **Esc that blurs a field (#150 item 4).** `Typing` notes at the end of each draw whether a text input has focus; `claim_keys`, in the one match that decides where Esc goes, also takes an Esc that arrives with no focus when `Typing` was `Yes` (and no drag is live). A custom node reading the keys sees neither.
- **The dialog example's role picker** stays the hand-made `Nav::Menu` it is; a `select` in a dialog is covered by `tests/it/select.rs` (opens above the trap, chooses, Esc closes it before the dialog).
- **Known edge.** Enter activates the highlighted row through its `on_click`, which runs in the row's draw closure; a row scrolled out of view with the wheel is not drawn, so Enter on it is lost. Keys that move the highlight scroll it into view first, so only wheel-then-Enter meets it.
- **Not yet.** `Focus::initial(key)`, presence (PR 6), the dialog and sheet looks, stock menus, tooltips (PR 7).

## As built in PR 6b (stock menus, `CustomChain::role`)

- **API.** `Builder::dropdown_menu(key, face)` is a `DropdownChain` (`.variant`, `.icon`, `.classes`, `.with_options(ButtonOptions)`, `.named`, `.enablement`, `.on_response`) finished with `.items(|m| ..)`; the trigger is the owned button (`opens_popup`). `NodeHandle::context_menu(|m| ..)` opens at the pointer on a secondary click (a node that did not sense now senses clicks and leaves the Tab order). `m` is a `Menu`: `item(key, label)` (`.shortcut`, `.destructive`, `.enablement`, finished by `.on_click`), `checkbox_item(key, label, Mark)` (finished by `.on_change(|app, Mark|)`), `radio_item(key, label, Mark)` (`.on_click`), `separator()`, `label(text)` and `submenu(key, label, |m| ..)`. `.command(id)` is #125 and is not here.
- **State.** Whether a menu is open, and which submenu is open at each depth, is `Kept::menus`, keyed by the trigger's id (the owner node's for a context menu), so it is per `LayoutState` and not in egui temp data. Activating an item runs its handler and removes the entry: the whole chain closes. The overlay's `on_dismiss` closes the menu (the submenu's closes its level only). A trigger that becomes disabled closes its menu.
- **Overlays.** The dropdown is a `Nav::Menu` overlay (`Placement::below()`, 4 pt gap, swallowing, role `Menu` named like the trigger). A submenu is declared inside its parent's box, anchored to its item with `Placement::submenu()`, `Nav::Submenu` and `Outside::PassThrough`: a swallowing submenu would put its screen-sized blocker over the parent, which could then be neither hovered (the hover-switch) nor clicked in one press. The press that dismisses the submenu therefore reaches the parent's item, and a press anywhere else still ends at the root's swallowing blocker.
- **Submenu protocol.** A click, Enter, Space or Right on the item opens it (the library's pointer-or-key rule decides whether focus enters it); Left and Esc close it and focus the item (library). Hover: with a submenu open at a depth, the pointer *moving* onto a sibling that owns a submenu switches to it and onto a plain item closes it (`Menus::hover`, unit-tested); with none open hover opens nothing; no safe triangle. A moved pointer is `pointer.delta() != 0`, so a resting pointer does not undo a key-opened submenu.
- **Accesskit.** The trigger has `has_popup(Menu)` and `expanded` both ways; a submenu item has `has_popup(Menu)` and `expanded`; check and radio items are `MenuItemCheckBox` / `MenuItemRadio` with their state.
- **Look.** `Style::menu` (`MenuStyle`): `popup` (box look), `radius`, `padding`, `min_width`, `item` (`hover` is the highlight, pointer or focus), `item_radius`, `item_padding`, `item_gap`, `destructive` (rest and hover), `separator`, `separator_margin`, `heading`, `text`, `shortcut`, `shortcut_text`, `shortcut_tracking`, `indicator` (check, radio and chevron square), `dot`. The shadcn preset fills it from DropdownMenu (new-york v4): `p-1 rounded-md border shadow-md min-w-[8rem]`, item `rounded-sm px-2 py-1.5 gap-2` highlighted by `bg-accent`, destructive `text-destructive` with `bg-destructive/10` (`/20` dark), separator `bg-border -mx-1 my-1 h-px`, shortcut `text-xs tracking-widest text-muted-foreground`; the check and chevron are 16 pt squares, the heading text is the item's (`font-medium` needs a medium family, as for the controls). The neutral default derives from egui's style. `MenuStyle::box_classes` and `row_classes` are the one definition the select list and the menus draw with; the select's goldens are unchanged. Row height is `line_height + 2 * item_padding.y` (32 pt in shadcn).
- **`CustomChain::role`.** `CustomChain::role(Role)` gives a custom-painted node the semantics of `NodeHandle::role`, with `named` as its name. Test: a swatch painted with `.custom(..)` and `MenuItemRadio` in a `Nav::Menu` overlay is reached by the arrows and activated by Enter.
- **Examples.** The `menu` example's popover is a stock dropdown (actions, a disabled item, headings, check and radio items, a submenu, a destructive item); its tall scrolling list stays hand-built, since stock menus do not scroll. The dialog's role picker stays hand-built: it is width-matched to its trigger and marks the current role in the label, which the stock dropdown (a `min-w` box with radio marks) does not do.
- **Not yet.** Typeahead, hover-open from closed, the safe triangle, a menu bar, scrolling stock menus, `.command(id)` (#125), keyboard opening of a context menu (Shift+F10, the Menu key), a controlled mode.

## As built in PR 6a (presence, sheets)

- **API.** On the overlay chain: `.shown(Shown | bool)`, `.transition(Duration)` (enter and exit), `.exit_transition(Duration)` (exit only; order against `transition` does not matter), `.easing(Easing)` with `.ease_linear/in/out/in_out()`, `.entrance(Entrance)` and `.on_exited(|app| ..)`. `Entrance` is `Fade | Zoom | Pop | Slide(Side)` (`SlideFade` of §7 is gone). **The box and its contents are never faded**: without group opacity (#40) a per-shape fade lets the page show through the box mid-motion, so only a modal's scrim fades with progress. `Zoom` scales the box up from 95% about its centre (shadcn's `zoom-in-95`); `Pop` is `Zoom` plus the 8 pt `slide-in-from-*-2` away from the anchor (a modal has no anchor and only zooms); `Slide` moves the box by its own size from that side. `Fade` has no motion of the box, so the box appears and vanishes at once while the scrim fades (a floating overlay has nothing to fade: it is instant, though it stays mounted, inert and blocking, for the exit's duration). `Shown` has no `From<bool>`: no naked bool arguments.
- **Deviation from §7: `shown` is optional** (§7 says required; decided to keep it optional). An overlay declared without `shown` is open exactly in the frames that declare it and has no motion, so every existing overlay (the stock select, menus and the tests) is unchanged. The motion methods without `shown` are a debug panic naming the declaration site.
- **State.** The phase (`Closed | Entering | Open | Exiting`) is `Kept::presences`, per `LayoutState`: a `Transition` per overlay id, stepped at build with `transition::Transition::toward`, so there is no second clock; the progress is the eased `Progress`, reversed from where it is (and lasting in proportion to the distance left, as CSS does). Entries of overlays not declared in a frame are swept. `children` is not called while the overlay is closed, and no root is declared for it. `on_exited` is queued at the build that finds the overlay closed after being present, and runs at the end of that frame's draw, once. Repaints are requested only while in flight.
- **Exiting overlays.** Inert (the subtree is drawn inert), but the root's blocker stays live, so the page behind takes no pointer input until the overlay is gone; a press on the scrim or Esc does nothing. An exiting overlay is not one of `open_overlays`: it is not judged by `dismissals` (outside press, focus leaving, window blur), takes no `Nav` keys, no Esc, no Return default action, and is not entered with focus. It stays in the stacking (the repair, `root_at`, the focus trap) and in the library's stack until it is closed, so focus is restored when the exit has run, not when it starts. An overlay that is entering is live in every respect: Esc while it enters dismisses it and it turns round.
- **Paint.** Layout and hit rects are final from the first frame; the draw moves what the box painted (`transform_range` on the shapes it pushed, so draw closures and shadows move with it). A modal's root paints the scrim at the progress, and its children are the box. A floating overlay's root is the box. Zoom scales the painted shapes (text included, through egui's mesh transform, so glyphs are resampled while moving) and hit rects stay where layout put them.
- **Style.** `Style::motion: MotionStyle { popover, menu, dialog, sheet }`, each a `KindMotion { entry, enter, exit, easing }` with `Entry::{Fade, Zoom, Pop, Slide, SlideFade}` (a `Slide` entry takes the edge of the `Modal::Edge` it is for; without one it fades). The chain's kind is `dialog` for `Modal::Center`, `sheet` for `Modal::Edge`, `menu` for a floating overlay with a `Nav`, `popover` otherwise. The neutral default is instant. shadcn preset: popover and menu `Pop` 150 ms; dialog `Zoom` 200 ms; sheet `Slide` 500 ms in, 300 ms out, `ease-in-out` (the tailwindcss-animate `ease` curve is not one of `Easing`, so the default curve stands in).
- **Continuity.** `Motion::Presents` marks the nodes of an entering or leaving overlay; the teleport check judges the paint offset the motion adds, which may not jump; where layout puts the box is its anchor's, which may move at once (a reflow). A test records a sheet opening and closing and asserts `assert_continuous`.
- **Examples.** The dialog example's dialog, confirmation and sheet use `shown`; the role picker is still declared only while open.
- **Stock widgets.** The select's list and the dropdown menu are always declared with `shown(open)`, so they enter and leave with the style's `menu` motion (shadcn: `Pop`, 150 ms). The open state is still the widget's own (`Listing`, `Menus`); closing it makes the overlay `Closed` while the list or menu is built one more time for the exit. Tests that look for a closed list or menu to be gone settle for the exit first. Submenus and the context menu are declared only while open and have no exit yet (the context menu's position is dropped with its open state).
- **One `LayoutState` per frame is not supported with motion.** The presence is `Kept` state, so a layout recreated every frame never leaves progress 0 and a moving overlay is never drawn (the stack and the layout animation already need the state to persist: see `LayoutState`). `tests/it/popup_style.rs` held a throwaway layout and now keeps one.
- **Review images.** `cargo test --features snapshots --test it -- --ignored motion_strips` (with `EGGSHELL_MOTION_DIR` and the lavapipe `VK_ICD_FILENAMES`, as `cargo xtask snapshots` sets) writes frames at 0, 25, 50, 75 and 100% of the enter and of the exit of the dialog, the sheet and the menu, in both themes.
- **Not yet.** Submenu and context-menu exits; the tooltip (PR 7); drawers.

## As built in PR 7 (tooltips)

- **Mechanism.** Every `.tooltip(..)` (the owned controls, `NodeHandle::tooltip`, `CustomChain`, a `Choice`'s) is a draw closure that calls `owned::tipped`, which registers the node as a trigger for the frame (`tooltip::register`); nothing is declared for it at build, and `Response::on_hover_text` is gone. At the end of `Placed::draw` the frame's triggers are resolved (`tooltip::end`) and the next build declares the overlays (`tooltip::declare`, after the application's view, in the same builder): one per tooltip that is open or leaving, keyed by its trigger, so moving from one trigger to another is one overlay leaving and one entering, never one overlay moving. The overlay is `Band::Tooltip`, `Anchor::Node(trigger)`, `Placement::tooltip()` with the style's gap, `Focus::None`, `DismissOn::Never`, `shown(open)`, so it has the style's tooltip motion and its exit. It never senses (no blocker, `pointer_events_none`), as the band already guaranteed.
- **Breaking: `LayoutState::build` needs `A: 'static`.** The library now declares overlays whose draw closures are boxed, as the stock select and menus already did. Application types are owned data in every case we know of.
- **Hover intent** is `Kept::tooltips` (D11a), a pure state machine (`Tooltips::tick`, unit-tested): a pointer trigger waits for `TooltipStyle::delay`; one that has keyboard focus (and the last input was a key, so a click that focuses a field shows nothing) opens at once; while a tooltip is open, or one closed within `TooltipStyle::grace`, the next opens at once (Radix's skip delay). A press anywhere, Esc, a live drag and the window losing focus close it and keep it closed until the trigger is left (or focus leaves it); the pointer leaving, or the trigger no longer being drawn, closes it. The pointer wins over focus, and among triggers under the pointer the one drawn last (the deepest) wins. A disabled trigger is a trigger: hover is `contains_pointer`, which every box registers. Scrolling the anchor out of view is the overlay machinery's (section 8). egui's `show_tooltips_only_when_still` is not honoured.
- **Accessibility.** The trigger carries the text as its accesskit `description` (the aria-describedby equivalent; egui's `WidgetInfo` has no description, so it is set with `accesskit_node_builder`) whenever it has a tooltip, open or not. The tooltip overlay itself has no accessibility node: it never senses and has no blocker, and its text is already the trigger's description.
- **Style: `Style::tooltip` (`TooltipStyle`).** `popup` (the box `Look`: fill, border, shadow), `radius`, `padding`, `text`, `max_width` (longer text wraps), `arrow: Option<Arrow { size }>`, `offset`, `delay`, `grace`; the motion is `Style::motion.tooltip`. The gap between trigger and box is `offset + arrow.size`, as Radix adds the arrow's height. The arrow is a 45-degree square painted by the box's draw closure, centred under the trigger and kept inside the box's corners, half inside the box (`ARROW_SINK`) as shadcn's `translate-y-[calc(-50%_-_2px)]`; its corners are not rounded (`rounded-[2px]` is under a pixel), and it takes the box's fill, so a box with a border should not have one. The neutral default is derived from egui: the popup look and `menu_margin` padding of its tooltip frame, `tooltip_width`, `interaction.tooltip_delay` and `tooltip_grace_time`, no arrow, instant motion. The shadcn preset is TooltipContent (new-york v4): `bg-foreground text-background rounded-md px-3 py-1.5 text-xs`, a 10 pt arrow, `Zoom` 150 ms, a 320 pt `max_width` (`max-w-xs`; shadcn has none).
- **Look and timing (decided 2026-11-13, from renders):** shadcn's inverted box with the arrow; a **300 ms** open delay (between shadcn's 0 ms and egui's 500 ms: a resting pointer is answered, crossing a toolbar shows nothing) and a 300 ms grace (Radix's `skipDelayDuration`). The candidates can be re-rendered with `cargo test --features snapshots --test it tooltip_candidates -- --ignored` (`EGGSHELL_TOOLTIP_DIR`, `VK_ICD_FILENAMES`). `LayoutState::build` requires `A: 'static` (accepted): tooltip overlays box their draw closures at build time.
- **Rich text.** Built in #174, below.
- **Not yet.** Hover cards, a tooltip on a `Headless` trigger without a draw, the `show_tooltips_only_when_still` rule, a per-trigger `side` or delay.

## As built in PR 8 (continuity and the overlay fuzz)

- **Continuity.** `Timeline::off_screens` (`OffScreen`) and `Timeline::reflips` (`Reflip`) run in `violations()` and `assert_continuous`, with `assert_on_screen` and `assert_no_reflips`. The draw records one `OverlayFrame` per drawn overlay (`bounds`, `anchor`, `natural` size, `side`); a detached overlay is not drawn and not recorded. `OffScreen` judges the painted rect against `bounds` (the window cut to the boundary, `Placement::region`, before `margin`), per axis: along an axis the overlay is larger than the bounds they cannot hold it, and `place` starts it at their edge. A box mid-entrance is judged where layout put it, since its motion starts displaced. `Reflip` compares the side of two frames only when the anchor, the natural size and the bounds are the same: the choice of side depends on those alone (§3, rule 4). `leaves_bounds` is the one definition of "outside", shared with the fuzz. `OverlayRef` gains `bounds`, `natural_size`, `is_exiting`, `focus` and `outside`.
- **Presents.** The teleport check judges a `Presents` node on the paint offset its motion adds, not on where it is painted: an overlay follows its anchor, and a reflow moves the anchor at once (a tooltip whose trigger a resize wraps to the next row is not a jump).
- **Tooltips never take the pointer, in egui's eyes too.** The area an overlay is seeded with was interactable for every band; a press on the page under a tooltip that was on screen was judged by egui as a press on that tooltip's layer, which raised it over the tooltip that opened after it. The seed is `interactable: band != Tooltip`, as the area is.
- **Stacking: an area that was not shown last frame.** egui raises such an area itself, which is what happens to an overlay whose anchor comes back into view (a popover in a scroll container, or a window that grows back). The repair now flags the first such layer of a band, and those above it, as it flags the pressed one, so it no longer rises over a dialog that opened while it was hidden.
- **Exclusivity sees hidden menus.** An open menu that is detached (its anchor or point left the window) takes no part in the other dismissal rules, but is dismissed when another overlay opens beside it (`overlay_input::hidden_menus_made_way_for`); otherwise it came back over the new one when the window grew.
- **The fuzz** is `tests/it/fuzz/overlays.rs`: a page with a stock dropdown (submenu, an item that opens the dialog), a stock select, two popovers (one anchored in a scroll container), a modal dialog with a select and a nested confirmation, tooltips, a context menu and a text field. Per frame: every overlay inside its bounds, egui's layer order is the library's stack order within a band and every nested overlay is over its parent, no tooltip takes the pointer. Per action, after frames for exits and the one-frame lags: no press reaches the page while a modal is up, Esc closes exactly one overlay (counting those that are open, seen and not tooltips) and gives focus back to the opener, at most one menu is open. Over the session: `assert_continuous`.
- **Tab is swallowed while a menu is open** (decided with the user, as in Radix). `claim_keys` consumes Tab and Shift+Tab, beside Esc and Return, while any overlay that swallows presses or navigates as a menu or submenu is open, however it was opened: nothing moves until Esc, a press outside or a choice. Before, a menu opened with the pointer took no focus and Tab moved focus to the page with the menu still open (`dismiss.rs` pins `Nowhere -> Elsewhere` as not leaving, which now cannot arise from Tab). Popovers and dialogs keep Tab, but a Tab that takes focus off an open overlay dismisses it even when focus was nowhere before or passes through nowhere on its way round to the first widget (`dismiss::leaving`, with `Input::tab`). egui applies Tab as the widgets are created, so a header layout drawn before the menu's has already moved focus when the keys are claimed: the swallowed Tab is undone after the draw (`FocusTrail`). A menu opened with the pointer surrenders focus that is on the page under it (a closing popover gives its focus back as the menu opens). The fuzz's focus invariant runs in the decided session.
- **Parked: #184.** With a pointer button held, End on a focused button in a scroll container scrolls it for the frame of the key press and puts it back in the frame of the release (a hidden tooltip for a frame is how the fuzz saw it). The decided session does not press the arrows, Home or End while a button is held (`Scroll::Raw` does, in an ignored test).

## As built in #174 (rich tooltip content)

- **Formatted text.** `.tooltip(text)` keeps egui's formatting. A `WidgetText::Text` is the library's text leaf, as before (the goldens are unchanged); a `RichText`, `LayoutJob` or galley is `Prim::Rich`, laid out by egui (`TextLeaf::rich`, `measure.rs`) with the node's role font as the fallback and none of its `override_font_id`, so `.monospace()` and `.code()` hold. Sections that set no line height take the role's, scaled by their size. egui resolves `strong` and `weak` against its visuals, not the box's colours, so the text is laid out in a style whose text colours are the node's ink (`ink_style`): on an inverted tooltip egui's strong colour would be the box's own. egui has no bold, so strong text reads as plain. The case, tracking, numerals and decoration of the node's text style apply to plain text only. The same leaf is public as `Builder::rich(tw, text)`. A min-content width is the widest whitespace-separated word, each word a job that keeps its sections' formatting (`words`), for plain and rich text alike.
- **Built content.** `.tooltip_with(|s| ..)` on every owned control's chain, the toggle, `CustomChain` and `NodeHandle` (not `ButtonOptions` or `Choice`, which name no application type) builds the content with the builder inside the tooltip's box. The closure is `Fn(&mut Builder<'_, '_, A>) + Send + Sync + 'static`: it is kept in `Kept::tooltips` (D11a, per `LayoutState`, not generic in `A`) as an `Arc<dyn Any + Send + Sync>`, as a drag payload is. The downcast needs `A: 'static`, which declaring the overlays may not ask of every layout (#176), so a trigger of built content, whose chain already asks it, leaves the frame a `fn` that downcasts and builds (`tooltip::BuildBuilt<A>`), and the declare builds through it. Keeping the content in `Kept`, not handing it from the trigger's build to the next declare, keeps a leaving tooltip's content, so its exit plays out when its trigger is no longer built, as a plain tooltip's does, as long as some trigger of built content is built that frame. When none is, the frame has no such `fn` and the tooltip ends at once rather than leave as an empty box.
- **Never sensing.** The content is in `Band::Tooltip`, whose area egui does not let interact, so a control in the content takes neither a press nor keyboard focus (tested with a button inside, over another button). It is not made `inert`, which would draw such a control disabled.
- **Description.** The trigger's accesskit description is the content's text, joined by spaces in draw order, whether or not the tooltip is open: each build, at the trigger's terminal call, the content is built where nothing shows it (`Builder::text_of`: its nodes are made under a key of their own and left out of the tree, never laid out or drawn). That build happens only while accesskit is on, which egui leaves off until the platform's accessibility layer asks for the tree (eframe enables it then). egui 0.37 has no public query for the flag, so the library asks whether the pass has the accesskit root node (`has_accesskit_node(accesskit_root_id())`), which egui makes only while it is on. With accesskit on, the description costs about 4 µs a trigger a build (criterion, 20 buttons, release, the build alone: 42 µs with plain tooltips, 121 µs with built ones, label and chip); with it off, nothing.
- **Key chips.** `Builder::kbd(keys)` is HTML's `<kbd>`, styled by `Style::kbd` (`KbdStyle`: `page` and `tooltip` colours, border width, radius, horizontal padding, height (also the least width), text role). The builder knows what it draws on (`Surface::{Page, Tooltip}`, carried with the enablement as the builder's `Ambient`); a tooltip's content is built on `Tooltip`, so a chip takes the tooltip colours there. The neutral default is egui's code background and text colour for both. The shadcn preset is Kbd: `bg-muted text-muted-foreground h-5 min-w-5 rounded-sm px-1 text-xs`, and on a tooltip `bg-background/20 text-background` (`/10` dark). `font-medium` has no weight to draw. Candidates: `cargo test --features snapshots --test it kbd_candidates -- --ignored`.
