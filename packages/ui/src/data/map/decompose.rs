use std::any::Any;
use std::marker::PhantomData;

use dioxus::prelude::*;

use super::vector_arithmetic::vector_text;
use crate::data::{
    Conversion, ConversionInput, ConversionOutput, Data, DataContext, DataEntry, MapRunner,
    NumberData, Segment, SourceCursor, Vector, format_number, keep_taken,
};

#[derive(Clone, Copy, PartialEq)]
pub struct DecomposeSettings {
    pub from_label: Signal<String>,
    /// The output label of x, y and (for a 3D vector) z: each left empty for
    /// none.
    pub x_label: Signal<String>,
    pub y_label: Signal<String>,
    pub z_label: Option<Signal<String>>,
}

/// The components of each vector of a label, at its time: each under its own
/// label, if it has one.
pub struct Decompose<V> {
    settings: DecomposeSettings,
    taken: Option<Taken>,
    cursor: SourceCursor,
    _vector: PhantomData<V>,
}

#[derive(PartialEq)]
struct Taken {
    from: String,
    /// Each component's output label, if any.
    to: Vec<Option<String>>,
}

impl<V> Decompose<V> {
    pub fn vector2d() -> Self {
        Self::new(None)
    }

    pub fn vector3d() -> Self {
        Self::new(Some(Signal::new(String::new())))
    }

    fn new(z_label: Option<Signal<String>>) -> Self {
        Self {
            settings: DecomposeSettings {
                from_label: Signal::new(String::new()),
                x_label: Signal::new(String::new()),
                y_label: Signal::new(String::new()),
                z_label,
            },
            taken: None,
            cursor: SourceCursor::default(),
            _vector: PhantomData,
        }
    }
}

impl<V: Vector> MapRunner for Decompose<V>
where
    Data<V>: DataEntry,
{
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let DecomposeSettings {
            from_label,
            x_label,
            y_label,
            z_label,
        } = self.settings;
        let from = from_label.peek().trim().to_string();
        let to: Vec<Option<String>> = [Some(x_label), Some(y_label), z_label]
            .into_iter()
            .flatten()
            .map(|label| Some(label.peek().trim().to_string()).filter(|label| !label.is_empty()))
            .collect();
        // At least one, and none its input
        let set = to.iter().flatten().count() > 0 && to.iter().flatten().all(|to| *to != from);
        let taken = (!from.is_empty() && set).then_some(Taken { from, to });
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
        }
    }

    fn run(&mut self, data: &mut DataContext, _timestamp: i64) -> Option<Conversion> {
        let Taken { from, to } = self.taken.as_ref()?;
        let Some(read) = self.cursor.new_entries::<Data<V>>(data, from) else {
            self.cursor.reset();
            return None;
        };

        for entry in &read.entries {
            for (label, &value) in to.iter().zip(entry.value().components()) {
                if let Some(label) = label {
                    data.push(label, NumberData::new(entry.timestamp(), value));
                }
            }
        }
        let vector = *read.entries.last()?.value();
        Some(Conversion {
            from: vec![ConversionInput::new(from, vector_text(&vector))],
            // Each component given, one above another
            to: to
                .iter()
                .zip(vector.components())
                .filter_map(|(label, &value)| {
                    Some(ConversionOutput::new(
                        vec![Segment::fixed(label.as_deref()?)],
                        vec![Segment::from_input(format_number(value))],
                    ))
                })
                .collect(),
        })
    }
}
