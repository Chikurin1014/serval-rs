use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdMoveRight, LdRegex, LdTag},
    Icon,
};

use std::any::Any;

use crate::{
    components::input::Input,
    data::{ConversionKind, DataType, RegexMatch, RegexMatchSettings, RegexOutput},
};

const CONVERSION_CSS: Asset = asset!("/assets/styling/conversion.css");

pub const REGEX_TO_STRING: ConversionKind = ConversionKind {
    name: "Regex (to String)",
    from: DataType::String,
    to: DataType::String,
    create: || Box::new(RegexMatch::new(RegexOutput::String)),
    // The result is a string, so offer the existing string labels as targets
    form: |settings: &dyn Any| regex_match_form(settings, Some("conversion-strings-labels")),
};

pub const REGEX_TO_NUMBER: ConversionKind = ConversionKind {
    name: "Regex (to Number)",
    from: DataType::String,
    to: DataType::Number,
    create: || Box::new(RegexMatch::new(RegexOutput::Number)),
    form: |settings: &dyn Any| regex_match_form(settings, None),
};

/// `target_list`: `datalist` id offered for the target label (defined in `ConversionList`).
fn regex_match_form(settings: &dyn Any, target_list: Option<&str>) -> Element {
    match settings.downcast_ref::<RegexMatchSettings>() {
        Some(&settings) => rsx! {
            RegexMatchForm {
                settings,
                target_list: target_list.map(str::to_string),
            }
        },
        None => VNode::empty(),
    }
}

/// Settings form of a `RegexMatch` conversion.
#[component]
pub fn RegexMatchForm(settings: RegexMatchSettings, target_list: Option<String>) -> Element {
    let RegexMatchSettings {
        mut from_label,
        mut to_label,
        mut pattern,
        mut replacement,
        mut pattern_error,
        mut replacement_error,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: CONVERSION_CSS }

        div {
            class: "conversion-row",
            div {
                class: "field-stack",
                label {
                    class: "field",
                    Icon { icon: LdTag {} }
                    Input {
                        list: "conversion-strings-labels", // Defined in `ConversionList` component
                        placeholder: "Source label",
                        autocomplete: "on",
                        value: "{from_label}",
                        oninput: move |event: FormEvent| from_label.set(event.value()),
                    }
                }
                label {
                    class: "field",
                    span { class: "field-label", "From" }
                    Icon { icon: LdRegex {} }
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
            Icon { icon: LdMoveRight {} }
            div {
                class: "field-stack",
                label {
                    class: "field",
                    Icon { icon: LdTag {} }
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
