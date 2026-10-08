use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, DataContext, Endpoints, MapRunner, Segment,
    SourceCursor, StringData, keep_taken,
};

#[derive(Clone, Copy, PartialEq)]
pub struct EncodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
}

/// Turns each string into its UTF-8 bytes.
pub struct Encode {
    settings: EncodeSettings,
    endpoints: Option<Endpoints>,
    cursor: SourceCursor,
}

impl Encode {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: EncodeSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
            },
            endpoints: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl MapRunner for Encode {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let EncodeSettings {
            from_label,
            to_label,
        } = self.settings;
        let endpoints = Endpoints::new(&from_label.peek(), &to_label.peek());
        if keep_taken(&mut self.endpoints, endpoints) {
            self.cursor.reset();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let Endpoints { from, to } = self.endpoints.as_ref()?;
        let Some(read) = self.cursor.new_entries::<StringData>(data, from) else {
            self.cursor.reset();
            return None;
        };
        let last = read.entries.last()?;

        let conversion = Conversion {
            from: vec![ConversionInput::new(from, last.value().as_str())],
            to_label: vec![Segment::fixed(to)],
            to_value: vec![Segment::from_input(last.value().as_str())],
        };
        for entry in &read.entries {
            data.push(
                to,
                ByteData::new(timestamp, entry.value().as_bytes().to_vec()),
            );
        }
        Some(conversion)
    }
}
