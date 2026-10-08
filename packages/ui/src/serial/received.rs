//! Received chunks, gathered for about a frame before they go into the data.

use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use crate::data::{ByteData, DataContext, RAW_BYTES_LABEL};

pub(super) const GATHER_MS: u32 = 16;

/// Past this, they go at once, so a flood is not dropped before it is read.
const MAX_GATHERED: usize = 256;

#[derive(Clone)]
pub(super) struct Received {
    chunks: Rc<RefCell<Vec<ByteData>>>,
    data: DataContext,
    rx_bytes: Signal<u64>,
}

pub(super) enum Gathered {
    /// The first: they go after `GATHER_MS`.
    First,
    /// They go now.
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
