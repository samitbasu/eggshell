//! Hover cards: shadcn's demo (`@nextjs`), a row of handles to sweep across,
//! a card with interactive content, and a tooltip and a popover beside them.
//!
//! Rest the pointer on a handle: the card opens after 700 ms and stays while
//! the pointer crosses into it. Tab to a handle: it opens after the same
//! delay, and focus stays on the handle. Esc, or a press outside, closes it.

#[path = "common/mod.rs"]
mod common;

use core::time::Duration;

use eggshell_ui::overlay::{Anchor, Placement, Side};
use eggshell_ui::owned::hover_card::HoverCardExt as _;
use eggshell_ui::theme::variant;
use eggshell_ui::{Builder, Distance, Icon, tw};

#[derive(Default)]
struct State {
    follows: u32,
    popover: bool,
}

fn calendar() -> Icon {
    Icon::new(egui::ImageSource::Bytes {
        uri: "bytes://lucide-calendar.svg".into(),
        bytes: br##"<svg color="#fff" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/></svg>"##
            .as_slice()
            .into(),
    })
}

/// shadcn's HoverCardDemo content: an avatar, the handle, the bio and when
/// they joined.
fn profile(b: &mut Builder<'_, '_, State>, handle: &str, bio: &str, joined: &str) {
    let t = common::tokens(b.ctx());
    let initial = handle
        .chars()
        .nth(1)
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    b.div(tw().flex().gap(4.0), |b| {
        b.div(
            tw().size_px(40.0)
                .rounded_full()
                .bg(t.muted)
                .flex()
                .items_center()
                .justify_center()
                .shrink_0(),
            |b| b.text(tw().font_semibold(), initial),
        );
        b.div(tw().flex().flex_col().gap(1.0), |b| {
            b.text(
                tw().font_semibold().role(common::sized(14.0)),
                handle.to_owned(),
            );
            b.text(tw().role(common::sized(14.0)), bio.to_owned());
            b.div(tw().flex().items_center().pt(2.0).gap(2.0), |b| {
                b.glyph(tw().size_px(16.0).text(t.muted_foreground), calendar());
                b.text(
                    tw().text(t.muted_foreground).role(common::sized(12.0)),
                    format!("Joined {joined}"),
                );
            });
        });
    });
}

fn handle(b: &mut Builder<'_, '_, State>, key: &'static str, text: &str) {
    b.button(key, text.to_owned())
        .variant(variant::GHOST)
        .on_click(|_: &mut State| {});
}

fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(6.0).p(8.0).items_start(), |b| {
        b.text(tw().font_semibold(), "shadcn's demo");
        b.hover_card("nextjs")
            .trigger(tw().flex(), |t| handle(t, "handle", "@nextjs"))
            .content(|c| {
                profile(
                    c,
                    "@nextjs",
                    "The React Framework – created and maintained by @vercel.",
                    "December 2021",
                );
            });

        b.text(
            tw().font_semibold(),
            "Sweep across: nothing opens until you rest",
        );
        b.div(tw().flex().gap(2.0), |b| {
            for (name, bio) in [
                ("@vercel", "Develop. Preview. Ship."),
                ("@shadcn", "Beautifully designed components."),
                ("@radix-ui", "Unstyled, accessible components."),
                ("@egui", "An easy-to-use immediate mode GUI."),
            ] {
                b.hover_card(name)
                    .trigger(tw().flex(), |t| handle(t, "handle", name))
                    .content(|c| profile(c, name, bio, "2024"));
            }
        });

        b.text(
            tw().font_semibold(),
            "Triggers that are not buttons: pointer only",
        );
        b.div(tw().flex().gap(6.0).items_center(), |b| {
            let t = common::tokens(b.ctx());
            b.hover_card("link")
                .trigger(tw().flex(), |t2| {
                    t2.text(
                        tw().text(t.primary).underline().role(common::sized(14.0)),
                        "@nextjs",
                    )
                })
                .content(|c| profile(c, "@nextjs", "A plain text link.", "December 2021"));
            b.hover_card("avatar")
                .trigger(tw().size_px(40.0).rounded_full(), |t2| {
                    t2.div(
                        tw().size_px(40.0)
                            .rounded_full()
                            .bg(t.muted)
                            .flex()
                            .items_center()
                            .justify_center(),
                        |t2| t2.text(tw().font_semibold(), "VC"),
                    );
                })
                .content(|c| profile(c, "@vercel", "An avatar.", "2024"));
        });

        b.text(tw().font_semibold(), "On each side");
        b.div(tw().flex().gap(4.0).items_center().px(48.0).py(24.0), |b| {
            for (name, side) in [
                ("Top", Side::Top),
                ("Bottom", Side::Bottom),
                ("Left", Side::Left),
                ("Right", Side::Right),
            ] {
                b.hover_card(name)
                    .placement(Placement::beside(side).gap(Distance::new(4.0)))
                    .trigger(tw().flex(), |t| handle(t, "handle", name))
                    .content(|c| c.text(tw(), format!("On the {name} side")));
            }
        });

        b.text(
            tw().font_semibold(),
            "Interactive content, a quicker open, 8 pt apart",
        );
        b.hover_card("follow")
            .open_delay(Duration::from_millis(200))
            .placement(Placement::below().gap(Distance::new(8.0)))
            .trigger(tw().flex(), |t| handle(t, "handle", "@eggshell"))
            .content(|c| {
                c.div(tw().flex().flex_col().gap(3.0), |c| {
                    c.text(tw().font_semibold(), "@eggshell");
                    c.text(tw().text(muted), format!("{} followers", state.follows));
                    c.button("follow", "Follow")
                        .tooltip("Follow @eggshell")
                        .on_click(|state: &mut State| state.follows += 1);
                });
            });

        b.text(tw().font_semibold(), "Beside a tooltip and a popover");
        b.div(tw().flex().gap(4.0).items_center(), |b| {
            b.button("tip", "Tooltip")
                .tooltip("A tooltip")
                .on_click(|_: &mut State| {});
            b.button("pop", "Popover")
                .on_click(|state: &mut State| state.popover = !state.popover);
            if state.popover {
                let anchor = b.id_of("pop");
                b.overlay("popover")
                    .anchor(Anchor::Node(anchor))
                    .placement(Placement::below().gap(Distance::new(4.0)))
                    .on_dismiss(|state: &mut State| state.popover = false)
                    .children(|s| {
                        let classes = common::popup_box(s).w_px(256.0).p(4.0);
                        s.div(classes, |s| s.text(tw(), "A popover"));
                    });
            }
        });
    });
}

fn main() -> eframe::Result {
    common::run_app("hover card", State::default(), layout)
}
