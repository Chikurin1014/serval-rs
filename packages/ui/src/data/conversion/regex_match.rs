use std::any::Any;

use dioxus::prelude::*;
use regex::{Captures, Regex};

use crate::data::{set_if_changed, Converter, DataContext, NumberData, SourceCursor, StringData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegexOutput {
    String,
    Number,
}

impl RegexOutput {
    fn push_to_data_context(
        self,
        data: &mut DataContext,
        label: &str,
        value: String,
        timestamp: i64,
    ) -> Result<(), String> {
        match self {
            RegexOutput::String => data.push_string(label, StringData::new(timestamp, value)),
            RegexOutput::Number => {
                let number = value
                    .parse::<f64>()
                    .map_err(|_| format!("The result '{value}' is not a number"))?;
                data.push_number(label, NumberData::new(timestamp, number));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct RegexMatchSettings {
    pub from_label: Signal<String>,
    /// May use the pattern's capture groups (`$1`).
    pub to_label: Signal<String>,
    pub pattern: Signal<String>,
    /// Regex replacement producing the value.
    pub replacement: Signal<String>,
    pub pattern_error: Signal<Option<String>>,
    pub replacement_error: Signal<Option<String>>,
}

/// Converts each string matching a regex, labelling the result with
/// `to_label` expanded from the same match (so one conversion can fan out to
/// several labels).
pub struct RegexMatch {
    output: RegexOutput,
    settings: RegexMatchSettings,
    /// Settings of the previous run; a change restarts from the start of the source.
    last_settings: Option<[String; 4]>,
    cursor: SourceCursor,
}

impl RegexMatch {
    pub fn new(output: RegexOutput) -> Self {
        Self {
            output,
            settings: RegexMatchSettings {
                from_label: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
                pattern: Signal::new(String::new()),
                replacement: Signal::new(String::new()),
                pattern_error: Signal::new(None),
                replacement_error: Signal::new(None),
            },
            last_settings: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl Converter for RegexMatch {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let RegexMatchSettings {
            from_label,
            to_label,
            pattern,
            replacement,
            mut pattern_error,
            mut replacement_error,
        } = self.settings;

        let current = [from_label(), to_label(), pattern(), replacement()];
        if self.last_settings.as_ref() != Some(&current) {
            self.last_settings = Some(current.clone());
            self.cursor.reset();
            set_if_changed(&mut pattern_error, None);
            set_if_changed(&mut replacement_error, None);
        }
        let [source, to_label, pattern, replacement] = current;
        if source.trim().is_empty()
            || to_label.trim().is_empty()
            || source.trim() == to_label.trim()
            || pattern.trim().is_empty()
        {
            return;
        }

        let regex = match Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(error) => {
                set_if_changed(&mut pattern_error, Some(format!("Invalid regex: {error}")));
                return;
            }
        };

        // A restart needs no special handling: what was converted before stays
        let Some(entries) = self
            .cursor
            .new_strings(data, &source)
            .map(|read| read.entries)
        else {
            return;
        };
        if entries.is_empty() {
            return;
        }

        let mut error = None;
        for entry in &entries {
            let input = entry.value();
            let Some(captures) = regex.captures(input) else {
                continue;
            };
            let label = regex.replace(input, to_label.as_str());
            let label = label.trim();
            if label.is_empty() {
                continue;
            }
            let value = expand(&captures, &replacement);
            if let Err(message) = self
                .output
                .push_to_data_context(data, label, value, timestamp)
            {
                error = Some(message);
            }
        }
        set_if_changed(&mut pattern_error, None);
        set_if_changed(&mut replacement_error, error);
    }
}

/// Builds the value for one match from the replacement and its capture groups,
/// leaving out the text around the match.
fn expand(captures: &Captures, replacement: &str) -> String {
    let mut value = String::new();
    captures.expand(replacement, &mut value);
    value
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::expand;

    fn expand_first(pattern: &str, input: &str, replacement: &str) -> String {
        let regex = Regex::new(pattern).unwrap();
        expand(&regex.captures(input).unwrap(), replacement)
    }

    #[test]
    fn value_keeps_only_what_the_replacement_names() {
        assert_eq!(expand_first(r"temp:(\S+)", "xx temp:20.5 yy", "$1"), "20.5");
        assert_eq!(
            expand_first(r"led: (on|off)", "led: on", "state=$1"),
            "state=on"
        );
    }

    #[test]
    fn value_parses_as_number() {
        assert!(expand_first(r"(\d+)", "abc123def", "$1")
            .parse::<f64>()
            .is_ok());
    }

    #[test]
    fn target_label_can_use_match_groups() {
        let regex = Regex::new(r"led: (on|off)").unwrap();
        assert_eq!(regex.replace("led: on", "led_$1"), "led_on");
    }
}
