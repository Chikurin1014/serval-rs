use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField};
use super::map_list::{NUMBERS_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{
    DataType, MapKind, MapPreset, MapRunner, RegexMatch, RegexOutput, RegexSettings,
};

/// A preset whose `pattern` matches a name (`$1`, the output label) and a
/// value (`$2`).
macro_rules! preset {
    ($name:literal, $output:ident, $pattern:expr) => {
        MapPreset {
            name: $name,
            detail: $pattern,
            create: || -> Box<dyn MapRunner> {
                Box::new(RegexMatch::with(
                    RegexOutput::$output,
                    "",
                    $pattern,
                    "$1",
                    "$2",
                ))
            },
        }
    };
}

/// `name: value` with a number value, e.g. `temp: 20.5`.
pub const NAME_COLON_NUMBER: &str = r"(\w+): (-?\d+(\.\d+)?(e\d+)?)";

pub const REGEX_TO_STRING: MapKind = MapKind {
    name: "Regex",
    from: &[DataType::String],
    to: DataType::String,
    create: || Box::new(RegexMatch::new(RegexOutput::String)),
    // Offer the existing labels of the result's type as outputs
    form: |settings: &dyn Any| regex_form(settings, STRINGS_LABELS_LIST_ID),
    presets: &[
        preset!("name: value", String, r"(.+): (.+)"),
        preset!("name=value", String, r"(.+)=(.+)"),
    ],
};

pub const REGEX_TO_NUMBER: MapKind = MapKind {
    name: "Regex",
    from: &[DataType::String],
    to: DataType::Number,
    create: || Box::new(RegexMatch::new(RegexOutput::Number)),
    form: |settings: &dyn Any| regex_form(settings, NUMBERS_LABELS_LIST_ID),
    presets: &[
        preset!("name: value", Number, NAME_COLON_NUMBER),
        preset!("name=value", Number, r"(\w+)=(-?\d+(\.\d+)?(e\d+)?)"),
        preset!("Teleplot", Number, r">(\w+):(-?\d+(\.\d+)?(e\d+)?)"),
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
    use regex::Regex;

    use super::{REGEX_TO_NUMBER, REGEX_TO_STRING};

    /// What `preset`'s pattern makes of `input`: its `$1` and `$2`.
    fn name_value(kind: &super::MapKind, preset: &str, input: &str) -> Option<(String, String)> {
        let preset = kind.presets.iter().find(|p| p.name == preset).unwrap();
        let captures = Regex::new(preset.detail).unwrap().captures(input)?;
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
        assert_eq!(
            name_value(kind, "name=value", "volt=3e2"),
            pair("volt", "3e2")
        );
        assert_eq!(
            name_value(kind, "Teleplot", ">temp:1.5e3"),
            pair("temp", "1.5e3")
        );
        assert_eq!(name_value(kind, "name: value", "temp: on"), None);
        assert_eq!(name_value(kind, "Teleplot", "temp:20"), None);
    }

    #[test]
    fn string_presets_match_their_formats() {
        let kind = &REGEX_TO_STRING;
        assert_eq!(
            name_value(kind, "name: value", "led: on"),
            pair("led", "on")
        );
        assert_eq!(
            name_value(kind, "name=value", "mode=auto"),
            pair("mode", "auto")
        );
    }
}
