//! Maps run on data as `MapProvider` runs them, for their tests.

use std::{any::Any, cell::RefCell};

use dioxus::{
    core::{Runtime, ScopeId},
    prelude::*,
    signals::Owner,
};

use crate::{
    data::{
        DataContext, DataEntry, DataProvider, Log, LogData, LogLevel, MapRunner, NumberData,
        NumberText,
    },
    helper::make_owned,
};

thread_local! {
    static DATA: RefCell<Option<DataContext>> = const { RefCell::new(None) };
}

#[component]
fn Grab() -> Element {
    let data = use_context::<DataContext>();
    DATA.with(|cell| *cell.borrow_mut() = Some(data));
    rsx! {}
}

/// The data, and maps that run again on it after each write, as long as any of
/// them gives anything.
pub(crate) struct Maps {
    dom: VirtualDom,
    data: DataContext,
    maps: Vec<Box<dyn MapRunner>>,
    timestamp: i64,
    _owners: Vec<Owner>,
}

impl Maps {
    pub(crate) fn new() -> Self {
        let mut dom = VirtualDom::new(|| rsx! { DataProvider { Grab {} } });
        dom.rebuild_in_place();
        let data = DATA.with(|cell| cell.borrow().expect("grabbed"));
        Self {
            dom,
            data,
            maps: Vec::new(),
            timestamp: 0,
            _owners: Vec::new(),
        }
    }

    /// Runs `f` where signals can be made, read and written.
    pub(crate) fn in_runtime<R>(&self, f: impl FnOnce() -> R) -> R {
        self.dom
            .in_runtime(|| Runtime::current().in_scope(ScopeId::ROOT, f))
    }

    /// Makes a map, sets it with `set` (given its settings) and turns it on; its
    /// settings, to read its errors by.
    pub(crate) fn add<M: MapRunner + 'static, S: Copy + 'static>(
        &mut self,
        make: impl FnOnce() -> M,
        set: impl FnOnce(S),
    ) -> S {
        let ((map, settings), owner) = self.in_runtime(|| {
            make_owned(ScopeId::ROOT, || {
                let mut map = make();
                let settings = *(map.settings() as &dyn Any)
                    .downcast_ref::<S>()
                    .expect("its settings");
                set(settings);
                map.start();
                (map, settings)
            })
        });
        self._owners.push(owner);
        self.maps.push(Box::new(map));
        self.run();
        settings
    }

    /// Appends `entry` to `label` and runs the maps on it.
    pub(crate) fn push<T: DataEntry>(&mut self, label: &str, entry: impl FnOnce(i64) -> T) {
        self.timestamp += 1_000;
        let (mut data, timestamp) = (self.data, self.timestamp);
        self.in_runtime(|| data.push(label, entry(timestamp)));
        self.run();
    }

    fn run(&mut self) {
        let (mut data, timestamp) = (self.data, self.timestamp);
        let maps = &mut self.maps;
        self.dom.in_runtime(|| {
            Runtime::current().in_scope(ScopeId::ROOT, || {
                // Each map once more for what another gave, as their effects would run
                for _ in 0..maps.len() {
                    let mut gave = false;
                    for map in maps.iter_mut() {
                        gave |= map.run(&mut data, timestamp).is_some();
                    }
                    if !gave {
                        break;
                    }
                }
            })
        });
    }

    /// Each value of `label`, oldest first, as the data list writes it.
    pub(crate) fn values(&self, label: &str) -> Vec<String> {
        self.in_runtime(|| {
            self.data
                .with_label(label, |data| {
                    data.newest_as_text(usize::MAX, NumberText::Rounded)
                        .into_iter()
                        .map(|(_, text)| text)
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    /// What `signal` holds, e.g. a map's error.
    pub(crate) fn read<T: Clone + 'static>(&self, signal: Signal<T>) -> T {
        self.in_runtime(|| signal.peek().clone())
    }
}

/// A log to [`Maps::push`].
pub(crate) fn log(level: LogLevel, message: &str) -> impl FnOnce(i64) -> LogData {
    let log = Log::new(level, message);
    move |timestamp| LogData::new(timestamp, log)
}

/// A number to [`Maps::push`].
pub(crate) fn number(value: f64) -> impl FnOnce(i64) -> NumberData {
    move |timestamp| NumberData::new(timestamp, value)
}
