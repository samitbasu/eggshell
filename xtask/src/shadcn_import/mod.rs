//! `cargo xtask shadcn-import`: reads shadcn's components (the registry
//! `.tsx` and each style's CSS) at the pinned commit (`xtask/shadcn.pin`),
//! maps every Tailwind class onto eggshell, and writes each widget's `Sheet`
//! rules with a report of how each class mapped and what does not yet.
//! `--write` puts the rules of the migrated widgets into the shadcn preset's
//! crate (`src/sheets/`); `--check` fails if those are not what the pinned
//! source gives.

mod classes;
mod emit;
mod source;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan, MietteDiagnostic, NamedSource};

use classes::{Classifier, Fate, Gap, Radius, TwTable};
use emit::Home;
use source::{Class, Origin, Sources};

/// The components of the spike, by their registry file name.
const COMPONENTS: [&str; 32] = [
    "accordion",
    "alert",
    "alert-dialog",
    "button",
    "badge",
    "card",
    "checkbox",
    "combobox",
    "command",
    "dialog",
    "dropdown-menu",
    "field",
    "hover-card",
    "input",
    "input-group",
    "item",
    "navigation-menu",
    "popover",
    "radio-group",
    "select",
    "separator",
    "sheet",
    "slider",
    "sonner",
    "switch",
    "table",
    "tabs",
    "textarea",
    "tooltip",
    "skeleton",
    "spinner",
    "progress",
];

/// The widgets whose `Sheet` rules are emitted, and how eggshell names
/// their shadcn variants and sizes: shadcn's `default` button is eggshell's
/// `primary`, and a button that names no variant is `secondary`.
const WIDGETS: [emit::Widget; 31] = [
    emit::Widget {
        component: "button",
        home: Home::Core("button"),
        checked_in: true,
        variants: &[
            ("default", &["primary"]),
            ("secondary", &["secondary", "default"]),
        ],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "input",
        home: Home::Core("input"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "input-group",
        home: Home::Core("input"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[
            emit::Base {
                part: "Input",
                component: "input",
            },
            emit::Base {
                part: "Textarea",
                component: "textarea",
            },
        ],
        aliases: &[("input", "input-group-control")],
        relations: &[
            emit::Relation {
                variants: "has-[>[data-align=inline-start]]:[&>input]",
                part: "Input",
                key: ("addon", "inline-start"),
            },
            emit::Relation {
                variants: "has-[>[data-align=inline-end]]:[&>input]",
                part: "Input",
                key: ("addon", "inline-end"),
            },
        ],
        slot_aliases: &[("input-group-control", &["input-group-textarea"])],
        inherited: &[("group-data-[disabled=true]/input-group", "disabled")],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "select",
        home: Home::Core("select"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &SELECT,
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &SELECT_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "combobox",
        home: Home::Feature {
            module: "command",
            feature: "command",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &COMBOBOX,
        parts: None,
        bases: &[emit::Base {
            part: "Input",
            component: "input-group",
        }],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &COMBOBOX_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "command",
        home: Home::Feature {
            module: "command",
            feature: "command",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &COMMAND,
        parts: None,
        bases: &[emit::Base {
            part: "InputGroup",
            component: "input-group",
        }],
        aliases: &[("cmdk-group-heading", "command-group-heading")],
        relations: &[],
        slot_aliases: &[],
        inherited: &[(
            "group-data-[checked=true]/command-item",
            "data-[checked=true]",
        )],
        absent: &[],
        // A palette's list is flat, with no element for a group: the list is
        // its one group, whose `p-1` sits inside the scrolling list as shadcn's.
        copies: &[("Group", &["List"])],
    },
    emit::Widget {
        component: "dropdown-menu",
        home: Home::Core("menu"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &MENU,
        parts: Some("Part"),
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &MENU_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "separator",
        home: Home::Core("separator"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "tooltip",
        home: Home::Core("tooltip"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[(
            "tooltip-arrow",
            "the overlay draws the arrow, filled like the content",
        )],
        copies: &[],
    },
    emit::Widget {
        component: "checkbox",
        home: Home::Core("checkbox"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[
            // A card holds its one control, so the card has a focused
            // control when this one is focused.
            (
                "group-has-[:focus-visible]/field-label",
                "data-[card=true]:focus-visible",
            ),
            ("group-has-disabled/field", "disabled"),
        ],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "radio-group",
        home: Home::Core("radio"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: Some("Part"),
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[
            // A card holds its one control, so the card has a focused
            // control when this one is focused.
            (
                "group-has-[:focus-visible]/field-label",
                "data-[card=true]:focus-visible",
            ),
        ],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "switch",
        home: Home::Core("switch"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[
            // A card holds its one control, so the card has a focused
            // control when this one is focused.
            (
                "group-has-[:focus-visible]/field-label",
                "data-[card=true]:focus-visible",
            ),
            ("group-data-[size=default]/switch", "data-[size=default]"),
            ("group-data-[size=sm]/switch", "data-[size=sm]"),
        ],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "field",
        home: Home::Core("field"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[
            ("group-data-[disabled=true]/field", "disabled"),
            // A label that holds its field is a choice card, which eggshell
            // knows as it builds it.
            ("has-[>[data-slot=field]]", "data-[card=true]"),
            (
                "not-has-[:disabled,[data-disabled]]",
                "not-data-[disabled-control=true]",
            ),
        ],
        absent: &FIELD_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "table",
        home: Home::Core("table"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[("table-cell", &["table-footer-cell"])],
        inherited: &[("data-[state=selected]", "aria-selected")],
        absent: &[
            ("table-container", "the table scrolls in its own scroller"),
            ("table-caption", "eggshell's table has no caption"),
        ],
        copies: &[("Cell", &["FooterCell"])],
    },
    emit::Widget {
        component: "hover-card",
        home: Home::Feature {
            module: "hover_card",
            feature: "hover-card",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "dialog",
        home: Home::Feature {
            module: "dialog",
            feature: "dialog",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &DIALOG_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "alert-dialog",
        home: Home::Feature {
            module: "dialog",
            feature: "dialog",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &ALERT_DIALOG,
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &ALERT_DIALOG_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "sheet",
        home: Home::Feature {
            module: "dialog",
            feature: "dialog",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &SHEET_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "popover",
        home: Home::Feature {
            module: "dialog",
            feature: "dialog",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &POPOVER_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "alert",
        home: Home::Feature {
            module: "alert",
            feature: "alert",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "navigation-menu",
        home: Home::Feature {
            module: "navigation_menu",
            feature: "navigation-menu",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &NAVIGATION_MENU,
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &NAVIGATION_MENU_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "badge",
        home: Home::Feature {
            module: "badge",
            feature: "badge",
        },
        checked_in: true,
        variants: &[("default", &["primary"])],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        // A chip that can be clicked is shadcn's badge rendered as a link.
        inherited: &[("[a]", "data-[link=true]")],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "card",
        home: Home::Feature {
            module: "form",
            feature: "form",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &CARD_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "item",
        home: Home::Feature {
            module: "form",
            feature: "form",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &ITEM_ABSENT,
        copies: &[],
    },
    emit::Widget {
        component: "slider",
        home: Home::Core("slider"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &SLIDER,
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "accordion",
        home: Home::Feature {
            module: "accordion",
            feature: "disclosure",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &ACCORDION,
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "sonner",
        home: Home::Feature {
            module: "notify",
            feature: "notify",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: Some("Part"),
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "skeleton",
        home: Home::Feature {
            module: "skeleton",
            feature: "skeleton",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "spinner",
        home: Home::Feature {
            module: "spinner",
            feature: "spinner",
        },
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
    emit::Widget {
        component: "progress",
        home: Home::Core("progress"),
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[(
            "progress-indicator",
            "eggshell paints the fill, to end straight at the value and follow the track's rounded end, in the style's range colour",
        )],
        copies: &[],
    },
    emit::Widget {
        component: "tabs",
        home: Home::Core("tabs"),
        checked_in: true,
        variants: &[("line", &[])],
        sizes: &[],
        facts: &TABS,
        parts: None,
        bases: &[],
        aliases: &[],
        relations: &[],
        slot_aliases: &[],
        inherited: &[],
        absent: &[],
        copies: &[],
    },
];

/// The parts of shadcn's item that a form's row does not draw: a row is an
/// item's content and actions, alone in its section.
const ITEM_ABSENT: [(&str, &str); 5] = {
    const ROW: &str = "a form's row is a title and its value";
    [
        ("item-group", ROW),
        ("item-separator", ROW),
        ("item-media", ROW),
        ("item-header", ROW),
        ("item-footer", ROW),
    ]
};

/// The parts of shadcn's field that eggshell does not draw: its choice
/// controls stand alone or in a card, not in sets and groups.
const FIELD_ABSENT: [(&str, &str); 5] = {
    const UNGROUPED: &str = "eggshell's choice fields are not set in groups";
    [
        ("field-set", UNGROUPED),
        ("field-group", UNGROUPED),
        ("field-separator", UNGROUPED),
        ("field-separator-content", UNGROUPED),
        ("field-error", "eggshell's choice fields show no error"),
    ]
};

/// eggshell's combobox says nothing matches only while nothing does.
const COMBOBOX: [emit::Assumed; 1] = [emit::Assumed {
    variant: "group-data-empty/combobox-content",
    fact: emit::Fact::Holds,
    why: "eggshell draws the empty note only while the list is empty",
}];

/// The parts of shadcn's combobox that eggshell's does not name: one value,
/// no chips or groups, and its own end buttons.
const COMBOBOX_ABSENT: [(&str, &str); 10] = {
    const ONE: &str = "eggshell's combobox picks one value, shown in its field";
    const BUTTON: &str = "shadcn's is an input-group button, whose look the combobox's sheet does not carry; the widget draws its own";
    [
        ("combobox-value", ONE),
        ("combobox-chips", ONE),
        ("combobox-chip", ONE),
        ("combobox-chip-remove", ONE),
        ("combobox-chip-input", ONE),
        ("combobox-chip-indicator-icon", ONE),
        ("input-group-button", BUTTON),
        (
            "combobox-group",
            "eggshell's combobox lists its options in one run",
        ),
        (
            "combobox-separator",
            "eggshell's combobox lists its options in one run",
        ),
        ("combobox-collection", "a list of the options, not drawn"),
    ]
};

/// The scrim behind a modal surface is the overlay's.
const SCRIM: &str = "the overlay draws the scrim behind a modal surface";

/// The parts of shadcn's dialog that eggshell's surface does not draw.
const DIALOG_ABSENT: [(&str, &str); 1] = [("dialog-overlay", SCRIM)];

/// eggshell's alert dialog has shadcn's default size only.
const ALERT_DIALOG: [emit::Assumed; 4] = {
    use emit::{Assumed, Fact::*};
    const DEFAULT: &str = "eggshell's alert dialog has the default size only";
    [
        Assumed {
            variant: "data-[size=default]",
            fact: Holds,
            why: DEFAULT,
        },
        Assumed {
            variant: "data-[size=sm]",
            fact: Never,
            why: DEFAULT,
        },
        Assumed {
            variant: "group-data-[size=default]/alert-dialog-content",
            fact: Holds,
            why: DEFAULT,
        },
        Assumed {
            variant: "group-data-[size=sm]/alert-dialog-content",
            fact: Never,
            why: DEFAULT,
        },
    ]
};

/// The parts of shadcn's alert dialog that eggshell's surface does not draw.
const ALERT_DIALOG_ABSENT: [(&str, &str); 4] = {
    const BUTTON: &str = "the footer's buttons are core buttons, in the style's variants";
    [
        ("alert-dialog-overlay", SCRIM),
        ("alert-dialog-media", "eggshell's alert dialog has no media"),
        ("alert-dialog-action", BUTTON),
        ("alert-dialog-cancel", BUTTON),
    ]
};

/// The parts of shadcn's sheet that eggshell's surface does not draw.
const SHEET_ABSENT: [(&str, &str); 1] = [("sheet-overlay", SCRIM)];

/// eggshell's navigation menu opens each panel under its own trigger
/// (shadcn's `viewport={false}`), and is built on Radix, whose open trigger is
/// `data-open`, not Base UI's `data-popup-open`.
const NAVIGATION_MENU: [emit::Assumed; 3] = {
    use emit::{Assumed, Fact::*};
    const RADIX: &str = "eggshell's navigation menu is Radix's: an open trigger is `data-open`";
    [
        Assumed {
            variant: "group-data-[viewport=false]/navigation-menu",
            fact: Holds,
            why: "eggshell's navigation menu has no shared viewport yet",
        },
        Assumed {
            variant: "data-popup-open",
            fact: Never,
            why: RADIX,
        },
        Assumed {
            variant: "group-data-popup-open/navigation-menu-trigger",
            fact: Never,
            why: RADIX,
        },
    ]
};

/// The parts of shadcn's navigation menu that eggshell's does not draw.
const NAVIGATION_MENU_ABSENT: [(&str, &str); 2] = {
    const VIEWPORT: &str = "eggshell's navigation menu has no shared viewport yet";
    [
        ("navigation-menu-viewport", VIEWPORT),
        ("navigation-menu-viewport-wrapper", VIEWPORT),
    ]
};

/// The parts of shadcn's popover that eggshell's does not draw: it holds
/// what the app builds.
const POPOVER_ABSENT: [(&str, &str); 3] = {
    const BODY: &str = "eggshell's popover holds what the app builds";
    [
        ("popover-header", BODY),
        ("popover-title", BODY),
        ("popover-description", BODY),
    ]
};

/// The parts of shadcn's card a form section does not draw: the section's
/// rows and title are its own.
const CARD_ABSENT: [(&str, &str); 6] = {
    const ROWS: &str = "a form section's rows and title are its own";
    [
        ("card-header", ROWS),
        ("card-title", ROWS),
        ("card-description", ROWS),
        ("card-action", ROWS),
        ("card-content", ROWS),
        ("card-footer", ROWS),
    ]
};

/// eggshell's sliders are horizontal, and Radix's thumb is a `<span>`,
/// which `:disabled` never matches: a disabled slider fades once, at its root.
const SLIDER: [emit::Assumed; 3] = {
    use emit::{Assumed, Fact::*};
    const HORIZONTAL: &str = "eggshell's sliders are horizontal only";
    [
        Assumed {
            variant: "disabled",
            fact: Never,
            why: "Radix's thumb is a span, which `:disabled` never matches",
        },
        Assumed {
            variant: "data-horizontal",
            fact: Holds,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "data-vertical",
            fact: Never,
            why: HORIZONTAL,
        },
    ]
};

/// A checked command shows its mark, its shortcut too (#345): shadcn's
/// palette hides the mark beside a shortcut.
const COMMAND: [emit::Assumed; 1] = [emit::Assumed {
    variant: "group-has-data-[slot=command-shortcut]/command-item",
    fact: emit::Fact::Never,
    why: "a checked command shows its mark beside its shortcut (#345)",
}];

/// eggshell turns the accordion's one chevron rather than hiding it.
const ACCORDION: [emit::Assumed; 1] = [emit::Assumed {
    variant: "group-aria-expanded/accordion-trigger",
    fact: emit::Fact::Never,
    why: "eggshell turns the accordion's one chevron, never hides it",
}];

/// eggshell draws an open menu only: a closed one is gone.
const MENU: [emit::Assumed; 1] = [emit::Assumed {
    variant: "data-[state=closed]",
    fact: emit::Fact::Never,
    why: "eggshell draws a menu only while it is open",
}];

/// The parts of shadcn's dropdown menu that eggshell's does not name.
const MENU_ABSENT: [(&str, &str); 1] = [(
    "rtl-flip",
    "a submenu's chevron is a glyph of its trigger, last in the row",
)];

/// eggshell's select opens its list below the trigger and keeps it off the
/// trigger by the menu's offset, as Radix's `popper` position does.
const SELECT: [emit::Assumed; 5] = {
    use emit::{Assumed, Fact::*};
    const BELOW: &str =
        "eggshell's select opens its list below the trigger, as Radix's `popper` position does";
    const OFFSET: &str = "the overlay keeps the list off its trigger by `MenuStyle::offset`";
    [
        Assumed {
            variant: "data-[align-trigger=true]",
            fact: Never,
            why: BELOW,
        },
        Assumed {
            variant: "data-[side=bottom]",
            fact: Never,
            why: OFFSET,
        },
        Assumed {
            variant: "data-[side=top]",
            fact: Never,
            why: OFFSET,
        },
        Assumed {
            variant: "data-[side=left]",
            fact: Never,
            why: OFFSET,
        },
        Assumed {
            variant: "data-[side=right]",
            fact: Never,
            why: OFFSET,
        },
    ]
};

/// The parts of shadcn's select that eggshell's does not draw. Its options
/// are one group, `select-group`.
const SELECT_ABSENT: [(&str, &str); 5] = {
    const UNGROUPED: &str = "eggshell's select lists its options without groups";
    const SCROLLS: &str = "a list taller than its room scrolls in the overlay's own scroller";
    [
        ("select-label", UNGROUPED),
        ("select-separator", UNGROUPED),
        ("select-viewport", SCROLLS),
        ("select-scroll-up-button", SCROLLS),
        ("select-scroll-down-button", SCROLLS),
    ]
};

/// eggshell's tabs are horizontal, and their list has only the default look:
/// the `line` variant names no eggshell variant.
const TABS: [emit::Assumed; 7] = {
    use emit::{Assumed, Fact::*};
    const HORIZONTAL: &str = "eggshell's tabs are horizontal only";
    const DEFAULT: &str = "eggshell's tab list has only the default look";
    [
        Assumed {
            variant: "data-horizontal",
            fact: Holds,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "data-vertical",
            fact: Never,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "group-data-horizontal/tabs",
            fact: Holds,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "group-data-vertical/tabs",
            fact: Never,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "group-data-[variant=default]/tabs-list",
            fact: Holds,
            why: DEFAULT,
        },
        Assumed {
            variant: "group-data-[variant=line]/tabs-list",
            fact: Never,
            why: DEFAULT,
        },
        Assumed {
            variant: "data-[variant=line]",
            fact: Never,
            why: DEFAULT,
        },
    ]
};

/// The `sheets` module: a module per migrated `component` and style, and
/// `write`, which writes them all for a style and theme, in the order of
/// `components`.
fn index(components: &[(&str, Home)]) -> String {
    let module = |style: &str, component: &str| format!("{style}_{}", component.replace('-', "_"));
    let mut out = String::from(
        "//! The widgets' rules in each shadcn style, generated from the pinned shadcn\n\
         //! source by `cargo xtask shadcn-import --write`; CI fails if they drift\n\
         //! (`--check`). Each rule's classes carry the shadcn class they came from.\n\
         //!\n\
         //! This file is generated too. Do not edit.\n\n\
         use crate::sheet::Sheet;\n\
         use egui::Theme;\n\n\
         use crate::theme::ShadcnStyle;\n",
    );
    out.push_str(
        "\n/// One half of a widget's rules: its light or its dark theme.\n\
         type Half = fn(&mut Sheet);\n",
    );
    for style in STYLES {
        for (component, home) in components {
            let _ = write!(out, "\n{}mod {};", home.cfg(), module(style, component));
        }
    }
    out.push_str(
        "\n\n/// Writes the rules of `style` for `theme` into `sheet`. Their colours are\n\
         /// the style's core tokens, which the sheet is resolved with.\n\
         pub(crate) fn write(sheet: &mut Sheet, style: ShadcnStyle, theme: Theme) {\n\
         \x20   let halves: &[(Half, Half)] = match style {\n",
    );
    for style in STYLES {
        let variant = format!("{}{}", style[..1].to_uppercase(), &style[1..]);
        let _ = writeln!(out, "        ShadcnStyle::{variant} => &[");
        for (component, home) in components {
            let m = module(style, component);
            let _ = writeln!(out, "            {}({m}::light, {m}::dark),", home.cfg());
        }
        out.push_str("        ],\n");
    }
    out.push_str(
        "    };\n\
         \x20   for (light, dark) in halves {\n\
         \x20       match theme {\n\
         \x20           Theme::Light => light(sheet),\n\
         \x20           Theme::Dark => dark(sheet),\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
    out
}

/// Where a widget's checked-in rules for `style` live.
fn checked_in(root: &Path, style: &str, component: &str) -> PathBuf {
    root.join(SHEETS)
        .join(format!("{style}_{}.rs", component.replace('-', "_")))
}

/// Where the generated sheets are checked in, from the workspace root.
const SHEETS: &str = "src/sheets";

/// What the run does besides the report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The report and every widget's rules, under `out`.
    Report,
    /// The migrated widgets' rules into the preset's crate.
    Write,
    /// Fail if the preset's crate holds other rules than the source gives.
    Check,
}

/// The commit the reference must be at.
fn pin(root: &Path) -> Result<String> {
    Ok(read(&root.join("xtask/shadcn.pin"))?.trim().to_owned())
}

/// Fails unless the shadcn checkout at `reference` is at the pinned commit.
fn check_pin(root: &Path, reference: &Path) -> Result<()> {
    let want = pin(root)?;
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(reference)
        .args(["rev-parse", "HEAD"])
        .output()
        .with_context(|| {
            format!(
                "reading the commit of {}; check out shadcn-ui/ui at {want} there \
                 (a sparse checkout of apps/v4/registry is enough)",
                reference.display()
            )
        })?;
    let have = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    anyhow::ensure!(
        head.status.success() && have == want,
        "{} is at `{have}`, not the pinned {want} (xtask/shadcn.pin)",
        reference.display()
    );
    Ok(())
}

/// The migrated widgets' rules for every style, written or compared.
fn sheets(root: &Path, registry: &Path, classifier: &Classifier<'_>, mode: Mode) -> Result<()> {
    let migrated: Vec<(&str, Home)> = WIDGETS
        .iter()
        .filter(|w| w.checked_in)
        .map(|w| (w.component, w.home))
        .collect();
    let mut files = vec![(root.join(SHEETS).join("mod.rs"), index(&migrated))];
    for style in STYLES {
        let mut scratch = Sources::default();
        let (rows, _) = extract(registry, style, classifier, &mut scratch)?;
        for widget in WIDGETS.iter().filter(|w| w.checked_in) {
            let emitted = emit::emit(classifier, &rows, &scratch, widget, style);
            files.push((checked_in(root, style, widget.component), emitted.rust));
        }
    }
    let mut stale = Vec::new();
    for (path, rust) in files {
        match mode {
            Mode::Write => fs::write(&path, &rust)?,
            Mode::Check | Mode::Report => {
                if fs::read_to_string(&path).ok().as_deref() != Some(rust.as_str()) {
                    stale.push(path.display().to_string());
                }
            }
        }
    }
    anyhow::ensure!(
        stale.is_empty(),
        "the generated sheets differ from the pinned shadcn source; run `cargo xtask shadcn-import --write`:\n  {}",
        stale.join("\n  ")
    );
    Ok(())
}

pub struct Plan {
    /// What the run does besides the report.
    pub mode: Mode,
    /// The shadcn checkout.
    pub reference: PathBuf,
    /// The style, as `style-<name>.css`.
    pub style: String,
    /// Where the report goes.
    pub out: PathBuf,
}

/// One class of one component, as the style expands it.
struct Row {
    component: &'static str,
    origin: Origin,
    /// The part it styles, as shadcn's `data-slot` names it.
    slot: String,
    /// The `cn-*` class it came from, if it came through the CSS.
    via: Option<String>,
    class: Class,
    fate: Fate,
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// The theme's `--radius` in points, from `registry/themes.ts`.
fn theme_radius(themes: &str) -> Option<f32> {
    let at = themes.find("radius: \"")? + "radius: \"".len();
    let value = &themes[at..at + themes[at..].find('"')?];
    value
        .strip_suffix("rem")?
        .parse::<f32>()
        .ok()
        .map(|r| r * 16.0)
}

/// `cn-*` rules that only a theme setting other than the default applies:
/// `menuColor` defaults to `"default"` (`registry/config.ts:186`), and
/// `cn-menu-translucent` styles the `*-translucent` menu colours.
const OFF_BY_DEFAULT: [&str; 1] = ["cn-menu-translucent"];

/// The styles the preset has, as shadcn names their CSS.
const STYLES: [&str; 2] = ["vega", "lyra"];

/// Every class of the components in `style`'s CSS, with its fate, and the
/// style's `@apply` rules.
fn extract(
    registry: &Path,
    style: &str,
    classifier: &Classifier<'_>,
    sources: &mut Sources,
) -> Result<(Vec<Row>, source::Rules)> {
    let css = read(&registry.join(format!("styles/style-{style}.css")))?;
    let css_file = sources.add(format!("registry/styles/style-{style}.css"), css.clone());
    let rules = source::apply_rules(&css, css_file);
    let mut rows = Vec::new();
    for name in COMPONENTS {
        let text = read(&registry.join(format!("bases/radix/ui/{name}.tsx")))?;
        let file = sources.add(format!("registry/bases/radix/ui/{name}.tsx"), text.clone());
        for (origin, slot, classes) in source::component(&text, file).parts {
            if is_hidden_twin(name, &slot, &classes) {
                continue;
            }
            for class in classes {
                if OFF_BY_DEFAULT.contains(&class.text.as_str()) {
                    continue;
                }
                match rules.get(&class.text) {
                    Some(applied) => rows.extend(applied.iter().map(|inner| Row {
                        component: name,
                        origin: origin.clone(),
                        slot: slot.clone(),
                        via: Some(class.text.clone()),
                        fate: classifier.classify(&inner.text),
                        class: inner.clone(),
                    })),
                    None => rows.push(Row {
                        component: name,
                        origin: origin.clone(),
                        slot: slot.clone(),
                        via: None,
                        fate: classifier.classify(&class.text),
                        class,
                    }),
                }
            }
        }
    }
    Ok((rows, rules))
}

/// Slots shadcn renders twice, the second `hidden` until a state shows it
/// in place of the first, where eggshell draws one element and turns it:
/// accordion's up chevron is its down chevron turned. The hidden twin's
/// classes are dropped.
const TURNED_TWINS: [(&str, &str); 1] = [("accordion", "accordion-trigger-icon")];

/// Whether an element of `component` is the hidden twin of a turned slot.
fn is_hidden_twin(component: &str, slot: &str, classes: &[source::Class]) -> bool {
    TURNED_TWINS.contains(&(component, slot)) && classes.iter().any(|c| c.text == "hidden")
}

/// `--check` with the defaults, for `cargo xtask ci`.
pub fn check() -> Result<()> {
    run(&Plan {
        mode: Mode::Check,
        reference: PathBuf::from("../ref/shadcn"),
        style: "vega".to_owned(),
        out: PathBuf::from("target/shadcn-import"),
    })
}

/// The `Tw` classes and methods, as `src/tw/*.rs` documents them.
fn tw_table(root: &Path) -> Result<TwTable> {
    let tw_sources = fs::read_dir(root.join("src/tw"))?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| read(&p))
        .collect::<Result<Vec<_>>>()?;
    Ok(TwTable::read(&tw_sources))
}

/// The `Tw` methods that take spacing units.
pub fn unit_methods(root: &Path) -> Result<BTreeSet<String>> {
    Ok(tw_table(root)?.units)
}

pub fn run(plan: &Plan) -> Result<()> {
    let root = crate::process::workspace_root();
    let registry = root.join(&plan.reference).join("apps/v4/registry");
    let tw = tw_table(&root)?;
    let tokens = classes::token_names(&read(&root.join("src/theme/tokens.rs"))?);
    let radius = theme_radius(&read(&registry.join("themes.ts"))?)
        .context("no `radius` in registry/themes.ts")?;
    let classifier = Classifier {
        tw: &tw,
        tokens: &tokens,
        radius: Radius { base: radius },
    };
    check_pin(&root, &root.join(&plan.reference))?;
    if plan.mode != Mode::Report {
        sheets(&root, &registry, &classifier, plan.mode)?;
        println!(
            "sheets: {}",
            if plan.mode == Mode::Write {
                "written"
            } else {
                "up to date"
            }
        );
        return Ok(());
    }
    let mut sources = Sources::default();
    let (rows, _) = extract(&registry, &plan.style, &classifier, &mut sources)?;
    let diagnostics = diagnostics(&rows, &sources);
    eprint!("{diagnostics}");
    let mut report = report(plan, &rows, &tw, radius);
    for style in STYLES {
        let mut scratch = Sources::default();
        let (rows, _) = extract(&registry, style, &classifier, &mut scratch)?;
        let _ = writeln!(
            report,
            "## 3. `{style}`: sheets
"
        );
        let sheets = root.join(&plan.out).join("sheets").join(style);
        fs::create_dir_all(&sheets)?;
        for widget in &WIDGETS {
            let emitted = emit::emit(&classifier, &rows, &scratch, widget, style);
            let file = format!("{}.rs", widget.component.replace('-', "_"));
            fs::write(sheets.join(&file), &emitted.rust)?;
            sheet_section(&mut report, style, widget, &file, &emitted);
        }
    }
    report.push_str(&listings(&rows));
    let out = root.join(&plan.out);
    fs::create_dir_all(&out)?;
    let report_path = out.join("report.md");
    fs::write(&report_path, &report)?;
    fs::write(out.join("diagnostics.txt"), &diagnostics)?;
    println!("{}", summary_line(&rows));
    println!("report: {}", report_path.display());
    Ok(())
}

/// The report's account of a generated sheet: how each class got there,
/// what it settles for the widget, and what is not there yet.
fn sheet_section(
    report: &mut String,
    style: &str,
    widget: &emit::Widget,
    file: &str,
    emitted: &emit::Emitted,
) {
    let _ = writeln!(
        report,
        "### The `Sheet` rules for `{}` (`sheets/{style}/{file}`, `{}`)\n",
        widget.component,
        widget.part_enum()
    );
    let _ = writeln!(report, "| mapping kind | classes |\n|---|---|");
    for (kind, count) in &emitted.kinds {
        let _ = writeln!(report, "| {} | {count} |", kind.label());
    }
    let _ = writeln!(report, "\nUnmapped, once each:\n");
    for (source, why) in &emitted.unmapped {
        let _ = writeln!(report, "- `{source}`: {why}");
    }
    if !emitted.assumed.is_empty() {
        let _ = writeln!(report, "\nTaken to hold:\n");
        for (variant, why) in &emitted.assumed {
            let _ = writeln!(report, "- `{variant}:`: {why}");
        }
    }
    if !emitted.dropped.is_empty() {
        let _ = writeln!(
            report,
            "\nDropped, a state the widget never has, once each:\n"
        );
        for (source, why) in &emitted.dropped {
            let _ = writeln!(report, "- `{source}`: {why}");
        }
    }
    let _ = writeln!(report);
}

fn is_clean(fate: &Fate) -> bool {
    !matches!(fate, Fate::Gap { .. } | Fate::Marker)
}

fn summary_line(rows: &[Row]) -> String {
    let counted: Vec<_> = rows.iter().filter(|r| r.fate != Fate::Marker).collect();
    let clean = counted.iter().filter(|r| is_clean(&r.fate)).count();
    format!(
        "{clean} of {} classes map ({}%); {} have no equivalent yet",
        counted.len(),
        clean * 100 / counted.len().max(1),
        counted.len() - clean
    )
}

/// One miette diagnostic per distinct unsupported class, at its first
/// occurrence, rendered without colour.
fn diagnostics(rows: &[Row], sources: &Sources) -> String {
    let mut first: BTreeMap<&str, (&Row, usize)> = BTreeMap::new();
    for row in rows {
        if let Fate::Gap { .. } = row.fate {
            first.entry(&row.class.text).or_insert((row, 0)).1 += 1;
        }
    }
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let mut out = String::new();
    for (class, (row, count)) in first {
        let Fate::Gap { gap, why } = &row.fate else {
            continue;
        };
        let file = &sources.files[row.class.file];
        let help = match count {
            1 => gap.label().to_owned(),
            n => format!("{}; {n} occurrences", gap.label()),
        };
        let diagnostic = MietteDiagnostic::new(format!("`{class}` has no eggshell equivalent"))
            .with_code(format!("shadcn_import::{gap:?}"))
            .with_label(LabeledSpan::at(row.class.span.clone(), why.clone()))
            .with_help(help);
        let report = miette::Report::new(diagnostic)
            .with_source_code(NamedSource::new(file.name.clone(), file.text.clone()));
        let _ = handler.render_report(&mut out, report.as_ref());
        out.push('\n');
    }
    out
}

fn report(plan: &Plan, rows: &[Row], tw: &TwTable, radius: f32) -> String {
    let mut r = String::new();
    let _ = writeln!(
        r,
        "# shadcn-import: Button, Badge, Combobox (`style-{}`)\n",
        plan.style
    );
    let _ = writeln!(
        r,
        "Read from `{}`. `Tw`'s table: {} class patterns and {} state variants, read from `src/tw/*.rs`'s docs. Theme radius: {radius} pt (`themes.ts`); the `--radius-*` scale (sm −4, md −2, lg ±0, xl +4) is shadcn's `globals.css`, not in the snapshot.\n",
        plan.reference.display(),
        tw.len(),
        tw.variants.len()
    );
    let _ = writeln!(r, "**{}**\n", summary_line(rows));

    let _ = writeln!(r, "## 1. How much maps\n");
    let _ = writeln!(
        r,
        "| component | classes | `Tw` call | preset token | preset state (`dark:`, `disabled:`) | no equivalent | maps |"
    );
    let _ = writeln!(r, "|---|---|---|---|---|---|---|");
    for name in COMPONENTS {
        let of: Vec<_> = rows
            .iter()
            .filter(|row| row.component == name && row.fate != Fate::Marker)
            .collect();
        let count = |f: fn(&Fate) -> bool| of.iter().filter(|row| f(&row.fate)).count();
        let tw = count(|f| matches!(f, Fate::Tw { .. }));
        let token = count(|f| matches!(f, Fate::Token { .. }));
        let preset = count(|f| matches!(f, Fate::Preset { .. }));
        let gap = count(|f| matches!(f, Fate::Gap { .. }));
        let _ = writeln!(
            r,
            "| {name} | {} | {tw} | {token} | {preset} | {gap} | {}% |",
            of.len(),
            (tw + token + preset) * 100 / of.len().max(1)
        );
    }

    let _ = writeln!(r, "\n## 2. No equivalent yet, grouped\n");
    let _ = writeln!(
        r,
        "Each distinct class once, with its count; the spans are in `diagnostics.txt`.\n"
    );
    let mut groups: BTreeMap<Gap, BTreeMap<&str, (usize, BTreeSet<&str>)>> = BTreeMap::new();
    for row in rows {
        if let Fate::Gap { gap, .. } = &row.fate {
            let entry = groups
                .entry(*gap)
                .or_default()
                .entry(row.class.text.as_str())
                .or_default();
            entry.0 += 1;
            entry.1.insert(row.component);
        }
    }
    for (gap, classes) in &groups {
        let total: usize = classes.values().map(|(n, _)| n).sum();
        let _ = writeln!(
            r,
            "### {} — {total} uses, {} distinct\n",
            gap.label(),
            classes.len()
        );
        for (class, (n, components)) in classes {
            let components: Vec<_> = components.iter().copied().collect();
            let _ = writeln!(r, "- `{class}` ×{n} ({})", components.join(", "));
        }
        let _ = writeln!(r);
    }

    r
}

/// What each variant's classes of the button and badge map to.
fn listings(rows: &[Row]) -> String {
    let mut r = String::new();
    for name in ["button", "badge"] {
        let _ = writeln!(r, "## Generated `{name}` looks, by variant\n");
        let _ = writeln!(
            r,
            "What each variant's classes map to (the selected style's CSS).\n"
        );
        let mut by_origin: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in rows
            .iter()
            .filter(|row| row.component == name && row.fate != Fate::Marker)
        {
            let key = match &row.origin {
                Origin::Base { .. } => "base".to_owned(),
                Origin::Variant { variant, name, .. } => format!("{variant}={name}"),
                Origin::Slot { function } => format!("slot {function}"),
            };
            let mark = if is_clean(&row.fate) { "" } else { "✗ " };
            by_origin.entry(key).or_default().push(match &row.via {
                Some(via) => format!("{mark}`{}` → {} (from `{via}`)", row.class.text, row.fate),
                None => format!("{mark}`{}` → {}", row.class.text, row.fate),
            });
        }
        for (origin, lines) in by_origin {
            let _ = writeln!(
                r,
                "<details><summary><code>{origin}</code> ({} classes)</summary>\n",
                lines.len()
            );
            for line in lines {
                let _ = writeln!(r, "- {line}");
            }
            let _ = writeln!(r, "\n</details>\n");
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheets_index_declares_and_writes_every_migrated_widget_in_each_style() {
        let index = index(&[
            ("button", Home::Core("button")),
            ("input-group", Home::Core("input")),
            (
                "combobox",
                Home::Feature {
                    module: "command",
                    feature: "command",
                },
            ),
        ]);
        assert!(index.contains("\n#[cfg(feature = \"command\")] mod vega_combobox;"));
        assert!(index.contains(
            "#[cfg(feature = \"command\")] (vega_combobox::light, vega_combobox::dark),"
        ));
        for module in [
            "vega_button",
            "vega_input_group",
            "lyra_button",
            "lyra_input_group",
        ] {
            assert!(index.contains(&format!("\nmod {module};")), "{module}");
            assert!(
                index.contains(&format!("({module}::light, {module}::dark),")),
                "{module} is written"
            );
        }
        let vega = index.find("ShadcnStyle::Vega => &[").expect("vega");
        let lyra = index.find("ShadcnStyle::Lyra => &[").expect("lyra");
        assert!(index[vega..lyra].contains("vega_input_group::light"));
        assert!(
            !index[vega..lyra].contains("lyra_"),
            "each style writes its own"
        );
        assert!(
            index.find("(vega_button::light").expect("button")
                < index.find("(vega_input_group::light").expect("group"),
            "in the table's order"
        );
    }
}
