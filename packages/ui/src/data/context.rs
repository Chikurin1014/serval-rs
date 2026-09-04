use std::collections::{BTreeMap, VecDeque};

use dioxus::prelude::*;

use super::data_type::{ByteData, TypedData};
use crate::serial::{RxData, SerialContext};

#[derive(Clone)]
pub struct DataContext {
    pub data_with_labels: Signal<BTreeMap<String, TypedData>>,
}

impl DataContext {
    pub fn push_raw(&mut self, data: ByteData) {
        let mut all_data = self.data_with_labels.write();

        let raw_data = all_data
            .entry("raw_data".to_string())
            .or_insert_with(|| TypedData::Bytes(VecDeque::new()));
        match raw_data {
            TypedData::Bytes(queue) => queue.push_back(data),
            other => *other = TypedData::Bytes(VecDeque::from([data])),
        }
    }

    pub fn raw_data(&self) -> Option<Vec<ByteData>> {
        self.data_with_labels
            .read()
            .get("raw_data")
            .and_then(|data| match data {
                TypedData::Bytes(queue) => Some(queue.iter().cloned().collect()),
                _ => None,
            })
    }

    pub fn clear_all(&mut self) {
        self.data_with_labels.write().clear();
    }

    pub fn clear_raw(&mut self) {
        self.clear("raw_data");
    }

    pub fn remove(&mut self, label: &str) -> bool {
        self.data_with_labels.write().remove(label).is_some()
    }

    fn clear(&mut self, label: &str) {
        if let Some(data) = self.data_with_labels.write().get_mut(label) {
            match data {
                TypedData::Number(queue) => queue.clear(),
                TypedData::String(queue) => queue.clear(),
                TypedData::Bytes(queue) => queue.clear(),
            }
        }
    }
}

#[component]
pub fn DataProvider(children: Element) -> Element {
    let data_with_labels = use_signal(BTreeMap::<String, TypedData>::new);
    let serial = use_context::<SerialContext>();
    let mut data_context = DataContext { data_with_labels };
    let data_context_for_provider = data_context.clone();

    use_effect(move || {
        let rx_data = (serial.rx_data)();
        let current_data = match rx_data {
            RxData {
                timestamp_ms: Some(timestamp),
                data: Some(data),
            } => Some(ByteData::new(timestamp, data)),
            _ => None,
        };

        let Some(current) = current_data else {
            return;
        };

        data_context.push_raw(current);
    });

    use_context_provider(|| data_context_for_provider.clone());

    children
}
