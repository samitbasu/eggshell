//! Locking that survives a panic on another thread.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// The guard of `mutex`, taken back from the poison a panic while it was held
/// leaves: the data it guards is the library's own state, which every writer
/// leaves consistent between statements.
#[expect(
    clippy::disallowed_methods,
    reason = "the one place a mutex is locked, recovering from poison"
)]
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
