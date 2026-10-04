use std::rc::Rc;

use dioxus::prelude::*;
use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

use ui::serial::{
    LocalFuture, PortInfo, SerialBackend, SerialPort, SerialResult, use_serial_provider,
};

/// Provides `SerialContext` with the browser's Web Serial.
///
/// Requires `DataContext` and `TimeContext` to be provided by an ancestor.
#[component]
pub fn SerialProvider(children: Element) -> Element {
    use_serial_provider(|| Rc::new(WebSerial));
    children
}

struct WebSerial;

impl SerialBackend for WebSerial {
    fn request_port(&self) -> LocalFuture<SerialResult<Rc<dyn SerialPort>>> {
        Box::pin(async {
            let port = web_serial::request_port().await.map_err(message)?;
            Ok(Rc::new(WebSerialPort(port)) as Rc<dyn SerialPort>)
        })
    }

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>> {
        Box::pin(async {
            let ports = web_serial::get_ports().await.map_err(message)?;
            Ok(Array::from(&ports)
                .into_iter()
                .map(|port| Rc::new(WebSerialPort(port)) as Rc<dyn SerialPort>)
                .collect())
        })
    }
}

/// A `SerialPort` object of Web Serial.
struct WebSerialPort(JsValue);

impl SerialPort for WebSerialPort {
    fn info(&self) -> PortInfo {
        port_info(&self.0)
    }

    fn open(&self, baudrate: u32) -> LocalFuture<SerialResult<()>> {
        let port = self.0.clone();
        Box::pin(async move {
            web_serial::open_port(&port, baudrate)
                .await
                .map_err(message)
        })
    }

    fn read(&self, mut on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>> {
        let port = self.0.clone();
        Box::pin(async move {
            let on_chunk =
                Closure::wrap(Box::new(move |chunk: Uint8Array| on_chunk(chunk.to_vec()))
                    as Box<dyn FnMut(Uint8Array)>);
            // `on_chunk` is dropped once the loop ends, as JS calls it no more
            web_serial::read_loop(&port, &on_chunk)
                .await
                .map_err(message)
        })
    }

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>> {
        let port = self.0.clone();
        Box::pin(async move {
            web_serial::write_port(&port, Uint8Array::from(bytes.as_slice()))
                .await
                .map_err(message)
        })
    }

    fn close(&self) -> LocalFuture<SerialResult<()>> {
        let port = self.0.clone();
        Box::pin(async move { web_serial::close_port(&port).await.map_err(message) })
    }
}

/// Web Serial calls, see `serial.js`.
mod web_serial {
    use js_sys::Uint8Array;
    use wasm_bindgen::{JsValue, closure::Closure, prelude::wasm_bindgen};

    #[wasm_bindgen(module = "/src/serial.js")]
    extern "C" {
        /// Resolves to an array of the ports this page was granted before.
        #[wasm_bindgen(catch, js_name = getPorts)]
        pub async fn get_ports() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, js_name = requestPort)]
        pub async fn request_port() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, js_name = openPort)]
        pub async fn open_port(port: &JsValue, baud_rate: u32) -> Result<(), JsValue>;

        /// Stops `readLoop` on the port first, if it is running.
        #[wasm_bindgen(catch, js_name = closePort)]
        pub async fn close_port(port: &JsValue) -> Result<(), JsValue>;

        /// Calls `on_chunk` with each chunk read until the stream ends.
        #[wasm_bindgen(catch, js_name = readLoop)]
        pub async fn read_loop(
            port: &JsValue,
            on_chunk: &Closure<dyn FnMut(Uint8Array)>,
        ) -> Result<(), JsValue>;

        /// Takes a `Uint8Array` copy rather than a view into wasm memory, which
        /// could move while the write is pending.
        #[wasm_bindgen(catch, js_name = writePort)]
        pub async fn write_port(port: &JsValue, bytes: Uint8Array) -> Result<(), JsValue>;

        #[wasm_bindgen(js_name = usbVendorId)]
        pub fn usb_vendor_id(port: &JsValue) -> Option<u16>;

        #[wasm_bindgen(js_name = usbProductId)]
        pub fn usb_product_id(port: &JsValue) -> Option<u16>;
    }
}

/// The USB device's name, from its vendor and product ids.
fn port_info(port: &JsValue) -> PortInfo {
    let name = web_serial::usb_vendor_id(port)
        .zip(web_serial::usb_product_id(port))
        .and_then(|(vid, pid)| usb_ids::Device::from_vid_pid(vid, pid))
        .map_or_else(
            || "Serial Device".to_string(),
            |device| device.name().to_string(),
        );
    PortInfo { name }
}

/// A JS error as a message.
fn message(error: JsValue) -> String {
    match error.dyn_ref::<js_sys::Error>() {
        Some(error) => error.message().into(),
        None => format!("{error:?}"),
    }
}
