//! Serial ports, through the platform's [`SerialBackend`].
//!
//! [`SerialContext`] keeps the ports and which one is open, and pushes what
//! the open port receives to [`RAW_DATA`] in `DataContext`, chunk by chunk.

use std::{future::Future, pin::Pin, rc::Rc};

use dioxus::{
    core::{Runtime, current_scope_id},
    prelude::*,
};

use crate::{
    data::{ByteData, DataContext, RAW_DATA_LABEL},
    time::TimeContext,
};

/// A future run on the UI thread, as platform serial APIs are not `Send`.
pub type LocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// What a platform failed to do, as a message.
pub type SerialResult<T> = Result<T, String>;

/// Where a platform's serial ports come from.
pub trait SerialBackend {
    /// Asks the user to grant a port.
    fn request_port(&self) -> LocalFuture<SerialResult<Rc<dyn SerialPort>>>;

    /// The ports granted before.
    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>>;
}

/// One port of a [`SerialBackend`].
pub trait SerialPort {
    fn info(&self) -> PortInfo;

    fn open(&self, baudrate: u32) -> LocalFuture<SerialResult<()>>;

    /// Passes each chunk received to `on_chunk`, until the port is closed or
    /// lost.
    fn read(&self, on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>>;

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>>;

    /// Closes the port, ending [`SerialPort::read`].
    fn close(&self) -> LocalFuture<SerialResult<()>>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortInfo {
    pub name: String,
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
    ports: Signal<Vec<Port>>,
    next_id: CopyValue<usize>,
    selected: Signal<Option<usize>>,
    /// The port open now; it stays selected while open.
    open: Signal<Option<usize>>,
    /// The provider's scope, which runs the port's tasks, so they outlive
    /// whichever component started them.
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

    /// Selects the port `id`, unless a port is open.
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

    /// Asks the user for a port and adds it, selected unless a port is open.
    pub fn request_port(&self) {
        let context = *self;
        self.spawn(async move {
            let backend = context.backend.cloned();
            if let Ok(handle) = backend.request_port().await {
                let id = context.add(handle);
                context.select(id);
            }
        });
    }

    /// Closes the open port, then lists the ports granted before, selecting
    /// the first one.
    pub fn refresh_ports(&self) {
        let context = *self;
        self.spawn(async move {
            if context.is_open() && context.close_port().await.is_err() {
                return;
            }
            let backend = context.backend.cloned();
            let Ok(handles) = backend.known_ports().await else {
                return;
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

    /// Opens the selected port at its baudrate, and starts receiving.
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
            if port.handle.open(baudrate).await.is_err() {
                return;
            }
            let mut open = context.open;
            open.set(Some(port.id));

            let mut data = context.data;
            let time = context.time;
            let on_chunk = Box::new(move |chunk: Vec<u8>| {
                // Straight into the data, so no chunk waits for (or is lost
                // before) a render
                data.push(RAW_DATA_LABEL, ByteData::new(time.read().current(), chunk));
            });
            let _ = port.handle.read(on_chunk).await;
            // Closed, or lost (e.g. unplugged)
            if *open.peek() == Some(port.id) {
                open.set(None);
            }
        });
    }

    pub fn close(&self) {
        let context = *self;
        self.spawn(async move {
            let _ = context.close_port().await;
        });
    }

    /// Writes `bytes` to the open port.
    pub fn send(&self, bytes: Vec<u8>) {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return;
        };
        self.spawn(async move {
            let _ = port.handle.write(bytes).await;
        });
    }

    async fn close_port(&self) -> SerialResult<()> {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return Ok(());
        };
        port.handle.close().await?;
        let mut open = self.open;
        open.set(None);
        Ok(())
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

/// Provides [`SerialContext`] with the platform's `backend`.
///
/// Requires `DataContext` and `TimeContext` to be provided by an ancestor.
pub fn use_serial_provider(backend: impl FnOnce() -> Rc<dyn SerialBackend>) -> SerialContext {
    let data = use_context::<DataContext>();
    let time = use_context::<TimeContext>();
    use_context_provider(|| SerialContext {
        backend: CopyValue::new(backend()),
        time: CopyValue::new(time),
        data,
        ports: Signal::new(Vec::new()),
        next_id: CopyValue::new(0),
        selected: Signal::new(None),
        open: Signal::new(None),
        scope: current_scope_id(),
    })
}
