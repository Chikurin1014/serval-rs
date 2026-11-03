#![cfg(unix)]

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::Duration,
};

use serde_json::{Value, json};
use serialport::{SerialPort, TTYPort};

struct Bridge {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Bridge {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_serial-bridge"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }

    fn send(&mut self, request: &Value) {
        writeln!(self.input, "{request}").unwrap();
    }

    fn line(&mut self) -> Value {
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    /// Answered with the request's `id`, in no error.
    fn ask(&mut self, request: &Value) {
        self.send(request);
        let answer = self.line();
        assert_eq!(answer["id"], request["id"], "{answer}");
        assert!(answer["error"].is_null(), "{answer}");
    }
}

#[test]
fn a_list_asked_for_as_its_input_ends_is_answered() {
    let mut bridge = Bridge::start();
    bridge.send(&json!({ "id": 1, "op": "list" }));
    drop(bridge.input);
    let mut line = String::new();
    bridge.output.read_line(&mut line).unwrap();
    let answer: Value = serde_json::from_str(&line).expect("an answer");
    assert_eq!(answer["id"], 1, "{answer}");
    assert!(answer["ports"].is_array(), "{answer}");
    assert!(bridge.child.wait().unwrap().success());
}

#[test]
fn opens_reads_writes_and_opens_again_once_closed() {
    let (mut device, port) = TTYPort::pair().expect("a pseudo terminal");
    let path = port.name().expect("its path");
    // Let go, for the bridge to open
    drop(port);
    device.set_timeout(Duration::from_secs(2)).unwrap();
    let mut bridge = Bridge::start();
    let open = json!({ "id": 1, "op": "open", "path": path, "baudRate": 115200 });

    bridge.ask(&open);
    bridge.ask(&json!({ "id": 2, "op": "read", "path": path }));

    device.write_all(b"hello").unwrap();
    let read = bridge.line();
    assert_eq!(read["path"], path.as_str());
    assert_eq!(read["data"], "68656c6c6f");

    bridge.ask(&json!({ "id": 3, "op": "write", "path": path, "data": "6f6e" }));
    let mut sent = [0; 2];
    device.read_exact(&mut sent).unwrap();
    assert_eq!(&sent, b"on");

    bridge.send(&json!({ "id": 5, "op": "close", "path": path }));
    let ended = bridge.line();
    assert_eq!(ended["ended"], true, "{ended}");
    assert!(ended["error"].is_null(), "{ended}");
    assert_eq!(bridge.line()["id"], 5);

    bridge.send(&json!({ "id": 6, "op": "write", "path": path, "data": "" }));
    assert_eq!(bridge.line()["error"], "The port is not open");

    bridge.ask(&open);

    // Ends with its input
    drop(bridge.input);
    assert!(bridge.child.wait().unwrap().success());
}
