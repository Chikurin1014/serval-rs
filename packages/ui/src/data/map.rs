//! Maps turn data under one label into data under another
//!
//! To add a kind of map, implement [`MapRunner`] and pass a
//! [`MapKind`] for it, with its settings form, to
//! [`MapProvider`] (see `crate::elements::map::builtin_map_kinds`
//! for the built-in ones).

mod arithmetic;
mod calculus;
mod concat;
mod conversion;
mod decode;
mod encode;
mod input;
mod regex;
mod replace;

use std::{any::Any, cell::RefCell, rc::Rc};

use dioxus::{core::current_scope_id, prelude::*, signals::Owner};

use crate::{
    data::{DataContext, DataType, make_owned},
    time::TimeContext,
};

pub use arithmetic::{Arithmetic, ArithmeticSettings, Operation};
pub use calculus::{Calculus, CalculusMap, CalculusSettings};
pub use concat::{Concat, ConcatSettings};
pub(crate) use conversion::trim_segments;
pub use conversion::{Conversion, ConversionInput, Segment};
pub use decode::{Decode, DecodeSettings};
pub use encode::{Encode, EncodeSettings};
pub(crate) use input::{Input, endpoints, take_newest_pair};
pub use regex::{RegexMatch, RegexOutput, RegexSettings};
pub use replace::{Replace, ReplaceSettings};

/// The processing of one map.
///
/// Keep settings and errors in signals created in the constructor (which runs
/// with the map's own owner, see [`MapKind::create`]), so the
/// kind's form and [`MapRunner::run`] share them.
pub trait MapRunner {
    /// What [`MapKind::form`] edits, typically a `Copy` struct of signals.
    fn settings(&self) -> &dyn Any;

    /// The latest conversion, set by [`MapRunner::run`] (with
    /// [`set_if_changed`]) and shown in the map's card.
    fn latest(&self) -> Signal<Option<Conversion>>;

    /// Processes new input data while the map is enabled.
    ///
    /// Re-runs whenever anything reactive read here changes: the input data
    /// (read through `data`) and the settings signals.
    fn run(&mut self, data: &mut DataContext, timestamp: i64);
}

/// A kind of map that can be added from the map list.
#[derive(Clone, Copy, Debug)]
pub struct MapKind {
    pub name: &'static str,
    /// The type of each input, in order.
    pub from: &'static [DataType],
    pub to: DataType,
    /// Creates a map with default settings.
    pub create: fn() -> Box<dyn MapRunner>,
    /// The settings form shown in a map's card, given its [`MapRunner::settings`].
    pub form: fn(&dyn Any) -> Element,
    /// Ready-made settings offered beside the kind in the add menus.
    pub presets: &'static [MapPreset],
}

/// A kind of map with ready-made settings.
#[derive(Clone, Copy, Debug)]
pub struct MapPreset {
    pub name: &'static str,
    /// What sets it apart, shown beside its name (e.g. a pattern).
    pub detail: &'static str,
    /// Creates a map of its kind with these settings.
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
    /// Whether its card shows the settings form, kept here as the card is
    /// dropped when scrolled out of view.
    pub open: Signal<bool>,
    pub latest: Signal<Option<Conversion>>,
    runner: Rc<RefCell<Box<dyn MapRunner>>>,
    /// Owns the signals created by the runner; they are dropped with the map.
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
    /// `MapProvider`'s scope, an ancestor of everything that uses a map.
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

    /// Adds a map with default settings, its card open to set them.
    pub fn add(&mut self, kind: MapKind) -> usize {
        self.insert(kind, false, true, kind.create)
    }

    /// Adds a map of `kind` with `preset`'s settings, its card open.
    pub fn add_preset(&mut self, kind: MapKind, preset: MapPreset) -> usize {
        self.insert(kind, false, true, preset.create)
    }

    /// Adds a map built by `create`, e.g. one with preset settings.
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
        // Owned by the map (dropped with it), not by whichever component
        // handled the event that added it
        let ((runner, enabled, open), owner) = make_owned(self.scope, || {
            (create(), Signal::new(enabled), Signal::new(open))
        });
        let latest = runner.latest();
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

/// Provides [`MapContext`] and runs every enabled map,
/// independent of whether any map UI is mounted.
///
/// Requires `DataContext` and `TimeContext` to be provided by an ancestor.
#[component]
pub fn MapProvider(
    /// The kinds that can be added.
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

/// Kept apart from `MapProvider` so that adding or removing a
/// map does not re-render the provider's children.
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

/// Renders nothing; runs one map whenever it is enabled and anything it
/// reads changes.
#[component]
fn MapTask(id: usize) -> Element {
    let context = use_context::<MapContext>();
    let mut data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    // Compares by id, so this only changes when the map is removed
    let map = use_memo(move || context.get(id));

    use_effect(move || {
        let Some(map) = map() else {
            return;
        };
        if !(map.enabled)() {
            return;
        }
        map.runner
            .borrow_mut()
            .run(&mut data_context, time_context.current());
    });

    rsx! {}
}

/// Sets `signal` only if the value differs, so a map re-running does
/// not re-render forms showing an unchanged value.
pub fn set_if_changed<T: PartialEq + 'static>(signal: &mut Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}

/// A delimiter as typed in a form, with its `\n`, `\r`, `\t` and `\\` escapes
/// turned into the characters they stand for.
pub(crate) fn unescape(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::unescape;

    #[test]
    fn unescape_supports_escape_sequences() {
        assert_eq!(unescape("\\n"), "\n");
        assert_eq!(unescape("\\r\\n"), "\r\n");
        assert_eq!(unescape(""), "");
    }
}
