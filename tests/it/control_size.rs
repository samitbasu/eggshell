//! Control sizes: a control draws at its own size, else the one its subtree
//! inherits, else the default (#127).

use eggshell_ui::contract::{Size, Style};
use eggshell_ui::owned::ButtonPart;
use eggshell_ui::sheet::When;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, vec2};

use crate::support;
use eggshell_ui::testing::{Driver, Target};

const SM: Size = Size("sm");
const XS: Size = Size("xs");

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

fn small_buttons(style: &mut Style) {
    style
        .sheet
        .as_app()
        .rule_when(ButtonPart::Root, When::always().size(SM), |t, _| {
            t.h_px(18.0)
        });
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.button("outside", "Outside").on_click(|_| {});
        s.div_id("dense", tw().flex().flex_col().control_size(SM), |s| {
            s.button("inherits", "Inherits").on_click(|_| {});
            s.button("explicit", "Explicit")
                .size(Size::DEFAULT)
                .on_click(|_| {});
            s.div_id("nested", tw().control_size(Size::DEFAULT), |s| {
                s.button("inner", "Inner").on_click(|_| {});
            });
        });
    });
}

fn height(d: &Driver<()>, path: &str) -> f32 {
    d.rect(Target::suffix(path)).height()
}

#[test]
fn a_control_takes_its_own_size_else_its_subtrees_else_the_default() {
    let mut d = support::driver(vec2(300.0, 300.0), base_text(), (), view)
        .style(support::shadcn_with(small_buttons))
        .build();
    d.frames(4);
    let default = height(&d, r#"/"Page"/"outside""#);
    let small = height(&d, r#"/"Page"/"dense"/"inherits""#);
    assert!(
        small < default,
        "{small} is the inherited size, {default} the default"
    );
    assert_eq!(
        height(&d, r#"/"Page"/"dense"/"explicit""#),
        default,
        "an explicit size beats the inherited one"
    );
    assert_eq!(
        height(&d, r#"/"Page"/"dense"/"nested"/"inner""#),
        default,
        "a nearer scope beats a farther one"
    );
}

/// What a gallery is built with: the size a subtree inherits, and the one
/// every control asks for itself.
#[derive(Clone, Copy)]
struct Gallery {
    inherited: Size,
    explicit: Option<Size>,
}

fn controls(s: &mut Builder<'_, '_, Gallery>, explicit: Option<Size>) {
    macro_rules! sized {
        ($chain:expr) => {
            match explicit {
                Some(size) => $chain.size(size),
                None => $chain,
            }
        };
    }
    sized!(s.button("button", "Button")).on_click(|_| {});
    sized!(s.checkbox("checkbox", "Check", true)).on_change(|_, _| {});
    sized!(s.radio_group("radio", 0, [(0, "One"), (1, "Two")])).on_change(|_, _| {});
    sized!(s.switch("switch", "Switch", true)).on_change(|_, _| {});
    sized!(s.segmented("segmented", 0, [(0, "A"), (1, "B")])).on_change(|_, _| {});
    sized!(s.slider("slider", 0.5, 0.0..=1.0).named("Slider")).on_change(|_, _| {});
    sized!(
        s.select("select", 0, [(0, "One"), (1, "Two")])
            .named("Select")
    )
    .on_change(|_, _| {});
    sized!(s.stepper("stepper", 1.0, 0.0..=9.0).named("Stepper")).on_change(|_, _| {});
    sized!(s.text_field("field", "text").named("Field")).on_change(|_, _| {});
}

fn gallery(b: &mut Builder<'_, '_, Gallery>, app: &Gallery) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.div_id(
            "scope",
            tw().flex().flex_col().gap(2.0).control_size(app.inherited),
            |s| controls(s, app.explicit),
        );
    });
}

/// The height of each control's main box when built at `app`.
fn heights(app: Gallery) -> Vec<(&'static str, f32)> {
    let mut d =
        support::prepared_driver(vec2(400.0, 600.0), base_text(), app, gallery, |_, _| {}).build();
    d.frames(4);
    [
        "button",
        "checkbox/Box",
        "radio/Circle(0)",
        "switch/Track",
        "segmented",
        "slider/Track/Thumb",
        "select",
        "stepper/Box",
        "field",
    ]
    .map(|control| {
        let path = control
            .split('/')
            .fold(String::from(r#"/"Page"/"scope""#), |path, part| {
                if part.contains('(') {
                    format!("{path}/{part}")
                } else {
                    format!(r#"{path}/"{part}""#)
                }
            });
        let path = path
            .replace(r#""Box""#, "Box")
            .replace(r#""Track""#, "Track")
            .replace(r#""Thumb""#, "Thumb");
        (control, d.rect(Target::suffix(&path)).height())
    })
    .to_vec()
}

#[test]
fn every_control_is_smaller_at_sm_and_smaller_again_at_xs() {
    let at = |size| {
        heights(Gallery {
            inherited: size,
            explicit: None,
        })
    };
    let (default, sm, xs) = (at(Size::DEFAULT), at(SM), at(XS));
    for ((control, d), ((_, s), (_, x))) in default.iter().zip(sm.iter().zip(&xs)) {
        assert!(d > s && s > x, "{control}: default {d}, sm {s}, xs {x}");
    }
}

#[test]
fn a_control_that_asks_for_a_size_ignores_the_one_its_subtree_inherits() {
    let own = heights(Gallery {
        inherited: XS,
        explicit: Some(SM),
    });
    let sm = heights(Gallery {
        inherited: SM,
        explicit: None,
    });
    assert_eq!(own, sm);
}
