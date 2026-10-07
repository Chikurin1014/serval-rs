use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField};
use super::map_list::{NUMBERS_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{
    DataType, MapKind, MapPreset, MapRunner, RegexMatch, RegexOutput, RegexSettings,
};

/// A preset whose `pattern` matches a name (`$1`, the output label) and a
/// value (`$2`), or with the output label and value given.
macro_rules! preset {
    ($name:literal, $output:ident, $pattern:expr) => {
        preset!($name, $output, $pattern, "$1", "$2")
    };
    ($name:literal, $output:ident, $pattern:expr, $to_label:expr, $replacement:expr) => {
        MapPreset {
            name: $name,
            detail: $pattern,
            create: || -> Box<dyn MapRunner> {
                Box::new(RegexMatch::with(
                    RegexOutput::$output,
                    "",
                    $pattern,
                    $to_label,
                    $replacement,
                ))
            },
        }
    };
}

/// A value with no label: a number at the start of the line, e.g. `20.5`.
/// The presets use the aliases (see `crate::data::PATTERN_ALIASES`).
pub const NUMBER_ONLY: &str = "^{number}";

/// The output label of the initial map reading the values with no label.
pub const ANONYMOUS_LABEL: &str = "anonymous data";

/// `name: value` with a number value, e.g. `temp: 20.5`.
pub const NAME_COLON_NUMBER: &str = "({word}): ({number})";

pub const REGEX_TO_STRING: MapKind = MapKind {
    name: "Regex",
    from: &[DataType::String],
    to: DataType::String,
    create: || Box::new(RegexMatch::new(RegexOutput::String)),
    // Offer the existing labels of the result's type as outputs
    form: |settings: &dyn Any| regex_form(settings, STRINGS_LABELS_LIST_ID),
    presets: &[
        preset!("name: value", String, "({word}): (.+)"),
        // As the Arduino IDE's serial plotter reads them
        preset!("Arduino", String, "({word}):(.+)"),
    ],
};

pub const REGEX_TO_NUMBER: MapKind = MapKind {
    name: "Regex",
    from: &[DataType::String],
    to: DataType::Number,
    create: || Box::new(RegexMatch::new(RegexOutput::Number)),
    form: |settings: &dyn Any| regex_form(settings, NUMBERS_LABELS_LIST_ID),
    presets: &[
        // The output label is left to be set, as there is no name to take it from
        preset!("value", Number, NUMBER_ONLY, "", "$0"),
        preset!("name: value", Number, NAME_COLON_NUMBER),
        // As the Arduino IDE's serial plotter reads them
        preset!("Arduino", Number, "({word}):({number})"),
        preset!("Teleplot", Number, ">({word}):({number})"),
    ],
};

/// `output_list`: `datalist` id offered for the output label (defined in `MapList`).
fn regex_form(settings: &dyn Any, output_list: &str) -> Element {
    match settings.downcast_ref::<RegexSettings>() {
        Some(&settings) => rsx! {
            RegexMatchForm {
                settings,
                output_list: output_list.to_string(),
            }
        },
        None => VNode::empty(),
    }
}

/// Settings form of a `Regex` map.
#[component]
pub fn RegexMatchForm(settings: RegexSettings, output_list: String) -> Element {
    let RegexSettings {
        from_label,
        to_label,
        pattern,
        replacement,
        pattern_error,
        replacement_error,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: from_label,
                    list: STRINGS_LABELS_LIST_ID,
                    placeholder: "Input label",
                }
                TextField {
                    name: "From",
                    icon: rsx! { lucide::Regex {} },
                    value: pattern,
                    placeholder: "Text to be matched",
                    error: pattern_error,
                }
                if let Some(error) = pattern_error() {
                    span { class: "field-error", "{error}" }
                }
            }
            lucide::MoveRight { size: 20 }
            div {
                class: "field-stack",
                LabelField { value: to_label, list: output_list, placeholder: "Output label" }
                TextField { name: "To", value: replacement, error: replacement_error }
                if let Some(error) = replacement_error() {
                    span { class: "field-error", "{error}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::compile_pattern;

    use super::{REGEX_TO_NUMBER, REGEX_TO_STRING};

    /// What `preset`'s pattern makes of `input`: its `$1` and `$2`.
    fn name_value(kind: &super::MapKind, preset: &str, input: &str) -> Option<(String, String)> {
        let preset = kind.presets.iter().find(|p| p.name == preset).unwrap();
        let captures = compile_pattern(preset.detail)
            .unwrap()
            .captures(input)
            .unwrap()?;
        Some((captures[1].to_string(), captures[2].to_string()))
    }

    fn pair(name: &str, value: &str) -> Option<(String, String)> {
        Some((name.to_string(), value.to_string()))
    }

    #[test]
    fn number_presets_match_their_formats() {
        let kind = &REGEX_TO_NUMBER;
        assert_eq!(
            name_value(kind, "name: value", "temp: -20.5"),
            pair("temp", "-20.5")
        );
        assert_eq!(name_value(kind, "Arduino", "volt:3e2"), pair("volt", "3e2"));
        assert_eq!(
            name_value(kind, "Teleplot", ">temp:1.5e3"),
            pair("temp", "1.5e3")
        );
        for value in ["20", "+20", "-0.5", ".5", "1.", "1.5E-3", "-2e+10"] {
            assert_eq!(
                name_value(kind, "name: value", &format!("temp: {value}")),
                pair("temp", value)
            );
            assert!(value.parse::<f64>().is_ok(), "{value}");
        }
        assert_eq!(name_value(kind, "name: value", "temp: on"), None);
        assert_eq!(name_value(kind, "name: value", "temp: ."), None);
        assert_eq!(name_value(kind, "Teleplot", "temp:20"), None);
    }

    #[test]
    fn value_preset_takes_a_number_with_no_label() {
        let preset = REGEX_TO_NUMBER
            .presets
            .iter()
            .find(|p| p.name == "value")
            .unwrap();
        let regex = compile_pattern(preset.detail).unwrap();
        let value = |input: &str| {
            regex
                .captures(input)
                .unwrap()
                .map(|captures| captures[0].to_string())
        };
        assert_eq!(value("-20.5"), Some("-20.5".to_string()));
        assert_eq!(value("1.5e3"), Some("1.5e3".to_string()));
        assert_eq!(value("temp: 20.5"), None);
    }

    #[test]
    fn string_presets_match_their_formats() {
        let kind = &REGEX_TO_STRING;
        assert_eq!(
            name_value(kind, "name: value", "led: on"),
            pair("led", "on")
        );
        assert_eq!(
            name_value(kind, "Arduino", "mode:auto"),
            pair("mode", "auto")
        );
        // The name is a word: what comes before it is not part of it
        assert_eq!(
            name_value(kind, "name: value", "[log] led: on"),
            pair("led", "on")
        );
    }
}
