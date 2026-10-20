//! Tasks: the public async functions of the plugins imported, each turned on
//! or off.
//! What they can do, and their running, are yet to come.

use dioxus::{core::current_scope_id, prelude::*, signals::Owner};

use crate::{
    data::{PluginFunction, public_async_functions},
    helper::make_owned,
};

#[derive(Clone)]
pub struct Task {
    pub id: usize,
    /// Its function's.
    pub name: String,
    /// The file of the plugin it is of.
    pub plugin: String,
    /// Its function as the plugin has it.
    pub definition: String,
    pub enabled: Signal<bool>,
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

    /// Adds a task, off, for each public async function of the plugin `source`
    /// (Rune), named `plugin`: but those of a function of the same name of a
    /// plugin of the same name, there already. How many were added and passed
    /// over, or what is wrong with it with none added.
    pub fn import_plugin(&mut self, plugin: &str, source: &str) -> Result<Imported, String> {
        let functions = public_async_functions(source)?;
        if functions.is_empty() {
            return Err(format!("No `pub async fn` in {plugin}"));
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
            let ((enabled, open), owner) =
                make_owned(self.scope, || (Signal::new(false), Signal::new(false)));
            self.list.write().push(Task {
                id,
                name,
                plugin: plugin.to_string(),
                definition,
                enabled,
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

/// Provides [`TaskContext`], with no task until a plugin is imported.
#[component]
pub fn TaskProvider(children: Element) -> Element {
    use_context_provider(|| TaskContext {
        list: Signal::new(Vec::new()),
        next_id: Signal::new(0),
        scope: current_scope_id(),
    });

    rsx! {
        {children}
    }
}
