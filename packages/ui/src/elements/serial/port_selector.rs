use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    },
    serial::SerialContext,
};

const PORT_SELECTOR_CSS: Asset = asset!("/assets/styling/port-selector.css");

#[component]
pub fn PortSelector() -> Element {
    let serial = use_context::<SerialContext>();

    let mut ports = serial.ports();
    ports.sort_by(|left, right| left.info.name.cmp(&right.info.name));
    let has_ports = !ports.is_empty();
    let is_open = serial.is_open();
    let selected_port = serial.selected_port();

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_SELECTOR_CSS }

        div {
            class: "port-selector",
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Ghost,
                onclick: move |_| serial.request_port(),
                lucide::CirclePlus {}
            }
            DropdownMenu {
                disabled: !has_ports || is_open,
                DropdownMenuTrigger {
                    class: "port-selector-trigger",
                    if let Some(port) = selected_port {
                        if is_open {
                            span {
                                class: "port-selector-name",
                                span { class: "port-selector-spinner" }
                                "{port.info.name}"
                            }
                        } else {
                            span {
                                class: "port-selector-name",
                                lucide::Unplug {}
                                "{port.info.name}"
                            }
                        }
                    } else if !has_ports {
                        "No Devices available"
                    } else {
                        "No Device selected"
                    }
                }
                DropdownMenuContent {
                    for (index, port) in ports.into_iter().enumerate() {
                        DropdownMenuItem {
                            key: "{port.id}",
                            value: port.id,
                            index,
                            on_select: move |id| serial.select(id),
                            "{port.info.name}"
                        }
                    }
                }
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Ghost,
                onclick: move |_| serial.refresh_ports(),
                lucide::RefreshCcw {}
            }
        }
    }
}
