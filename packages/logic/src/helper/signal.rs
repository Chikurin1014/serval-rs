use dioxus::{
    core::{Runtime, ScopeId, with_owner},
    prelude::*,
    signals::Owner,
};

/// Sets `signal` only if the value differs, so readers do not run for nothing.
pub fn set_if_changed<T: PartialEq + 'static>(signal: &mut Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}

/// Runs `make` in `scope` with a new [`Owner`] for the signals it makes, rather
/// than the component whose event or effect calls this.
pub(crate) fn make_owned<T>(scope: ScopeId, make: impl FnOnce() -> T) -> (T, Owner) {
    let owner = Owner::default();
    let made = Runtime::current().in_scope(scope, || with_owner(owner.clone(), make));
    (made, owner)
}
