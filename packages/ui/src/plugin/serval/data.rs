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

/// A log: a message, and its level.
///
/// What `get` gives of a Log label, and what `push` takes.
///
/// * `level`: the level's name (`"ERROR"`, `"WARN"`, `"INFO"`, `"DEBUG"`, or
///   `"TRACE"`), or `None` for none.
/// * `message`: the text.
///
/// # Examples
///
/// ```rune
/// use serval::data;
///
/// /// Prints each warning of `log`.
/// pub async fn warnings() {
///     let logs = data::get("log")?;
///     while let Some(log) = logs.next().await {
///         if log.level == Some("WARN") {
///             println!("{}", log.message);
///         }
///     }
/// }
/// ```
#[derive(Any, Debug, Clone)]
#[rune(item = ::serval::data)]
pub struct LogEntry {
    #[rune(get, set)]
    pub level: Option<String>,
    #[rune(get, set)]
    pub message: String,
}

impl LogEntry {
    /// A log of `level` (a level's name, or `None`) and `message`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use serval::data;
    ///
    /// pub async fn alert() {
    ///     data::push("alerts", data::LogEntry::new(Some("WARN"), "battery low"))?;
    /// }
    /// ```
    #[rune::function(path = Self::new)]
    fn new(level: Option<String>, message: String) -> Self {
        Self { level, message }
    }
}

/// The data: values under labels, read as they arrive and pushed to.
///
/// Each type is read and pushed as:
///
/// * Number: a float (an integer is taken too).
/// * Vector2d: a tuple of two floats, `(x, y)`.
/// * Vector3d: a tuple of three floats, `(x, y, z)`.
/// * Log: a `LogEntry` (a string is taken too, as a log of no level).
/// * Bytes: `Bytes`.
///
/// # Examples
///
/// ```rune
/// use serval::data;
///
/// /// Pushes the power, of the voltage and the current, to `power`.
/// pub async fn power() {
///     let volts = data::get("volt")?;
///     let amps = data::get("amp")?;
///     loop {
///         let volt = volts.next().await.unwrap();
///         let amp = amps.next().await.unwrap();
///         data::push("power", volt * amp)?;
///     }
/// }
/// ```
#[rune::module(::serval::data)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<LogEntry>()?;
    module.function_meta(LogEntry::new)?;
    module.ty::<Reader>()?;
    module.function("next", next).build_associated::<Reader>()?;
    module.function_meta(get__meta)?;
    module.function_meta(get_with_time__meta)?;
    module.function_meta(push__meta)?;
    Ok(module)
}

/// The values of `label`, as a stream.
///
/// It gives those the label keeps, then each new one as it arrives.
///
/// * `next().await` waits for the next value. It never gives `None`: the
///   stream does not end.
/// * The label need not be there yet: the stream waits for it.
/// * If the label is cleared, the stream reads it again from its start.
///
/// # Examples
///
/// ```rune
/// use serval::data;
///
/// /// Pushes the mean of all the temperatures so far to `temp mean`.
/// pub async fn mean() {
///     let temps = data::get("temp")?;
///     let sum = 0.0;
///     let count = 0.0;
///     while let Some(temp) = temps.next().await {
///         sum += temp;
///         count += 1.0;
///         data::push("temp mean", sum / count)?;
///     }
/// }
/// ```
#[rune::function(keep)]
fn get(label: &str) -> Result<Value, String> {
    stream(label, false)
}

/// As `get`, each value with the time it arrived.
///
/// Each item is `(time, value)`, its time a `time::Instant`.
///
/// # Examples
///
/// ```rune
/// use serval::data;
///
/// /// Prints how long after the one before each voltage arrives.
/// pub async fn intervals() {
///     let volts = data::get_with_time("volt")?;
///     let last = None;
///     while let Some((time, volt)) = volts.next().await {
///         if let Some(previous) = last {
///             println!("{} ms", time.duration_since(previous).as_millis());
///         }
///         last = Some(time);
///     }
/// }
/// ```
#[rune::function(keep)]
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

/// The reader behind the streams of `get` and `get_with_time`.
// It reads its label as Serval's own readers do.
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

/// Pushes `value` to `label`, at the current time.
///
/// The label's type is that of `value`:
///
/// * A float or an integer: Number.
/// * `(x, y)`: Vector2d.
/// * `(x, y, z)`: Vector3d.
/// * A string, or a `LogEntry`: Log.
/// * `Bytes`: Bytes.
///
/// A value of another type than the label's replaces the label.
///
/// # Examples
///
/// ```rune
/// use serval::data;
///
/// pub async fn values() {
///     data::push("speed", 1.5)?;
///     data::push("position", (1.0, 2.0))?;
///     data::push("attitude", (0.0, 0.5, 1.0))?;
///     data::push("note", "started")?;
///     data::push("alerts", data::LogEntry::new(Some("WARN"), "battery low"))?;
///     data::push("frame", b"\x02ok\x03")?;
/// }
/// ```
#[rune::function(keep)]
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
