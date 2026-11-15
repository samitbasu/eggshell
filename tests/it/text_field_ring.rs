//! A focused text field shows shadcn's focus: `border-ring` and the ring
//! around the box, as the owned controls do.

use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use eggshell_ui_shadcn::Shadcn;
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig};

struct Name(String);

type H = Harness<'static, Rig<Name>>;

const FIELD: &str = "/\"name\"";

fn view(b: &mut Builder<'_, '_, Name>, name: &Name) {
    b.div(tw().p(16.0).w_px(240.0), |b| {
        b.text_field("name", name.0.clone())
            .named("Name")
            .on_change(|name: &mut Name, new| name.0 = new);
    });
}

fn harness(theme: Theme) -> H {
    let base = TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    );
    let mut h = support::harness_prepared(
        vec2(320.0, 240.0),
        base,
        Name("Pedro".to_owned()),
        view,
        |ctx, _| {
            Shadcn::apply_both(ctx);
        },
    );
    h.state_mut().page = support::Page::Themed;
    h.state_mut().companion = support::Companion::Alone;
    h.ctx.set_theme(theme);
    h.run_steps(4);
    h
}

fn focused(theme: Theme) -> H {
    let mut h = harness(theme);
    support::click_node(&mut h, FIELD);
    h.run_steps(6);
    h
}

#[test]
fn a_focused_field_is_ringed_and_bordered_in_the_ring_colour() {
    for theme in [Theme::Light, Theme::Dark] {
        let tokens = Shadcn::for_theme(theme).tokens;
        let h = focused(theme);
        let rect = support::rect_of(&h, FIELD);
        assert!(
            support::ring_at(&h, rect)
                .is_some_and(|ring| ring.stroke.color == tokens.ring.gamma_multiply(0.5)),
            "{theme:?}: ring/50 around the field"
        );
        let bordered = support::shapes(&h).into_iter().any(|shape| {
            matches!(shape, Shape::Rect(r)
                if rect.expand(0.5).contains_rect(r.rect) && r.stroke.color == tokens.ring)
        });
        assert!(bordered, "{theme:?}: the border is the ring colour");
    }
}

#[test]
fn a_field_that_is_not_focused_has_no_ring() {
    let h = harness(Theme::Light);
    let rect = support::rect_of(&h, FIELD);
    assert!(support::ring_at(&h, rect).is_none());
}
