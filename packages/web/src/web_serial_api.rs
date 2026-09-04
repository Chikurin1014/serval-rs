use std::{collections::HashMap, rc::Rc};

use dioxus::prelude::*;
use js_sys::{Array, Function, Object, Promise};
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use ui::{
    serial::{
        Port, PortInfo, RefreshPortsAction, RequestPortAction, RxData, SendBytesAction,
        SerialContext, SetOpenAction,
    },
    time::TimeContext,
};

#[component]
pub fn SerialProvider(children: Element) -> Element {
    let ports = use_signal(|| HashMap::<Uuid, Port>::new());
    let selected_id = use_signal(|| None::<Uuid>);
    let is_open = use_signal(|| false);
    let rx_data = use_signal(RxData::new);
    let request_port = use_signal(|| None::<RequestPortAction>);
    let refresh_ports = use_signal(|| None::<RefreshPortsAction>);
    let set_open = use_signal(|| None::<SetOpenAction>);
    let tx_send = use_signal(|| None::<SendBytesAction>);

    use_context_provider(|| SerialContext {
        ports,
        selected_id,
        is_open,
        rx_data,
        request_port,
        refresh_ports,
        set_open,
        tx_send,
    });

    let time_context = use_context::<TimeContext>();
    let web_ports = use_signal(|| HashMap::<Uuid, JsValue>::new());

    use_effect(move || {
        let mut request_port = request_port.clone();
        let mut refresh_ports = refresh_ports.clone();
        let mut set_open = set_open.clone();
        let mut tx_send = tx_send.clone();

        *request_port.write() = Some(Rc::new({
            move || {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(port_js) = request_port_js().await {
                        let id = Uuid::new_v4();
                        let info = js_value_to_port_info(&port_js);

                        web_ports.clone().write().insert(id, port_js);
                        ports.clone().write().insert(
                            id,
                            Port {
                                id,
                                info,
                                baudrate: None,
                            },
                        );
                        *selected_id.clone().write() = Some(id);
                        *is_open.clone().write() = false;
                        rx_data.clone().write().clear();
                    }
                });
            }
        }));

        *refresh_ports.write() = Some(Rc::new({
            let mut ports = ports.clone();

            move || {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(action) = set_open() {
                        action(false);
                    }

                    if let Ok(list) = get_ports_js().await {
                        let mut next_web_ports = HashMap::new();
                        let mut next_ports = HashMap::new();

                        for port_js in list {
                            let id = Uuid::new_v4();
                            next_web_ports.insert(id, port_js);
                        }

                        for (id, port_js) in &next_web_ports {
                            next_ports.insert(
                                *id,
                                Port {
                                    id: *id,
                                    info: js_value_to_port_info(port_js),
                                    baudrate: None,
                                },
                            );
                        }

                        *ports.write() = next_ports;
                        *web_ports.clone().write() = next_web_ports;
                        *selected_id.clone().write() = ports().keys().next().copied();
                    }
                });
            }
        }));

        *set_open.write() = Some(Rc::new({
            let web_ports = web_ports.clone();
            let time_context = time_context.clone();

            move |should_open| {
                let web_ports = web_ports.clone();
                let time_context = time_context.clone();
                let Some(selected_id) = selected_id() else {
                    return;
                };

                let Some(baudrate) = ports().get(&selected_id).and_then(|port| port.baudrate)
                else {
                    return;
                };

                let Some(port_js) = web_ports.read().get(&selected_id).cloned() else {
                    return;
                };

                wasm_bindgen_futures::spawn_local(async move {
                    let mut rx_data = rx_data.clone();
                    let mut is_open = is_open.clone();

                    if should_open {
                        if is_open() {
                            return;
                        }

                        if open_port_js(&port_js, baudrate).await.is_ok() {
                            let time_context = time_context.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                let on_chunk =
                                    Closure::wrap(Box::new(move |chunk: js_sys::Uint8Array| {
                                        let timestamp_ms = time_context.current();
                                        let data = chunk.to_vec();
                                        rx_data.write().push_raw(timestamp_ms, data);
                                    })
                                        as Box<dyn FnMut(js_sys::Uint8Array)>);

                                let _ = start_read_loop_js(&port_js, &on_chunk).await;
                                drop(on_chunk);
                            });

                            rx_data.write().clear();
                            *is_open.write() = true;
                        }
                    } else {
                        if close_port_js(&port_js).await.is_ok() {
                            rx_data.write().clear();
                        }
                        *is_open.write() = false;
                    }
                });
            }
        }));

        *tx_send.write() = Some(Rc::new({
            let web_ports = web_ports.clone();

            let Some(selected_id) = selected_id() else {
                return;
            };

            move |data| {
                wasm_bindgen_futures::spawn_local(async move {
                    let Some(port_js) = web_ports.read().get(&selected_id).cloned() else {
                        return;
                    };
                    let _ = write_port_js(&port_js, &data).await;
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
    on_chunk: &Closure<dyn FnMut(js_sys::Uint8Array)>,
) -> Result<(), JsValue> {
    let read_loop = Function::new_with_args(
        "port, onChunk",
        r#"
            return (async () => {
                const reader = port.readable.getReader();
                try {
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) {
                            break;
                        }
                        if (value) {
                            onChunk(value);
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

fn js_value_to_port_info(port_js: &JsValue) -> PortInfo {
    if let Ok(get_info) = js_sys::Reflect::get(port_js, &JsValue::from_str("getInfo")) {
        if !get_info.is_undefined() {
            if let Ok(info_fn) = get_info.dyn_into::<Function>() {
                if let Ok(info) = info_fn.call0(port_js) {
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

fn js_value_to_u16(value_js: &JsValue) -> Option<u16> {
    value_js.as_f64().and_then(|number| {
        if number.is_finite() && number >= 0.0 && number <= u16::MAX as f64 {
            Some(number as u16)
        } else {
            None
        }
    })
}
