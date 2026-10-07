//! The chunks a port receives, gathered for about a frame before they go
//! into the data together (see `SerialContext::open`).

use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use crate::data::{ByteData, DataContext, RAW_BYTES_LABEL};

/// How long received chunks are gathered before they go into the data
/// together: about a frame, so what reads the data (maps, views) runs at most
/// about once a frame however fast chunks come.
pub(super) const GATHER_MS: u32 = 16;

/// How many chunks are gathered at most: past this, they go into the data at
/// once, so a flood of them (many within a frame) is not dropped from the
/// data (see `MAX_ENTRIES_PER_LABEL`) before what reads it gets to them.
const MAX_GATHERED: usize = 256;

/// The chunks received and not yet in the data: gathered, then pushed together
/// with one write, which what reads the data reacts to once.
#[derive(Clone)]
pub(super) struct Received {
    chunks: Rc<RefCell<Vec<ByteData>>>,
    data: DataContext,
    rx_bytes: Signal<u64>,
}

/// Where a chunk [`Received::gather`] took leaves the gathered ones.
pub(super) enum Gathered {
    /// The first since they last went into the data: they go after a while.
    First,
    /// As many as are gathered: they go now.
    Full,
    More,
}

impl Received {
    pub(super) fn new(data: DataContext, rx_bytes: Signal<u64>) -> Self {
        Self {
            chunks: Rc::default(),
            data,
            rx_bytes,
        }
    }

    pub(super) fn gather(&self, chunk: ByteData) -> Gathered {
        let mut chunks = self.chunks.borrow_mut();
        chunks.push(chunk);
        match chunks.len() {
            1 => Gathered::First,
            n if n >= MAX_GATHERED => Gathered::Full,
            _ => Gathered::More,
        }
    }

    /// Pushes the gathered chunks, oldest first, and counts their bytes.
    pub(super) fn flush(&self) {
        let chunks = std::mem::take(&mut *self.chunks.borrow_mut());
        if chunks.is_empty() {
            return;
        }
        let (mut data, mut rx_bytes) = (self.data, self.rx_bytes);
        *rx_bytes.write() += chunks
            .iter()
            .map(|chunk| chunk.value().len() as u64)
            .sum::<u64>();
        data.push_all(RAW_BYTES_LABEL, chunks);
    }
}
