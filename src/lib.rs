#![deny(missing_docs)]
//! Eggshell: taffy lays out, egui draws and interacts.
//!
//! This crate is under construction. Each frame is planned as three phases:
//!
//! 1. **Build**: walk application state with a nested builder and emit a
//!    taffy tree whose identified nodes carry key paths, measures and draw
//!    closures.
//! 2. **Layout**: taffy solves every root to absolute rectangles.
//! 3. **Draw**: a pre-order walk paints boxes and content, then opens a child
//!    [`egui::Ui`] per identified node and runs its draw closure.
//!
//! The design brief lives in `docs/design.md`.
//!
//! # Styling: named styles and merge
//!
//! A [`Tw`] is a plain value, so a style is a function that returns one. Name
//! the small styles an interface repeats (`card`, `toolbar`, `muted`) and
//! compose them; a caller's overrides go on top with [`Tw::merge`], where the
//! later list wins property by property, as `cn()` with `tailwind-merge` does
//! in a web app: `padding` over `px` leaves the vertical padding alone.
//!
//! ```html
//! <div class="flex flex-col gap-2 p-4 rounded-lg border"></div>  <!-- card() -->
//! <div class="flex flex-col gap-2 p-4 rounded-lg border w-full px-2"></div>  <!-- card().merge(w-full px-2) -->
//! ```
//!
//! ```
//! use eggshell_ui::{Tw, tw};
//!
//! fn card() -> Tw {
//!     tw().flex().flex_col().gap(2.0).p(4.0).rounded_lg().border(1.0)
//! }
//!
//! let wide = card().merge(tw().w_full().px(2.0));
//! assert_eq!(
//!     wide.style(),
//!     tw().flex().flex_col().gap(2.0).p(4.0).rounded_lg().border(1.0).w_full().px(2.0).style()
//! );
//! ```
//!
//! A style is a `fn`, not a `const`: the class methods are not `const fn`
//! (they are generic over their arguments and write through closures), so a
//! `const CARD: Tw` cannot be built. Calling the function is cheap.
//!
//! # Grids: striped rows
//!
//! A row stripe is not painted on the cells: each cell's background stops at
//! the column gap. As in CSS, give each striped row one more item that spans
//! every column (`col-span-full`) in that row, declared before the row's
//! cells so it is painted under them. The cells then need explicit places
//! (`col-start`, `row-start`), because auto-placement would treat the
//! stripe's row as full.
//!
//! ```html
//! <div class="grid grid-cols-[auto_1fr] gap-x-4">
//!   <div class="col-span-full row-start-2 rounded-md bg-muted"></div>
//!   <span class="col-start-1 row-start-1">Port</span><span class="col-start-2 row-start-1">COM3</span>
//!   <span class="col-start-1 row-start-2">Baud</span><span class="col-start-2 row-start-2">115200</span>
//! </div>
//! ```
//!
//! ```
//! use eggshell_ui::testing::Headless;
//! use eggshell_ui::{Builder, Track, tw};
//! use egui::{Color32, vec2};
//!
//! const ROWS: [(&str, &str); 3] = [("Port", "COM3"), ("Baud", "115200"), ("Parity", "None")];
//!
//! fn striped(b: &mut Builder<'_, '_, ()>, _: &()) {
//!     let columns = [Track::auto(), Track::fr(1.0)];
//!     b.div_id("table", tw().grid().grid_cols_tracks(&columns).gap_x(4.0).w_px(240.0), |b| {
//!         for (index, (key, value)) in ROWS.into_iter().enumerate() {
//!             let row = index as i16 + 1;
//!             if index % 2 == 1 {
//!                 let stripe = tw().col_span_full().row_start(row).rounded_md();
//!                 b.div_id(("stripe", index), stripe.bg(Color32::from_gray(240)), |_| {});
//!             }
//!             b.span(("key", index), tw().col_start(1).row_start(row), key);
//!             b.span(("value", index), tw().col_start(2).row_start(row), value);
//!         }
//!     });
//! }
//!
//! let placed = Headless::new(vec2(300.0, 200.0)).layout(&(), striped);
//! let table = placed.at("table");
//! let (stripe, key, value) = (
//!     table.at(("stripe", 1_usize)).rect(),
//!     table.at(("key", 1_usize)).rect(),
//!     table.at(("value", 1_usize)).rect(),
//! );
//! assert!(value.left() > key.right(), "the row has a column gap");
//! // The stripe runs from the first cell to the last, across the gap.
//! assert_eq!((stripe.left(), stripe.right()), (key.left(), table.rect().right()));
//! assert_eq!((stripe.top(), stripe.bottom()), (key.top(), key.bottom()));
//! ```
//!
//! # Controls
//!
//! The owned controls are chained builders that start from the builder and
//! end in a call that adds them and says what the user's input changes.
//! Options come first. A control left without its terminating call is not
//! added, and `#[must_use]` makes that a warning.
//!
//! Every control that holds a value is built with the value it shows, and its
//! terminator is `on_change`, which receives the value the user chose: the
//! application stores it, and the next frame shows it. A checkbox or switch
//! over a `bool` needs nothing more; over another two-valued type it names
//! the values. Hooks come before the terminator: `on_response` on every
//! control, and `when_submitted` and `when_blurred` on a text field and a
//! stepper.
//!
//! ```
//! use eggshell_ui::Builder;
//!
//! #[derive(Hash, Debug)]
//! enum Part { Save, Terms, Volume }
//!
//! struct App { saves: u32, terms: bool, volume: f64 }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.button(Part::Save, "Save").tooltip("Save the form").on_click(|app| app.saves += 1);
//!     b.checkbox(Part::Terms, "I agree", app.terms).on_change(|app, terms| app.terms = terms);
//!     b.slider(Part::Volume, app.volume, 0.0..=1.0).on_change(|app, volume| app.volume = volume);
//! }
//! ```
//!
//! A two-valued enum works the same way:
//!
//! ```
//! use eggshell_ui::Builder;
//!
//! #[derive(Clone, Copy, PartialEq)]
//! enum Mode { Light, Dark }
//!
//! struct App { mode: Mode, name: String }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.switch("dark", "Dark mode", app.mode)
//!         .values(Mode::Light, Mode::Dark)
//!         .on_change(|app, mode| app.mode = mode);
//!     b.text_field("name", &app.name)
//!         .when_submitted(|app| println!("hello {}", app.name))
//!         .on_change(|app, name| app.name = name);
//! }
//! ```
//!
//! A radio group, a segmented control or a select takes its options as any
//! iterator of things that convert to a [`Choice`](owned::Choice) (see
//! [`IntoChoice`](owned::IntoChoice)), so the options need not be collected
//! into a `Vec` each frame. A value that is `Display`, a reference to one, a
//! pair, a triple with an icon and a `Choice` (for a tooltip or a disabled
//! option) all do:
//!
//! ```
//! use std::fmt;
//! use eggshell_ui::Builder;
//! use eggshell_ui::owned::{Choice, Enablement};
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
//! enum Unit { Metric, Imperial }
//!
//! impl fmt::Display for Unit {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write!(f, "{self:?}")
//!     }
//! }
//!
//! impl Unit {
//!     const ALL: &[Self] = &[Self::Metric, Self::Imperial];
//! }
//!
//! struct App { unit: Unit }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     // `&T` with `T: Display`, labelled by its `Display`
//!     b.radio_group("radio", app.unit, Unit::ALL)
//!         .on_change(|app, unit| app.unit = unit);
//!     // `(value, label)`
//!     b.segmented("seg", app.unit, [(Unit::Metric, "Metric"), (Unit::Imperial, "Imperial")])
//!         .on_change(|app, unit| app.unit = unit);
//!     // `Choice`: a tooltip, an icon, a disabled option
//!     b.select(
//!         "select",
//!         app.unit,
//!         [
//!             Choice::new(Unit::Metric, "Metric").tooltip("Kilometres"),
//!             Choice::new(Unit::Imperial, "Imperial").enablement(Enablement::Disabled),
//!         ],
//!     )
//!     .on_change(|app, unit| app.unit = unit);
//! }
//! ```
//!
//! # Differences from CSS
//!
//! egui paints shapes one at a time into rectangular clip rects, so three
//! things CSS does are not available:
//!
//! - **No backdrop blur.** What is behind a layer cannot be blurred.
//! - **No rounded clipping.** `overflow_hidden` clips a subtree to the node's
//!   rectangle, not to its rounded shape. A child that reaches a rounded corner
//!   must be rounded itself: the owned slider's range is `rounded_full`.
//! - **No group opacity.** `opacity` and a colour's alpha fade each shape on its
//!   own, so overlapping shapes show through one another where CSS would fade
//!   the composited subtree. Disabled looks therefore pre-blend each colour
//!   against the opaque surface it sits on
//!   ([`Colors::faded_over`](contract::Colors::faded_over)) and are never
//!   translucent.

mod actions;
mod builder;
mod continuity;
pub mod contract;
mod debug;
mod dismiss;
mod dnd;
mod draw;
mod focus;
mod kept;
mod key;
mod measure;
mod nav;
pub mod overlay;
mod overlay_chain;
mod overlay_input;
pub mod owned;
mod pipeline;
mod presence;
pub mod prim;
mod region;
mod roots;
pub mod scroll;
mod scroll_keys;
mod shadow_corners;
mod sizing;
mod stack;
pub mod testing;
mod text;
mod tooltip;
mod transition;
mod tw;
pub mod widget;

pub use builder::{Builder, NodeHandle, Scope};
pub use contract::Style;
pub use debug::Debug;
pub use dnd::{
    Cause, DragEnd, DragImage, Dragging, Drop, EdgeBand, EdgeScroll, File, Files, Over,
    ScrollSpeed, drag_grab, drag_pointer, insertion_index,
};
pub use draw::NodeUi;
pub use focus::focus_within;
pub use key::Key;
pub use measure::{Measure, fill};
pub use overlay::{Boundary, Distance, InsetBoundary};
pub use overlay_input::Blur;
pub use pipeline::{Built, LayoutState, NodeKind, Placed, PlacedNode};
pub use prim::Icon;
pub use roots::OverlayRef;
pub use sizing::Width;
pub use text::{
    FontStyle, TextAlign, TextCase, TextClasses, TextDecoration, TextSpec, TextStyle, Tracking,
    WhiteSpace,
};
#[doc(hidden)]
pub use transition::tracked_layout_nodes;
pub use tw::{
    Angle, BoxPaint, Divide, Easing, Mirror, Opacity, Paint, Progress, Ring, RingPlacement, Sides,
    Track, Tw, tw,
};
