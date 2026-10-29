//! `serval::serial`: the port, what is sent to it and what it receives.

use rune::{
    Any, ContextError, Module, Value,
    runtime::{Bytes, Function, InstAddress, Memory, Mut, Object, Output, VmResult},
};

use super::{host, or_stop, stream_of, to_value};
use crate::{
    helper::{Choice, Delimiter},
    plugin::ErrorText,
    serial::{Baudrate, ExtraSettings, FlowControl, Parity, Restart},
};

/// The serial port: what is sent to it, what it receives, and its signals.
///
/// Most functions fail with an `Err` when no port is open: take it with `?` to
/// stop the task.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Asks the device for its version once the port is open.
/// pub async fn version() {
///     serial::set_settings(#{ baudrate: 115200 })?;
///     serial::open().await?;
///     serial::send("version\r\n")?;
///     let reply = serial::wait_for("\n").await?;
///     println!("{}", String::from_utf8(reply)?);
/// }
/// ```
#[rune::module(::serval::serial)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.function_meta(send__meta)?;
    module.function_meta(get_send_buffer__meta)?;
    module.function_meta(flush__meta)?;
    module.ty::<ReceivedReader>()?;
    module
        .function("next", next_received)
        .build_associated::<ReceivedReader>()?;
    module.function_meta(received__meta)?;
    module
        .raw_function("wait_for", wait_for)
        .build()?
        .docs(WAIT_FOR_DOCS.lines())?;
    module.function_meta(open__meta)?;
    module.function_meta(close__meta)?;
    module.ty::<PortSettings>()?;
    module.function_meta(get_settings__meta)?;
    module.function_meta(set_settings__meta)?;
    module.function_meta(get_line_ending__meta)?;
    module.function_meta(set_line_ending__meta)?;
    module.function_meta(get_dtr__meta)?;
    module.function_meta(get_rts__meta)?;
    module.function_meta(set_dtr__meta)?;
    module.function_meta(set_rts__meta)?;
    module.function_meta(toggle_dtr__meta)?;
    module.function_meta(toggle_rts__meta)?;
    module.function_meta(restart__meta)?;
    Ok(module)
}

/// Sends `data` to the open port.
///
/// `data` is a string (sent as UTF-8), `Bytes`, or a vector of byte values.
///
/// It goes to the send buffer, and returns at once: use `flush` to wait until
/// the port takes it. Fails while a file is being sent.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn hello() {
///     serial::send("hello\r\n")?;
///     serial::send(b"\x02ok\x03")?;
///     serial::send([0x02, 0x41, 0x03])?;
/// }
/// ```
#[rune::function(keep)]
fn send(data: Value) -> Result<(), String> {
    let serial = host()?.serial;
    if !serial.is_open() {
        return Err("No port is open".to_string());
    }
    // It would go in between the file's bytes: the port takes none of it
    if serial.is_sending_file() {
        return Err("A file is being sent".to_string());
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

/// What `send` was given that the port has yet to take.
///
/// As `Bytes`, oldest first.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn backlog() {
///     let unsent = serial::get_send_buffer()?;
///     println!("{} bytes to go", unsent.len());
/// }
/// ```
#[rune::function(keep)]
fn get_send_buffer() -> Result<Bytes, String> {
    let unsent = host()?.serial.unsent();
    Bytes::from_slice(unsent.as_slice()).err_text()
}

/// How often `flush` looks whether all is written.
const FLUSH_CHECK_MS: u32 = 10;

/// Waits until the port has taken all that was sent.
///
/// A file being sent included. It also returns if the port failed to take it.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Sends a command, then closes the port once it is sent.
/// pub async fn send_and_close() {
///     serial::send("sleep\r\n")?;
///     serial::flush().await?;
///     serial::close().await?;
/// }
/// ```
#[rune::function(keep)]
async fn flush() -> Result<(), String> {
    let host = host()?;
    while host.serial.has_pending() {
        host.time.after_ms(FLUSH_CHECK_MS).await;
    }
    Ok(())
}

/// How often what is received is looked for, as there is none new.
const RECEIVED_CHECK_MS: u32 = 10;

/// The reader behind the stream of `received`.
// It reads the console's history, apart from the data.
#[derive(Any, Debug)]
#[rune(item = ::serval::serial)]
pub struct ReceivedReader {
    /// Where it read to, counted from the first byte ever received
    read_to: u64,
}

impl ReceivedReader {
    fn new() -> Result<Self, String> {
        Ok(Self {
            read_to: host()?.serial.received_end(),
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

/// What the port receives from now on, as a stream of chunks.
///
/// Each item is a chunk, as `Bytes`.
///
/// `next().await` waits for the next chunk; the stream does not end. It reads
/// the bytes as they arrive, apart from the maps.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Prints how many bytes each chunk has.
/// pub async fn chunks() {
///     let chunks = serial::received()?;
///     while let Some(chunk) = chunks.next().await {
///         println!("{} bytes", chunk.len());
///     }
/// }
/// ```
#[rune::function(keep)]
fn received() -> Result<Value, String> {
    stream_of(ReceivedReader::new()?, false)
}

async fn next_received(
    mut reader: Mut<ReceivedReader>,
    _with_time: bool,
) -> VmResult<Option<Value>> {
    or_stop(
        async {
            let bytes = reader.next_bytes().await?;
            to_value(Bytes::from_slice(bytes.as_slice()).err_text()?).map(Some)
        }
        .await,
    )
}

/// How much `wait_for` keeps of what comes before `pattern`, if it is not told.
const WAIT_FOR_LIMIT: i64 = 1000;

/// The docs of `wait_for`, a raw function, which takes no doc comments.
pub(super) const WAIT_FOR_DOCS: &str = "\
Waits until the port receives `pattern`, from now on.

`wait_for(pattern)`, `wait_for(pattern, limit)`: `pattern` is a string,
`Bytes`, or a vector of byte values.

Gives what it received until then, `pattern` included, as `Bytes`: its last
`limit` bytes at most (1000 if not given).

# Examples

```rune
use serval::serial;

/// Sends an AT command, and prints the reply.
pub async fn at() {
    serial::send(\"AT\\r\\n\")?;
    let reply = serial::wait_for(\"OK\\r\\n\").await?;
    println!(\"{}\", String::from_utf8(reply)?);
}
```

```rune
use serval::serial;

/// Waits for a line, keeping 100 bytes at most.
pub async fn line() {
    serial::wait_for(b\"\\n\", 100).await
}
```
";

/// `wait_for(pattern, limit = 1000)`: Rune has no default arguments, so it
/// takes what it is given, and waits as `wait_until_received` does.
fn wait_for(stack: &mut dyn Memory, addr: InstAddress, args: usize, out: Output) -> VmResult<()> {
    let arguments = rune::vm_try!(stack.slice_at(addr, args));
    let (pattern, limit) = match arguments {
        [pattern] => (pattern.clone(), Value::from(WAIT_FOR_LIMIT)),
        [pattern, limit] => (pattern.clone(), limit.clone()),
        _ => {
            return VmResult::panic(format!(
                "`wait_for` takes a pattern and a limit, not {args} arguments"
            ));
        }
    };
    let waiting = rune::vm_try!(Function::new(wait_until_received).call::<Value>((pattern, limit)));
    rune::vm_try!(out.store(stack, waiting));
    VmResult::Ok(())
}

/// Waits until the port receives `pattern` (a string, bytes or a list of
/// bytes) from now on: what it received until then, `pattern` with it, of its
/// last `limit` bytes at most (as much is kept as it waits).
async fn wait_until_received(pattern: Value, limit: Value) -> Result<Bytes, String> {
    let pattern = to_bytes(pattern)?;
    let limit = limit
        .as_integer::<i64>()
        .ok()
        .and_then(|limit| usize::try_from(limit).ok())
        .filter(|&limit| limit >= pattern.len().max(1))
        .ok_or_else(|| format!("Not a limit of at least {} bytes", pattern.len().max(1)))?;
    if pattern.is_empty() {
        return Ok(Bytes::new());
    }
    let mut reader = ReceivedReader::new()?;
    let mut received = Vec::new();
    loop {
        // From where it could start in what came before
        let from = received.len().saturating_sub(pattern.len() - 1);
        received.extend(reader.next_bytes().await?);
        if let Some(at) = received[from..]
            .windows(pattern.len())
            .position(|window| window == pattern.as_slice())
        {
            let end = from + at + pattern.len();
            return Bytes::from_slice(&received[end.saturating_sub(limit)..end]).err_text();
        }
        // What is to be given, and where `pattern` could start: no more
        let kept = limit.max(pattern.len() - 1);
        received.drain(..received.len().saturating_sub(kept));
    }
}

/// Opens the selected port with its settings.
///
/// The port is open when it returns. See `set_settings`.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn connect() {
///     serial::set_settings(#{ baudrate: 9600 })?;
///     serial::open().await?;
/// }
/// ```
#[rune::function(keep)]
async fn open() -> Result<(), String> {
    host()?.serial.open_port().await
}

/// Closes the open port. The port is closed when it returns.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn disconnect() {
///     serial::close().await?;
/// }
/// ```
#[rune::function(keep)]
async fn close() -> Result<(), String> {
    host()?.serial.close_port().await
}

/// A port's settings: what it opens with.
///
/// * `baudrate`: a positive integer, or `None` (the port cannot open so).
/// * `data_bits`: 7 or 8.
/// * `parity`: `"none"`, `"even"`, or `"odd"`.
/// * `stop_bits`: 1 or 2.
/// * `flow_control`: `"none"`, or `"hardware"` (RTS and CTS).
///
/// `get_settings` gives them, and `set_settings` takes them (or an object of
/// some of them).
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Prints the data bits, parity, and stop bits, e.g. "8 none 1".
/// pub async fn frame() {
///     let settings = serial::get_settings()?.unwrap();
///     println!("{} {} {}", settings.data_bits, settings.parity, settings.stop_bits);
/// }
/// ```
#[derive(Any, Debug, Clone)]
#[rune(item = ::serval::serial)]
pub struct PortSettings {
    #[rune(get, set)]
    pub baudrate: Option<i64>,
    #[rune(get, set)]
    pub data_bits: i64,
    #[rune(get, set)]
    pub parity: String,
    #[rune(get, set)]
    pub stop_bits: i64,
    #[rune(get, set)]
    pub flow_control: String,
}

impl PortSettings {
    /// Of `object`'s fields, those left out as a port has them at first: as
    /// Rust's `PortSettings { baudrate, ..Default::default() }`.
    fn of_object(object: &Object) -> Result<Self, String> {
        let mut settings = Self::default();
        for (field, value) in object.iter() {
            let integer = || {
                rune::from_value::<i64>(value.clone())
                    .map_err(|_| format!("Not an integer: {field}"))
            };
            let text = || {
                rune::from_value::<String>(value.clone())
                    .map_err(|_| format!("Not a string: {field}"))
            };
            match field.as_str() {
                // A number, as given in Rust's way too (`Some(9600)`, or `None`)
                "baudrate" => {
                    settings.baudrate = match rune::from_value::<Option<i64>>(value.clone()) {
                        Ok(baudrate) => baudrate,
                        Err(_) => Some(integer()?),
                    }
                }
                "data_bits" => settings.data_bits = integer()?,
                "parity" => settings.parity = text()?,
                "stop_bits" => settings.stop_bits = integer()?,
                "flow_control" => settings.flow_control = text()?,
                _ => return Err(format!("Not a port setting: {field}")),
            }
        }
        Ok(settings)
    }
}

/// Those a port has at first: no baud rate, 8N1, and no flow control.
impl Default for PortSettings {
    fn default() -> Self {
        crate::serial::PortSettings::default().into()
    }
}

impl From<crate::serial::PortSettings> for PortSettings {
    fn from(settings: crate::serial::PortSettings) -> Self {
        let extra = settings.extra;
        Self {
            baudrate: settings.baudrate.map(|baudrate| i64::from(baudrate.bps())),
            data_bits: i64::from(extra.data_bits),
            parity: extra.parity.name().to_string(),
            stop_bits: i64::from(extra.stop_bits),
            flow_control: extra.flow_control.name().to_string(),
        }
    }
}

impl TryFrom<&PortSettings> for crate::serial::PortSettings {
    type Error = String;

    /// Fails on the first value a port cannot take, saying which.
    fn try_from(settings: &PortSettings) -> Result<Self, String> {
        let baudrate = settings
            .baudrate
            .map(|baudrate| {
                u32::try_from(baudrate)
                    .ok()
                    .and_then(Baudrate::new)
                    .ok_or_else(|| format!("Not a baud rate: {baudrate}"))
            })
            .transpose()?;
        let bits = |bits: i64, choices: [u8; 2], name: &str| {
            u8::try_from(bits)
                .ok()
                .filter(|bits| choices.contains(bits))
                .ok_or_else(|| format!("Not {name}: {bits} ({} or {})", choices[0], choices[1]))
        };
        let extra = ExtraSettings {
            data_bits: bits(settings.data_bits, ExtraSettings::DATA_BITS, "data bits")?,
            parity: Parity::parse(&settings.parity)?,
            stop_bits: bits(settings.stop_bits, ExtraSettings::STOP_BITS, "stop bits")?,
            flow_control: FlowControl::parse(&settings.flow_control)?,
        };
        Ok(Self { baudrate, extra })
    }
}

/// The settings of the selected port: `None` if no port is selected.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// e.g. "9600 bps, 8 none 1"
/// pub async fn settings() {
///     let settings = serial::get_settings()?.unwrap();
///     format!(
///         "{} bps, {} {} {}",
///         settings.baudrate.unwrap_or(0),
///         settings.data_bits,
///         settings.parity,
///         settings.stop_bits,
///     )
/// }
/// ```
#[rune::function(keep)]
fn get_settings() -> Result<Option<PortSettings>, String> {
    Ok(host()?.serial.settings().map(PortSettings::from))
}

/// Sets the settings of the selected port.
///
/// `settings` is a `PortSettings` (as `get_settings` gives), or an object of
/// some of its fields: those left out are as a port has at first (no baud rate,
/// 8N1, and no flow control).
///
/// They take effect when the port is opened next. Fails if no port is
/// selected, while the port is open (close it first), or on a value a port
/// cannot take (or a field it has not): then nothing is set.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Opens the port again at 115200 bps, all else as at first.
/// pub async fn reopen() {
///     serial::close().await?;
///     serial::set_settings(#{ baudrate: 115200 })?;
///     serial::open().await?;
/// }
///
/// /// As 7E1, at 9600 bps.
/// pub async fn seven_e_one() {
///     serial::set_settings(#{ baudrate: 9600, data_bits: 7, parity: "even" })?;
/// }
///
/// /// Hardware flow control from now on, all else as it is.
/// pub async fn hardware_flow_control() {
///     let settings = serial::get_settings()?.unwrap();
///     settings.flow_control = "hardware";
///     serial::set_settings(settings)?;
/// }
/// ```
#[rune::function(keep)]
fn set_settings(settings: Value) -> Result<(), String> {
    let settings = if let Ok(settings) = rune::from_value::<PortSettings>(settings.clone()) {
        settings
    } else if let Ok(object) = rune::from_value::<Object>(settings) {
        PortSettings::of_object(&object)?
    } else {
        return Err("Not port settings: a PortSettings or an object".to_string());
    };
    let settings = crate::serial::PortSettings::try_from(&settings)?;
    host()?.serial.set_settings(settings)
}

/// What Enter sends in the console: `"\r"`, `"\n"`, or `"\r\n"`.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Sends a command as if it were typed, Enter and all.
/// pub async fn reset() {
///     serial::send(`reset${serial::get_line_ending()?}`)?;
/// }
/// ```
#[rune::function(keep)]
fn get_line_ending() -> Result<String, String> {
    Ok(host()?.serial.line_ending().text().to_string())
}

/// Sets what Enter sends in the console: `"\r"`, `"\n"`, or `"\r\n"`.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn lf() {
///     serial::set_line_ending("\n")?;
/// }
/// ```
#[rune::function(keep)]
fn set_line_ending(line_ending: &str) -> Result<(), String> {
    let line_ending = Delimiter::from_text(line_ending).ok_or_else(|| {
        format!("Not a line ending: {line_ending:?} (\"\\r\", \"\\n\", or \"\\r\\n\")")
    })?;
    host()?.serial.set_line_ending(line_ending);
    Ok(())
}

/// Whether DTR is on, as Serval last set it (it is not read back from the port).
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn signals() {
///     println!("DTR: {}, RTS: {}", serial::get_dtr()?, serial::get_rts()?);
/// }
/// ```
#[rune::function(keep)]
fn get_dtr() -> Result<bool, String> {
    Ok(host()?.serial.signals().dtr)
}

/// Whether RTS is on, as Serval last set it (it is not read back from the port).
/// See `get_dtr`.
#[rune::function(keep)]
fn get_rts() -> Result<bool, String> {
    Ok(host()?.serial.signals().rts)
}

/// Turns DTR on or off on the open port. It is set when it returns.
///
/// # Examples
///
/// ```rune
/// use serval::{serial, time};
///
/// /// Pulses DTR off for 100 ms.
/// pub async fn pulse() {
///     serial::set_dtr(false).await?;
///     time::sleep(time::Duration::from_millis(100)).await?;
///     serial::set_dtr(true).await?;
/// }
/// ```
#[rune::function(keep)]
async fn set_dtr(on: bool) -> Result<(), String> {
    host()?.serial.write_signals(Some(on), None).await
}

/// Turns RTS on or off on the open port.
///
/// It is set when it returns. See `set_dtr`.
#[rune::function(keep)]
async fn set_rts(on: bool) -> Result<(), String> {
    host()?.serial.write_signals(None, Some(on)).await
}

/// Turns DTR off if it is on, and on if it is off.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// pub async fn flip() {
///     serial::toggle_dtr().await?;
///     serial::toggle_rts().await?;
/// }
/// ```
#[rune::function(keep)]
async fn toggle_dtr() -> Result<(), String> {
    set_dtr(!get_dtr()?).await
}

/// Turns RTS off if it is on, and on if it is off. See `toggle_dtr`.
#[rune::function(keep)]
async fn toggle_rts() -> Result<(), String> {
    set_rts(!get_rts()?).await
}

/// Restarts the board on the open port, through DTR and RTS.
///
/// As `kind` is:
///
/// * `"arduino"`: an Arduino, as avrdude does.
/// * `"espressif"` (or `"esp32"`, `"esp8266"`): an ESP32, ESP8266, or the
///   like, as esptool does.
///
/// It is restarted when it returns.
///
/// # Examples
///
/// ```rune
/// use serval::serial;
///
/// /// Restarts the board, and waits for it to say it is ready.
/// pub async fn reboot() {
///     serial::restart("espressif").await?;
///     serial::wait_for("ready").await?;
/// }
/// ```
#[rune::function(keep)]
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
    use super::PortSettings;

    #[test]
    fn port_settings_go_to_the_port_and_back() {
        let host = crate::serial::PortSettings::default();
        let settings = PortSettings::from(host);
        assert_eq!(settings.parity, "none");
        assert_eq!(crate::serial::PortSettings::try_from(&settings), Ok(host));
    }

    #[test]
    fn a_value_a_port_cannot_take_is_named() {
        let mut settings = PortSettings {
            baudrate: Some(0),
            ..PortSettings::default()
        };
        let refused = crate::serial::PortSettings::try_from(&settings);
        assert_eq!(refused, Err("Not a baud rate: 0".to_string()));
        settings.baudrate = Some(9600);
        settings.parity = "mark".to_string();
        let refused = crate::serial::PortSettings::try_from(&settings);
        assert_eq!(
            refused,
            Err("Not a parity: mark (none, even, or odd)".to_string())
        );
    }
}
