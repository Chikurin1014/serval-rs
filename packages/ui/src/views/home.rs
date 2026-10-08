use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::tabs::{TabContent, TabList, TabTrigger, Tabs},
    data::RAW_BYTES_LABEL,
    elements::{
        DataList, FilterContext, FilterKind,
        graph::GraphBoard,
        map::MapList,
        serial::{
            PortBaudrateConfigurator, PortInfoPanel, PortIoConsole, PortOpenCloseButton,
            PortSelector,
        },
    },
};

const HOME_CSS: Asset = asset!("/assets/styling/home.css");

/// Requires `SerialContext`, `DataContext` and `MapContext`.
#[component]
pub fn Home() -> Element {
    // Above the tabs, so the filters stay when the data list is left
    use_context_provider(|| FilterContext::with(&[(FilterKind::Hide, RAW_BYTES_LABEL)]));

    rsx! {
        document::Link { rel: "stylesheet", href: HOME_CSS }

        Tabs {
            class: "home",
            default_value: "console".to_string(),
            div {
                class: "toolbar",
                div {
                    class: "toolbar-port",
                    PortSelector {}
                    PortBaudrateConfigurator {}
                    PortOpenCloseButton {}
                }
                TabList {
                    class: "toolbar-tabs",
                    TabTrigger {
                        class: "home-tab-trigger",
                        index: 0usize,
                        value: "console".to_string(),
                        lucide::SquareTerminal { size: 20 }
                        "Console"
                    }
                    TabTrigger {
                        class: "home-tab-trigger",
                        index: 1usize,
                        value: "data".to_string(),
                        lucide::Database { size: 20 }
                        "Data"
                    }
                    TabTrigger {
                        class: "home-tab-trigger",
                        index: 2usize,
                        value: "graph".to_string(),
                        lucide::ChartLine { size: 20 }
                        "Graph"
                    }
                }
            }
            TabContent {
                class: "home-tab",
                index: 0usize,
                value: "console".to_string(),
                div {
                    class: "console-grid",
                    PortIoConsole {}
                    PortInfoPanel {}
                }
            }
            TabContent {
                class: "home-tab",
                index: 1usize,
                value: "data".to_string(),
                div {
                    class: "data-grid",
                    DataList {}
                    MapList {}
                }
            }
            TabContent {
                class: "home-tab",
                index: 2usize,
                value: "graph".to_string(),
                GraphBoard {}
            }
        }
    }
}
