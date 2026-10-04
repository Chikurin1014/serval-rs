use std::{collections::HashMap, rc::Rc};

use dioxus::prelude::*;
use js_sys::{Array, Uint8Array};
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, JsValue};

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
                    if let Ok(port_js) = web_serial::request_port().await {
                        let id = Uuid::new_v4();
                        let info = port_info(&port_js);

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

                    if let Ok(list) = web_serial::get_ports()
                        .await
                        .map(|ports| Array::from(&ports))
                    {
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
                                    info: port_info(port_js),
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

                        if web_serial::open_port(&port_js, baudrate).await.is_ok() {
                            let time_context = time_context.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                let on_chunk = Closure::wrap(Box::new(move |chunk: Uint8Array| {
                                    let timestamp_ms = time_context.current();
                                    let data = chunk.to_vec();
                                    rx_data.write().push_raw(timestamp_ms, data);
                                })
                                    as Box<dyn FnMut(Uint8Array)>);

                                let _ = web_serial::read_loop(&port_js, &on_chunk).await;
                                drop(on_chunk);
                            });

                            rx_data.write().clear();
                            *is_open.write() = true;
                        }
                    } else {
                        if web_serial::close_port(&port_js).await.is_ok() {
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
                    let _ =
                        web_serial::write_port(&port_js, Uint8Array::from(data.as_slice())).await;
                });
            }
        }));

        ()
    });

    children
}

/// Web Serial calls, see `serial.js`.
mod web_serial {
    use js_sys::Uint8Array;
    use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsValue};

    #[wasm_bindgen(module = "/src/serial.js")]
    extern "C" {
        /// Resolves to an array of the ports this page was granted before.
        #[wasm_bindgen(catch, js_name = getPorts)]
        pub async fn get_ports() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, js_name = requestPort)]
        pub async fn request_port() -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, js_name = openPort)]
        pub async fn open_port(port: &JsValue, baud_rate: u32) -> Result<(), JsValue>;

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
