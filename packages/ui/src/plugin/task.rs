//! Tasks: the public async functions of the plugins imported, each run as it
//! is turned on, and turned off again as it ends.

use std::collections::VecDeque;

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

/// How many lines the standard output keeps: as the console's scrollback.
const STDOUT_LINES: usize = 10_000;

/// A line a task printed.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintedLine {
    /// The task's, as `<file>::<function>`.
    pub source: String,
    /// When it was printed, in ms since the Unix epoch.
    pub time: i64,
    pub text: String,
}

/// What the tasks printed, by lines: the last [`STDOUT_LINES`].
#[derive(Debug, Default)]
struct Stdout {
    lines: VecDeque<PrintedLine>,
    /// Lines dropped from the front
    dropped: usize,
    /// Where it was last cleared, counted from the first line ever
    cleared_at: usize,
}

/// Lines printed since a reader's last read.
#[derive(Clone, Debug, PartialEq)]
pub struct NewLines {
    pub lines: Vec<PrintedLine>,
    /// Where they end, counted from the first line ever: the next read's start.
    pub end: usize,
    /// Cleared since the last read: what was shown is to go.
    pub cleared: bool,
}

impl Stdout {
    fn push(&mut self, line: PrintedLine) {
        if self.lines.len() == STDOUT_LINES {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back(line);
    }

    /// The lines from `from` (counted from the first ever) that are kept.
    fn since(&self, from: usize) -> NewLines {
        let skip = from.saturating_sub(self.dropped);
        NewLines {
            lines: self.lines.iter().skip(skip).cloned().collect(),
            end: self.dropped + self.lines.len(),
            cleared: from < self.cleared_at,
        }
    }

    fn clear(&mut self) {
        // One past the lines dropped: every read before it ends before it
        self.dropped += self.lines.len() + 1;
        self.lines.clear();
        self.cleared_at = self.dropped;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct TaskContext {
    list: Signal<Vec<Task>>,
    next_id: Signal<usize>,
    stdout: Signal<Stdout>,
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

    /// The lines printed from `from` (counted from the first ever; all for
    /// none), of those kept.
    pub fn printed_since(&self, from: Option<usize>) -> NewLines {
        self.stdout.read().since(from.unwrap_or(0))
    }

    fn print(&mut self, line: PrintedLine) {
        self.stdout.write().push(line);
    }

    /// Drops all that was printed, from what is shown.
    pub fn clear_printed(&mut self) {
        self.stdout.write().clear();
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
        stdout: Signal::new(Stdout::default()),
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
    let mut context = use_context::<TaskContext>();
    let time = use_context::<TimeContext>();
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
            let time = time.clone();
            let source = format!("{}::{}", task.plugin, task.name);
            let run = spawn(async move {
                let printed = |line: &str| {
                    context.print(PrintedLine {
                        source: source.clone(),
                        time: time.current(),
                        text: line.to_string(),
                    });
                    // Beside its name: the last line of something
                    if !line.trim().is_empty() {
                        latest.set(Some(TaskOutcome::Printed(line.to_string())));
                    }
                };
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

#[cfg(test)]
mod tests {
    use super::{PrintedLine, STDOUT_LINES, Stdout};

    fn line(text: &str) -> PrintedLine {
        PrintedLine {
            source: "test.rn::talks".into(),
            time: 0,
            text: text.into(),
        }
    }

    #[test]
    fn printed_lines_are_read_from_where_a_reader_left() {
        let mut stdout = Stdout::default();
        stdout.push(line("first"));
        let read = stdout.since(0);
        assert_eq!(read.lines, [line("first")]);
        stdout.push(line("second"));
        assert_eq!(stdout.since(read.end).lines, [line("second")]);
        assert!(stdout.since(stdout.since(0).end).lines.is_empty());
    }

    #[test]
    fn the_oldest_lines_are_dropped_past_the_last_kept() {
        let mut stdout = Stdout::default();
        for index in 0..STDOUT_LINES + 2 {
            stdout.push(line(&index.to_string()));
        }
        let read = stdout.since(0);
        assert_eq!(read.lines.len(), STDOUT_LINES);
        assert_eq!(read.lines[0], line("2"));
        assert_eq!(read.end, STDOUT_LINES + 2);
        // Those read before dropped: from the first kept
        assert_eq!(stdout.since(1).lines[0], line("2"));
        assert_eq!(
            stdout.since(STDOUT_LINES + 1).lines,
            [line(&(STDOUT_LINES + 1).to_string())]
        );
    }

    #[test]
    fn a_reader_is_told_of_a_clear_once() {
        let mut stdout = Stdout::default();
        stdout.push(line("first"));
        let read = stdout.since(0);
        assert!(!read.cleared);
        stdout.clear();
        stdout.push(line("second"));
        let read = stdout.since(read.end);
        assert!(read.cleared);
        assert_eq!(read.lines, [line("second")]);
        assert!(!stdout.since(read.end).cleared);
    }
}
