//! The Rune modules `serval::time`, `serval::data`, `serval::serial`,
//! `serval::math` (methods of `f64`) and `serval::rand`.
//!
//! Rune functions must be `Send`, so they take Serval's contexts from a
//! thread-local [`Host`] instead of holding them.

mod clock;
mod math;
mod random;

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use rune::{
    Any, ContextError, Module, Value,
    runtime::{Bytes, Function, Mut, VmResult},
};

use crate::{
    data::{
        ByteData, DataContext, DataEntry, DataType, Log, LogData, LogLevel, NewEntries, NumberData,
        SourceCursor, Vector2d, Vector2dData, Vector3d, Vector3dData,
    },
    serial::{Restart, SerialContext},
    time::TimeContext,
};

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
        time()?,
        data()?,
        serial()?,
        math::module()?,
        random::module()?,
    ])
}

fn time() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("serval", ["time"])?;
    clock::install(&mut module)?;
    module.function("sleep", sleep).build()?;
    module.function("sleep_until", sleep_until).build()?;
    Ok(module)
}

/// Waits `duration`, by its ms.
async fn sleep(duration: clock::Duration) -> Result<(), String> {
    let time = host()?.time;
    let ms = u32::try_from(duration.inner.as_millis()).unwrap_or(u32::MAX);
    let wait = time.after_ms(ms);
    wait.await;
    Ok(())
}

/// How often `sleep_until` asks its condition again.
const CONDITION_CHECK_MS: u32 = 10;

/// Waits until `condition` (a function of no argument, async or not) gives
/// `true`, checked every [`CONDITION_CHECK_MS`]; an `Err` from it is returned.
async fn sleep_until(condition: Function) -> Result<(), String> {
    let time = host()?.time;
    loop {
        let mut met: Value = condition
            .call(())
            .into_result()
            .map_err(|error| error.to_string())?;
        if let Ok(future) = met.clone().into_future() {
            met = future
                .await
                .into_result()
                .map_err(|error| error.to_string())?;
        }
        let met = match rune::from_value::<bool>(met.clone()) {
            Ok(met) => met,
            Err(_) => match rune::from_value::<Result<bool, Value>>(met.clone()) {
                Ok(Ok(met)) => met,
                Ok(Err(error)) => return Err(format!("The condition failed: {error:?}")),
                Err(_) => {
                    let found = met.type_info();
                    return Err(format!("The condition gave a {found}, not a bool"));
                }
            },
        };
        if met {
            return Ok(());
        }
        time.after_ms(CONDITION_CHECK_MS).await;
    }
}

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

fn data() -> Result<Module, ContextError> {
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

thread_local! {
    /// The async generator of the task polled, which makes a stream of a
    /// reader: Rune's streams are of its generators alone.
    static STREAM_OF: RefCell<Option<Rc<Function>>> = const { RefCell::new(None) };
}

/// Sets the generator the streams are made of, for the task to be polled.
pub fn set_stream_of(generator: Rc<Function>) {
    STREAM_OF.with(|cell| *cell.borrow_mut() = Some(generator));
}

/// The entries of `label` as a Rune `Stream`: the kept ones, then each new
/// one, waited for. It ends once the label is gone.
fn get(label: &str) -> VmResult<Value> {
    stream(label, false)
}

/// As `get`, each value with its time (an `Instant`).
fn get_with_time(label: &str) -> VmResult<Value> {
    stream(label, true)
}

fn stream(label: &str, with_time: bool) -> VmResult<Value> {
    let reader = match host() {
        Ok(host) => Reader {
            label: label.to_string(),
            cursor: SourceCursor::default(),
            taken: VecDeque::new(),
            seen: false,
            let_go: host.time.current(),
        },
        Err(error) => return VmResult::panic(error),
    };
    stream_of(reader, with_time)
}

/// A Rune `Stream` of `reader`, by its async `next(with_time)`.
fn stream_of(reader: impl rune::ToValue, with_time: bool) -> VmResult<Value> {
    let Some(generator) = STREAM_OF.with(|cell| cell.borrow().clone()) else {
        return VmResult::panic("No task to make a stream for");
    };
    generator.call((reader, with_time))
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
        let bytes =
            Bytes::from_slice(self.value().as_slice()).map_err(|error| error.to_string())?;
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
    /// Whether the label has been there: it ends as the label is gone.
    seen: bool,
    /// When it last let the page go on
    let_go: i64,
}

impl Reader {
    /// Takes what is new to it from the queue; whether the label is there.
    fn take(&mut self) -> Result<bool, String> {
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
        let Some(entries) = read else {
            return Ok(false);
        };
        self.seen = true;
        self.taken.extend(entries);
        Ok(true)
    }

    /// The next entry, waited for: none as the label is gone.
    async fn next_entry(&mut self) -> Result<Option<(i64, Value)>, String> {
        let time = host()?.time;
        loop {
            if self.taken.is_empty() && !self.take()? && self.seen {
                return Ok(None);
            }
            if let Some(entry) = self.taken.pop_front() {
                if time.current() - self.let_go >= LET_GO_EVERY_MS {
                    time.after_ms(0).await;
                    self.let_go = time.current();
                }
                return Ok(Some(entry));
            }
            time.after_ms(NEW_ENTRIES_CHECK_MS).await;
            self.let_go = time.current();
        }
    }
}

/// The next value (with its time, if `with_time`); none once the label is
/// gone. Errors stop the task: a generator has no error to give.
async fn next(mut reader: Mut<Reader>, with_time: bool) -> VmResult<Option<Value>> {
    let entry = match reader.next_entry().await {
        Ok(entry) => entry,
        Err(error) => return VmResult::panic(error),
    };
    match entry {
        Some((time, value)) if with_time => to_value((clock::Instant { ms: time }, value))
            .map_or_else(VmResult::panic, |entry| VmResult::Ok(Some(entry))),
        entry => VmResult::Ok(entry.map(|(_, value)| value)),
    }
}

fn to_value(value: impl rune::ToValue) -> Result<Value, String> {
    rune::to_value(value).map_err(|error| error.to_string())
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

fn serial() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("serval", ["serial"])?;
    module.function("send", send).build()?;
    module
        .function("get_send_buffer", get_send_buffer)
        .build()?;
    module.function("flush", flush).build()?;
    module.ty::<ReceivedReader>()?;
    module
        .function("next", next_received)
        .build_associated::<ReceivedReader>()?;
    module.function("received", received).build()?;
    module.function("wait_for", wait_for).build()?;
    module.function("open", open).build()?;
    module.function("close", close).build()?;
    module.function("get_baudrate", get_baudrate).build()?;
    module.function("set_baudrate", set_baudrate).build()?;
    module.function("get_dtr", get_dtr).build()?;
    module.function("get_rts", get_rts).build()?;
    module.function("set_dtr", set_dtr).build()?;
    module.function("set_rts", set_rts).build()?;
    module.function("toggle_dtr", toggle_dtr).build()?;
    module.function("toggle_rts", toggle_rts).build()?;
    module.function("restart", restart).build()?;
    Ok(module)
}

/// Pushes `data` to the send buffer, sent as the port takes it: a string as
/// UTF-8, bytes, or a vector of byte values.
fn send(data: Value) -> Result<(), String> {
    let serial = host()?.serial;
    if !serial.is_open() {
        return Err("No port is open".to_string());
    }
    serial.send(to_bytes(data)?);
    Ok(())
}

/// The bytes of a string, bytes or a list of bytes.
fn to_bytes(data: Value) -> Result<Vec<u8>, String> {
    if let Ok(text) = rune::from_value::<String>(data.clone()) {
        Ok(text.into_bytes())
    } else if let Ok(bytes) = rune::from_value::<Bytes>(data.clone()) {
        Ok(bytes.as_slice().to_vec())
    } else if let Ok(values) = rune::from_value::<Vec<i64>>(data.clone()) {
        values
            .into_iter()
            .map(|value| u8::try_from(value).map_err(|_| format!("Not a byte: {value}")))
            .collect()
    } else {
        Err(format!("Not bytes: {}", data.type_info()))
    }
}

/// What `send` was given that is yet to be written to the port, oldest first
/// (not of a file being sent).
fn get_send_buffer() -> Result<Bytes, String> {
    let unsent = host()?.serial.unsent();
    Bytes::from_slice(unsent.as_slice()).map_err(|error| error.to_string())
}

/// How often `flush` looks whether all is written.
const FLUSH_CHECK_MS: u32 = 10;

/// Waits until all that was sent (and a file being sent) is written to the
/// port, or failed to be.
async fn flush() -> Result<(), String> {
    let host = host()?;
    while host.serial.has_pending() {
        host.time.after_ms(FLUSH_CHECK_MS).await;
    }
    Ok(())
}

/// How often what is received is looked for, as there is none new.
const RECEIVED_CHECK_MS: u32 = 10;

/// What the port receives from when it was made, as `serial::received` reads
/// it: from the console's history, apart from the data.
#[derive(Any, Debug)]
#[rune(item = ::serval::serial)]
pub struct ReceivedReader {
    /// Where it read to, counted from the first byte ever received
    read_to: u64,
}

impl ReceivedReader {
    fn new() -> Result<Self, String> {
        Ok(Self {
            read_to: host()?.serial.received_since(None).end,
        })
    }

    /// What came since it last read, waited for as none did.
    async fn next_bytes(&mut self) -> Result<Vec<u8>, String> {
        let host = host()?;
        loop {
            let unread = host.serial.received_since(Some(self.read_to));
            self.read_to = unread.end;
            if !unread.bytes.is_empty() {
                return Ok(unread.bytes);
            }
            host.time.after_ms(RECEIVED_CHECK_MS).await;
        }
    }
}

/// The bytes the port receives from now on as a Rune `Stream`, each item what
/// came at once (as `Bytes`). It does not end.
fn received() -> VmResult<Value> {
    match ReceivedReader::new() {
        Ok(reader) => stream_of(reader, false),
        Err(error) => VmResult::panic(error),
    }
}

async fn next_received(
    mut reader: Mut<ReceivedReader>,
    _with_time: bool,
) -> VmResult<Option<Value>> {
    let bytes = match reader.next_bytes().await {
        Ok(bytes) => bytes,
        Err(error) => return VmResult::panic(error),
    };
    match Bytes::from_slice(bytes.as_slice()).map_err(|error| error.to_string()) {
        Ok(bytes) => {
            to_value(bytes).map_or_else(VmResult::panic, |bytes| VmResult::Ok(Some(bytes)))
        }
        Err(error) => VmResult::panic(error),
    }
}

/// Waits until the port receives `pattern` (a string, bytes or a list of
/// bytes) from now on: what it received until then, `pattern` with it.
async fn wait_for(pattern: Value) -> Result<Bytes, String> {
    let pattern = to_bytes(pattern)?;
    if pattern.is_empty() {
        return Ok(Bytes::new());
    }
    let mut reader = ReceivedReader::new()?;
    let mut received = Vec::new();
    loop {
        // From where it could start in what came before
        let from = received
            .len()
            .saturating_sub(pattern.len().saturating_sub(1));
        received.extend(reader.next_bytes().await?);
        if let Some(at) = received[from..]
            .windows(pattern.len())
            .position(|window| window == pattern.as_slice())
        {
            received.truncate(from + at + pattern.len());
            return Bytes::from_slice(received.as_slice()).map_err(|error| error.to_string());
        }
    }
}

/// Opens the port selected, at its baudrate: open as it returns.
async fn open() -> Result<(), String> {
    host()?.serial.open_port().await
}

/// Closes the open port: closed as it returns.
async fn close() -> Result<(), String> {
    host()?.serial.close_port().await
}

/// The baudrate of the port selected, if it has one.
fn get_baudrate() -> Result<Option<i64>, String> {
    let port = host()?.serial.selected_port();
    Ok(port.and_then(|port| port.baudrate).map(i64::from))
}

/// Sets the baudrate of the port selected, for it to be opened at.
fn set_baudrate(baudrate: i64) -> Result<(), String> {
    let baudrate = u32::try_from(baudrate)
        .ok()
        .filter(|&baudrate| baudrate > 0)
        .ok_or_else(|| format!("Not a baudrate: {baudrate}"))?;
    host()?.serial.set_baudrate(baudrate);
    Ok(())
}

fn get_dtr() -> Result<bool, String> {
    Ok(host()?.serial.signals().dtr)
}

fn get_rts() -> Result<bool, String> {
    Ok(host()?.serial.signals().rts)
}

/// Sets DTR on the open port: set as it returns.
async fn set_dtr(on: bool) -> Result<(), String> {
    host()?.serial.write_signals(Some(on), None).await
}

async fn set_rts(on: bool) -> Result<(), String> {
    host()?.serial.write_signals(None, Some(on)).await
}

async fn toggle_dtr() -> Result<(), String> {
    set_dtr(!get_dtr()?).await
}

async fn toggle_rts() -> Result<(), String> {
    set_rts(!get_rts()?).await
}

/// Restarts the board on the open port, as `kind` is (`"arduino"` or
/// `"espressif"`, for ESP32 and the like): restarted as it returns.
async fn restart(kind: String) -> Result<(), String> {
    let restart = match kind.to_ascii_lowercase().as_str() {
        "arduino" => Restart::Arduino,
        "espressif" | "esp32" | "esp8266" => Restart::Espressif,
        _ => return Err(format!("No restart of {kind}: arduino or espressif")),
    };
    host()?.serial.restart_board(restart).await
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use crate::plugin::{compile_plugin, run_task};

    pub(super) fn block_on<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

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
            pub async fn label() { data::get(\"temp\") }\n\
            pub async fn baudrate() { serial::get_baudrate() }\n\
            pub async fn entry() { data::LogEntry::new(Some(\"INFO\"), \"up\").message }\n",
        )
        .unwrap();
        // Off the page: told so, as an error the plugin may take
        assert_eq!(
            block_on(run_task(&plugin, "baudrate", |_| {})),
            Ok(Some("Err(\"Serval is not there to run on\")".into()))
        );
        assert_eq!(
            block_on(run_task(&plugin, "entry", |_| {})),
            Ok(Some("up".into()))
        );
        assert_eq!(
            block_on(run_task(&plugin, "spans", |_| {})),
            Ok(Some("2500 2 true 2.5s".into()))
        );
        // A stream, or an instant, of no error to give: the task stopped
        for name in ["label", "now"] {
            let failed = block_on(run_task(&plugin, name, |_| {})).unwrap_err();
            assert!(
                failed.contains("Serval is not there to run on"),
                "{name}: {failed}"
            );
        }
    }
}
