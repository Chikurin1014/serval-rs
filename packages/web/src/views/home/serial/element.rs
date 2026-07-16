use dioxus::prelude::*;

use super::web_serial_api::{
    close_port_js, get_ports_js, open_port_js, request_port_js, SerialContext,
};

const BAUDRATE_PRESETS: [u32; 6] = [9600_u32, 19200, 38400, 57600, 115200, 230400];

#[component]
pub fn PortSelector() -> Element {
    let SerialContext {
        ports,
        mut selected_index,
        active_index,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        div {
            class: "card bg-base-100 shadow-xl",
            div {
                class: "card-body gap-4",
                // h2 { class: "card-title", "Detected Ports" }

                if !ports().is_empty() {
                    div {
                        class: "overflow-x-auto",
                        table {
                            class: "table table-zebra",
                            thead {
                                tr {
                                    th { "Device" }
                                    th { "Details" }
                                    th { "State" }
                                    th { "Action" }
                                }
                            }
                            tbody {
                                {ports().into_iter().enumerate().map(|(index, port)| {
                                    let is_selected = selected_index() == Some(index);
                                    let is_active = active_index() == Some(index);
                                    rsx!(
                                        tr {
                                            td {
                                                div { class: "flex items-center gap-2",
                                                    span { "{port.title()}" }
                                                    if is_selected {
                                                        span { class: "badge badge-primary badge-sm", "Selected" }
                                                    }
                                                    if is_active {
                                                        span { class: "badge badge-success badge-sm", "Open" }
                                                    }
                                                }
                                            }
                                            td {
                                                if port.details().is_empty() {
                                                    span { class: "text-base-content/60", "No USB metadata available." }
                                                } else {
                                                    div { class: "flex flex-wrap gap-2",
                                                        {port.details().iter().map(|detail| rsx!( span { class: "badge badge-ghost", "{detail}" } ))}
                                                    }
                                                }
                                            }
                                            td {
                                                if is_active {
                                                    span { class: "badge badge-success", "Open" }
                                                } else if is_selected {
                                                    span { class: "badge badge-primary", "Selected" }
                                                } else {
                                                    span { class: "badge", "Idle" }
                                                }
                                            }
                                            td {
                                                button {
                                                    class: "btn btn-xs btn-outline",
                                                    onclick: move |_| {
                                                        *selected_index.write() = Some(index);
                                                        *status_message.write() = format!("Selected {}.", port.title());
                                                    },
                                                    "Select"
                                                }
                                            }
                                        }
                                    )
                                })}
                            }
                        }
                    }
                } else {
                    p { class: "text-base-content/70", "No serial ports detected. Please connect a device and click 'Refresh Ports'." }
                }
            }
        }
    }
}

#[component]
pub fn PortRequestButton() -> Element {
    let SerialContext {
        mut ports,
        mut selected_index,
        mut active_index,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        button {
            class: "btn btn-outline",
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
            "Request Port"
        }
    }
}

#[component]
pub fn PortRefreshButton() -> Element {
    let SerialContext {
        mut ports,
        mut selected_index,
        mut active_index,
        mut status_message,
        ..
    } = use_context::<SerialContext>();

    rsx! {
        button {
            class: "btn btn-secondary",
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
            "Refresh Ports"
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
