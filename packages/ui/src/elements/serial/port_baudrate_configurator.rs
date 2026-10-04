use dioxus::prelude::*;

use crate::{components::input::Input, serial::SerialContext};

const PORT_BAUDRATE_CONFIGURATOR_CSS: Asset =
    asset!("/assets/styling/port-baudrate-configurator.css");

const BAUDRATE_PRESETS: [u32; 4] = [9600_u32, 19200, 57600, 115200];

#[component]
pub fn PortBaudrateConfigurator() -> Element {
    let SerialContext {
        mut ports,
        selected_id,
        ..
    } = use_context::<SerialContext>();

    let selected_baudrate =
        selected_id().and_then(|id| ports().get(&id).and_then(|port| port.baudrate));

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_BAUDRATE_CONFIGURATOR_CSS }

        label {
            class: "baudrate-field",
            Input {
                type: "number",
                min: "1",
                step: "1",
                placeholder: "Baudrate (e.g. 9600)",
                list: "baudrate-presets",
                value: selected_baudrate.map(|baudrate| baudrate.to_string()),
                oninput: move |event: FormEvent| {
                    if let Some(id) = selected_id() {
                        if let Some(baudrate) = parse_baudrate(&event.value()) {
                            if let Some(port) = ports.write().get_mut(&id) {
                                port.baudrate = Some(baudrate);
                            }
                        }
                    }
                }
            }
            span { class: "baudrate-unit", "bps" }
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

fn parse_baudrate(value: &str) -> Option<u32> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|baudrate| *baudrate > 0)
}
