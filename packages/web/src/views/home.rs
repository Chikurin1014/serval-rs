mod display;
mod serial;

use dioxus::prelude::*;

use display::{ConnectionStatus, TextMonitor};
use serial::{
    BaudrateSelector, PortCloseButton, PortOpenButton, PortSelector, PortWritePanel, SerialProvider,
};

#[component]
pub fn Home() -> Element {
    rsx! {
    SerialProvider {
        // div {
        //     class: "min-h-screen bg-base-200 text-base-content",
        //     div {
        //         class: "mx-auto flex w-full max-w-6xl flex-col gap-6 p-6 lg:p-10",
                        div {
                            class: "flex justify-between",
                            PortSelector {}
                            PortOpenButton {}
                            PortCloseButton {}
                        }
                        BaudrateSelector {}
                        ConnectionStatus {}
                        PortWritePanel {}

                        div {
                            class: "grid gap-6 lg:grid-cols-2",
                            TextMonitor {}
                        }
                    }
            //     }
            // }
        }
}
