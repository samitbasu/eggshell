//! TODO(prose): an accordion: sections in a column, opened one at a time or
//! any number at once, as the mode's type says.
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui::owned::accordion::{Ext, Single};
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
//! enum Faq { Access, Motion }
//!
//! struct App { faq: Option<Faq> }
//!
//! fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
//!     b.accordion("faq", Single::collapsible(app.faq))
//!         .on_change(|app: &mut App, open| app.faq = open)
//!         .items(|acc| {
//!             acc.item(Faq::Access, "Is it accessible?")
//!                 .children(|b| b.text(tw(), "Yes."));
//!             acc.item(Faq::Motion, "Is it animated?")
//!                 .children(|b| b.text(tw(), "Yes."));
//!         });
//! }
//! ```

#[cfg(feature = "accordion")]
mod widget;

#[cfg(feature = "accordion")]
pub use widget::{
    Always, BoundAccordion, Chain, Collapsible, Ext, ItemChain, Items, Mode, Multiple, Single,
    Value,
};

/// TODO(prose): a disclosure section's and an accordion's anatomy for the
/// style's sheet, as shadcn's `accordion.tsx` names it: a section is an
/// accordion's item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccordionPart {
    /// TODO(prose): an accordion's column of items.
    Root,
    /// TODO(prose): a section: its header and its region.
    Item,
    /// TODO(prose): the header that opens and closes the section.
    Trigger,
    /// TODO(prose): the header's chevron.
    TriggerIcon,
    /// TODO(prose): the region: what opens and closes.
    Content,
    /// TODO(prose): the region's content, inside it.
    ContentInner,
}

impl crate::sheet::Part for AccordionPart {
    const SOURCE: &'static str = "accordion.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "accordion",
            Self::Item => "accordion-item",
            Self::Trigger => "accordion-trigger",
            Self::TriggerIcon => "accordion-trigger-icon",
            Self::Content => "accordion-content",
            Self::ContentInner => "accordion-content-inner",
        }
    }
}
