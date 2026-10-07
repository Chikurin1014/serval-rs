//! What a map shows of its latest conversion: the input values it took, and
//! the label and value it made, in segments marking what came from the input.

/// What a map took in (one value of each input) and what it was turned into.
#[derive(Clone, Debug, PartialEq)]
pub struct Conversion {
    pub from: Vec<ConversionInput>,
    pub to_label: Vec<Segment>,
    pub to_value: Vec<Segment>,
}

/// One input's value in a [`Conversion`].
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

/// A piece of a [`Conversion`]'s result.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub text: String,
    /// Taken from the input (e.g. a regex's `$1`), rather than written in
    /// the map's settings.
    pub from_input: bool,
}

impl Segment {
    pub fn fixed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from_input: false,
        }
    }

    pub fn from_input(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from_input: true,
        }
    }
}

/// Trims whitespace around the text that `segments` make up, dropping
/// segments left empty.
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
