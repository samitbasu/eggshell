//! `focus_on_appear`: a field that is built after not being built takes focus
//! once, and not again while it stays.

use eggshell_ui::testing::Driver;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

#[derive(Default)]
struct App {
    shown: bool,
    text: String,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(8.0)
            .p(8.0)
            .w_px(300.0),
        |b| {
            b.text_field("other", "")
                .named("Other")
                .on_change(|_: &mut App, _| {});
            if app.shown {
                b.text_field("name", app.text.clone())
                    .named("Name")
                    .focus_on_appear()
                    .on_change(|app: &mut App, text| app.text = text);
                b.stepper("count", 1.0, 0.0..=10.0)
                    .named("Count")
                    .focus_on_appear()
                    .on_change(|_: &mut App, _| {});
            }
        },
    );
}

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.frames(6);
    d
}

fn focused_label(d: &Driver<App>) -> Option<String> {
    ["Other", "Name", "Count"].into_iter().find_map(|label| {
        let node = d.harness().query_by_label(label)?;
        egui_kittest::kittest::NodeT::accesskit_node(&node)
            .is_focused()
            .then(|| label.to_owned())
    })
}

fn show(d: &mut Driver<App>, shown: bool) {
    d.app_mut().shown = shown;
    d.frames(4);
}

#[test]
fn a_field_that_appears_takes_focus_once() {
    let mut d = harness();
    d.harness().get_by_label("Other").focus();
    d.frames(3);
    assert_eq!(focused_label(&d).as_deref(), Some("Other"), "precondition");

    show(&mut d, true);
    let taken = focused_label(&d);
    assert!(
        matches!(taken.as_deref(), Some("Name" | "Count")),
        "an appearing field took focus: {taken:?}"
    );

    d.harness().get_by_label("Other").focus();
    d.frames(6);
    assert_eq!(
        focused_label(&d).as_deref(),
        Some("Other"),
        "a field that stays built does not take focus again"
    );

    show(&mut d, false);
    show(&mut d, true);
    let again = focused_label(&d);
    assert!(
        matches!(again.as_deref(), Some("Name" | "Count")),
        "it takes focus when it appears again: {again:?}"
    );
}

#[test]
fn without_the_option_an_appearing_field_leaves_focus_alone() {
    fn plain(b: &mut Builder<'_, '_, App>, app: &App) {
        b.text_field("other", "")
            .named("Other")
            .on_change(|_: &mut App, _| {});
        if app.shown {
            b.text_field("name", "")
                .named("Name")
                .on_change(|_: &mut App, _| {});
        }
    }
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        plain,
    )
    .build();
    d.frames(4);
    d.harness().get_by_label("Other").focus();
    d.frames(3);
    show(&mut d, true);
    assert_eq!(focused_label(&d).as_deref(), Some("Other"));
}
