use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, DataContext, Endpoints, MapRunner, Segment,
    SourceCursor, StringData, keep_taken,
};
use crate::helper::{AnsiStripper, decode_utf8};

/// A line end a [`Decode`] splits at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Delimiter {
    Cr,
    Lf,
    CrLf,
}

impl Delimiter {
    pub const ALL: [Self; 3] = [Self::Cr, Self::Lf, Self::CrLf];

    pub fn text(self) -> &'static str {
        match self {
            Self::Cr => "\r",
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

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

/// The lines of `text` ended by one of `delimiters`, each with its end, and the
/// rest. A last CR waits for what follows while CRLF is one of them.
fn split_lines(text: &str, delimiters: &[Delimiter]) -> (Vec<(String, Delimiter)>, String) {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        let end = match c {
            '\r' if delimiters.contains(&Delimiter::CrLf) => match chars.peek() {
                Some((_, '\n')) => {
                    chars.next();
                    Some(Delimiter::CrLf)
                }
                None => break,
                Some(_) => delimiters.contains(&Delimiter::Cr).then_some(Delimiter::Cr),
            },
            '\r' if delimiters.contains(&Delimiter::Cr) => Some(Delimiter::Cr),
            '\n' if delimiters.contains(&Delimiter::Lf) => Some(Delimiter::Lf),
            _ => None,
        };
        if let Some(end) = end {
            lines.push((text[start..index].to_string(), end));
            start = index + end.text().len();
        }
    }
    (lines, text[start..].to_string())
}

#[cfg(test)]
mod tests {
    use super::{Delimiter::*, *};

    fn line(text: &str, end: Delimiter) -> (String, Delimiter) {
        (text.to_string(), end)
    }

    #[test]
    fn split_lines_keeps_the_unfinished_line() {
        assert_eq!(
            split_lines("led: on\nled: off\nled", &[Lf]),
            (
                vec![line("led: on", Lf), line("led: off", Lf)],
                "led".to_string()
            )
        );
    }

    #[test]
    fn split_lines_splits_at_any_of_the_delimiters() {
        assert_eq!(
            split_lines("a\rb\nc\r\nd", &[Cr, Lf]),
            (
                vec![line("a", Cr), line("b", Lf), line("c", Cr), line("", Lf)],
                "d".to_string()
            )
        );
        assert_eq!(
            split_lines("a\r\nb\rc\nd", &[CrLf]),
            (vec![line("a", CrLf)], "b\rc\nd".to_string())
        );
    }

    #[test]
    fn split_lines_takes_crlf_whole_before_cr_or_lf() {
        assert_eq!(
            split_lines("a\r\nb\rc\nd", &[Cr, Lf, CrLf]),
            (
                vec![line("a", CrLf), line("b", Cr), line("c", Lf)],
                "d".to_string()
            )
        );
    }

    #[test]
    fn split_lines_waits_on_a_last_cr_while_crlf_may_follow() {
        assert_eq!(split_lines("a\r", &[Cr, CrLf]), (vec![], "a\r".to_string()));
        assert_eq!(
            split_lines("a\r", &[Cr]),
            (vec![line("a", Cr)], String::new())
        );
    }
}
