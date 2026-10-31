use dioxus::prelude::*;
use fancy_regex::Regex;

use super::matching::{expand, label_segments, log_of, log_segments, text};
use crate::data::{
    ByteData, Conversion, ConversionInput, ConversionOutput, DataContext, Endpoints, LogData,
    MapRunner, MapSettings, SourceCursor, compile_for_map, keep_taken,
};
use crate::helper::{
    AnsiStripper, Choice, Delimiter, decode_utf8, hex, set_if_changed, split_lines,
};

/// What matches a whole line, for its message.
pub const WHOLE_LINE: &str = ".*";

#[derive(Clone, Copy, PartialEq)]
pub struct DecodeSettings {
    pub from_label: Signal<String>,
    /// At least one, in the order of [`Delimiter::ALL`].
    pub delimiters: Signal<Vec<Delimiter>>,
    /// What each line must match, as a Match map to a log.
    pub pattern: Signal<String>,
    /// May use the pattern's groups, as the level and message.
    pub to_label: Signal<String>,
    /// A level's name; nothing for None.
    pub level: Signal<String>,
    pub message: Signal<String>,
    pub pattern_error: Signal<Option<String>>,
    pub replacement_error: Signal<Option<String>>,
}

super::saved_settings!(DecodeSettings {
    from_label,
    delimiters,
    pattern,
    to_label,
    level,
    message
});

/// Turns a byte stream into UTF-8 lines, without ANSI escape sequences: each
/// that matches a pattern into a log, as a Match map does.
pub struct Decode {
    settings: DecodeSettings,
    taken: Option<Taken>,
    /// The pattern compiled, if it is valid.
    regex: Option<Regex>,
    cursor: SourceCursor,
    partial: Partial,
}

#[derive(PartialEq)]
struct Taken {
    endpoints: Endpoints,
    delimiters: Vec<Delimiter>,
    pattern: String,
    level: String,
    message: String,
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
    /// Each line whole as a log message, of no level.
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self::with(from_label, WHOLE_LINE, to_label, "", "$0")
    }

    pub fn with(
        from_label: &str,
        pattern: &str,
        to_label: &str,
        level: &str,
        message: &str,
    ) -> Self {
        Self {
            settings: DecodeSettings {
                from_label: Signal::new(from_label.to_string()),
                delimiters: Signal::new(Delimiter::ALL.to_vec()),
                pattern: Signal::new(pattern.to_string()),
                to_label: Signal::new(to_label.to_string()),
                level: Signal::new(level.to_string()),
                message: Signal::new(message.to_string()),
                pattern_error: Signal::new(None),
                replacement_error: Signal::new(None),
            },
            taken: None,
            regex: None,
            cursor: SourceCursor::default(),
            partial: Partial::default(),
        }
    }
}

impl MapRunner for Decode {
    fn settings(&self) -> &dyn MapSettings {
        &self.settings
    }

    fn start(&mut self) {
        let DecodeSettings {
            from_label,
            delimiters,
            pattern,
            to_label,
            level,
            message,
            mut pattern_error,
            mut replacement_error,
        } = self.settings;
        // Even with labels missing, to report pattern errors
        let compiled = compile_for_map(&pattern.peek(), &mut pattern_error);
        let taken = Endpoints::new(&from_label.peek(), &to_label.peek())
            .filter(|_| compiled.is_some())
            .map(|endpoints| Taken {
                endpoints,
                delimiters: delimiters.peek().clone(),
                pattern: pattern.peek().clone(),
                level: level.peek().clone(),
                message: message.peek().clone(),
            });
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
            self.partial = Partial::default();
        }
        self.regex = compiled;
        set_if_changed(&mut replacement_error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let DecodeSettings {
            mut pattern_error,
            mut replacement_error,
            ..
        } = self.settings;
        let Taken {
            endpoints: Endpoints { from, to },
            delimiters,
            level,
            message,
            ..
        } = self.taken.as_ref()?;
        let regex = self.regex.as_ref()?;
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
        let mut text_read = std::mem::take(&mut partial.buffer);
        for entry in &read.entries {
            let decoded = decode_utf8(&mut partial.pending, entry.value());
            text_read.push_str(&partial.stripper.strip(&decoded));
        }
        let (lines, rest) = split_lines(&text_read, delimiters);
        partial.buffer = rest;

        let mut error = None;
        let mut failure = None;
        let mut latest = None;
        for (line, delimiter) in &lines {
            let captures = match regex.captures(line.as_str()) {
                Ok(Some(captures)) => captures,
                Ok(None) => continue,
                Err(failed) => {
                    failure = Some(format!("Matching failed: {failed}"));
                    continue;
                }
            };
            let label = text(&label_segments(&captures, to));
            if label.is_empty() {
                continue;
            }
            match log_of(&expand(&captures, level), &expand(&captures, message)) {
                Ok(log) => {
                    data.push(&label, LogData::new(timestamp, log));
                    latest = Some((line, delimiter, captures));
                }
                Err(message) => error = Some(message),
            }
        }
        set_if_changed(&mut pattern_error, failure);
        set_if_changed(&mut replacement_error, error);

        let (line, delimiter, captures) = latest?;
        Some(Conversion {
            // Bytes in hex, as they are shown
            from: vec![ConversionInput::new(
                from,
                hex(format!("{line}{}", delimiter.text()).as_bytes()),
            )],
            to: vec![ConversionOutput::new(
                label_segments(&captures, to),
                log_segments(&captures, level, message),
            )],
        })
    }
}
