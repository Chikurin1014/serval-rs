use dioxus::prelude::*;

use crate::web_serial_api::SerialProvider;
use ui::data::DataProvider;
use ui::elements::{
    ConversionList, DataList, PortBaudrateConfigurator, PortIoConsole, PortOpenCloseButton,
    PortSelector,
};

#[component]
pub fn Home() -> Element {
    rsx! {
        SerialProvider {
            DataProvider {
                    div {
                        class: "home",
                        div {
                            class: "toolbar",
                            PortSelector {}
                            PortBaudrateConfigurator {}
                            PortOpenCloseButton {}
                        }
                        div {
                            class: "upper-grid",
                            DataList {}
                            ConversionList {}
                        }
                        div {
                            class: "console-slot",
                            PortIoConsole {}
                        }
                    }
            }
        }
    }
}
