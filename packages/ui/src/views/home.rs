use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::tabs::{TabContent, TabList, TabTrigger, Tabs},
    elements::{
        ConversionList, DataList, PortBaudrateConfigurator, PortIoConsole, PortOpenCloseButton,
        PortSelector,
    },
    graph::GraphBoard,
};

const HOME_CSS: Asset = asset!("/assets/styling/home.css");

/// Requires `SerialContext`, `DataContext` and `ConversionContext` to be
/// provided by an ancestor.
#[component]
pub fn Home() -> Element {
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
                PortIoConsole {}
            }
            TabContent {
                class: "home-tab",
                index: 1usize,
                value: "data".to_string(),
                div {
                    class: "data-grid",
                    DataList {}
                    ConversionList {}
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
