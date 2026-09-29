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
