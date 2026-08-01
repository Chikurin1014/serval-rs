use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCirclePlus, LdRefreshCcw, LdUnplug},
    Icon,
};

use crate::serial::SerialContext;

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
        div {
            class: "join",
            button {
                class: "btn btn-sm join-item",
                onclick: move |_| {
                    if let Some(action) = request_port() {
                        action();
                    }
                },
                Icon {
                    icon: LdCirclePlus {},
                }
            }
            button {
                class: "btn btn-sm join-item",
                popovertarget: "device-selector-dropdown",
                style: "anchor-name:--anchor-device-selector-dropdown;",
                disabled: !has_ports || is_open(),
                if let Some(port) = selected_port.clone() {
                    if is_open() {
                        span {
                            class: "flex gap-3",
                            span { class: "loading loading-spinner loading-sm" }
                            "{port.info.name}"
                        }
                    } else {
                        span {
                            class: "flex gap-3",
                            Icon {
                                icon: LdUnplug {},
                            }
                            "{port.info.name}"
                        }
                    }
                } else if !has_ports {
                    "No Devices available"
                } else {
                    "No Device selected"
                }
            }
            ul {
                class: "dropdown menu rounded-box shadow-sm bg-base-300 w-52",
                popover: true,
                id: "device-selector-dropdown",
                style: "position-anchor:--anchor-device-selector-dropdown;",
                {
                    ports_for_list.into_iter().map(|port| {
                        let id = port.id;
                        let name = port.info.name.clone();

                        rsx!(
                            li {
                                onclick: move |_| {
                                    *selected_id.write() = Some(id);
                                    *is_open.write() = false;
                                },
                                a { "{name}" }
                            }
                        )
                    })
                }
            }
            button {
                class: "btn btn-sm join-item",
                onclick: move |_| {
                    if let Some(action) = refresh_ports() {
                        action();
                    }
                },
                Icon {
                    icon: LdRefreshCcw {},
                }
            }
        }
    }
}
