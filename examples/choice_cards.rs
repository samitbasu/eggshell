//! Choice cards: radio options, a checkbox and a switch, each a card whose
//! label holds the control after a title and a description, and which a
//! click anywhere on chooses or toggles.

use eggshell_examples::common;

use eggshell_ui::owned::{Enablement, choice::Choice};
use eggshell_ui::{Builder, tw};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compute {
    Kubernetes,
    Machine,
    Bare,
}

pub struct State {
    pub compute: Compute,
    pub backups: bool,
    pub shared: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            compute: Compute::Kubernetes,
            backups: false,
            shared: true,
        }
    }
}

/// ```html
/// <div class="flex flex-col gap-6 p-4 w-[380px]">
///   <div role="radiogroup" class="grid gap-3">
///     <label data-slot="field-label">
///       <div data-slot="field" data-orientation="horizontal">
///         <div data-slot="field-content">
///           <div data-slot="field-title">Kubernetes</div>
///           <p data-slot="field-description">Run GPU workloads on a K8s cluster.</p>
///         </div>
///         <button role="radio" aria-checked="true"></button>
///       </div>
///     </label> …
///   </div>
///   <label data-slot="field-label">… <button role="checkbox"></button></label>
///   <label data-slot="field-label">… <button role="switch" aria-checked="true"></button></label>
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    b.div(tw().flex().flex_col().gap(6.0).p(4.0).w_px(380.0), |b| {
        b.radio_group(
            "compute",
            state.compute,
            [
                Choice::new(Compute::Kubernetes, "Kubernetes")
                    .description("Run GPU workloads on a K8s configured cluster."),
                Choice::new(Compute::Machine, "Virtual Machine")
                    .description("Access a VM configured cluster to run workloads."),
                Choice::new(Compute::Bare, "Bare metal")
                    .description("Not available in this region.")
                    .enablement(Enablement::Disabled),
            ],
        )
        .cards()
        .on_change(|state: &mut State, compute| state.compute = compute);
        b.checkbox("backups", "Nightly backups", state.backups)
            .card()
            .description("Keep a snapshot of every volume for seven days.")
            .on_change(|state: &mut State, backups| state.backups = backups);
        b.switch("shared", "Share across devices", state.shared)
            .card()
            .description("Focus is shared across devices, and turns off when you leave the app.")
            .on_change(|state: &mut State, shared| state.shared = shared);
    });
}

fn main() -> eframe::Result {
    common::run_app("choice_cards", State::default(), layout)
}
