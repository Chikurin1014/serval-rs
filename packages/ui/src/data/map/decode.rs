use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, DataContext, Endpoints, MapRunner, Segment,
    SourceCursor, StringData, keep_taken,
};
use crate::helper::{AnsiStripper, Delimiter, decode_utf8, split_lines};

#[derive(Clone, Copy, PartialEq)]
pub struct DecodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    /// At least one, in the order of [`Delimiter::ALL`].
    pub delimiters: Signal<Vec<Delimiter>>,
}

/// Turns a byte stream into UTF-8 lines, without ANSI escape sequences.
pub struct Decode {
    settings: DecodeSettings,
    taken: Option<(Endpoints, Vec<Delimiter>)>,
    cursor: SourceCursor,
    partial: Partial,
}

/// What is read but not yet in a line.
#[derive(Default)]
struct Partial {
    /// The start of a character cut off at the end of the previous entry.
    pending: Vec<u8>,
    stripper: AnsiStripper,
    /// Text after the last delimiter.
    buffer: String,
}

impl Decode {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: DecodeSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
                delimiters: Signal::new(Delimiter::ALL.to_vec()),
            },
            taken: None,
            cursor: SourceCursor::default(),
            partial: Partial::default(),
        }
    }
}

impl MapRunner for Decode {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let DecodeSettings {
            from_label,
            to_label,
            delimiters,
        } = self.settings;
        let taken = Endpoints::new(&from_label.peek(), &to_label.peek())
            .map(|endpoints| (endpoints, delimiters.peek().clone()));
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
            self.partial = Partial::default();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let (Endpoints { from, to }, delimiters) = self.taken.as_ref()?;
        let Some(read) = self.cursor.new_entries::<ByteData>(data, from) else {
            self.cursor.reset();
            self.partial = Partial::default();
            return None;
        };
        if read.restarted {
            self.partial = Partial::default();
        }
        if read.entries.is_empty() {
            return None;
        }

        let partial = &mut self.partial;
        let mut text = std::mem::take(&mut partial.buffer);
        for entry in &read.entries {
            let decoded = decode_utf8(&mut partial.pending, entry.value());
            text.push_str(&partial.stripper.strip(&decoded));
        }
        let (lines, rest) = split_lines(&text, delimiters);
        partial.buffer = rest;

        let conversion = lines.last().map(|(last, delimiter)| Conversion {
            from: vec![ConversionInput::new(
                from,
                format!("{last}{}", delimiter.text()),
            )],
            to_label: vec![Segment::fixed(to)],
            to_value: vec![Segment::from_input(last.as_str())],
        });
        for (line, _) in lines {
            data.push(to, StringData::new(timestamp, line));
        }
        conversion
    }
}
