//! Tasks: the public async functions of the plugins imported, each run as it
//! is turned on, and turned off again as it ends.

use dioxus::{core::current_scope_id, prelude::*, signals::Owner};

use super::{
    CompiledPlugin, Host, PluginFunction, compile_plugin, public_async_functions, run_task,
    set_host,
};
use crate::{data::DataContext, helper::make_owned, serial::SerialContext, time::TimeContext};

/// The latest of a task's run: what it printed last as it goes, then how it
/// ended (that kept if it returned nothing).
#[derive(Clone, Debug, PartialEq)]
pub enum TaskOutcome {
    /// The last line it printed.
    Printed(String),
    /// What it returned, formatted.
    Returned(String),
    Failed(String),
}

#[derive(Clone)]
pub struct Task {
    pub id: usize,
    /// Its function's.
    pub name: String,
    /// The file of the plugin it is of.
    pub plugin: String,
    /// Its function as the plugin has it.
    pub definition: String,
    /// The plugin it is of, compiled as it was imported.
    compiled: CompiledPlugin,
    /// Running while on: turned off as it ends, or to stop it.
    pub enabled: Signal<bool>,
    /// Of its last run: none if it printed and returned nothing (yet).
    pub latest: Signal<Option<TaskOutcome>>,
    /// Whether its card is open; kept here as the cards are made anew when
    /// tasks are added or removed.
    pub open: Signal<bool>,
    _owner: Owner,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Why a plugin was not imported.
#[derive(Clone, Debug, PartialEq)]
pub enum ImportError {
    /// What the compiler told: each error, its line and the code there.
    Compile(String),
    Other(String),
}

/// What a plugin's import made.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Imported {
    pub added: usize,
    /// Of the tasks there already.
    pub passed_over: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TaskContext {
    list: Signal<Vec<Task>>,
    next_id: Signal<usize>,
    scope: ScopeId,
}

impl TaskContext {
    pub fn list(&self) -> Vec<Task> {
        self.list.read().clone()
    }

    pub fn is_empty(&self) -> bool {
        self.list.read().is_empty()
    }

    pub fn get(&self, id: usize) -> Option<Task> {
        self.list.read().iter().find(|task| task.id == id).cloned()
    }

    /// Compiles `source` (of the file `plugin`) and adds a task, off, for each
    /// public async function not already imported from a file of the same name.
    pub fn import_plugin(&mut self, plugin: &str, source: &str) -> Result<Imported, ImportError> {
        let compiled = compile_plugin(plugin, source).map_err(ImportError::Compile)?;
        let functions = public_async_functions(source).map_err(ImportError::Other)?;
        if functions.is_empty() {
            return Err(ImportError::Other(format!("No `pub async fn` in {plugin}")));
        }
        let mut imported = Imported::default();
        for PluginFunction { name, definition } in functions {
            let there = self
                .list
                .peek()
                .iter()
                .any(|task| task.plugin == plugin && task.name == name);
            if there {
                imported.passed_over += 1;
                continue;
            }
            imported.added += 1;
            let id = *self.next_id.peek();
            self.next_id.set(id + 1);
            let ((enabled, latest, open), owner) = make_owned(self.scope, || {
                (Signal::new(false), Signal::new(None), Signal::new(false))
            });
            self.list.write().push(Task {
                id,
                name,
                plugin: plugin.to_string(),
                definition,
                compiled: compiled.clone(),
                enabled,
                latest,
                open,
                _owner: owner,
            });
        }
        Ok(imported)
    }

    pub fn remove(&mut self, id: usize) {
        self.list.write().retain(|task| task.id != id);
    }
}

/// Provides [`TaskContext`] and runs the tasks turned on. Requires
/// `DataContext`, `SerialContext` and `TimeContext`.
#[component]
pub fn TaskProvider(children: Element) -> Element {
    let data = use_context::<DataContext>();
    let serial = use_context::<SerialContext>();
    let time = use_context::<TimeContext>();
    use_hook(|| set_host(Host { data, serial, time }));
    use_context_provider(|| TaskContext {
        list: Signal::new(Vec::new()),
        next_id: Signal::new(0),
        scope: current_scope_id(),
    });

    rsx! {
        TaskRuns {}
        {children}
    }
}

/// Apart from `TaskProvider`, so a task added does not render its children.
#[component]
fn TaskRuns() -> Element {
    let context = use_context::<TaskContext>();
    let ids = use_memo(move || {
        context
            .list
            .read()
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
    });

    rsx! {
        for id in ids() {
            TaskRun { key: "{id}", id }
        }
    }
}

/// Runs task `id` as it is turned on, then turns it off as it ends; stops it
/// as it is turned off before (or removed, as this goes with it).
#[component]
fn TaskRun(id: usize) -> Element {
    let context = use_context::<TaskContext>();
    let task = use_memo(move || context.get(id));
    let mut running = use_signal(|| None::<dioxus::core::Task>);

    use_effect(move || {
        let Some(task) = task() else {
            return;
        };
        let (mut enabled, mut latest) = (task.enabled, task.latest);
        if enabled() {
            if running.peek().is_some() {
                return;
            }
            latest.set(None);
            let run = spawn(async move {
                let printed = |line: &str| latest.set(Some(TaskOutcome::Printed(line.to_string())));
                let outcome = match run_task(&task.compiled, &task.name, printed).await {
                    Ok(returned) => returned.map(TaskOutcome::Returned),
                    Err(error) => Some(TaskOutcome::Failed(error)),
                };
                running.set(None);
                // What it printed last, if it returned nothing
                if outcome.is_some() {
                    latest.set(outcome);
                }
                enabled.set(false);
            });
            running.set(Some(run));
        } else if let Some(run) = running.take() {
            run.cancel();
        }
    });

    rsx! {}
}
