use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField};
use super::form_of;
use super::map_list::{
    LOG_LABELS_LIST_ID, NUMBERS_LABELS_LIST_ID, VECTOR2D_LABELS_LIST_ID, VECTOR3D_LABELS_LIST_ID,
};
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

pub const MATCH_TO_NUMBER: MapKind = MapKind {
    name: "Match",
    from: &[DataType::Log],
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

/// A preset to a vector or log, its components (or level and message) from the
/// groups `$components`.
macro_rules! components_preset {
    ($name:literal, $output:ident, $pattern:expr, $to_label:expr, $components:expr) => {
        MapPreset {
            name: $name,
            detail: $pattern,
            create: || -> Box<dyn MapRunner> {
                Box::new(Match::with_components(
                    MatchOutput::$output,
                    "",
                    $pattern,
                    $to_label,
                    &$components,
                ))
            },
        }
    };
}

/// A 2D vector alone, e.g. `(1.5, -2)`.
pub const XY_ONLY: &str = r"^\(({number}), ({number})\)$";

/// `name: (x, y)`, e.g. `pos:  (1.5, -2)`.
pub const NAME_COLON_XY: &str = r"({word}):  \(({number}), ({number})\)";

/// A 3D vector alone, e.g. `(1.5, -2, 0)`.
pub const XYZ_ONLY: &str = r"^\(({number}), ({number}), ({number})\)$";

/// `name: (x, y, z)`, e.g. `acc:  (1.5, -2, 0)`.
pub const NAME_COLON_XYZ: &str = r"({word}):  \(({number}), ({number}), ({number})\)";

/// A vector's components from the groups, e.g. `$2` and `$3`.
pub const MATCH_TO_VECTOR2D: MapKind = MapKind {
    name: "Match",
    from: &[DataType::Log],
    to: DataType::Vector2d,
    create: || Box::new(Match::new(MatchOutput::Vector2d)),
    form: |settings: &dyn Any| match_form(settings, VECTOR2D_LABELS_LIST_ID),
    presets: &[
        // No name to take the output label from
        components_preset!("(x, y)", Vector2d, XY_ONLY, "", ["$1", "$2"]),
        components_preset!("name: (x, y)", Vector2d, NAME_COLON_XY, "$1", ["$2", "$3"]),
    ],
};

pub const MATCH_TO_VECTOR3D: MapKind = MapKind {
    name: "Match",
    from: &[DataType::Log],
    to: DataType::Vector3d,
    create: || Box::new(Match::new(MatchOutput::Vector3d)),
    form: |settings: &dyn Any| match_form(settings, VECTOR3D_LABELS_LIST_ID),
    presets: &[
        // No name to take the output label from
        components_preset!("(x, y, z)", Vector3d, XYZ_ONLY, "", ["$1", "$2", "$3"]),
        components_preset!(
            "name: (x, y, z)",
            Vector3d,
            NAME_COLON_XYZ,
            "$1",
            ["$2", "$3", "$4"]
        ),
    ],
};

/// A log's level and message from the groups, e.g. `$1` and `$2`.
pub const MATCH_TO_LOG: MapKind = MapKind {
    name: "Match",
    from: &[DataType::Log],
    to: DataType::Log,
    create: || Box::new(Match::new(MatchOutput::Log)),
    form: |settings: &dyn Any| {
        form_of(settings, |settings: MatchSettings| {
            rsx! {
                MatchForm {
                    settings,
                    output_list: LOG_LABELS_LIST_ID.to_string(),
                    component_names: vec!["Level".to_string(), "Message".to_string()],
                }
            }
        })
    },
    presets: &[],
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

/// A vector's or log's fields named by `component_names` (x, y and z unless
/// given).
#[component]
pub fn MatchForm(
    settings: MatchSettings,
    output_list: String,
    #[props(default)] component_names: Vec<String>,
) -> Element {
    let MatchSettings {
        from_label,
        to_label,
        pattern,
        replacement,
        y_replacement,
        z_replacement,
        pattern_error,
        replacement_error,
        weakest_level,
    } = settings;
    let name = |index: usize| {
        component_names
            .get(index)
            .cloned()
            .unwrap_or_else(|| ["x", "y", "z"][index].to_string())
    };

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: from_label,
                    list: LOG_LABELS_LIST_ID,
                    placeholder: "Input label",
                    weakest_level,
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
                if let Some(y_replacement) = y_replacement {
                    // A vector or log: each component in its own field
                    TextField { name: name(0), value: replacement, error: replacement_error }
                    TextField { name: name(1), value: y_replacement, error: replacement_error }
                    if let Some(z_replacement) = z_replacement {
                        TextField { name: name(2), value: z_replacement, error: replacement_error }
                    }
                } else {
                    TextField { name: "To", value: replacement, error: replacement_error }
                }
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

    use super::{MATCH_TO_NUMBER, MATCH_TO_VECTOR2D, MATCH_TO_VECTOR3D};

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

    /// The groups of `preset`'s pattern in `input`, if it matches.
    fn groups(kind: &super::MapKind, preset: &str, input: &str) -> Option<Vec<String>> {
        let preset = kind.presets.iter().find(|p| p.name == preset).unwrap();
        let captures = compile_pattern(preset.detail)
            .unwrap()
            .captures(input)
            .unwrap()?;
        Some(
            captures
                .iter()
                .skip(1)
                .map(|group| group.map_or(String::new(), |group| group.as_str().to_string()))
                .collect(),
        )
    }

    #[test]
    fn vector2d_presets_match_their_formats() {
        let kind = &MATCH_TO_VECTOR2D;
        assert_eq!(
            groups(kind, "(x, y)", "(1.5, -2)"),
            Some(vec!["1.5".into(), "-2".into()])
        );
        assert_eq!(groups(kind, "(x, y)", "(1, 2, 3)"), None);
        assert_eq!(
            groups(kind, "name: (x, y)", "pos:  (1, 2)"),
            Some(vec!["pos".into(), "1".into(), "2".into()])
        );
    }

    #[test]
    fn vector3d_presets_match_their_formats() {
        let kind = &MATCH_TO_VECTOR3D;
        assert_eq!(
            groups(kind, "(x, y, z)", "(1.5, -2, 3e2)"),
            Some(vec!["1.5".into(), "-2".into(), "3e2".into()])
        );
        assert_eq!(groups(kind, "(x, y, z)", "acc: (1, 2, 3)"), None);
        assert_eq!(
            groups(kind, "name: (x, y, z)", "acc:  (1, 2, 3)"),
            Some(vec!["acc".into(), "1".into(), "2".into(), "3".into()])
        );
    }
}
