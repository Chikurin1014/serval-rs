//! Serial ports, through the platform's [`SerialBackend`]. What the open port
//! receives goes to `RAW_BYTES_LABEL`.

use std::{collections::VecDeque, future::Future, pin::Pin, rc::Rc};

use dioxus::{
    core::{Runtime, current_scope_id},
    prelude::*,
};

pub mod history;
pub mod log;
mod received;

use history::{ReceivedHistory, Unread};
use log::{LogFormat, LogSink, Logger};
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

const MAX_NOTIFICATIONS: usize = 100;
const MAX_OUTGOING: usize = 1000;
/// How long a send stays in [`SerialContext::outgoing`] once sent (or failed).
const OUTGOING_MS: u32 = 3000;

pub trait SerialBackend {
    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>>;

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>>;

    /// A file the user picks for a log, named with `extension`: a new one, or
    /// with `append`, one to add to. `None` if they cancel.
    fn open_log_file(
        &self,
        extension: &str,
        append: bool,
    ) -> LocalFuture<SerialResult<Option<Rc<dyn LogSink>>>> {
        let _ = (extension, append);
        Box::pin(async { Err("Saving a log is not supported here".to_string()) })
    }
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
pub enum NotificationKind {
    Success,
    Info,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    pub time_ms: i64,
    pub kind: NotificationKind,
    pub title: String,
    pub detail: String,
}

/// Bytes given to [`SerialContext::send`], and whether the port took them.
#[derive(Clone, Debug, PartialEq)]
pub struct Outgoing {
    id: u64,
    pub bytes: Vec<u8>,
    pub sent: bool,
    /// Sent (or failed) `OUTGOING_MS` ago.
    expired: bool,
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
    notifications: Signal<VecDeque<Notification>>,
    /// What the console shows; on while ports open and close.
    history: Signal<ReceivedHistory>,
    /// The log file being written, if any; on while ports open and close.
    logger: CopyValue<Option<Logger>>,
    log_format: Signal<Option<LogFormat>>,
    outgoing: Signal<VecDeque<Outgoing>>,
    next_outgoing: CopyValue<u64>,
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

    /// What happened to the ports, newest first.
    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.read().iter().rev().cloned().collect()
    }

    /// The received bytes after `read_to` (see [`ReceivedHistory::since`]); a
    /// reader runs again as more come.
    pub fn received_since(&self, read_to: Option<u64>) -> Unread {
        self.history.read().since(read_to)
    }

    /// The format of the log file being written, if one is.
    pub fn log_format(&self) -> Option<LogFormat> {
        (self.log_format)()
    }

    /// Writes what the port receives (and sends, if `format` keeps it) to
    /// `sink`, until [`Self::stop_log`]; a log already being written ends first.
    pub fn start_log(&self, format: LogFormat, sink: Rc<dyn LogSink>) {
        self.stop_log();
        let (mut logger, mut log_format) = (self.logger, self.log_format);
        logger.set(Some(Logger::new(format, sink)));
        log_format.set(Some(format));
    }

    /// Asks for a file (see [`SerialBackend::open_log_file`]) and logs to it
    /// in `format`. Call it as the user asks: browsers pick files only then.
    pub fn open_log(&self, format: LogFormat, append: bool) {
        let context = *self;
        self.spawn(async move {
            let backend = context.backend.cloned();
            match backend.open_log_file(log::extension(format), append).await {
                Ok(Some(sink)) => {
                    context.start_log(format, sink);
                    context.report(NotificationKind::Info, "Logging started", "");
                }
                Ok(None) => {}
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to open the log", &error)
                }
            }
        });
    }

    /// Ends the log file, with its unfinished lines.
    pub fn stop_log(&self) {
        let (mut logger, mut log_format) = (self.logger, self.log_format);
        let Some(logger) = logger.write().take() else {
            return;
        };
        log_format.set(None);
        let closed = logger.close();
        let context = *self;
        self.spawn(async move {
            match closed.await {
                Ok(()) => context.report(NotificationKind::Info, "Logging stopped", ""),
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to save the log", &error)
                }
            }
        });
    }

    /// The sends not yet sent, or sent in the last `OUTGOING_MS`, oldest first.
    pub fn outgoing(&self) -> Vec<Outgoing> {
        self.outgoing.read().iter().cloned().collect()
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
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to add a port", &error)
                }
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
                    context.report(NotificationKind::Error, "Failed to list the ports", &error);
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
                context.report(NotificationKind::Error, "Failed to open the port", &error);
                return;
            }
            let mut open = context.open;
            open.set(Some(port.id));
            let (mut rx_bytes, mut tx_bytes) = (context.rx_bytes, context.tx_bytes);
            rx_bytes.set(0);
            tx_bytes.set(0);
            let mut outgoing = context.outgoing;
            outgoing.write().clear();
            context.report(
                NotificationKind::Success,
                "Port opened",
                &format!("{} at {baudrate} bps", port.info.name),
            );

            let time = context.time;
            let received = Received::new(context.data, rx_bytes, context.history, context.logger);
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
                    context.report(NotificationKind::Error, "Connection lost", &error);
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
        let id = self.queue(bytes.clone());
        let logged = self.logger.peek().is_some().then(|| bytes.clone());
        let context = *self;
        self.spawn(async move {
            let length = bytes.len() as u64;
            let sent = match port.handle.write(bytes).await {
                Ok(()) => {
                    let mut tx_bytes = context.tx_bytes;
                    *tx_bytes.write() += length;
                    if let Some(bytes) = logged {
                        let now = context.time.read().current();
                        let mut logger = context.logger;
                        let mut logger = logger.write();
                        if let Some(logger) = logger.as_mut() {
                            logger.sent(now, &bytes);
                        }
                    }
                    true
                }
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to send", &error);
                    false
                }
            };
            context.update_outgoing(id, |outgoing| outgoing.sent = sent);
            let expiry = context.time.read().after_ms(OUTGOING_MS);
            expiry.await;
            context.update_outgoing(id, |outgoing| outgoing.expired = true);
        });
    }

    /// Adds `bytes` to the outgoing ones, not yet sent.
    fn queue(&self, bytes: Vec<u8>) -> u64 {
        let mut next_outgoing = self.next_outgoing;
        let id = *next_outgoing.peek();
        next_outgoing.set(id + 1);
        let mut outgoing = self.outgoing;
        let mut outgoing = outgoing.write();
        outgoing.push_back(Outgoing {
            id,
            bytes,
            sent: false,
            expired: false,
        });
        while outgoing.len() > MAX_OUTGOING {
            outgoing.pop_front();
        }
        id
    }

    /// Changes the outgoing `id`, then drops the expired ones from the oldest,
    /// so they go from the left.
    fn update_outgoing(&self, id: u64, change: impl FnOnce(&mut Outgoing)) {
        let mut outgoing = self.outgoing;
        let mut outgoing = outgoing.write();
        if let Some(found) = outgoing.iter_mut().find(|outgoing| outgoing.id == id) {
            change(found);
        }
        while outgoing.front().is_some_and(|oldest| oldest.expired) {
            outgoing.pop_front();
        }
    }

    /// `false` if it failed to close.
    async fn close_port(&self) -> bool {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return true;
        };
        if let Err(error) = port.handle.close().await {
            self.report(NotificationKind::Error, "Failed to close the port", &error);
            return false;
        }
        let mut open = self.open;
        open.set(None);
        self.report(NotificationKind::Info, "Port closed", &port.info.name);
        true
    }

    /// Shows `title` in a toast and keeps it in the notifications.
    fn report(&self, kind: NotificationKind, title: &str, detail: &str) {
        match kind {
            NotificationKind::Success => self.toaster.success(title, detail),
            NotificationKind::Info => self.toaster.info(title, detail),
            NotificationKind::Error => self.toaster.error(title, detail),
        }
        let mut notifications = self.notifications;
        let mut notifications = notifications.write();
        notifications.push_back(Notification {
            time_ms: self.time.read().current(),
            kind,
            title: title.to_string(),
            detail: detail.to_string(),
        });
        while notifications.len() > MAX_NOTIFICATIONS {
            notifications.pop_front();
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
        notifications: Signal::new(VecDeque::new()),
        history: Signal::new(ReceivedHistory::default()),
        logger: CopyValue::new(None),
        log_format: Signal::new(None),
        outgoing: Signal::new(VecDeque::new()),
        next_outgoing: CopyValue::new(0),
        scope: current_scope_id(),
    })
}
