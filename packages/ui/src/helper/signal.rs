//! Wrappers around Dioxus signals and their owners.

use dioxus::{
    core::{Runtime, ScopeId, with_owner},
    prelude::*,
    signals::Owner,
};

/// Sets `signal` only if the value differs, so what reads it (e.g. a form
/// showing it) does not run again for an unchanged value.
pub fn set_if_changed<T: PartialEq + 'static>(signal: &mut Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}

/// Runs `make` in `scope` (e.g. a provider's, above everything that uses what
/// it makes), with the signals it makes owned by a new [`Owner`], given back
/// with what it made: they live until it is dropped.
///
/// Made where an event or effect runs, they would otherwise be owned by that
/// component, and dropped with it; made in a scope below where they are used,
/// Dioxus would warn of it.
pub(crate) fn make_owned<T>(scope: ScopeId, make: impl FnOnce() -> T) -> (T, Owner) {
    let owner = Owner::default();
    let made = Runtime::current().in_scope(scope, || with_owner(owner.clone(), make));
    (made, owner)
}
