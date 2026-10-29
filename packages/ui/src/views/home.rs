use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::{
    app::tools_in_menu,
    components::tabs::{TabContent, TabList, TabTrigger, Tabs},
    data::RAW_BYTES_LABEL,
    elements::{
        DataList, FilterContext, FilterKind, TaskList, TaskStdout,
        graph::GraphBoard,
        map::MapList,
        serial::{
            PortConfigurator, PortInfoPanel, PortIoConsole, PortOpenCloseButton, PortSelector,
            PortSignalButton,
        },
    },
    plugin::TaskContext,
};

mod toolbar_menu;

use toolbar_menu::ToolbarMenu;

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
    // With no navbar, as the desktop app: the toolbar always in the menu
    let toolbar_in_menu = tools_in_menu();
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
            "data-toolbar-in-menu": toolbar_in_menu,
            // Narrow, the toolbar is hidden by CSS, not unmounted: a part
            // unmounted as it first renders may take its component's stylesheet
            // with it (as the desktop app links them). Its place is known at
            // once with no navbar
            if !toolbar_in_menu {
                div {
                    class: "toolbar",
                    div {
                        class: "toolbar-port",
                        PortSelector {}
                        PortConfigurator {}
                        PortOpenCloseButton {}
                        PortSignalButton {}
                    }
                    HomeTabList { class: "toolbar-tabs", has_tasks: has_tasks() }
                }
            }
            ToolbarMenu {
                app_tools: toolbar_in_menu,
                HomeTabList { class: "toolbar-menu-tabs", has_tasks: has_tasks() }
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
                    div {
                        class: "task-grid",
                        TaskList {}
                        TaskStdout {}
                    }
                }
            }
        }
    }
}

/// The tabs' triggers, on the toolbar or in its menu.
#[component]
fn HomeTabList(class: String, has_tasks: bool) -> Element {
    rsx! {
        TabList {
            class,
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
            if has_tasks {
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
}
