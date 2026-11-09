//! The examples that once hard-coded light palettes, built under each theme:
//! their main surfaces carry that theme's tokens. Shapes only, no GPU.

use bullseye::{Builder, LayoutState, NodeKind, Placed, PlacedNode};
use bullseye_shadcn::{Shadcn, ThemeColor, Tokens};
use egui::{Color32, Context, ImageSource, Rect, Theme, pos2};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/button.rs"]
mod button;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/controls_gallery.rs"]
mod controls_gallery;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/kaolin/main.rs"]
mod kaolin;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/known_gaps.rs"]
mod known_gaps;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/todo.rs"]
mod todo;

const AREA: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(1100.0, 800.0),
};

const THEMES: [Theme; 2] = [Theme::Light, Theme::Dark];

fn ctx_in(theme: Theme) -> Context {
    let ctx = Context::default();
    ctx.set_theme(theme);
    ctx.run_ui(Default::default(), |_| {})
        .drop_without_applying_deltas();
    Shadcn::apply_both(&ctx);
    ctx
}

fn place<A>(
    theme: Theme,
    build: impl for<'f> FnOnce(&mut Builder<'_, 'f, A>),
) -> (Placed<A>, Tokens) {
    let ctx = ctx_in(theme);
    let placed = LayoutState::new()
        .with_base_text(common::base_text(theme))
        .build(&ctx, build)
        .layout(AREA);
    (placed, Tokens::for_theme(theme))
}

#[track_caller]
fn node<'p, A>(placed: &'p Placed<A>, path: &str) -> &'p PlacedNode {
    placed
        .nodes()
        .find(|node| node.path() == path)
        .unwrap_or_else(|| panic!("no node {path}: {}", placed.dump()))
}

fn backgrounds<A>(placed: &Placed<A>) -> Vec<Color32> {
    placed
        .nodes()
        .filter_map(|node| node.paint().background)
        .collect()
}

#[track_caller]
fn assert_themes_differ(light: Color32, dark: Color32) {
    assert_ne!(light, dark, "precondition: the two themes' tokens differ");
}

#[test]
fn button_takes_the_themes_tokens() {
    let mut seen = Vec::new();
    for theme in THEMES {
        let (placed_off, t) = place(theme, |b| button::layout(b, &button::State::default()));
        let placed = &placed_off;
        let fills = button::Fills::of(&t);
        let button = node(placed, "/\"button\"");
        assert_eq!(button.paint().background, Some(fills.rest), "{theme:?}");
        assert_eq!(button.text().color, t.foreground, "{theme:?}");
        assert_eq!(button.paint().border_colors.top, Some(t.input), "{theme:?}");
        assert!(
            backgrounds(placed).contains(&t.card),
            "{theme:?}: the controls panel is a card"
        );
        seen.push(fills.rest);

        let (placed, _) = place(theme, |b| {
            button::layout(
                b,
                &button::State {
                    clicks: 0,
                    selected: true,
                },
            )
        });
        let selected = node(&placed, "/\"selectable\"");
        assert_eq!(selected.paint().background, Some(t.accent));
        assert_eq!(selected.text().color, t.accent_foreground);
        let resting = node(&placed_off, "/\"selectable\"");
        assert_eq!(resting.paint().background, None);
        assert_eq!(resting.text().color, t.foreground);
    }
    assert_themes_differ(seen[0], seen[1]);
}

#[test]
fn todo_takes_the_themes_tokens() {
    let mut seen = Vec::new();
    for theme in THEMES {
        let (placed, t) = place(theme, |b| todo::layout(b, &todo::State::default()));
        let state = todo::State::default();
        let row = node(&placed, &format!("/{:?}", state.items[0].key));
        assert_eq!(row.paint().background, Some(t.muted), "{theme:?}");
        assert_eq!(row.paint().border_colors.top, Some(t.input), "{theme:?}");
        assert_eq!(row.text().color, t.foreground, "{theme:?}");
        let up = node(&placed, &format!("/{:?}/\"up\"", state.items[0].key));
        assert_eq!(up.text().color, t.muted_foreground, "{theme:?}");
        seen.push(t.muted);

        let mut cleared = todo::State::default();
        cleared.items.clear();
        let (placed, _) = place(theme, |b| todo::layout(b, &cleared));
        let empty = placed
            .nodes()
            .find(|node| node.kind() == NodeKind::Text)
            .expect("the empty list says so");
        assert_eq!(empty.text().color, t.muted_foreground, "{theme:?}");
    }
    assert_themes_differ(seen[0], seen[1]);
}

fn gallery_source() -> ImageSource<'static> {
    ImageSource::Uri("bytes://logo".into())
}

#[test]
fn kaolin_gallery_chrome_takes_the_themes_tokens() {
    let mut seen = Vec::new();
    for theme in THEMES {
        let (placed, t) = place(theme, |b| kaolin::window(b, 0, gallery_source()));
        let sidebar = placed
            .nodes()
            .find(|node| node.rect().width() == 200.0)
            .expect("the sidebar is 200 points wide");
        assert_eq!(sidebar.paint().background, Some(t.background), "{theme:?}");
        assert_ne!(
            sidebar.paint().background,
            Some(t.accent),
            "{theme:?}: the selected row (accent) shows on the sidebar"
        );
        assert_eq!(sidebar.text().color, t.foreground, "{theme:?}");
        assert!(
            backgrounds(&placed).contains(&t.background),
            "{theme:?}: the stage is the page"
        );
        let hello = placed
            .nodes()
            .find(|node| node.kind() == NodeKind::Text && node.rect().height() == 60.0)
            .expect("the heading is 60 points tall");
        assert_eq!(hello.text().color, t.foreground, "{theme:?}");
        seen.push(t.background);
    }
    assert_themes_differ(seen[0], seen[1]);
}

#[test]
fn kaolin_greys_follow_the_theme_and_the_demo_colours_do_not() {
    for theme in THEMES {
        let (placed, t) = place(theme, |b| kaolin::window(b, 1, gallery_source()));
        let colours = backgrounds(&placed);
        assert!(colours.contains(&t.muted), "{theme:?}: the containers");
        assert!(
            colours.contains(&Color32::DARK_BLUE),
            "{theme:?}: the boxes are the subject of the demo"
        );
        assert!(
            !colours.contains(&Color32::LIGHT_GRAY),
            "{theme:?}: no hard-coded grey is left"
        );

        let (placed, t) = place(theme, |b| kaolin::window(b, 4, gallery_source()));
        let colours = backgrounds(&placed);
        assert!(colours.contains(&t.muted), "{theme:?}: main content");
        assert!(colours.contains(&Color32::DARK_BLUE), "{theme:?}: header");
        assert!(colours.contains(&Color32::DARK_GRAY), "{theme:?}: footer");
    }
}

#[test]
fn known_gaps_chrome_follows_the_theme_and_the_shadow_stage_stays_light() {
    let light = Tokens::light();
    for theme in THEMES {
        let (placed, t) = place::<()>(theme, |b| known_gaps::window(b, None));
        let colours = backgrounds(&placed);
        assert!(colours.contains(&t.muted), "{theme:?}: the shadow section");
        assert!(colours.contains(&t.background), "{theme:?}: weights");
        assert!(
            colours.contains(&light.muted),
            "{theme:?}: the shadow stage"
        );

        let cards: Vec<_> = placed
            .nodes()
            .filter(|node| node.rect().size() == known_gaps::CARD_SIZE.into())
            .filter(|node| node.paint().background.is_some())
            .collect();
        assert!(
            cards.len() >= known_gaps::SHADOWS.len(),
            "{theme:?}: precondition: the cards are on screen"
        );
        for card in cards {
            assert_eq!(
                card.paint().background,
                Some(light.card),
                "{theme:?}: shadows show on a light card"
            );
        }
    }
}

#[test]
fn controls_gallery_uses_the_example_accent() {
    for theme in THEMES {
        let ctx = ctx_in(theme);
        let neutral = Shadcn::of(&ctx).tokens.primary;
        controls_gallery::prepare(&ctx, &controls_gallery::State::default());
        let primary = Shadcn::of(&ctx).tokens.primary;
        assert_eq!(
            primary,
            Tokens::for_theme(theme)
                .with_theme_color(ThemeColor::Blue, theme)
                .primary,
            "{theme:?}"
        );
        assert_ne!(
            primary, neutral,
            "{theme:?}: precondition: Blue is not neutral"
        );
    }
}
