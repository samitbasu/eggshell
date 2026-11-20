//! A focused text field shows shadcn's focus: `border-ring` and the ring
//! around the box, as the owned controls do.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use eggshell_ui_shadcn::Shadcn;
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Theme, vec2};

use crate::support;

struct Name(String);

type H = Driver<Name>;

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
    let mut d = support::prepared_driver(
        vec2(320.0, 240.0),
        base,
        Name("Pedro".to_owned()),
        view,
        |ctx, _| {
            Shadcn::apply_both(ctx);
        },
    )
    .build();
    support::set_page(&d, support::Page::Themed);
    support::set_companion(&d, support::Companion::Alone);
    d.ctx().set_theme(theme);
    d.frames(4);
    d
}

fn focused(theme: Theme) -> H {
    let mut d = harness(theme);
    d.click(Target::suffix(FIELD));
    d.frames(6);
    d
}

#[test]
fn a_focused_field_is_ringed_and_bordered_in_the_ring_colour() {
    for theme in [Theme::Light, Theme::Dark] {
        let tokens = Shadcn::for_theme(theme).tokens;
        let d = focused(theme);
        let rect = d.rect(Target::suffix(FIELD));
        assert!(
            support::ring_at(&d, rect)
                .is_some_and(|ring| ring.stroke.color == tokens.ring.gamma_multiply(0.5)),
            "{theme:?}: ring/50 around the field"
        );
        let bordered = support::shapes(&d).into_iter().any(|shape| {
            matches!(shape, Shape::Rect(r)
                if rect.expand(0.5).contains_rect(r.rect) && r.stroke.color == tokens.ring)
        });
        assert!(bordered, "{theme:?}: the border is the ring colour");
    }
}

#[test]
fn a_field_that_is_not_focused_has_no_ring() {
    let d = harness(Theme::Light);
    let rect = d.rect(Target::suffix(FIELD));
    assert!(support::ring_at(&d, rect).is_none());
}
