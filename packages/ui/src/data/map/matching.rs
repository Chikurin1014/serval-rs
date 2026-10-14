use std::any::Any;

use dioxus::prelude::*;
use fancy_regex::{Captures, Regex};

use crate::data::{
    Conversion, ConversionInput, ConversionOutput, DataContext, Endpoints, Log, LogData, LogLevel,
    MapRunner, NumberData, Segment, SourceCursor, Vector2d, Vector2dData, Vector3d, Vector3dData,
    compile_for_map, expand_levels, format_number, keep_taken, trim_segments,
};
use crate::helper::set_if_changed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchOutput {
    Number,
    Vector2d,
    Vector3d,
    /// Its level, then its message.
    Log,
}

impl MatchOutput {
    /// How many values it takes: a vector's components, a log's level and
    /// message, or one.
    fn components(self) -> usize {
        match self {
            MatchOutput::Number => 1,
            MatchOutput::Vector2d | MatchOutput::Log => 2,
            MatchOutput::Vector3d => 3,
        }
    }

    fn push_to_data_context(
        self,
        data: &mut DataContext,
        label: &str,
        values: Vec<String>,
        timestamp: i64,
    ) -> Result<(), String> {
        if let (MatchOutput::Log, [level, message]) = (self, values.as_slice()) {
            data.push(label, LogData::new(timestamp, log_of(level, message)?));
            return Ok(());
        }
        let numbers = values
            .iter()
            .map(|value| {
                value
                    .parse::<f64>()
                    .map_err(|_| format!("The result '{value}' is not a number"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        match (self, numbers.as_slice()) {
            (MatchOutput::Number, &[number]) => {
                data.push(label, NumberData::new(timestamp, number));
            }
            (MatchOutput::Vector2d, &[x, y]) => {
                data.push(label, Vector2dData::new(timestamp, Vector2d::new(x, y)));
            }
            (MatchOutput::Vector3d, &[x, y, z]) => {
                data.push(label, Vector3dData::new(timestamp, Vector3d::new(x, y, z)));
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct MatchSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    pub pattern: Signal<String>,
    /// The value; a vector's x; a log's level.
    pub replacement: Signal<String>,
    /// A vector's y (or a log's message), and a 3D one's z.
    pub y_replacement: Option<Signal<String>>,
    pub z_replacement: Option<Signal<String>>,
    pub pattern_error: Signal<Option<String>>,
    pub replacement_error: Signal<Option<String>>,
    /// The weakest level of the input taken: those weaker left out.
    pub weakest_level: Signal<LogLevel>,
}

/// Converts each log message matching a regex; the output label may use its
/// groups.
pub struct Match {
    output: MatchOutput,
    settings: MatchSettings,
    /// The input and output labels, pattern and replacement; then a vector's y
    /// (and z); then the weakest level taken.
    taken: ([String; 4], Vec<String>, LogLevel),
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

impl Match {
    pub fn new(output: MatchOutput) -> Self {
        Self::with(output, "", "", "", "")
    }

    pub fn with(
        output: MatchOutput,
        from_label: &str,
        pattern: &str,
        to_label: &str,
        replacement: &str,
    ) -> Self {
        Self::with_components(output, from_label, pattern, to_label, &[replacement])
    }

    /// As [`Self::with`], with a vector's components, `[x, y]` or `[x, y, z]`.
    pub fn with_components(
        output: MatchOutput,
        from_label: &str,
        pattern: &str,
        to_label: &str,
        components: &[&str],
    ) -> Self {
        let component = |index: usize| {
            (index < output.components()).then(|| {
                Signal::new(
                    components
                        .get(index)
                        .copied()
                        .unwrap_or_default()
                        .to_string(),
                )
            })
        };
        Self {
            output,
            settings: MatchSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
                pattern: Signal::new(pattern.to_string()),
                replacement: Signal::new(
                    components.first().copied().unwrap_or_default().to_string(),
                ),
                y_replacement: component(1),
                z_replacement: component(2),
                pattern_error: Signal::new(None),
                replacement_error: Signal::new(None),
                weakest_level: Signal::new(LogLevel::None),
            },
            taken: Default::default(),
            matching: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl MapRunner for Match {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let MatchSettings {
            from_label,
            to_label,
            pattern,
            replacement,
            y_replacement,
            z_replacement,
            mut pattern_error,
            mut replacement_error,
            weakest_level,
        } = self.settings;
        let mut taken =
            [from_label, to_label, pattern, replacement].map(|text| text.peek().clone());
        // Whitespace only counts as none
        if taken[2].trim().is_empty() {
            taken[2].clear();
        }
        let components = [y_replacement, z_replacement]
            .into_iter()
            .flatten()
            .map(|text| text.peek().clone())
            .collect();
        if keep_taken(&mut self.taken, (taken, components, *weakest_level.peek())) {
            self.cursor.reset();
            self.matching = Matching::new(&self.taken.0, &mut pattern_error);
        }
        set_if_changed(&mut replacement_error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let MatchSettings {
            mut pattern_error,
            mut replacement_error,
            ..
        } = self.settings;
        let Matching {
            endpoints: Endpoints { from, to },
            regex,
            replacement,
        } = self.matching.as_ref()?;
        // The value, or a vector's components
        let templates: Vec<&String> = std::iter::once(replacement).chain(&self.taken.1).collect();

        let weakest = self.taken.2;
        let entries = self.cursor.new_entries::<LogData>(data, from)?.entries;
        if entries.is_empty() {
            return None;
        }

        let mut error = None;
        let mut failure = None;
        let mut latest = None;
        for entry in &entries {
            if entry.value().level() > weakest {
                continue;
            }
            let input = entry.value().message();
            // Its level, where the templates name it
            let levels = [entry.value().level()];
            let to = expand_levels(to, &levels);
            let replacements: Vec<String> = templates
                .iter()
                .map(|template| expand_levels(template, &levels))
                .collect();
            let captures = match regex.captures(input) {
                Ok(Some(captures)) => captures,
                Ok(None) => continue,
                Err(failed) => {
                    failure = Some(format!("Matching failed: {failed}"));
                    continue;
                }
            };
            let label = text(&label_segments(&captures, &to));
            if label.is_empty() {
                continue;
            }
            let values = replacements
                .iter()
                .map(|replacement| expand(&captures, replacement))
                .collect();
            match self
                .output
                .push_to_data_context(data, &label, values, timestamp)
            {
                Ok(()) => latest = Some((input, captures, to, replacements)),
                Err(message) => error = Some(message),
            }
        }
        set_if_changed(&mut pattern_error, failure);
        set_if_changed(&mut replacement_error, error);

        let (input, captures, to, replacements) = latest?;
        let to_value = match self.output {
            MatchOutput::Number => number_segments(&captures, &replacements[0]),
            MatchOutput::Vector2d | MatchOutput::Vector3d => {
                let mut segments = vec![Segment::fixed("(")];
                for (index, replacement) in replacements.iter().enumerate() {
                    if index > 0 {
                        segments.push(Segment::fixed(", "));
                    }
                    segments.extend(number_segments(&captures, replacement));
                }
                segments.push(Segment::fixed(")"));
                segments
            }
            MatchOutput::Log => log_segments(
                &captures,
                &replacements[0],
                replacements.get(1).map_or("", String::as_str),
            ),
        };
        Some(Conversion {
            from: vec![ConversionInput::new(from, input)],
            to: vec![ConversionOutput::new(
                label_segments(&captures, &to),
                to_value,
            )],
        })
    }
}

/// A log of `level` (a level's name, or nothing for None) and `message`.
pub(super) fn log_of(level: &str, message: &str) -> Result<Log, String> {
    let level = level
        .parse::<LogLevel>()
        .map_err(|error| error.to_string())?;
    Ok(Log::new(level, message))
}

/// A log as it is shown: its level as a tag before its message; the message
/// alone, with no level.
pub(super) fn log_segments(captures: &Captures<str>, level: &str, message: &str) -> Vec<Segment> {
    let message = replacement_segments(captures, message);
    let level_segments = replacement_segments(captures, level);
    let from_input = level_segments.iter().any(|segment| segment.from_input);
    let mut segments = match text(&level_segments).parse::<LogLevel>() {
        Ok(LogLevel::None) => return message,
        Ok(name) => vec![Segment::level(name, from_input)],
        // Not a level: as it is, with the error beside the form
        Err(_) => {
            let mut segments = vec![Segment::fixed("[")];
            segments.extend(level_segments);
            segments.push(Segment::fixed("] "));
            segments
        }
    };
    segments.extend(message);
    segments
}

pub(super) fn text(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

/// What [`replacement_segments`] builds, as one formatted number if it is one.
fn number_segments(captures: &Captures<str>, replacement: &str) -> Vec<Segment> {
    let segments = replacement_segments(captures, replacement);
    number_segment(&segments).map_or(segments, |segment| vec![segment])
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

pub(super) fn label_segments(captures: &Captures<str>, template: &str) -> Vec<Segment> {
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
    use dioxus::prelude::*;
    use fancy_regex::Regex;

    use super::{
        Match, MatchOutput, MatchSettings, expand, label_segments, replacement_segments, text,
    };
    use crate::data::{
        LogLevel, Segment,
        map::testing::{Maps, log},
    };

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

    #[test]
    fn an_invalid_pattern_is_an_error() {
        let mut maps = Maps::new();
        let settings: MatchSettings = maps.add(
            || Match::with(MatchOutput::Number, "log", "(", "out", "$0"),
            |_| {},
        );
        assert!(
            maps.read(settings.pattern_error)
                .is_some_and(|error| error.contains("Invalid regex"))
        );
    }

    #[test]
    fn a_level_it_does_not_know_is_an_error() {
        let mut maps = Maps::new();
        let settings: MatchSettings = maps.add(
            || {
                Match::with_components(
                    MatchOutput::Log,
                    "log",
                    r"(\w+): (.*)",
                    "out",
                    &["$1", "$2"],
                )
            },
            |_| {},
        );
        maps.push("log", log(LogLevel::None, "fatal: no sensor"));
        assert_eq!(
            maps.read(settings.replacement_error).as_deref(),
            Some("No log level 'fatal'")
        );
        assert!(maps.values("out").is_empty());
    }

    #[test]
    fn templates_name_the_level_of_the_input() {
        let mut maps = Maps::new();
        maps.add(
            || {
                Match::with_components(
                    MatchOutput::Log,
                    "log",
                    "disk (.*)",
                    "kept_${level:0}",
                    &["${level:0}", "${level:0}: $1"],
                )
            },
            |_: MatchSettings| {},
        );
        maps.push("log", log(LogLevel::Error, "disk lost"));
        assert_eq!(maps.values("kept_ERROR"), ["[ERROR] ERROR: lost"]);
    }

    #[test]
    fn weaker_levels_are_left_out() {
        let mut maps = Maps::new();
        maps.add(
            || {
                Match::with(
                    MatchOutput::Number,
                    "log",
                    "temp=({number})",
                    "warned",
                    "$1",
                )
            },
            |mut settings: MatchSettings| settings.weakest_level.set(LogLevel::Warn),
        );
        maps.push("log", log(LogLevel::Error, "temp=1"));
        maps.push("log", log(LogLevel::Info, "temp=2"));
        maps.push("log", log(LogLevel::Warn, "temp=3"));
        assert_eq!(maps.values("warned"), ["1.0000", "3.0000"]);
    }
}
