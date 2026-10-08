use std::any::Any;

use dioxus::prelude::*;
use fancy_regex::Regex;

use super::regex::replacement_segments;
use crate::data::{
    Conversion, ConversionInput, DataContext, MapRunner, Segment, SourceCursor, StringData,
    compile_for_map, endpoints, keep_taken, set_if_changed,
};

#[derive(Clone, Copy, PartialEq)]
pub struct ReplaceSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    pub pattern: Signal<String>,
    /// Put in place of each match; may use its capture groups (`$1`).
    pub replacement: Signal<String>,
    pub pattern_error: Signal<Option<String>>,
}

/// Replaces every match of a regex in each string, passing on the rest of
/// the string (and strings with no match) as they are.
pub struct Replace {
    settings: ReplaceSettings,
    latest: Signal<Option<Conversion>>,
    /// The input label, output label, pattern and replacement, as taken in
    /// when turned on
    taken: Option<[String; 4]>,
    /// The pattern, compiled as the map was turned on
    regex: Option<Regex>,
    cursor: SourceCursor,
}

impl Replace {
    pub fn new() -> Self {
        Self {
            settings: ReplaceSettings {
                from_label: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
                pattern: Signal::new(String::new()),
                replacement: Signal::new(String::new()),
                pattern_error: Signal::new(None),
            },
            latest: Signal::new(None),
            taken: None,
            regex: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl Default for Replace {
    fn default() -> Self {
        Self::new()
    }
}

impl MapRunner for Replace {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn latest(&self) -> Signal<Option<Conversion>> {
        self.latest
    }

    fn start(&mut self) {
        let ReplaceSettings {
            from_label,
            to_label,
            pattern,
            replacement,
            mut pattern_error,
        } = self.settings;
        let taken = [from_label, to_label, pattern, replacement].map(|text| text.peek().clone());
        let pattern = taken[2].clone();
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
            self.regex = compile_for_map(&pattern, &mut pattern_error);
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let mut pattern_error = self.settings.pattern_error;
        let (Some([from, to, _, replacement]), Some(regex)) = (&self.taken, &self.regex) else {
            return;
        };
        let Some((from, to)) = endpoints(from, to) else {
            return;
        };

        // A restart needs no special handling: what was replaced before stays
        let Some(read) = self.cursor.new_entries::<StringData>(data, from) else {
            return;
        };
        let Some(last) = read.entries.last() else {
            return;
        };

        // Split into segments only for the one shown
        let conversion = Conversion {
            from: vec![ConversionInput::new(from, last.value().as_str())],
            to_label: vec![Segment::fixed(to)],
            to_value: replaced_segments(regex, last.value(), replacement),
        };
        set_if_changed(&mut self.latest, Some(conversion));
        let mut failure = None;
        for entry in &read.entries {
            match regex.try_replacen(entry.value(), 0, replacement.as_str()) {
                Ok(value) => data.push(to, StringData::new(timestamp, value.into_owned())),
                // E.g. backtracking too much on this input
                Err(failed) => failure = Some(format!("Matching failed: {failed}")),
            }
        }
        set_if_changed(&mut pattern_error, failure);
    }
}

/// What `regex.replace_all(input, replacement)` gives, as segments: the text
/// around the matches and their groups come from the input.
fn replaced_segments(regex: &Regex, input: &str, replacement: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut end = 0;
    // Up to a match that fails, as the replacing does
    for captures in regex.captures_iter(input).map_while(Result::ok) {
        let whole = captures.get(0).expect("group 0 is the whole match");
        segments.push(Segment::from_input(&input[end..whole.start()]));
        segments.extend(replacement_segments(&captures, replacement));
        end = whole.end();
    }
    segments.push(Segment::from_input(&input[end..]));
    segments.retain(|segment| !segment.text.is_empty());
    segments
}

#[cfg(test)]
mod tests {
    use fancy_regex::Regex;

    use super::replaced_segments;
    use crate::data::Segment;

    #[test]
    fn replaced_segments_make_what_replace_all_makes() {
        let regex = Regex::new(r"(\d+)").unwrap();
        let input = "a1 b22 c";
        let segments = replaced_segments(&regex, input, "<$1>");
        let text: String = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(text, regex.replace_all(input, "<$1>"));
        assert_eq!(
            segments,
            vec![
                Segment::from_input("a"),
                Segment::fixed("<"),
                Segment::from_input("1"),
                Segment::fixed(">"),
                Segment::from_input(" b"),
                Segment::fixed("<"),
                Segment::from_input("22"),
                Segment::fixed(">"),
                Segment::from_input(" c"),
            ]
        );
    }

    #[test]
    fn replaced_segments_keep_a_string_with_no_match() {
        let regex = Regex::new("x").unwrap();
        assert_eq!(
            replaced_segments(&regex, "abc", "y"),
            vec![Segment::from_input("abc")]
        );
    }
}
