use std::collections::HashMap;

use dioxus::prelude::*;

use crate::{
    data::{ByteData, DataEntry, TypedData},
    serial::{RxData, SerialContext},
};

#[derive(Clone, Copy)]
pub struct DataContext {
    data_with_labels: Signal<HashMap<String, TypedData>>,
}

impl DataContext {
    pub fn data_with_labels(&self) -> HashMap<String, TypedData> {
        self.data_with_labels.read().clone()
    }

    /// Runs `f` on the stored data without cloning it.
    pub fn with_data<R>(&self, f: impl FnOnce(&HashMap<String, TypedData>) -> R) -> R {
        f(&self.data_with_labels.read())
    }

    pub fn raw_data(&self) -> Option<Vec<ByteData>> {
        self.with_data(|data| {
            let queue = data.get("raw_data").and_then(ByteData::queue)?;
            Some(queue.iter().cloned().collect())
        })
    }

    /// Appends `entry` to `label`. If `label` holds another type, its entries
    /// are replaced by this one.
    pub fn push<T: DataEntry>(&mut self, label: &str, entry: T) {
        let mut data_with_labels = self.data_with_labels.write();
        if let Some(queue) = data_with_labels.get_mut(label).and_then(T::queue_mut) {
            queue.push_back(entry);
        } else {
            data_with_labels.insert(label.to_string(), entry.into());
        }
    }

    pub fn remove(&mut self, label: &str) -> bool {
        self.data_with_labels.write().remove(label).is_some()
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

        data_context.push("raw_data", current);
    });

    use_context_provider(|| data_context);

    children
}
