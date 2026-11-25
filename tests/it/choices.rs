//! The options of the choice widgets: every form `IntoChoice` accepts, and an
//! option that is disabled on its own, through kittest's real pointer and key
//! events and the accessibility check the support harness switches on.

use std::fmt;

use eggshell_ui::owned::{Enablement, choice::Choice};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, Icon, TextStyle, tw};
use egui::{Key, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Dir {
    #[default]
    A,
    B,
    C,
}

impl Dir {
    const VARIANTS: &[Self] = &[Self::A, Self::B, Self::C];
}

impl fmt::Display for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::A => "Alpha",
            Self::B => "Beta",
            Self::C => "Gamma",
        })
    }
}

const FORMS: usize = 5;

#[derive(Default)]
struct App {
    radio: Dir,
    segmented: Dir,
    select: Dir,
    forms: [Dir; FORMS],
}

fn icon() -> Icon {
    Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
        egui::TextureId::default(),
        vec2(4.0, 4.0),
    )))
}

/// Beta is disabled on its own in each of the three widgets.
fn options() -> [Choice<Dir>; 3] {
    [
        Choice::new(Dir::A, "Alpha"),
        Choice::new(Dir::B, "Beta").enablement(Enablement::Disabled),
        Choice::new(Dir::C, "Gamma"),
    ]
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <div role="radiogroup">Alpha <del>Beta</del> Gamma</div>
///   <div class="segmented">Alpha <del>Beta</del> Gamma</div>
///   <button role="combobox" aria-label="Pick">Alpha</button>
/// </div>
/// ```
fn disabled_option(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.radio_group("r", app.radio, options())
            .on_change(|app: &mut App, dir| app.radio = dir);
        b.segmented("s", app.segmented, options())
            .on_change(|app: &mut App, dir| app.segmented = dir);
        b.select("p", app.select, options())
            .named("Pick")
            .on_change(|app: &mut App, dir| app.select = dir);
    });
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <div class="segmented">Alpha Beta Gamma</div> <!-- five times -->
/// </div>
/// ```
fn forms(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.segmented("f0", app.forms[0], Dir::VARIANTS)
            .on_change(|app: &mut App, dir| app.forms[0] = dir);
        b.segmented("f1", app.forms[1], [Dir::A, Dir::B, Dir::C])
            .on_change(|app: &mut App, dir| app.forms[1] = dir);
        b.segmented(
            "f2",
            app.forms[2],
            [(Dir::A, "Alpha"), (Dir::B, "Beta"), (Dir::C, "Gamma")],
        )
        .on_change(|app: &mut App, dir| app.forms[2] = dir);
        b.segmented(
            "f3",
            app.forms[3],
            [
                (Dir::A, icon(), "Alpha"),
                (Dir::B, icon(), "Beta"),
                (Dir::C, icon(), "Gamma"),
            ],
        )
        .on_change(|app: &mut App, dir| app.forms[3] = dir);
        b.segmented(
            "f4",
            app.forms[4],
            Dir::VARIANTS
                .iter()
                .map(|&dir| Choice::new(dir, dir.to_string())),
        )
        .on_change(|app: &mut App, dir| app.forms[4] = dir);
    });
}

fn harness(view: support::View<App>) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(12);
    d
}

fn press(d: &mut Driver<App>, key: Key) {
    d.press(key).frames(3);
}

fn tab_to(d: &mut Driver<App>, path: &str) {
    for _ in 0..16 {
        if focused_path(d).as_deref() == Some(path) {
            return;
        }
        press(d, Key::Tab);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(d));
}

#[test]
fn every_form_of_option_builds_a_working_control() {
    let mut d = harness(forms);
    for form in 0..FORMS {
        for label in ["Alpha", "Beta", "Gamma"] {
            assert!(
                d.harness()
                    .query_all_by_role_and_label(Role::RadioButton, label)
                    .count()
                    >= FORMS,
                "{label} is a cell of every control"
            );
        }
        d.click(Target::suffix(&format!("/\"f{form}\"/2")));
        d.frames(3);
        assert_eq!(d.app().forms[form], Dir::C, "form {form}");
    }
}

#[test]
fn a_display_value_and_a_reference_to_one_label_a_radio_group_and_a_select() {
    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.radio_group("r", app.radio, Dir::VARIANTS)
            .on_change(|app: &mut App, dir| app.radio = dir);
        b.select("p", app.select, [Dir::A, Dir::B, Dir::C])
            .named("Pick")
            .on_change(|app: &mut App, dir| app.select = dir);
    }
    let mut d = harness(view);
    d.click(Target::suffix("/\"r\"/Circle(1)"));
    assert_eq!(d.app().radio, Dir::B);
    assert!(
        d.harness()
            .query_by_role_and_label(Role::RadioButton, "Gamma")
            .is_some()
    );
    d.click(Target::suffix("/\"p\""));
    d.frames(4);
    let gamma = d
        .harness()
        .get_by_role_and_label(Role::ListBoxOption, "Gamma")
        .rect();
    d.click(gamma.center());
    d.frames(4);
    assert_eq!(d.app().select, Dir::C);
}

#[test]
fn a_disabled_option_is_announced_disabled_and_the_others_are_not() {
    let mut d = harness(disabled_option);
    let radios = |d: &Driver<App>, label: &str| {
        d.harness()
            .get_all_by_role_and_label(Role::RadioButton, label)
            .map(|node| node.accesskit_node().is_disabled())
            .collect::<Vec<_>>()
    };
    for (label, disabled) in [("Alpha", false), ("Beta", true), ("Gamma", false)] {
        assert_eq!(
            radios(&d, label),
            [disabled, disabled],
            "radio group and segmented cell: {label}"
        );
    }
    d.click(Target::suffix("/\"p\""));
    d.frames(4);
    for (label, disabled) in [("Alpha", false), ("Beta", true), ("Gamma", false)] {
        assert_eq!(
            d.harness()
                .get_by_role_and_label(Role::ListBoxOption, label)
                .accesskit_node()
                .is_disabled(),
            disabled,
            "select option: {label}"
        );
    }
}

#[test]
fn a_disabled_option_cannot_be_chosen_with_the_pointer() {
    let mut d = harness(disabled_option);
    d.click(Target::suffix("/\"r\"/Circle(1)"));
    d.click(Target::suffix("/\"r\"/Label(1)"));
    d.click(Target::suffix("/\"s\"/1"));
    d.frames(3);
    assert_eq!(d.app().radio, Dir::A);
    assert_eq!(d.app().segmented, Dir::A);

    d.click(Target::suffix("/\"p\""));
    d.frames(4);
    let beta = d
        .harness()
        .get_by_role_and_label(Role::ListBoxOption, "Beta")
        .rect();
    d.click(beta.center());
    d.frames(4);
    assert_eq!(d.app().select, Dir::A);
}

#[test]
fn the_arrow_keys_skip_a_disabled_radio_option_and_tab_skips_a_disabled_cell() {
    let mut d = harness(disabled_option);
    tab_to(&mut d, "/\"r\"/Circle(0)");
    press(&mut d, Key::ArrowDown);
    assert_eq!(d.app().radio, Dir::C, "Beta is skipped");
    assert_eq!(focused_path(&d).as_deref(), Some("/\"r\"/Circle(2)"));
    press(&mut d, Key::ArrowDown);
    assert_eq!(d.app().radio, Dir::A, "and the group wraps");
    press(&mut d, Key::ArrowUp);
    assert_eq!(d.app().radio, Dir::C, "backwards too");

    let mut visited = Vec::new();
    for _ in 0..8 {
        press(&mut d, Key::Tab);
        visited.extend(focused_path(&d));
    }
    assert!(
        visited.iter().any(|path| path == "/\"s\"/0"),
        "the segmented control is reached: {visited:?}"
    );
    assert!(visited.iter().all(|path| path != "/\"s\"/1"), "{visited:?}");
}

#[test]
fn the_arrow_keys_skip_a_disabled_select_option() {
    let mut d = harness(disabled_option);
    tab_to(&mut d, "/\"p\"");
    press(&mut d, Key::Space);
    press(&mut d, Key::ArrowDown);
    press(&mut d, Key::Enter);
    assert_eq!(d.app().select, Dir::C, "Beta is skipped");

    press(&mut d, Key::Space);
    press(&mut d, Key::Home);
    press(&mut d, Key::ArrowDown);
    press(&mut d, Key::ArrowDown);
    press(&mut d, Key::Enter);
    assert_eq!(d.app().select, Dir::C, "and it is not passed on");
}

#[test]
fn the_select_trigger_shows_the_label_of_the_current_value() {
    let mut d = harness(disabled_option);
    let shown = |d: &Driver<App>| {
        d.harness()
            .get_by_role_and_label(Role::ComboBox, "Pick")
            .accesskit_node()
            .value()
            .map(|value| value.to_string())
    };
    assert_eq!(shown(&d).as_deref(), Some("Alpha"));
    tab_to(&mut d, "/\"p\"");
    press(&mut d, Key::Space);
    press(&mut d, Key::ArrowDown);
    press(&mut d, Key::Enter);
    assert_eq!(shown(&d).as_deref(), Some("Gamma"));
    assert!(
        d.harness()
            .query_by_role_and_label(Role::ComboBox, "Pick")
            .is_some()
    );
}
