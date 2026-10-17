//! Submenus: a `[role=menu]` of `[role=menuitem]`s beside a dropdown item
//! (`[role=option]`), or beside an item of another submenu; opened from it by
//! the keyboard, and moved through by it.

use dioxus::prelude::*;

/// From the item into its submenu.
const FOCUS_FIRST: &str =
    "document.activeElement?.querySelector('[role=menu] [role=menuitem]')?.focus()";

/// From a submenu's item into the submenu beside it, if it has one.
const FOCUS_NESTED: &str = r#"document.activeElement?.parentElement
  ?.querySelector(":scope > [data-submenu] [role=menuitem]")?.focus();"#;

/// Back to what the focused submenu is beside: the menu's item, or another
/// submenu's.
const FOCUS_OWNER: &str = r#"const owner = document.activeElement?.closest("[data-submenu]")?.parentElement;
(owner?.matches("[role=option]") ? owner : owner?.querySelector(":scope > [role=menuitem]"))?.focus();"#;

/// In a submenu: the item `step` from the focused one, of its own (not those of
/// a submenu in it).
fn focus_sibling(step: i32) -> String {
    format!(
        r#"const current = document.activeElement;
const menu = current?.closest("[role=menu]");
if (menu) {{
  const items = [...menu.querySelectorAll("[role=menuitem]")]
    .filter((item) => item.closest("[role=menu]") === menu);
  items[items.indexOf(current) + ({step})]?.focus();
}}"#
    )
}

/// For a dropdown menu's keys: into the focused item's submenu, if it has one.
pub(crate) fn open_submenu(event: KeyboardEvent) {
    if event.key() == Key::ArrowRight {
        event.prevent_default();
        document::eval(FOCUS_FIRST);
    }
}

/// A submenu of `children`, named `label`: `focused` while one of them is, as
/// the menu it is in is to stay open meanwhile.
#[component]
pub(crate) fn Submenu(
    #[props(into)] class: String,
    #[props(into)] list_class: String,
    label: String,
    focused: Signal<bool>,
    children: Element,
) -> Element {
    let mut focused = focused;

    rsx! {
        div {
            class,
            "data-submenu": true,
            onfocusin: move |_| focused.set(true),
            onfocusout: move |_| focused.set(false),
            onkeydown: move |event: KeyboardEvent| {
                let script = match event.key() {
                    Key::ArrowDown => focus_sibling(1),
                    Key::ArrowUp => focus_sibling(-1),
                    Key::ArrowRight => FOCUS_NESTED.to_string(),
                    Key::ArrowLeft | Key::Escape => FOCUS_OWNER.to_string(),
                    Key::Enter => {
                        event.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                event.prevent_default();
                // Not for the submenu (or menu) it is in
                event.stop_propagation();
                document::eval(&script);
            },
            div {
                class: list_class,
                role: "menu",
                aria_label: label,
                {children}
            }
        }
    }
}
