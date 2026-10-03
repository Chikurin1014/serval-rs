use dioxus::prelude::*;

use crate::elements::{
    ConversionList, DataList, PortBaudrateConfigurator, PortIoConsole, PortOpenCloseButton,
    PortSelector,
};

const HOME_CSS: Asset = asset!("/assets/styling/home.css");

/// Requires `SerialContext` and `DataContext` to be provided by an ancestor.
#[component]
pub fn Home() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: HOME_CSS }

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
