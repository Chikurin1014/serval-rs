//! Map settings as a layout file keeps them: by [`saved_settings!`], each
//! setting under its field's name.

use dioxus::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map as JsonMap, Value};

/// The settings of a map, kept in and taken from a layout file.
pub trait SavedSettings {
    /// Each setting by its name; not their errors, nor what the kind fixes.
    fn save(&self) -> Value;

    /// Sets those in `saved`, those missing left as they are; else what is
    /// wrong, with none set.
    fn load(&self, saved: &Value) -> Result<(), String>;
}

/// A setting: a signal, or one a kind may not have (a 2D vector's z).
pub trait SavedField {
    type Value: Serialize + DeserializeOwned;

    /// Whether the kind has it.
    fn is_present(&self) -> bool;

    /// Its value, if it has one.
    fn saved_value(&self) -> Option<Value>;

    /// `saved`'s value for it, unless it has none or that is missing.
    fn read_saved(
        &self,
        name: &str,
        saved: &JsonMap<String, Value>,
    ) -> Result<Option<Self::Value>, String> {
        match saved.get(name) {
            Some(value) if self.is_present() => serde_json::from_value(value.clone())
                .map(Some)
                .map_err(|error| format!("'{name}': {error}")),
            _ => Ok(None),
        }
    }

    /// Sets it to `value`, if any.
    fn set_saved(&self, value: Option<Self::Value>);
}

impl<T: Serialize + DeserializeOwned + 'static> SavedField for Signal<T> {
    type Value = T;

    fn is_present(&self) -> bool {
        true
    }

    fn saved_value(&self) -> Option<Value> {
        serde_json::to_value(&*self.peek()).ok()
    }

    fn set_saved(&self, value: Option<T>) {
        if let Some(value) = value {
            let mut signal = *self;
            signal.set(value);
        }
    }
}

impl<T: Serialize + DeserializeOwned + 'static> SavedField for Option<Signal<T>> {
    type Value = T;

    fn is_present(&self) -> bool {
        self.is_some()
    }

    fn saved_value(&self) -> Option<Value> {
        self.as_ref().and_then(SavedField::saved_value)
    }

    fn set_saved(&self, value: Option<T>) {
        if let Some(signal) = self {
            signal.set_saved(value);
        }
    }
}

/// Implements [`SavedSettings`] for `$settings`, of the fields listed.
macro_rules! saved_settings {
    ($settings:ty { $($field:ident),* $(,)? }) => {
        impl $crate::data::SavedSettings for $settings {
            fn save(&self) -> serde_json::Value {
                use $crate::data::SavedField;
                let mut saved = serde_json::Map::new();
                $(
                    if let Some(value) = self.$field.saved_value() {
                        saved.insert(stringify!($field).to_string(), value);
                    }
                )*
                saved.into()
            }

            fn load(&self, saved: &serde_json::Value) -> Result<(), String> {
                use $crate::data::SavedField;
                let saved = saved
                    .as_object()
                    .ok_or_else(|| "Its settings are not an object".to_string())?;
                // All read before any is set
                let ($($field,)*) = ($(self.$field.read_saved(stringify!($field), saved)?,)*);
                $(self.$field.set_saved($field);)*
                Ok(())
            }
        }
    };
}

pub(crate) use saved_settings;
