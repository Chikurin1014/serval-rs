//! The tasks of the plugins imported, as the maps are listed: each its card,
//! turned on to run (off again as it ends), what it last returned beside its
//! name, opened to its function's definition.

use dioxus::prelude::*;
use dioxus_icons::lucide;
use dioxus_primitives::scroll_area::ScrollDirection;

use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        card::{Card, CardContent, CardHeader},
        collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
        dialog::{Dialog, DialogDescription, DialogTitle},
        scroll_area::ScrollArea,
        switch::Switch,
    },
    data::{ImportError, Imported, Task, TaskContext, TaskOutcome},
    helper::single_line,
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
                        // Its own component: the card must not render again with it
                        TaskLatest { latest: task.latest }
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

/// The latest of the task's run, as the latest conversion of a map: what it
/// printed last, then what it returned, or why it failed.
#[component]
fn TaskLatest(latest: ReadSignal<Option<TaskOutcome>>) -> Element {
    rsx! {
        match latest() {
            Some(TaskOutcome::Returned(value)) => rsx! {
                span {
                    class: "map-latest task-latest",
                    title: "{value}",
                    span { class: "map-latest-value", {single_line(&value)} }
                }
            },
            // As a console's line
            Some(TaskOutcome::Printed(line)) => rsx! {
                span {
                    class: "map-latest task-latest",
                    "data-printed": true,
                    title: "{line}",
                    lucide::SquareTerminal {}
                    span { class: "map-latest-value", {single_line(&line)} }
                }
            },
            Some(TaskOutcome::Failed(error)) => rsx! {
                span {
                    class: "map-latest task-latest",
                    "data-failed": true,
                    title: "{error}",
                    lucide::CircleAlert {}
                    span { class: "map-latest-value", {single_line(&error)} }
                }
            },
            None => rsx! {},
        }
    }
}

/// Imports a plugin, a Rune file picked: a task for each of its public async
/// functions.
#[component]
pub fn PluginButton() -> Element {
    let mut context = use_context::<TaskContext>();
    let toaster = use_toaster();
    // The file that did not compile, and what the compiler told
    let mut failed = use_signal(|| None::<(String, String)>);

    rsx! {
        document::Link { rel: "stylesheet", href: TASK_LIST_CSS }

        CompileErrorDialog { failed }
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
                    Err(error) => Err(ImportError::Other(error.to_string())),
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
                    Err(ImportError::Compile(told)) => failed.set(Some((name, told))),
                    Err(ImportError::Other(error)) => {
                        toaster.error("Failed to import the plugin", &error)
                    }
                }
            },
        }
    }
}

/// What the compiler told of the plugin that did not compile, there until
/// closed.
#[component]
fn CompileErrorDialog(failed: Signal<Option<(String, String)>>) -> Element {
    let mut failed = failed;
    let (name, told) = failed().unwrap_or_default();

    rsx! {
        Dialog {
            class: "compile-error-dialog",
            open: failed.read().is_some(),
            on_open_change: move |open: bool| {
                if !open {
                    failed.set(None);
                }
            },
            Button {
                class: "compile-error-close",
                variant: ButtonVariant::Ghost,
                size: ButtonSize::IconSm,
                aria_label: "Close",
                title: "Close",
                onclick: move |_| failed.set(None),
                lucide::X {}
            }
            DialogTitle { "Failed to compile {name}" }
            DialogDescription { "Fix it, then import it again." }
            ScrollArea {
                class: "compile-error",
                direction: ScrollDirection::Both,
                aria_label: "Compile errors",
                pre {
                    code { "{told}" }
                }
            }
        }
    }
}

/// `count` tasks, e.g. `1 task`.
fn tasks(count: usize) -> String {
    format!("{count} task{}", if count == 1 { "" } else { "s" })
}
