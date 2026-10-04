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
    let SerialContext {
        ports,
        mut selected_id,
        mut is_open,
        request_port,
        refresh_ports,
        ..
    } = use_context::<SerialContext>();

    let mut sorted_ports = ports().values().cloned().collect::<Vec<_>>();
    sorted_ports.sort_by(|left, right| left.info.name.cmp(&right.info.name));
    let ports_for_list = sorted_ports.clone();
    let has_ports = !ports_for_list.is_empty();

    let selected_port = selected_id().and_then(|id| ports().get(&id).cloned());

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_SELECTOR_CSS }

        div {
            class: "port-selector",
            if let Some(action) = request_port() {
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| {
                        action();
                    },
                    lucide::CirclePlus {}
                }
            }
            DropdownMenu {
                disabled: !has_ports || is_open(),
                DropdownMenuTrigger {
                    class: "port-selector-trigger",
                    if let Some(port) = selected_port.clone() {
                        if is_open() {
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
                    for (index, port) in ports_for_list.into_iter().enumerate() {
                        DropdownMenuItem {
                            key: "{port.id}",
                            value: port.id,
                            index,
                            on_select: move |id| {
                                *selected_id.write() = Some(id);
                                *is_open.write() = false;
                            },
                            "{port.info.name}"
                        }
                    }
                }
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Ghost,
                onclick: move |_| {
                    if let Some(action) = refresh_ports() {
                        action();
                    }
                },
                lucide::RefreshCcw {}
            }
        }
    }
}
