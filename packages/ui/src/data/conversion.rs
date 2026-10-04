//! Conversions turn data under one label into data under another
//!
//! To add a kind of conversion, implement [`Converter`] and pass a
//! [`ConversionKind`] for it, with its settings form, to
//! [`ConversionProvider`] (see `crate::elements::builtin_conversion_kinds`
//! for the built-in ones).

mod regex_match;
mod split_from_byte;

pub use regex_match::{RegexMatch, RegexMatchSettings, RegexOutput};
pub use split_from_byte::{SplitFromByte, SplitFromByteSettings};

use std::{any::Any, cell::RefCell, collections::VecDeque, rc::Rc};

use dioxus::{core::with_owner, prelude::*, signals::Owner};

use super::data_type::Data;
use crate::{
    data::{ByteData, DataContext, DataType, NumberData, StringData, TypedData},
    time::TimeContext,
};

/// The processing of one conversion.
///
/// Keep settings and errors in signals created in the constructor (which runs
/// with the conversion's own owner, see [`ConversionKind::create`]), so the
/// kind's form and [`Converter::run`] share them.
pub trait Converter {
    /// What [`ConversionKind::form`] edits, typically a `Copy` struct of signals.
    fn settings(&self) -> &dyn Any;

    /// Processes new source data while the conversion is enabled.
    ///
    /// Re-runs whenever anything reactive read here changes: the source data
    /// (read through `data`) and the settings signals.
    fn run(&mut self, data: &mut DataContext, timestamp: i64);
}

/// A kind of conversion that can be added from the conversion list.
#[derive(Clone, Copy, Debug)]
pub struct ConversionKind {
    pub name: &'static str,
    pub from: DataType,
    pub to: DataType,
    /// Creates a conversion with default settings.
    pub create: fn() -> Box<dyn Converter>,
    /// The settings form shown in a conversion's card, given its [`Converter::settings`].
    pub form: fn(&dyn Any) -> Element,
}

impl PartialEq for ConversionKind {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.from == other.from && self.to == other.to
    }
}

/// A conversion present from the start, e.g. one with preset settings.
#[derive(Clone, Copy, Debug)]
pub struct InitialConversion {
    pub kind: ConversionKind,
    pub enabled: bool,
    pub create: fn() -> Box<dyn Converter>,
}

impl PartialEq for InitialConversion {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.enabled == other.enabled
    }
}

#[derive(Clone)]
pub struct Conversion {
    pub id: usize,
    pub kind: ConversionKind,
    pub enabled: Signal<bool>,
    converter: Rc<RefCell<Box<dyn Converter>>>,
    /// Owns the signals created by the converter; they are dropped with the conversion.
    _owner: Owner,
}

impl PartialEq for Conversion {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Conversion {
    pub fn form(&self) -> Element {
        (self.kind.form)(self.converter.borrow().settings())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct ConversionContext {
    kinds: Signal<Vec<ConversionKind>>,
    list: Signal<Vec<Conversion>>,
    next_id: Signal<usize>,
}

impl ConversionContext {
    pub fn kinds(&self) -> Vec<ConversionKind> {
        self.kinds.read().clone()
    }

    pub fn list(&self) -> Vec<Conversion> {
        self.list.read().clone()
    }

    pub fn get(&self, id: usize) -> Option<Conversion> {
        self.list.read().iter().find(|c| c.id == id).cloned()
    }

    pub fn add(&mut self, kind: ConversionKind) -> usize {
        self.push(kind, false, kind.create)
    }

    /// Adds a conversion built by `create`, e.g. one with preset settings.
    pub fn push(
        &mut self,
        kind: ConversionKind,
        enabled: bool,
        create: impl FnOnce() -> Box<dyn Converter>,
    ) -> usize {
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        // Signals made here belong to the conversion, not to whichever
        // component happened to handle the event that added it
        let owner = Owner::default();
        let (converter, enabled) = with_owner(owner.clone(), || (create(), Signal::new(enabled)));
        self.list.write().push(Conversion {
            id,
            kind,
            enabled,
            converter: Rc::new(RefCell::new(converter)),
            _owner: owner,
        });
        id
    }

    pub fn remove(&mut self, id: usize) {
        self.list.write().retain(|c| c.id != id);
    }
}

/// Provides [`ConversionContext`] and runs every enabled conversion,
/// independent of whether any conversion UI is mounted.
///
/// Requires `DataContext` and `TimeContext` to be provided by an ancestor.
#[component]
pub fn ConversionProvider(
    /// The kinds that can be added.
    kinds: Vec<ConversionKind>,
    #[props(default)] initial: Vec<InitialConversion>,
    children: Element,
) -> Element {
    use_context_provider(|| {
        let mut context = ConversionContext {
            kinds: Signal::new(kinds),
            list: Signal::new(Vec::new()),
            next_id: Signal::new(0),
        };
        for conversion in initial {
            context.push(conversion.kind, conversion.enabled, conversion.create);
        }
        context
    });

    rsx! {
        ConversionRunners {}
        {children}
    }
}

/// Kept apart from `ConversionProvider` so that adding or removing a
/// conversion does not re-render the provider's children.
#[component]
fn ConversionRunners() -> Element {
    let context = use_context::<ConversionContext>();
    let ids = use_memo(move || context.list.read().iter().map(|c| c.id).collect::<Vec<_>>());

    rsx! {
        for id in ids() {
            ConversionRunner { key: "{id}", id }
        }
    }
}

/// Renders nothing; runs one conversion whenever it is enabled and anything it
/// reads changes.
#[component]
fn ConversionRunner(id: usize) -> Element {
    let context = use_context::<ConversionContext>();
    let mut data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    // Compares by id, so this only changes when the conversion is removed
    let conversion = use_memo(move || context.get(id));

    use_effect(move || {
        let Some(conversion) = conversion() else {
            return;
        };
        if !(conversion.enabled)() {
            return;
        }
        conversion
            .converter
            .borrow_mut()
            .run(&mut data_context, time_context.current());
    });

    rsx! {}
}

/// Tracks how far a converter has read a source queue, so each run only sees
/// what was added since the previous one.
#[derive(Default)]
pub struct SourceCursor {
    label: String,
    processed: usize,
}

impl SourceCursor {
    /// Starts over from the beginning of the queue on the next read.
    pub fn reset(&mut self) {
        self.processed = 0;
    }

    /// Bytes added under `label` since the last call, or `None` if `label`
    /// holds no bytes.
    pub fn new_bytes(&mut self, data: &DataContext, label: &str) -> Option<Vec<ByteData>> {
        self.take(data, label, |data| match data {
            TypedData::Bytes(queue) => Some(queue),
            _ => None,
        })
    }

    /// Strings added under `label` since the last call, or `None` if `label`
    /// holds no strings.
    pub fn new_strings(&mut self, data: &DataContext, label: &str) -> Option<Vec<StringData>> {
        self.take(data, label, |data| match data {
            TypedData::String(queue) => Some(queue),
            _ => None,
        })
    }

    /// Numbers added under `label` since the last call, or `None` if `label`
    /// holds no numbers.
    pub fn new_numbers(&mut self, data: &DataContext, label: &str) -> Option<Vec<NumberData>> {
        self.take(data, label, |data| match data {
            TypedData::Number(queue) => Some(queue),
            _ => None,
        })
    }

    fn take<T: Clone>(
        &mut self,
        data: &DataContext,
        label: &str,
        queue_of: impl FnOnce(&TypedData) -> Option<&VecDeque<Data<T>>>,
    ) -> Option<Vec<Data<T>>> {
        if self.label != label {
            self.label = label.to_string();
            self.processed = 0;
        }
        data.with_data(|data| {
            let queue = data.get(label).and_then(queue_of)?;
            // Queues only grow at the back; a shorter one was cleared
            if self.processed > queue.len() {
                self.processed = 0;
            }
            let entries = queue.iter().skip(self.processed).cloned().collect();
            self.processed = queue.len();
            Some(entries)
        })
    }
}

/// Sets `signal` only if the value differs, so a conversion re-running does
/// not re-render forms showing an unchanged value.
pub fn set_if_changed<T: PartialEq + 'static>(signal: &mut Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}
