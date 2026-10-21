//! Plugins compiled as they are imported, their tasks run from that: each
//! function called and awaited, what it prints taken as it goes.

use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context as TaskContext, Poll},
};

use super::serval;

use rune::{
    Context, Diagnostics, Source, Sources, Vm,
    modules::capture_io::{self, CaptureIo},
    runtime::{Function, Unit, budget},
    termcolor::NoColor,
};

/// What a task's function may do between its awaits, in instructions: past
/// it, as in a loop of no await, it is stopped (the page would stand still).
pub const INSTRUCTIONS_PER_STEP: usize = 10_000_000;

/// Appended to each plugin: formatting of what a task returns, and the
/// generator of `serval::data::get`'s streams (Rune makes them of generators only).
const HELPERS: &str = "
pub fn __serval_display(value) { format!(\"{}\", value) }
pub fn __serval_debug(value) { format!(\"{:?}\", value) }
pub async fn __serval_stream(reader, with_time) {
    while let Some(entry) = reader.next(with_time).await { yield entry; }
}
";

/// A plugin, compiled; run as many times as its tasks are.
#[derive(Clone)]
pub struct CompiledPlugin(Arc<Unit>);

/// The default modules, but what is printed taken to `io` rather than a
/// stdout; and Serval's (`serval::*`).
fn context(io: &CaptureIo) -> Result<Context, String> {
    let mut context = Context::with_config(false).map_err(|error| error.to_string())?;
    context
        .install(capture_io::module(io).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    for module in serval::modules().map_err(|error| error.to_string())? {
        context.install(module).map_err(|error| error.to_string())?;
    }
    Ok(context)
}

/// Compiles the plugin `source`, of the file `name`; else what is wrong with
/// it, as a compiler tells it: each error, its line and the code there.
pub fn compile_plugin(name: &str, source: &str) -> Result<CompiledPlugin, String> {
    let context = context(&CaptureIo::new())?;
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(name, format!("{source}\n{HELPERS}")).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let mut diagnostics = Diagnostics::new();
    let built = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();
    match built {
        Ok(unit) if !diagnostics.has_error() => Ok(CompiledPlugin(Arc::new(unit))),
        _ => {
            let mut told = NoColor::new(Vec::new());
            diagnostics
                .emit(&mut told, &sources)
                .map_err(|error| error.to_string())?;
            Err(String::from_utf8_lossy(&told.into_inner())
                .trim_end()
                .to_string())
        }
    }
}

/// Runs the async function `name` of `plugin`: what it returns, formatted
/// (none for `()`), or why it failed. `on_print` gets each line it prints, as it
/// goes (one left unended, as it ends).
pub async fn run_task(
    plugin: &CompiledPlugin,
    name: &str,
    on_print: impl FnMut(&str),
) -> Result<Option<String>, String> {
    // Its own output, so tasks running together do not mix theirs
    let io = CaptureIo::new();
    let runtime = Arc::new(context(&io)?.runtime().map_err(|error| error.to_string())?);
    let mut vm = Vm::new(runtime, plugin.0.clone());

    let stream_of: Rc<Function> = vm
        .lookup_function(["__serval_stream"])
        .map_err(|error| error.to_string())?
        .into();

    let output = Stepped::new(io, stream_of, Box::new(on_print), async {
        let called = vm.async_call([name], ()).await?;
        // An async function gives its future, to be awaited
        match called.clone().into_future() {
            Ok(future) => future.await.into_result(),
            Err(_) => Ok(called),
        }
    })
    .await
    .map_err(|error| error.to_string())?;

    if rune::from_value::<()>(output.clone()).is_ok() {
        return Ok(None);
    }
    let formatted = vm
        .call(["__serval_display"], (output.clone(),))
        .or_else(|_| vm.call(["__serval_debug"], (output,)))
        .and_then(|text| rune::from_value::<String>(text).map_err(Into::into))
        .map_err(|error| error.to_string())?;
    Ok(Some(formatted))
}

/// A task's future, given [`INSTRUCTIONS_PER_STEP`] anew on each poll.
/// Takes what it printed, by lines, and sets its stream generator on each.
struct Stepped<'a, F> {
    future: Pin<Box<F>>,
    io: CaptureIo,
    stream_of: Rc<Function>,
    on_print: Box<dyn FnMut(&str) + 'a>,
    /// Printed, yet to end its line
    partial: String,
}

impl<'a, F> Stepped<'a, F> {
    fn new(
        io: CaptureIo,
        stream_of: Rc<Function>,
        on_print: Box<dyn FnMut(&str) + 'a>,
        future: F,
    ) -> Self {
        Self {
            future: Box::pin(future),
            io,
            stream_of,
            on_print,
            partial: String::new(),
        }
    }
}

impl<F, T> Future for Stepped<'_, F>
where
    F: Future<Output = Result<T, rune::runtime::VmError>>,
{
    type Output = Result<T, StepError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        serval::set_stream_of(self.stream_of.clone());
        let future = self.future.as_mut();
        let (poll, exhausted) = budget::with(INSTRUCTIONS_PER_STEP, || {
            let poll = future.poll(cx);
            (poll, !budget::acquire().take())
        })
        .call();
        if let Ok(printed) = self.io.drain_utf8() {
            self.partial.push_str(&printed);
        }
        if let Some(end) = self.partial.rfind('\n') {
            let lines: String = self.partial.drain(..=end).collect();
            for line in lines.lines() {
                (self.on_print)(line);
            }
        }
        if poll.is_ready() && !self.partial.is_empty() {
            let line = std::mem::take(&mut self.partial);
            (self.on_print)(&line);
        }
        match poll {
            Poll::Ready(Err(_)) if exhausted => Poll::Ready(Err(StepError::Exhausted)),
            Poll::Ready(result) => Poll::Ready(result.map_err(StepError::Vm)),
            Poll::Pending => Poll::Pending,
        }
    }
}

enum StepError {
    Vm(rune::runtime::VmError),
    /// It ran past [`INSTRUCTIONS_PER_STEP`] without awaiting.
    Exhausted,
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vm(error) => write!(f, "{error}"),
            Self::Exhausted => write!(
                f,
                "Ran past {INSTRUCTIONS_PER_STEP} instructions without awaiting"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use super::{compile_plugin, run_task};

    /// Runs `future`, which must not wait on anything outside it.
    fn block_on<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

    fn run(source: &str, name: &str) -> Result<Option<String>, String> {
        let plugin = compile_plugin("test.rn", source)?;
        block_on(run_task(&plugin, name, |_| {}))
    }

    #[test]
    fn what_a_task_prints_is_taken_by_lines() {
        let mut printed = Vec::new();
        let source = "pub async fn talks() {\n\
            println!(\"first\");\n    print!(\"second {}\\n\\n\", 2);\n    \
            helper().await;\n    dbg!(42);\n    print!(\"unended\");\n    \"done\"\n}\n\
            async fn helper() { println!(\"from a helper\") }\n";
        let plugin = compile_plugin("test.rn", source).unwrap();
        let returned = block_on(run_task(&plugin, "talks", |line| {
            printed.push(line.to_string())
        }));
        assert_eq!(returned, Ok(Some("done".into())));
        assert_eq!(
            printed,
            ["first", "second 2", "", "from a helper", "42", "unended"]
        );
    }

    #[test]
    fn what_a_task_returns_is_formatted() {
        let source = "pub async fn number() { 1 + 2 }\n\
            pub async fn text() { helper().await }\n\
            async fn helper() { \"done\" }\n\
            pub async fn list() { [1, 2] }\n\
            pub async fn object() { #{ ok: true } }\n\
            pub async fn nothing() {}\n";
        assert_eq!(run(source, "number"), Ok(Some("3".into())));
        assert_eq!(run(source, "text"), Ok(Some("done".into())));
        assert_eq!(run(source, "list"), Ok(Some("[1, 2]".into())));
        // Of no display: as Rune debugs it
        assert_eq!(run(source, "object"), Ok(Some("{\"ok\": true}".into())));
        assert_eq!(run(source, "nothing"), Ok(None));
    }

    #[test]
    fn what_fails_is_told() {
        let source = "pub async fn fails() { panic!(\"broken\") }\n\
            pub async fn divides() { 1 / 0 }\n";
        assert!(run(source, "fails").unwrap_err().contains("broken"));
        assert!(run(source, "divides").is_err());
    }

    #[test]
    fn a_loop_of_no_await_is_stopped() {
        let source = "pub async fn spins() { loop {} }\n";
        assert_eq!(
            run(source, "spins").unwrap_err(),
            "Ran past 10000000 instructions without awaiting"
        );
    }

    #[test]
    fn what_does_not_compile_is_told_where() {
        let error = compile_plugin(
            "broken.rn",
            "pub async fn ok() {}\n\npub async fn calls() {\n    missing()\n}\n",
        )
        .err()
        .unwrap();
        assert!(error.contains("missing"), "{error}");
        assert!(error.contains("broken.rn:4:"), "{error}");
        assert!(error.contains("    missing()"), "{error}");
        let error = compile_plugin("syntax.rn", "pub async fn broken( {\n")
            .err()
            .unwrap();
        assert!(error.contains("syntax.rn:1:"), "{error}");
    }
}
