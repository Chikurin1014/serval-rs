//! The tasks of the plugins imported, as the maps are listed: each its card,
//! turned on or off, opened to its function's definition.

use dioxus::prelude::*;
use dioxus_icons::lucide;
use dioxus_primitives::scroll_area::ScrollDirection;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        card::{Card, CardContent, CardHeader},
        collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
        scroll_area::ScrollArea,
        switch::Switch,
    },
    data::{Imported, Task, TaskContext},
    toast::use_toaster,
};

// The cards as the maps'
const MAP_LIST_CSS: Asset = asset!("/assets/styling/map-list.css");
const TASK_LIST_CSS: Asset = asset!("/assets/styling/task-list.css");
const PLUGIN_INPUT_ID: &str = "plugin-file-input";

/// Lists the tasks, in the order their plugins were imported.
#[component]
pub fn TaskList() -> Element {
    let context = use_context::<TaskContext>();
    let tasks = use_memo(move || context.list());

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_LIST_CSS }
        document::Link { rel: "stylesheet", href: TASK_LIST_CSS }

        div {
            class: "map-panel task-panel",
            ul {
                class: "task-list",
                aria_label: "Tasks",
                for task in tasks() {
                    li {
                        key: "{task.id}",
                        TaskCard { task }
                    }
                }
            }
        }
    }
}

#[component]
fn TaskCard(task: Task) -> Element {
    let mut context = use_context::<TaskContext>();
    let Task {
        id,
        mut enabled,
        mut open,
        ..
    } = task;

    rsx! {
        Collapsible {
            class: "map-card task-card reveals",
            open: Some(open()),
            on_open_change: move |value| open.set(value),
            Card {
                CardHeader {
                    Switch {
                        checked: enabled(),
                        on_checked_change: move |checked| enabled.set(checked),
                        aria_label: "Toggle task",
                    }
                    CollapsibleTrigger {
                        class: "map-title",
                        span {
                            class: "map-heading",
                            span { class: "map-title-text", "{task.name}" }
                            span { class: "task-plugin", "{task.plugin}" }
                        }
                    }
                    Button {
                        class: "map-remove reveal-on-hover",
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::IconSm,
                        aria_label: "Delete task",
                        title: "Delete task",
                        onclick: move |_| context.remove(id),
                        lucide::X {}
                    }
                }
                CollapsibleContent {
                    CardContent {
                        ScrollArea {
                            class: "task-definition",
                            direction: ScrollDirection::Both,
                            aria_label: "Definition of {task.name}",
                            pre {
                                code { "{task.definition}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Imports a plugin, a Rune file picked: a task for each of its public async
/// functions.
#[component]
pub fn PluginButton() -> Element {
    let mut context = use_context::<TaskContext>();
    let toaster = use_toaster();

    rsx! {
        document::Link { rel: "stylesheet", href: TASK_LIST_CSS }

        Button {
            class: "plugin-button",
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconSm,
            aria_label: "Import plugin",
            title: "Import a plugin (Rune)",
            onclick: move |_| {
                document::eval(&format!(
                    "document.getElementById({PLUGIN_INPUT_ID:?})?.click()"
                ));
            },
            lucide::Blocks {}
        }
        input {
            id: PLUGIN_INPUT_ID,
            class: "plugin-file-input",
            r#type: "file",
            accept: ".rn",
            onchange: move |event| async move {
                let Some(file) = event.files().into_iter().next() else {
                    return;
                };
                let name = file.name();
                let imported = match file.read_string().await {
                    Ok(source) => context.import_plugin(&name, &source),
                    Err(error) => Err(error.to_string()),
                };
                // So the same file picked again is a change
                document::eval(&format!(
                    "document.getElementById({PLUGIN_INPUT_ID:?}).value = ''"
                ));
                match imported {
                    Ok(Imported { added: 0, .. }) => {
                        toaster.info("Plugin already imported", &format!("All the tasks of {name}"));
                    }
                    Ok(Imported { added, passed_over }) => {
                        let mut detail = format!("{} of {name}", tasks(added));
                        if passed_over > 0 {
                            detail += &format!(" ({} already there)", tasks(passed_over));
                        }
                        toaster.success("Plugin imported", &detail);
                    }
                    Err(error) => toaster.error("Failed to import the plugin", &error),
                }
            },
        }
    }
}

/// `count` tasks, e.g. `1 task`.
fn tasks(count: usize) -> String {
    format!("{count} task{}", if count == 1 { "" } else { "s" })
}
