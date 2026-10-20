use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    components::tabs::{TabContent, TabList, TabTrigger, Tabs},
    data::RAW_BYTES_LABEL,
    elements::{
        DataList, FilterContext, FilterKind, TaskList,
        graph::GraphBoard,
        map::MapList,
        serial::{
            PortBaudrateConfigurator, PortInfoPanel, PortIoConsole, PortOpenCloseButton,
            PortSelector, PortSignalButton,
        },
    },
    plugin::TaskContext,
};

const HOME_CSS: Asset = asset!("/assets/styling/home.css");

/// Requires `SerialContext`, `DataContext`, `MapContext`, `GraphContext` and
/// `TaskContext`.
#[component]
pub fn Home() -> Element {
    // Above the tabs, so the filters stay when the data list is left
    use_context_provider(|| FilterContext::with(&[(FilterKind::Hide, RAW_BYTES_LABEL)]));
    let tasks = use_context::<TaskContext>();
    // Its tab once a plugin gives tasks
    let has_tasks = use_memo(move || !tasks.is_empty());
    let mut tab = use_signal(|| "console".to_string());
    // Its tab gone with the last task: the console's instead
    use_effect(move || {
        if !has_tasks() && *tab.peek() == "task" {
            tab.set("console".to_string());
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: HOME_CSS }

        Tabs {
            class: "home",
            value: Some(tab()),
            on_value_change: move |value| tab.set(value),
            div {
                class: "toolbar",
                div {
                    class: "toolbar-port",
                    PortSelector {}
                    PortBaudrateConfigurator {}
                    PortOpenCloseButton {}
                    PortSignalButton {}
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
                    if has_tasks() {
                        TabTrigger {
                            class: "home-tab-trigger",
                            index: 3usize,
                            value: "task".to_string(),
                            lucide::Blocks { size: 20 }
                            "Task"
                        }
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
            if has_tasks() {
                TabContent {
                    class: "home-tab",
                    index: 3usize,
                    value: "task".to_string(),
                    TaskList {}
                }
            }
        }
    }
}
