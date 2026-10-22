//! `serval::serial`: the port, what is sent to it and what it receives.

use rune::{
    Any, ContextError, Module, Value,
    runtime::{Bytes, Function, InstAddress, Memory, Mut, Output, VmResult},
};

use super::{host, or_stop, stream_of, to_value};
use crate::{plugin::ErrorText, serial::Restart};

pub fn module() -> Result<Module, ContextError> {
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
    module.raw_function("wait_for", wait_for).build()?;
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

/// What `send` was given that is yet to be written to the port, oldest first
/// (not of a file being sent).
fn get_send_buffer() -> Result<Bytes, String> {
    let unsent = host()?.serial.unsent();
    Bytes::from_slice(unsent.as_slice()).err_text()
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

/// The bytes the port receives from now on as a Rune `Stream`, each item what
/// came at once (as `Bytes`). It does not end.
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
