use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::{NUMBERS_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::{
    components::input::Input,
    data::{DataType, MapKind, RegexMatch, RegexOutput, RegexSettings},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub const REGEX_TO_STRING: MapKind = MapKind {
    name: "Regex",
    from: &[DataType::String],
    to: DataType::String,
    create: || Box::new(RegexMatch::new(RegexOutput::String)),
    // Offer the existing labels of the result's type as outputs
    form: |settings: &dyn Any| regex_form(settings, STRINGS_LABELS_LIST_ID),
};

pub const REGEX_TO_NUMBER: MapKind = MapKind {
    name: "Regex",
    from: &[DataType::String],
    to: DataType::Number,
    create: || Box::new(RegexMatch::new(RegexOutput::Number)),
    form: |settings: &dyn Any| regex_form(settings, NUMBERS_LABELS_LIST_ID),
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
                        list: STRINGS_LABELS_LIST_ID,
                        placeholder: "Input label",
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
                        placeholder: "Output label",
                        list: output_list,
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
