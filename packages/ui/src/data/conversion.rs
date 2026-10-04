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

use std::{any::Any, cell::RefCell, rc::Rc};

use dioxus::{
    core::{Runtime, current_scope_id, with_owner},
    prelude::*,
    signals::Owner,
};

use crate::{
    data::{DataContext, DataType},
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
    /// `ConversionProvider`'s scope, an ancestor of everything that uses a conversion.
    scope: ScopeId,
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
        // Signals made here are owned by the conversion (dropped with it), not by
        // whichever component handled the event that added it. They are made
        // in the provider's scope, so Dioxus sees them used only below where
        // they were made (by runners and forms), and does not warn
        let owner = Owner::default();
        let (converter, enabled) = Runtime::current().in_scope(self.scope, || {
            with_owner(owner.clone(), || (create(), Signal::new(enabled)))
        });
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
            scope: current_scope_id(),
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

/// Sets `signal` only if the value differs, so a conversion re-running does
/// not re-render forms showing an unchanged value.
pub fn set_if_changed<T: PartialEq + 'static>(signal: &mut Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}
