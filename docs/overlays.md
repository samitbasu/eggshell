# Overlays (M6): design note

**Status:** proposal for approval. Nothing here is built. Two prototype test files back the claims that needed evidence (`tests/it/overlay_egui_evidence.rs`, `tests/it/overlay_placement_prototype.rs`); both are marked as prototypes and are replaced by real tests in the build PRs. Questions only the user can decide are collected in **User decisions** at the end.

Inputs: plan issue #1 (Container API, M6), D18, D19, D22 and §6 and §11 of `design.md`, #21, #35, #85, #40, `dnd.md`, and the code as of origin/main 855eb20 (M5 scroll is built; the owned `Select` uses egui's popup; tooltips use egui's).

## 0. Summary

| # | Question | Recommendation |
|---|---|---|
| 1 | Tree model | An overlay is a **separate root in the same tree, solved and drawn in the same frame**, like scroll content. One `Placed`. Drag and drop across overlays needs nothing; #85 stays open only for apps that use several `Placed`, and is not an M6 dependency. |
| 2 | Stacking | `Band` maps to `egui::Order`; **stacking inside a band is opening order**, with nested overlays always above their parent. **D18's "build order via `move_to_top` every frame" does not work** (finding F1) and is replaced. |
| 3 | Placement | One pure function, `place(anchor, size, &Placement, bounds) -> Placed`, flipping to the opposite side, shifting on the cross axis, reporting the room left (for scrolling a tall list). Prototyped with 12 tests. |
| 4 | Dismissal | Judged **at the press**, top-down through the stack, stops at the first overlay that contains the press (its descendants and its anchor count as inside). Esc goes to the topmost dismissible overlay. Nothing is judged while a drag is live. |
| 5 | Focus | Three modes, `Enter`, `Stay`, `None`. Esc blurs focus inside egui (F4), so the library restores focus to the trigger on close. |
| 6 | Modals | Eggshell's own scrim (a full-screen root that senses clicks), plus `Memory::set_modal_layer` for the Tab trap. **Not `egui::Modal`.** Scroll lock is free (F5). |
| 7 | Sheets, #35 | An edge-anchored overlay. Exit motion by a small `Presence` helper that keeps the overlay mounted during its exit. **M6 does not need ghosts**; #35 stays independent. |
| 8 | Scroll | Anchored overlays follow their anchor with no lag (same frame), hide when the anchor is clipped away, and do not close on scroll. Point and pointer anchors stay in screen space. |
| 9 | Drag and drop | One resolver for "which root is under the pointer"; drop targets inside overlays work; the drag image stays on top. |
| 10 | Migrations | Move the owned `Select` onto overlays in M6. Move tooltips too, last, if the user wants the shadcn tooltip look. |
| 11 | Look | New style-contract entries `popover`, `menu`, `dialog`, `sheet`, `tooltip`, `scrim`. Every aesthetic value is the user's to pick. |
| 12 | Testing | `Placed` exposes overlays; continuity gains two invariants; the fuzzer gets an `OverlayAction` session. |
| 13 | API | Section 13. |
| 14 | Build order | Section 14: 7 PRs. |

### Findings from the prototypes

| | Finding | Evidence |
|---|---|---|
| F1 | `ctx.move_to_top(layer)` only sets a flag. At the end of the frame egui does a **stable sort by `(order, flagged)`**. Flagging every overlay every frame lifts them above other layers of the same `Order`, but cannot reorder them among themselves: they keep the order they were first shown in. Swapping the declaration order does not restack. | `flagging_every_frame_in_a_new_declaration_order_does_not_restack`; egui's own comment on `wants_to_be_on_top` ("they will keep their relative order") |
| F2 | The flag still has to be set every frame: an `Area` raises itself on a press, which would put a parent above its open submenu. | `a_press_on_a_parent_raises_it_over_its_submenu_unless_all_are_flagged` |
| F3 | `Memory::set_modal_layer` traps **focus** (Tab no longer reaches layers below) but does **not** block the pointer. `egui::Modal` blocks the pointer with a full-screen click-and-drag widget in its layer; without it clicks pass through. The same applies to any overlay panel: a non-sensing box registers no widget, so the pointer falls through it to the page. Every overlay root needs a blocker. | `a_modal_layer_blocks_layers_below_it_and_not_those_above` |
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
| **#85** | Not needed for overlays. It remains for apps that lay out *panels* as separate `Placed` (unchanged by M6). The dnd resolver is lifted from "walk the root" to "walk every root, in stack order, with blockers" (section 9); that is the same-`Placed` generalisation and does not need the context-wide design. Recommend: re-title #85 "across separate `Placed`" and drop the M6 label. |
| **Hit-testing** | Registration order is egui's hit order inside a layer, and each overlay is its own layer (F8). Each overlay root registers a **blocker** widget first (F3), so the pointer cannot reach the page through a panel. The same "which root is under the pointer" function serves dismissal, drop targets and hover (section 4, section 9). |
| **Culling** | Overlays are not clipped by the base tree's `overflow`, nor culled by the scroll viewports the anchor sits in (that is the point of an overlay). The overlay's clip is the screen. An overlay's own scroll containers cull as today. |
| **Keyboard focus** | Tab order is egui's registration order: base, then overlays in stack order, so Tab walks past the base into the overlays. A modal narrows this with `set_modal_layer` (section 6). Nodes inside overlays get `UiBuilder::id(path)` as everywhere, so ids do not depend on the layer; `focus::focus_within` works for an overlay subtree unchanged. |
| **Continuity checker** | Overlay nodes are recorded like any identified node (path, painted rect, opacity). Opening is a first appearance, closing is an absence at the end, neither is a violation. Two new invariants (section 12). |

**Mechanics (for the build, not the API).**

- `Frame` gets `roots: Vec<RootDecl>`; each root records its kind (base, scroll content, overlay), its parent root, and the scroll leaves inside it. Today `scrolls` is one flat list solved after the base; it has to be partitioned per root so that a scroll container *inside* an overlay is solved after the overlay is placed.
- D19: the overlay's slot is reserved in `roots` **before** its body is built, so overlays declared inside the body (submenus) come later in the list and are solved, placed and stacked after it.
- Layout order: base, then its scroll contents (existing), then each overlay in slot order: solve at MaxContent, resolve the anchor from nodes collected so far, `place`, solve again with a clamped size if the natural size exceeds the room (section 3), solve its scroll contents, collect.
- Draw: the `Walk` is run once per root; each overlay inside its own `egui::Area` (id from the overlay's path, tight rect via `ui.allocate_rect`) so it is its own layer.

**Recommendation.** Same tree, separate root, one `Placed`. Re-scope #85 to separate `Placed` only.

## 2. Layers and stacking

**Band to `Order`.**

| `Band` | `egui::Order` | Used for |
|---|---|---|
| `Floating` | `Middle` | Non-modal panels that sit with the page: sheets, docked flyouts. |
| `Popup` (default) | `Foreground` | Popovers, menus, selects, dialogs and their scrims. |
| `Tooltip` | `Tooltip` | Tooltips; hover cards. Never sensing. |

The drag image stays on `Order::Tooltip` (`draw.rs`, `Id::NULL.with("eggshell-drag-layer")`). It is above every `Floating` and `Popup` overlay and shares the order with `Tooltip` overlays, so the library flags it last each frame. Hover-intent tooltips are not opened while a drag is live (the library decides; nothing relies on egui doing so), so the image never competes with one.

**Stacking inside a band (corrects D18).** D18 says "stacking = band, then build order; the driver calls `move_to_top` every frame". F1 shows the second half does not produce the first: egui sorts stably on `(order, flag)`, so the final order inside a band is the order in which the layers were *first shown*. Reopening keeps the previous rank (egui documents this).

What is actually guaranteed, and enough:

1. **Band, then opening order.** A newly opened overlay is above every overlay already open in its band.
2. **Nested overlays are always above their parent** (D19, "submenus above parents"). A nested overlay is declared inside its parent's body, so it can only exist while the parent does, and so it was first shown after (or in the same frame as) the parent. In the same frame egui keeps the order the layers were first created in, which is declaration order for new layers. A parent that closes and reopens takes its children down and up with it, so the pair never reverses.
3. Every overlay is flagged every frame (F2), so a press cannot raise a parent over its submenu, and overlays stay above any plain egui `Area` the host shows in the same order.

What is **not** guaranteed: two *unrelated* overlays opened in an order that differs from their declaration order. The app opens them, so the app controls opening order; there is no use case for restacking an open overlay.

An explicit `.raise()` (re-open the layer id) is not offered. If a use appears (a dialog opened from a popover must be above it; it is, because it opens later) revisit with a repair loop (flag one layer per frame in declared order), which is possible but not needed.

**Modals.** A modal is a `Popup`-band overlay whose root is the scrim. Opening order puts it above what is open. A popover opened from inside a dialog (a select in a form) is nested in the dialog's body, so it is above it and, being above the modal layer, interactive (verified in `a_modal_layer_blocks_layers_below_it_and_not_those_above`).

**Recommendation.** Keep `Band { Floating, Popup, Tooltip }`; amend D18 to "band, then opening order; nested above parent; all flagged every frame". Add the F1 test to the build PR so a future egui that changes this is noticed.

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

**Recommendation.** Adopt the `Anchor`, `Placement` and `place` above. Bounds are the screen by default; `.boundary(Rect)` is deferred until needed (a region-scoped popover can pass `Anchor::Rect` and a clamp rect later).

## 4. Dismissal (D22, encoded once)

Open state lives on `App`; the library calls `on_dismiss` and the app closes. Everything below is **one function** run after the draw, over this frame's final rects and input; no site decides dismissal on its own.

```rust
fn dismissals(stack: &[OverlayState], input: &FrameInput) -> Vec<usize>   // overlays to dismiss, top-down
```

**Rules.**

1. **Nothing is judged while a drag gesture is live** (dnd's `Dragging`). A drag that starts inside an overlay and ends outside cannot dismiss it, and neither can a drag from the page onto it.
2. **Outside press.** On the frame a pointer button goes down (any button; F6 shows `any_pressed` is the press frame only), walk the stack **top-down**. An overlay *contains* the press if the press position is inside its root rect, or inside any overlay nested in it (declared in its body), or inside its anchor node's rect (so the trigger's own click toggles instead of dismiss-then-reopen). For each overlay in turn: if it contains the press, **stop**; otherwise (and if its `DismissOn` includes `Outside`) dismiss it. A modal also stops the walk: the scrim is the dialog's "outside", so a press on it dismisses the dialog and nothing below. Judged at the press, not the release, as Radix does, so there is no click-versus-drag ambiguity and a release outside never dismisses (F6).
3. **Esc.** The topmost overlay with `Esc` in its `DismissOn` is dismissed and the key is consumed. Esc during a live drag is egui's (it consumes it at the start of the pass to cancel the drag, so the library never sees it): the drag cancel wins and the overlay stays.
4. **Focus leaving.** If focus was inside the overlay or its anchor last frame and is now on a widget outside both, dismiss (menus and popovers; not dialogs, which trap, nor tooltips). Focus becoming *none* is not "leaving": an Esc blur or a press on a non-focusable area is handled by rules 2 and 3.
5. **Window focus lost** (`WindowFocused(false)`): dismiss every non-modal overlay. This matches dnd's cancel on focus loss.
6. **Scroll of the anchor's container: no dismissal.** The overlay follows (section 8).
7. **Nested overlays.** An outside press on a parent's area closes only the child (the parent contains the press, so the walk stops there). A press on the page closes both, child first. Esc closes the child only.
8. **Activation** (choosing a menu item, a select option) is the *app's* handler setting its state, as in D22's example. The library does not guess that an item "closes the menu".

`DismissOn` is a closed set, not flags: `Default` (Outside, Esc and FocusOut), `EscOnly` (a confirm dialog that must be answered, or an autosuggest whose outside click is the field's own concern), `Never` (tooltips, which the library closes itself).

**Order of effects in a frame.** Draw runs first (so a click on an item inside runs its handler), dismissal runs second, from the same frame's input. If the same press activates an item and is judged "inside", nothing dismisses; if an item handler closed its own overlay, the dismissal list simply no longer contains it next frame.

**Recommendation.** Adopt rules 1 to 8 as one function with a unit-test matrix (stack shapes x press positions x Esc x drag live). `on_dismiss` stays `FnOnce(&mut A)`, as in D22.

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

Items are nodes with an item role (`MenuItem`, `MenuItemCheckbox`, `MenuItemRadio`, `ListBoxOption`); the library collects them from the overlay's subtree in paint order. The existing `Select` already implements the `Listbox` behaviour by hand (`List::show`, `hold_keys`, `step`, `edge`); that code moves into the library and is deleted from `select.rs`. Right (or Enter, or hover intent) on an item that owns a submenu is the **item's** concern (`s.menu_item(..).submenu(..)` in the stock widgets), because only it knows its submenu's open state on `App`.

**Accesskit.** Roles come from the overlay: `.role(Role::Dialog | Menu | ListBox | Tooltip)`, set with `named(..)` (F9). A modal root additionally sets accesskit's modal flag (`aria-modal`). The trigger gets `expanded` and `has_popup`, as `Select` already sets `expanded`. Item roles are as above. Tooltips are described to the trigger (`aria-describedby`) rather than focused. All of this is what egui already sends for widgets; there is no accessibility work beyond roles and flags.

**Recommendation.** Adopt `Focus` and `Nav` as above. Typeahead and the submenu "safe triangle" (keeping a hover-opened submenu open while the pointer crosses to it) are post-0.1; v0.1 submenus open on click/Enter/Right and on hover after a short delay, and close when a sibling item is hovered with motion (the `moved` rule `Select` uses).

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

**Recommendation.** Own scrim, built from the two egui mechanisms above. `Modal` is a presentation of the overlay, not a second type.

## 7. Sheets, drawers, slide-in sidebars (#21) and #35

**API.**

- **Sheet (modal):** `s.overlay("settings").modal(Modal::Edge(Side::Right))`, the box sized by its classes (`w_px(380.0)`, `h_full()`).
- **Sheet (non-modal):** `s.overlay("panel").anchor(Anchor::Screen).placement(Placement::edge(Side::Right))`, in `Band::Floating`, no scrim, the page stays usable.
- **Drawer** (draggable bottom sheet, vaul): the same with `Side::Bottom` plus the app's drag handle (`sense_drag` and a paint-only translate that follows the pointer, #21 part 2). Library-side needs nothing beyond `Presence` and the existing translate; a springing helper can come later. Defer to the #21 PR; it is not an M6 requirement.
- **Pushing sidebar:** a layout pattern, not an overlay. As #21 says: animate the width at build, clip, an example. No M6 machinery.

**Enter and exit motion, and #35.** Overlays are *real nodes of the app's tree* while they exist. So their exit can be done by the app keeping them mounted, which is the mechanism #21 proposes and #35 notes as an alternative to ghosts. Library help is one small, pure-ish type:

```rust
pub enum Shown { Open, Closed }

/// Phase and eased progress of an open/close animation, driven from egui's animation clock.
pub struct Presence { /* phase: Closed | Entering | Open | Exiting; progress: Progress */ }
impl Presence {
    pub fn drive(ctx: &Context, key: impl Key, shown: Shown, duration: Duration, ease: Ease) -> Self;
    pub fn mounted(&self) -> bool;     // anything but Closed: declare the overlay
}
// on the overlay:
.presence(presence, Entrance::Slide(Side::Right))   // or Fade, Zoom, SlideFade
```

- `drive` uses `ctx.animate_value_with_time` (the clock the pushing-sidebar pattern already uses), so it requests repaints only while moving and is idle otherwise.
- `.presence()` makes the library (a) fade the scrim by `progress`, (b) translate the content box by `(1 - progress) x its own size` toward the edge, **resolved at draw time** from the node's final rect (the same trick as `paint_centered_at`, since the size is unknown at build), both paint-only (layout is final from the first frame), and (c) make the overlay `inert` while `Exiting`, so a closing dialog takes no input, but it **keeps blocking the page** until it is gone. Re-opening during the exit just flips `shown` and the progress reverses from where it is; no special case, and never both a ghost and a live node.
- The app declares the overlay while `presence.mounted()`, and `on_dismiss` sets its `shown` to `Closed`.

**Do we need ghosts (#35) for M6?** No. Ghosts exist for nodes that *vanish because their data did* (todo's Remove): there is no state left to keep them mounted from. An overlay's existence is an app flag, so the app can hold it. Ghosts are also the wrong tool for modals: a ghost takes no input, so the page would be clickable through a fading scrim, whereas a mounted-but-inert exiting overlay keeps blocking.

Cost of not using ghosts: the app (or a stock widget) carries a `Shown` and calls `drive`. That is one line per overlay and is how the pushing sidebar already works. If #35 lands later, nothing here changes; ghosts can additionally serve overlays that the app removes abruptly.

**Recommendation.** Build `Presence` with the sheet PR (small, and used by both sheet and dialog). Decouple M6 from #35: say so on #35. The "enter" side needs no machinery beyond the same helper. Motion choice, duration and easing are the user's (section 11).

## 8. Scroll interplay

**An overlay anchored to a node inside a scroll container.**

- **Follow, free.** The overlay is solved after the scroll content, with the content already at this frame's offset (§11: "laid out at the offset egui holds"), so the anchor rect is current. The overlay is placed against it in the same frame; there is no lag to hide and no reposition event to subscribe to. (Floating UI needs `autoUpdate` for this; the same-tree model does not.)
- **Hide when clipped.** If the anchor's *visible* part (its rect cut by its ancestors' clips and the scroll viewport; the same clip chain `dnd::targets` already computes) is empty, `place` reports `Detached` and the overlay is not drawn, takes no input and does not take part in dismissal. Its open state and its place in the stack are kept, so scrolling back shows it again. A partially visible anchor keeps the overlay (placed against its full rect).
- **No dismissal on scroll.** Menus and popovers stay open; the wheel over the *page* while a popover is open scrolls the page, and the popover follows. Native context menus that close on scroll are an app choice (the app can compare the viewport offset, or use `Anchor::Point`).
- **`Anchor::Point` and `Pointer` do not follow.** They are screen-space and fixed, as native context menus are. Content that wants a menu to stay on a content point stores content coordinates and converts each frame with `Viewport::to_screen` (it exists) into `Anchor::Rect`.

**Overlays containing scroll containers.** Supported by construction: the overlay's scroll leaves are solved after the overlay is placed (section 1). A tall list: pass 1 at MaxContent, `place` reports `available`, pass 2 clamps the overlay root's max height to it; the list's scroll container (`grow`, `min_h_0`) takes the clamped height and scrolls. The wheel goes to the overlay under the pointer (its layer rect is tight, F8) and, at the list's end, does *not* chain to the page (the page is another layer), matching native popups.

**Recommendation.** Follow, hide when detached, never close on scroll.

## 9. Drag and drop interplay

- **One `Placed`, one gesture.** A source in an overlay and a target in the page (or the reverse, or both in overlays) are in the same `nodes`. `Dragging`, `index_hint` and `DragEnd` need no change.
- **Resolver.** `dnd::targets` walks from node 0. It becomes a walk over every root in stack order, and a target is *reachable* at pointer `p` only if the topmost root whose **blocker** contains `p` is the target's own root (floating overlay: its root rect; modal: the whole screen; page: everything else). That function, `Placed::root_at(p)`, is the same one dismissal uses (section 4) and the one hover uses, so the three cannot disagree (CLAUDE.md: encode priority once). It replaces `dnd.md`'s suggestion of consulting `ctx.layer_id_at`: the library owns the overlay rects and does not need to ask egui, which also works in the headless harness.
- **Drag image.** On `Order::Tooltip`, above `Floating`/`Popup` overlays, flagged last (section 2). Unclipped.
- **Dragging over an open popover.** The popover is not dismissed (dismissal is press-based, and not judged during a drag). It blocks the targets under it. If it declares a target of its own it is a normal target.
- **Light dismiss and a drag.** Answers `dnd.md` M6 requirement (3): a payload's drop **does not** dismiss. egui's click-away fires on the release; ours fires on the press and is suppressed while a drag is live, so a drop onto a popover or a release outside it is inert. A drag whose source lives in an overlay that is dismissed mid-drag (by a window blur) ends as `SourceGone`; Esc cancels the drag first.
- **Auto-scroll** in an overlay's scroll container works as in the page (`EdgeScroll` is per scroll node).
- **Spring-loaded opening** (hover a trigger during a drag to open it) is post-0.1.

**#85.** Not needed. Keep it open for separate `Placed`, and note that the "which root is under the pointer" function is the part that would be lifted to the context if #85 is built.

**Recommendation.** One resolver, `root_at`, shared by dnd, dismissal and hover.

## 10. Migrations

**`Select` (owned): migrate in M6.** The brief in `select.rs` says "until eggshell has overlays of its own". It uses `egui::Popup` with its own open memory, close behaviour, Esc, click-away, width and gap, and hand-painted rows. Moving it gives:

- one dismissal path (an Esc in a dialog closes the select first, then the dialog: correct nesting, which egui's `Popup` and eggshell's dismissal could not agree on);
- the list is eggshell nodes: identified, keyed, inspectable in `Headless` by path, covered by the continuity checker and the debug overlay, themed by the style contract and tested with `get_by_role(ListBoxOption)`;
- flip, clamp and a scrollable list for long option sets (`place`'s `available`); egui's popup has neither the clamp-to-room nor our scroll;
- it works inside a modal and inside a scroll container (anchor follows);
- it deletes `List::show`, `hold_keys`-for-popup and `Popup` plumbing (about 150 lines), replaced by `Focus::Stay` + `Nav::Listbox` shared with autosuggest and the combobox.

Select's open state moves from egui popup memory to `App`? **No:** a stock widget with no `App` state keeps it in egui temp data (as it does now, `highlight_id`), and the overlay's `on_dismiss` clears it. D22 is about application-owned overlays; stock widgets own their own. State this in the build PR.

**Tooltips: migrate, last, subject to the look decision.** `tipped` uses `Response::on_hover_text`. egui's tooltip is a plain popup frame, so the shadcn tooltip (inverted colours, small radius, arrow, short delay) cannot be had from it. A migrated tooltip: hover-intent state in egui temp data keyed by the trigger id (delay from `Style::interaction.tooltip_delay` and grace from `tooltip_grace_time`, so timing is unchanged), a `Band::Tooltip` overlay, `Focus::None`, never sensing, `DismissOn::Never` (closed by the library when the pointer leaves, on press or Esc, or when a drag starts), described to the trigger. Buys: the style-contract look and arrow, rich content (a label with a `kbd` chip), the same placement and flip, visibility in `Headless`, no stray egui layer. Costs: re-doing the hover-intent timing and the disabled-hover case (`on_disabled_hover_text`), both small.

If the user does not want a different tooltip look in v0.1, keep egui's and defer this PR: nothing else depends on it. The tooltip is the least coupled migration and goes last.

**Recommendation.** Migrate `Select` in the same milestone (PR 5). Migrate tooltips in PR 7 if the user picks a shadcn look; otherwise defer them past 0.1.

## 11. Look

Look decisions are the user's. The note proposes **which style-contract entries exist**, not their values; shadcn's component sources are the reference, as in the `shadcn defaults` rule.

New entries in `Style` (each a `Look` with a rest state, border, radius, shadow, padding, as `SelectStyle::popup` is):

| Entry | Covers | Open choices |
|---|---|---|
| `popover` | popover box | background, border, radius, shadow, padding, default width |
| `menu` | menu box and `item`, `item_destructive`, `separator`, `label`, `shortcut`, check/radio indicators, submenu chevron | item height and padding, highlight colour (focus vs hover), shortcut text style |
| `dialog` | box, header/footer, title and description text, close button | width, radius, padding, footer layout, close button present or not |
| `sheet` | box per side, border side, header/footer | default width, border, shadow |
| `tooltip` | bubble, arrow | **inverted vs popover look**, radius, padding, arrow on or off, delay |
| `scrim` | modal backdrop | colour/alpha (shadcn `bg-black/50`), blur is out of scope (#40, known difference) |
| `motion` | open/close duration, easing, entrance per kind | the numbers and curves |

`SelectStyle::popup`, `popup_radius`, `popup_padding`, `item*` are reused by the migrated select; `SelectStyle` keeps its fields and gains none.

The shadcn preset crate fills these from the shadcn tokens, as it does for the controls. The **neutral default** (no preset) gets plain values from the egui style (no change of default look without a decision).

**Needs the user's choice, with candidates rendered at the size they will be used beside what surrounds them:** the tooltip look (inverted or popover; arrow or none), menu item highlight (hover vs focus ring), dialog and sheet geometry, scrim alpha, all durations and easings, close-button presence. Offer: shadcn's values as the recommended candidate for each, as rendered images, in the build PRs that introduce them, not decided here.

**Known CSS differences that touch overlays** (#40): no rounded clipping, so a scrolling list inside a rounded popover needs its own inner radius; no group opacity, so fading a whole overlay fades each shape separately (the `Presence` fade is a scrim fade plus per-shape alpha; overlapping translucent shapes inside a dialog could show through during the fade). Pre-blend opaque surfaces as the controls already do.

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
    .presence(Presence, Entrance)       // enter/exit motion (section 7)
    .on_dismiss(|app| ..)               // FnOnce(&mut A), D22
    .children(|s| ..);

s.overlay(key).modal(Modal::Center)     // OverlayChain<Modal>: no anchor or placement
    .focus(..).dismiss_on(..).role(Role::Dialog).named("..")
    .presence(..).on_dismiss(..).children(|s| ..);
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
let sheet = Presence::drive(s.ctx(), "settings-sheet", self.sheet, Duration::from_millis(200), Ease::Out);
if sheet.mounted() {
    s.overlay("settings-sheet")
        .modal(Modal::Edge(Side::Right))
        .presence(sheet, Entrance::Slide(Side::Right))
        .role(Role::Dialog).named("Settings")
        .on_dismiss(|app| app.sheet = Shown::Closed)
        .children(|s| sheet_box(s, |s| { /* w_px(380.0).h_full() */ }));
}
// non-modal: .anchor(Anchor::Screen).placement(Placement::edge(Side::Right)).band(Band::Floating)
```

## 14. Issues to file and build order

**Issues to file** (all linked from #1):

1. M6a: overlay core: roots, `place`, `Anchor`/`Placement`, band layers, blocker, stacking (this note's sections 1 to 3).
2. M6b: dismissal, focus modes, `Nav` (sections 4 and 5), `root_at`.
3. M6c: modal presentation, scrim, focus trap (section 6).
4. M6d: `Presence` and sheets (section 7; replaces the M6 part of #21; also records "#35 not required").
5. M6e: owned `Select` onto overlays (section 10).
6. M6f: stock menu widgets (`menu_item`, separators, submenu protocol), `menu` example, autosuggest test.
7. M6g: tooltips onto overlays (needs the look decision).
8. dnd: `root_at` in the resolver (part of M6b; a dnd test that a target under a popover is unreachable and one inside is).
9. Amend D18 (stacking) and D22 (stock widgets own their state) in `design.md`; re-title #85.
10. Continuity: `OffScreen`, `Reflip`. Fuzz: `overlay_session`.
11. Post-0.1, label it: typeahead, submenu safe triangle, spring-loaded opening, arrows, `.boundary(Rect)`, drawer drag-to-dismiss.

**Build order** (each PR small, tested, with the user's hands on any UI change before approval, per CLAUDE.md):

| PR | Content | Why this position |
|---|---|---|
| 1 | `src/overlay.rs`: `place` + tests (promote the prototype); `Anchor`, `Placement`, `Band` types; amend D18 | Pure, reviewable alone, nothing integrated |
| 2 | Roots in the pipeline: `roots` partition, slot reservation (D19), solve/place/collect, `Placed::overlays`, `s.overlay(..).children(..)` for floating overlays, per-overlay `Area` with blocker, flag-every-frame, stacking tests (F1, F2), `Headless` accessors, a minimal `menu` example | The tree model; everything else builds on it |
| 3 | Dismissal (`dismissals`, `root_at`, `on_dismiss`), `Focus` modes and restore, `Nav::Menu`/`Listbox`/`Submenu`, accesskit roles; dnd resolver uses `root_at` | Behaviour on a working core |
| 4 | Modal: scrim, `set_modal_layer`, focus trap, scroll-lock test | Needs 3 |
| 5 | `Select` migrated; autosuggest test | First consumer; proves `Stay`/`Listbox` |
| 6 | `Presence`, sheets (modal and non-modal), `menu`/`sheets` examples; stock menu widgets, `chat_shell` menus | Look decisions land here with rendered candidates |
| 7 | Tooltips migrated (after the look decision); continuity invariants; `overlay_session` fuzz | Last: least coupled |

PRs 1 and 3's pure parts can proceed while the look entries (6) wait for the user. The user merges each.

## User decisions needed

1. **Stacking (D18 amendment).** Accept "band, then opening order; nested above parent" in place of build order? (Section 2. A repair loop is the alternative if build-order authority matters.)
2. **Tree model.** Same tree, one `Placed`; re-scope #85 to separate `Placed`. (Section 1.)
3. **Modal approach.** Own scrim plus `set_modal_layer`, not `egui::Modal`. (Section 6.)
4. **#35.** Confirm M6 does not use ghosts and `Presence` is the way overlays animate out; #35 stays an independent v0.1 candidate. (Section 7.)
5. **Dismissal details.** Judged at the press (not the click); trigger press counts as inside; dismiss on window focus loss; `Esc` to the topmost only. (Section 4.)
6. **Scroll.** Anchored overlays follow and hide when clipped, never close on scroll. (Section 8.)
7. **Tooltips.** Migrate to an overlay with the shadcn look, or keep egui's for v0.1. (Section 10.)
8. **Stock widgets' state.** Select and tooltips keep their own state rather than `App` (a narrow exception to D22). (Section 10.)
9. **All looks** (section 11): tooltip style and arrow, menu highlight, dialog and sheet geometry, scrim alpha, close button, durations and easings. To be shown rendered in the PRs that add them.
10. **Scope of v0.1 stock widgets:** menu items with submenus (needed by the reference shell?), or only the overlay primitives plus `Select`? Typeahead and the safe triangle are assumed post-0.1.
11. **Submenu open gesture:** hover-after-delay in addition to click/Right/Enter, or click/keyboard only for v0.1.
