use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, DataContext, MapRunner, Segment, SourceCursor,
    StringData, endpoints, keep_taken,
};
use crate::helper::{decode_utf8, set_if_changed, unescape};

#[derive(Clone, Copy, PartialEq)]
pub struct DecodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    /// `\n`-style escapes allowed; empty to turn each entry into a string as it is
    pub delimiter: Signal<String>,
}

/// Turns a byte stream into UTF-8 strings: split at a delimiter, or one
/// string per entry without one.
pub struct Decode {
    settings: DecodeSettings,
    latest: Signal<Option<Conversion>>,
    /// The input and output labels and the delimiter (unescaped), as taken in
    /// when turned on
    taken: Option<(String, String, String)>,
    cursor: SourceCursor,
    /// The start of a character the previous entry ended in the middle of.
    pending: Vec<u8>,
    /// Text after the last delimiter, waiting for the rest of the line.
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
            latest: Signal::new(None),
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

    fn latest(&self) -> Signal<Option<Conversion>> {
        self.latest
    }

    fn start(&mut self) {
        let DecodeSettings {
            from_label,
            to_label,
            delimiter,
        } = self.settings;
        let taken = (
            from_label.peek().clone(),
            to_label.peek().clone(),
            unescape(&delimiter.peek()),
        );
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
            self.pending.clear();
            self.buffer.clear();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let Some((from, to, delimiter)) = self.taken.as_ref() else {
            return;
        };
        let Some((from, to)) = endpoints(from, to) else {
            return;
        };

        let Some(read) = self.cursor.new_entries::<ByteData>(data, from) else {
            self.cursor.reset();
            self.pending.clear();
            self.buffer.clear();
            return;
        };
        if read.restarted {
            // A partial line from the previous queue does not continue in this one
            self.pending.clear();
            self.buffer.clear();
        }
        let new_entries = read.entries;
        if new_entries.is_empty() {
            return;
        }

        let (pieces, from_value) = if delimiter.is_empty() {
            // Each entry as it is; what was buffered for a delimiter goes first
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
            // The bytes that made the line
            let from_value = pieces.last().map(|last| format!("{last}{delimiter}"));
            (pieces, from_value)
        };

        if let (Some(last), Some(from_value)) = (pieces.last(), from_value) {
            let conversion = Conversion {
                from: vec![ConversionInput::new(from, from_value)],
                to_label: vec![Segment::fixed(to)],
                to_value: vec![Segment::from_input(last.as_str())],
            };
            set_if_changed(&mut self.latest, Some(conversion));
        }
        for value in pieces {
            data.push(to, StringData::new(timestamp, value));
        }
    }
}

/// The pieces of `text` ended by `delimiter`, and the text after the last one.
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
