//! What is sent to the open port: bytes, or a file a chunk at a time, each
//! kept a while once sent to show whether the port took it.

use dioxus::prelude::*;

use super::{NotificationKind, Port, SerialContext};
use crate::helper::format_bytes;

const MAX_OUTGOING: usize = 1000;
/// How long a send stays in [`SerialContext::outgoing`] once sent (or failed).
const OUTGOING_MS: u32 = 3000;
/// How much of a file is written at once: a while each, to show its progress
/// and to stop it between them.
const FILE_CHUNK_BYTES: usize = 4096;

/// Bytes given to [`SerialContext::send`], or a file to
/// [`SerialContext::send_file`], and whether the port took them.
#[derive(Clone, Debug, PartialEq)]
pub struct Outgoing {
    id: u64,
    /// None for a file: too many to show.
    pub bytes: Vec<u8>,
    pub file: Option<OutgoingFile>,
    pub sent: bool,
    /// Written to the port, or failed to be: not to be waited for.
    done: bool,
    /// Sent (or failed) `OUTGOING_MS` ago.
    expired: bool,
}

impl Outgoing {
    /// Whether it is yet to be written to the port.
    pub fn is_pending(&self) -> bool {
        !self.done
    }
}

/// A file being sent, by its name, and how far.
#[derive(Clone, Debug, PartialEq)]
pub struct OutgoingFile {
    pub name: String,
    pub size: usize,
    pub sent: usize,
    /// Stopped before all of it was sent.
    pub stopped: bool,
}

impl SerialContext {
    /// The sends not yet sent, or sent in the last `OUTGOING_MS`, oldest first.
    pub fn outgoing(&self) -> Vec<Outgoing> {
        self.outgoing.read().iter().cloned().collect()
    }

    /// Not while a file is being sent: they would go in between its bytes.
    pub fn send(&self, bytes: Vec<u8>) {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return;
        };
        if self.sending_file.peek().is_some() {
            return;
        }
        let id = self.queue(bytes.clone(), None);
        let context = *self;
        self.spawn(async move {
            let sent = context.write_sent(&port, bytes).await;
            context.update_outgoing(id, |outgoing| {
                outgoing.sent = sent;
                outgoing.done = true;
            });
            context.expire_outgoing(id).await;
        });
    }

    /// The bytes given to [`Self::send`] yet to be written, oldest first.
    pub fn unsent(&self) -> Vec<u8> {
        self.outgoing
            .read()
            .iter()
            .filter(|outgoing| outgoing.is_pending() && outgoing.file.is_none())
            .flat_map(|outgoing| outgoing.bytes.iter().copied())
            .collect()
    }

    /// Whether any send (or file) is yet to be written.
    pub fn has_pending(&self) -> bool {
        self.outgoing.read().iter().any(Outgoing::is_pending)
    }

    /// Whether a file is being sent.
    pub fn is_sending_file(&self) -> bool {
        self.sending_file.read().is_some()
    }

    /// The file being sent, and how far, if one is.
    pub fn sending_file(&self) -> Option<OutgoingFile> {
        let id = (*self.sending_file.read())?;
        self.outgoing
            .read()
            .iter()
            .find(|outgoing| outgoing.id == id)
            .and_then(|outgoing| outgoing.file.clone())
    }

    /// Sends the file `name` holding `bytes`, a while at a time, until all of
    /// it is sent or [`Self::stop_file`]; one at a time.
    pub fn send_file(&self, name: String, bytes: Vec<u8>) {
        let Some(port) = (*self.open.peek()).and_then(|id| self.port(id)) else {
            return;
        };
        if self.sending_file.peek().is_some() {
            return;
        }
        let file = OutgoingFile {
            name: name.clone(),
            size: bytes.len(),
            sent: 0,
            stopped: false,
        };
        let id = self.queue(Vec::new(), Some(file));
        let (mut sending_file, mut stop_file) = (self.sending_file, self.stop_file);
        sending_file.set(Some(id));
        stop_file.set(false);
        let context = *self;
        self.spawn(async move {
            let mut sent = 0;
            for chunk in bytes.chunks(FILE_CHUNK_BYTES) {
                if *stop_file.peek() || !context.write_sent(&port, chunk.to_vec()).await {
                    break;
                }
                sent += chunk.len();
                context.update_outgoing(id, |outgoing| {
                    if let Some(file) = outgoing.file.as_mut() {
                        file.sent = sent;
                    }
                });
            }
            let whole = sent == bytes.len();
            context.update_outgoing(id, |outgoing| {
                outgoing.sent = whole;
                outgoing.done = true;
                if let Some(file) = outgoing.file.as_mut() {
                    file.stopped = !whole;
                }
            });
            sending_file.set(None);
            if whole {
                context.report(NotificationKind::Success, "File sent", &name);
            } else if *stop_file.peek() {
                let detail = format!(
                    "{name}: {} of {}",
                    format_bytes(sent as u64),
                    format_bytes(bytes.len() as u64)
                );
                context.report(NotificationKind::Info, "Sending stopped", &detail);
            }
            context.expire_outgoing(id).await;
        });
    }

    /// Stops the file being sent, after the part being written.
    pub fn stop_file(&self) {
        let mut stop_file = self.stop_file;
        stop_file.set(true);
    }

    /// Writes `bytes` to `port`, counted and recorded; whether it was.
    async fn write_sent(&self, port: &Port, bytes: Vec<u8>) -> bool {
        let length = bytes.len() as u64;
        let recorded = self.recorder.peek().is_some().then(|| bytes.clone());
        if let Err(error) = port.handle.write(bytes).await {
            self.report(NotificationKind::Error, "Failed to send", &error);
            return false;
        }
        let mut tx_bytes = self.tx_bytes;
        *tx_bytes.write() += length;
        if let Some(bytes) = recorded {
            let now = self.time.read().current();
            let mut recorder = self.recorder;
            let mut recorder = recorder.write();
            if let Some(recorder) = recorder.as_mut() {
                recorder.sent(now, &bytes);
            }
        }
        true
    }

    /// Lets the outgoing `id`, done, go after `OUTGOING_MS`.
    async fn expire_outgoing(&self, id: u64) {
        let expiry = self.time.read().after_ms(OUTGOING_MS);
        expiry.await;
        self.update_outgoing(id, |outgoing| outgoing.expired = true);
    }

    /// Adds `bytes` to the outgoing ones, not yet sent.
    fn queue(&self, bytes: Vec<u8>, file: Option<OutgoingFile>) -> u64 {
        let mut next_outgoing = self.next_outgoing;
        let id = *next_outgoing.peek();
        next_outgoing.set(id + 1);
        let mut outgoing = self.outgoing;
        let mut outgoing = outgoing.write();
        outgoing.push_back(Outgoing {
            id,
            bytes,
            file,
            sent: false,
            done: false,
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
}
