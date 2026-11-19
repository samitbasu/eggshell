#![cfg_attr(
    not(test),
    deny(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented,
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        clippy::get_unwrap,
        clippy::string_slice,
        clippy::indexing_slicing,
        clippy::unchecked_time_subtraction,
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        clippy::fn_params_excessive_bools
    )
)]
//! TODO(prose)

#[cfg(feature = "badge")]
pub mod badge;
#[cfg(feature = "form")]
pub mod form;
#[cfg(feature = "hover-card")]
pub mod hover_card;
#[cfg(feature = "markdown")]
pub mod markdown;
#[cfg(feature = "spinner")]
pub mod spinner;
#[cfg(any(
    feature = "badge",
    feature = "form",
    feature = "markdown",
    feature = "spinner"
))]
mod styles;
