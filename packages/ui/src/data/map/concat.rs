use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionInput, DataContext, MapRunner, Segment, SourceCursor, StringData,
    set_if_changed, unescape,
};

#[derive(Clone, Copy, PartialEq)]
pub struct ConcatSettings {
    pub first_label: Signal<String>,
    pub second_label: Signal<String>,
    pub to_label: Signal<String>,
    /// Put between the two; `\n`-style escapes allowed, empty for nothing
    pub separator: Signal<String>,
}

/// Joins two strings, one from each source, once both have a new one:
/// the newest of each, dropping any older ones that came in between.
pub struct Concat {
    settings: ConcatSettings,
    latest: Signal<Option<Conversion>>,
    first: Source,
    second: Source,
}

/// One of the sources and its newest string since the last join.
#[derive(Default)]
struct Source {
    cursor: SourceCursor,
    newest: Option<String>,
}

impl Source {
    /// Takes in what is new under `label`.
    fn read(&mut self, data: &DataContext, label: &str) {
        match self.cursor.new_entries::<StringData>(data, label) {
            Some(read) => {
                if read.restarted {
                    self.newest = None;
                }
                if let Some(last) = read.entries.last() {
                    self.newest = Some(last.value().clone());
                }
            }
            None => {
                self.cursor.reset();
                self.newest = None;
            }
        }
    }
}

impl Concat {
    pub fn new() -> Self {
        Self {
            settings: ConcatSettings {
                first_label: Signal::new(String::new()),
                second_label: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
                separator: Signal::new(String::new()),
            },
            latest: Signal::new(None),
            first: Source::default(),
            second: Source::default(),
        }
    }
}

impl Default for Concat {
    fn default() -> Self {
        Self::new()
    }
}

impl MapRunner for Concat {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn latest(&self) -> Signal<Option<Conversion>> {
        self.latest
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let ConcatSettings {
            first_label,
            second_label,
            to_label,
            separator,
        } = self.settings;
        let (first, second, target) = (first_label(), second_label(), to_label());
        let (first, second, target) = (first.trim(), second.trim(), target.trim());
        if first.is_empty() || second.is_empty() || target.is_empty() {
            return;
        }
        if target == first || target == second {
            return;
        }

        self.first.read(data, first);
        self.second.read(data, second);
        let (Some(first_value), Some(second_value)) =
            (self.first.newest.clone(), self.second.newest.clone())
        else {
            return;
        };

        let separator = unescape(&separator());
        let conversion = Conversion {
            from: vec![
                ConversionInput::new(first, first_value.as_str()),
                ConversionInput::new(second, second_value.as_str()),
            ],
            to_label: vec![Segment::fixed(target)],
            to_value: concat_segments(&first_value, &separator, &second_value),
        };
        let value = format!("{first_value}{separator}{second_value}");
        self.first.newest = None;
        self.second.newest = None;
        set_if_changed(&mut self.latest, Some(conversion));
        data.push(target, StringData::new(timestamp, value));
    }
}

/// `first`, `separator` and `second` as segments: the two values come from
/// the input, the separator from the settings.
fn concat_segments(first: &str, separator: &str, second: &str) -> Vec<Segment> {
    let mut segments = vec![
        Segment::from_input(first),
        Segment::fixed(separator),
        Segment::from_input(second),
    ];
    segments.retain(|segment| !segment.text.is_empty());
    segments
}

#[cfg(test)]
mod tests {
    use super::concat_segments;
    use crate::data::Segment;

    #[test]
    fn concat_segments_leave_out_an_empty_separator() {
        assert_eq!(
            concat_segments("20.5", ",", "3.3"),
            vec![
                Segment::from_input("20.5"),
                Segment::fixed(","),
                Segment::from_input("3.3"),
            ]
        );
        assert_eq!(
            concat_segments("a", "", "b"),
            vec![Segment::from_input("a"), Segment::from_input("b")]
        );
    }
}
