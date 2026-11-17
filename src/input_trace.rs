//! Pointer presses and releases on text that opens a field, traced at debug
//! level on target `eggshell::input`. Costs one `log_enabled!` check when the
//! target is off.

use egui::{Context, Event, Id, Pos2, Rect, Response, Ui};

const TARGET: &str = "eggshell::input";

/// Which of a pair is drawn this frame: the text, or the field replacing it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Shown {
    Text,
    Field,
}

/// The last release and press seen, as of the start of a pass and as of now.
#[derive(Clone, Copy, Default)]
struct Marks {
    release_time: Option<f64>,
    press_pos: Option<Pos2>,
}

#[derive(Clone, Copy, Default)]
struct History {
    pass: u64,
    before: Marks,
    now: Marks,
}

#[derive(Clone, Copy, Default)]
struct Rects {
    text: Option<Rect>,
    field: Option<Rect>,
}

pub(crate) fn enabled() -> bool {
    log::log_enabled!(target: TARGET, log::Level::Debug)
}

/// Logs each press and release this frame near `response` (anywhere, for a
/// field), with what `key`'s pair decided.
pub(crate) fn pointer(
    ui: &Ui,
    key: Id,
    name: &str,
    shown: Shown,
    response: &Response,
    decision: &str,
) {
    if !enabled() {
        return;
    }
    let ctx = ui.ctx();
    let before = history(ctx);
    let rects = remember_rect(ctx, key, shown, response.rect);
    let max_dist = ctx.options(|options| options.input_options.max_click_dist);
    let (time, events, double, triple) = ui.input(|input| {
        (
            input.time,
            buttons(input),
            input
                .pointer
                .button_double_clicked(egui::PointerButton::Primary),
            input
                .pointer
                .button_triple_clicked(egui::PointerButton::Primary),
        )
    });
    let near = response.rect.expand(4.0 * max_dist);
    let focused = ctx
        .memory(|memory| memory.focused())
        .map(|id| id.short_debug_format());
    for (pos, pressed) in events {
        if shown == Shown::Text && !near.contains(pos) {
            continue;
        }
        let since_release = before
            .release_time
            .map(|at| format!("{:.0} ms", (time - at) * 1000.0));
        let from_press = before
            .press_pos
            .map(|at| format!("{:.1}", at.distance(pos)));
        log::debug!(
            target: TARGET,
            "{name:?} {} {shown:?} at {pos:?} t={time:.3} | since last release {} | from last press {} (max {max_dist}) | egui double {double} triple {triple} | {} clicked {} double {} hovered {} down-on {} | focused {focused:?} | text {:?} hit {} | field {:?} hit {} | {decision}",
            if pressed { "press" } else { "release" },
            since_release.as_deref().unwrap_or("-"),
            from_press.as_deref().unwrap_or("-"),
            response.id.short_debug_format(),
            response.clicked(),
            response.double_clicked(),
            response.hovered(),
            response.is_pointer_button_down_on(),
            rects.text,
            rects.text.is_some_and(|rect| rect.contains(pos)),
            rects.field,
            rects.field.is_some_and(|rect| rect.contains(pos)),
        );
    }
}

/// The marks as of the start of this pass; this pass's events are recorded
/// for the next.
fn history(ctx: &Context) -> Marks {
    let pass = ctx.cumulative_pass_nr();
    let (time, buttons) = ctx.input(|input| (input.time, buttons(input)));
    let history_id = crate::key::viewport_id(ctx, TARGET);
    ctx.data_mut(|data| {
        let history = data.get_temp_mut_or_default::<History>(history_id);
        if history.pass != pass {
            history.before = history.now;
            history.pass = pass;
            for (pos, pressed) in buttons {
                if pressed {
                    history.now.press_pos = Some(pos);
                } else {
                    history.now.release_time = Some(time);
                }
            }
        }
        history.before
    })
}

fn buttons(input: &egui::InputState) -> Vec<(Pos2, bool)> {
    input
        .events
        .iter()
        .filter_map(|event| match event {
            Event::PointerButton { pos, pressed, .. } => Some((*pos, *pressed)),
            _ => None,
        })
        .collect()
}

fn remember_rect(ctx: &Context, key: Id, shown: Shown, rect: Rect) -> Rects {
    ctx.data_mut(|data| {
        let rects = data.get_temp_mut_or_default::<Rects>(key.with(TARGET));
        match shown {
            Shown::Text => rects.text = Some(rect),
            Shown::Field => rects.field = Some(rect),
        }
        *rects
    })
}
