//! A style's extensions (#435): a widget's own look kept inside the `Style`,
//! read with `extension::<T>()`, derived from the style when unset, and
//! carried by every clone, scope and layout the style goes to.

use std::sync::Arc;

use eggshell_ui::contract::{Extension, Themed};
use eggshell_ui::{Builder, LayoutState, Style, tw};
use egui::vec2;
use egui_kittest::Harness;

/// A widget's look: its gap, derived from the core style's table padding.
#[derive(Clone, Debug, PartialEq)]
struct Probe {
    gap: f32,
}

impl Extension for Probe {
    fn derive(style: &Style) -> Self {
        Self {
            gap: style.scroll.width * 2.0,
        }
    }
}

fn padded(padding: f32) -> Style {
    let mut style = Style::default();
    style.scroll.width = padding;
    style
}

#[test]
fn an_unset_extension_derives_from_the_style() {
    assert_eq!(*padded(5.0).extension::<Probe>(), Probe { gap: 10.0 });
}

#[test]
fn a_set_extension_is_read_back_and_kept_by_a_clone() {
    let mut style = padded(5.0);
    style.set_extension(Probe { gap: 3.0 });
    assert_eq!(*style.extension::<Probe>(), Probe { gap: 3.0 });
    let copy = style.clone();
    assert_eq!(*copy.extension::<Probe>(), Probe { gap: 3.0 });
}

fn probe_of(b: &Builder<'_, '_, ()>) -> f32 {
    b.style().extension::<Probe>().gap
}

#[test]
fn a_scoped_style_carries_its_extension_to_the_nodes_it_builds_alone() {
    let mut seen = (0.0, 0.0);
    let mut harness = Harness::builder()
        .with_size(vec2(200.0, 100.0))
        .build_ui_state(
            |ui, (layout, seen): &mut (LayoutState, (f32, f32))| {
                layout
                    .build(ui.ctx(), |b: &mut Builder<'_, '_, ()>| {
                        b.styled(
                            |style| style.set_extension(Probe { gap: 7.0 }),
                            |b| seen.0 = probe_of(b),
                        );
                        seen.1 = probe_of(b);
                        b.div(tw(), |_| {});
                    })
                    .layout(ui.max_rect())
                    .draw(&mut (), ui);
            },
            (
                LayoutState::new().with_style(Themed {
                    light: Arc::new(padded(5.0)),
                    dark: Arc::new(padded(5.0)),
                }),
                seen,
            ),
        );
    harness.run_steps(1);
    seen = harness.state().1;
    assert_eq!(seen, (7.0, 10.0), "inside the scope, then outside it");
}

#[test]
fn a_layout_s_style_carries_its_extensions_to_the_shell() {
    let mut style = padded(5.0);
    style.set_extension(Probe { gap: 9.0 });
    let style = Arc::new(style);
    let layout = LayoutState::new().with_style(Themed {
        light: style.clone(),
        dark: style,
    });
    let ctx = egui::Context::default();
    assert_eq!(layout.shell(&ctx).style().extension::<Probe>().gap, 9.0);
}
