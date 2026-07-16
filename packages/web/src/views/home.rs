mod display;
mod serial;

use dioxus::prelude::*;

use display::{ConnectionStatus, TextMonitor};
use serial::{
    BaudrateSelector, PortCloseButton, PortOpenButton, PortRefreshButton, PortRequestButton,
    PortSelector, PortWritePanel, SerialProvider,
};

#[component]
pub fn Home() -> Element {
    rsx! {
        SerialProvider {
            div {
                class: "min-h-screen bg-base-200 text-base-content",
                div {
                    class: "mx-auto flex w-full max-w-6xl flex-col gap-6 p-6 lg:p-10",

                    div {
                        class: "card bg-base-100 shadow-xl",
                        div {
                            class: "card-body gap-6",
                            div {
                                class: "flex flex-col gap-2",
                                h1 { class: "card-title text-3xl", "Serial Monitor" }
                                p { class: "text-base-content/70", "Select a serial device, set the baud rate, then open or close the connection from the browser." }
                            }

                            div {
                                class: "grid gap-4 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]",

                                PortSelector {}
                                BaudrateSelector {}

                                div {
                                    class: "flex flex-wrap gap-3",

                                    PortRefreshButton {}
                                    PortRequestButton {}
                                    PortOpenButton {}
                                    PortCloseButton {}
                                }

                                ConnectionStatus {}
                                PortWritePanel {}
                            }

                            div {
                                class: "grid gap-6 lg:grid-cols-2",
                                TextMonitor {}
                            }
                        }
                    }
                }
            }
        }
    }
}
