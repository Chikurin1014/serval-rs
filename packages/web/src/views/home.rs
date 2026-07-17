mod display;
mod serial;

use dioxus::prelude::*;

use display::{ConnectionStatus, TextMonitor};
use serial::{
    BaudrateConfigurator, DeviceSelector, OpenCloseButton, PortWritePanel, SerialProvider,
};

#[component]
pub fn Home() -> Element {
    rsx! {
    SerialProvider {
        DeviceSelector {}
            div {
                BaudrateConfigurator {}
                OpenCloseButton {}
            }
            ConnectionStatus {}
            PortWritePanel {}

            div {
                class: "grid gap-6 lg:grid-cols-2",
                TextMonitor {}
            }
        }
    }
}
