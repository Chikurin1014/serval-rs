//! Serial ports, through the platform's [`SerialBackend`]. What the open port
//! receives goes to `RAW_BYTES_LABEL`.

use std::{collections::VecDeque, future::Future, pin::Pin, rc::Rc};

use dioxus::{
    core::{Runtime, current_scope_id},
    prelude::*,
};

mod received;

use received::{GATHER_MS, Gathered, Received};

use crate::{
    data::{ByteData, DataContext},
    time::TimeContext,
    toast::{Toaster, use_toaster},
};

/// Not `Send`, as platform serial APIs are not.
pub type LocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

pub type SerialResult<T> = Result<T, String>;

pub const DATA_BITS: u8 = 8;
pub const PARITY: &str = "none";
pub const STOP_BITS: u8 = 1;
pub const FLOW_CONTROL: &str = "none";

const MAX_LOG_ENTRIES: usize = 100;

pub trait SerialBackend {
    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>>;

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>>;
}

pub trait SerialPort {
    fn info(&self) -> PortInfo;

    fn open(&self, baudrate: u32) -> LocalFuture<SerialResult<()>>;

    /// Passes each chunk to `on_chunk` until the port is closed or lost.
    fn read(&self, on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>>;

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>>;

    fn close(&self) -> LocalFuture<SerialResult<()>>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortInfo {
    pub name: String,
    pub vendor: Option<String>,
    pub product: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    Success,
    Info,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub time_ms: i64,
    pub kind: LogKind,
    pub title: String,
    pub detail: String,
}

#[derive(Clone)]
pub struct Port {
    pub id: usize,
    pub info: PortInfo,
    pub baudrate: Option<u32>,
    handle: Rc<dyn SerialPort>,
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.info == other.info && self.baudrate == other.baudrate
    }
}

#[derive(Clone, Copy)]
pub struct SerialContext {
    backend: CopyValue<Rc<dyn SerialBackend>>,
    time: CopyValue<TimeContext>,
    data: DataContext,
    toaster: Toaster,
    ports: Signal<Vec<Port>>,
    next_id: CopyValue<usize>,
    selected: Signal<Option<usize>>,
    open: Signal<Option<usize>>,
    rx_bytes: Signal<u64>,
    tx_bytes: Signal<u64>,
    log: Signal<VecDeque<LogEntry>>,
    /// Runs the port's tasks, so they outlive the component that started them.
    scope: ScopeId,
}

impl SerialContext {
    pub fn ports(&self) -> Vec<Port> {
        self.ports.read().clone()
    }

    pub fn selected_port(&self) -> Option<Port> {
        let selected = (self.selected)()?;
        self.port(selected)
    }

    pub fn is_open(&self) -> bool {
        self.open.read().is_some()
    }

    pub fn rx_bytes(&self) -> u64 {
        (self.rx_bytes)()
    }

    pub fn tx_bytes(&self) -> u64 {
        (self.tx_bytes)()
    }

    pub fn log(&self) -> Vec<LogEntry> {
        self.log.read().iter().rev().cloned().collect()
    }

    pub fn select(&self, id: usize) {
        if !self.is_open() {
            let mut selected = self.selected;
            selected.set(Some(id));
        }
    }

    pub fn set_baudrate(&self, baudrate: u32) {
        let Some(selected) = *self.selected.peek() else {
            return;
        };
        let mut ports = self.ports;
        let mut ports = ports.write();
        if let Some(port) = ports.iter_mut().find(|port| port.id == selected) {
            port.baudrate = Some(baudrate);
        }
    }

    pub fn request_port(&self) {
        let context = *self;
        self.spawn(async move {
            let backend = context.backend.cloned();
            match backend.request_port().await {
                Ok(Some(handle)) => {
                    let id = context.add(handle);
                    context.select(id);
                }
                Ok(None) => {}
                Err(error) => context.report(LogKind::Error, "Failed to add a port", &error),
            }
        });
    }

    pub fn refresh_ports(&self) {
        let context = *self;
        self.spawn(async move {
            if context.is_open() && !context.close_port().await {
                return;
            }
            let backend = context.backend.cloned();
            let handles = match backend.known_ports().await {
                Ok(handles) => handles,
                Err(error) => {
                    context.report(LogKind::Error, "Failed to list the ports", &error);
                    return;
                }
            };
            let mut ports = context.ports;
            ports.write().clear();
            let ids = handles
                .into_iter()
                .map(|handle| context.add(handle))
                .collect::<Vec<_>>();
            let mut selected = context.selected;
            selected.set(ids.first().copied());
        });
    }

    pub fn open(&self) {
        if self.is_open() {
            return;
        }
        let Some(port) = self.selected_port() else {
            return;
        };
        let Some(baudrate) = port.baudrate else {
            return;
        };
        let context = *self;
        self.spawn(async move {
            if let Err(error) = port.handle.open(baudrate).await {
                context.report(LogKind::Error, "Failed to open the port", &error);
                return;
            }
            let mut open = context.open;
            open.set(Some(port.id));
            let (mut rx_bytes, mut tx_bytes) = (context.rx_bytes, context.tx_bytes);
            rx_bytes.set(0);
            tx_bytes.set(0);
            context.report(
                LogKind::Success,
                "Port opened",
                &format!("{} at {baudrate} bps", port.info.name),
            );

            let time = context.time;
            let received = Received::new(context.data, rx_bytes);
            let on_chunk = Box::new(move |chunk: Vec<u8>| {
                let chunk = ByteData::new(time.read().current(), chunk);
                match received.gather(chunk) {
                    Gathered::First => {
                        let received = received.clone();
                        let wait = time.read().after_ms(GATHER_MS);
                        context.spawn(async move {
                            wait.await;
                            received.flush();
                        });
                    }
                    Gathered::Full => received.flush(),
                    Gathered::More => {}
                }
            });
            let read = port.handle.read(on_chunk).await;
            if *open.peek() == Some(port.id) {
                open.set(None);
                if let Err(error) = read {
                    context.report(LogKind::Error, "Connection lost", &error);
                }
            }
        });
    }

    pub fn close(&self) {
        let context = *self;
        self.spawn(async move {
            context.close_port().await;
        });
    }

    pub fn send(&self, bytes: Vec<u8>) {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return;
        };
        let context = *self;
        self.spawn(async move {
            let length = bytes.len() as u64;
            match port.handle.write(bytes).await {
                Ok(()) => {
                    let mut tx_bytes = context.tx_bytes;
                    *tx_bytes.write() += length;
                }
                Err(error) => context.report(LogKind::Error, "Failed to send", &error),
            }
        });
    }

    /// `false` if it failed to close.
    async fn close_port(&self) -> bool {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return true;
        };
        if let Err(error) = port.handle.close().await {
            self.report(LogKind::Error, "Failed to close the port", &error);
            return false;
        }
        let mut open = self.open;
        open.set(None);
        self.report(LogKind::Info, "Port closed", &port.info.name);
        true
    }

    /// Shows `title` in a toast and logs it.
    fn report(&self, kind: LogKind, title: &str, detail: &str) {
        match kind {
            LogKind::Success => self.toaster.success(title, detail),
            LogKind::Info => self.toaster.info(title, detail),
            LogKind::Error => self.toaster.error(title, detail),
        }
        let mut log = self.log;
        let mut log = log.write();
        log.push_back(LogEntry {
            time_ms: self.time.read().current(),
            kind,
            title: title.to_string(),
            detail: detail.to_string(),
        });
        while log.len() > MAX_LOG_ENTRIES {
            log.pop_front();
        }
    }

    fn port(&self, id: usize) -> Option<Port> {
        self.ports.read().iter().find(|port| port.id == id).cloned()
    }

    fn add(&self, handle: Rc<dyn SerialPort>) -> usize {
        let mut next_id = self.next_id;
        let id = *next_id.peek();
        next_id.set(id + 1);
        let mut ports = self.ports;
        ports.write().push(Port {
            id,
            info: handle.info(),
            baudrate: None,
            handle,
        });
        id
    }

    fn spawn(&self, task: impl Future<Output = ()> + 'static) {
        Runtime::current().spawn(self.scope, task);
    }
}

/// Requires `DataContext`, `TimeContext` and `ToastProvider`.
pub fn use_serial_provider(backend: impl FnOnce() -> Rc<dyn SerialBackend>) -> SerialContext {
    let data = use_context::<DataContext>();
    let time = use_context::<TimeContext>();
    let toaster = use_toaster();
    use_context_provider(|| SerialContext {
        backend: CopyValue::new(backend()),
        time: CopyValue::new(time),
        data,
        toaster,
        ports: Signal::new(Vec::new()),
        next_id: CopyValue::new(0),
        selected: Signal::new(None),
        open: Signal::new(None),
        rx_bytes: Signal::new(0),
        tx_bytes: Signal::new(0),
        log: Signal::new(VecDeque::new()),
        scope: current_scope_id(),
    })
}
