//! Dropdown menus that open while the pointer is over them, side by side in a
//! bar (the maps' and the graphs' add menus).

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuTrigger};

const HOVER_MENU_CSS: Asset = asset!("/assets/styling/hover-menu.css");

/// The menus of one bar: which one is open, and which one the pointer is over.
#[derive(Clone, Copy, PartialEq)]
pub struct HoverMenus {
    open: Signal<Option<usize>>,
    hovered: Signal<Option<usize>>,
}

impl HoverMenus {
    /// All closed; call in the bar's component, as it makes signals.
    pub fn new() -> Self {
        Self {
            open: Signal::new(None),
            hovered: Signal::new(None),
        }
    }

    pub fn is_open(&self, menu: usize) -> bool {
        (self.open)() == Some(menu)
    }

    /// Closes `menu`, if it is the one open.
    pub fn close(mut self, menu: usize) {
        if *self.open.peek() == Some(menu) {
            self.open.set(None);
        }
    }
}

impl Default for HoverMenus {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu number `menu` of `menus`' bar: `trigger`, with an arrow showing whether
/// it is open, over `children` (`DropdownMenuItem`s), which may close it with
/// [`HoverMenus::close`]. It opens on hover, and stays open while hovered,
/// even when its trigger is clicked.
#[component]
pub fn HoverMenu(
    menus: HoverMenus,
    menu: usize,
    trigger: Element,
    children: Element,
    #[props(default)] disabled: bool,
    /// Class of the content, for its items' layout.
    #[props(into, default)]
    content_class: String,
    /// Keeps it open while true, e.g. while focus is in a panel by its items.
    #[props(default)]
    keep_open: ReadSignal<bool>,
    /// Keys pressed in it, past what the menu itself does with them.
    #[props(default)]
    onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let HoverMenus {
        mut open,
        mut hovered,
    } = menus;
    let is_open = menus.is_open(menu);

    rsx! {
        document::Link { rel: "stylesheet", href: HOVER_MENU_CSS }

        div {
            class: "hover-menu",
            onmouseenter: move |_| {
                hovered.set(Some(menu));
                if !disabled {
                    open.set(Some(menu));
                }
            },
            onmouseleave: move |_| {
                hovered.set(None);
                menus.close(menu);
            },
            onkeydown: move |event| onkeydown.call(event),
            DropdownMenu {
                open: Some(is_open),
                on_open_change: move |value| {
                    if value {
                        open.set(Some(menu));
                    } else if hovered() != Some(menu) && !keep_open() {
                        menus.close(menu);
                    }
                },
                disabled,
                DropdownMenuTrigger {
                    class: "hover-menu-trigger",
                    {trigger}
                    if is_open {
                        lucide::ChevronUp {}
                    } else {
                        lucide::ChevronDown {}
                    }
                }
                DropdownMenuContent {
                    class: "hover-menu-content {content_class}",
                    {children}
                }
            }
        }
    }
}
