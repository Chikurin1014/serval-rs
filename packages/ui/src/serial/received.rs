//! Received chunks, gathered for about a frame before they go into the data.

use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use super::{history::ReceivedHistory, log::Logger};
use crate::data::{ByteData, DataContext, RAW_BYTES_LABEL};

pub(super) const GATHER_MS: u32 = 16;

/// Past this, they go at once, so a flood is not dropped before it is read.
const MAX_GATHERED: usize = 256;

#[derive(Clone)]
pub(super) struct Received {
    chunks: Rc<RefCell<Vec<ByteData>>>,
    data: DataContext,
    rx_bytes: Signal<u64>,
    history: Signal<ReceivedHistory>,
    logger: CopyValue<Option<Logger>>,
}

pub(super) enum Gathered {
    /// The first: they go after `GATHER_MS`.
    First,
    /// They go now.
    Full,
    More,
}

impl Received {
    pub(super) fn new(
        data: DataContext,
        rx_bytes: Signal<u64>,
        history: Signal<ReceivedHistory>,
        logger: CopyValue<Option<Logger>>,
    ) -> Self {
        Self {
            chunks: Rc::default(),
            data,
            rx_bytes,
            history,
            logger,
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
        let (mut data, mut rx_bytes, mut history, mut logger) =
            (self.data, self.rx_bytes, self.history, self.logger);
        *rx_bytes.write() += chunks
            .iter()
            .map(|chunk| chunk.value().len() as u64)
            .sum::<u64>();
        // Before the data, where they may be dropped or cleared
        history.write().extend(&chunks);
        if let Some(logger) = logger.write().as_mut() {
            logger.received(&chunks);
        }
        data.push_all(RAW_BYTES_LABEL, chunks);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use dioxus::prelude::*;

    use super::*;
    use crate::{
        data::{DataProvider, TypedData},
        serial::log::{LogFormat, tests::Recorded},
    };

    thread_local! {
        static GRABBED: RefCell<Option<(Received, DataContext)>> = const { RefCell::new(None) };
        static SINK: Rc<Recorded> = Rc::new(Recorded::default());
    }

    #[component]
    fn Grab() -> Element {
        let data = use_context::<DataContext>();
        let rx_bytes = use_signal(|| 0);
        let history = use_signal(ReceivedHistory::default);
        let logger = use_hook(|| {
            let format = LogFormat::Text {
                timestamps: false,
                sent: false,
            };
            CopyValue::new(Some(Logger::new(format, SINK.with(Rc::clone) as _)))
        });
        GRABBED.with(|cell| {
            *cell.borrow_mut() = Some((Received::new(data, rx_bytes, history, logger), data))
        });
        rsx! {}
    }

    fn raw_bytes(data: &DataContext) -> usize {
        data.with_label(RAW_BYTES_LABEL, |data| match data {
            TypedData::Bytes(queue) => queue.len(),
            _ => 0,
        })
        .unwrap_or(0)
    }

    #[test]
    fn flush_logs_the_chunks_and_keeps_them_in_the_data() {
        let mut dom = VirtualDom::new(|| rsx! { DataProvider { Grab {} } });
        dom.rebuild_in_place();
        dom.in_runtime(|| {
            let (received, mut data) = GRABBED.with(|cell| cell.borrow().clone()).unwrap();
            received.gather(ByteData::new(1, b"temp:20".to_vec()));
            received.gather(ByteData::new(2, b".5\n".to_vec()));
            received.flush();
            assert_eq!(raw_bytes(&data), 2);
            assert_eq!(SINK.with(|sink| sink.text()), "temp:20.5\n");

            // The log keeps what the data no longer does
            data.clear_all();
            received.gather(ByteData::new(3, b"volt:3.3\n".to_vec()));
            received.flush();
            assert_eq!(SINK.with(|sink| sink.text()), "temp:20.5\nvolt:3.3\n");
        });
    }
}
