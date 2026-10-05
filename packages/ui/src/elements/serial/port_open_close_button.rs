use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::button::{Button, ButtonSize, ButtonVariant},
    serial::SerialContext,
};

const PORT_OPEN_CLOSE_BUTTON_CSS: Asset = asset!("/assets/styling/port-open-close-button.css");

#[component]
pub fn PortOpenCloseButton() -> Element {
    let serial = use_context::<SerialContext>();
    let currently_open = serial.is_open();
    let port_available = serial
        .selected_port()
        .and_then(|port| port.baudrate)
        .is_some();

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_OPEN_CLOSE_BUTTON_CSS }

        Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::IconSm,
            class: "port-open-close",
            "data-open": currently_open,
            disabled: !port_available,
            aria_label: if currently_open { "Close port" } else { "Open port" },
            onclick: move |_| {
                if currently_open {
                    serial.close();
                } else {
                    serial.open();
                }
            },
            if currently_open { lucide::Pause {} } else { lucide::Play {} }
        }
    }
}
