//! The serial ports of the OS, for a page in a frame of another (as the VS Code
//! extension's panel): opened by the `serial-bridge` program, which the parent
//! page talks to (through `bridge_glue.js`), each read as on the desktop.

use std::rc::Rc;

use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsValue, closure::Closure};

use super::web::message;
use crate::{
    Baudrate, Choice, ExtraSettings, LocalFuture, PortInfo, SerialBackend, SerialPort,
    SerialResult, UsbDevice,
};

/// Whether the page's host has a bridge (as it says with `servalHost`): there,
/// as in the VS Code extension, Web Serial may not be.
pub fn is_active() -> bool {
    js::is_active()
}

pub fn backend() -> Rc<dyn SerialBackend> {
    Rc::new(BridgeBackend)
}

struct BridgeBackend;

impl SerialBackend for BridgeBackend {
    // All are listed: none is requested
    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>> {
        Box::pin(async { Ok(None) })
    }

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>> {
        Box::pin(async {
            let ports = js::list_ports().await.map_err(message)?;
            Ok(Array::from(&ports)
                .iter()
                .map(|port| Rc::new(BridgePort::new(&port)) as Rc<dyn SerialPort>)
                .collect())
        })
    }
}

struct BridgePort {
    info: PortInfo,
    path: String,
}

impl BridgePort {
    fn new(port: &JsValue) -> Self {
        let path = js::port_path(port);
        let usb = js::usb_device(port).map(|device| {
            let device = Array::from(&device);
            UsbDevice {
                vendor_id: device.get(0).as_f64().unwrap_or_default() as u16,
                product_id: device.get(1).as_f64().unwrap_or_default() as u16,
                manufacturer: device.get(2).as_string(),
                product: device.get(3).as_string(),
            }
        });
        Self {
            info: PortInfo::of_device(path.clone(), usb),
            path,
        }
    }
}

impl SerialPort for BridgePort {
    fn info(&self) -> PortInfo {
        self.info.clone()
    }

    fn open(&self, baudrate: Baudrate, extra: ExtraSettings) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        Box::pin(async move {
            js::open_port(
                &path,
                baudrate.bps(),
                extra.data_bits,
                extra.stop_bits,
                extra.parity.name(),
                extra.flow_control.name(),
            )
            .await
            .map_err(message)
        })
    }

    fn read(&self, mut on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        Box::pin(async move {
            let on_chunk =
                Closure::wrap(Box::new(move |chunk: Uint8Array| on_chunk(chunk.to_vec()))
                    as Box<dyn FnMut(Uint8Array)>);
            js::read_loop(&path, &on_chunk).await.map_err(message)
        })
    }

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        Box::pin(async move { js::write_port(&path, &bytes).await.map_err(message) })
    }

    fn set_signals(&self, dtr: Option<bool>, rts: Option<bool>) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        Box::pin(async move { js::set_signals(&path, dtr, rts).await.map_err(message) })
    }

    fn set_break(&self, on: bool) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        Box::pin(async move { js::set_break(&path, on).await.map_err(message) })
    }

    fn close(&self) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        Box::pin(async move { js::close_port(&path).await.map_err(message) })
    }
}

mod js {
    use js_sys::Uint8Array;
    use wasm_bindgen::{JsValue, closure::Closure, prelude::wasm_bindgen};

    #[wasm_bindgen(module = "/src/backend/bridge_glue.js")]
    extern "C" {
        #[wasm_bindgen(js_name = isActive)]
        pub fn is_active() -> bool;

        #[wasm_bindgen(catch, js_name = listPorts)]
        pub async fn list_ports() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(js_name = portPath)]
        pub fn port_path(port: &JsValue) -> String;

        /// IDs, manufacturer and product, of a USB port.
        #[wasm_bindgen(js_name = usbDevice)]
        pub fn usb_device(port: &JsValue) -> Option<js_sys::Array>;

        #[wasm_bindgen(catch, js_name = openPort)]
        pub async fn open_port(
            path: &str,
            baud_rate: u32,
            data_bits: u8,
            stop_bits: u8,
            parity: &str,
            flow_control: &str,
        ) -> Result<(), JsValue>;

        #[wasm_bindgen(catch, js_name = readLoop)]
        pub async fn read_loop(
            path: &str,
            on_chunk: &Closure<dyn FnMut(Uint8Array)>,
        ) -> Result<(), JsValue>;

        #[wasm_bindgen(catch, js_name = writePort)]
        pub async fn write_port(path: &str, bytes: &[u8]) -> Result<(), JsValue>;

        /// `None` leaves a signal as it is.
        #[wasm_bindgen(catch, js_name = setSignals)]
        pub async fn set_signals(
            path: &str,
            dtr: Option<bool>,
            rts: Option<bool>,
        ) -> Result<(), JsValue>;

        #[wasm_bindgen(catch, js_name = setBreak)]
        pub async fn set_break(path: &str, on: bool) -> Result<(), JsValue>;

        #[wasm_bindgen(catch, js_name = closePort)]
        pub async fn close_port(path: &str) -> Result<(), JsValue>;
    }
}
