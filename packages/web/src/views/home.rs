use dioxus::prelude::*;

use ui::component::{
    ConversionList, DataList, PortBaudrateConfigurator, PortIoConsole, PortOpenCloseButton,
    PortSelector,
};

#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            class: "flex h-full flex-col gap-3 p-3",
            div {
                class: "flex flex-wrap items-center gap-2",
                PortSelector {}
                PortBaudrateConfigurator {}
                PortOpenCloseButton {}
            }
            div {
                class: "md:flex overflow-auto h-full min-h-0",
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
