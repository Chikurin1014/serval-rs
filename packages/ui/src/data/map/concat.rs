use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionInput, ConversionOutput, DataContext, Input, Log, LogData, LogLevel,
    MapRunner, Segment, endpoints, expand_levels, keep_taken, take_newest_pair,
};
use crate::helper::{set_if_changed, unescape};

/// The level of a log joined from two, unless set: that of the one that
/// matters more.
pub const MORE_LEVEL: &str = "${level:max}";

#[derive(Clone, Copy, PartialEq)]
pub struct ConcatSettings {
    pub first_label: Signal<String>,
    pub second_label: Signal<String>,
    /// The weakest level of each input taken: those weaker left out.
    pub first_weakest_level: Signal<LogLevel>,
    pub second_weakest_level: Signal<LogLevel>,
    pub to_label: Signal<String>,
    /// With `\n`-style escapes; and the inputs' levels by their names (as
    /// `${level:0}`).
    pub separator: Signal<String>,
    /// A level's name, or the inputs' by theirs; nothing for None.
    pub level: Signal<String>,
    pub error: Signal<Option<String>>,
}

/// Joins the newest log messages of two inputs, once both have a new one.
pub struct Concat {
    settings: ConcatSettings,
    taken: Option<Taken>,
    first: Input<Log>,
    second: Input<Log>,
}

#[derive(PartialEq)]
struct Taken {
    first: String,
    second: String,
    first_weakest: LogLevel,
    second_weakest: LogLevel,
    to: String,
    separator: String,
    level: String,
}

impl Concat {
    pub fn new() -> Self {
        Self {
            settings: ConcatSettings {
                first_label: Signal::new(String::new()),
                second_label: Signal::new(String::new()),
                first_weakest_level: Signal::new(LogLevel::None),
                second_weakest_level: Signal::new(LogLevel::None),
                to_label: Signal::new(String::new()),
                separator: Signal::new(String::new()),
                level: Signal::new(MORE_LEVEL.to_string()),
                error: Signal::new(None),
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
            first_weakest_level,
            second_weakest_level,
            to_label,
            separator,
            level,
            mut error,
        } = self.settings;
        let (first, second, to) = (first_label.peek(), second_label.peek(), to_label.peek());
        let taken = match (endpoints(&first, &to), endpoints(&second, &to)) {
            (Some((first, to)), Some((second, _))) => Some(Taken {
                first: first.to_string(),
                second: second.to_string(),
                first_weakest: *first_weakest_level.peek(),
                second_weakest: *second_weakest_level.peek(),
                to: to.to_string(),
                separator: unescape(&separator.peek()),
                level: level.peek().clone(),
            }),
            _ => None,
        };
        if keep_taken(&mut self.taken, taken) {
            self.first.forget();
            self.second.forget();
        }
        set_if_changed(&mut error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let mut error = self.settings.error;
        let Taken {
            first,
            second,
            first_weakest,
            second_weakest,
            to,
            separator,
            level,
        } = self.taken.as_ref()?;
        self.first
            .read_newest_where(data, first, |log| log.level() <= *first_weakest);
        self.second
            .read_newest_where(data, second, |log| log.level() <= *second_weakest);
        let (first_log, second_log) = take_newest_pair(&mut self.first, &mut self.second)?;
        let (first_value, second_value) = (first_log.message(), second_log.message());

        // The inputs' levels, where the separator or level names them
        let levels = [first_log.level(), second_log.level()];
        let separator = expand_levels(separator, &levels);
        let level = match expand_levels(level, &levels).parse::<LogLevel>() {
            Ok(level) => level,
            Err(failed) => {
                set_if_changed(&mut error, Some(failed.to_string()));
                return None;
            }
        };
        set_if_changed(&mut error, None);

        let conversion = Conversion {
            from: vec![
                ConversionInput::new(first, first_value),
                ConversionInput::new(second, second_value),
            ],
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(to)],
                concat_segments(first_value, &separator, second_value),
            )],
        };
        let message = format!("{first_value}{separator}{second_value}");
        data.push(to, LogData::new(timestamp, Log::new(level, message)));
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
