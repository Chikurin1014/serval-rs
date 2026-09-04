use dioxus::prelude::*;

use ui::component::{
    ConversionList, DataList, PortBaudrateConfigurator, PortIoConsole, PortOpenCloseButton,
    PortSelector,
};

#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            class: "flex h-full flex-col overflow-hidden gap-3 p-3",
            div {
                class: "flex shrink-0 flex-wrap items-center gap-2",
                PortSelector {}
                PortBaudrateConfigurator {}
                PortOpenCloseButton {}
            }
            div {
                class: "grid md:grid-cols-2 h-full gap-1 overflow-auto",
                DataList {}
                ConversionList {}
            }
            div {
                class: "h-[30vh] shrink-0",
                PortIoConsole {}
            }
        }
    }
}
