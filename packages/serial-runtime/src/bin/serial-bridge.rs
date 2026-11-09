//! The OS's serial ports, for a page that cannot open them. Takes a JSON
//! request on each line of its input, and answers each with a line of the same
//! `id` (with `error`, if it failed). What a port reads comes in lines of its
//! `path`: with `data`, or with `ended` (and `error`, if lost). Bytes are in
//! hex. It ends with its input.

use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use serde_json::{Value, json};
use serial_runtime::{Baudrate, Choice, ExtraSettings, FlowControl, Parity, native};
use serialport::{SerialPort, SerialPortInfo, SerialPortType};

type Output = Arc<Mutex<io::Stdout>>;

fn main() {
    let output: Output = Arc::new(Mutex::new(io::stdout()));
    // Each open port's requests, on a thread of its own: in order, and so a
    // write that waits on one holds up no other
    let mut ports: HashMap<String, mpsc::Sender<Value>> = HashMap::new();
    // Lists still being made: answered before it ends
    let mut listing = Vec::new();

    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let path = text(&request, "path").to_string();
        match text(&request, "op") {
            "list" => {
                let output = output.clone();
                listing.retain(|list: &thread::JoinHandle<()>| !list.is_finished());
                listing.push(thread::spawn(move || {
                    let answer = native::listed_ports()
                        .map(|ports| json!({ "ports": ports.iter().map(port).collect::<Vec<_>>() }))
                        .map_err(|error| error.to_string());
                    respond(&output, &request, answer);
                }));
            }
            "open" => match settings(&request)
                .and_then(|(baudrate, extra)| native::open(&path, baudrate, extra))
            {
                Ok(port) => {
                    let (sender, requests) = mpsc::channel();
                    // One lost and opened again: the old one's thread ends
                    if let Some(old) = ports.insert(path.clone(), sender.clone()) {
                        let _ = old.send(json!({ "op": "drop" }));
                    }
                    respond(&output, &request, Ok(json!({})));
                    let output = output.clone();
                    thread::spawn(move || serve_port(port, &path, &requests, &sender, &output));
                }
                Err(error) => respond(&output, &request, Err(error)),
            },
            // The bridge's own, of no request
            op @ ("lost" | "drop") => {
                respond(&output, &request, Err(format!("Unknown request: {op}")));
            }
            op => {
                let sender = if op == "close" {
                    ports.remove(&path)
                } else {
                    ports.get(&path).cloned()
                };
                match sender {
                    Some(sender) => {
                        let _ = sender.send(request);
                    }
                    None => respond(&output, &request, Err("The port is not open".to_string())),
                }
            }
        }
    }
    for list in listing {
        let _ = list.join();
    }
}

/// Answers the requests of an open port, until it is closed. Lost (as its
/// device is unplugged), it lets the port go at once, so it may open again
/// (on some OSes, only once the old one is let go): till closed, the requests
/// are answered that it is not open.
fn serve_port(
    port: Box<dyn SerialPort>,
    path: &str,
    requests: &mpsc::Receiver<Value>,
    own: &mpsc::Sender<Value>,
    output: &Output,
) {
    let closing = Arc::new(AtomicBool::new(false));
    let mut reading: Option<thread::JoinHandle<()>> = None;
    let mut port = Some(port);
    for request in requests {
        let op = text(&request, "op");
        if op == "drop" {
            closing.store(true, Ordering::Relaxed);
            if let Some(reading) = reading.take() {
                let _ = reading.join();
            }
            return;
        }
        if op == "lost" {
            // From the read thread, which has ended
            if let Some(reading) = reading.take() {
                let _ = reading.join();
            }
            port = None;
            continue;
        }
        let Some(open) = port.as_mut() else {
            if op == "close" {
                respond(output, &request, Ok(json!({})));
                return;
            }
            respond(output, &request, Err("The port is not open".to_string()));
            continue;
        };
        let answer = match op {
            "read" => open
                .try_clone()
                .map(|reader| {
                    let (closing, output, path, own) = (
                        closing.clone(),
                        output.clone(),
                        path.to_string(),
                        own.clone(),
                    );
                    reading = Some(thread::spawn(move || {
                        if read(reader, &path, &closing, &output) {
                            let _ = own.send(json!({ "op": "lost" }));
                        }
                    }));
                })
                .map_err(|error| error.to_string()),
            "write" => from_hex(text(&request, "data"))
                .and_then(|bytes| {
                    open.write_all(&bytes)?;
                    open.flush()
                })
                .map_err(|error| error.to_string()),
            "signals" => set_signals(open.as_mut(), &request).map_err(|error| error.to_string()),
            "break" => if request["on"].as_bool().unwrap_or_default() {
                open.set_break()
            } else {
                open.clear_break()
            }
            .map_err(|error| error.to_string()),
            "close" => {
                closing.store(true, Ordering::Relaxed);
                // Closed once the read thread lets its handle go too
                if let Some(reading) = reading.take() {
                    let _ = reading.join();
                }
                drop(port);
                respond(output, &request, Ok(json!({})));
                return;
            }
            op => Err(format!("Unknown request: {op}")),
        };
        respond(output, &request, answer.map(|()| json!({})));
    }
}

/// Writes what `reader` reads, till closed or lost: whether it was lost.
fn read(reader: Box<dyn SerialPort>, path: &str, closing: &AtomicBool, output: &Output) -> bool {
    let mut error = None;
    native::read_loop(reader, closing, |read| {
        match read {
            Ok(bytes) => write_line(output, &json!({ "path": path, "data": to_hex(&bytes) })),
            Err(why) => error = Some(why),
        }
        true
    });
    let lost = error.is_some();
    write_line(
        output,
        &json!({ "path": path, "ended": true, "error": error }),
    );
    lost
}

fn set_signals(port: &mut dyn SerialPort, request: &Value) -> serialport::Result<()> {
    if let Some(dtr) = request["dtr"].as_bool() {
        port.write_data_terminal_ready(dtr)?;
    }
    if let Some(rts) = request["rts"].as_bool() {
        port.write_request_to_send(rts)?;
    }
    Ok(())
}

/// The settings an `open` asks for, those it leaves out as by default (8N1
/// with no flow control); else what is wrong.
fn settings(request: &Value) -> Result<(Baudrate, ExtraSettings), String> {
    let bps = request["baudRate"].as_u64().unwrap_or_default();
    let baudrate = u32::try_from(bps)
        .ok()
        .and_then(Baudrate::new)
        .ok_or_else(|| format!("Not a baud rate: {}", request["baudRate"]))?;
    let default = ExtraSettings::default();
    let number = |key: &str, default: u8| {
        request[key].as_u64().map_or(Ok(default), |value| {
            u8::try_from(value).map_err(|_| format!("Not {key}: {value}"))
        })
    };
    let choice = |key: &str| request[key].as_str().map(str::to_string);
    Ok((
        baudrate,
        ExtraSettings {
            data_bits: number("dataBits", default.data_bits)?,
            stop_bits: number("stopBits", default.stop_bits)?,
            parity: choice("parity").map_or(Ok(default.parity), |name| Parity::parse(&name))?,
            flow_control: choice("flowControl")
                .map_or(Ok(default.flow_control), |name| FlowControl::parse(&name))?,
        },
    ))
}

fn port(port: &SerialPortInfo) -> Value {
    let usb = match &port.port_type {
        SerialPortType::UsbPort(usb) => Some(usb),
        _ => None,
    };
    json!({
        "path": port.port_name,
        "vendorId": usb.map(|usb| usb.vid),
        "productId": usb.map(|usb| usb.pid),
        "manufacturer": usb.and_then(|usb| usb.manufacturer.clone()),
        "product": usb.and_then(|usb| usb.product.clone()),
    })
}

fn text<'a>(request: &'a Value, key: &str) -> &'a str {
    request[key].as_str().unwrap_or_default()
}

/// `answer`'s fields with the request's `id`, or why it failed.
fn respond(output: &Output, request: &Value, answer: Result<Value, String>) {
    let mut line = answer.unwrap_or_else(|error| json!({ "error": error }));
    line["id"] = request["id"].clone();
    write_line(output, &line);
}

fn write_line(output: &Output, line: &Value) {
    let mut output = output.lock().unwrap_or_else(|error| error.into_inner());
    // Or the host is gone, and its end closes the input
    let _ = writeln!(output, "{line}").and_then(|()| output.flush());
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(hex: &str) -> io::Result<Vec<u8>> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "Not hex");
    if !hex.len().is_multiple_of(2) {
        return Err(invalid());
    }
    (0..hex.len())
        .step_by(2)
        .map(|start| {
            hex.get(start..start + 2)
                .and_then(|byte| u8::from_str_radix(byte, 16).ok())
                .ok_or_else(invalid)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use serial_runtime::{ExtraSettings, FlowControl, Parity};

    use super::{from_hex, settings, to_hex};

    #[test]
    fn an_open_takes_its_settings_else_says_what_is_wrong() {
        let (baudrate, extra) = settings(&json!({ "baudRate": 115200 })).unwrap();
        assert_eq!(baudrate.bps(), 115200);
        assert_eq!(extra, ExtraSettings::default());

        let (_, extra) = settings(&json!({
            "baudRate": 9600, "dataBits": 7, "stopBits": 2, "parity": "even", "flowControl": "hardware",
        }))
        .unwrap();
        assert_eq!(extra.frame(), "7E2");
        assert_eq!(extra.parity, Parity::Even);
        assert_eq!(extra.flow_control, FlowControl::Hardware);

        assert!(settings(&json!({})).is_err());
        assert!(settings(&json!({ "baudRate": 0 })).is_err());
        let wrong = settings(&json!({ "baudRate": 9600, "parity": "mark" })).unwrap_err();
        assert!(wrong.starts_with("Not a parity: mark"), "{wrong}");
    }

    #[test]
    fn hex_goes_both_ways() {
        assert_eq!(to_hex(&[0, 0x1f, 0xff]), "001fff");
        assert_eq!(from_hex("001fff").unwrap(), [0, 0x1f, 0xff]);
        assert!(from_hex("1").is_err());
        assert!(from_hex("zz").is_err());
    }
}
