//! Maps turn data under one label into data under another. To add a kind,
//! implement [`MapRunner`] and pass a [`MapKind`] for it to [`MapProvider`].

mod arithmetic;
mod calculus;
mod compose;
mod concat;
mod conversion;
mod decode;
mod decompose;
mod encode;
mod input;
mod magnitude;
mod matching;
mod replace;
mod saved;
mod vector_arithmetic;

#[cfg(test)]
pub(crate) mod testing;

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
pub use calculus::{Calculus, CalculusMap, CalculusSettings, VectorCalculus};
pub use compose::{Compose, ComposeSettings};
pub use concat::{Concat, ConcatSettings, MORE_LEVEL};
pub(crate) use conversion::trim_segments;
pub use conversion::{Conversion, ConversionInput, ConversionOutput, Segment};
pub use decode::{Decode, DecodeSettings};
pub use decompose::{Decompose, DecomposeSettings};
pub use encode::{Encode, EncodeSettings};
pub(crate) use input::{Endpoints, Input, Operand, endpoints, take_newest_all, take_newest_pair};
pub use magnitude::{Magnitude, MagnitudeSettings};
pub use matching::{Match, MatchOutput, MatchSettings};
pub use replace::{Replace, ReplaceSettings};
pub(crate) use saved::saved_settings;
pub use saved::{SavedField, SavedSettings};
pub use vector_arithmetic::{VectorArithmetic, VectorArithmeticSettings, VectorOperation};

/// The processing of one map. Its settings are signals made in its
/// constructor, shared with its form.
pub trait MapRunner {
    /// What [`MapKind::form`] edits.
    fn settings(&self) -> &dyn Any;

    /// Its settings as a layout file keeps them.
    fn saved(&self) -> &dyn SavedSettings;

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

/// A map present from the start, or taken from a layout file.
#[derive(Clone, Debug, PartialEq)]
pub struct InitialMap {
    pub kind: MapKind,
    pub enabled: bool,
    /// As [`SavedSettings::save`] gives them.
    pub settings: serde_json::Value,
}

#[derive(Clone)]
pub struct Map {
    pub id: usize,
    pub kind: MapKind,
    pub enabled: Signal<bool>,
    /// Whether its card is open; kept here as the cards are made anew when maps
    /// are added or removed.
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
        self.insert(kind, kind.create)
    }

    pub fn add_preset(&mut self, kind: MapKind, preset: MapPreset) -> usize {
        self.insert(kind, preset.create)
    }

    /// Adds a map, off, its card open.
    fn insert(&mut self, kind: MapKind, create: impl FnOnce() -> Box<dyn MapRunner>) -> usize {
        let Ok(map) = self.make(kind, false, true, || {
            Ok::<_, std::convert::Infallible>(create())
        });
        let id = map.id;
        self.list.write().push(map);
        id
    }

    fn make<E>(
        &mut self,
        kind: MapKind,
        enabled: bool,
        open: bool,
        create: impl FnOnce() -> Result<Box<dyn MapRunner>, E>,
    ) -> Result<Map, E> {
        let (made, owner) = make_owned(self.scope, || {
            Ok((
                create()?,
                Signal::new(enabled),
                Signal::new(open),
                Signal::new(None),
            ))
        });
        // Its signals dropped with the owner if not made
        let (runner, enabled, open, latest) = made?;
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        Ok(Map {
            id,
            kind,
            enabled,
            open,
            latest,
            runner: Rc::new(RefCell::new(runner)),
            _owner: owner,
        })
    }

    /// Its settings as a layout file keeps them.
    pub fn saved_settings(&self, id: usize) -> Option<serde_json::Value> {
        let map = self.list.peek().iter().find(|map| map.id == id).cloned()?;
        let saved = map.runner.borrow().saved().save();
        Some(saved)
    }

    /// Puts `maps` in place of all there are; else what is wrong with one, with
    /// none changed.
    pub fn replace_all(&mut self, maps: Vec<InitialMap>) -> Result<(), String> {
        let made = maps
            .into_iter()
            .map(
                |InitialMap {
                     kind,
                     enabled,
                     settings,
                 }| {
                    self.make(kind, enabled, false, || {
                        let runner = (kind.create)();
                        runner
                            .saved()
                            .load(&settings)
                            .map_err(|error| format!("{}: {error}", kind.name))?;
                        Ok(runner)
                    })
                },
            )
            .collect::<Result<Vec<_>, String>>()?;
        self.list.set(made);
        Ok(())
    }

    pub fn remove(&mut self, id: usize) {
        self.list.write().retain(|c| c.id != id);
    }

    /// Puts the maps in the order of `ids`; whether they are all of them (if
    /// not, they stay as they are).
    pub fn reorder(&mut self, ids: &[usize]) -> bool {
        let list = self.list.peek();
        if list.len() != ids.len() {
            return false;
        }
        if list.iter().map(|map| map.id).eq(ids.iter().copied()) {
            return true;
        }
        let Some(reordered) = ids
            .iter()
            .map(|&id| list.iter().find(|map| map.id == id).cloned())
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        drop(list);
        self.list.set(reordered);
        true
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
        if let Err(error) = context.replace_all(initial) {
            panic!("the initial maps are wrong: {error}");
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
