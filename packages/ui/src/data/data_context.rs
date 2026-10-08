use std::collections::{BTreeMap, HashMap};

use dioxus::{core::current_scope_id, prelude::*, signals::Owner};

use crate::{
    data::{DataEntry, DataType, TypedData},
    helper::make_owned,
};

/// The label the bytes received from the serial port go to.
pub const RAW_BYTES_LABEL: &str = "raw_bytes";

/// The data, by label. Each label is its own signal, so a reader runs again only
/// for writes to the labels it reads (and for labels coming and going).
#[derive(Clone, Copy)]
pub struct DataContext {
    /// Each label's type; written only as labels come, go or change type.
    labels: Signal<BTreeMap<String, DataType>>,
    /// Not reactive: only each label's own signal is.
    queues: CopyValue<HashMap<String, LabelData>>,
    scope: ScopeId,
}

struct LabelData {
    data: Signal<TypedData>,
    _owner: Owner,
}

impl DataContext {
    /// The labels holding `data_type`, sorted.
    pub fn labels_of(&self, data_type: DataType) -> Vec<String> {
        self.labels
            .read()
            .iter()
            .filter(|(_, held)| **held == data_type)
            .map(|(label, _)| label.clone())
            .collect()
    }

    /// The type `label` holds, if it is there.
    pub fn data_type_of(&self, label: &str) -> Option<DataType> {
        self.labels.read().get(label).copied()
    }

    /// Runs `f` on `label`'s entries, if there is such a label.
    pub fn with_label<R>(&self, label: &str, f: impl FnOnce(&TypedData) -> R) -> Option<R> {
        if !self.labels.read().contains_key(label) {
            return None;
        }
        let data = self.queues.read().get(label)?.data;
        let read = data.read();
        Some(f(&read))
    }

    /// Runs `f` on the labels (of `data_type`, if given), sorted, with their entries.
    pub fn with_each<R>(
        &self,
        data_type: Option<DataType>,
        f: impl FnOnce(&[(&str, &TypedData)]) -> R,
    ) -> R {
        let labels = self.labels.read();
        let signals = {
            let queues = self.queues.read();
            labels
                .iter()
                .filter(|(_, held)| data_type.is_none_or(|data_type| **held == data_type))
                .filter_map(|(label, _)| Some((label.as_str(), queues.get(label)?.data)))
                .collect::<Vec<_>>()
        };
        let reads = signals
            .iter()
            .map(|(label, data)| (*label, data.read_unchecked()))
            .collect::<Vec<_>>();
        let data = reads
            .iter()
            .map(|(label, data)| (*label, &**data))
            .collect::<Vec<_>>();
        f(&data)
    }

    /// Appends `entry` to `label`, replacing what it held if of another type.
    pub fn push<T: DataEntry>(&mut self, label: &str, entry: T) {
        self.push_all(label, [entry]);
    }

    /// As [`Self::push`] for each of `entries`, in one write.
    pub fn push_all<T: DataEntry>(&mut self, label: &str, entries: impl IntoIterator<Item = T>) {
        let mut entries = entries.into_iter().peekable();
        if entries.peek().is_none() {
            return;
        }
        let existing = self
            .queues
            .read()
            .get(label)
            .map(|label_data| label_data.data);
        if let Some(mut data) = existing.filter(|data| T::queue(&data.peek()).is_some()) {
            let mut data = data.write();
            let queue = T::queue_mut(&mut data).expect("checked above");
            for entry in entries {
                queue.push(entry);
            }
            return;
        }

        let mut typed: TypedData = entries.next().expect("checked above").into();
        let queue = T::queue_mut(&mut typed).expect("made of a `T`");
        for entry in entries {
            queue.push(entry);
        }
        let data_type = typed.data_type();
        match existing {
            Some(mut data) => data.set(typed),
            None => {
                let (data, owner) = make_owned(self.scope, || Signal::new(typed));
                self.queues.write().insert(
                    label.to_string(),
                    LabelData {
                        data,
                        _owner: owner,
                    },
                );
            }
        }
        self.labels.write().insert(label.to_string(), data_type);
    }

    /// Whether there was such a label.
    pub fn remove(&mut self, label: &str) -> bool {
        let label_removed = self.labels.write().remove(label).is_some();
        let queue_removed = self.queues.write().remove(label).is_some();
        debug_assert_eq!(
            label_removed, queue_removed,
            "labels and queues out of step"
        );
        label_removed || queue_removed
    }

    /// Whether there was any label.
    pub fn clear_all(&mut self) -> bool {
        let had_labels = !std::mem::take(&mut *self.labels.write()).is_empty();
        let had_queues = !std::mem::take(&mut *self.queues.write()).is_empty();
        debug_assert_eq!(had_labels, had_queues, "labels and queues out of step");
        had_labels || had_queues
    }
}

#[component]
pub fn DataProvider(children: Element) -> Element {
    use_context_provider(|| DataContext {
        labels: Signal::new(BTreeMap::new()),
        queues: CopyValue::new(HashMap::new()),
        scope: current_scope_id(),
    });

    children
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use dioxus::prelude::*;

    use super::{DataContext, DataProvider};
    use crate::data::{DataType, NumberData, StringData, TypedData};

    thread_local! {
        static DATA: RefCell<Option<DataContext>> = const { RefCell::new(None) };
        static RENDERS: Cell<u32> = const { Cell::new(0) };
        static TEMP_LEN: Cell<Option<usize>> = const { Cell::new(None) };
    }

    #[component]
    fn Grab() -> Element {
        let data = use_context::<DataContext>();
        DATA.with(|cell| *cell.borrow_mut() = Some(data));
        rsx! {}
    }

    #[component]
    fn TempReader() -> Element {
        let data = use_context::<DataContext>();
        RENDERS.with(|renders| renders.set(renders.get() + 1));
        let len = data.with_label("temp", |temp| match temp {
            TypedData::Number(queue) => queue.len(),
            _ => 0,
        });
        TEMP_LEN.with(|cell| cell.set(len));
        rsx! {}
    }

    /// How often `TempReader` rendered after `write`, and the `temp` length it saw.
    fn write(dom: &mut VirtualDom, write: impl FnOnce(&mut DataContext)) -> (u32, Option<usize>) {
        let before = RENDERS.with(Cell::get);
        dom.in_runtime(|| {
            let mut data = DATA.with(|cell| cell.borrow().expect("grabbed"));
            write(&mut data);
        });
        dom.process_events();
        dom.render_immediate_to_vec();
        (RENDERS.with(Cell::get) - before, TEMP_LEN.with(Cell::get))
    }

    fn dom() -> VirtualDom {
        let mut dom = VirtualDom::new(|| {
            rsx! {
                DataProvider {
                    Grab {}
                    TempReader {}
                }
            }
        });
        dom.rebuild_in_place();
        dom
    }

    #[test]
    fn a_reader_runs_again_only_for_its_label() {
        let mut dom = dom();
        assert_eq!(TEMP_LEN.with(Cell::get), None);

        assert_eq!(
            write(&mut dom, |data| data.push("temp", NumberData::new(1, 1.0))),
            (1, Some(1))
        );
        assert_eq!(
            write(&mut dom, |data| data.push("temp", NumberData::new(2, 2.0))),
            (1, Some(2))
        );

        // Another label: only its creation is seen, not its writes
        assert_eq!(
            write(&mut dom, |data| data.push("other", NumberData::new(1, 1.0))).0,
            1
        );
        assert_eq!(
            write(&mut dom, |data| data.push("other", NumberData::new(2, 2.0))).0,
            0
        );
        assert_eq!(
            write(&mut dom, |data| data
                .push_all("other", (3..10).map(|t| NumberData::new(t, 0.0))))
            .0,
            0
        );
    }

    #[test]
    fn labels_change_type_and_go() {
        let mut dom = dom();
        write(&mut dom, |data| {
            data.push_all("temp", [NumberData::new(1, 1.0), NumberData::new(2, 2.0)]);
            data.push("message", StringData::new(1, "a".to_string()));
        });
        dom.in_runtime(|| {
            let data = DATA.with(|cell| cell.borrow().expect("grabbed"));
            assert_eq!(data.labels_of(DataType::Number), ["temp"]);
            assert_eq!(data.data_type_of("message"), Some(DataType::String));
        });

        assert_eq!(
            write(&mut dom, |data| data
                .push("temp", StringData::new(3, "x".to_string()))),
            (1, Some(0))
        );
        dom.in_runtime(|| {
            let data = DATA.with(|cell| cell.borrow().expect("grabbed"));
            assert_eq!(data.labels_of(DataType::String), ["message", "temp"]);
            data.with_each(Some(DataType::String), |each| {
                assert_eq!(
                    each.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
                    ["message", "temp"]
                );
            });
        });

        assert_eq!(
            write(&mut dom, |data| assert!(data.remove("temp"))),
            (1, None)
        );
        dom.in_runtime(|| {
            let mut data = DATA.with(|cell| cell.borrow().expect("grabbed"));
            assert!(!data.remove("temp"), "already gone");
        });
        assert_eq!(
            write(&mut dom, |data| data.push("temp", NumberData::new(4, 4.0))),
            (1, Some(1))
        );
        assert_eq!(write(&mut dom, |data| assert!(data.clear_all())), (1, None));
        dom.in_runtime(|| {
            let mut data = DATA.with(|cell| cell.borrow().expect("grabbed"));
            assert!(!data.clear_all(), "nothing left");
        });
    }
}
