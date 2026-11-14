# UI Layout Framework: taffy + egui with explicit identity

Status: design draft, revision 4 (2026-11-06). Written for an implementing agent.
Targets: **taffy 0.15.x**, **egui ≥ 0.33** (needs `UiBuilder::id`).
API names marked ⚠ have not been checked against the pinned crate versions. Check them before relying on them.
Code in this document is design sketch. Shapes marked *(compiled)* were compiled and run on rustc 1.97 against stand-in types (not real taffy/egui).

---

## 1. Summary

The application owns all state in plain Rust structs. Every frame the UI is rebuilt in three phases:

1. **Build.** Walk application state with a nested builder (`b.div(tw, |b| …)`) and emit a taffy tree. Every identified node has an explicit key path built from names and data keys, and declares its own measure (leaves) and draw (interactive nodes) where it is created. Styles are written in Tailwind vocabulary and hold fully resolved values.
2. **Layout.** taffy lays out the base root, then each overlay root and scroll root, producing absolute rects.
3. **Draw.** Walk the tree in pre-order. The library paints boxes and display content. For every identified node it opens a child `egui::Ui` whose ID is the hash of the node's key path and calls that node's draw closure with `&mut App`.

egui supplies interaction (input, focus, text editing, IME, popups). taffy supplies layout. The key path ties layout, rendering, egui state and application data together.

## 2. Goals and non-goals

**Goals**
- Compile-time checks where they matter: a leaf cannot be declared without its measure, and an interactive node cannot be declared without its draw.
- Stable identity derived from application data, never from draw order.
- Content-dependent layout (min-content, max-content, height-for-width) in one taffy pass per root.
- Tailwind vocabulary for layout, box paint and text, with inherited text classes.
- The library knows nothing about the application's theme: it consumes resolved colors and text specs.

**Non-goals**
- Overlapping, user-reorderable document windows. Stacking is application-determined.
- Retained widget trees, diffing or reconciliation.
- A component trait or ID enums. Components are plain functions (D9).
- Backdrop blur (egui cannot blur what is behind a layer; see §13).
- Rounded clipping and group opacity (egui clips to rectangles and fades shape by shape; see "Differences from CSS" in the crate docs).

## 3. Settled decisions

| # | Decision | Rationale |
|---|---|---|
| D1 | Rebuild the taffy tree every frame. | No reconciliation. taffy is fast enough. Allocations are reused. |
| D2 | One taffy tree per frame. Overlays and scroll contents are additional roots in the same tree. | Content-dependent sizing is solved per root in one pass. Only leaves are measured. |
| D3 | **Identity is a key path.** `scope`, identified groups and roots each push a key; identified nodes add their own. A key is any value of the `Key` trait (`Hash + Debug + 'static`, implemented for every such type): a name, a data key, or both (`("pin", pin_key)`). | Explicit identity without a type per component. Keys stay next to the code that uses them. |
| D4 | Dynamic items use stable data keys, never `Vec` indices. | Index IDs reintroduce reorder bugs. |
| D5 | **A node's egui ID is the hash of its key path.** Uniqueness is checked: debug builds assert that no path occurs twice per frame and report the readable path. Each segment hashes the key's type name with its value, so keys of different types never collide (`#[derive(Hash)]` alone does not hash the type). | One identity system for layout, egui widget state and local state. Collisions surface on the first frame (the reference shell hit one). |
| D6 | The builder is **nested closures**: `b.div(tw, \|b\| …)` attaches children to the current parent automatically. | Reads like HTML. Inherited text classes resolve at build time. No `Vec<NodeId>` juggling. |
| D7 | `b.scope(key, \|b\| …)` pushes a key. `div_id`, `scroll`, `region`, `overlay` and virtual-list rows scope their children under their own key. | Keys only need to be unique among siblings. |
| D8 | Per-frame build data lives in a private `BuildState` (plus the frame's closures). The only way to get a `Builder` is `LayoutState::build(\|b\| …)`, which returns `Built`. `Built::layout` returns `Placed`, and `Placed::draw` draws. | Module privacy plus typestate enforce the tree's invariants and the build → layout → draw order. |
| D9 | **No component trait and no ID enums.** A component is a function that builds a subtree; each identified node carries its own draw closure. | Building, measuring and drawing a control happen in one place. No routing code. |
| D10 | **Draw closures own their captures and receive `&mut App`.** They narrow to their data with captured keys (`app.doc.block_mut(k)`). Reusable components receive a lens (`impl Fn(&mut App) -> Option<&mut T> + Copy + 'static`). | The borrow checker accepts draw closures created during build, because they hold no borrow of the app. |
| D11 | State shared between nodes as application meaning lives on `App`. State local to the UI lives in egui memory under an ID derived from a key path: `ui.make_persistent_id(salt)` in draw, or an ID computed once in build (`b.id_of(key).with(salt)`) and captured by the draw that writes it. | Uses egui's memory model, made safe by stable paths. |
| D11a | **Eggshell's own state lives in `LayoutState`; egui keeps egui's.** The overlay stack, listbox highlight, layout animation, typing note and layout area are in the `LayoutState` (`kept.rs`), so two states in one egui context (a bar and a page) cannot overwrite each other. egui's own state (focus, text-edit memory, the clock) stays in egui, and diagnostics (debug overlay, continuity recorder) are context-wide by design, and so is one flag, "a menu that swallows Tab was open last pass" (`overlay_input::HoldTab`): egui sets the pass's focus move in `begin_pass`, before any layout, so only a context-wide hook can cancel it for every layout. State left in the context under `root_id()` keys is a bug of this class: see #152. | Both bugs of the class (overlay stack, listbox highlight) were two layouts sharing one context-wide entry. |
| D12 | A node's draw `Ui` is created with `UiBuilder::scope_id(path_hash.with("eggshell-draw"))`, so the IDs of widgets inside it are independent of layer and parent `Ui`. The node's own interaction uses `path_hash` itself (`NodeUi::node_id`); the draw `Ui` cannot share it, because egui registers a `Ui`'s scope ID as a widget and that would overwrite the node's border-box widget. | Build and draw compute identical IDs from the path alone. |
| D13 | Styles are written with a `Tw` builder in Tailwind vocabulary. **`Tw` holds resolved values only**: `Color32`, `TextSpec`, points. Theming is an application concern (Appendix A). | No theme abstraction in the library. Since the tree is rebuilt every frame, resolving at build time is late enough. |
| D14 | One spacing unit = 4 points. `.p(1.5)` = `p-1.5` = 6 pt. | Tailwind's scale (0.25 rem at 16 px). |
| D15 | `tw()` starts as `Display::Block` (CSS default). `flex()` / `grid()` must be explicit. | Tailwind code reads the same as on the web. |
| D16 | `Tw` carries layout, box paint and text classes. Text classes are inherited down the tree, as in CSS. | One class list per element, as in HTML. |
| D17 | Floating chrome inside a region is **absolutely positioned** in the base tree (`absolute`, `inset`, `z`). **Overlays** are only for popups that need their own layer and must escape clipping. | Matches the CSS it replaces. Keeps chrome clipped to its region. |
| D18 | Overlays declare a key, a `Band` (maps to `egui::Order`), an `Anchor` and a `Placement`. Stacking = band, then opening order; a nested overlay is above its parent. A per-frame repair step flags the misplaced suffix of the stack (`docs/overlays.md` §2). | egui orders layers by first show, so flagging every frame does not give build order. |
| D19 | Overlay slots are reserved before the overlay's subtree is built. | A parent popup is laid out and stacked below its submenus. |
| D20 | Display primitives (text, icons, rules, images, custom paint) are stored on the node and measured and drawn **by the library**. Interactive leaves are measured and drawn by the application. | Labels need no `match` arms. Measure and draw of display content cannot drift apart. |
| D21 | Measurement and drawing of every interactive primitive go through **one shared sizing helper**. | Prevents drift between predicted and actual size. |
| D22 | Overlay open/close state lives on `App`. Each overlay declares `on_dismiss: impl FnOnce(&mut App)`, run by the library on an outside click or Esc. | No lookup from popup state to overlay identity. |
| D23 | **State variants change paint only**: background, border/ring color, text color, opacity, underline, shadow. Never layout or font size. Enforced by a `Paint` builder type that has no layout methods. | Hover never requires measuring or laying out again. Nothing is a frame behind. |
| D24 | **The library owns interaction for identified nodes that declare a sense** (explicitly, or implied by a variant). It interacts once, with the node's border box, before painting. The node's draw reads the result with `ui.node_response()` and never calls `ui.interact` with the node's own ID. | One hit test per node. Hover styling and clicks always agree. |
| D24a | **Every box registers a hit-test widget in paint order (#167).** A node that paints a background or border and does not sense registers a non-focusable click widget (and a drag one unless an eggshell ancestor senses drags or scrolls by dragging), so egui's hit test, the only one, agrees with the paint. A press a box wins goes along eggshell ancestry to the nearest ancestor that senses clicks, nearest handler only, and into its `node_response()`; hover reaches every sensing ancestor. egui-native content is never a routing target: a box over it occludes it, `pointer_events_none` lets the press through. | One hit tester, so what is painted is what is hit, and the press latch gives capture across eggshell and egui gestures; replaces #102's overlap and ancestor reasoning. |
| D25 | **Regions**: `b.region(key, tw, \|b, size\| …)` builds a subtree *after* the outer layout, with the region leaf's solved size, as a separate root laid out at exactly that rect. A region's size never depends on its contents. | Build can depend on solved size (container queries) with no previous-frame lag. |
| D26 | **Choosing what fits**: inside a region, `b.max_content_width(\|b\| …)` builds a candidate into a scratch tree and returns its max-content width, so the app can pick the first of several alternatives that fits. | SwiftUI `ViewThatFits`-style fallback ladders (e.g. pane title bars) without guessing widths. |
| D27 | `divide_x` / `divide_y` are container paint: hairlines drawn between consecutive in-flow children. | Grouped form sections (macOS style) need them everywhere. Earlier marked "not expressible"; it is, since the library knows child rects. |
| D28 | **Nodes declare their own measure and draw**: `b.custom(key, tw).measure(m).ui(draw)` (`measure` optional: the size then comes from classes; `leaf` is the crate-private primitive under it). Measure closures may borrow application data for the frame (`'f`) and are dropped when `layout` returns. Draw closures own their captures (`'static`) and run with `&mut App`. | A leaf cannot lack a measure, an interactive node cannot lack a draw, and nothing dispatches by ID. |
| D30 | **A component takes `&mut Scope`.** `Scope` is the builder that `b.scope(key, …)` and `div_id(key, …)` hand to their bodies. It dereferences to `Builder`, so the whole builder API is available and a `&mut Scope` passes where a `&mut Builder` is wanted; the reverse does not compile. A component function takes `&mut Scope<'_, '_, A>`, so each use sits under a key its caller chose. Each component has its own key enum. | Keys a component declares cannot leak into its caller's scope or collide with siblings, and two uses in one parent need two instance keys, all checked at compile time. A `Scope<A, K>` with typed child keys was rejected: it makes helper functions awkward. |
| D31 | **Owned widgets are chained builders that start from `b`**: `b.button(key, "Save").variant(PRIMARY).on_click(\|app\| …)`. `b.<widget>(key, …)` returns a `#[must_use]` builder that borrows `b`; options come first and a terminating call adds the widget: `on_click` (button, and anything else that only triggers), `on_change(|app, value| …)` on every control that holds a value (checkbox, switch, toggle button, radio, segmented, slider, select, stepper, text field). The control is given the current value when it is built and shows it; `on_change` receives the value the user chose, and the application stores it. A checkbox or switch over a `bool` needs nothing more; over any other two-valued type it takes `.values(off, on)` first. `add()` adds a button that does nothing. Hooks come before the terminator: `on_response(|app, &Response| …)` on every control, `when_submitted` and `when_blurred` on the text field and the stepper. A builder dropped without its terminator is an `unused_must_use` warning, which CI denies. | The widget is named once; a forgotten terminator cannot pass CI. |
| D32 | **Styles are plain values**: `fn card() -> Tw`, composed with `merge` (the later list wins per property, as `cn()` / `tailwind-merge` do). No class-string macro. | A `Tw` is a pure value, so styles compose and can be checked on their own. Class methods are not `const fn`, so a style is a function. |
| D33 | **Size animates in layout, not in paint**: `b.collapsible(key, Shown)` is a clipping wrapper whose size along one axis is its content's natural size times its presence progress, found by laying the tree out twice in a frame with a wrapper in motion; neighbours reflow, the content is clipped and never scaled. | No group transforms (#40), so a scaled paint would stretch text; the two-pass solve needs no measured history and has no one-frame lag. |
| D34 | **Reduced motion is one switch**: egui's `style.animation_time == 0` makes every eggshell motion (state transitions, slides, presence, size) instant, through `transition::allowed`. | One place decides, so a new kind of motion cannot forget it. |
| D29 | **Keys are local and readable.** Each module opens its own scope; keys are names and data keys; keys referred to from elsewhere (anchors, local state) are computed once (`b.id_of`) and passed on. | Typos and collisions are confined and caught by the duplicate check. |

## 4. Identity

Identity is a **path of keys**. Every `scope`, identified group, scroll area, region, overlay and virtual-list row pushes one key. Every identified node adds its own key. A key is any value of the [`Key`] trait, `Hash + Debug + 'static` (implemented for every such type; no `Eq` or `Clone` needed): a name (`"title"`), a data key (`BlockKey(2)`), or both (`("pin", pin_key)`).

```rust
b.scope("props", |b| {
    b.scope(("block", blk.key), |b| {
        b.custom("title", …);                      // path: /"props"/("block", BlockKey(2))/"title"
        for p in &blk.pins {
            b.scope(("pin", p.key), |b| {
                b.custom("name", …);               // path: /"props"/("block", BlockKey(2))/("pin", PinKey(3))/"name"
            });
        }
    });
});
```

The node's egui ID is the running hash of its path (D5): `egui::Id` of the parent's path combined with the key, segment by segment.

Rules:
- **Keys come from data, never from position** (D4). Lists use data keys (`BlockKey`, `RevKey`), not indices.
- **Uniqueness is required only among siblings in one scope.** The path makes it global. `div_id` and the roots scope their children under their own key, so keys inside a row only need to be unique within that row.
- **Keys of different types never collide.** Each segment hashes `(std::any::type_name::<K>(), key)`. `#[derive(Hash)]` alone hashes an enum's variant index and fields, not its type, so without the type name `Appearance::Light` and `Lettering::Basic` (both variant 0) would collide as bare keys in the same scope; the reference shell hit exactly this. The same holds for `1u32` and `BlockKey(1)`. `type_name` (not `TypeId`) keeps IDs stable across runs, so state egui persists survives a restart. A collision that remains is a genuine duplicate: same type, same value, same scope.
- **`Key` is the one bound.** `Hash` gives the id (with the type name mixed in), `Debug` the readable path, `'static` makes keys owned values. Every signature that takes a key says `impl Key`.
- **Components get their own key enum and an instance key.** A component function takes `&mut Scope` (§6), and its parts are the variants of one enum, so they cannot collide with a sibling's keys whatever they are called; the caller names the instance with `b.scope(Instance::Header, |s| header(s, …))`. Dynamic lists use data keys.
- **Duplicates are detected.** In debug builds the builder asserts that no path occurs twice in a frame, and reports the readable path (built from each key's `Debug` text, e.g. `/"viewport"/Settings/"scroll"/Basic`).
- **Only identified nodes get keys:** nodes with a draw (interactive or tooltip), anchors, scroll areas, regions, overlays, virtual-list rows. Pure decoration is anonymous (`text`, `glyph`, `rule`, `div`).
- **Refer to another node with `b.id_of(key)`**, evaluated in the scope where that node is (or will be) declared. Used for overlay anchors and for reading local state at build time (D11). Keep such keys in one place (a constant, or compute the ID once and pass it on) so a typo cannot split them.

## 5. Builder

### 5.1 Types

```rust
/// Build data. Private to the layout module (D8). The tree persists across frames
/// (allocations reused); the closures of one frame live in `Frame`.
struct BuildState {
    tree: TaffyTree<Node>,
    #[cfg(debug_assertions)]
    seen: HashSet<egui::Id>,
}

/// taffy node context.
struct Node {
    id: Option<egui::Id>,     // hash of the key path (identified nodes only)
    paint: BoxPaint,          // background, border colors, radius, ring, shadow, opacity
    text: TextStyle,          // fully resolved (own classes merged over inherited)
    prim: Option<Prim>,       // display content drawn by the library
    measure: Option<u32>,     // index into Frame::measures (application leaves)
    draw: Option<u32>,        // index into Frame::draws
    z: i16,
    flags: NodeFlags,         // clip children, inert, sense, variants, …
}

pub enum Prim {
    Text(String),
    Icon(Icon),
    Image(egui::ImageSource<'static>),
    Paint(Box<dyn Fn(&egui::Painter, Rect)>),
}

/// One frame's closures. `'f` is how long the frame's build may borrow application data.
struct Frame<'f, A> {
    measures: Vec<Box<dyn Fn(&Measure) -> Size<f32> + 'f>>,       // dropped at the end of layout
    regions: Vec<Box<dyn FnOnce(&mut Builder<'_, 'f, A>, Vec2) + 'f>>,
    draws: Vec<Box<dyn FnOnce(&mut A, &mut egui::Ui)>>,             // own their captures; run at draw time
    overlays: Vec<Overlay<A>>,                                      // includes each overlay's on_dismiss
    scrolls: Vec<ScrollRoot>,
}

/// Generic over the application type `A`.
pub struct Builder<'b, 'f, A> {
    st: &'b mut BuildState,
    frame: &'b mut Frame<'f, A>,
    path: egui::Id,                // hash of the current key path
    #[cfg(debug_assertions)]
    names: String,                 // readable path for diagnostics
    kids: &'b mut Vec<NodeId>,     // children of the current parent
    inherited: TextStyle,          // text classes in effect here
}
```

### 5.2 Entry point and invariants

```rust
pub struct LayoutState { st: BuildState }

impl LayoutState {
    /// The ONLY way to obtain a Builder. Clears all per-frame state first.
    /// `'f` covers this borrow and the application data the frame's measure and region closures capture.
    /// The root text style is `with_base_text` if set, else `TextStyle::from_egui(ctx)`.
    pub fn build<'f, A>(&'f mut self, ctx: &'f egui::Context, f: impl FnOnce(&mut Builder<'_, 'f, A>)) -> Built<'f, A>;
    /// Replaces the egui-derived default root text style for every build.
    pub fn with_base_text(self, base: TextStyle) -> Self;
}

pub struct Built<'f, A> { /* &mut BuildState, Frame<'f, A>, base root */ }
impl<'f, A> Built<'f, A> {
    /// Calls each leaf's measure, runs regions, lays out every root. Drops the measure and
    /// region closures (ending their borrows of application data); keeps the draw closures.
    pub fn layout(self, area: Rect) -> Placed<A>;
}

pub struct Placed<A> { /* owned: draw lists per root, overlay placements, draw and dismiss closures */ }
impl<A> Placed<A> {
    /// Pre-order per root: interaction, box paint, primitive, then the node's draw closure
    /// with `&mut A`. Afterwards, runs `on_dismiss` for overlays clicked outside or Esc'd (D22).
    pub fn draw(self, app: &mut A, ui: &mut egui::Ui);
}
```

| Invariant | Enforced by |
|---|---|
| Tree empty at start of build | `LayoutState::build` is the only `Builder` constructor and clears first |
| Every identity is a full key path | Tree is private. Only builder methods create nodes |
| No duplicate paths per frame | `seen` check (debug), with the readable path in the message |
| Every leaf has a measure; every interactive node has its draw | `leaf` / `div_id` / `span` / `icon` require them |
| Every overlay and scroll root is complete | Assertion at the end of `build` |
| Build → layout → draw | Typestate: `Built` only from `build`, `Placed` only from `layout` |
| Draw may mutate app state | `layout` consumes `Built`, dropping every closure that borrows app data. Draw closures own their captures (`'static`) and receive `&mut A` as an argument |

### 5.3 Methods

All methods attach the new node to the current parent. `Tw` is defined in §7. `K` stands for `impl Key` (a key segment). `S` is `Scope<'_, 'f, A>`, a `Builder` known to sit under a key. `D` stands for `impl FnOnce(&mut A, &mut egui::Ui) + 'static` (a draw).

```rust
impl<'b, 'f, A> Builder<'b, 'f, A> {
    // ---- identity
    pub fn scope(&mut self, key: K, body: impl FnOnce(&mut S));
    pub fn id_of(&self, key: K) -> egui::Id;                      // a node's ID in this scope (anchors, state)

    // ---- containers
    pub fn div(&mut self, tw: Tw, body: impl FnOnce(&mut Builder<'_, 'f, A>));
    pub fn div_id(&mut self, key: K, tw: Tw, body: impl FnOnce(&mut S)) -> NodeHandle; // children scoped under key

    // ---- identified leaves
    pub fn custom(&mut self, key: K, tw: Tw) -> CustomChain;  // [.measure(f)] [.named(n)] [.tooltip(t)] [.on_response(f)] .ui(|app, ui| …)
    pub fn span(&mut self, key: K, tw: Tw, text: impl Into<String>) -> NodeHandle;  // library measures/paints the text
    pub fn icon(&mut self, key: K, tw: Tw, icon: Icon) -> NodeHandle;              // library measures/paints the icon

    // ---- owned widgets: each returns a #[must_use] builder; options, then the terminator
    pub fn button(&mut self, key: K, face: impl Into<Face>) -> ButtonChain;          // .on_click(f) | .add()
    pub fn toggle(&mut self, key: K, face: impl Into<Face>, current: T) -> ToggleChain<T>; // [.values(off, on)] .on_change(|app, T|)
    pub fn checkbox(&mut self, key: K, label: impl Into<String>, current: T) -> CheckboxChain<T>; // [.values(off, on)] .on_change(|app, T|)
    pub fn switch(&mut self, key: K, label: impl Into<String>, current: T) -> SwitchChain<T>;     // [.values(off, on)] .on_change(|app, T|)
    pub fn radio_group<T>(&mut self, key: K, current: T, options: impl IntoIterator<Item: IntoChoice<T>>) -> RadioGroupChain; // .on_change(f)
    pub fn segmented<T>(&mut self, key: K, current: T, options: impl IntoIterator<Item: IntoChoice<T>>) -> SegmentedChain; // .on_change(f) | .on_change_with(f): f also gets &egui::Context
    pub fn slider(&mut self, key: K, value: f64, range: RangeInclusive<f64>) -> SliderChain;            // .on_change(f)
    pub fn select<T>(&mut self, key: K, current: T, options: impl IntoIterator<Item: IntoChoice<T>>) -> SelectChain; // .on_change(f) | .on_change_with(f)
    pub fn stepper(&mut self, key: K, value: f64, range: RangeInclusive<f64>) -> StepperChain;          // .on_change(f)
    pub fn text_field(&mut self, key: K, text: impl Into<String>) -> TextFieldChain;                    // .on_change(|app, String|); .when_submitted(f), .when_blurred(|app, Blur|)

    // ---- anonymous display content (no key)
    pub fn text(&mut self, tw: Tw, text: impl Into<String>);
    pub fn glyph(&mut self, tw: Tw, icon: Icon);
    pub fn image(&mut self, tw: Tw, src: egui::ImageSource<'static>);
    pub fn rule(&mut self, tw: Tw);   // sugar: an empty, anonymous box. Orientation and color come from `tw`
    pub fn paint(&mut self, tw: Tw, f: impl Fn(&egui::Painter, Rect) + 'static);  // runs at draw time: owns its data

    // ---- roots (children scoped under key)
    pub fn scroll(&mut self, key: K, tw: Tw, axis: Axis, body: impl FnOnce(&mut Builder<'_, 'f, A>));
    pub fn region(&mut self, key: K, tw: Tw, body: impl FnOnce(&mut Builder<'_, 'f, A>, Vec2) + 'f);
    pub fn virtual_list<R: Key + Copy>(&mut self, key: K, tw: Tw, rows: &'f [R]) -> ListChain<'_, 'f, A, R>;
                   // .estimate_px(h) | .fixed_px(h), .overscan_px(d), then .items(|b, row| …): each row scoped under its own key
    pub fn overlay(&mut self, key: K, band: Band, anchor: Anchor, place: Placement,
                   on_dismiss: impl FnOnce(&mut A) + 'static, body: impl FnOnce(&mut Builder<'_, 'f, A>));
    pub fn max_content_width(&mut self, f: impl FnOnce(&mut Builder<'_, 'f, A>)) -> Width;
}

/// Library helpers
pub fn fill(_: &Measure) -> Size<f32>;            // leaves sized entirely by classes or by their parent

/// What div_id, span and icon return; every method may be called any number of times, handlers run in call order
impl NodeHandle<'_, '_, A> {
    pub fn draw(self, f: impl FnOnce(&mut A, &mut Ui) + 'static) -> Self;           // the node's own Ui at draw time
    pub fn on_click(self, f: impl FnOnce(&mut A) + 'static) -> Self;
    pub fn on_response(self, f: impl FnOnce(&mut A, &Response) + 'static) -> Self;
}
```

**Measure and draw closures differ in lifetime** *(compiled)*:
- A **measure** is called during layout, while the application is still only borrowed (`&A`). It may borrow application data (`+ 'f`) and is `Fn` because taffy may call it several times.
- A **draw** is called once, at draw time, when the application is borrowed mutably. It must own what it captures (`'static`: keys, IDs, copied values) and receives `&mut A` as an argument. It reaches its data by narrowing from `A` with the keys it captured:

```rust
let k = blk.key;
b.custom("title", kit::small_field(th)).measure(prim::text_field::measure)
    .ui(move |app, ui| if let Some(blk) = app.doc.block_mut(k) { prim::text_field::show(ui, &mut blk.title); });
```

`scope` pushes `key` onto the path: `path = path.with(key)`. `div` creates an empty child list, runs `body` with `inherited = tw.text.over(self.inherited)`, then creates the group from the collected children (no path segment). `overlay` reserves its slot first (D19), builds `body` as a new root with the text style in effect at the declaration point, and records `on_dismiss`. `scroll` creates a leaf (the viewport) and builds `body` as a separate scroll root (§11).

Common measures:

```rust
b.custom("title", kit::small_field(k)).measure(prim::text_field::measure)...;           // a primitive's measure fn
b.custom("dir", kit::small_field(k)).measure(|m| prim::select::measure(m, &DIRS))...;   // with arguments
b.custom("note", tw()).measure(|m| text_measure(m, &self.note))...;                     // borrows app data ('f)
b.custom("canvas", tw().absolute().inset(0.0).sense_click_drag())...;                   // size from layout only
```

### 5.4 Example *(compiled shape)*

```rust
pub type B<'b, 'f> = Builder<'b, 'f, App>;      // application alias

impl App {
    fn build<'f>(&'f self, b: &mut B<'_, 'f>) {
        let k = &self.theme;
        b.div(tw().flex().flex_col().size_full().bg(k[Ground]).role(k[Body]).text(k[Ink]), |b| {
            topbar::build(self, k, b);
            b.div(tw().flex().flex_1().min_h_0(), |b| {
                activity::build(self, k, b);
                viewport::build(self, k, b);
                if self.props_open { props::build(self, k, b); }
            });
            statusbar::build(self, k, b);
        });
    }
}
```

Library components are generic over `A` and reach their data through a lens (§6).

**Lifetimes in application code** *(compiled)*. Most build functions write `b: &mut B<'_, '_>` and never name `'f`. A function must name it (`fn build<'f>(app: &'f App, k: &'f Theme, b: &mut B<'_, 'f>)`) when it declares a region, or a leaf whose measure borrows its arguments, and so must every caller on the path up to the root, because those closures outlive the call. In the reference shell that is `App::build` and `viewport::build` only. Draw closures never need `'f`: they own their captures.

## 6. Components (convention)

A component is a function (or a module of functions) that builds a subtree into the scope it is given: `fn header(s: &mut Scope<'_, '_, A>, …)`. The caller supplies the key (`b.scope(Key, |s| header(s, …))`, or the key of the `div_id` the component fills), which a plain `&mut Builder` cannot do (D30; a trybuild test pins it). There is no component type, no ID enum and no draw dispatch: each identified node carries its own draw.

```rust
pub mod topbar {
    pub fn build(app: &App, k: &Theme, b: &mut B<'_, '_>) {
        b.scope("top", |b| b.div(bar(k), |b| {
            b.icon("undo", kit::tap(k), icons::undo()).draw(click(|app| app.undo()));
            b.icon("props", props_tap(app, k), icons::properties()).draw(click(|app| app.props_open = !app.props_open));
            // …
        }));
    }
}
```

Conventions:

- **Each module opens its own scope** (`"top"`, `"props"`, a section value), so its keys only need to be unique within it.
- **Draw closures narrow from the application with captured keys.** Helpers make the common shapes short. These are application code, not library API:

  ```rust
  /// run on a click of this node (D24: the library did the hit test)
  fn click(f: impl FnOnce(&mut App) + 'static) -> impl FnOnce(&mut App, &mut Ui) + 'static {
      move |app, ui| if ui.node_response().clicked() { f(app) }
  }
  /// a draw that edits block `k`, if it still exists
  fn on_block(k: BlockKey, f: impl FnOnce(&mut Block, &mut Ui) + 'static) -> impl FnOnce(&mut App, &mut Ui) + 'static {
      move |app, ui| if let Some(blk) = app.doc.block_mut(k) { f(blk, ui) }
  }
  ```

  Put accessors like `block_mut` on the data's own type (`Document`), not on `App`, so a draw can borrow the block and another `App` field (e.g. `popup`) in turn.
- **Reusable components take a lens.** A component that edits data it does not own receives the data for build and a lens for draw. The reference shell's tags-and-properties editor (`props::meta`) is used by the block form and the document form:

  ```rust
  pub fn build(meta: &Meta, k: &Theme, b: &mut B<'_, '_>, lens: impl Fn(&mut App) -> Option<&mut Meta> + Copy + 'static) {
      for (t, label) in &meta.tags {
          let t = *t;
          kit::tag(b, k, label, ("tag", t), move |app| if let Some(m) = lens(app) { m.tags.retain(|(x, _)| *x != t) });
      }
      // …
  }
  // block form
  b.scope("meta", |b| meta::build(&blk.meta, k, b, move |app: &mut App| app.doc.block_mut(key).map(|blk| &mut blk.meta)));
  // document form
  b.scope("meta", |b| meta::build(&doc.meta, k, b, |app: &mut App| Some(&mut app.doc.meta)));
  ```

  Library components (generic over `A`) use the same pattern.
- **Local state is keyed by an ID computed once.** When build reads state that a draw writes, compute the ID in build and capture it in the draw:

  ```rust
  let open_id = b.id_of(("row", key)).with("open");
  let open = b.ctx().data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
  b.icon("fold", …, chevron(open)).draw(move |_app, ui| if ui.node_response().clicked() {
      ui.data_mut(|d| { let o = d.get_temp_mut_or_default::<bool>(open_id); *o = !*o; });
  });
  ```

State rules (D11):

| State | Examples | Lives in |
|---|---|---|
| Domain | parts, pins, selection | `Document` on `App` (undoable through `Undoer`) |
| Shared UI | active section, open popups, panel toggles, `enabled` flags | fields on `App` |
| Local UI | a tree row's expanded flag, text cursor, egui popup state, drag in progress | egui memory under an ID derived from a key path (D11) |

State whose meaning spans several nodes (an open popup closed from its rows, the selection) goes on `App`.

## 7. Style vocabulary (`Tw`)

### 7.1 What `Tw` holds

| Class kind | Examples | Writes to | Consumed by |
|---|---|---|---|
| Layout | `flex`, `p(3.0)`, `w(86.0)`, `absolute`, `inset(0.0)`, `grid_cols_tracks` | `taffy::Style` | taffy |
| Box paint | `bg(c)`, `border_color(c)`, `rounded_2xl`, `ring(w, c)`, `shadow(s)`, `opacity(x)` | `BoxPaint` | library draw |
| Text (inherited) | `role(spec)`, `text(c)`, `italic`, `truncate`, `text_center`, `tracking(x)`, `uppercase` | `TextStyle` | measure (size, family) and draw (color) |
| Stacking and behaviour | `z(n)`, `overflow_hidden`, `inert` | node flags | library draw |

All colors are `impl Into<Color32>`. `role` takes a `TextSpec { size, line_height, family: egui::FontFamily }`. Font weight is expressed by the family, because egui does not synthesize weights. The application resolves weights to registered families.

```rust
#[derive(Clone)]
pub struct Tw { style: taffy::Style, paint: BoxPaint, text: TextStyle /* Option fields */, z: i16, flags: NodeFlags }
pub fn tw() -> Tw { /* Display::Block (D15) */ }
```

`Tw` is a newtype, not an extension trait on `taffy::Style`, so its methods never clash with taffy's style traits (`gap()`, `size()`, `inset()`). Use taffy's generic `style_helpers` (`length`, `percent`, `auto`) internally ⚠.

### 7.2 Translation rules

| Tailwind | Rust | Rule |
|---|---|---|
| `flex-col` | `.flex_col()` | `-` → `_` |
| `p-4`, `p-1.5` | `.p(4.0)`, `.p(1.5)` | scale number as `f32` argument, × 4 pt |
| `-mt-2` | `.mt(-2.0)` | negative arguments |
| `w-1/2` | `.w_frac(1.0, 2.0)` | fractions → percent |
| `w-px`, `w-[37px]` | `.w_px(1.0)`, `.w_px(37.0)` | `_px(n)` takes points directly |
| `mx-auto` | `.mx_auto()` | `auto` keyword |
| `border`, `border-2` | `.border(1.0)`, `.border(2.0)` | border widths in points (as in Tailwind) |
| `grid-cols-[3rem_minmax(0,1fr)_…]` | `.grid_cols_tracks(&[…])` | arbitrary track list |
| `bg-bar` | `.bg(k[Bar])` | colors are resolved values (Appendix A) |
| `text-sm`, `text-meta` | `.role(k[Meta])` | text roles are resolved `TextSpec`s |
| `static` | `.static_()` | Rust keyword gets `_` |
| `z-30` | `.z(30.0)` | sibling paint and hit order |

### 7.3 Method set (v1)

- Display: `flex block grid hidden`
- Flex: `flex_row flex_col flex_row_reverse flex_col_reverse flex_wrap flex_nowrap flex_1 flex_auto flex_initial flex_none grow grow_0 shrink shrink_0 basis(n) basis_px basis_frac basis_full basis_auto`
- Gap: `gap gap_x gap_y`
- Padding / margin: `p px py pt pr pb pl`, `m mx my mt mr mb ml`, `*_auto`
- Size: `w h size min_w min_h max_w max_h`, each with `_px`, `_frac`, `_full`, plus `w_auto h_auto w_fit min_w_0 min_h_0`
- Aspect: `aspect_square aspect_video aspect(r)`
- Align: `items_* justify_* self_* content_* justify_items_* justify_self_*`
- Position: `relative absolute static_ inset inset_x inset_y top right bottom left` (with `_px`, `_frac`, `_auto`), `z(n)`
- Overflow: `overflow_visible overflow_hidden overflow_clip` (+ `_x` / `_y`). Scrolling is `b.scroll`, not a class
- Border width (layout): `border border_t border_r border_b border_l border_x border_y`
- Box sizing: `box_border box_content`
- Grid: `grid_cols(n)` (= `repeat(n, minmax(0,1fr))`), `grid_cols_tracks(&[Track])`, `grid_rows(n)`, `col_span col_span_full col_start col_end row_span row_start row_end grid_flow_row grid_flow_col grid_flow_dense` ⚠ (taffy's grid template types changed across versions)
- Box paint: `bg(c) border_color(c) border_t_color(c)… divide_x(w, c) divide_y(w, c) rounded_none rounded_sm rounded rounded_md rounded_lg rounded_xl rounded_2xl rounded_3xl rounded_full rounded_px(r) ring(w, c) shadow(Shadow) opacity(x)`
- Text: `role(TextSpec) text(c) font(family) italic underline truncate whitespace_nowrap text_left text_center text_right tracking(x) uppercase capitalize`
- Behaviour: `inert` (subtree non-interactive), `pointer_events_none` (the subtree's boxes register no hit-test widget, so the pointer reaches what lies beneath, D24a), `sense_*`, `group`
- Variants (paint only): `hover active group_hover transition` (§7.5)

Not expressible on a single node, so do not implement: `space-x-*`, `space-y-*` (use `gap`), `container` (use regions, §11.2). Breakpoint variants (`md:`) are branches in build on the window width. State variants are in §7.5.

### 7.4 Inheritance

Text classes (`role`, `text`, `font`, `italic`, `underline`, `tracking`, `uppercase`, `capitalize`, `text_*` alignment, `truncate`, `whitespace_nowrap`) are inherited: `div` merges its text classes over the inherited style for its children. A node's own classes override field by field *(compiled)*. The root's inherited style is `LayoutState::with_base_text` if set, else `TextStyle::from_egui(ctx)` (egui's `Body` font, text colour and row height, recomputed per build so theme changes apply). Overlays inherit from their declaration point.

The library also applies the node's resolved text style to the leaf's `Ui` (font override and text color ⚠), so egui widgets drawn by the application pick it up.

### 7.5 Variants and interaction (D23, D24)

```rust
let tap = tw().grid().items_center().justify_center().size(11.0).rounded_px(12.0).text(k[Ctl])
    .hover(|p| p.bg(k[Hover]))
    .transition(0.15);

b.div_id(("row", key), tw().flex().h(7.0).items_center().gap(1.0).rounded_md()
        .group().sense_click().hover(|p| p.bg(k[HoverSoft])), |b| {
    // …
    b.icon("open", tw().size(6.0).opacity(0.0).group_hover(|p| p.opacity(1.0)).sense_click(), icons::enter()).draw(click(|app| { /* … */ }));
}).draw(click(move |app| app.doc.selection = Some(key)));
```

**Sense.** `sense_click()`, `sense_drag()`, `sense_click_drag()`, `sense_hover()`. A node with `hover` / `active` implies at least the matching sense. Only identified nodes (`div_id`, `leaf`, `span`, `icon`) may declare a sense, a group, `hover` / `active` variants or a `transition`; on an anonymous node that is a build error (debug assertion). `group_hover` alone is the exception: it only reads the group's state, so an anonymous child may follow its parent's hover (a `transition` on it needs an identified node).

**Variants (v1).**
- `hover(|p| …)`: pointer over the node's border box with no sensing node above it in its layer, or over a sensing descendant (CSS `:hover`, so a row stays highlighted over its buttons). Calls accumulate: the closure receives the paint so far.
- `active(|p| …)`: primary button held on the node.
- `group_hover(|p| …)`: hover of the nearest ancestor marked `.group()`. That ancestor must be identified with a sense. It is drawn before its descendants, so its state is known when they are drawn.
- `transition(Duration)`: animate between base and variant paint with `ctx.animate_value_with_time(node_id.with(variant), …)` ⚠. Applies to colors and opacity.
- `animate_layout(Duration)`: when layout moves an identified node, paint it from its previous position to the new one (FLIP), along the node's `ease_*` curve. Layout and hit rects are final at once; only the paint-only translate moves. Positions are compared within the parent's rect, so a moving parent carries its children and they animate only their own moves. First appearances and every node of a frame whose area changed (window resize) are placed at once, a deliberate difference from CSS. A move interrupted mid-flight starts from the painted position. Position only: size is the collapsible (§11.5), exit animations #35. While a size animation is in flight in a frame, layout moves nodes smoothly itself, so no move starts a slide in it (`StartsSlides`, one predicate).
- Deferred: `focus_within`, `focus_visible`.

States that the application knows at build time (selected, pressed, armed, current, disabled) are conditional classes, not variants. `.tap.dead:hover{background:none}` becomes "do not add `.hover` when dead", plus `inert()`.

**`Paint` builder.** The closure argument has only `bg`, `text`, `border_color`, `ring`, `shadow`, `opacity`, `underline`. The compiler rejects layout and font-size changes.

**Interaction sequence per identified node with a sense:**
1. `let r = ui.interact(border_box, node_id, sense)`. `node_id` is the hash of the node's key path (D5).
2. Resolve active variants from `r` (and the group stack), blend with transitions, paint box and primitive.
3. Store `r` for the node, then call the node's draw closure.
4. The draw reads it through an extension trait: `ui.node_response() -> Response` (clicks, drags, context-click, pointer position).

```rust
pub trait NodeUi { fn node_response(&self) -> egui::Response; fn node_id(&self) -> egui::Id; }   // library-provided; the draw `Ui`'s scope id is derived from the node id, which egui keeps for the node's border-box widget
```

The application may still create its own sub-interactions inside a node using derived IDs (`ui.make_persistent_id("handle")`), e.g. the canvas's own hit testing. Leaves that contain egui widgets with their own IDs (`TextEdit`, `ComboBox`, `Checkbox`) need no node sense: those widgets interact and style themselves.

## 8. Box model, draw order and clipping

- **Margin** positions the box and is not painted. **Border** and **padding** are inset from the border box. taffy reports border and padding per node, so the library derives the border box, padding box and content box.
- **Measure sees content only.** For leaves, taffy subtracts padding and border from available space before calling measure and adds them back afterwards.
- **Primitives and draw closures receive the content box.** Box paint (background, border, ring, shadow) uses the border box. Rings and shadows are outside the box and do not affect layout.
- **Draw order is pre-order**: a node's hit-test widget (its own if it senses, else its box's if it paints one, D24a), its box paint with active variants, then its primitive, then its draw closure (if it has one), then its children. Children of one parent are stable-sorted by `z` before drawing. `z` affects paint and hit order only, never layout.
- **Clipping**: `overflow_hidden` / `overflow_clip` intersect the clip rect with the node's padding box for its whole subtree. Each leaf's `Ui` uses the current clip.
- **Absolute positioning** (`absolute` + insets) is relative to the parent's box, as in taffy/CSS.
- **Opacity** multiplies down the subtree ⚠ (`Ui::multiply_opacity`). **Inert** disables the subtree's `Ui`s (`UiBuilder::disabled` ⚠).
- One-sided borders are painted as line segments. egui's rect stroke is uniform on all sides.

## 9. Measurement

### 9.1 taffy queries

The measure closure may be called several times per leaf:

| Input | Meaning | Return |
|---|---|---|
| `known.width = Some(w)` | width fixed: height-for-width | size at width `w` |
| `avail.width = Definite(w)` | final pass (`known` cleared) | size at width `w` |
| `avail.width = MaxContent` | preferred width | e.g. single-line text |
| `avail.width = MinContent` | narrowest acceptable | e.g. widest single word |
| both `known` axes `Some` | fully determined | return `known` unchanged |

taffy 0.15 closure shape: `|known_dimensions, available_space, node_id, node_context, style|` ⚠. The library dispatches: a node with `prim` → library measure, a `leaf` → its own measure closure (D28), anything else → `Size::ZERO`.

```rust
pub struct Measure {
    pub ctx: egui::Context,        // owned (cheap clone)
    pub known: Size<Option<f32>>,
    pub avail: Size<AvailableSpace>,
    pub text: TextStyle,           // the leaf's resolved text style
}
impl Measure {
    pub fn text_size(&self, s: &str) -> Size<f32>;   // per the table above, via egui galleys (cached)
    pub fn width(&self) -> Option<f32>;              // known or definite width, if any
}
```

Text measurement: use egui galley layout ⚠. For MinContent, measure the widest whitespace-separated word unwrapped (egui breaks inside words, so wrap width 0 is wrong). Labels that toggle between strings measure the widest variant.

### 9.2 Primitive sizing kinds

| Kind | Examples | Width | Height |
|---|---|---|---|
| Content | text, `Button`, labelled `Checkbox` | text + padding | text + padding |
| Fixed | `Icon`, bare `Checkbox`, `Separator` | given size | given size |
| Flexible | `Slider`, `TextField`, `NumberField`, `ProgressBar`, `Select` | preferred/minimum constant, otherwise what layout gives | control height constant |

Whether a widget stretches is a layout decision (`flex_1`, `w_full`), never the widget's.

## 10. Primitives

### 10.1 Display (library-drawn)

`text`, `glyph` / `icon`, `image`, `paint`, and `span` (text with an ID). Rules (separators) are not primitives: `b.rule(tw)` is an empty box whose `bg` is the line color, horizontal with `h_px(1.0).w_full()` (or `self_stretch` in a column), vertical with `w_px(1.0).self_stretch()` or a fixed `h`. The library measures them, paints them in the content box using the node's resolved text style, and, for identified ones (`span`, `icon`), then calls the node's draw closure, which reads `ui.node_response()` for clicks, tooltips and cursor (§7.5).

`Icon` is a library value: `Icon { src: ImageSource<'static>, rotate: f32, mirror: Mirror }`, tinted with the inherited text color. Applications define their icon set as functions returning `ImageSource` ⚠.

### 10.2 Interactive (drawn by the node's closure, library-provided helpers)

Each interactive primitive is a pair of functions sharing one sizing helper (D21):

```rust
pub mod prim::slider {
    pub fn measure(m: &Measure, opts: &Opts) -> Size<f32>;      // used as a leaf's measure: |m| slider::measure(m, &opts)
    pub fn show(ui: &mut egui::Ui, value: &mut f32, opts: &Opts) -> egui::Response;
}
```

v1 set: `button`, `checkbox`, `switch`, `slider`, `text_field`, `number_field`, `stepper`, `numeric`, `progress`, `select` (wraps egui's `ComboBox`, which keeps its popup state inside the leaf), `segmented`, `splitter`.

- **`numeric`** displays a live value without moving layout: `measure` uses a template (`"+#,###.# rpm"`, `#` = widest digit), draw uses the current value, right-aligned in the reserved width. Formatting, units and out-of-range handling are the application's.
- **`splitter`** is a drag-sensing leaf with a wide hit area and a 1 pt painted line. Its position is application state. See the companion `splitter-test` project.

Rules:
- **Fill the given rect.** egui's self-sizing widgets are told the rect's size inside the leaf's `Ui`: `ui.spacing_mut().slider_width = …`, `TextEdit::desired_width(…)`, `ProgressBar::desired_width(…)`. The changes are local to the child `Ui`.
- **Explicit IDs where egui accepts them.** `TextEdit::id(ui.make_persistent_id("text"))` keeps focus and cursor through reorders. Widgets without an ID setter rely on the leaf's scope: one such widget per leaf, or a fixed order within it.
- **Never use a bare `Id::new("…")`** inside a primitive. Always derive from the leaf's `Ui`.

## 11. Scroll areas

```rust
b.scroll("scroll", tw().grow().min_h_0()).axis(Axis::Y).children(|s| { /* content, scoped under "scroll" */ });
```

- The scroll node is a leaf in the current tree, sized by layout like any other leaf. In v1 it does not size to its content: give it `grow`, an explicit height or `max_h`. Its padding is its content's (it scrolls); its border and background stay put.
- The content is a separate root of the same taffy tree, solved right after the tree around it, in the same frame: fixed to the viewport's width and as tall as it needs (`Axis::Y`), the reverse for `Axis::X`, unconstrained for `Axis::Both`. Content shorter than the viewport is solved again at the viewport's height, so that it can fill it (`h_full`, `grow`).
- The content is laid out at the offset egui holds (`ScrollArea`'s state, clamped to this frame's content size), so its rects are screen rects. Scrolling moves the content root; a node's position within its parent does not change, and `animate_layout` compares those, so scrolling alone starts no slide.
- Draw: an `egui::ScrollArea` keyed by the scroll node's id owns the offset, the wheel, the bars and the momentum. The content's nodes are walked inside it with those outside the viewport left out (their draw closures do not run, and Tab does not reach them). The node that holds the keyboard focus or the lifted drag, and the drag's image, are never left out.
- Programmatic scrolling (`scroll_by`, `scroll_to_rect`, `scroll_into_view`) is a request that the next layout applies to the rects of its own frame; `viewport` reads where the viewport and the content are.
- While a drag is live, a container near whose edge the pointer is scrolls by `EdgeScroll` and requests frames; the innermost container that can move takes it.
- Scroll bars are never overlays. The container always reserves a gutter (`ScrollStyle::width`) on the axis it scrolls, so layout is single-pass and nothing depends on overflow; egui draws its bar in the gutter only when the content is larger than the viewport. The look is `Style::scroll` (the shadcn preset gives `ScrollArea`'s), per container `scrollbar_style`.
- Text inheritance continues from the scroll node into its content.
- Scroll roots may nest (a scrolling table inside a pane inside a scrolling grid). Containers are solved outermost first; an inner `ScrollArea` is drawn inside its outer one, so egui chains the wheel to the outer one at the inner one's edge.
- Long lists use `virtual_list` (§11.1), which builds only the visible range.

### 11.1 Virtual lists

For long lists whose rows differ in height (History: date headers, revision rows, the current-revision card; Assets: rows whose tags wrap).

```rust
b.virtual_list(key, tw, &rows)            // rows: &'f [R], R: Key + Copy; each row is its own key
    .estimate_px(44.0)                    // rows differ in height: the height of a row not yet measured
    .overscan_px(600.0)                   // optional: rows this far past the viewport are built too
    .items(|b, row| …);                   // builds one row, under the row's key; FnMut + 'f

b.virtual_list(key, tw, &lines).fixed_px(20.0).items(|b, line| …);   // all rows one height
```

`estimate_px` and `fixed_px` set one setting; the last call wins. With neither, a row counts as one line of the list's text until it is measured. Lengths are bare points, as `Tw`'s `_px` classes are.

```rust
#[derive(Clone, Copy, Hash, Debug)] enum HistRow { Day(&'static str), Current(RevKey), Rev(RevKey) }

fn history<'f>(b: &mut Builder<'_, 'f, App>, rows: &'f [HistRow]) {
    b.virtual_list("list", tw().flex_1().min_h_0(), rows)
        .estimate_px(44.0)
        .items(|b, row| match row {
            HistRow::Day(d)       => b.text(tw().pt(3.0).px(3.0).pb(1.0).role(k[Meta]), d),
            HistRow::Current(key) => current_card(b, k, rev(key)),     // keys inside a row are unique within the row
            HistRow::Rev(key)     => rev_row(b, k, rev(key)),
        });
}
```

Mechanism (library-owned, per list):

1. **Height cache.** Each row's height as last measured, keyed by the row's key, held in `LayoutState` (D11a). Rows never laid out use the estimate. The cache is cleared when the list's content width changes, because heights depend on width. Entries for keys absent this frame are dropped.
2. **Offsets.** Each frame, compute running totals over `rows` from cached or estimated heights. One linear pass. Total = content height.
3. **Visible range.** Chosen once the list's container is laid out in the frame, as a region's body runs (§11.2): the viewport and the offset are the frame's own. Binary-search the running totals for `[offset − overscan, offset + viewport + overscan]`. Call `item` only for that range. The built rows are stacked in one column, absolutely positioned at the first one's offset inside a content node of the total height.
4. **Correction.** After layout, store the real heights of built rows. If rows above the first visible row changed height, adjust the scroll offset by the difference so visible content does not move (scroll anchoring), and request a repaint.
5. **Pinned rows.** A row that holds keyboard focus or owns an open popup is always built, even outside the range, because egui drops focus from widgets that are not drawn in a frame.
6. **Scroll to a row.** `vlist::scroll_to(ctx, list_id, key, align)` uses the running-total offset (possibly an estimate), then corrects once after measuring.
7. **Uniform rows.** `fixed_px(h)` skips the cache: the offsets are `i × h` and the range is arithmetic.

Rows are ordinary built content scoped under their row key (D4, D5), so egui state for a row survives scrolling away and back.

#### As built (M8)

- **Structure.** A list is a scroll container (§11) whose content is one leaf, `<rows>`, as wide as the content and as tall as the running total. The rows are that leaf's region content (`RegionBody::List`): run in the region pass, so they are built at the frame's viewport and offset, with no frame of lag on the first show or a resize. `rows` and `item` borrow for `'f`, so a function that declares a list names `'f`, as for a region (§11.4).
- **Placement.** The rows in the range sit in flow in one flex column, placed at the first one's running offset; a wrong estimate never makes built rows overlap or leave gaps, and only the space above and the total depend on estimates. A pinned row outside the range sits alone at its own running offset, before the column if it is above, after it if below, so the Tab order follows the list.
- **Anchoring, two ways.** After the rows are solved, the difference between the measured and the used heights of the built rows above the first visible row is applied to the offset in the same frame (`scroll_by`, applied by that layout's settle step), so a row measured anew never moves what is seen. The first visible row's laid-out top is kept; the next frame compares it with the row's new running offset and moves the offset by the difference before it chooses the range, so rows inserted or removed above, and pinned rows measured anew, do not move it either. Neither applies at offset 0: at the very top, rows inserted above show, as a browser's scroll anchoring does not hold the top. A list scrolled to its end holds its end instead: as the rows there are measured and the total changes, the offset follows the end, so one scroll past the end lands on the last row, asking for frames until the heights settle. A `scroll_by` asked of the list is counted when the range is chosen, so the rows at the new offset are built in the frame that applies it (the keyboard's scrolling is `scroll_by`).
- **The cache's key** is the row's own hash (`IdSalt`), not its egui id: within one list the rows are one type, and the cheaper hash keeps the per-frame pass over every row short. The row's egui id (`list_id.with(row_key)`) is what its nodes are keyed under.
- **Pinned rows.** A row built in the last frame is built again while `focus_within` its node, while an overlay declared inside it is open, or while an overlay declared this frame is anchored to a node that was inside it (without that, the anchor would be missing).
- **egui's rubber band.** egui bounds a `ScrollArea`'s offset by the last frame's content and springs an offset past it back, carrying the spring's velocity into the next frames. A container whose offset the layout sets in a frame in which its content grew (a list whose rows were just measured) is shown without the rubber band for that frame, so the offset lands where the layout put it.
- **Tab** reaches the built rows only. The container's Tab-stop rule is settled after the rows are built, so a list of controls is no stop of its own.
- **`scroll_to`** is kept in the context under the list's id until the list next runs, which builds the target where its running offset puts it and aligns it through `scroll_to_rect`; the anchoring corrects it once the row is measured.
- **Fixed rows** are given exactly their height. `ScrollArea::show_rows` is not used: the list draws through the eggshell scroll container, which taffy lays out, and the range is the same formula's.
- **Cost.** A fixed list's frame is O(visible rows). A measured list also makes one pass over every row's key each frame, hashing it and looking up its height; `benches/frame.rs` measures both at 1,000 to 100,000 rows.

### 11.2 Regions (D25)

```rust
b.region("pane_host", tw().flex_1().min_w_0().min_h_0(), move |b, size| {
    let cols = self.columns.min(((size.x / 300.0).floor() as usize).max(1));
    b.scroll("grid", tw().size_full(), Axis::Y, |b| {
        b.div(tw().grid().grid_cols(cols as u16).gap(2.0).p(2.0), |b| { /* panes */ });
    });
});
```

- The region is a leaf in the current tree. Its size comes only from layout (`flex_1`, explicit sizes), never from its contents, the same restriction CSS places on container queries.
- Its closure is stored in `Built` (frame lifetime `'f`, so it may borrow application state; the declaring function names `'f`, §5.4) and runs after the enclosing root is laid out, with the leaf's content-box size. The subtree it builds is a separate root laid out at exactly that rect.
- Regions nest. They run outermost first, and scroll roots and overlays inside a region follow the same ordering.
- Text inheritance continues into the region.
- Anchors inside a region may refer to items outside it (laid out earlier), not the reverse.

### 11.3 Choosing alternatives that fit (D26)

Inside a region (where the available width is known), the application can measure candidates before choosing:

```rust
b.region("title_bar", tw().flex_1().min_w_0().h(7.0), move |b, size| {
    let full = |b: &mut B<'_, '_>| title_full(b, pane);
    let compact = |b: &mut B<'_, '_>| title_compact(b, pane);
    if b.max_content_width(full) <= Width::new(size.x) { full(b) } else { compact(b) }   // else an icons-only form, …
});
```

- `max_content_width` builds into a scratch tree (same text inheritance, paths not registered, no duplicate check, closures discarded) and returns the root's max-content width.
- Cost is one extra measure pass per candidate tried. Text measurement goes through egui's galley cache.

### 11.4 As built (M7)

- **Where the content lives.** A region is an identified leaf (`NodeKind::Region`) with no measure, so its size is its classes' alone. Its content is a `NodeKind::RegionContent` root that hangs off the leaf's context, as a scroll container's content does, and is collected as the leaf's child at the leaf's content box. Paths read `/key/<regioncontent>/…`; the children are keyed under the region's key, so `placed.at("host").at("child")` finds them.
- **The content root** is `Display::Block` at exactly the content box, as the base root is: a child that wants the whole region says `size_full()`, and `flex_1` has no effect directly under it.
- **Order.** After a root (the base, or an overlay) is solved, its scroll containers' content is solved, an enclosing one first, and then its regions in declaration order. Each region closure runs, its content is solved, and the scroll containers and regions it declared are handled the same way before the next sibling. An overlay a region declares is laid out after the overlays the build declared, so a stack's opening order follows the order of layout. A region in an overlay is run when the overlay is.
- **State.** A closure builds into the frame's one `BuildState` and `Frame`, so ids, the duplicate check, the cross-layout check, measures, draws, drag declarations and dismissal handlers are the same as for the rest of the build. The closure builds under the region's readable path, and inherits the text style and the inert state at the leaf.
- **`max_content_width`** builds into a fresh `BuildState` and `Frame` (its own tree and its own `Handle`, `Pass::Scratch`), solves the root at max-content in the way an overlay's root is solved, and drops all of it. Nothing it declares reaches the frame; nothing it reads of `kept` (open menus, presences, the last area) is the frame's. Its keys are never registered, and the cross-layout claim is skipped.
- **Lifetimes.** A function that declares a region names `'f` (`fn view<'f>(app: &'f App, b: &mut Builder<'_, 'f, App>)`), and so do its callers up to the root. `tests/ui/region_needs_a_named_lifetime.rs` keeps the error. A function pointer type of the form `for<'b, 'f> fn(&mut Builder<'b, 'f, S>, &S)`, as the examples' `AppLayout`, cannot name it: take `&'f S`.

### 11.5 Collapsibles (#62)

`b.collapsible(key, Shown)` builds a wrapper (`overflow-hidden`, a flex box along the axis) around a content block (`shrink-0`, so it keeps its natural size). `.axis(CollapseAxis::Height | Width)`, `.transition`/`.ease_*` (200 ms ease-out by default, shadcn's accordion), `.classes` for the wrapper, `.content` for the block, `.on_collapsed(f)` once it has finished closing.

- **Presence.** The progress is `presence::Presences` stepping the node's id at build, with `First::Settled`: a region first seen open is open, one that has rested closed opens with motion. A toggle in the middle reverses from where it is, lasting in proportion to the distance left. A closed region is built as an empty wrapper of no size and its content is not built.
- **Two-pass solve.** `solve` lays the root out with every wrapper in motion `auto`, reads each wrapper's natural size, sets it to natural × progress and lays out again; it does this for every root it solves (the page, overlays, scroll contents, regions). Cost: one more solve in frames where a wrapper moves. Nested wrappers in motion are set together, so an outer one's natural size counts an inner one at its natural size; both are exact when they settle.
- **Scratch passes** (`max_content_width`) have no kept state: a collapsible there is at what it is shown as, so a measured row is its target size.
- **Clipping** is the wrapper's rectangle (no rounded clipping, #40); the wrapper's own padding and border do not open and close, so put them on the content. The wrapper is `inert` while it closes.
- **Gaps.** A closed wrapper stays a zero-size child, so a parent's `gap` counts it: put spacing in the content's padding.
- **FLIP.** While a size animation is in flight, `LayoutAnimation::starts_slides` is `No`: layout itself moves what follows, so no move starts a slide, and slides already running go on.
- **Header.** `children_with_header(|b, progress| .., |b| ..)` builds a header just before the wrapper and gives it the eased open progress the size follows (zero closed, one open), so a chevron turning `180° × progress` cannot drift from the size, and snaps with it under reduced motion. The header is an ordinary node: a `sense_click` row with `.role(Role::Button)`, `.named(..)` and `.expanded(open)` is focusable and toggles on Enter and Space.
- **Continuity.** A wrapper records `Motion::Resizes`; `Timeline::resizes` bounds its size change per frame (`max_resize`).

## 12. Layers and overlays

### 12.1 Absolute chrome vs overlays (D17)

Floating chrome that belongs to a region (tool cluster, notices, toast, sidebar panel, selection bar) is an `absolute` child of that region in the base tree, ordered with `z`, clipped by the region. Overlays are for popups: menus, pickers, dropdowns built by the application, tooltips that must escape clipping.

### 12.2 Types

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band { Floating, Popup, Tooltip }      // → Order::Middle / Foreground / Tooltip

pub enum Anchor {
    Node(egui::Id), // an identified node: `b.id_of(key)` in the node's scope
    Point(Pos2),    // e.g. context menu at the click position (stored on App)
    Rect(Rect),     // e.g. a canvas-space rect computed by the canvas during build
    Pointer,        // follows the cursor
    Free(Pos2),
}

pub enum Placement { Below, Above, RightOf, LeftOf, At }  // flipped and clamped to the screen
```

### 12.3 Behaviour

- The overlay's root is laid out at `MaxContent` (sized to content) after the base root and earlier overlays. Then its position is resolved from the anchor's rect and the placement, flipped and clamped to the screen.
- Each overlay is drawn in an `egui::Area` with ID `overlay_id.with("overlay")` (its key path's hash), `order(band)`, `fixed_pos`, `constrain(false)`, `movable(false)`. The driver's per-frame repair step flags the overlays egui has misplaced (D18).
- Nodes inside an overlay get the same `UiBuilder::id(path)` treatment as base nodes, so their IDs are independent of the layer.
- Open/close state lives on `App` (D22). Each overlay declares `on_dismiss`, which the library runs after drawing if the user clicked outside the overlay or pressed Esc that frame.

### 12.4 Example: canvas context menu *(compiled shape)*

```rust
// App: canvas_menu: Option<Pos2>
// build (inside the viewport region)
b.custom("canvas", tw().absolute().inset(0.0).sense_click_drag()).ui(|app, ui| {
    let r = ui.node_response();
    canvas::draw(&mut app.doc, ui, &r);
    if r.secondary_clicked() { app.canvas_menu = r.interact_pointer_pos(); }
});
if let Some(p) = self.canvas_menu {
    b.overlay("canvas_menu", Band::Popup, Anchor::Point(p), Placement::At, |app| app.canvas_menu = None, |b| {
        b.div(kit::menu_box(k), |b| {
            for (cmd, icon, label, key) in ROWS {
                b.div_id(cmd, kit::menu_row(k), |b| {
                    b.glyph(tw().size_px(18.0).flex_none(), icon());
                    b.text(tw().min_w_0().flex_1(), label);
                    kit::kbd(b, k, key);
                }).draw(click(move |app| { app.run(cmd); app.canvas_menu = None }));
            }
        });
    });
}
```

A popup anchored to a node computes the anchor in that node's scope:

```rust
let caret = b.id_of("caret");
b.overlay("flyout", Band::Popup, Anchor::Node(caret), Placement::Above, |app| app.popup = None, |b| { /* … */ });
```

## 13. Frame driver

Call site *(compiled shape)*. `layout` and `app` are separate fields, so the closures borrow independently:

```rust
struct Shell { app: App, layout: LayoutState }

// eframe::App::update, inside a CentralPanel with Frame::NONE
let area = ui.max_rect();
let placed = self.layout
    .build(ui.ctx(), |b| self.app.build(b))
    .layout(area);
placed.draw(&mut self.app, ui);          // each node's draw, then overlay dismissal
```

Inside `layout`:
1. Set the base root's size to `area`. Compute with the measure dispatcher (§9.1). Then run the root's regions (§11.2), which may add nodes and closures.
2. Collect the base root in pre-order: box paint, primitive, content box, clip rect, z, ID, draw index. Record identified rects (for anchors).
3. For each scroll root: compute with the scroll leaf's content width and unlimited height. Collect relative to the content origin.
4. Stable-sort overlays by band. For each: compute at MaxContent, resolve its position from the anchor and placement, collect.
5. Drop the measure and region closures. `Placed` keeps the draw and dismiss closures, which own their captures.

Inside `draw`, for each identified node:

```rust
fn node_ui(ui: &mut egui::Ui, content: Rect, clip: Rect, id: egui::Id, text: &TextStyle, f: impl FnOnce(&mut egui::Ui)) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(content).scope_id(id.with("eggshell-draw")), |ui| {   // D12
        ui.set_clip_rect(clip);
        apply_text_style(ui, text);   // font + color overrides ⚠
        f(ui);                        // interaction (D24), then the node's draw closure with &mut A
    });
}
```

Recommended defaults:
- `TaffyTree::disable_rounding()`, then round rects to physical pixels at draw time using `pixels_per_point`. taffy's default rounds to whole points, which misaligns on HiDPI.
- After drawing, if an application leaf's used size differs from its measured size, call `ctx.request_repaint()`.
- A debug overlay (`Debug::Outlines`, set on the egui context, `src/debug.rs`): a one-pixel outline of every node's border box, in red where the node's content did not fit (a leaf's measured content size exceeds its content box, or an in-flow child extends past a non-clipping parent's padding box), and the readable path and rect of the hovered node. The examples toggle it with F12.
- Layout tests without drawing: build + layout, then assert rects by path.

## 14. Interaction model

- Draw closures mutate application state directly. No message passing.
- Changes that affect layout appear next frame. egui repaints after input.
- Order nodes so that views that change state draw before views that display it.
- The canvas (CAD viewport) is an ordinary interactive leaf (`absolute inset(0)` or `flex_1`, `sense_click_drag`). Its draw reads `ui.node_response()`, does its own sub-hit-testing with derived IDs, and paints with the painter or a wgpu callback.
- Hit testing follows layer order, then paint order within a layer (pre-order + `z`), so later siblings win.

## 15. Open questions

1. **Baseline alignment.** `items_baseline` needs leaf baselines. taffy's high-level measure appears to return only a size ⚠. Verify against 0.15. If unsupported, approximate with `items_end` plus line-height alignment.
2. **Clamped absolute placement.** Resolved by regions (§11.2): make the viewport a region, and its closure knows the viewport size when it places the selection bar, so it can clamp the position.
3. **Icon stroke widths.** The CSS varies SVG stroke width per context (1.7–2.2). Rasterized SVG bakes the stroke. Use separate assets or vector path rendering.
4. **Tabular figures.** egui does not expose OpenType features ⚠. Choose a UI font with tabular digits by default, or add a numerals face.
5. **Later details:** per-row grid track lists (`grid_rows_tracks` with `MinMax(pt, fr)`), flex grow factors (`grow_by(x)`), scroll content at least as tall as its viewport (`min-h-full` inside a scroll area, so `fr` rows fill when content is short), dashed borders.
6. **Deferred to the applications that need them:** wheel routing (a selected pane consumes the wheel before its enclosing `ScrollArea` ⚠).
7. **Drag and drop** is in: `drag_source` (with an optional `drag_handle`), `drop_target`, the `drag_accepts` / `drag_over` / `drag_lifted` variants and the live drag image (the `dnd` module and the `dnd_*` examples). Auto-scroll is a hook (`EdgeScroll`) until M5; overlays as targets wait for M6.
8. **Deferred:** `focus_within` / `focus_visible` variants, rich text runs and inline links, explicit tab order, accessibility (the AccessKit tree is flat), first-frame overlay flicker, a macro for pass-through `match` arms.

## 16. Prior art

- **Clay** (C, github.com/nicbarker/clay): closest model. Per-frame declarative tree, explicit IDs, a text-measure callback, floating elements attached to a parent ID. Layout only. Hit testing uses the previous frame's layout.
- **gpui** (Zed): per-frame elements on taffy, request-layout → prepaint → paint phases, anchored and deferred overlays, taffy rounding disabled.
- **Xilem**: view-ID paths for routing.
- **Qt**: `sizeHint` / `minimumSizeHint` / `heightForWidth` ≈ MaxContent / MinContent / known-width. Size policy ≈ flex classes.
- **Tailwind v4**: utilities generated from a project `@theme`. Here, theming stays in the application and the library takes resolved values.

---

## Appendix A. Application guidance: theming (not library API)

The library takes resolved `Color32` and `TextSpec`. A recommended application pattern:

```rust
#[derive(Clone, Copy, Debug)] pub enum Tok  { Bar, Panel, Ground, Surface, Ink, InkMid, Line, Interact, InteractText, Hover, Field, Edge, MenuBg, /* … */ }
#[derive(Clone, Copy, Debug)] pub enum Role { Title, Heading, Body, Label, Meta }

pub struct Theme { mode: Mode, past: bool, colors: [Color32; Tok::COUNT], roles: [TextSpec; Role::COUNT] }

impl Index<Tok> for Theme  { type Output = Color32;  fn index(&self, t: Tok)  -> &Color32  { &self.colors[t as usize] } }
impl Index<Role> for Theme { type Output = TextSpec; fn index(&self, r: Role) -> &TextSpec { &self.roles[r as usize] } }

// usage in build
let k = &self.theme;
tw().bg(k[Bar]).text(k[InkMid]).role(k[Meta]).p(4.0)
```

- Build the tables with an exhaustive `match (Tok, Mode)` so every token has every mode's value. Rebuild them when the mode or "viewing past" changes.
- Compute derived values (CSS `color-mix`) when building the tables, or inline (`k[Interact].gamma_multiply(0.12)`).
- `TextSpec.family` names a registered font family per weight (e.g. `"Sans-SemiBold"`).
- Keep the token source single: generate the tables from `tokens.css` (or generate both from TOML) with a build script, so the web page, the canvas and the egui chrome agree.

## Appendix B. Reference shell mapping

`shell3` (companion project) writes the entire current shell (`cad-shell-current.html`) against this API, one module per functional block (`topbar`, `activity`, `viewport/…`, `panel/…`, `props` with `props/meta` and `props/hints`, `statusbar`), with `theme` and `kit` holding the application's tokens, shared classes and draw helpers (`click`, `tooltip`). Build and draw, compiled against stand-in library types; the debug duplicate check runs on every frame. Notes:

- Chrome inside the viewport, in z order: canvas (`absolute inset(0)`), tool cluster (z 10), notices (z 20), selection bar (z 24), sidebar panel (z 30), toast (z 40). All are `absolute` children of the `div_id(Viewport)` group.
- Overlays (`Band::Popup`): canvas menu (`Anchor::Point`), accent picker and "5 more" (`Anchor::Item` on the selection-bar leaves), sheet-tools flyout (`Anchor::Item` on the caret, `Placement::Above`).
- Glass surfaces translate to translucent background + ring + shadow, without blur.
- The pin table uses `grid_cols_tracks([48, minmax(0,1fr), 68, 73.6, 48, 20])`.
- Tree rows are `div_id` groups containing guides (`paint`), fold chevron (`icon` with `rotate`), pick group, count and open button. The open button fades in on row hover with `group_hover` (§7.5).

## Appendix C. Application guidance: thumbnails in long lists (not library API)

For lists with thousands of image rows (Assets):

- Only visible rows are built, so only they request thumbnails (falls out of `virtual_list`).
- Keep an application thumbnail cache: on request, decode and downscale off the UI thread to the displayed size × 2 (e.g. 96 px for a 48 pt thumb), upload with `ctx.load_texture`, and show a placeholder until ready.
- Bound memory with a least-recently-used set of a few hundred textures. Free evicted ones.
- Give thumbnails a fixed size so they never affect row height. Only wrapping content (tags) varies, which keeps height estimates close.
