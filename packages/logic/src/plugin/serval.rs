//! The Rune modules `serval::time`, `serval::data`, `serval::serial`,
//! `serval::math` (methods of `f64`) and `serval::rand`.
//!
//! Rune functions must be `Send`, so they take Serval's contexts from a
//! thread-local [`Host`] instead of holding them.
//!
//! What fails is returned as an `Err` (the plugin takes it, as with `?`), but
//! where the API followed (`rune-modules`, `rand`, `num_traits` or Rune's own
//! streams) stops the task instead: a stream's items, `Instant::now`, an empty
//! range of `rand`, `clamp` of no range.

mod data;
mod math;
mod random;
mod serial;
mod time;

use std::{cell::RefCell, rc::Rc};

use rune::{
    ContextError, Module, Value,
    runtime::{Function, VmResult},
};

use crate::{data::DataContext, plugin::ErrorText, serial::SerialContext, time::TimeContext};

/// What the tasks run on.
#[derive(Clone)]
pub struct Host {
    pub data: DataContext,
    pub serial: SerialContext,
    pub time: TimeContext,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// Sets what the tasks run on, from then on.
pub fn set_host(host: Host) {
    HOST.with(|cell| *cell.borrow_mut() = Some(host));
}

fn host() -> Result<Host, String> {
    HOST.with(|cell| cell.borrow().clone())
        .ok_or_else(|| "Serval is not there to run on".to_string())
}

/// The modules, each of its functions.
pub fn modules() -> Result<[Module; 5], ContextError> {
    Ok([
        time::module()?,
        data::module()?,
        serial::module()?,
        math::module()?,
        random::module()?,
    ])
}

thread_local! {
    /// The async generator of the task polled, which makes a stream of a
    /// reader: Rune's streams are of its generators alone.
    static STREAM_OF: RefCell<Option<Rc<Function>>> = const { RefCell::new(None) };
}

/// Sets the generator the streams are made of, for the task to be polled.
pub fn set_stream_of(generator: Rc<Function>) {
    STREAM_OF.with(|cell| *cell.borrow_mut() = Some(generator));
}

/// A Rune `Stream` of `reader`, by its async `next(with_time)`.
fn stream_of(reader: impl rune::ToValue, with_time: bool) -> Result<Value, String> {
    let generator = STREAM_OF
        .with(|cell| cell.borrow().clone())
        .ok_or_else(|| "No task to make a stream for".to_string())?;
    generator.call((reader, with_time)).into_result().err_text()
}

fn to_value(value: impl rune::ToValue) -> Result<Value, String> {
    rune::to_value(value).err_text()
}

/// `result` as a native function of Rune's protocols (as a stream's `next`)
/// gives it: an error stops the task.
fn or_stop<T>(result: Result<T, String>) -> VmResult<T> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(error) => VmResult::panic(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::plugin::{
        compile_plugin, run_task,
        testing::{block_on, run_body},
    };

    #[test]
    fn serval_is_reached_by_its_paths() {
        let plugin = compile_plugin(
            "paths.rn",
            "use serval::{data, serial, time};\n\
            pub async fn now() { time::Instant::now() }\n\
            pub async fn spans() {\n\
                let span = time::Duration::from_millis(1500);\n\
                span += time::Duration::from_secs(1);\n\
                let longer = span > time::Duration::from_secs(2);\n\
                format!(\"{} {} {} {:?}\", span.as_millis(), span.as_secs(), longer, span)\n\
            }\n\
            pub async fn sleeps_twice() {\n\
                let span = time::Duration::from_millis(1);\n\
                let first = time::sleep(span).await;\n\
                let second = time::sleep(span).await;\n\
                format!(\"{:?} {:?} {}\", first, second, span.as_millis())\n\
            }\n\
            pub async fn label() { data::get(\"temp\") }\n\
            pub async fn settings() { serial::get_settings() }\n\
            pub async fn entry() { data::LogEntry::new(Some(\"INFO\"), \"up\").message }\n",
        )
        .unwrap();
        // Off the page: told so, as an error the plugin may take
        for name in ["settings", "label"] {
            assert_eq!(
                block_on(run_task(&plugin, name, |_| {})),
                Ok(Some("Err(\"Serval is not there to run on\")".into())),
                "{name}"
            );
        }
        assert_eq!(
            block_on(run_task(&plugin, "entry", |_| {})),
            Ok(Some("up".into()))
        );
        // Its duration left to it, to wait again
        assert_eq!(
            block_on(run_task(&plugin, "sleeps_twice", |_| {})),
            Ok(Some(
                "Err(\"Serval is not there to run on\") Err(\"Serval is not there to run on\") 1"
                    .into()
            ))
        );
        assert_eq!(
            block_on(run_task(&plugin, "spans", |_| {})),
            Ok(Some("2500 2 true 2.5s".into()))
        );
        // As `rune-modules`' `Instant::now`, of no error to give: the task stopped
        let failed = block_on(run_task(&plugin, "now", |_| {})).unwrap_err();
        assert!(failed.contains("Serval is not there to run on"), "{failed}");
    }

    /// The `rune` code blocks of `docs`, by its lines.
    fn rune_blocks<'a>(docs: impl Iterator<Item = &'a str>) -> Vec<String> {
        let mut blocks = Vec::new();
        let mut block: Option<String> = None;
        for line in docs {
            match (&mut block, line) {
                (None, "```rune") => block = Some(String::new()),
                (Some(_), "```") => blocks.extend(block.take()),
                (Some(code), line) => {
                    code.push_str(line);
                    code.push('\n');
                }
                (None, _) => {}
            }
        }
        blocks
    }

    /// The examples of the modules' docs, each a plugin: of their doc comments,
    /// and of `wait_for`'s, which has none.
    fn doc_examples() -> Vec<(&'static str, String)> {
        let files = [
            ("data.rs", include_str!("serval/data.rs")),
            ("math.rs", include_str!("serval/math.rs")),
            ("random.rs", include_str!("serval/random.rs")),
            ("serial.rs", include_str!("serval/serial.rs")),
            ("time.rs", include_str!("serval/time.rs")),
        ];
        let mut examples = Vec::new();
        for (file, source) in files {
            let docs = source.lines().filter_map(|line| {
                let doc = line.trim_start().strip_prefix("///")?;
                Some(doc.strip_prefix(' ').unwrap_or(doc))
            });
            examples.extend(rune_blocks(docs).into_iter().map(|block| (file, block)));
        }
        let wait_for = rune_blocks(super::serial::WAIT_FOR_DOCS.lines());
        examples.extend(wait_for.into_iter().map(|block| ("serial.rs", block)));
        examples
    }

    #[test]
    fn doc_examples_compile() {
        let examples = doc_examples();
        assert!(!examples.is_empty(), "{} examples", examples.len());
        for (file, example) in examples {
            if let Err(error) = compile_plugin("example.rn", &example) {
                panic!("an example of {file} does not compile:\n{example}\n{error}");
            }
        }
    }

    #[test]
    fn wait_for_takes_a_limit_or_its_default() {
        let waits = |arguments: &str| {
            run_body(&format!(
                "use serval::serial;\nserial::wait_for({arguments}).await"
            ))
        };
        // Of the default: as far as there is a port to wait on
        assert_eq!(
            waits("\"OK\""),
            Ok(Some("Err(\"Serval is not there to run on\")".into()))
        );
        assert_eq!(
            waits("\"OK\", 1"),
            Ok(Some("Err(\"Not a limit of at least 2 bytes\")".into()))
        );
        let failed = waits("\"OK\", 10, 3").unwrap_err();
        assert!(failed.contains("takes a pattern and a limit"), "{failed}");
    }
}
