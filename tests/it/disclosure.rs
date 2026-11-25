//! The extras' section header and disclosure button through the real path:
//! build, layout, draw, pointer and key events, and the accessibility tree.

use eggshell_ui::overlay::Shown;
use eggshell_ui::owned::Enablement;
use eggshell_ui::owned::disclosure::{DisclosureExt, SectionExt};
use eggshell_ui::{Builder, Problem, tw};
use egui::accesskit::Role as AccessRole;
use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, rect_of};
use eggshell_ui::testing::{Animation, Target};

#[path = "../../examples/common/mod.rs"]
mod common;

const TITLE: &str = "Environment";
const HEADER: &str = r#"/"env"/Header"#;
const REGION: &str = r#"/"env"/Region"#;

struct App {
    env: Shown,
    enablement: Enablement,
    changes: Vec<Shown>,
    pressed: u32,
    fold: Shown,
    folds: Vec<Shown>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            env: Shown::Closed,
            enablement: Enablement::Enabled,
            changes: Vec::new(),
            pressed: 0,
            fold: Shown::Closed,
            folds: Vec::new(),
        }
    }
}

/// ```html
/// <div class="flex w-[300px] flex-col">
///   <section> <button aria-expanded aria-controls>Environment ⌄</button>
///     <div role="region"> <p>Branch main</p> <button>Inside</button> </div> </section>
///   <div class="flex"> <button aria-expanded>›</button> src </div>
/// </div>
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div(tw().flex().flex_col().w_px(300.0), |b| {
        b.section("env", TITLE, app.env)
            .enablement(app.enablement)
            .on_change(|app: &mut App, shown| {
                app.changes.push(shown);
                app.env = shown;
            })
            .children(|b| {
                b.text(tw(), "Branch main");
                b.button("inside", "Inside")
                    .on_click(|app: &mut App| app.pressed += 1);
            });
        b.div(tw().flex().items_center().gap(2.0), |b| {
            b.disclosure("fold", app.fold)
                .named("src")
                .on_change(|app: &mut App, shown| {
                    app.folds.push(shown);
                    app.fold = shown;
                });
            b.text(tw(), "src");
        });
    });
}

fn harness_of(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness_stepped(
        vec2(400.0, 400.0),
        0.01,
        common::base_text(egui::Theme::Light),
        app,
        view,
    );
    harness.run_steps(4);
    harness
}

fn harness() -> Harness<'static, Rig<App>> {
    harness_of(App::default())
}

fn header<'h>(harness: &'h Harness<'_, Rig<App>>) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(AccessRole::Button, TITLE)
}

fn expanded(harness: &Harness<'_, Rig<App>>) -> Option<bool> {
    header(harness).accesskit_node().data().is_expanded()
}

fn controlled(harness: &Harness<'_, Rig<App>>) -> Vec<(AccessRole, Option<String>)> {
    header(harness)
        .accesskit_node()
        .controls()
        .map(|node| (node.role(), node.label()))
        .collect()
}

fn region_height(harness: &Harness<'_, Rig<App>>) -> f32 {
    rect_of(harness, REGION).height()
}

/// The turn of the chevron painted in the header, in degrees from its
/// closed direction.
fn chevron_degrees(harness: &Harness<'_, Rig<App>>) -> f32 {
    let head = rect_of(harness, HEADER);
    let icons: Vec<_> = support::textured(harness)
        .into_iter()
        .filter(|mesh| head.contains(mesh.calc_bounds().center()))
        .collect();
    assert_eq!(icons.len(), 1, "the header draws one chevron");
    support::turn(icons[0])
}

#[test]
fn a_click_on_the_header_asks_to_open_and_then_to_close() {
    let mut harness = harness();
    assert_eq!(region_height(&harness), 0.0, "precondition: closed");
    header(&harness).click();
    harness.run_steps(40);
    assert_eq!(harness.state().app.changes, vec![Shown::Open]);
    assert!(region_height(&harness) > 20.0, "it opened");
    header(&harness).click();
    harness.run_steps(40);
    assert_eq!(
        harness.state().app.changes,
        vec![Shown::Open, Shown::Closed]
    );
    assert_eq!(region_height(&harness), 0.0, "it closed");
}

#[test]
fn enter_and_space_on_the_focused_header_toggle_it() {
    let mut harness = harness();
    support::past_bar(&mut harness);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(
        support::focused_path(&harness).as_deref(),
        Some(HEADER),
        "precondition: the header is the first Tab stop"
    );
    harness.key_press(Key::Enter);
    harness.run_steps(40);
    harness.key_press(Key::Space);
    harness.run_steps(40);
    assert_eq!(
        harness.state().app.changes,
        vec![Shown::Open, Shown::Closed]
    );
}

#[test]
fn the_header_is_a_button_that_says_whether_the_section_is_open() {
    let mut harness = harness();
    assert_eq!(expanded(&harness), Some(false));
    harness.state_mut().app.env = Shown::Open;
    harness.run_steps(40);
    assert_eq!(expanded(&harness), Some(true));
}

#[test]
fn the_header_controls_the_region_only_while_the_region_is_built() {
    let mut harness = harness();
    assert!(controlled(&harness).is_empty(), "closed: no relation");
    assert!(
        harness.query_by_role(AccessRole::Region).is_none(),
        "closed: no region in the tree"
    );
    harness.state_mut().app.env = Shown::Open;
    harness.run_steps(40);
    assert_eq!(
        controlled(&harness),
        vec![(AccessRole::Region, Some(TITLE.to_owned()))]
    );
    assert!(
        harness.query_by_label("Inside").is_some(),
        "precondition: the content is in the tree"
    );
}

#[test]
fn tab_from_the_open_header_goes_into_its_content() {
    let mut harness = harness_of(App {
        env: Shown::Open,
        ..App::default()
    });
    support::past_bar(&mut harness);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(support::focused_path(&harness).as_deref(), Some(HEADER));
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    let focused = support::focused_path(&harness).unwrap_or_default();
    assert!(focused.ends_with(r#""inside""#), "{focused}");
}

#[test]
fn the_chevron_turns_with_the_height_by_the_same_progress() {
    let mut harness = harness_of(App {
        env: Shown::Open,
        ..App::default()
    });
    let natural = region_height(&harness);
    assert!(natural > 20.0, "precondition: open");
    assert_eq!(chevron_degrees(&harness).round(), 180.0);
    harness.state_mut().app.env = Shown::Closed;
    let mut between = 0;
    for _ in 0..40 {
        harness.step();
        let progress = region_height(&harness) / natural;
        let angle = chevron_degrees(&harness);
        assert!(
            (angle - 180.0 * progress).abs() < 3.0,
            "{angle} degrees at progress {progress}"
        );
        between += usize::from(angle > 20.0 && angle < 160.0);
    }
    assert!(between > 5, "precondition: it was in between frames");
    assert_eq!(chevron_degrees(&harness).round() % 360.0, 0.0);
}

#[test]
fn reduced_motion_snaps_the_height_and_the_chevron() {
    let mut d = support::driver(
        vec2(400.0, 400.0),
        common::base_text(egui::Theme::Light),
        App::default(),
        view,
    )
    .step(std::time::Duration::from_millis(10))
    .motion(Animation::Reduced)
    .build();
    d.frames(4);
    d.click(Target::role(AccessRole::Button, TITLE));
    d.frames(2);
    assert_eq!(d.app().env, Shown::Open, "precondition: opened");
    assert!(
        d.rect(Target::suffix("/Region")).height() > 20.0,
        "the height snapped open"
    );
    let head = d.rect(Target::suffix("/Header"));
    let icons: Vec<_> = support::textured(&d)
        .into_iter()
        .filter(|mesh| head.contains(mesh.calc_bounds().center()))
        .collect();
    assert_eq!(icons.len(), 1, "the header draws one chevron");
    assert_eq!(support::turn(icons[0]).round(), 180.0);
}

#[test]
fn a_disabled_header_neither_toggles_nor_takes_focus() {
    let mut harness = harness_of(App {
        enablement: Enablement::Disabled,
        ..App::default()
    });
    header(&harness).click();
    harness.run_steps(10);
    support::past_bar(&mut harness);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.state().app.changes.is_empty());
    assert_ne!(support::focused_path(&harness).as_deref(), Some(HEADER));
}

#[test]
fn the_disclosure_button_keeps_its_name_and_says_whether_it_is_open() {
    let mut harness = harness();
    let button = |harness: &Harness<'_, Rig<App>>| {
        harness
            .get_by_role_and_label(AccessRole::Button, "src")
            .accesskit_node()
            .data()
            .is_expanded()
    };
    assert_eq!(button(&harness), Some(false));
    harness
        .get_by_role_and_label(AccessRole::Button, "src")
        .click();
    harness.run_steps(2);
    assert_eq!(harness.state().app.folds, vec![Shown::Open]);
    assert_eq!(button(&harness), Some(true));
    assert!(
        harness
            .get_by_role_and_label(AccessRole::Button, "src")
            .accesskit_node()
            .controls()
            .next()
            .is_none(),
        "it controls no region"
    );
}

#[test]
fn the_disclosure_button_snaps() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(AccessRole::Button, "src")
        .click();
    harness.step();
    harness.step();
    let rect = rect_of(&harness, r#"/"fold""#);
    let turns: Vec<_> = support::textured(&harness)
        .into_iter()
        .filter(|mesh| rect.contains(mesh.calc_bounds().center()))
        .map(support::turn)
        .collect();
    assert_eq!(turns.len(), 1, "precondition: the button draws one chevron");
    assert_eq!(turns[0].round(), 90.0, "it turned all the way at once");
}

#[test]
fn an_unnamed_disclosure_button_is_reported() {
    struct Rig {
        layout: eggshell_ui::LayoutState,
        fold: Shown,
    }
    let mut harness = Harness::builder()
        .with_size(vec2(100.0, 100.0))
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, rig: &mut Rig| {
                let Rig { layout, fold } = rig;
                let shown = *fold;
                layout
                    .build(ui.ctx(), |b| {
                        b.disclosure("fold", shown)
                            .on_change(|fold: &mut Shown, to| *fold = to);
                    })
                    .layout(ui.max_rect())
                    .draw(fold, ui);
            },
            Rig {
                layout: eggshell_ui::LayoutState::new(),
                fold: Shown::Closed,
            },
        );
    harness.run_steps(2);
    eggshell_ui::testing::assert_reported(&harness.ctx, Problem::Unnamed);
}

#[test]
fn the_disclosure_button_is_a_square_with_an_accent_tint_under_the_pointer() {
    use eggshell_ui::theme::Shadcn;
    use egui::epaint::Shape;
    let mut harness = harness();
    let rect = rect_of(&harness, r#"/"fold""#);
    assert_eq!(
        (rect.width(), rect.height()),
        (24.0, 24.0),
        "a 24 pt square"
    );
    let accent = Shadcn::of(&harness.ctx).tokens.accent;
    let tinted = |harness: &Harness<'_, Rig<App>>| {
        support::shapes(harness)
            .into_iter()
            .any(|shape| match shape {
                Shape::Rect(shape) => support::same(shape.rect, rect) && shape.fill == accent,
                _ => false,
            })
    };
    assert!(!tinted(&harness), "precondition: no tint at rest");
    support::hover_over(&mut harness, rect.center());
    harness.run_steps(40);
    assert!(tinted(&harness), "the accent tint under the pointer");
}

fn driver_with(edit: impl Fn(&mut eggshell_ui::Style)) -> eggshell_ui::testing::Driver<App> {
    let mut d = support::driver(
        vec2(400.0, 400.0),
        common::base_text(egui::Theme::Light),
        App::default(),
        view,
    )
    .style(support::shadcn_with(edit))
    .build();
    d.frames(4);
    d
}

/// A section's header is shadcn's accordion trigger (accordion.tsx, vega):
/// `border` (transparent) and `py-4` around a `text-sm` line.
#[test]
fn a_sections_header_is_shadcns_accordion_trigger() {
    let d = driver_with(|_| {});
    let header = d.rect(Target::suffix(HEADER));
    assert_eq!(
        header.height(),
        1.0 + 16.0 + 20.0 + 16.0 + 1.0,
        "{header:?}"
    );
}

/// The section draws from the style's sheet: an App rule restyles its header.
#[test]
fn an_app_rule_restyles_a_sections_header() {
    let d = driver_with(|style| {
        style.sheet.as_app().rule(
            eggshell_ui::owned::accordion::AccordionPart::Trigger,
            |t, _| t.bg(egui::Color32::RED),
        );
    });
    let header = d.rect(Target::suffix(HEADER));
    assert_eq!(support::box_at(&d, header).fill, egui::Color32::RED);
}

/// The header's title and its chevron.
fn title_and_chevron(d: &eggshell_ui::testing::Driver<App>) -> (egui::Rect, egui::Rect) {
    (
        rect_of(d, &format!("{HEADER}/<div>")),
        rect_of(d, &format!("{HEADER}/<glyph>")),
    )
}

/// Recorded exceptions to shadcn's accordion (#350, #384): the header keeps
/// its gap between the title and the chevron, and the chevron its nudge
/// down to the title's line.
#[test]
fn a_sections_header_keeps_its_gap_and_its_chevrons_nudge() {
    let d = driver_with(|_| {});
    let style = eggshell_ui::theme::Shadcn::for_theme(egui::Theme::Light)
        .style()
        .disclosure;
    let header = d.rect(Target::suffix(HEADER));
    let (title, chevron) = title_and_chevron(&d);
    assert_eq!(chevron.left() - title.right(), style.header.gap, "the gap");
    assert_eq!(
        chevron.top() - title.top(),
        style.nudge,
        "the nudge: {header:?} {title:?} {chevron:?}"
    );
}
