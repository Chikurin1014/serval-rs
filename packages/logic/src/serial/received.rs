//! Received chunks, gathered for about a frame before they go into the data.

use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    rc::Rc,
    task::{Poll, Waker},
};

use dioxus::{core::queue_effect, prelude::*};

use super::{history::ReceivedHistory, record::Recorder};
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
    recorder: CopyValue<Option<Recorder>>,
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
        recorder: CopyValue<Option<Recorder>>,
    ) -> Self {
        Self {
            chunks: Rc::default(),
            data,
            rx_bytes,
            history,
            recorder,
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
        let (mut data, mut rx_bytes, mut history, mut recorder) =
            (self.data, self.rx_bytes, self.history, self.recorder);
        *rx_bytes.write() += chunks
            .iter()
            .map(|chunk| chunk.value().len() as u64)
            .sum::<u64>();
        // Before the data, where they may be dropped or cleared
        history.write().extend(&chunks);
        if let Some(recorder) = recorder.write().as_mut() {
            recorder.received(&chunks);
        }
        data.push_all(RAW_BYTES_LABEL, chunks);
    }
}

/// Done once the effects queued so far have run, e.g. those of a page just
/// shown, which add its stylesheets. Dioxus runs effects only once no task
/// makes a component render: chunks flushed every `GATHER_MS` would render
/// the page each time before them, and they would never run.
pub(super) fn after_effects() -> impl Future<Output = ()> {
    let ran: Rc<RefCell<(bool, Option<Waker>)>> = Rc::default();
    queue_effect({
        let ran = ran.clone();
        move || {
            let mut ran = ran.borrow_mut();
            ran.0 = true;
            if let Some(waker) = ran.1.take() {
                waker.wake();
            }
        }
    });
    poll_fn(move |cx| {
        let mut ran = ran.borrow_mut();
        if ran.0 {
            Poll::Ready(())
        } else {
            ran.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use dioxus::prelude::*;

    use super::*;
    use crate::{
        data::{DataProvider, TypedData},
        serial::record::{RecordFormat, tests::Recorded},
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
        let recorder = use_hook(|| {
            let format = RecordFormat::Text {
                timestamps: false,
                sent: false,
            };
            CopyValue::new(Some(Recorder::new(format, SINK.with(Rc::clone) as _)))
        });
        GRABBED.with(|cell| {
            *cell.borrow_mut() = Some((Received::new(data, rx_bytes, history, recorder), data))
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
    fn flush_records_the_chunks_and_keeps_them_in_the_data() {
        let mut dom = VirtualDom::new(|| rsx! { DataProvider { Grab {} } });
        dom.rebuild_in_place();
        dom.in_runtime(|| {
            let (received, mut data) = GRABBED.with(|cell| cell.borrow().clone()).unwrap();
            received.gather(ByteData::new(1, b"temp:20".to_vec()));
            received.gather(ByteData::new(2, b".5\n".to_vec()));
            received.flush();
            assert_eq!(raw_bytes(&data), 2);
            assert_eq!(SINK.with(|sink| sink.text()), "temp:20.5\n");

            // The record keeps what the data no longer does
            data.clear_all();
            received.gather(ByteData::new(3, b"volt:3.3\n".to_vec()));
            received.flush();
            assert_eq!(SINK.with(|sink| sink.text()), "temp:20.5\nvolt:3.3\n");
        });
    }
}
