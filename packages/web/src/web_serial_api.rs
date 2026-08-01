use std::{collections::HashMap, rc::Rc};

use dioxus::prelude::*;
use js_sys::{Array, Function, Object, Promise};
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use ui::serial::{Port, PortInfo, SerialContext};

#[component]
pub fn WebSerialProvider(children: Element) -> Element {
    let serial = use_context::<SerialContext>();
    let web_ports = use_signal(|| HashMap::<Uuid, JsValue>::new());
    let open_port_id = use_signal(|| None::<Uuid>);

    use_effect(move || {
        let mut serial = serial.clone();
        let mut web_ports = web_ports.clone();

        *serial.request_port.write() = Some(Rc::new({
            move || {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(port) = request_port_js().await {
                        let id = Uuid::new_v4();
                        let info = port_info_from_port(&port);

                        web_ports.write().insert(id, port);
                        serial.ports.write().insert(
                            id,
                            Port {
                                id,
                                info,
                                baudrate: None,
                            },
                        );
                        *serial.selected_id.write() = Some(id);
                        *serial.is_open.write() = false;
                        serial.rx_data.write().clear();
                    }
                });
            }
        }));

        *serial.refresh_ports.write() = Some(Rc::new({
            let mut serial = serial.clone();
            let mut web_ports = web_ports.clone();

            move || {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(list) = get_ports_js().await {
                        let mut next_web_ports = HashMap::new();
                        let mut next_ports = HashMap::new();

                        for port in list {
                            let id = Uuid::new_v4();
                            next_web_ports.insert(id, port);
                        }

                        for (id, port) in &next_web_ports {
                            let previous = (serial.ports)().get(id).cloned();
                            next_ports.insert(
                                *id,
                                Port {
                                    id: *id,
                                    info: port_info_from_port(port),
                                    baudrate: previous.as_ref().and_then(|port| port.baudrate),
                                },
                            );
                        }

                        *serial.ports.write() = next_ports;
                        *web_ports.write() = next_web_ports;
                        *serial.selected_id.write() = (serial.ports)().keys().next().copied();
                    }
                });
            }
        }));

        *serial.set_open.write() = Some(Rc::new({
            let serial = serial.clone();
            let web_ports = web_ports.clone();
            let mut open_port_id = open_port_id.clone();

            move |should_open| {
                let mut serial = serial.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    if should_open {
                        let Some(selected_id) = (serial.selected_id)() else {
                            return;
                        };

                        let Some(baudrate) = (serial.ports)()
                            .get(&selected_id)
                            .and_then(|port| port.baudrate)
                        else {
                            return;
                        };

                        let Some(port) = web_ports.read().get(&selected_id).cloned() else {
                            return;
                        };

                        if open_port_js(&port, baudrate).await.is_ok() {
                            *serial.is_open.write() = true;
                            *open_port_id.write() = Some(selected_id);
                            serial.rx_data.write().clear();

                            let read_port = port.clone();
                            let read_id = open_port_id.clone();
                            let mut serial = serial.clone();

                            wasm_bindgen_futures::spawn_local(async move {
                                let on_chunk = Closure::wrap(Box::new(move |chunk: String| {
                                    if read_id().is_some() {
                                        let timestamp_ms = js_sys::Date::now() as i64;
                                        let data = chunk.into_bytes();
                                        serial.rx_data.write().push_raw(timestamp_ms, data);
                                    }
                                })
                                    as Box<dyn FnMut(String)>);

                                let _ = start_read_loop_js(&read_port, &on_chunk).await;
                                drop(on_chunk);
                            });
                        }
                    } else if let Some(selected_id) = open_port_id() {
                        if let Some(port) = web_ports.read().get(&selected_id).cloned() {
                            if close_port_js(&port).await.is_ok() {
                                *serial.is_open.write() = false;
                                *open_port_id.write() = None;
                                serial.rx_data.write().clear();
                            }
                        }
                    } else {
                        *serial.is_open.write() = false;
                    }
                });
            }
        }));

        *serial.tx_send.write() = Some(Rc::new({
            let serial = serial.clone();
            let web_ports = web_ports.clone();
            let open_port_id = open_port_id.clone();

            move |data| {
                wasm_bindgen_futures::spawn_local(async move {
                    let selected_id = open_port_id().or_else(|| (serial.selected_id)());
                    let Some(selected_id) = selected_id else {
                        return;
                    };

                    let Some(port) = web_ports.read().get(&selected_id).cloned() else {
                        return;
                    };

                    let _ = write_port_js(&port, &data).await;
                });
            }
        }));

        ()
    });

    children
}

async fn get_ports_js() -> Result<Vec<JsValue>, JsValue> {
    let serial = serial_api()?;

    let get_ports = js_sys::Reflect::get(&serial, &JsValue::from_str("getPorts"))?;
    let func: Function = get_ports.dyn_into()?;
    let promise = func.call0(&serial)?;
    let ports_js: JsValue = JsFuture::from(Promise::from(promise)).await?;
    let arr = Array::from(&ports_js);

    Ok(arr.iter().collect())
}

async fn request_port_js() -> Result<JsValue, JsValue> {
    let serial = serial_api()?;

    let request = js_sys::Reflect::get(&serial, &JsValue::from_str("requestPort"))?;
    let func: Function = request.dyn_into()?;
    let promise = func.call0(&serial)?;
    let port: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(port)
}

async fn open_port_js(port: &JsValue, baudrate: u32) -> Result<(), JsValue> {
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

async fn close_port_js(port: &JsValue) -> Result<(), JsValue> {
    let close = js_sys::Reflect::get(port, &JsValue::from_str("close"))?;
    let func: Function = close.dyn_into()?;
    let promise = func.call0(port)?;
    let _: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(())
}

async fn start_read_loop_js(
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

async fn write_port_js(port: &JsValue, text: &[u8]) -> Result<(), JsValue> {
    let write = Function::new_with_args(
        "port, text",
        r#"
            return (async () => {
                const writer = port.writable.getWriter();
                try {
                    await writer.write(text);
                } finally {
                    writer.releaseLock();
                }
            })();
        "#,
    );
    let promise = write.call2(
        &JsValue::NULL,
        port,
        &JsValue::from(js_sys::Uint8Array::from(text)),
    )?;
    let _: JsValue = JsFuture::from(Promise::from(promise)).await?;
    Ok(())
}

fn serial_api() -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or(JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    js_sys::Reflect::get(&navigator, &JsValue::from_str("serial"))
}

fn port_info_from_port(port: &JsValue) -> PortInfo {
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
                            if let Some(device) = usb_ids::Device::from_vid_pid(vid, pid) {
                                return PortInfo {
                                    name: device.name().to_string(),
                                };
                            }
                        }
                    }
                }
            }
        }
    }

    PortInfo {
        name: "Serial Device".to_string(),
    }
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
