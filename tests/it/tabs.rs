//! Tabs through kittest's real keys and pointer: the roles, the one Tab stop,
//! the arrows, activation on focus or on Enter, the lazy panel, and the look,
//! which is the segmented control's.

use std::cell::RefCell;

use eggshell_ui::owned::tabs::Ext as _;
use eggshell_ui::owned::{Enablement, choice::Choice, tabs::TabsPart};
use eggshell_ui::sheet::When;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Aria, Builder, TextStyle, tw};
use egui::{Event, Key, Modifiers, MouseWheelUnit, Role, Theme, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, rect_of};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Rust,
    Html,
    Css,
    Toml,
}

const PAGES: [Page; 4] = [Page::Rust, Page::Html, Page::Css, Page::Toml];

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Toml => "TOML",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Activation {
    OnFocus,
    OnEnter,
}

struct App {
    page: Page,
    activation: Activation,
    /// The whole control's enablement.
    enablement: Enablement,
    disabled: Option<Page>,
    changes: Vec<Page>,
    /// Each page whose panel was built, a frame at a time.
    built: RefCell<Vec<Page>>,
    note: String,
}

impl App {
    fn new(page: Page) -> Self {
        Self {
            page,
            activation: Activation::OnFocus,
            enablement: Enablement::Enabled,
            disabled: None,
            changes: Vec::new(),
            built: RefCell::new(Vec::new()),
            note: String::new(),
        }
    }
}

const ROOT: &str = "/\"lang\"";

fn tab(index: usize) -> String {
    format!("{ROOT}/List/Tab({index})")
}

fn panel(index: usize) -> String {
    format!("{ROOT}/Panel({index})")
}

/// ```html
/// <button>Before</button>
/// <div role="tablist" aria-label="Language">Rust HTML CSS TOML</div>
/// <div role="tabpanel">the page's text field, and a tall scroll area</div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.button("before", "Before").on_click(|_: &mut App| {});
        let options = PAGES.map(|page| {
            Choice::new(page, page.label()).enablement(if app.disabled == Some(page) {
                Enablement::Disabled
            } else {
                Enablement::Enabled
            })
        });
        let tabs = b
            .tabs("lang", app.page, options)
            .named("Language")
            .enablement(app.enablement);
        let tabs = match app.activation {
            Activation::OnFocus => tabs,
            Activation::OnEnter => tabs.activate_on_enter(),
        };
        let note = app.note.clone();
        tabs.on_change(|app: &mut App, page| {
            app.page = page;
            app.changes.push(page);
        })
        .panel(|b, page| {
            b.text_field("note", note)
                .named(format!("{} note", page.label()))
                .classes(tw().w_px(160.0))
                .on_change(|app: &mut App, text| app.note = text);
            b.scroll("code", tw().h_px(60.0).w_px(160.0)).children(|b| {
                for line in 0..20 {
                    b.text(tw().h_px(20.0), format!("{} {line}", page.label()));
                }
            });
            app.built.borrow_mut().push(page);
        });
    });
}

fn harness(app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(500.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.run_steps(4);
    harness
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press_modifiers(Modifiers::NONE, key);
    harness.run_steps(4);
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..6 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        press(harness, Key::Tab);
    }
    panic!("could not Tab to {path}: {:?}", focused_path(harness));
}

fn focused_tab(harness: &Harness<'_, Rig<App>>) -> Option<usize> {
    let path = focused_path(harness)?;
    (0..PAGES.len()).find(|index| path == tab(*index))
}

mod roles {
    use super::*;

    #[test]
    fn the_list_is_a_named_tablist_of_tabs_the_current_one_selected() {
        let harness = harness(App::new(Page::Html));
        let list = harness.get_by_role(Role::TabList);
        assert_eq!(list.accesskit_node().label().as_deref(), Some("Language"));
        let tabs: Vec<_> = harness
            .get_all_by_role(Role::Tab)
            .map(|node| {
                let node = node.accesskit_node();
                (node.label(), node.is_selected())
            })
            .collect();
        assert_eq!(
            tabs,
            [
                (Some("Rust".to_owned()), Some(false)),
                (Some("HTML".to_owned()), Some(true)),
                (Some("CSS".to_owned()), Some(false)),
                (Some("TOML".to_owned()), Some(false)),
            ]
        );
    }

    #[test]
    fn the_panel_is_named_by_its_tab_and_the_tab_controls_it() {
        let harness = harness(App::new(Page::Html));
        let panels: Vec<_> = harness.get_all_by_role(Role::TabPanel).collect();
        assert_eq!(panels.len(), 1, "only the selected panel is built");
        let panel = panels.first().expect("the panel").accesskit_node();
        assert_eq!(panel.label().as_deref(), Some("HTML"));
        let controls: Vec<_> = harness
            .get_all_by_role(Role::Tab)
            .map(|node| node.accesskit_node().data().controls().to_vec())
            .collect();
        let panel_id = support::id_of(&harness, &super::panel(1)).accesskit_id();
        assert_eq!(
            controls,
            [vec![], vec![panel_id], vec![], vec![]],
            "only the selected tab names a panel: the others' are not built"
        );
    }
}

mod pointer {
    use super::*;

    #[test]
    fn a_click_selects_the_tab_and_its_panel_replaces_the_last() {
        let mut harness = harness(App::new(Page::Rust));
        click_node(&mut harness, &tab(2));
        harness.run_steps(4);
        assert_eq!(harness.state().app.changes, [Page::Css]);
        assert_eq!(harness.state().app.page, Page::Css);
        let nodes = &harness.state().nodes;
        assert!(nodes.iter().any(|(path, _)| *path == panel(2)));
        assert!(!nodes.iter().any(|(path, _)| *path == panel(0)));
    }

    #[test]
    fn disabled_tabs_take_no_click() {
        let mut harness = harness(App {
            enablement: Enablement::Disabled,
            ..App::new(Page::Rust)
        });
        click_node(&mut harness, &tab(2));
        harness.run_steps(4);
        assert!(harness.state().app.changes.is_empty());
        assert_eq!(
            harness.get_all_by_role(Role::TabPanel).count(),
            1,
            "the selected panel still shows"
        );
    }

    #[test]
    fn a_disabled_tab_does_not_select() {
        let mut harness = harness(App {
            disabled: Some(Page::Css),
            ..App::new(Page::Rust)
        });
        click_node(&mut harness, &tab(2));
        harness.run_steps(4);
        assert!(harness.state().app.changes.is_empty());
    }
}

mod keyboard {
    use super::*;

    #[test]
    fn the_list_is_one_tab_stop_on_the_selected_tab_and_tab_goes_on_into_the_panel() {
        let mut harness = harness(App::new(Page::Css));
        tab_to(&mut harness, "/\"before\"");
        press(&mut harness, Key::Tab);
        assert_eq!(focused_tab(&harness), Some(2), "the selected tab");
        press(&mut harness, Key::Tab);
        assert_eq!(
            support::focused_node(&harness).as_deref(),
            Some(format!("{}/\"note\"", panel(2)).as_str()),
            "the panel's first focusable, past the other tabs"
        );
    }

    #[test]
    fn the_arrows_select_as_they_move_and_wrap() {
        let mut harness = harness(App::new(Page::Html));
        tab_to(&mut harness, &tab(1));
        for (key, to) in [
            (Key::ArrowRight, Page::Css),
            (Key::ArrowRight, Page::Toml),
            (Key::ArrowRight, Page::Rust),
            (Key::ArrowLeft, Page::Toml),
        ] {
            press(&mut harness, key);
            assert_eq!(harness.state().app.page, to, "{key:?}");
            let index = PAGES.iter().position(|page| *page == to);
            assert_eq!(focused_tab(&harness), index, "focus follows {key:?}");
        }
        assert_eq!(
            harness.state().app.changes,
            [Page::Css, Page::Toml, Page::Rust, Page::Toml],
            "one change a key"
        );
    }

    #[test]
    fn home_and_end_go_to_the_ends() {
        let mut harness = harness(App::new(Page::Html));
        tab_to(&mut harness, &tab(1));
        press(&mut harness, Key::End);
        assert_eq!(harness.state().app.page, Page::Toml);
        press(&mut harness, Key::Home);
        assert_eq!(harness.state().app.page, Page::Rust);
    }

    #[test]
    fn the_arrows_pass_over_a_disabled_tab() {
        let mut harness = harness(App {
            disabled: Some(Page::Css),
            ..App::new(Page::Html)
        });
        tab_to(&mut harness, &tab(1));
        press(&mut harness, Key::ArrowRight);
        assert_eq!(harness.state().app.page, Page::Toml);
    }

    #[test]
    fn up_and_down_do_nothing() {
        let mut harness = harness(App::new(Page::Html));
        tab_to(&mut harness, &tab(1));
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::ArrowUp);
        assert!(harness.state().app.changes.is_empty());
        assert_eq!(focused_tab(&harness), Some(1));
    }

    #[test]
    fn activated_on_enter_the_arrows_only_move_and_enter_selects() {
        let mut harness = harness(App {
            activation: Activation::OnEnter,
            ..App::new(Page::Html)
        });
        tab_to(&mut harness, &tab(1));
        press(&mut harness, Key::ArrowRight);
        press(&mut harness, Key::ArrowRight);
        assert_eq!(focused_tab(&harness), Some(3), "focus moved");
        assert!(harness.state().app.changes.is_empty(), "nothing selected");
        press(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.changes, [Page::Toml]);
        assert_eq!(focused_tab(&harness), Some(3), "focus stays on the tab");
    }
}

mod panels {
    use super::*;

    fn wheel(harness: &mut Harness<'_, Rig<App>>, path: &str) {
        let at = rect_of(harness, path).center();
        harness.hover_at(at);
        harness.step();
        harness.event(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -40.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        });
        harness.run_steps(8);
    }

    fn offset(harness: &Harness<'_, Rig<App>>, path: &str) -> Vec2 {
        support::shell(harness)
            .viewport(support::id_of(harness, path))
            .expect("the scroll area was laid out")
            .offset()
    }

    #[test]
    fn only_the_selected_panel_is_built() {
        let mut harness = harness(App::new(Page::Html));
        harness.state().app.built.borrow_mut().clear();
        harness.run_steps(3);
        let built = harness.state().app.built.borrow();
        assert!(!built.is_empty(), "precondition: the panel was built");
        assert!(built.iter().all(|page| *page == Page::Html), "{built:?}");
    }

    #[test]
    fn a_panel_keeps_its_scroll_across_a_switch_away_and_back() {
        let mut harness = harness(App::new(Page::Rust));
        let code = format!("{}/\"code\"", panel(0));
        wheel(&mut harness, &code);
        let scrolled = offset(&harness, &code);
        assert!(scrolled.y > 0.0, "precondition: the code scrolled");
        click_node(&mut harness, &tab(1));
        harness.run_steps(4);
        assert!(
            !harness.state().nodes.iter().any(|(path, _)| *path == code),
            "precondition: the Rust panel is gone"
        );
        click_node(&mut harness, &tab(0));
        harness.run_steps(4);
        assert_eq!(offset(&harness, &code), scrolled);
    }

    #[test]
    fn when_the_selected_tab_is_disabled_the_first_enabled_tab_is_the_stop() {
        let mut harness = harness(App {
            disabled: Some(Page::Rust),
            ..App::new(Page::Rust)
        });
        assert_eq!(
            harness.get_all_by_role(Role::TabPanel).count(),
            1,
            "a disabled selected tab still shows its panel"
        );
        tab_to(&mut harness, "/\"before\"");
        press(&mut harness, Key::Tab);
        assert_eq!(focused_tab(&harness), Some(1));
    }
}

mod look {
    use super::*;

    /// The library's segmented control over the same pages, and the tabs.
    fn side_by_side(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
            b.segmented(
                "segmented",
                app.page,
                PAGES.map(|page| (page, page.label())),
            )
            .on_change(|_: &mut App, _| {});
            b.tabs("lang", app.page, PAGES.map(|page| (page, page.label())))
                .on_change(|_: &mut App, _| {})
                .panel(|_, _| {});
        });
    }

    /// A path under the tab list as the same node's under a segmented
    /// control: `/Tab(1)/…` is `/1/…`.
    fn as_segmented(path: &str) -> String {
        match path
            .strip_prefix("/Tab(")
            .and_then(|rest| rest.split_once(')'))
        {
            Some((index, rest)) => format!("/{index}{rest}"),
            None => path.to_owned(),
        }
    }

    #[test]
    fn the_tab_list_draws_as_the_segmented_control_does() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            vec2(500.0, 300.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(Page::Html),
            side_by_side,
        );
        harness.run_steps(4);
        let nodes = &harness.state().nodes;
        let under = |prefix: &str| -> Vec<(String, egui::Rect)> {
            let origin = rect_of(&harness, prefix).min;
            nodes
                .iter()
                .filter_map(|(path, rect)| {
                    let rest = path.strip_prefix(prefix)?;
                    Some((rest.to_owned(), rect.translate(-origin.to_vec2())))
                })
                .collect()
        };
        let segmented = under("/\"segmented\"");
        let tabs: Vec<_> = under(&format!("{ROOT}/List"))
            .into_iter()
            .map(|(path, rect)| (as_segmented(&path), rect))
            .collect();
        assert!(
            segmented.len() > PAGES.len(),
            "precondition: the track and its cells: {segmented:?}"
        );
        assert_eq!(segmented, tabs);
        let fill = |path: &str| support::box_at(&harness, rect_of(&harness, path)).fill;
        assert_eq!(fill("/\"segmented\""), fill(&format!("{ROOT}/List")));
        for index in 0..PAGES.len() {
            assert_eq!(
                fill(&format!("/\"segmented\"/{index}")),
                fill(&tab(index)),
                "tab {index}"
            );
        }
    }

    const TRACK: egui::Color32 = egui::Color32::from_rgb(20, 40, 200);
    const CHOSEN: egui::Color32 = egui::Color32::from_rgb(200, 10, 10);

    /// An app's rules on the tabs' parts: the list, the selected trigger,
    /// and the gap between the list and the panel.
    fn restyled(style: &mut eggshell_ui::Style) {
        style
            .sheet
            .as_app()
            .rule(TabsPart::List, |t, _| t.bg(TRACK))
            .rule_when(
                TabsPart::Trigger,
                When::always().aria(Aria::Selected),
                |t, _| t.bg(CHOSEN),
            )
            .rule(TabsPart::Root, |t, _| t.gap(5.0));
    }

    #[test]
    fn an_app_rule_on_a_tabs_part_restyles_the_tabs_and_the_segmented_control() {
        let tokens = Tokens::for_theme(Theme::Light);
        let mut harness = support::harness(
            vec2(500.0, 300.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            App::new(Page::Html),
            side_by_side,
        );
        harness
            .state_mut()
            .layout
            .set_style(support::shadcn_with(restyled));
        harness.run_steps(4);
        let fill = |path: &str| support::box_at(&harness, rect_of(&harness, path)).fill;
        for list in ["/\"segmented\"".to_owned(), format!("{ROOT}/List")] {
            assert_eq!(fill(&list), TRACK, "{list}");
        }
        assert_eq!(fill("/\"segmented\"/1"), CHOSEN);
        assert_eq!(fill(&tab(1)), CHOSEN);
        assert_ne!(
            fill(&tab(0)),
            CHOSEN,
            "only the selected trigger: the preset's other rules stand"
        );
        let list = rect_of(&harness, &format!("{ROOT}/List"));
        let panel = rect_of(&harness, &panel(1));
        assert_eq!(panel.top() - list.bottom(), 20.0, "the root's gap");
    }
}
