//! Maps turn data under one label into data under another. To add a kind,
//! implement [`MapRunner`] and pass a [`MapKind`] for it to [`MapProvider`].

mod arithmetic;
mod calculus;
mod concat;
mod conversion;
mod decode;
mod encode;
mod input;
mod regex;
mod replace;

use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

use dioxus::{core::current_scope_id, prelude::*, signals::Owner};

use crate::{
    data::{DataContext, DataType},
    helper::{make_owned, set_if_changed},
    time::TimeContext,
};

pub use arithmetic::{Arithmetic, ArithmeticSettings, Operation};
pub use calculus::{Calculus, CalculusMap, CalculusSettings};
pub use concat::{Concat, ConcatSettings};
pub(crate) use conversion::trim_segments;
pub use conversion::{Conversion, ConversionInput, Segment};
pub use decode::{Decode, DecodeSettings, Delimiter};
pub use encode::{Encode, EncodeSettings};
pub(crate) use input::{Endpoints, Input, endpoints, take_newest_pair};
pub use regex::{RegexMatch, RegexOutput, RegexSettings};
pub use replace::{Replace, ReplaceSettings};

/// The processing of one map. Its settings are signals made in its
/// constructor, shared with its form.
pub trait MapRunner {
    /// What [`MapKind::form`] edits.
    fn settings(&self) -> &dyn Any;

    /// Takes in and checks the settings as the map is turned on (they are locked
    /// while it is on). If they changed since, the input is read from the start.
    fn start(&mut self);

    /// Processes new input; gives the latest conversion made, if any. Runs again
    /// whenever the input it reads changes.
    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion>;
}

/// A kind of map that can be added from the map list.
#[derive(Clone, Copy, Debug)]
pub struct MapKind {
    pub name: &'static str,
    pub from: &'static [DataType],
    pub to: DataType,
    pub create: fn() -> Box<dyn MapRunner>,
    pub form: fn(&dyn Any) -> Element,
    pub presets: &'static [MapPreset],
}

/// A kind of map with ready-made settings.
#[derive(Clone, Copy, Debug)]
pub struct MapPreset {
    pub name: &'static str,
    /// Shown beside its name, e.g. a pattern.
    pub detail: &'static str,
    pub create: fn() -> Box<dyn MapRunner>,
}

impl PartialEq for MapKind {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.from == other.from && self.to == other.to
    }
}

/// A map present from the start, e.g. one with preset settings.
#[derive(Clone, Copy, Debug)]
pub struct InitialMap {
    pub kind: MapKind,
    pub enabled: bool,
    pub create: fn() -> Box<dyn MapRunner>,
}

impl PartialEq for InitialMap {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.enabled == other.enabled
    }
}

#[derive(Clone)]
pub struct Map {
    pub id: usize,
    pub kind: MapKind,
    pub enabled: Signal<bool>,
    /// Whether its card is open; kept here as cards scrolled out are dropped.
    pub open: Signal<bool>,
    pub latest: Signal<Option<Conversion>>,
    runner: Rc<RefCell<Box<dyn MapRunner>>>,
    _owner: Owner,
}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Map {
    pub fn form(&self) -> Element {
        (self.kind.form)(self.runner.borrow().settings())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct MapContext {
    kinds: Signal<Vec<MapKind>>,
    list: Signal<Vec<Map>>,
    next_id: Signal<usize>,
    scope: ScopeId,
}

impl MapContext {
    pub fn kinds(&self) -> Vec<MapKind> {
        self.kinds.read().clone()
    }

    pub fn list(&self) -> Vec<Map> {
        self.list.read().clone()
    }

    pub fn get(&self, id: usize) -> Option<Map> {
        self.list.read().iter().find(|c| c.id == id).cloned()
    }

    pub fn add(&mut self, kind: MapKind) -> usize {
        self.insert(kind, false, true, kind.create)
    }

    pub fn add_preset(&mut self, kind: MapKind, preset: MapPreset) -> usize {
        self.insert(kind, false, true, preset.create)
    }

    pub fn push(
        &mut self,
        kind: MapKind,
        enabled: bool,
        create: impl FnOnce() -> Box<dyn MapRunner>,
    ) -> usize {
        self.insert(kind, enabled, false, create)
    }

    fn insert(
        &mut self,
        kind: MapKind,
        enabled: bool,
        open: bool,
        create: impl FnOnce() -> Box<dyn MapRunner>,
    ) -> usize {
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        let ((runner, enabled, open, latest), owner) = make_owned(self.scope, || {
            (
                create(),
                Signal::new(enabled),
                Signal::new(open),
                Signal::new(None),
            )
        });
        self.list.write().push(Map {
            id,
            kind,
            enabled,
            open,
            latest,
            runner: Rc::new(RefCell::new(runner)),
            _owner: owner,
        });
        id
    }

    pub fn remove(&mut self, id: usize) {
        self.list.write().retain(|c| c.id != id);
    }
}

/// Provides [`MapContext`] and runs the enabled maps. Requires `DataContext`
/// and `TimeContext`.
#[component]
pub fn MapProvider(
    kinds: Vec<MapKind>,
    #[props(default)] initial: Vec<InitialMap>,
    children: Element,
) -> Element {
    use_context_provider(|| {
        let mut context = MapContext {
            kinds: Signal::new(kinds),
            list: Signal::new(Vec::new()),
            next_id: Signal::new(0),
            scope: current_scope_id(),
        };
        for map in initial {
            context.push(map.kind, map.enabled, map.create);
        }
        context
    });

    rsx! {
        MapTasks {}
        {children}
    }
}

/// Apart from `MapProvider`, so adding a map does not re-render its children.
#[component]
fn MapTasks() -> Element {
    let context = use_context::<MapContext>();
    let ids = use_memo(move || context.list.read().iter().map(|c| c.id).collect::<Vec<_>>());

    rsx! {
        for id in ids() {
            MapTask { key: "{id}", id }
        }
    }
}

#[component]
fn MapTask(id: usize) -> Element {
    let context = use_context::<MapContext>();
    let mut data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    let map = use_memo(move || context.get(id));
    let was_enabled = use_hook(|| Rc::new(Cell::new(false)));

    use_effect(move || {
        let Some(map) = map() else {
            return;
        };
        let enabled = (map.enabled)();
        if !was_enabled.replace(enabled) && enabled {
            map.runner.borrow_mut().start();
        }
        if !enabled {
            return;
        }
        let latest = map
            .runner
            .borrow_mut()
            .run(&mut data_context, time_context.current());
        if let Some(conversion) = latest {
            let mut shown = map.latest;
            set_if_changed(&mut shown, Some(conversion));
        }
    });

    rsx! {}
}

/// Stores `taken` in `kept`; whether it changed.
pub(crate) fn keep_taken<S: PartialEq>(kept: &mut S, taken: S) -> bool {
    let changed = *kept != taken;
    *kept = taken;
    changed
}
