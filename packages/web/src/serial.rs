use std::rc::Rc;

use dioxus::prelude::*;
use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

use ui::serial::{
    self, LocalFuture, PortInfo, SerialBackend, SerialPort, SerialResult, log::LogSink,
    use_serial_provider,
};

use crate::log_file;

/// Provides `SerialContext` with Web Serial. Requires `DataContext` and `TimeContext`.
#[component]
pub fn SerialProvider(children: Element) -> Element {
    use_serial_provider(|| Rc::new(WebSerial));
    children
}

pub fn is_supported() -> bool {
    web_serial::is_supported()
}

struct WebSerial;

impl SerialBackend for WebSerial {
    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>> {
        Box::pin(async {
            match web_serial::request_port().await {
                Ok(port) => Ok(Some(Rc::new(WebSerialPort(port)) as Rc<dyn SerialPort>)),
                Err(error) if error_name(&error).as_deref() == Some("NotFoundError") => Ok(None),
                Err(error) => Err(message(error)),
            }
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

    fn open_log_file(
        &self,
        extension: &str,
        append: bool,
    ) -> LocalFuture<SerialResult<Option<Rc<dyn LogSink>>>> {
        let extension = extension.to_string();
        Box::pin(async move { log_file::open(&extension, append).await })
    }
}

struct WebSerialPort(JsValue);

impl SerialPort for WebSerialPort {
    fn info(&self) -> PortInfo {
        port_info(&self.0)
    }

    fn open(&self, baudrate: u32) -> LocalFuture<SerialResult<()>> {
        let port = self.0.clone();
        Box::pin(async move {
            web_serial::open_port(
                &port,
                baudrate,
                serial::DATA_BITS,
                serial::STOP_BITS,
                serial::PARITY,
                serial::FLOW_CONTROL,
            )
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

mod web_serial {
    use js_sys::Uint8Array;
    use wasm_bindgen::{JsValue, closure::Closure, prelude::wasm_bindgen};

    #[wasm_bindgen(module = "/src/serial.js")]
    extern "C" {
        #[wasm_bindgen(js_name = isSupported)]
        pub fn is_supported() -> bool;

        #[wasm_bindgen(catch, js_name = getPorts)]
        pub async fn get_ports() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, js_name = requestPort)]
        pub async fn request_port() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, js_name = openPort)]
        pub async fn open_port(
            port: &JsValue,
            baud_rate: u32,
            data_bits: u8,
            stop_bits: u8,
            parity: &str,
            flow_control: &str,
        ) -> Result<(), JsValue>;

        #[wasm_bindgen(catch, js_name = closePort)]
        pub async fn close_port(port: &JsValue) -> Result<(), JsValue>;

        #[wasm_bindgen(catch, js_name = readLoop)]
        pub async fn read_loop(
            port: &JsValue,
            on_chunk: &Closure<dyn FnMut(Uint8Array)>,
        ) -> Result<(), JsValue>;

        /// A copy, as wasm memory could move while the write is pending.
        #[wasm_bindgen(catch, js_name = writePort)]
        pub async fn write_port(port: &JsValue, bytes: Uint8Array) -> Result<(), JsValue>;

        #[wasm_bindgen(js_name = usbVendorId)]
        pub fn usb_vendor_id(port: &JsValue) -> Option<u16>;

        #[wasm_bindgen(js_name = usbProductId)]
        pub fn usb_product_id(port: &JsValue) -> Option<u16>;
    }
}

fn port_info(port: &JsValue) -> PortInfo {
    let vendor_id = web_serial::usb_vendor_id(port);
    let device = vendor_id
        .zip(web_serial::usb_product_id(port))
        .and_then(|(vid, pid)| usb_ids::Device::from_vid_pid(vid, pid));
    let vendor = vendor_id
        .and_then(<usb_ids::Vendor as usb_ids::FromId<u16>>::from_id)
        .map(|vendor| vendor.name().to_string());
    let product = device.map(|device| device.name().to_string());
    PortInfo {
        name: product
            .clone()
            .unwrap_or_else(|| "Serial Device".to_string()),
        vendor,
        product,
    }
}

fn error_name(error: &JsValue) -> Option<String> {
    js_sys::Reflect::get(error, &JsValue::from_str("name"))
        .ok()?
        .as_string()
}

pub(crate) fn message(error: JsValue) -> String {
    match error.dyn_ref::<js_sys::Error>() {
        Some(error) => error.message().into(),
        None => format!("{error:?}"),
    }
}
