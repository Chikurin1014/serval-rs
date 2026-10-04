use std::any::Any;

use dioxus::prelude::*;

use crate::data::{ByteData, Converter, DataContext, SourceCursor, StringData};

#[derive(Clone, Copy, PartialEq)]
pub struct SplitFromByteSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    /// `\n`-style escapes allowed
    pub delimiter: Signal<String>,
}

/// Splits a byte stream into strings at a delimiter.
pub struct SplitFromByte {
    settings: SplitFromByteSettings,
    cursor: SourceCursor,
    /// Text after the last delimiter, waiting for the rest of the line.
    buffer: String,
}

impl SplitFromByte {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: SplitFromByteSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
                delimiter: Signal::new("\\n".to_string()),
            },
            cursor: SourceCursor::default(),
            buffer: String::new(),
        }
    }
}

impl Converter for SplitFromByte {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let SplitFromByteSettings {
            from_label,
            to_label,
            delimiter,
        } = self.settings;
        let (source, target) = (from_label(), to_label());
        let (source, target) = (source.trim(), target.trim());
        if source.is_empty() || target.is_empty() || source == target {
            return;
        }

        let Some(read) = self.cursor.new_entries::<ByteData>(data, source) else {
            self.cursor.reset();
            self.buffer.clear();
            return;
        };
        if read.restarted {
            // A partial line from the previous queue does not continue in this one
            self.buffer.clear();
        }
        let new_entries = read.entries;
        if new_entries.is_empty() {
            return;
        }

        let (pieces, buffer) =
            split_pending_bytes(&new_entries, &self.buffer, &decode_delimiter(&delimiter()));
        self.buffer = buffer;
        for value in pieces {
            data.push(target, StringData::new(timestamp, value));
        }
    }
}

fn decode_delimiter(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

fn split_pending_bytes(
    entries: &[ByteData],
    current: &str,
    delimiter: &str,
) -> (Vec<String>, String) {
    let mut buffer = current.to_string();
    let mut values = Vec::new();

    for entry in entries {
        buffer.push_str(&String::from_utf8_lossy(entry.value()));

        let mut parts = buffer
            .split(delimiter)
            .map(str::to_string)
            .collect::<Vec<_>>();

        if buffer.ends_with(delimiter) {
            buffer.clear();
            parts.pop();
            values.extend(parts);
        } else {
            let trailing = parts.pop().unwrap_or_default();
            buffer = trailing;
            values.extend(parts);
        }
    }

    (values, buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_delimiter_supports_escape_sequences() {
        assert_eq!(decode_delimiter("\\n"), "\n");
        assert_eq!(decode_delimiter("\\r\\n"), "\r\n");
    }

    #[test]
    fn split_pending_bytes_keeps_all_same_timestamp_chunks() {
        let entries = vec![
            ByteData::new(123, b"led:".to_vec()),
            ByteData::new(123, b" on\nled:".to_vec()),
            ByteData::new(123, b" off\n".to_vec()),
        ];

        let (values, remaining) = split_pending_bytes(&entries, "", "\n");
        assert_eq!(values, vec!["led: on", "led: off"]);
        assert_eq!(remaining, "");
    }
}
