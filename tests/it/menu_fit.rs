//! A dropdown menu taller than the room below its trigger stays in the
//! window and scrolls, as shadcn's `max-h-(--available-height)
//! overflow-y-auto` does; its first frame is measured, not shown.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Vec2, vec2};

use crate::support;

const ITEMS: usize = 24;
const MENU: &str = r#"/("file", Menu)/Box"#;
const ROWS: &str = r#"/("file", Menu)/Box/Rows"#;

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().p(8.0), |b| {
        b.dropdown_menu("file", "File").items(|m| {
            for index in 0..ITEMS {
                m.item(index, format!("Item {index}")).on_click(|_| {});
            }
        });
    });
}

fn harness(window: Vec2) -> Driver<()> {
    let mut d = support::driver(window, base_text(), (), view).build();
    d.frames(4);
    d
}

#[test]
fn a_menu_taller_than_its_room_stays_in_the_window_and_scrolls() {
    let mut d = harness(vec2(300.0, 300.0));
    d.click(Target::suffix(r#"/"file""#)).frames(6);
    let window = d.ctx().content_rect();
    let menu = d.rect(Target::suffix(MENU));
    assert!(
        window.contains_rect(menu),
        "{menu:?} is cut off by {window:?}"
    );
    let rows = d.rect(Target::suffix(ROWS));
    assert!(
        menu.contains_rect(rows),
        "the rows scroll inside the menu's box"
    );
    let last = d.rect(Target::suffix(&format!(
        r#"/("file", Menu)/Box/Rows/{}"#,
        ITEMS - 1
    )));
    assert!(
        last.min.y > rows.max.y,
        "precondition: the last item is scrolled out of view"
    );
}

#[test]
fn a_menu_with_room_does_not_scroll() {
    let mut d = harness(vec2(300.0, 1200.0));
    d.click(Target::suffix(r#"/"file""#)).frames(6);
    assert!(d.painted().shows(&format!("Item {}", ITEMS - 1)));
    assert!(
        !support::nodes(&d)
            .iter()
            .any(|(path, _)| path.ends_with(ROWS)),
        "no scroll container, so no gutter"
    );
}

#[test]
fn a_menu_is_measured_unseen_for_a_frame_before_it_shows() {
    let mut d = harness(vec2(300.0, 300.0));
    let trigger = d.rect(Target::suffix(r#"/"file""#)).center();
    support::press_and_release(&mut d, trigger);
    let measured = (0..6)
        .find(|_| {
            d.frames(1);
            support::nodes(&d)
                .iter()
                .any(|(path, _)| path.ends_with(MENU))
        })
        .expect("the menu is laid out");
    let menu = d.rect(Target::suffix(MENU));
    assert!(
        !d.painted().shows("Item 0"),
        "frame {measured}: nothing of it is painted"
    );
    assert!(
        support::try_box_at(&d, menu).is_none(),
        "frame {measured}: its box is not painted"
    );
    d.frames(1);
    assert!(d.painted().shows("Item 0"), "it shows on the next frame");
}
