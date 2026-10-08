use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionInput, DataContext, Input, MapRunner, Segment, StringData, endpoints,
    keep_taken, take_newest_pair,
};
use crate::helper::unescape;

#[derive(Clone, Copy, PartialEq)]
pub struct ConcatSettings {
    pub first_label: Signal<String>,
    pub second_label: Signal<String>,
    pub to_label: Signal<String>,
    /// With `\n`-style escapes.
    pub separator: Signal<String>,
}

/// Joins the newest strings of two inputs, once both have a new one.
pub struct Concat {
    settings: ConcatSettings,
    taken: Option<Taken>,
    first: Input<String>,
    second: Input<String>,
}

#[derive(PartialEq)]
struct Taken {
    first: String,
    second: String,
    to: String,
    separator: String,
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
            taken: None,
            first: Input::default(),
            second: Input::default(),
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

    fn start(&mut self) {
        let ConcatSettings {
            first_label,
            second_label,
            to_label,
            separator,
        } = self.settings;
        let (first, second, to) = (first_label.peek(), second_label.peek(), to_label.peek());
        let taken = match (endpoints(&first, &to), endpoints(&second, &to)) {
            (Some((first, to)), Some((second, _))) => Some(Taken {
                first: first.to_string(),
                second: second.to_string(),
                to: to.to_string(),
                separator: unescape(&separator.peek()),
            }),
            _ => None,
        };
        if keep_taken(&mut self.taken, taken) {
            self.first.forget();
            self.second.forget();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let Taken {
            first,
            second,
            to,
            separator,
        } = self.taken.as_ref()?;
        self.first.read_newest(data, first);
        self.second.read_newest(data, second);
        let (first_value, second_value) = take_newest_pair(&mut self.first, &mut self.second)?;

        let conversion = Conversion {
            from: vec![
                ConversionInput::new(first, first_value.as_str()),
                ConversionInput::new(second, second_value.as_str()),
            ],
            to_label: vec![Segment::fixed(to)],
            to_value: concat_segments(&first_value, separator, &second_value),
        };
        let value = format!("{first_value}{separator}{second_value}");
        data.push(to, StringData::new(timestamp, value));
        Some(conversion)
    }
}

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
