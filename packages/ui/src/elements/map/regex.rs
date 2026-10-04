use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use crate::{
    components::input::Input,
    data::{DataType, MapKind, RegexMatch, RegexOutput, RegexSettings},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub const REGEX_TO_STRING: MapKind = MapKind {
    name: "Regex (to String)",
    from: DataType::String,
    to: DataType::String,
    create: || Box::new(RegexMatch::new(RegexOutput::String)),
    // The result is a string, so offer the existing string labels as targets
    form: |settings: &dyn Any| regex_form(settings, Some("map-strings-labels")),
};

pub const REGEX_TO_NUMBER: MapKind = MapKind {
    name: "Regex (to Number)",
    from: DataType::String,
    to: DataType::Number,
    create: || Box::new(RegexMatch::new(RegexOutput::Number)),
    form: |settings: &dyn Any| regex_form(settings, None),
};

/// `target_list`: `datalist` id offered for the target label (defined in `MapList`).
fn regex_form(settings: &dyn Any, target_list: Option<&str>) -> Element {
    match settings.downcast_ref::<RegexSettings>() {
        Some(&settings) => rsx! {
            RegexMatchForm {
                settings,
                target_list: target_list.map(str::to_string),
            }
        },
        None => VNode::empty(),
    }
}

/// Settings form of a `Regex` map.
#[component]
pub fn RegexMatchForm(settings: RegexSettings, target_list: Option<String>) -> Element {
    let RegexSettings {
        mut from_label,
        mut to_label,
        mut pattern,
        mut replacement,
        mut pattern_error,
        mut replacement_error,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_FORM_CSS }

        div {
            class: "map-row",
            div {
                class: "field-stack",
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        list: "map-strings-labels", // Defined in `MapList` component
                        placeholder: "Source label",
                        autocomplete: "on",
                        value: "{from_label}",
                        oninput: move |event: FormEvent| from_label.set(event.value()),
                    }
                }
                label {
                    class: "field",
                    span { class: "field-label", "From" }
                    lucide::Regex {}
                    Input {
                        placeholder: "Text to be matched",
                        value: "{pattern}",
                        oninput: move |event: FormEvent| {
                            pattern.set(event.value());
                            pattern_error.set(None);
                        },
                    }
                }
                if let Some(error) = pattern_error() {
                    span { class: "field-error", "{error}" }
                }
            }
            lucide::MoveRight { size: 20 }
            div {
                class: "field-stack",
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        placeholder: "Target label",
                        list: target_list,
                        value: "{to_label}",
                        oninput: move |event: FormEvent| to_label.set(event.value()),
                    }
                }
                label {
                    class: "field",
                    span { class: "field-label", "To" }
                    Input {
                        value: "{replacement}",
                        oninput: move |event: FormEvent| {
                            replacement.set(event.value());
                            replacement_error.set(None);
                        },
                    }
                }
                if let Some(error) = replacement_error() {
                    span { class: "field-error", "{error}" }
                }
            }
        }
    }
}
