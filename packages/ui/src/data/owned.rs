//! Signals made for a provider's data (a map, a label), owned apart from
//! whichever component's event or effect made them.

use dioxus::{
    core::{Runtime, ScopeId, with_owner},
    signals::Owner,
};

/// Runs `make` in `scope` (a provider's, above everything that uses what it
/// makes), with the signals it makes owned by a new [`Owner`], given back
/// with what it made: they live until it is dropped.
///
/// Made where an event or effect runs, they would otherwise be owned by that
/// component, and dropped with it; made in another scope than the provider's,
/// Dioxus would warn of them being used above where they were made.
pub(crate) fn make_owned<T>(scope: ScopeId, make: impl FnOnce() -> T) -> (T, Owner) {
    let owner = Owner::default();
    let made = Runtime::current().in_scope(scope, || with_owner(owner.clone(), make));
    (made, owner)
}
