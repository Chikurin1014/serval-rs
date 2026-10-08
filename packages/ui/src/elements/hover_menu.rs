//! Dropdown menus in a bar that open on hover.

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuTrigger};

const HOVER_MENU_CSS: Asset = asset!("/assets/styling/hover-menu.css");

#[derive(Clone, Copy, PartialEq)]
pub struct HoverMenus {
    open: Signal<Option<usize>>,
    hovered: Signal<Option<usize>>,
}

impl HoverMenus {
    pub fn new() -> Self {
        Self {
            open: Signal::new(None),
            hovered: Signal::new(None),
        }
    }

    pub fn is_open(&self, menu: usize) -> bool {
        (self.open)() == Some(menu)
    }

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

/// Menu number `menu` of `menus`' bar; open while hovered.
#[component]
pub fn HoverMenu(
    menus: HoverMenus,
    menu: usize,
    trigger: Element,
    children: Element,
    #[props(default)] disabled: bool,
    #[props(into, default)] content_class: String,
    #[props(default)] keep_open: ReadSignal<bool>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
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
