use std::{collections::HashMap, rc::Rc};

use dioxus::prelude::*;
use uuid::Uuid;

pub type RequestPortAction = Rc<dyn Fn()>;
pub type RefreshPortsAction = Rc<dyn Fn()>;
pub type SetOpenAction = Rc<dyn Fn(bool)>;
pub type SendBytesAction = Rc<dyn Fn(Vec<u8>)>;

#[derive(Clone, Debug, PartialEq)]
pub struct PortInfo {
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RxData {
    pub timestamp_ms: Option<i64>,
    pub data: Option<Vec<u8>>,
}

impl RxData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_raw(&mut self, timestamp_ms: i64, data: Vec<u8>) {
        self.timestamp_ms = Some(timestamp_ms);
        self.data = Some(data);
    }

    pub fn clear(&mut self) {
        self.timestamp_ms = None;
        self.data = None;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub id: Uuid,
    pub info: PortInfo,
    pub baudrate: Option<u32>,
}

#[derive(Clone)]
pub struct SerialContext {
    pub ports: Signal<HashMap<Uuid, Port>>,
    pub selected_id: Signal<Option<Uuid>>,
    pub is_open: Signal<bool>,
    pub rx_data: Signal<RxData>,
    pub request_port: Signal<Option<RequestPortAction>>,
    pub refresh_ports: Signal<Option<RefreshPortsAction>>,
    pub set_open: Signal<Option<SetOpenAction>>,
    pub tx_send: Signal<Option<SendBytesAction>>,
}

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

    children
}
