use std::collections::HashMap;

use dioxus::prelude::*;

use crate::data::{DataEntry, DataType, TypedData};

/// The label the bytes received from the serial port go to.
pub const RAW_DATA_LABEL: &str = "raw_data";

#[derive(Clone, Copy)]
pub struct DataContext {
    data_with_labels: Signal<HashMap<String, TypedData>>,
}

impl DataContext {
    /// Runs `f` on the stored data without cloning it.
    pub fn with_data<R>(&self, f: impl FnOnce(&HashMap<String, TypedData>) -> R) -> R {
        f(&self.data_with_labels.read())
    }

    /// The labels holding `data_type`, sorted.
    pub fn labels_of(&self, data_type: DataType) -> Vec<String> {
        let mut labels = self.with_data(|data| {
            data.iter()
                .filter(|(_, data)| data.data_type() == data_type)
                .map(|(label, _)| label.clone())
                .collect::<Vec<_>>()
        });
        labels.sort();
        labels
    }

    /// Appends `entry` to `label`. If `label` holds another type, its entries
    /// are replaced by this one.
    pub fn push<T: DataEntry>(&mut self, label: &str, entry: T) {
        let mut data_with_labels = self.data_with_labels.write();
        if let Some(queue) = data_with_labels.get_mut(label).and_then(T::queue_mut) {
            queue.push(entry);
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
    use_context_provider(|| DataContext {
        data_with_labels: Signal::new(HashMap::new()),
    });

    children
}
