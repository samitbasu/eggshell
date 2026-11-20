//! `Nav::Column` and `nav_item`, through the keys: Up and Down move real
//! focus between the marked items, Home and End go to the ends, buttons that
//! are not items are passed by, and every item stays a Tab stop.

use eggshell_ui::overlay::{Nav, Wrap};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::accesskit::Role;
use egui::{Color32, FontFamily, Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig};

struct App {
    wrap: Wrap,
}

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

/// ```html
/// <div> <button>A</button> <p><button>Inside</button></p> <button>B</button> <button>C</button> </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("column", tw().flex().flex_col().w_px(200.0), |b| {
        for (key, name) in [("a", "A"), ("b", "B"), ("c", "C")] {
            b.div_id(key, tw().sense_click().h_px(24.0), |b| b.text(tw(), name))
                .role(Role::Button)
                .named(name)
                .nav_item()
                .on_click(|_: &mut App| {});
            if key == "a" {
                b.button("inside", "Inside").on_click(|_: &mut App| {});
            }
        }
    })
    .nav(Nav::Column(app.wrap));
}

type H = Harness<'static, Rig<App>>;

fn harness(wrap: Wrap) -> H {
    let mut harness = support::harness(vec2(300.0, 300.0), base(), App { wrap }, view);
    harness.run_steps(4);
    harness
}

fn focused(h: &H) -> Option<String> {
    ["A", "B", "C", "Inside"]
        .into_iter()
        .find(|name| {
            h.get_by_role_and_label(Role::Button, name)
                .accesskit_node()
                .is_focused()
        })
        .map(str::to_owned)
}

fn press(h: &mut H, key: Key) {
    h.key_press(key);
    h.run_steps(4);
}

fn focus(h: &mut H, name: &str) {
    h.get_by_role_and_label(Role::Button, name).focus();
    h.run_steps(4);
    assert_eq!(focused(h).as_deref(), Some(name), "precondition");
}

#[test]
fn down_and_up_move_focus_between_the_items_and_pass_a_button_that_is_not_one() {
    let mut h = harness(Wrap::Around);
    focus(&mut h, "A");
    press(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("B"), "Inside is passed by");
    press(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("C"));
    press(&mut h, Key::ArrowUp);
    assert_eq!(focused(&h).as_deref(), Some("B"));
}

#[test]
fn home_and_end_go_to_the_ends() {
    let mut h = harness(Wrap::Around);
    focus(&mut h, "B");
    press(&mut h, Key::End);
    assert_eq!(focused(&h).as_deref(), Some("C"));
    press(&mut h, Key::Home);
    assert_eq!(focused(&h).as_deref(), Some("A"));
}

#[test]
fn the_ends_wrap_or_stop_as_asked() {
    let mut h = harness(Wrap::Around);
    focus(&mut h, "C");
    press(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("A"), "around");
    let mut h = harness(Wrap::Stop);
    focus(&mut h, "C");
    press(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("C"), "stop");
}

#[test]
fn every_item_stays_a_tab_stop() {
    let mut h = harness(Wrap::Around);
    focus(&mut h, "A");
    let mut visited = vec![focused(&h)];
    for _ in 0..3 {
        press(&mut h, Key::Tab);
        visited.push(focused(&h));
    }
    assert_eq!(
        visited,
        ["A", "Inside", "B", "C"].map(|name| Some(name.to_owned())),
        "Tab reaches each header, with no roving"
    );
}
