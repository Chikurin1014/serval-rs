use crate::data::LogLevel;

/// A map's latest conversion, as shown in its card.
#[derive(Clone, Debug, PartialEq)]
pub struct Conversion {
    pub from: Vec<ConversionInput>,
    /// Each label and value it gave: one, or one a component (as `Decompose`).
    pub to: Vec<ConversionOutput>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversionOutput {
    pub label: Vec<Segment>,
    pub value: Vec<Segment>,
}

impl ConversionOutput {
    pub fn new(label: Vec<Segment>, value: Vec<Segment>) -> Self {
        Self { label, value }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversionInput {
    pub label: String,
    pub value: String,
}

impl ConversionInput {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub text: String,
    /// From the input (e.g. a regex's `$1`), not the settings.
    pub from_input: bool,
    /// A log's level, shown as a tag (its name the text).
    pub level: Option<LogLevel>,
}

impl Segment {
    pub fn fixed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from_input: false,
            level: None,
        }
    }

    pub fn from_input(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from_input: true,
            level: None,
        }
    }

    /// A log's level, as a tag; `from_input` as [`Self::from_input`]'s.
    pub fn level(level: LogLevel, from_input: bool) -> Self {
        Self {
            text: level.as_str().to_string(),
            from_input,
            level: Some(level),
        }
    }
}

/// Trims the whitespace around the text of `segments`.
pub(crate) fn trim_segments(mut segments: Vec<Segment>) -> Vec<Segment> {
    segments.retain(|segment| !segment.text.is_empty());
    while let Some(first) = segments.first_mut() {
        first.text = first.text.trim_start().to_string();
        if !first.text.is_empty() {
            break;
        }
        segments.remove(0);
    }
    while let Some(last) = segments.last_mut() {
        last.text = last.text.trim_end().to_string();
        if !last.text.is_empty() {
            break;
        }
        segments.pop();
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::{Segment, trim_segments};

    #[test]
    fn trim_segments_trims_the_whole_text() {
        let segments = vec![
            Segment::from_input(" "),
            Segment::fixed(" led_"),
            Segment::from_input(""),
            Segment::from_input("on "),
            Segment::fixed(" "),
        ];
        assert_eq!(
            trim_segments(segments),
            vec![Segment::fixed("led_"), Segment::from_input("on")]
        );
    }
}
