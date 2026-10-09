//! Moving the focus in a submenu: a `[role=menu]` of `[role=menuitem]`s in a
//! dropdown item (`[role=option]`), opened from it by the keyboard.

/// From the item into its submenu.
pub(crate) const FOCUS_FIRST: &str =
    "document.activeElement?.querySelector('[role=menu] [role=menuitem]')?.focus()";
pub(crate) const FOCUS_NEXT: &str = "document.activeElement?.nextElementSibling?.focus()";
pub(crate) const FOCUS_PREVIOUS: &str = "document.activeElement?.previousElementSibling?.focus()";
/// Back to the item the submenu is in.
pub(crate) const FOCUS_ITEM: &str = "document.activeElement?.closest('[role=option]')?.focus()";
