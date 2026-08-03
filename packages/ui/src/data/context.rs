use std::collections::{BTreeMap, VecDeque};

use dioxus::prelude::*;

use super::data_type::{LabeledData, TypedQueue};
use crate::serial::SerialContext;

#[derive(Clone)]
pub struct DataContext {
    pub all_data: Signal<BTreeMap<String, LabeledData>>,
}

impl DataContext {
    pub fn push_raw(&mut self, timestamp_ms: i64, data: Vec<u8>) {
        let mut all_data = self.all_data.write();
        let raw_time = all_data
            .entry("raw_time".to_string())
            .or_insert_with(|| LabeledData {
                label: "raw_time".to_string(),
                queue: TypedQueue::Timestamp(VecDeque::new()),
            });
        match &mut raw_time.queue {
            TypedQueue::Timestamp(queue) => queue.push_back(timestamp_ms),
            other => *other = TypedQueue::Timestamp(VecDeque::from([timestamp_ms])),
        }

        let raw_data = all_data
            .entry("raw_data".to_string())
            .or_insert_with(|| LabeledData {
                label: "raw_data".to_string(),
                queue: TypedQueue::Bytes(VecDeque::new()),
            });
        match &mut raw_data.queue {
            TypedQueue::Bytes(queue) => queue.push_back(data),
            other => *other = TypedQueue::Bytes(VecDeque::from([data])),
        }
    }

    pub fn raw_data(&self) -> Option<Vec<Vec<u8>>> {
        self.all_data
            .read()
            .get("raw_data")
            .and_then(|entry| match &entry.queue {
                TypedQueue::Bytes(queue) => Some(queue.iter().cloned().collect()),
                _ => None,
            })
    }

    pub fn raw_time(&self) -> Option<Vec<i64>> {
        self.all_data
            .read()
            .get("raw_time")
            .and_then(|entry| match &entry.queue {
                TypedQueue::Timestamp(queue) => Some(queue.iter().cloned().collect()),
                _ => None,
            })
    }

    pub fn clear_all(&mut self) {
        self.all_data.write().clear();
    }

    pub fn clear_raw(&mut self) {
        self.clear("raw_data");
        self.clear("raw_time");
    }

    fn clear(&mut self, label: &str) {
        if let Some(entry) = self.all_data.write().get_mut(label) {
            match &mut entry.queue {
                TypedQueue::Timestamp(queue) => queue.clear(),
                TypedQueue::Number(queue) => queue.clear(),
                TypedQueue::String(queue) => queue.clear(),
                TypedQueue::Bytes(queue) => queue.clear(),
            }
        }
    }
}

#[component]
pub fn DataProvider(children: Element) -> Element {
    let all_data = use_signal(BTreeMap::<String, LabeledData>::new);
    let serial = use_context::<SerialContext>();
    let last_seen = use_signal(|| None::<(Option<i64>, Option<Vec<u8>>)>);
    let data_context = DataContext { all_data };
    let data_context_for_provider = data_context.clone();

    use_effect(move || {
        let mut data_context = data_context.clone();
        let mut last_seen = last_seen;

        let rx_data = (serial.rx_data)();
        let current = (rx_data.timestamp_ms, rx_data.data.clone());
        let previous = last_seen();

        if previous.as_ref() != Some(&current) {
            if let (Some(timestamp_ms), Some(data)) = current.clone() {
                data_context.push_raw(timestamp_ms, data);
            }

            *last_seen.write() = Some(current);
        }
    });

    use_context_provider(|| data_context_for_provider.clone());

    children
}
