use std::marker::PhantomData;

use dioxus::prelude::*;

use super::vector_arithmetic::vector_text;
use crate::data::{
    Conversion, ConversionInput, ConversionOutput, Data, DataContext, DataEntry, Endpoints,
    MapRunner, MapSettings, NumberData, Segment, SourceCursor, Vector, format_number, keep_taken,
};

#[derive(Clone, Copy, PartialEq)]
pub struct MagnitudeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
}

super::saved_settings!(MagnitudeSettings {
    from_label,
    to_label
});

/// The length of each vector of a label, at its time.
pub struct Magnitude<V> {
    settings: MagnitudeSettings,
    endpoints: Option<Endpoints>,
    cursor: SourceCursor,
    _vector: PhantomData<V>,
}

impl<V> Magnitude<V> {
    pub fn new() -> Self {
        Self {
            settings: MagnitudeSettings {
                from_label: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
            },
            endpoints: None,
            cursor: SourceCursor::default(),
            _vector: PhantomData,
        }
    }
}

impl<V> Default for Magnitude<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Vector> MapRunner for Magnitude<V>
where
    Data<V>: DataEntry,
{
    fn settings(&self) -> &dyn MapSettings {
        &self.settings
    }

    fn start(&mut self) {
        let MagnitudeSettings {
            from_label,
            to_label,
        } = self.settings;
        let endpoints = Endpoints::new(&from_label.peek(), &to_label.peek());
        if keep_taken(&mut self.endpoints, endpoints) {
            self.cursor.reset();
        }
    }

    fn run(&mut self, data: &mut DataContext, _timestamp: i64) -> Option<Conversion> {
        let Endpoints { from, to } = self.endpoints.as_ref()?;
        let Some(read) = self.cursor.new_entries::<Data<V>>(data, from) else {
            self.cursor.reset();
            return None;
        };

        let mut latest = None;
        for entry in &read.entries {
            let magnitude = entry.value().magnitude();
            data.push(to, NumberData::new(entry.timestamp(), magnitude));
            latest = Some((*entry.value(), magnitude));
        }
        let (vector, magnitude) = latest?;
        Some(Conversion {
            from: vec![ConversionInput::new(from, vector_text(&vector))],
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(to)],
                vec![Segment::from_input(format_number(magnitude))],
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::data::{Vector, Vector2d, Vector3d};

    #[test]
    fn magnitudes_are_lengths() {
        assert_eq!(Vector2d::new(3.0, -4.0).magnitude(), 5.0);
        assert_eq!(Vector3d::new(2.0, 3.0, 6.0).magnitude(), 7.0);
        assert_eq!(Vector3d::new(0.0, 0.0, 0.0).magnitude(), 0.0);
    }
}
