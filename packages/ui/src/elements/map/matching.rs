use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField};
use super::form_of;
use super::map_list::{NUMBERS_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{DataType, MapKind, MapPreset, MapRunner, Match, MatchOutput, MatchSettings};

/// A preset whose `pattern` gives the label as `$1` and the value as `$2`.
macro_rules! preset {
    ($name:literal, $output:ident, $pattern:expr) => {
        preset!($name, $output, $pattern, "$1", "$2")
    };
    ($name:literal, $output:ident, $pattern:expr, $to_label:expr, $replacement:expr) => {
        MapPreset {
            name: $name,
            detail: $pattern,
            create: || -> Box<dyn MapRunner> {
                Box::new(Match::with(
                    MatchOutput::$output,
                    "",
                    $pattern,
                    $to_label,
                    $replacement,
                ))
            },
        }
    };
}

/// A number with no label, e.g. `20.5`.
pub const NUMBER_ONLY: &str = "^{number}";

pub const ANONYMOUS_LABEL: &str = "anonymous data";

/// `name: value` with a number value, e.g. `temp: 20.5`.
pub const NAME_COLON_NUMBER: &str = "({word}): ({number})";

pub const MATCH_TO_STRING: MapKind = MapKind {
    name: "Match",
    from: &[DataType::String],
    to: DataType::String,
    create: || Box::new(Match::new(MatchOutput::String)),
    form: |settings: &dyn Any| match_form(settings, STRINGS_LABELS_LIST_ID),
    presets: &[
        preset!("name: value", String, "({word}): (?!{number})(.+)"),
        preset!("Arduino", String, "({word}):(?!{number})(.+)"),
    ],
};

pub const MATCH_TO_NUMBER: MapKind = MapKind {
    name: "Match",
    from: &[DataType::String],
    to: DataType::Number,
    create: || Box::new(Match::new(MatchOutput::Number)),
    form: |settings: &dyn Any| match_form(settings, NUMBERS_LABELS_LIST_ID),
    presets: &[
        // No name to take the output label from
        preset!("value", Number, NUMBER_ONLY, "", "$0"),
        preset!("name: value", Number, NAME_COLON_NUMBER),
        preset!("Arduino", Number, "({word}):({number})"),
        preset!("Teleplot", Number, ">({word}):({number})"),
    ],
};

fn match_form(settings: &dyn Any, output_list: &str) -> Element {
    form_of(settings, |settings: MatchSettings| {
        rsx! {
            MatchForm {
                settings,
                output_list: output_list.to_string(),
            }
        }
    })
}

#[component]
pub fn MatchForm(settings: MatchSettings, output_list: String) -> Element {
    let MatchSettings {
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

    use super::{MATCH_TO_NUMBER, MATCH_TO_STRING};

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
        let kind = &MATCH_TO_NUMBER;
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
        let preset = MATCH_TO_NUMBER
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
        let kind = &MATCH_TO_STRING;
        assert_eq!(
            name_value(kind, "name: value", "led: on"),
            pair("led", "on")
        );
        assert_eq!(
            name_value(kind, "Arduino", "mode:auto"),
            pair("mode", "auto")
        );
        assert_eq!(
            name_value(kind, "name: value", "[log] led: on"),
            pair("led", "on")
        );
    }
}
