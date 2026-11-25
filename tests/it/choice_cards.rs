//! Choice cards (#446): a label that holds the field, the control after its
//! title and description, chosen or toggled by a click anywhere on it.

use eggshell_ui::owned::{Enablement, choice::Choice};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, Tokens};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Key, Rect, Role, Theme, pos2, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, box_at, focused_path, press_and_release, ring_at};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Plan {
    #[default]
    Hobby,
    Pro,
    Closed,
}

#[derive(Default)]
struct App {
    plan: Plan,
    backups: bool,
    shared: bool,
}

/// ```html
/// <div class="flex flex-col gap-6 p-4 w-[380px]">
///   <div role="radiogroup">
///     <label data-slot="field-label">Hobby…</label>
///     <label data-slot="field-label">Pro…</label>
///     <label data-slot="field-label" data-disabled>Closed…</label>
///   </div>
///   <label data-slot="field-label">Backups… <button role="checkbox" /></label>
///   <label data-slot="field-label">Shared… <button role="switch" /></label>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().gap(6.0).p(4.0).w_px(380.0), |b| {
        b.radio_group(
            "plan",
            app.plan,
            [
                Choice::new(Plan::Hobby, "Hobby").description("For one person."),
                Choice::new(Plan::Pro, "Pro").description("For teams."),
                Choice::new(Plan::Closed, "Closed")
                    .description("Not offered.")
                    .enablement(Enablement::Disabled),
            ],
        )
        .cards()
        .on_change(|app: &mut App, plan| app.plan = plan);
        b.checkbox("backups", "Backups", app.backups)
            .card()
            .description("Every night.")
            .on_change(|app: &mut App, backups| app.backups = backups);
        b.switch("shared", "Shared", app.shared)
            .card()
            .description("On every device.")
            .on_change(|app: &mut App, shared| app.shared = shared);
    });
}

fn driver(theme: Theme) -> Driver<App> {
    let tokens = Tokens::for_theme(theme);
    let mut d = support::driver(
        vec2(420.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.ctx().set_theme(theme);
    d.set_style(eggshell_ui::contract::Themed::both(std::sync::Arc::new(
        Shadcn::for_theme(theme).style(),
    )));
    d.frames(6);
    d
}

const PRO: &str = "/\"plan\"/Label(1)";
const CLOSED: &str = "/\"plan\"/Label(2)";
const BACKUPS: &str = "/\"backups\"/Label";
const SHARED: &str = "/\"shared\"/Label";

/// A point on `card` away from its control: its bottom left corner, inside
/// the padding.
fn away_from_the_control(card: Rect) -> egui::Pos2 {
    pos2(card.left() + 6.0, card.bottom() - 6.0)
}

fn click_card(d: &mut Driver<App>, path: &str) {
    let at = away_from_the_control(d.rect(Target::suffix(path)));
    d.hover(at);
    press_and_release(d, at);
    d.frames(4);
}

#[test]
fn a_click_anywhere_on_a_card_chooses_or_toggles_its_control() {
    let mut d = driver(Theme::Light);
    click_card(&mut d, PRO);
    assert_eq!(d.app().plan, Plan::Pro, "the radio card");
    click_card(&mut d, BACKUPS);
    assert!(d.app().backups, "the checkbox card");
    click_card(&mut d, SHARED);
    assert!(d.app().shared, "the switch card");
    click_card(&mut d, BACKUPS);
    assert!(!d.app().backups, "and back");
}

#[test]
fn a_disabled_card_ignores_a_click() {
    let mut d = driver(Theme::Light);
    click_card(&mut d, CLOSED);
    assert_eq!(d.app().plan, Plan::Hobby);
}

#[test]
fn a_checked_card_is_tinted_with_the_primary_colour() {
    for (theme, fill, border) in [(Theme::Light, 0.05, 0.3), (Theme::Dark, 0.1, 0.2)] {
        let d = driver(theme);
        let primary = Shadcn::for_theme(theme).style().tokens.primary;
        let chosen = box_at(&d, d.rect(Target::suffix("/\"plan\"/Label(0)")));
        assert_eq!(chosen.fill, primary.gamma_multiply(fill), "{theme:?}");
        assert_eq!(
            chosen.stroke.color,
            primary.gamma_multiply(border),
            "{theme:?}"
        );
        assert_eq!(chosen.stroke.width, 1.0);
        let other = box_at(&d, d.rect(Target::suffix(PRO)));
        assert_eq!(
            other.stroke.color,
            Shadcn::for_theme(theme).style().tokens.border,
            "{theme:?}: an unchecked card's border"
        );
    }
}

#[test]
fn the_control_sits_at_the_end_of_the_card_inside_its_padding() {
    let d = driver(Theme::Light);
    let card = d.rect(Target::suffix(BACKUPS));
    let control = d.rect(Target::suffix("/\"backups\"/Label/Box"));
    assert_eq!(card.right() - control.right(), 1.0 + 12.0, "border and p-3");
    assert_eq!(
        control.top() - card.top(),
        1.0 + 12.0 + 1.0,
        "border, p-3 and mt-px"
    );
}

#[test]
fn a_focused_control_rings_its_card_not_itself() {
    let mut d = driver(Theme::Light);
    for _ in 0..8 {
        if focused_path(&d).as_deref() == Some("/\"backups\"/Label/Box") {
            break;
        }
        d.press(Key::Tab).frames(1);
    }
    assert_eq!(focused_path(&d).as_deref(), Some("/\"backups\"/Label/Box"));
    d.frames(4);
    let card = d.rect(Target::suffix(BACKUPS));
    let control = d.rect(Target::suffix("/\"backups\"/Label/Box"));
    let ring = ring_at(&d, card).expect("the card rings");
    let focus = Shadcn::for_theme(Theme::Light).style().tokens.focus();
    assert_eq!(ring.stroke.width, focus.width);
    assert!(ring_at(&d, control).is_none(), "the control does not");
}

#[test]
fn a_cards_title_names_its_control() {
    let d = driver(Theme::Light);
    d.harness().get_by_role_and_label(Role::RadioButton, "Pro");
    d.harness().get_by_role_and_label(Role::CheckBox, "Backups");
    d.harness().get_by_role_and_label(Role::Switch, "Shared");
}

/// A disabled card keeps its border, as shadcn's label holds the field and
/// is not inside it; its title, inside the field, fades once
/// (`group-data-[disabled=true]/field:opacity-50`).
#[test]
fn a_disabled_cards_title_fades_once_and_its_box_does_not() {
    let d = driver(Theme::Light);
    let t = Tokens::for_theme(Theme::Light);
    let (pro, closed) = (d.rect(Target::suffix(PRO)), d.rect(Target::suffix(CLOSED)));
    assert_eq!(
        box_at(&d, closed).stroke.color,
        box_at(&d, pro).stroke.color,
        "the box as an enabled card's"
    );
    assert_eq!(
        support::text_color(&d, closed, "Closed"),
        t.foreground.gamma_multiply(0.5),
        "the title at half"
    );
}
