//! `serval::data`: the data's labels, read as streams and pushed to.

use std::collections::VecDeque;

use rune::{
    Any, ContextError, Module, Value,
    runtime::{Bytes, Mut, VmResult},
};

use super::{host, or_stop, stream_of, time::Instant, to_value};
use crate::{
    data::{
        ByteData, DataEntry, DataType, Log, LogData, LogLevel, NewEntries, NumberData,
        SourceCursor, Vector2d, Vector2dData, Vector3d, Vector3dData,
    },
    plugin::ErrorText,
};

/// A log, as `serval::data::get` gives it and `push` takes it.
#[derive(Any, Debug, Clone)]
#[rune(item = ::serval::data)]
pub struct LogEntry {
    /// Its level's name, as `INFO`; none for none.
    #[rune(get, set)]
    pub level: Option<String>,
    #[rune(get, set)]
    pub message: String,
}

impl LogEntry {
    #[rune::function(path = Self::new)]
    fn new(level: Option<String>, message: String) -> Self {
        Self { level, message }
    }
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("serval", ["data"])?;
    module.ty::<LogEntry>()?;
    module.function_meta(LogEntry::new)?;
    module.ty::<Reader>()?;
    module.function("next", next).build_associated::<Reader>()?;
    module.function("get", get).build()?;
    module.function("get_with_time", get_with_time).build()?;
    module.function("push", push).build()?;
    Ok(module)
}

/// The entries of `label` as a Rune `Stream`: the kept ones, then each new
/// one, waited for. It does not end: the label gone (as all the data is
/// cleared), it waits for it again, read anew from its start.
fn get(label: &str) -> Result<Value, String> {
    stream(label, false)
}

/// As `get`, each value with its time (an `Instant`).
fn get_with_time(label: &str) -> Result<Value, String> {
    stream(label, true)
}

fn stream(label: &str, with_time: bool) -> Result<Value, String> {
    let reader = Reader {
        label: label.to_string(),
        cursor: SourceCursor::default(),
        taken: VecDeque::new(),
        let_go: host()?.time.current(),
    };
    stream_of(reader, with_time)
}

/// An entry as plugins have it: its time and value.
trait ToRune {
    fn to_rune(&self) -> Result<(i64, Value), String>;
}

impl ToRune for NumberData {
    fn to_rune(&self) -> Result<(i64, Value), String> {
        Ok((self.timestamp(), to_value(*self.value())?))
    }
}

impl ToRune for Vector2dData {
    fn to_rune(&self) -> Result<(i64, Value), String> {
        let v = self.value();
        Ok((self.timestamp(), to_value((v.x(), v.y()))?))
    }
}

impl ToRune for Vector3dData {
    fn to_rune(&self) -> Result<(i64, Value), String> {
        let v = self.value();
        Ok((self.timestamp(), to_value((v.x(), v.y(), v.z()))?))
    }
}

impl ToRune for LogData {
    fn to_rune(&self) -> Result<(i64, Value), String> {
        let log = self.value();
        let entry = LogEntry {
            level: (log.level() != LogLevel::None).then(|| log.level().to_string()),
            message: log.message().to_string(),
        };
        Ok((self.timestamp(), to_value(entry)?))
    }
}

impl ToRune for ByteData {
    fn to_rune(&self) -> Result<(i64, Value), String> {
        let bytes = Bytes::from_slice(self.value().as_slice()).err_text()?;
        Ok((self.timestamp(), to_value(bytes)?))
    }
}

/// How often a reader looks for what is new, as it has none.
const NEW_ENTRIES_CHECK_MS: u32 = 10;

/// How long a reader gives what it has without letting the page go on, in ms:
/// so a task that takes many entries at once does not stand the page still.
const LET_GO_EVERY_MS: i64 = 16;

/// What a stream of `serval::data::get` reads its label by, as Serval's own
/// readers do.
#[derive(Any, Debug)]
#[rune(item = ::serval::data)]
pub struct Reader {
    label: String,
    cursor: SourceCursor,
    /// Read from the queue, yet to be given
    taken: VecDeque<(i64, Value)>,
    /// When it last let the page go on
    let_go: i64,
}

impl Reader {
    /// Takes what is new to it from the queue, if the label is there.
    fn take(&mut self) -> Result<(), String> {
        fn taken<T: DataEntry + ToRune>(
            read: Option<NewEntries<T>>,
        ) -> Result<Option<Vec<(i64, Value)>>, String> {
            read.map(|read| read.entries.iter().map(ToRune::to_rune).collect())
                .transpose()
        }

        let data = host()?.data;
        let (label, cursor) = (self.label.as_str(), &mut self.cursor);
        let read = match data.data_type_of(label) {
            Some(DataType::Number) => taken(cursor.new_entries::<NumberData>(&data, label))?,
            Some(DataType::Vector2d) => taken(cursor.new_entries::<Vector2dData>(&data, label))?,
            Some(DataType::Vector3d) => taken(cursor.new_entries::<Vector3dData>(&data, label))?,
            Some(DataType::Log) => taken(cursor.new_entries::<LogData>(&data, label))?,
            Some(DataType::Bytes) => taken(cursor.new_entries::<ByteData>(&data, label))?,
            None => None,
        };
        self.taken.extend(read.into_iter().flatten());
        Ok(())
    }

    /// The next entry, waited for.
    async fn next_entry(&mut self) -> Result<(i64, Value), String> {
        let time = host()?.time;
        loop {
            if self.taken.is_empty() {
                self.take()?;
            }
            if let Some(entry) = self.taken.pop_front() {
                if time.current() - self.let_go >= LET_GO_EVERY_MS {
                    time.after_ms(0).await;
                    self.let_go = time.current();
                }
                return Ok(entry);
            }
            time.after_ms(NEW_ENTRIES_CHECK_MS).await;
            self.let_go = time.current();
        }
    }
}

/// The next value (with its time, if `with_time`), waited for: never none, as
/// the stream does not end. Errors stop the task: a generator has no error to
/// give.
async fn next(mut reader: Mut<Reader>, with_time: bool) -> VmResult<Option<Value>> {
    or_stop(
        async {
            let (time, value) = reader.next_entry().await?;
            if !with_time {
                return Ok(Some(value));
            }
            to_value((Instant { ms: time }, value)).map(Some)
        }
        .await,
    )
}

/// Pushes `value` to `label`: a number, a 2- or 3-tuple of numbers (vector),
/// a string or [`LogEntry`] (log), or bytes. Another type replaces the label.
fn push(label: &str, value: Value) -> Result<(), String> {
    let host = host()?;
    let (mut data, now) = (host.data, host.time.current());
    if let Ok(number) = number(&value) {
        data.push(label, NumberData::new(now, number));
    } else if let Ok((x, y)) = rune::from_value::<(Value, Value)>(value.clone()) {
        data.push(
            label,
            Vector2dData::new(now, Vector2d::new(number(&x)?, number(&y)?)),
        );
    } else if let Ok((x, y, z)) = rune::from_value::<(Value, Value, Value)>(value.clone()) {
        let vector = Vector3d::new(number(&x)?, number(&y)?, number(&z)?);
        data.push(label, Vector3dData::new(now, vector));
    } else if let Ok(text) = rune::from_value::<String>(value.clone()) {
        data.push(label, LogData::new(now, Log::new(LogLevel::None, text)));
    } else if let Ok(entry) = rune::from_value::<LogEntry>(value.clone()) {
        let level = match entry.level {
            Some(level) => level.parse().map_err(|error| format!("{error}"))?,
            None => LogLevel::None,
        };
        data.push(label, LogData::new(now, Log::new(level, entry.message)));
    } else if let Ok(bytes) = rune::from_value::<Bytes>(value.clone()) {
        data.push(label, ByteData::new(now, bytes.as_slice().to_vec()));
    } else {
        return Err(format!("Cannot push a {}", value.type_info()));
    }
    Ok(())
}

/// A number, of an integer or a float.
fn number(value: &Value) -> Result<f64, String> {
    rune::from_value::<f64>(value.clone())
        .or_else(|_| value.as_integer::<i64>().map(|integer| integer as f64))
        .map_err(|_| format!("Not a number: {}", value.type_info()))
}
