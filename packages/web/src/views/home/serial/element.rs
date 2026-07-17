use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCirclePlus, LdPause, LdPlay, LdRefreshCcw, LdUnplug},
    Icon,
};
use wasm_bindgen::closure::Closure;

use super::web_serial_api::{
    close_port_js, get_ports_js, open_port_js, request_port_js, start_read_loop_js, write_port_js,
    SerialContext,
};

const BAUDRATE_PRESETS: [u32; 4] = [9600_u32, 19200, 57600, 115200];

#[component]
pub fn DeviceSelector() -> Element {
    let SerialContext {
        mut ports,
        mut selected_index,
        mut active_index,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        div {
                class: "join",
            button {
                class: "btn btn-sm join-item",
                onclick: move |_| {
                    wasm_bindgen_futures::spawn_local(async move {
                        match request_port_js().await {
                            Ok(port) => {
                                let mut current_ports = ports();
                                current_ports.push(port);
                                let new_index = current_ports.len().saturating_sub(1);
                                *ports.write() = current_ports;
                                *selected_index.write() = Some(new_index);
                                *active_index.write() = None;
                                *status_message.write() = "New port added and selected.".to_string();
                            }
                            Err(err) => {
                                *status_message.write() = format!("Port request failed: {:?}", err);
                            }
                        }
                    });
                },
                Icon {
                    icon: LdCirclePlus {},
                }
            }
            button {
                class: "btn btn-sm join-item",
                popovertarget: "device-selector-dropdown",
                style: "anchor-name:--anchor-device-selector-dropdown;",
                disabled: ports().is_empty() || active_index().is_some(),
                if !ports().is_empty() {
                    if let Some(index) = selected_index() {
                        if let Some(port) = ports().get(index) {
                            if active_index().is_some() {
                                span {
                                    class: "flex gap-3",
                                    span { class: "loading loading-spinner loading-sm" }
                                    "{port.title()}"
                                }
                            } else {
                                span {
                                    class: "flex gap-3",
                                    Icon {
                                        icon: LdUnplug {},
                                    }
                                    "{port.title()}"
                                }
                            }
                        } else {
                            "No Device selected"
                        }
                    } else {
                        "No Device selected"
                    }
                } else {
                    "No Devices available"
                }
            }
            ul {
                class: "dropdown menu rounded-box shadow-sm bg-base-300 w-52",
                popover: true,
                id: "device-selector-dropdown",
                style: "position-anchor:--anchor-device-selector-dropdown;",
                {
                    ports().into_iter().enumerate().map(|(index, port)| {
                        rsx!(
                            li {
                                onclick: move |_| {
                                    *selected_index.write() = Some(index);
                                    *active_index.write() = None;
                                    *status_message.write() = format!("Selected {}.", port.title());
                                },
                                a { "{port.title()}" }
                            }
                        )
                    })
                }
            }
            button {
                class: "btn btn-sm join-item",
                onclick: move |_| {
                    wasm_bindgen_futures::spawn_local(async move {
                        match get_ports_js().await {
                            Ok(list) => {
                                let new_selected_index = if list.is_empty() { None } else { Some(selected_index().unwrap_or(0).min(list.len().saturating_sub(1))) };
                                *ports.write() = list;
                                *selected_index.write() = new_selected_index;
                                *active_index.write() = None;
                                *status_message.write() = "Port list refreshed.".to_string();
                            }
                            Err(err) => {
                                *status_message.write() = format!("Failed to refresh ports: {:?}", err);
                            }
                        }
                    });
                },
                Icon {
                    icon: LdRefreshCcw {},
                }
            }
        }
    }
}

#[component]
pub fn BaudrateConfigurator() -> Element {
    let mut baudrate = use_context::<SerialContext>().baudrate;

    rsx! {
        label {
            class: "input w-64",
        input {
            class: "input input-sm",
            type: "number",
            min: "1",
            step: "1",
            placeholder: "Baudrate (e.g. 9600)",
            list: "baudrate-presets",
            oninput: move |event| {
                *baudrate.write() = parse_baudrate(&event.value()).unwrap_or(BAUDRATE_PRESETS[0]);
            }
        }
        span { class: "label", "bps" }
        }
        datalist {
            id: "baudrate-presets",
            {BAUDRATE_PRESETS.iter().map(|baudrate_preset| {
                rsx!(
                    option {
                        value: "{baudrate_preset}"
                    }
                )
            })}
        }
    }
}

#[component]
pub fn OpenCloseButton() -> Element {
    let SerialContext {
        ports,
        selected_index,
        mut active_index,
        baudrate,
        mut status_message,
        received_text,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        label {
            class: "btn btn-circle btn-sm swap swap-rotate",
            input {
                type: "checkbox",
                disabled: selected_index().is_none(),
                onclick: move |_| {
                    let will_open = active_index().is_none();

                    if let Some(index) = selected_index() {
                        if let Some(entry) = ports().get(index).cloned() {
                            *status_message.write() = if will_open {
                                format!("Opening {} at {} bps...", entry.title(), baudrate())
                            } else {
                                format!("Closing {}...", entry.title())
                            };

                            wasm_bindgen_futures::spawn_local(async move {
                                if will_open {
                                    match open_port_js(entry.port(), baudrate()).await {
                                        Ok(()) => {
                                            *active_index.write() = selected_index();
                                            *status_message.write() = format!("Opened {} at {} bps.", entry.title(), baudrate());

                                            let read_port = entry.port().clone();
                                            let received_text = received_text;
                                            let status_message = status_message;
                                            wasm_bindgen_futures::spawn_local(async move {
                                                let mut received_text = received_text;
                                                let mut status_message = status_message;
                                                let on_chunk = Closure::wrap(Box::new(move |chunk: String| {
                                                    received_text.write().push_str(&chunk);
                                                }) as Box<dyn FnMut(String)>);

                                                if let Err(err) = start_read_loop_js(&read_port, &on_chunk).await {
                                                    *status_message.write() = format!("Read loop ended: {:?}", err);
                                                }

                                                drop(on_chunk);
                                            });
                                        }
                                        Err(err) => {
                                            *status_message.write() = format!("Failed to open port: {:?}", err);
                                        }
                                    }
                                } else {
                                    match close_port_js(entry.port()).await {
                                        Ok(()) => {
                                            *active_index.write() = None;
                                            *status_message.write() = format!("Closed {}.", entry.title());
                                        }
                                        Err(err) => {
                                            *status_message.write() = format!("Failed to close port: {:?}", err);
                                        }
                                    }
                                }
                            });
                        } else {
                            *status_message.write() = "Selected port is no longer available.".to_string();
                        }
                    } else {
                        *status_message.write() = "Select a port first.".to_string();
                    }
                }
            }
            Icon {
                class: "swap-on text-error",
                icon: LdPause {},
            }
            if selected_index().is_some() {
                Icon {
                    class: "swap-off text-success",
                    icon: LdPlay {},
                }
            } else {
                Icon {
                    class: "swap-off text-base-content/60",
                    icon: LdPlay {},
                }
            }
        }
    }
}

#[component]
pub fn PortWritePanel() -> Element {
    let SerialContext {
        ports,
        active_index,
        mut outgoing_text,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        div {
            class: "card bg-base-100 shadow-xl",
            div {
                class: "card-body gap-4",
                h2 { class: "card-title", "Send Data" }
                textarea {
                    class: "textarea textarea-bordered min-h-36 w-full",
                    placeholder: "Type text to send to the active port",
                    value: "{outgoing_text()}",
                    oninput: move |event| {
                        *outgoing_text.write() = event.value();
                    }
                }
                div {
                    class: "flex flex-wrap gap-3",
                    button {
                        class: "btn btn-primary",
                        disabled: active_index().is_none() || outgoing_text().trim().is_empty(),
                        onclick: move |_| {
                            let selected_entry = active_index().and_then(|index| ports().get(index).cloned());
                            let text = outgoing_text().clone();

                            if let Some(entry) = selected_entry {
                                *status_message.write() = format!("Sending {} bytes...", text.len());
                                wasm_bindgen_futures::spawn_local(async move {
                                    let mut outgoing_text = outgoing_text;
                                    let mut status_message = status_message;
                                    match write_port_js(entry.port(), &text).await {
                                        Ok(()) => {
                                            *outgoing_text.write() = String::new();
                                            *status_message.write() = format!("Sent {} bytes to {}.", text.len(), entry.title());
                                        }
                                        Err(err) => {
                                            *status_message.write() = format!("Failed to send data: {:?}", err);
                                        }
                                    }
                                });
                            } else {
                                *status_message.write() = "Open a port before sending data.".to_string();
                            }
                        },
                        "Send"
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            *outgoing_text.write() = String::new();
                            *status_message.write() = "Cleared outgoing buffer.".to_string();
                        },
                        "Clear"
                    }
                }
            }
        }
    }
}

fn parse_baudrate(value: &str) -> Option<u32> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|baudrate| *baudrate > 0)
}
