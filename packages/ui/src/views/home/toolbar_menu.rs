use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    app::{AppTools, LicensesLink, tools_in_menu},
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        sheet::{Sheet, SheetContentClose, SheetHeader, SheetTitle},
    },
    elements::serial::{PortConfigurator, PortOpenCloseButton, PortSelector, PortSignalButton},
};

/// The toolbar in a menu from the left, for when it has no room: the port's
/// items, the app's tools and licenses with no navbar, and the tabs
/// (`children`) at its bottom. Its button is faint over the top left, shown
/// only when needed (by CSS).
#[component]
pub fn ToolbarMenu(children: Element) -> Element {
    let mut open = use_signal(|| false);
    let app_tools = tools_in_menu();

    rsx! {
        Button {
            class: "toolbar-menu-button",
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconSm,
            aria_label: "Menu",
            title: "Menu",
            onclick: move |_| open.set(true),
            lucide::Menu {}
        }
        Sheet {
            class: "toolbar-menu",
            "data-side": "left",
            open: open(),
            on_open_change: move |value| open.set(value),
            SheetHeader {
                SheetTitle { "Serval" }
            }
            SheetContentClose { aria_label: "Close" }
            div {
                class: "toolbar-menu-body",
                section {
                    class: "toolbar-menu-section",
                    aria_label: "Port",
                    h3 { class: "toolbar-menu-heading", "Port" }
                    PortSelector {}
                    PortConfigurator {}
                    div {
                        class: "toolbar-menu-row",
                        PortOpenCloseButton {}
                        PortSignalButton {}
                    }
                }
                if app_tools {
                    section {
                        class: "toolbar-menu-section",
                        aria_label: "App",
                        h3 { class: "toolbar-menu-heading", "App" }
                        div {
                            class: "toolbar-menu-row",
                            AppTools {}
                        }
                        // Opened, the menu gives way to it
                        div {
                            onclick: move |_| open.set(false),
                            LicensesLink { class: "toolbar-menu-link" }
                        }
                    }
                }
            }
            // Out of the scrolled part, so always at the bottom. A tab picked,
            // the menu gives way to it
            div {
                class: "toolbar-menu-footer",
                onclick: move |_| open.set(false),
                {children}
            }
        }
    }
}
