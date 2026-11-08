//! The System / Light / Dark toggle every example shows, through the real
//! path: the strip the examples' loops share, clicked with the pointer.

#![allow(
    clippy::duplicate_mod,
    reason = "each test crate includes the shared examples/common module"
)]

use std::fs;
use std::path::{Path, PathBuf};

use egui::ThemePreference;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

#[path = "../examples/common/mod.rs"]
mod common;

fn preference(harness: &Harness<'_, common::ThemeBar>) -> ThemePreference {
    harness.ctx.options(|options| options.theme_preference)
}

fn bar() -> Harness<'static, common::ThemeBar> {
    Harness::builder()
        .with_size(egui::vec2(600.0, 200.0))
        .build_ui_state(
            |ui, bar: &mut common::ThemeBar| bar.show(ui),
            common::ThemeBar::new(),
        )
}

#[test]
fn the_theme_follows_the_system_until_the_toggle_says_otherwise() {
    assert_eq!(
        egui::Context::default().options(|options| options.theme_preference),
        ThemePreference::System,
        "an egui context starts out following the system"
    );
    let mut harness = bar();
    harness.ctx.set_theme(ThemePreference::System);
    harness.run();
    assert!(
        harness.get_by_label("System").accesskit_node().toggled()
            == Some(egui::accesskit::Toggled::True),
        "System is the pressed cell"
    );

    for (label, expected, theme) in [
        ("Dark", ThemePreference::Dark, Some(egui::Theme::Dark)),
        ("Light", ThemePreference::Light, Some(egui::Theme::Light)),
        ("System", ThemePreference::System, None),
    ] {
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(preference(&harness), expected, "{label}");
        if let Some(theme) = theme {
            assert_eq!(harness.ctx.theme(), theme, "{label} shows its theme");
        }
        assert_eq!(
            harness.get_by_label(label).accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True),
            "{label} is now the pressed cell"
        );
    }
}

/// Each example's source: a file, or all of a directory's files.
fn sources(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.ends_with("common") {
                continue;
            }
            let text = fs::read_dir(&path)
                .unwrap()
                .map(|file| fs::read_to_string(file.unwrap().path()).unwrap())
                .collect::<String>();
            found.push((path, text));
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap();
            found.push((path, text));
        }
    }
    found
}

#[test]
fn every_example_shows_the_toggle_and_none_pins_a_theme() {
    let examples = sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"));
    assert!(
        examples.len() >= 15,
        "precondition: {} examples",
        examples.len()
    );
    for (file, text) in examples {
        let shared_runner = text.contains("common::run(")
            || text.contains("common::run_app(")
            || text.contains("common::run_app_styled(");
        let own_bar = text.contains("ThemeBar::new()") || text.contains("common::theme_toggle(");
        assert!(
            shared_runner || own_bar,
            "{} has no theme toggle",
            file.display()
        );
        assert!(
            !text.contains("set_theme("),
            "{} pins the theme instead of following the system",
            file.display()
        );
    }
}

#[test]
fn the_shared_runner_shows_the_bar() {
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/common/mod.rs"))
            .unwrap();
    let from = text.find("fn shell<").unwrap();
    let shell = &text[from..from + text[from..].find("\n}\n").unwrap()];
    assert!(shell.contains("theme_bar.show(ui)"));
    assert!(
        !shell.contains("set_theme("),
        "the runner does not pin Light"
    );
}
