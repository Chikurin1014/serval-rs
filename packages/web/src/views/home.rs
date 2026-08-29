use dioxus::prelude::*;

use crate::web_serial_api::WebSerialProvider;
use ui::{
    component::{
        DataList, PortBaudrateConfigurator, PortIoConsole, PortOpenCloseButton, PortSelector,
    },
    data::DataProvider,
    serial::SerialProvider,
};

#[component]
pub fn Home() -> Element {
    rsx! {
        SerialProvider {
            DataProvider {
                WebSerialProvider {
                    PortSelector {}
                    div {
                        PortBaudrateConfigurator {}
                        PortOpenCloseButton {}
                    }
                    div {
                        class: "grid gap-6 lg:grid-cols-2",
                        PortIoConsole {}
                        DataList {}
                    }
                }
            }
        }
    }
}
