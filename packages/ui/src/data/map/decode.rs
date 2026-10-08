use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, DataContext, Endpoints, MapRunner, Segment,
    SourceCursor, StringData, keep_taken,
};
use crate::helper::{decode_utf8, unescape};

#[derive(Clone, Copy, PartialEq)]
pub struct DecodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    /// With `\n`-style escapes; empty for one string per entry.
    pub delimiter: Signal<String>,
}

/// Turns a byte stream into UTF-8 strings, split at a delimiter.
pub struct Decode {
    settings: DecodeSettings,
    taken: Option<(Endpoints, String)>,
    cursor: SourceCursor,
    /// The start of a character cut off at the end of the previous entry.
    pending: Vec<u8>,
    /// Text after the last delimiter.
    buffer: String,
}

impl Decode {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: DecodeSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
                delimiter: Signal::new("\\n".to_string()),
            },
            taken: None,
            cursor: SourceCursor::default(),
            pending: Vec::new(),
            buffer: String::new(),
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
            delimiter,
        } = self.settings;
        let taken = Endpoints::new(&from_label.peek(), &to_label.peek())
            .map(|endpoints| (endpoints, unescape(&delimiter.peek())));
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
            self.pending.clear();
            self.buffer.clear();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let (Endpoints { from, to }, delimiter) = self.taken.as_ref()?;
        let Some(read) = self.cursor.new_entries::<ByteData>(data, from) else {
            self.cursor.reset();
            self.pending.clear();
            self.buffer.clear();
            return None;
        };
        if read.restarted {
            self.pending.clear();
            self.buffer.clear();
        }
        let new_entries = read.entries;
        if new_entries.is_empty() {
            return None;
        }

        let (pieces, from_value) = if delimiter.is_empty() {
            let mut pieces = Vec::new();
            if !self.buffer.is_empty() {
                pieces.push(std::mem::take(&mut self.buffer));
            }
            for entry in &new_entries {
                let text = decode_utf8(&mut self.pending, entry.value());
                if !text.is_empty() {
                    pieces.push(text);
                }
            }
            let from_value = pieces.last().cloned();
            (pieces, from_value)
        } else {
            let mut text = std::mem::take(&mut self.buffer);
            for entry in &new_entries {
                text.push_str(&decode_utf8(&mut self.pending, entry.value()));
            }
            let (pieces, buffer) = split_complete(&text, delimiter);
            self.buffer = buffer;
            let from_value = pieces.last().map(|last| format!("{last}{delimiter}"));
            (pieces, from_value)
        };

        let conversion = match (pieces.last(), from_value) {
            (Some(last), Some(from_value)) => Some(Conversion {
                from: vec![ConversionInput::new(from, from_value)],
                to_label: vec![Segment::fixed(to)],
                to_value: vec![Segment::from_input(last.as_str())],
            }),
            _ => None,
        };
        for value in pieces {
            data.push(to, StringData::new(timestamp, value));
        }
        conversion
    }
}

/// The pieces of `text` ended by `delimiter`, and the rest.
fn split_complete(text: &str, delimiter: &str) -> (Vec<String>, String) {
    let mut pieces = text
        .split(delimiter)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let rest = pieces.pop().unwrap_or_default();
    (pieces, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_complete_keeps_the_unfinished_line() {
        let (pieces, rest) = split_complete("led: on\nled: off\nled", "\n");
        assert_eq!(pieces, vec!["led: on", "led: off"]);
        assert_eq!(rest, "led");

        let (pieces, rest) = split_complete("a\r\nb\r\n", "\r\n");
        assert_eq!(pieces, vec!["a", "b"]);
        assert_eq!(rest, "");
    }
}
