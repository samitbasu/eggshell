//! The options of the choice widgets: every form `IntoChoice` accepts, and an
//! option that is disabled on its own, through kittest's real pointer and key
//! events and the accessibility check the support harness switches on.

use std::fmt;

use bullseye::owned::{Choice, Enablement};
use bullseye::{Builder, Icon, TextStyle, tw};
use bullseye_shadcn::Tokens;
use egui::{Key, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path};

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

fn harness(view: support::View<App>) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(12);
    harness
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(4);
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..16 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        press(harness, Key::Tab);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(harness));
}

#[test]
fn every_form_of_option_builds_a_working_control() {
    let mut harness = harness(forms);
    for form in 0..FORMS {
        for label in ["Alpha", "Beta", "Gamma"] {
            assert!(
                harness
                    .query_all_by_role_and_label(Role::RadioButton, label)
                    .count()
                    >= FORMS,
                "{label} is a cell of every control"
            );
        }
        click_node(&mut harness, &format!("/\"f{form}\"/C"));
        harness.run_steps(3);
        assert_eq!(harness.state().app.forms[form], Dir::C, "form {form}");
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
    let mut harness = harness(view);
    click_node(&mut harness, "/\"r\"/Circle(B)");
    assert_eq!(harness.state().app.radio, Dir::B);
    assert!(
        harness
            .query_by_role_and_label(Role::RadioButton, "Gamma")
            .is_some()
    );
    click_node(&mut harness, "/\"p\"");
    harness.run_steps(4);
    let gamma = harness
        .get_by_role_and_label(Role::ListBoxOption, "Gamma")
        .rect();
    support::hover_over(&mut harness, gamma.center());
    support::press_and_release(&mut harness, gamma.center());
    harness.run_steps(4);
    assert_eq!(harness.state().app.select, Dir::C);
}

#[test]
fn a_disabled_option_is_announced_disabled_and_the_others_are_not() {
    let mut harness = harness(disabled_option);
    let radios = |harness: &Harness<'_, Rig<App>>, label: &str| {
        harness
            .get_all_by_role_and_label(Role::RadioButton, label)
            .map(|node| node.accesskit_node().is_disabled())
            .collect::<Vec<_>>()
    };
    for (label, disabled) in [("Alpha", false), ("Beta", true), ("Gamma", false)] {
        assert_eq!(
            radios(&harness, label),
            [disabled, disabled],
            "radio group and segmented cell: {label}"
        );
    }
    click_node(&mut harness, "/\"p\"");
    harness.run_steps(4);
    for (label, disabled) in [("Alpha", false), ("Beta", true), ("Gamma", false)] {
        assert_eq!(
            harness
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
    let mut harness = harness(disabled_option);
    click_node(&mut harness, "/\"r\"/Circle(B)");
    click_node(&mut harness, "/\"r\"/Label(B)");
    click_node(&mut harness, "/\"s\"/B");
    harness.run_steps(3);
    assert_eq!(harness.state().app.radio, Dir::A);
    assert_eq!(harness.state().app.segmented, Dir::A);

    click_node(&mut harness, "/\"p\"");
    harness.run_steps(4);
    let beta = harness
        .get_by_role_and_label(Role::ListBoxOption, "Beta")
        .rect();
    support::hover_over(&mut harness, beta.center());
    support::press_and_release(&mut harness, beta.center());
    harness.run_steps(4);
    assert_eq!(harness.state().app.select, Dir::A);
}

#[test]
fn the_arrow_keys_skip_a_disabled_radio_option_and_tab_skips_a_disabled_cell() {
    let mut harness = harness(disabled_option);
    tab_to(&mut harness, "/\"r\"/Circle(A)");
    press(&mut harness, Key::ArrowDown);
    assert_eq!(harness.state().app.radio, Dir::C, "Beta is skipped");
    assert_eq!(focused_path(&harness).as_deref(), Some("/\"r\"/Circle(C)"));
    press(&mut harness, Key::ArrowDown);
    assert_eq!(harness.state().app.radio, Dir::A, "and the group wraps");
    press(&mut harness, Key::ArrowUp);
    assert_eq!(harness.state().app.radio, Dir::C, "backwards too");

    let mut visited = Vec::new();
    for _ in 0..8 {
        press(&mut harness, Key::Tab);
        visited.extend(focused_path(&harness));
    }
    assert!(
        visited.iter().any(|path| path == "/\"s\"/A"),
        "the segmented control is reached: {visited:?}"
    );
    assert!(visited.iter().all(|path| path != "/\"s\"/B"), "{visited:?}");
}

#[test]
fn the_arrow_keys_skip_a_disabled_select_option() {
    let mut harness = harness(disabled_option);
    tab_to(&mut harness, "/\"p\"");
    press(&mut harness, Key::Space);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().app.select, Dir::C, "Beta is skipped");

    press(&mut harness, Key::Space);
    press(&mut harness, Key::Home);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::Enter);
    assert_eq!(
        harness.state().app.select,
        Dir::C,
        "and it is not passed on"
    );
}

#[test]
fn the_select_trigger_shows_the_label_of_the_current_value() {
    let mut harness = harness(disabled_option);
    let shown = |harness: &Harness<'_, Rig<App>>| {
        harness
            .get_by_role_and_label(Role::ComboBox, "Pick")
            .accesskit_node()
            .value()
            .map(|value| value.to_string())
    };
    assert_eq!(shown(&harness).as_deref(), Some("Alpha"));
    tab_to(&mut harness, "/\"p\"");
    press(&mut harness, Key::Space);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::Enter);
    assert_eq!(shown(&harness).as_deref(), Some("Gamma"));
    assert!(
        harness
            .query_by_role_and_label(Role::ComboBox, "Pick")
            .is_some()
    );
}
