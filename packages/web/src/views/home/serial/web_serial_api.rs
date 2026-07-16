use std::fmt::Display;

use dioxus::prelude::*;
use js_sys::{Array, Function, Object, Promise};
use usb_ids::Device;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

#[derive(Clone)]
pub struct SerialContext {
    pub ports: Signal<Vec<PortEntry>>,
    pub selected_index: Signal<Option<usize>>,
    pub active_index: Signal<Option<usize>>,
    pub baudrate: Signal<u32>,
    pub received_text: Signal<String>,
    pub outgoing_text: Signal<String>,
    pub status_message: Signal<String>,
}

#[component]
pub fn SerialProvider(children: Element) -> Element {
    let ports = use_signal(|| Vec::<PortEntry>::new());
    let selected_port = use_signal(|| None::<usize>);
    let active_port = use_signal(|| None::<usize>);
    let baudrate = use_signal(|| 9600u32);
    let received_text = use_signal(String::new);
    let outgoing_text = use_signal(String::new);
    let status_message = use_signal(|| String::new());

    use_context_provider(|| SerialContext {
        ports: ports,
        selected_index: selected_port,
        active_index: active_port,
        baudrate: baudrate,
        received_text: received_text,
        outgoing_text: outgoing_text,
        status_message: status_message,
    });

    children
}

#[derive(Clone, PartialEq)]
struct PortTitle(String);

impl Display for PortTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PortTitle {
    fn from_port(port: &JsValue) -> Self {
        if let Ok(get_info) = js_sys::Reflect::get(port, &JsValue::from_str("getInfo")) {
            if !get_info.is_undefined() {
                if let Ok(info_fn) = get_info.dyn_into::<Function>() {
                    if let Ok(info) = info_fn.call0(port) {
                        if let Some(vid) =
                            js_sys::Reflect::get(&info, &JsValue::from_str("usbVendorId"))
                                .ok()
                                .and_then(|value| js_value_to_u16(&value))
                        {
                            if let Some(pid) =
                                js_sys::Reflect::get(&info, &JsValue::from_str("usbProductId"))
                                    .ok()
                                    .and_then(|value| js_value_to_u16(&value))
                            {
                                if let Some(device) = Device::from_vid_pid(vid, pid) {
                                    return PortTitle(device.name().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        PortTitle("Serial Device".to_string())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, PartialEq)]
struct PortDetails(Vec<String>);

impl AsRef<[String]> for PortDetails {
    fn as_ref(&self) -> &[String] {
        &self.0
    }
}

impl Display for PortDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join(", "))
    }
}

impl PortDetails {
    fn from_port(port: &JsValue) -> Self {
        let mut details = Vec::new();

        if let Ok(get_info) = js_sys::Reflect::get(port, &JsValue::from_str("getInfo")) {
            if !get_info.is_undefined() {
                if let Ok(info_fn) = get_info.dyn_into::<Function>() {
                    if let Ok(info) = info_fn.call0(port) {
                        let vendor = js_sys::Reflect::get(&info, &JsValue::from_str("usbVendorId"))
                            .ok()
                            .and_then(|value| js_value_to_u16(&value));
                        let product =
                            js_sys::Reflect::get(&info, &JsValue::from_str("usbProductId"))
                                .ok()
                                .and_then(|value| js_value_to_u16(&value));

                        if let Some(vendor) = vendor {
                            details.push(format!("Vendor ID: 0x{vendor:04X}"));
                        }
                        if let Some(product) = product {
                            details.push(format!("Product ID: 0x{product:04X}"));
                        }
                    }
                }
            }
        }

        PortDetails(details)
    }
}

#[derive(Clone, PartialEq)]
struct PortInfo {
    title: PortTitle,
    details: PortDetails,
}

impl PortInfo {
    fn from_port(port: &JsValue) -> Self {
        Self {
            title: PortTitle::from_port(port),
            details: PortDetails::from_port(port),
        }
    }
}

#[derive(Clone)]
pub struct PortEntry {
    port: JsValue,
    info: PortInfo,
}

impl PortEntry {
    fn from_port(port: &JsValue) -> Self {
        Self {
            port: port.clone(),
            info: PortInfo::from_port(port),
        }
    }

    pub fn port(&self) -> &JsValue {
        &self.port
    }

    pub fn title(&self) -> &str {
        self.info.title.as_str()
    }

    pub fn details(&self) -> &[String] {
        &self.info.details.as_ref()
    }
}

pub async fn get_ports_js() -> Result<Vec<PortEntry>, JsValue> {
    let serial = serial_api()?;

    let get_ports = js_sys::Reflect::get(&serial, &JsValue::from_str("getPorts"))?;
    let func: Function = get_ports.dyn_into()?;
    let promise = func.call0(&serial)?;
    let ports_js: JsValue = JsFuture::from(Promise::from(promise)).await?;
    let arr = Array::from(&ports_js);

    let mut names = Vec::new();
    for val in arr.iter() {
        names.push(PortEntry::from_port(&val));
    }

    Ok(names)
}

pub async fn request_port_js() -> Result<PortEntry, JsValue> {
    let serial = serial_api()?;

    let request = js_sys::Reflect::get(&serial, &JsValue::from_str("requestPort"))?;
    let func: Function = request.dyn_into()?;
    let promise = func.call0(&serial)?;
    let port: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(PortEntry::from_port(&port))
}

pub async fn open_port_js(port: &JsValue, baudrate: u32) -> Result<(), JsValue> {
    let options = Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("baudRate"),
        &JsValue::from_f64(baudrate as f64),
    )?;

    let open = js_sys::Reflect::get(port, &JsValue::from_str("open"))?;
    let func: Function = open.dyn_into()?;
    let promise = func.call1(port, &options)?;
    let _: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(())
}

pub async fn close_port_js(port: &JsValue) -> Result<(), JsValue> {
    let close = js_sys::Reflect::get(port, &JsValue::from_str("close"))?;
    let func: Function = close.dyn_into()?;
    let promise = func.call0(port)?;
    let _: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(())
}

pub async fn start_read_loop_js(
    port: &JsValue,
    on_chunk: &Closure<dyn FnMut(String)>,
) -> Result<(), JsValue> {
    let read_loop = Function::new_with_args(
        "port, onChunk",
        r#"
            return (async () => {
                const reader = port.readable.getReader();
                const decoder = new TextDecoder();
                try {
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) {
                            break;
                        }
                        if (value) {
                            onChunk(decoder.decode(value, { stream: true }));
                        }
                    }
                } finally {
                    reader.releaseLock();
                }
            })();
        "#,
    );
    let promise = read_loop.call2(&JsValue::NULL, port, on_chunk.as_ref())?;
    let _: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(())
}

pub async fn write_port_js(port: &JsValue, text: &str) -> Result<(), JsValue> {
    let write = Function::new_with_args(
        "port, text",
        r#"
            return (async () => {
                const writer = port.writable.getWriter();
                const encoder = new TextEncoder();
                try {
                    await writer.write(encoder.encode(text));
                } finally {
                    writer.releaseLock();
                }
            })();
        "#,
    );
    let promise = write.call2(&JsValue::NULL, port, &JsValue::from_str(text))?;
    let _: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(())
}

fn serial_api() -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or(JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    js_sys::Reflect::get(&navigator, &JsValue::from_str("serial"))
}

fn js_value_to_u16(value: &JsValue) -> Option<u16> {
    value.as_f64().and_then(|number| {
        if number.is_finite() && number >= 0.0 && number <= u16::MAX as f64 {
            Some(number as u16)
        } else {
            None
        }
    })
}
