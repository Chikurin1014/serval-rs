use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, DataContext, MapRunner, Segment, SourceCursor,
    StringData, endpoints, keep_taken,
};
use crate::helper::set_if_changed;

#[derive(Clone, Copy, PartialEq)]
pub struct EncodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
}

/// Turns each string into its UTF-8 bytes: the reverse of [`Decode`](super::Decode)
/// without a delimiter.
pub struct Encode {
    settings: EncodeSettings,
    latest: Signal<Option<Conversion>>,
    /// The input and output labels, as taken in when turned on
    taken: Option<(String, String)>,
    cursor: SourceCursor,
}

impl Encode {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: EncodeSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
            },
            latest: Signal::new(None),
            taken: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl MapRunner for Encode {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn latest(&self) -> Signal<Option<Conversion>> {
        self.latest
    }

    fn start(&mut self) {
        let EncodeSettings {
            from_label,
            to_label,
        } = self.settings;
        let taken = (from_label.peek().clone(), to_label.peek().clone());
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let Some((from, to)) = self
            .taken
            .as_ref()
            .and_then(|(from, to)| endpoints(from, to))
        else {
            return;
        };

        let Some(read) = self.cursor.new_entries::<StringData>(data, from) else {
            self.cursor.reset();
            return;
        };
        let Some(last) = read.entries.last() else {
            return;
        };

        let conversion = Conversion {
            from: vec![ConversionInput::new(from, last.value().as_str())],
            to_label: vec![Segment::fixed(to)],
            to_value: vec![Segment::from_input(last.value().as_str())],
        };
        set_if_changed(&mut self.latest, Some(conversion));
        for entry in &read.entries {
            data.push(
                to,
                ByteData::new(timestamp, entry.value().as_bytes().to_vec()),
            );
        }
    }
}
