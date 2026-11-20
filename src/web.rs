//! The web build's keyboard: keeping the browser off the chords the
//! application's commands take.
//!
//! eframe 0.37 asks its `WebOptions::should_prevent_default` hook about
//! text, but never about a key press, so a browser shortcut on a chord a
//! command claims (`Cmd+G`, find next) runs as well as the command.
//! `prevent_claimed_chords` (wasm32 only) listens to the canvas's key
//! presses alongside eframe and prevents the browser's default for every
//! chord an enabled command claims, read from a [`ClaimedChords`] that each
//! pass refreshes.
//!
//! An app installs it where it makes its app, with the canvas it gave the
//! runner:
//!
//! ```no_run
//! # #[cfg(target_arch = "wasm32")]
//! # fn start(canvas: web_sys::HtmlCanvasElement, runner: eframe::WebRunner) {
//! # let _ = runner.start(canvas.clone(), eframe::WebOptions::default(), Box::new(move |cc| {
//! eggshell_ui::web::prevent_claimed_chords(&canvas, ClaimedChords::of(&cc.egui_ctx)).ok();
//! # Ok(Box::new(MyApp))
//! # }));
//! # }
//! ```

use std::sync::{Arc, Mutex};

use egui::{Context, Id, KeyboardShortcut};

use crate::commands::{is_clipboard_chord, presses};
use crate::sync::lock;

/// TODO(prose): the chords the enabled commands of a context's live tables
/// take, as the last pass left them. A cheap handle: clones share one
/// snapshot.
///
/// ```
/// use eggshell_ui::web::ClaimedChords;
/// use egui::{Key, KeyboardShortcut, Modifiers};
///
/// let ctx = egui::Context::default();
/// let claimed = ClaimedChords::of(&ctx);
/// assert!(!claimed.claims(KeyboardShortcut::new(Modifiers::COMMAND, Key::G)));
/// ```
#[derive(Clone, Default)]
pub struct ClaimedChords(Arc<Mutex<Snapshot>>);

#[derive(Default)]
struct Snapshot {
    pass: u64,
    chords: Vec<KeyboardShortcut>,
}

impl ClaimedChords {
    /// TODO(prose): the handle of `ctx`'s claimed chords.
    pub fn of(ctx: &Context) -> Self {
        ctx.data_mut(|data| {
            data.get_temp_mut_or_default::<Self>(Id::NULL.with("eggshell-claimed-chords"))
                .clone()
        })
    }

    /// TODO(prose): whether pressing `pressed` runs a command, so the
    /// browser's own action for it should be prevented. Cut, copy and paste
    /// are never claimed: the browser sends them as clipboard events only if
    /// their key press goes through.
    pub fn claims(&self, pressed: KeyboardShortcut) -> bool {
        lock(&self.0).chords.iter().any(|&chord| {
            !is_clipboard_chord(chord) && presses(chord, pressed.logical_key, pressed.modifiers)
        })
    }

    /// Adds the chords a layout's draw found armed. Every layout drawn in a
    /// pass adds to that pass's snapshot; the first to draw in a new pass
    /// starts it over.
    pub(crate) fn record(&self, ctx: &Context, chords: Vec<KeyboardShortcut>) {
        let pass = ctx.cumulative_pass_nr();
        let mut snapshot = lock(&self.0);
        if snapshot.pass == pass {
            snapshot.chords.extend(chords);
        } else {
            *snapshot = Snapshot { pass, chords };
        }
    }
}

/// A browser `keydown` as egui sees it.
///
/// This mirrors eframe 0.37 (`eframe-0.37.0/src/web/`): `input.rs`'s
/// `modifiers_from_kb_event` and `translate_key`, and `events.rs`'s
/// `on_keydown`, which falls back from the logical key to the physical one.
/// The chord here must be the `egui::Event::Key` eframe sends for the same
/// event, or a command would run while the browser acts too; check it again
/// when eframe is upgraded.
#[cfg(any(target_arch = "wasm32", test))]
struct KeyDown<'e> {
    key: &'e str,
    code: &'e str,
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
}

#[cfg(any(target_arch = "wasm32", test))]
impl KeyDown<'_> {
    fn chord(&self) -> Option<KeyboardShortcut> {
        let key = egui::Key::from_name(self.key).or_else(|| egui::Key::from_name(self.code))?;
        let modifiers = egui::Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: self.meta,
            command: self.ctrl || self.meta,
        };
        Some(KeyboardShortcut::new(modifiers, key))
    }
}

/// TODO(prose): prevents the browser's own action for a key press on
/// `canvas` that runs a command, as `claimed` says. Install it once, after
/// the runner has the canvas; the listener lives as long as the page.
///
/// It listens on the canvas's parent, before eframe does, for a press on the
/// canvas or on the text input eframe places right after it while a text
/// field has the keyboard. It never stops the press reaching eframe.
#[cfg(target_arch = "wasm32")]
pub fn prevent_claimed_chords(
    canvas: &web_sys::HtmlCanvasElement,
    claimed: ClaimedChords,
) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::closure::Closure;

    let canvas = canvas.clone();
    let Some(parent) = canvas.parent_node() else {
        return Err(wasm_bindgen::JsValue::from_str(
            "the canvas has no parent to listen on",
        ));
    };
    let on_keydown =
        Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            let Some(target) = event.target() else {
                return;
            };
            let on_canvas = &target == AsRef::<web_sys::EventTarget>::as_ref(&canvas);
            let on_text_agent = canvas.next_sibling().is_some_and(|text_agent| {
                &target == AsRef::<web_sys::EventTarget>::as_ref(&text_agent)
            });
            if !(on_canvas || on_text_agent) {
                return;
            }
            let down = KeyDown {
                key: &event.key(),
                code: &event.code(),
                alt: event.alt_key(),
                ctrl: event.ctrl_key(),
                shift: event.shift_key(),
                meta: event.meta_key(),
            };
            if down.chord().is_some_and(|pressed| claimed.claims(pressed)) {
                event.prevent_default();
            }
        });
    parent.add_event_listener_with_callback_and_bool(
        "keydown",
        on_keydown.as_ref().unchecked_ref(),
        true,
    )?;
    // The listener is the page's for good: dropping the closure would detach it.
    on_keydown.forget();
    Ok(())
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};

    use super::*;

    fn down<'e>(key: &'e str, code: &'e str, [alt, ctrl, shift, meta]: [bool; 4]) -> KeyDown<'e> {
        KeyDown {
            key,
            code,
            alt,
            ctrl,
            shift,
            meta,
        }
    }

    const META: [bool; 4] = [false, false, false, true];
    const CTRL_ALT: [bool; 4] = [true, true, false, false];
    const CTRL_SHIFT: [bool; 4] = [false, true, true, false];

    fn alt_command() -> Modifiers {
        Modifiers::COMMAND | Modifiers::ALT
    }

    #[test]
    fn cmd_g_is_command_g() {
        let chord = down("g", "KeyG", META).chord();
        assert_eq!(
            chord,
            Some(KeyboardShortcut::new(
                Modifiers {
                    mac_cmd: true,
                    command: true,
                    ..Modifiers::NONE
                },
                Key::G
            ))
        );
    }

    #[test]
    fn ctrl_alt_g_is_ctrl_alt_g() {
        let chord = down("g", "KeyG", CTRL_ALT).chord();
        assert_eq!(
            chord,
            Some(KeyboardShortcut::new(
                Modifiers {
                    alt: true,
                    ctrl: true,
                    command: true,
                    ..Modifiers::NONE
                },
                Key::G
            ))
        );
    }

    #[test]
    fn a_key_alt_turns_into_a_symbol_falls_back_to_its_code() {
        let chord = down("©", "KeyG", [true, false, false, true]).chord();
        assert_eq!(chord.map(|chord| chord.logical_key), Some(Key::G));
    }

    #[test]
    fn a_shifted_key_is_the_key_it_types() {
        let bang = down("!", "Digit1", CTRL_SHIFT).chord();
        assert_eq!(
            bang.map(|chord| chord.logical_key),
            Some(Key::Exclamationmark)
        );
        let upper = down("G", "KeyG", CTRL_SHIFT).chord();
        assert_eq!(upper.map(|chord| chord.logical_key), Some(Key::G));
        let brace = down("{", "BracketLeft", [false, false, true, true]).chord();
        assert_eq!(
            brace.map(|chord| chord.logical_key),
            Some(Key::OpenCurlyBracket)
        );
    }

    #[test]
    fn a_key_egui_does_not_know_is_no_chord() {
        assert_eq!(down("Dead", "IntlRo", META).chord(), None);
    }

    fn claimed(chords: Vec<KeyboardShortcut>) -> ClaimedChords {
        let ctx = Context::default();
        let claimed = ClaimedChords::of(&ctx);
        claimed.record(&ctx, chords);
        claimed
    }

    #[test]
    fn a_browser_press_of_a_claimed_chord_is_claimed_on_either_platform() {
        let claimed = claimed(vec![KeyboardShortcut::new(alt_command(), Key::G)]);
        for held in [CTRL_ALT, [true, false, false, true]] {
            let pressed = down("g", "KeyG", held).chord();
            assert!(
                pressed.is_some_and(|pressed| claimed.claims(pressed)),
                "{held:?}"
            );
        }
        let plain = down("g", "KeyG", META).chord();
        assert!(plain.is_some_and(|pressed| !claimed.claims(pressed)));
    }

    #[test]
    fn cut_copy_and_paste_are_never_claimed() {
        let claimed = claimed(
            [Key::X, Key::C, Key::V]
                .map(|key| KeyboardShortcut::new(Modifiers::COMMAND, key))
                .into(),
        );
        for key in ["x", "c", "v"] {
            let pressed = down(key, "", META).chord();
            assert!(
                pressed.is_some_and(|pressed| !claimed.claims(pressed)),
                "{key}"
            );
        }
    }

    #[test]
    fn a_new_pass_starts_the_snapshot_over_and_layouts_of_one_pass_add_up() {
        let ctx = Context::default();
        let claimed = ClaimedChords::of(&ctx);
        let g = KeyboardShortcut::new(Modifiers::COMMAND, Key::G);
        let k = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
        claimed.record(&ctx, vec![g]);
        claimed.record(&ctx, vec![k]);
        assert!(claimed.claims(g) && claimed.claims(k));
        ctx.run_ui(Default::default(), |_| {})
            .drop_without_applying_deltas();
        claimed.record(&ctx, vec![k]);
        assert!(!claimed.claims(g) && claimed.claims(k));
    }
}
