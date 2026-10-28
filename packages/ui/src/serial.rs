//! Serial ports, through the platform's [`SerialBackend`]. What the open port
//! receives goes to `RAW_BYTES_LABEL`.

use std::{collections::VecDeque, fmt, future::Future, pin::Pin, rc::Rc};

use dioxus::{
    core::{Runtime, current_scope_id},
    prelude::*,
};

mod control;
pub mod history;
mod outgoing;
mod received;
pub mod record;

pub use outgoing::{Outgoing, OutgoingFile};
mod recording;

use history::{ReceivedHistory, Unread};
use received::{GATHER_MS, Gathered, Received};
use record::{RecordFile, RecordFormat, Recorder};

use crate::{
    data::{ByteData, DataContext},
    helper::{Choice, Delimiter},
    time::TimeContext,
    toast::{Toaster, use_toaster},
};

/// Not `Send`, as platform serial APIs are not.
pub type LocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

pub type SerialResult<T> = Result<T, String>;

const MAX_NOTIFICATIONS: usize = 100;

pub trait SerialBackend {
    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>>;

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>>;

    /// A file the user picks for a record, named with `extension`: a new one, or
    /// with `append`, one to add to (with its tail). `None` if they cancel.
    fn open_record_file(
        &self,
        extension: &str,
        append: bool,
    ) -> LocalFuture<SerialResult<Option<RecordFile>>> {
        let _ = (extension, append);
        Box::pin(async { Err("Saving a record is not supported here".to_string()) })
    }
}

pub trait SerialPort {
    fn info(&self) -> PortInfo;

    fn open(&self, baudrate: Baudrate, extra: ExtraSettings) -> LocalFuture<SerialResult<()>>;

    /// Passes each chunk to `on_chunk` until the port is closed or lost.
    fn read(&self, on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>>;

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>>;

    /// Sets DTR and RTS, each if given.
    fn set_signals(&self, dtr: Option<bool>, rts: Option<bool>) -> LocalFuture<SerialResult<()>>;

    /// Holds the line low (a break) while on.
    fn set_break(&self, on: bool) -> LocalFuture<SerialResult<()>>;

    fn close(&self) -> LocalFuture<SerialResult<()>>;
}

/// DTR and RTS, as last set: they are not read back. On as a port opens, as
/// the platforms open one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signals {
    pub dtr: bool,
    pub rts: bool,
}

/// A restart of a board through DTR and RTS, as the tools for it do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restart {
    /// DTR, to the reset pin through a capacitor: off a moment, then on again
    /// (as avrdude).
    Arduino,
    /// ESP32, ESP8266 and the like: RTS to EN, through the boards' two
    /// transistors: EN low a moment, DTR off for IO0 high, so it starts its
    /// program (as esptool).
    Espressif,
}

impl Restart {
    /// DTR and RTS to set, each if given, then how long to wait.
    fn steps(self) -> &'static [(Option<bool>, Option<bool>, u32)] {
        match self {
            Self::Arduino => &[(Some(false), Some(false), 100), (Some(true), Some(true), 0)],
            Self::Espressif => &[(Some(false), Some(true), 100), (None, Some(false), 0)],
        }
    }
}

/// What a port opens with: kept for each port, and taken as it opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortSettings {
    /// None until set: a port cannot open without one.
    pub baudrate: Option<Baudrate>,
    pub extra: ExtraSettings,
}

/// A baud rate: above 0 bps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Baudrate(u32);

impl Baudrate {
    pub fn new(bps: u32) -> Option<Self> {
        (bps > 0).then_some(Self(bps))
    }

    pub fn bps(self) -> u32 {
        self.0
    }
}

/// e.g. "9600 bps"
impl fmt::Display for Baudrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bps", self.0)
    }
}

/// How a port frames its bytes, but the baud rate: 8N1 with no flow control
/// at first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtraSettings {
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: u8,
    pub flow_control: FlowControl,
}

impl ExtraSettings {
    /// Those the platforms all take.
    pub const DATA_BITS: [u8; 2] = [7, 8];
    pub const STOP_BITS: [u8; 2] = [1, 2];

    /// e.g. "8N1"
    pub fn frame(&self) -> String {
        format!(
            "{}{}{}",
            self.data_bits,
            self.parity.letter(),
            self.stop_bits
        )
    }
}

impl Default for ExtraSettings {
    fn default() -> Self {
        Self {
            data_bits: 8,
            parity: Parity::None,
            stop_bits: 1,
            flow_control: FlowControl::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

impl Choice for Parity {
    const KIND: &'static str = "parity";
    const ALL: &'static [Self] = &[Self::None, Self::Even, Self::Odd];

    /// As Web Serial names it.
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Even => "even",
            Self::Odd => "odd",
        }
    }
}

impl Parity {
    fn letter(self) -> char {
        match self {
            Self::None => 'N',
            Self::Even => 'E',
            Self::Odd => 'O',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowControl {
    None,
    /// RTS and CTS.
    Hardware,
}

impl Choice for FlowControl {
    const KIND: &'static str = "flow control";
    const ALL: &'static [Self] = &[Self::None, Self::Hardware];

    /// As Web Serial names it.
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Hardware => "hardware",
        }
    }
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

#[derive(Clone)]
pub struct Port {
    pub id: usize,
    pub info: PortInfo,
    pub settings: PortSettings,
    handle: Rc<dyn SerialPort>,
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.info == other.info && self.settings == other.settings
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
    signals: Signal<Signals>,
    /// What Enter sends in the console: a CR and an LF at first.
    line_ending: Signal<Delimiter>,
    /// A break is being sent.
    breaking: CopyValue<bool>,
    notifications: Signal<VecDeque<Notification>>,
    /// What the console shows; on while ports open and close.
    history: Signal<ReceivedHistory>,
    /// The record file being written, if any; on while ports open and close.
    recorder: CopyValue<Option<Recorder>>,
    record_format: Signal<Option<RecordFormat>>,
    /// Counts the records started, to tell the one being written from those before.
    record_id: CopyValue<u64>,
    outgoing: Signal<VecDeque<Outgoing>>,
    /// The outgoing file being sent, if any.
    sending_file: Signal<Option<u64>>,
    /// Asks the file being sent to stop.
    stop_file: CopyValue<bool>,
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

    pub fn signals(&self) -> Signals {
        (self.signals)()
    }

    pub fn line_ending(&self) -> Delimiter {
        (self.line_ending)()
    }

    pub fn set_line_ending(&self, line_ending: Delimiter) {
        let mut current = self.line_ending;
        current.set(line_ending);
    }

    /// What happened to the ports, newest first.
    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.read().iter().rev().cloned().collect()
    }

    /// Forgets the received bytes the console shows (not the data, nor a record
    /// being written): it starts over, empty.
    pub fn clear_history(&self) {
        let mut history = self.history;
        history.write().clear();
    }

    /// The received bytes after `read_to` (see [`ReceivedHistory::since`]); a
    /// reader runs again as more come.
    pub fn received_since(&self, read_to: Option<u64>) -> Unread {
        self.history.read().since(read_to)
    }

    /// Where a reader of the received bytes from now on starts (see
    /// [`Self::received_since`]).
    pub fn received_end(&self) -> u64 {
        self.history.read().end()
    }

    pub fn select(&self, id: usize) {
        if !self.is_open() {
            let mut selected = self.selected;
            selected.set(Some(id));
        }
    }

    /// The selected port's settings, if one is selected.
    pub fn settings(&self) -> Option<PortSettings> {
        self.selected_port().map(|port| port.settings)
    }

    /// Whether the settings may be changed: a port is selected, and none is
    /// open.
    pub fn settings_editable(&self) -> bool {
        self.selected.read().is_some() && !self.is_open()
    }

    /// Sets the selected port's settings: refused with no port selected, or
    /// while it is open.
    pub fn set_settings(&self, settings: PortSettings) -> SerialResult<()> {
        let selected = (*self.selected.peek()).ok_or("No port is selected")?;
        if self.open.peek().is_some() {
            return Err("The port is open: close it to change its settings".to_string());
        }
        let mut ports = self.ports;
        let mut ports = ports.write();
        let port = ports
            .iter_mut()
            .find(|port| port.id == selected)
            .ok_or("No port is selected")?;
        port.settings = settings;
        Ok(())
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
            if context.is_open() && context.close_port().await.is_err() {
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
        let context = *self;
        self.spawn(async move {
            let _ = context.open_port().await;
        });
    }

    /// Opens the port selected with its settings, if none is open: read from as
    /// it returns. Why it failed is told as by [`Self::open`] too.
    pub async fn open_port(&self) -> SerialResult<()> {
        if self.is_open() {
            return Ok(());
        }
        let port = self.selected_port().ok_or("No port is selected")?;
        let settings = port.settings;
        let baudrate = settings.baudrate.ok_or("No baud rate is set")?;
        if let Err(error) = port.handle.open(baudrate, settings.extra).await {
            self.report(NotificationKind::Error, "Failed to open the port", &error);
            return Err(error);
        }
        let context = *self;
        let mut open = context.open;
        open.set(Some(port.id));
        let (mut rx_bytes, mut tx_bytes) = (context.rx_bytes, context.tx_bytes);
        rx_bytes.set(0);
        let mut signals = context.signals;
        signals.set(Signals {
            dtr: true,
            rts: true,
        });
        tx_bytes.set(0);
        let mut outgoing = context.outgoing;
        outgoing.write().clear();
        let opened = format!("{} at {baudrate}", port.info.name);
        context.record_event(&format!("Port opened: {opened}"));
        context.report(NotificationKind::Success, "Port opened", &opened);

        // Read until closed or lost, on its own
        self.spawn(async move {
            let time = context.time;
            let received = Received::new(context.data, rx_bytes, context.history, context.recorder);
            let last = received.clone();
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
            // After the last chunks, still gathered
            last.flush();
            context.record_event(&match &read {
                Ok(()) => format!("Port closed: {}", port.info.name),
                Err(error) => format!("Connection lost: {}: {error}", port.info.name),
            });
            if *open.peek() == Some(port.id) {
                open.set(None);
                if let Err(error) = read {
                    context.report(NotificationKind::Error, "Connection lost", &error);
                }
            }
        });
        Ok(())
    }

    pub fn close(&self) {
        let context = *self;
        self.spawn(async move {
            let _ = context.close_port().await;
        });
    }

    /// Closes the open port, if one is: closed as it returns. Why it failed is
    /// told as by [`Self::close`] too.
    pub async fn close_port(&self) -> SerialResult<()> {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return Ok(());
        };
        if let Err(error) = port.handle.close().await {
            self.report(NotificationKind::Error, "Failed to close the port", &error);
            return Err(error);
        }
        let mut open = self.open;
        open.set(None);
        self.report(NotificationKind::Info, "Port closed", &port.info.name);
        Ok(())
    }

    /// Writes what happened to the port to the record file, if one is written.
    fn record_event(&self, event: &str) {
        let now = self.time.read().current();
        let mut recorder = self.recorder;
        let mut recorder = recorder.write();
        if let Some(recorder) = recorder.as_mut() {
            recorder.event(now, event);
        }
    }

    /// Shows an error in a toast and keeps it in the notifications.
    pub fn report_error(&self, title: &str, detail: &str) {
        self.report(NotificationKind::Error, title, detail);
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
            settings: PortSettings::default(),
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
        signals: Signal::new(Signals {
            dtr: true,
            rts: true,
        }),
        line_ending: Signal::new(Delimiter::CrLf),
        breaking: CopyValue::new(false),
        notifications: Signal::new(VecDeque::new()),
        history: Signal::new(ReceivedHistory::default()),
        recorder: CopyValue::new(None),
        record_format: Signal::new(None),
        record_id: CopyValue::new(0),
        outgoing: Signal::new(VecDeque::new()),
        sending_file: Signal::new(None),
        stop_file: CopyValue::new(false),
        next_outgoing: CopyValue::new(0),
        scope: current_scope_id(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Baudrate, ExtraSettings, Parity};

    #[test]
    fn frame_is_8n1_at_first() {
        assert_eq!(ExtraSettings::default().frame(), "8N1");
    }

    #[test]
    fn frame_names_the_parity() {
        let settings = ExtraSettings {
            data_bits: 7,
            parity: Parity::Even,
            stop_bits: 2,
            ..ExtraSettings::default()
        };
        assert_eq!(settings.frame(), "7E2");
    }

    #[test]
    fn a_baud_rate_is_above_0() {
        assert_eq!(Baudrate::new(0), None);
        assert_eq!(Baudrate::new(9600).map(Baudrate::bps), Some(9600));
        assert_eq!(Baudrate::new(9600).unwrap().to_string(), "9600 bps");
    }
}
