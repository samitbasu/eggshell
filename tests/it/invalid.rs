//! A control marked invalid takes shadcn's `aria-invalid` look, and an input
//! group around it turns too, because its child is invalid. A disabled or
//! refused control keeps its disabled look; assistive technology hears it is
//! invalid either way.

use eggshell_ui::owned::Enablement;
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Icon, TextStyle, icons, tw};
use egui::accesskit::Invalid;
use egui::{Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, box_at, ring_at};

const ERROR: Option<&str> = Some("Required");

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(4.0).p(4.0).w_px(300.0),
        |s| {
            s.text_field("plain", "")
                .named("Plain")
                .on_change(|_, _| {});
            s.text_field("bad", "")
                .named("Bad")
                .invalid(ERROR)
                .on_change(|_, _| {});
            s.text_field("off", "")
                .named("Off")
                .invalid(ERROR)
                .enablement(Enablement::Disabled)
                .on_change(|_, _| {});
            s.text_field("search", "")
                .named("Search")
                .leading(Icon::new(icons::SEARCH))
                .on_change(|_, _| {});
            s.text_field("bad_search", "")
                .named("Bad search")
                .leading(Icon::new(icons::SEARCH))
                .invalid(ERROR)
                .on_change(|_, _| {});
            s.select("locked", 0, [(0, "Viewer"), (1, "Editor")])
                .named("Locked")
                .refused("Ask an admin")
                .invalid(ERROR)
                .on_change(|_, _| {});
            for (key, error) in [("ok", None), ("bad", ERROR)] {
                let container = if error.is_some() { "invalid" } else { "valid" };
                s.div_id(container, tw().flex().flex_col().gap(2.0), |s| {
                    s.stepper("age", 3.0, 0.0..=9.0)
                        .named(format!("{key} age"))
                        .invalid(error)
                        .on_change(|_, _| {});
                    s.select("role", 0, [(0, "Viewer"), (1, "Editor")])
                        .named(format!("{key} role"))
                        .invalid(error)
                        .on_change(|_, _| {});
                    s.checkbox("terms", format!("{key} terms"), false)
                        .invalid(error)
                        .on_change(|_, _| {});
                    s.radio_group(
                        "plan",
                        0,
                        [(0, format!("{key} free")), (1, format!("{key} pro"))],
                    )
                    .named(format!("{key} plan"))
                    .invalid(error)
                    .on_change(|_, _| {});
                    s.switch("news", format!("{key} news"), false)
                        .invalid(error)
                        .on_change(|_, _| {});
                    s.combobox("fruit", None::<u8>, [(0u8, "Apple".to_owned())])
                        .named(format!("{key} fruit"))
                        .invalid(error)
                        .on_change(|_, _| {});
                });
            }
        },
    );
}

fn harness(theme: Theme) -> Driver<()> {
    let tokens = Tokens::for_theme(theme);
    let mut d = support::driver(
        vec2(400.0, 1000.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        (),
        view,
    )
    .build();
    d.ctx().set_theme(theme);
    d.frames(4);
    d
}

fn border(d: &Driver<()>, path: &str) -> egui::Color32 {
    box_at(d, d.rect(Target::suffix(path))).stroke.color
}

fn invalid_said(d: &Driver<()>, name: &str) -> (Option<Invalid>, Option<String>) {
    let node = d.harness().get_by_label(name);
    let node = node.accesskit_node();
    (
        node.data().invalid(),
        node.description().map(|d| d.to_string()),
    )
}

#[test]
fn an_invalid_text_field_has_a_destructive_border_and_ring() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let tokens = Tokens::for_theme(theme);
        assert_ne!(
            border(&d, r#"/"plain""#),
            tokens.destructive,
            "precondition"
        );
        // style-vega.css:678: `aria-invalid:border-destructive
        // dark:aria-invalid:border-destructive/50`.
        let destructive = match theme {
            Theme::Light => tokens.destructive,
            Theme::Dark => tokens.destructive.gamma_multiply(0.5),
        };
        assert_eq!(border(&d, r#"/"bad""#), destructive, "{theme:?}");
        let ring = ring_at(&d, d.rect(Target::suffix(r#"/"bad""#))).expect("a ring at rest");
        // `aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40`.
        let halo = match theme {
            Theme::Light => tokens.destructive.gamma_multiply(0.2),
            Theme::Dark => tokens.destructive.gamma_multiply(0.4),
        };
        assert_eq!(ring.stroke.color, halo, "{theme:?}");
        assert!(ring_at(&d, d.rect(Target::suffix(r#"/"plain""#))).is_none());
    }
}

#[test]
fn an_input_group_turns_destructive_because_its_child_is_invalid() {
    for theme in [Theme::Light, Theme::Dark] {
        let d = harness(theme);
        let tokens = Tokens::for_theme(theme);
        assert_ne!(
            border(&d, r#"/"search""#),
            tokens.destructive,
            "precondition"
        );
        assert_eq!(
            border(&d, r#"/"bad_search""#),
            tokens.destructive,
            "{theme:?}"
        );
    }
}

#[test]
fn a_disabled_invalid_field_keeps_its_disabled_look_but_is_reported_invalid() {
    let d = harness(Theme::Light);
    let tokens = Tokens::for_theme(Theme::Light);
    assert_ne!(border(&d, r#"/"off""#), tokens.destructive);
    assert!(ring_at(&d, d.rect(Target::suffix(r#"/"off""#))).is_none());
    assert_eq!(
        invalid_said(&d, "Off"),
        (Some(Invalid::True), Some("Required".into()))
    );
}

#[test]
fn a_refused_invalid_control_keeps_its_disabled_look_and_says_both_why() {
    let d = harness(Theme::Light);
    let tokens = Tokens::for_theme(Theme::Light);
    assert_ne!(border(&d, r#"/"locked""#), tokens.destructive);
    assert_eq!(
        invalid_said(&d, "Locked"),
        (Some(Invalid::True), Some("Required. Ask an admin".into()))
    );
}

#[test]
fn assistive_technology_hears_invalid_and_the_error() {
    let d = harness(Theme::Light);
    assert_eq!(
        invalid_said(&d, "Bad"),
        (Some(Invalid::True), Some("Required".into()))
    );
    assert_eq!(invalid_said(&d, "Bad search").0, Some(Invalid::True));
    assert_eq!(invalid_said(&d, "Plain"), (None, None));
}

/// The look boxes of the controls in the container `container`.
fn looks(container: &str) -> [String; 6] {
    [
        format!("/\"{container}\"/\"age\"/Box"),
        format!("/\"{container}\"/\"role\""),
        format!("/\"{container}\"/\"terms\"/Box"),
        format!("/\"{container}\"/\"plan\"/Circle(0)"),
        format!("/\"{container}\"/\"news\"/Track"),
        // The combobox keys its group by the id of its key.
        format!("\"{container}\")).with((\"&str\", \"fruit\")), Group)"),
    ]
}

#[test]
fn every_control_takes_the_invalid_border_and_only_when_invalid() {
    let d = harness(Theme::Light);
    let tokens = Tokens::for_theme(Theme::Light);
    for (valid, invalid) in looks("valid").iter().zip(looks("invalid").iter()) {
        assert_ne!(border(&d, valid), tokens.destructive, "{valid}");
        assert_eq!(border(&d, invalid), tokens.destructive, "{invalid}");
    }
}

#[test]
fn every_control_tells_assistive_technology_it_is_invalid() {
    let d = harness(Theme::Light);
    for name in ["age", "role", "terms", "free", "news", "fruit"] {
        assert_eq!(
            invalid_said(&d, &format!("ok {name}")),
            (None, None),
            "{name}"
        );
        assert_eq!(
            invalid_said(&d, &format!("bad {name}")),
            (Some(Invalid::True), Some("Required".into())),
            "{name}"
        );
    }
}

#[test]
fn a_choices_field_label_turns_destructive_when_it_is_invalid() {
    let d = harness(Theme::Light);
    let tokens = Tokens::for_theme(Theme::Light);
    for (key, label) in [("terms", "terms"), ("news", "news")] {
        let ok = d.rect(Target::suffix(&format!("/\"valid\"/\"{key}\"/Label")));
        let bad = d.rect(Target::suffix(&format!("/\"invalid\"/\"{key}\"/Label")));
        assert_ne!(
            support::text_color(&d, ok, &format!("ok {label}")),
            tokens.destructive
        );
        assert_eq!(
            support::text_color(&d, bad, &format!("bad {label}")),
            tokens.destructive
        );
    }
}

fn hinted(b: &mut Builder<'_, '_, ()>, _: &()) {
    use eggshell_ui::owned::field;
    use eggshell_ui::sheet::Key;
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(4.0).p(4.0).w_px(300.0),
        |s| {
            s.text_field("plain", "")
                .named("Plain")
                .hint("Ada")
                .on_change(|_, _| {});
            let field = s.part(field::Part::Root, &[Key::Attr("invalid", "true")], tw());
            s.div_id("field", field, |s| {
                s.text_field("bad", "")
                    .named("Bad")
                    .hint("Lovelace")
                    .invalid(ERROR)
                    .on_change(|_, _| {});
            });
        },
    );
}

/// input.tsx's `placeholder:text-muted-foreground`, in a valid field and in
/// an invalid one inside a `data-invalid` Field, whose text is destructive.
#[test]
fn a_hint_is_muted_even_in_an_invalid_field() {
    for theme in [Theme::Light, Theme::Dark] {
        let tokens = Tokens::for_theme(theme);
        let mut d = support::driver(
            vec2(400.0, 300.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            (),
            hinted,
        )
        .build();
        d.ctx().set_theme(theme);
        d.frames(4);
        let plain = d.rect(Target::suffix(r#"/"plain""#));
        let bad = d.rect(Target::suffix(r#"/"bad""#));
        assert_eq!(
            support::text_color(&d, plain, "Ada"),
            tokens.muted_foreground,
            "{theme:?}"
        );
        assert_eq!(
            support::text_color(&d, bad, "Lovelace"),
            tokens.muted_foreground,
            "{theme:?}: not the Field's destructive"
        );
    }
}
