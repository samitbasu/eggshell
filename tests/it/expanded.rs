//! A trigger whose popup is open says so, the web's `aria-expanded`: it takes
//! shadcn's open look, and a container's `has-aria-expanded` rule sees it.

use eggshell_ui::contract::Variant;
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Aria, Builder, Match, TextStyle, tw};
use egui::{Color32, Theme, vec2};

use crate::support::{self, box_at};

const OPEN_TINT: Color32 = Color32::from_rgb(200, 0, 200);

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        for key in ["menu_row", "select_row", "combo_row"] {
            let row = tw()
                .flex()
                .p(2.0)
                .w_px(220.0)
                .has(Match::aria(Aria::Expanded), |p| p.bg(OPEN_TINT));
            s.div_id(key, row, |s| match key {
                "menu_row" => s
                    .dropdown_menu("menu", "Actions")
                    .variant(Variant("outline"))
                    .items(|m| {
                        m.item("copy", "Copy").on_click(|_| {});
                        m.submenu("more", "More", |m| {
                            m.item("front", "Front").on_click(|_| {});
                        });
                    }),
                "select_row" => s
                    .select("role", 0, [(0, "Viewer"), (1, "Editor")])
                    .named("Role")
                    .on_change(|_, _| {}),
                _ => s
                    .combobox("fruit", None::<u8>, [(0u8, "Apple".to_owned())])
                    .named("Fruit")
                    .on_change(|_, _| {}),
            });
        }
    });
}

fn harness() -> Driver<()> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        view,
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(4);
    d
}

fn fill(d: &Driver<()>, path: &str) -> Color32 {
    box_at(d, d.rect(Target::suffix(path))).fill
}

fn row_tint(d: &Driver<()>, row: &str) -> Option<Color32> {
    support::fill_of(d, &format!("/\"Page\"/\"{row}\""))
}

#[test]
fn an_open_dropdowns_trigger_takes_the_open_look() {
    let mut d = harness();
    let tokens = Tokens::for_theme(Theme::Light);
    let trigger = r#"/"menu_row"/"menu""#;
    assert_ne!(fill(&d, trigger), tokens.muted, "precondition: closed");
    d.click(Target::suffix(trigger)).frames(6);
    assert_eq!(fill(&d, trigger), tokens.muted, "aria-expanded:bg-muted");
}

#[test]
fn a_container_sees_its_open_trigger() {
    for (row, trigger) in [
        ("menu_row", r#"/"menu_row"/"menu""#),
        ("select_row", r#"/"select_row"/"role""#),
        ("combo_row", "Chevron)"),
    ] {
        let mut d = harness();
        assert_ne!(
            row_tint(&d, row),
            Some(OPEN_TINT),
            "{row}: precondition: closed"
        );
        d.click(Target::suffix(trigger)).frames(6);
        assert_eq!(row_tint(&d, row), Some(OPEN_TINT), "{row}: open");
        for other in ["menu_row", "select_row", "combo_row"]
            .into_iter()
            .filter(|o| *o != row)
        {
            assert_ne!(
                row_tint(&d, other),
                Some(OPEN_TINT),
                "{other} while {row} is open"
            );
        }
    }
}

#[test]
fn an_open_submenus_item_takes_the_accent() {
    let mut d = harness();
    let tokens = Tokens::for_theme(Theme::Light);
    d.click(Target::suffix(r#"/"menu_row"/"menu""#)).frames(6);
    let item = r#"/("menu", Menu)/Box/"more""#;
    d.click(Target::suffix(item)).frames(6);
    // The pointer in the submenu: the item's accent is the open submenu's,
    // not the hover's.
    d.hover(Target::suffix(r#"/("more", Submenu)/Box/"front""#))
        .frames(6);
    assert_eq!(fill(&d, item), tokens.accent, "data-open:bg-accent");
}
