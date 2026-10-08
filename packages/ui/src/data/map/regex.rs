use std::any::Any;

use dioxus::prelude::*;
use fancy_regex::{Captures, Regex};

use crate::data::{
    Conversion, ConversionInput, DataContext, Endpoints, MapRunner, NumberData, Segment,
    SourceCursor, StringData, compile_for_map, format_number, keep_taken, trim_segments,
};
use crate::helper::set_if_changed;

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
            RegexOutput::String => data.push(label, StringData::new(timestamp, value)),
            RegexOutput::Number => {
                let number = value
                    .parse::<f64>()
                    .map_err(|_| format!("The result '{value}' is not a number"))?;
                data.push(label, NumberData::new(timestamp, number));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct RegexSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    pub pattern: Signal<String>,
    pub replacement: Signal<String>,
    pub pattern_error: Signal<Option<String>>,
    pub replacement_error: Signal<Option<String>>,
}

/// Converts each string matching a regex; the output label may use its groups.
pub struct RegexMatch {
    output: RegexOutput,
    settings: RegexSettings,
    taken: [String; 4],
    matching: Option<Matching>,
    cursor: SourceCursor,
}

/// The settings of a map matching a pattern, checked and compiled.
pub(super) struct Matching {
    pub(super) endpoints: Endpoints,
    pub(super) regex: Regex,
    pub(super) replacement: String,
}

impl Matching {
    /// From the input label, output label, pattern and replacement, if valid.
    pub(super) fn new(
        settings: &[String; 4],
        pattern_error: &mut Signal<Option<String>>,
    ) -> Option<Self> {
        let [from, to, pattern, replacement] = settings;
        // Even with labels missing, to report pattern errors
        let regex = compile_for_map(pattern, pattern_error);
        Some(Self {
            endpoints: Endpoints::new(from, to)?,
            regex: regex?,
            replacement: replacement.clone(),
        })
    }
}

impl RegexMatch {
    pub fn new(output: RegexOutput) -> Self {
        Self::with(output, "", "", "", "")
    }

    pub fn with(
        output: RegexOutput,
        from_label: &str,
        pattern: &str,
        to_label: &str,
        replacement: &str,
    ) -> Self {
        Self {
            output,
            settings: RegexSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
                pattern: Signal::new(pattern.to_string()),
                replacement: Signal::new(replacement.to_string()),
                pattern_error: Signal::new(None),
                replacement_error: Signal::new(None),
            },
            taken: Default::default(),
            matching: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl MapRunner for RegexMatch {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let RegexSettings {
            from_label,
            to_label,
            pattern,
            replacement,
            mut pattern_error,
            mut replacement_error,
        } = self.settings;
        let mut taken =
            [from_label, to_label, pattern, replacement].map(|text| text.peek().clone());
        // Whitespace only counts as none
        if taken[2].trim().is_empty() {
            taken[2].clear();
        }
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
            self.matching = Matching::new(&self.taken, &mut pattern_error);
        }
        set_if_changed(&mut replacement_error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let RegexSettings {
            mut pattern_error,
            mut replacement_error,
            ..
        } = self.settings;
        let Matching {
            endpoints: Endpoints { from, to },
            regex,
            replacement,
        } = self.matching.as_ref()?;

        let entries = self.cursor.new_entries::<StringData>(data, from)?.entries;
        if entries.is_empty() {
            return None;
        }

        let mut error = None;
        let mut failure = None;
        let mut latest = None;
        for entry in &entries {
            let input = entry.value().as_str();
            let captures = match regex.captures(input) {
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
            let value = expand(&captures, replacement);
            match self
                .output
                .push_to_data_context(data, &label, value, timestamp)
            {
                Ok(()) => latest = Some((input, captures)),
                Err(message) => error = Some(message),
            }
        }
        set_if_changed(&mut pattern_error, failure);
        set_if_changed(&mut replacement_error, error);

        let (input, captures) = latest?;
        let mut to_value = replacement_segments(&captures, replacement);
        if self.output == RegexOutput::Number {
            to_value = number_segment(&to_value).map_or(to_value, |segment| vec![segment]);
        }
        Some(Conversion {
            from: vec![ConversionInput::new(from, input)],
            to_label: label_segments(&captures, to),
            to_value,
        })
    }
}

fn text(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

/// The number that `segments` make up, as one formatted segment.
fn number_segment(segments: &[Segment]) -> Option<Segment> {
    let number = text(segments).parse::<f64>().ok()?;
    let text = format_number(number);
    Some(if segments.iter().any(|segment| segment.from_input) {
        Segment::from_input(text)
    } else {
        Segment::fixed(text)
    })
}

/// `replacement` with the groups of `captures` filled in.
pub(super) fn expand(captures: &Captures<str>, replacement: &str) -> String {
    let mut value = String::new();
    captures.expand(replacement, &mut value);
    value
}

fn label_segments(captures: &Captures<str>, template: &str) -> Vec<Segment> {
    trim_segments(replacement_segments(captures, template))
}

/// What [`expand`] builds, split into fixed text and filled-in groups.
pub(super) fn replacement_segments(captures: &Captures<str>, replacement: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut fixed = String::new();
    let mut rest = replacement;
    while let Some(dollar) = rest.find('$') {
        fixed.push_str(&rest[..dollar]);
        rest = &rest[dollar..];
        if rest[1..].starts_with('$') {
            fixed.push('$');
            rest = &rest[2..];
            continue;
        }
        let Some(end) = group_reference_end(rest) else {
            fixed.push('$');
            rest = &rest[1..];
            continue;
        };
        if !fixed.is_empty() {
            segments.push(Segment::fixed(std::mem::take(&mut fixed)));
        }
        segments.push(Segment::from_input(expand(captures, &rest[..end])));
        rest = &rest[end..];
    }
    fixed.push_str(rest);
    segments.push(Segment::fixed(fixed));
    segments.retain(|segment| !segment.text.is_empty());
    segments
}

/// The length of the group reference (`$1`, `${name}`) `text` starts with.
fn group_reference_end(text: &str) -> Option<usize> {
    let rest = &text[1..];
    if let Some(braced) = rest.strip_prefix('{') {
        return braced.find('}').map(|close| close + 3);
    }
    let name = rest
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .count();
    (name > 0).then_some(name + 1)
}

#[cfg(test)]
mod tests {
    use fancy_regex::Regex;

    use super::{expand, label_segments, replacement_segments, text};
    use crate::data::Segment;

    fn expand_first(pattern: &str, input: &str, replacement: &str) -> String {
        let regex = Regex::new(pattern).unwrap();
        expand(&regex.captures(input).unwrap().unwrap(), replacement)
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
    fn value_can_be_the_whole_match() {
        assert_eq!(expand_first(r"[\d.]+", "20.5 x", "$0"), "20.5");
    }

    #[test]
    fn value_parses_as_number() {
        assert!(
            expand_first(r"(\d+)", "abc123def", "$1")
                .parse::<f64>()
                .is_ok()
        );
    }

    #[test]
    fn replacement_segments_mark_the_groups() {
        let regex = Regex::new(r"(?<name>\w+): (\w+)").unwrap();
        let captures = regex.captures("led: on").unwrap().unwrap();
        assert_eq!(
            replacement_segments(&captures, "${name}_$2!"),
            vec![
                Segment::from_input("led"),
                Segment::fixed("_"),
                Segment::from_input("on"),
                Segment::fixed("!"),
            ]
        );
    }

    #[test]
    fn replacement_segments_make_what_expand_makes() {
        let regex = Regex::new(r"(?<name>\w+): (\w+)").unwrap();
        let captures = regex.captures("led: on").unwrap().unwrap();
        for replacement in [
            "$1",
            "a$$b",
            "$",
            "$1x",
            "${1}x",
            "${2",
            "$9",
            "x $name y",
            "$$1",
        ] {
            assert_eq!(
                text(&replacement_segments(&captures, replacement)),
                expand(&captures, replacement),
                "{replacement}"
            );
        }
    }

    #[test]
    fn label_leaves_out_the_text_around_the_match() {
        let regex = Regex::new(r"[\d.]+").unwrap();
        let captures = regex.captures("temp:20.5 C").unwrap().unwrap();
        assert_eq!(
            label_segments(&captures, "foo"),
            vec![Segment::fixed("foo")]
        );
    }

    #[test]
    fn label_uses_the_match_groups_trimmed() {
        let regex = Regex::new(r"(\w+): (\S+)").unwrap();
        let captures = regex.captures("xx led: on").unwrap().unwrap();
        let segments = label_segments(&captures, " ${1}_$2 ");
        assert_eq!(text(&segments), "led_on");
        assert_eq!(
            segments,
            vec![
                Segment::from_input("led"),
                Segment::fixed("_"),
                Segment::from_input("on"),
            ]
        );
    }
}
