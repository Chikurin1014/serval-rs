use std::collections::{HashMap, VecDeque};

use dioxus::prelude::*;

use super::data_type::{ByteData, TypedData};
use crate::{
    data::{NumberData, StringData},
    serial::{RxData, SerialContext},
};

#[derive(Clone)]
pub struct DataContext {
    data_with_labels: Signal<HashMap<String, TypedData>>,
}

impl DataContext {
    pub fn data_with_labels(&self) -> HashMap<String, TypedData> {
        self.data_with_labels.read().clone()
    }

    pub fn get(&self, label: &str) -> Option<TypedData> {
        self.data_with_labels.read().get(label).cloned()
    }

    pub fn get_bytes(&self, label: &str) -> Option<VecDeque<ByteData>> {
        match self.get(label) {
            Some(TypedData::Bytes(queue)) => Some(queue),
            _ => None,
        }
    }

    pub fn get_string(&self, label: &str) -> Option<VecDeque<StringData>> {
        match self.get(label) {
            Some(TypedData::String(queue)) => Some(queue),
            _ => None,
        }
    }

    pub fn raw_data(&self) -> Option<Vec<ByteData>> {
        self.get_bytes("raw_data")
            .map(|queue| queue.into_iter().collect())
    }

    pub fn push_number(&mut self, label: &str, data: NumberData) {
        let mut data_with_labels = self.data_with_labels.write();
        if let Some(existing_data) = data_with_labels.get_mut(label) {
            match existing_data {
                TypedData::Number(queue) => queue.push_back(data),
                _ => {
                    *existing_data = data.into();
                }
            }
        } else {
            data_with_labels.insert(label.to_string(), data.into());
        }
    }

    pub fn push_string(&mut self, label: &str, data: StringData) {
        let mut data_with_labels = self.data_with_labels.write();
        if let Some(existing_data) = data_with_labels.get_mut(label) {
            match existing_data {
                TypedData::String(queue) => queue.push_back(data),
                _ => {
                    *existing_data = data.into();
                }
            }
        } else {
            data_with_labels.insert(label.to_string(), data.into());
        }
    }

    pub fn push_bytes(&mut self, label: &str, data: ByteData) {
        let mut data_with_labels = self.data_with_labels.write();
        if let Some(existing_data) = data_with_labels.get_mut(label) {
            match existing_data {
                TypedData::Bytes(queue) => queue.push_back(data),
                _ => {
                    *existing_data = data.into();
                }
            }
        } else {
            data_with_labels.insert(label.to_string(), data.into());
        }
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

    pub fn clear_raw(&mut self) {
        self.clear("raw_data");
    }

    pub fn clear_all(&mut self) {
        self.data_with_labels.write().clear();
    }
}

#[component]
pub fn DataProvider(children: Element) -> Element {
    let data_with_labels = use_signal(HashMap::<String, TypedData>::new);
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

        data_context.push_bytes("raw_data", current);
    });

    use_context_provider(|| data_context_for_provider.clone());

    children
}
