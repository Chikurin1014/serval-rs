use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCirclePlus, LdRefreshCcw, LdUsb},
    Icon,
};
use wasm_bindgen::closure::Closure;

use super::web_serial_api::{
    close_port_js, get_ports_js, open_port_js, request_port_js, start_read_loop_js, write_port_js,
    SerialContext,
};

const BAUDRATE_PRESETS: [u32; 6] = [9600_u32, 19200, 38400, 57600, 115200, 230400];

#[component]
pub fn PortSelector() -> Element {
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
                class: "btn btn-sm btn-outline join-item",
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
            div {
                class: "dropdown join-item",
                div {
                    tabindex: "0",
                    role: "button",
                    class: "btn btn-sm btn-outline join-item",
                    Icon {
                        icon: LdUsb {},
                    }
                    if ports().is_empty() {
                        "No Devices available"
                    } else if let Some(index) = selected_index() {
                        if let Some(port) = ports().get(index) {
                            "{port.title()}"
                        } else {
                            "No Device selected"
                        }
                    } else {
                        "No Device selected"
                    }
                }
                ul {
                    tabindex: "-1",
                    class: "dropdown-content menu bg-base-100 p-2 w-52  shadow-sm",
                    {ports().into_iter().enumerate().map(|(index, port)| {
                        rsx!(
                            li {
                                class: "hover:bg-base-200",
                                onclick: move |_| {
                                    *selected_index.write() = Some(index);
                                    *active_index.write() = None;
                                    *status_message.write() = format!("Selected {}.", port.title());
                                },
                                a { "{port.title()}" }
                            }
                        )
                    })}
                }
            }
            button {
                class: "btn btn-sm btn-outline join-item",
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
pub fn PortOpenButton() -> Element {
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
        button {
            class: "btn btn-primary",
            disabled: selected_index().is_none(),
            onclick: move |_| {
                let selected_index_entry = selected_index().and_then(|index| ports().get(index).cloned());

                if let Some(entry) = selected_index_entry {
                    *status_message.write() = format!("Opening {} at {} bps...", entry.title(), baudrate());
                    wasm_bindgen_futures::spawn_local(async move {
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
                    });
                } else {
                    *status_message.write() = "Select a port first.".to_string();
                }
            },
            "Open"
        }
    }
}

#[component]
pub fn PortCloseButton() -> Element {
    let SerialContext {
        ports,
        mut active_index,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        button {
            class: "btn btn-error",
            disabled: active_index().is_none(),
            onclick: move |_| {
                let active_index_entry = active_index().and_then(|index| ports().get(index).cloned());

                if let Some(entry) = active_index_entry {
                    *status_message.write() = format!("Closing {}...", entry.title());
                    wasm_bindgen_futures::spawn_local(async move {
                        match close_port_js(entry.port()).await {
                            Ok(()) => {
                                *active_index.write() = None;
                                *status_message.write() = format!("Closed {}.", entry.title());
                            }
                            Err(err) => {
                                *status_message.write() = format!("Failed to close port: {:?}", err);
                            }
                        }
                    });
                } else {
                    *status_message.write() = "No active port to close.".to_string();
                }
            },
            "Close"
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

#[component]
pub fn BaudrateSelector() -> Element {
    let mut baudrate = use_context::<SerialContext>().baudrate;

    rsx! {
        div {
            class: "grid gap-4 sm:grid-cols-2",

            div {
                class: "form-control gap-2",
                label { class: "label", span { class: "label-text font-medium", "Baud Rate Presets" } }
                select {
                    class: "select select-bordered w-full",
                    onchange: move |event| {
                        let value = event.value();
                        if let Some(new_baudrate) = parse_baudrate(&value) {
                            *baudrate.write() = new_baudrate;
                        }
                    },
                    option { value: "", "Choose a preset" }
                    {BAUDRATE_PRESETS.iter().map(|baudrate| {
                        rsx!(
                            option {
                                value: "{baudrate}",
                                "{baudrate}"
                            }
                        )
                    })}
                }
            }

            div {
                class: "form-control gap-2",
                label { class: "label", span { class: "label-text font-medium", "Custom Baud Rate" } }
                input {
                    class: "input input-bordered w-full",
                    r#type: "number",
                    min: "1",
                    step: "1",
                    value: "{baudrate().to_string()}",
                    placeholder: "{BAUDRATE_PRESETS[0].to_string()}",
                    oninput: move |event| {
                        *baudrate.write() = parse_baudrate(&event.value()).unwrap_or(BAUDRATE_PRESETS[0]);
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
