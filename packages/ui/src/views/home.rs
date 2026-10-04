use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdDatabase, LdLineChart, LdSquareTerminal},
    Icon,
};

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
                        Icon { icon: LdSquareTerminal }
                        "Console"
                    }
                    TabTrigger {
                        class: "home-tab-trigger",
                        index: 1usize,
                        value: "data".to_string(),
                        Icon { icon: LdDatabase }
                        "Data"
                    }
                    TabTrigger {
                        class: "home-tab-trigger",
                        index: 2usize,
                        value: "graph".to_string(),
                        Icon { icon: LdLineChart }
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
